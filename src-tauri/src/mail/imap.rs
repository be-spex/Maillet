//! IMAP access: one watching connection per account (IDLE) and a short-lived
//! command connection for reading, marking and moving mails.

use anyhow::{bail, Context, Result};
use async_imap::extensions::idle::IdleResponse;
use async_imap::types::NameAttribute;
use async_trait::async_trait;
use futures_util::TryStreamExt;
use native_tls::TlsConnector;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Notify};
use tokio_native_tls::TlsConnector as TokioTlsConnector;

use super::model::{MailBody, MailContent, MailSummary};
use super::parse::{self, Origin};
use super::store::MailStore;
use crate::base::config::{AccountConfig, Security};
use crate::base::{log, status};

/// Re-issue IDLE this often. Short enough to notice silently dropped
/// connections (NAT, VPN, suspend) quickly; the server must answer DONE.
const IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
/// Max time for the server to answer while leaving IDLE / checking.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
/// Unread mails shown per account; the count still covers all of them.
const LIST_LIMIT: usize = 200;
/// Bytes of the body fetched for the preview.
const PREVIEW_BYTES: u32 = 4096;
/// UIDs per FETCH command.
const FETCH_CHUNK: usize = 50;
/// A command connection unused for this long is replaced by a new one.
const COMMAND_SESSION_MAX_IDLE: Duration = Duration::from_secs(4 * 60);

type Session = async_imap::Session<tokio_native_tls::TlsStream<TcpStream>>;

/// What the mail core asks of the features layer, without knowing any feature.
#[async_trait]
pub trait MailHook: Send + Sync {
    /// Does any feature need the full body of this new mail?
    fn wants_body(&self, account: &AccountConfig, mail: &MailSummary) -> bool;
    /// Lets features look at a new mail; they add findings to it.
    async fn on_new_mail(&self, account: &AccountConfig, mail: &mut MailSummary, body: Option<&MailBody>);
}

async fn with_timeout<T>(fut: impl std::future::Future<Output = Result<T, async_imap::error::Error>>) -> Result<T> {
    tokio::time::timeout(COMMAND_TIMEOUT, fut)
        .await
        .context("Server did not respond – connection lost")?
        .map_err(Into::into)
}

pub async fn connect(account: &AccountConfig) -> Result<Session> {
    tokio::time::timeout(CONNECT_TIMEOUT, connect_inner(account))
        .await
        .with_context(|| format!("Timeout connecting to {}", account.server))?
}

async fn connect_inner(account: &AccountConfig) -> Result<Session> {
    let addr = format!("{}:{}", account.server, account.port);
    let tcp = TcpStream::connect(&addr)
        .await
        .with_context(|| format!("Failed to connect to {addr}"))?;

    let tcp = match account.security {
        Security::Ssl => tcp,
        Security::Starttls => {
            let mut plain = async_imap::Client::new(tcp);
            plain
                .read_response()
                .await
                .context("No greeting from server")?
                .context("Connection closed before greeting")?;
            plain
                .run_command_and_check_ok("STARTTLS", None)
                .await
                .context("STARTTLS failed – try SSL/TLS instead")?;
            plain.into_inner()
        }
    };

    let tls = TokioTlsConnector::from(
        TlsConnector::builder()
            .danger_accept_invalid_certs(account.allow_invalid_certs)
            .danger_accept_invalid_hostnames(account.allow_invalid_certs)
            .build()?,
    );
    let tls_stream = tls.connect(&account.server, tcp).await.map_err(|e| {
        let msg = e.to_string();
        let hint = if msg.contains("wrong version number") && account.security == Security::Ssl {
            " – the server expects STARTTLS, change the encryption"
        } else if msg.contains("certificate") && !account.allow_invalid_certs {
            " – self-signed certificate? enable \"Accept self-signed certificate\""
        } else {
            ""
        };
        anyhow::anyhow!("TLS handshake failed: {msg}{hint}")
    })?;

    async_imap::Client::new(tls_stream)
        .login(&account.user, &account.pass)
        .await
        .map_err(|(e, _)| anyhow::anyhow!("Login failed for {}: {e}", account.label))
}

/// Logs in and opens the mailbox, for the "Test connection" button.
pub async fn test_connection(account: &AccountConfig) -> Result<String> {
    let mut session = connect(account).await?;
    let mailbox = session
        .select(&account.mailbox)
        .await
        .with_context(|| format!("Login OK, but mailbox '{}' not found", account.mailbox))?;
    session.logout().await.ok();
    Ok(format!("Connected – {} mails in {}", mailbox.exists, account.mailbox))
}

// ---------------------------------------------------------------- Watcher

/// Everything a watcher needs; shared across reconnects.
pub struct Watch {
    pub app: AppHandle,
    pub account: AccountConfig,
    pub check: Arc<Notify>,
    /// Highest UID handed to the features; survives reconnects so nothing is analysed twice.
    pub last_uid: Arc<AtomicU32>,
    pub store: Arc<MailStore>,
    pub hook: Arc<dyn MailHook>,
}

/// Connects, loads the unread mails and then waits for changes (IDLE).
/// Returns only on errors; the caller reconnects.
pub async fn run_watcher(w: &Watch) -> Result<()> {
    let account = &w.account;
    status::set(&w.app, &account.id, "connecting", format!("Connecting to {}…", account.server));
    log::event(&w.app, &account.id, &account.label, "info", format!("Connecting to {}:{}", account.server, account.port));

    let mut session = connect(account).await?;
    let mailbox = session
        .select(&account.mailbox)
        .await
        .with_context(|| format!("Failed to select mailbox '{}'", account.mailbox))?;
    let uid_validity = mailbox.uid_validity.unwrap_or(0);

    // Features only see mails arriving from now on. A UIDNEXT below the
    // stored UID means the server renumbered the mailbox (UIDVALIDITY change).
    if let Some(next) = mailbox.uid_next {
        let seen = w.last_uid.load(Ordering::Relaxed);
        if seen == 0 || next <= seen {
            w.last_uid.store(next.saturating_sub(1), Ordering::Relaxed);
        }
    }
    tracing::info!("[{}] Connected to {} – watching {}", account.label, account.server, account.mailbox);
    log::event(&w.app, &account.id, &account.label, "info", format!("Connected – watching {}", account.mailbox));

    sync(&mut session, w, uid_validity).await?;

    loop {
        let mut handle = session.idle();
        handle.init().await.context("Failed to init IDLE")?;
        let idle_started = Instant::now();
        let response = {
            let (wait_fut, stop) = handle.wait_with_timeout(IDLE_TIMEOUT);
            tokio::pin!(wait_fut);
            tokio::select! {
                r = &mut wait_fut => r?,
                _ = w.check.notified() => {
                    drop(stop);
                    wait_fut.await?
                }
            }
        };
        match response {
            IdleResponse::NewData(_) => {
                tracing::debug!("[{}] IDLE woke up after {:.1?}", account.label, idle_started.elapsed());
                log::event(&w.app, &account.id, &account.label, "info", "Server reported new data (IDLE)");
            }
            IdleResponse::Timeout => tracing::debug!("[{}] IDLE timeout – re-checking", account.label),
            IdleResponse::ManualInterrupt => log::event(&w.app, &account.id, &account.label, "info", "Manual check"),
        }
        session = tokio::time::timeout(COMMAND_TIMEOUT, handle.done())
            .await
            .context("Server did not respond – connection lost")?
            .context("Failed to end IDLE")?;
        sync(&mut session, w, uid_validity).await?;
    }
}

/// Brings the store in line with the server and hands new mails to the features.
async fn sync(session: &mut Session, w: &Watch, uid_validity: u32) -> Result<()> {
    let account = &w.account;
    let started = Instant::now();
    let origin = |uid| Origin { account_id: &account.id, account: &account.label, uid_validity, uid };

    let mut unseen: Vec<u32> = with_timeout(session.uid_search("UNSEEN")).await.context("SEARCH UNSEEN failed")?.into_iter().collect();
    unseen.sort_unstable();
    let total = unseen.len();
    let shown: Vec<u32> = unseen.iter().rev().take(LIST_LIMIT).copied().collect();

    // New since the last check, read or not: another client may already
    // have marked a code mail as read.
    let seen = w.last_uid.load(Ordering::Relaxed);
    let mut fresh: Vec<u32> = with_timeout(session.uid_search(format!("UID {}:*", seen + 1)))
        .await
        .context("UID SEARCH failed")?
        .into_iter()
        // `n:*` always includes the newest mail, even if its UID is below n.
        .filter(|&uid| uid > seen)
        .collect();
    fresh.sort_unstable();

    let mut known = w.store.known(&account.id);
    let to_fetch: Vec<u32> = shown.iter().chain(fresh.iter()).copied().filter(|uid| !known.contains_key(uid)).collect::<HashSet<_>>().into_iter().collect();
    let mut fetched = fetch_summaries(session, &to_fetch, &origin).await?;

    for uid in &fresh {
        let Some(mail) = fetched.get_mut(uid) else { continue };
        let body = if w.hook.wants_body(account, mail) { Some(fetch_body(session, *uid).await?) } else { None };
        w.hook.on_new_mail(account, mail, body.as_ref()).await;
        w.last_uid.store(*uid, Ordering::Relaxed);
    }

    let mails: Vec<MailSummary> = shown.iter().filter_map(|uid| known.remove(uid).or_else(|| fetched.remove(uid))).collect();
    w.store.set_account(&account.id, &account.label, total, mails);

    let message = format!("Connected – {total} unread");
    status::set(&w.app, &account.id, "ok", message);
    if !fresh.is_empty() {
        let message = format!("Mailbox checked – {} new after {} ms", fresh.len(), started.elapsed().as_millis());
        log::event(&w.app, &account.id, &account.label, "info", message);
    }
    Ok(())
}

/// Header plus the first bytes of the body, in chunks.
async fn fetch_summaries<'a>(session: &mut Session, uids: &[u32], origin: &(dyn Fn(u32) -> Origin<'a> + Sync)) -> Result<HashMap<u32, MailSummary>> {
    let mut out = HashMap::new();
    for chunk in uids.chunks(FETCH_CHUNK) {
        let set = chunk.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
        let query = format!("(UID BODY.PEEK[HEADER] BODY.PEEK[TEXT]<0.{PREVIEW_BYTES}>)");
        let stream = with_timeout(session.uid_fetch(&set, &query)).await.context("FETCH failed")?;
        let fetches: Vec<_> = tokio::time::timeout(COMMAND_TIMEOUT, stream.try_collect())
            .await
            .context("Server did not respond – connection lost")??;
        for f in fetches {
            let Some(uid) = f.uid else { continue };
            let summary = parse::summarize(&origin(uid), f.header().unwrap_or(&[]), f.text().unwrap_or(&[]));
            out.insert(uid, summary);
        }
    }
    Ok(out)
}

async fn fetch_raw(session: &mut Session, uid: u32) -> Result<Vec<u8>> {
    let stream = with_timeout(session.uid_fetch(uid.to_string(), "BODY.PEEK[]")).await.context("FETCH failed")?;
    let fetches: Vec<_> = tokio::time::timeout(COMMAND_TIMEOUT, stream.try_collect())
        .await
        .context("Server did not respond – connection lost")??;
    match fetches.iter().find_map(|f| f.body()) {
        Some(raw) => Ok(raw.to_vec()),
        None => bail!("Mail no longer exists"),
    }
}

async fn fetch_body(session: &mut Session, uid: u32) -> Result<MailBody> {
    Ok(parse::parse_body(&fetch_raw(session, uid).await?))
}

// ---------------------------------------------------------------- Commands

/// Command connections per account. The watching connection sits in IDLE,
/// so reading and moving mails needs a second one; it is kept for reuse.
#[derive(Default)]
pub struct Commands {
    sessions: Mutex<HashMap<String, (Session, Instant)>>,
}

impl Commands {
    async fn checkout(&self, account: &AccountConfig) -> Result<Session> {
        if let Some((session, used)) = self.sessions.lock().await.remove(&account.id) {
            if used.elapsed() < COMMAND_SESSION_MAX_IDLE {
                return Ok(session);
            }
        }
        let mut session = connect(account).await?;
        with_timeout(session.select(&account.mailbox))
            .await
            .with_context(|| format!("Failed to select mailbox '{}'", account.mailbox))?;
        Ok(session)
    }

    async fn checkin(&self, account: &AccountConfig, session: Session) {
        self.sessions.lock().await.insert(account.id.clone(), (session, Instant::now()));
    }

    /// Runs `op` on a command connection; a broken connection is replaced once.
    async fn run<T>(&self, account: &AccountConfig, op: impl AsyncOp<T>) -> Result<T> {
        for attempt in 0..2 {
            let mut session = self.checkout(account).await?;
            match op.call(&mut session).await {
                Ok(v) => {
                    self.checkin(account, session).await;
                    return Ok(v);
                }
                Err(e) if attempt == 0 && is_connection_error(&e) => continue,
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }

    pub async fn content(&self, account: &AccountConfig, uid: u32) -> Result<MailContent> {
        self.run(account, Content(uid)).await
    }

    pub async fn set_seen(&self, account: &AccountConfig, uid: u32, seen: bool) -> Result<()> {
        self.run(account, SetSeen(uid, seen)).await
    }

    /// Marks the mail as read and moves it to the archive folder. Returns the folder.
    pub async fn archive(&self, account: &AccountConfig, uid: u32) -> Result<String> {
        self.run(account, Archive(uid, account.mailbox.clone())).await
    }

    /// Moves the mail to the trash, or flags and expunges it if there is none.
    pub async fn delete(&self, account: &AccountConfig, uid: u32) -> Result<()> {
        self.run(account, Delete(uid, account.mailbox.clone())).await
    }
}

fn is_connection_error(e: &anyhow::Error) -> bool {
    let msg = format!("{e:#}").to_lowercase();
    ["connection", "broken pipe", "closed", "timed out", "did not respond", "bye"].iter().any(|m| msg.contains(m))
}

/// An operation on a command connection (named types instead of async closures).
trait AsyncOp<T> {
    async fn call(&self, s: &mut Session) -> Result<T>;
}

struct Content(u32);
impl AsyncOp<MailContent> for Content {
    async fn call(&self, s: &mut Session) -> Result<MailContent> {
        Ok(parse::parse_content(&fetch_raw(s, self.0).await?))
    }
}

struct SetSeen(u32, bool);
impl AsyncOp<()> for SetSeen {
    async fn call(&self, s: &mut Session) -> Result<()> {
        let op = if self.1 { "+FLAGS.SILENT (\\Seen)" } else { "-FLAGS.SILENT (\\Seen)" };
        let stream = with_timeout(s.uid_store(self.0.to_string(), op)).await.context("STORE failed")?;
        let _: Vec<_> = stream.try_collect().await?;
        Ok(())
    }
}

/// Special-use folder (RFC 6154) or a folder with one of the usual names.
async fn find_folder(s: &mut Session, want: &[NameAttribute<'static>], names: &[&str], not: &str) -> Result<Option<String>> {
    let list: Vec<_> = with_timeout(s.list(Some(""), Some("*"))).await?.try_collect().await?;
    let usable = |n: &&async_imap::types::Name| n.name() != not && !n.attributes().contains(&NameAttribute::NoSelect);
    for attr in want {
        if let Some(n) = list.iter().filter(usable).find(|n| n.attributes().contains(attr)) {
            return Ok(Some(n.name().to_string()));
        }
    }
    Ok(list
        .iter()
        .filter(usable)
        .find(|n| {
            let leaf = n.name().rsplit(['/', '.']).next().unwrap_or("").to_lowercase();
            names.contains(&leaf.as_str())
        })
        .map(|n| n.name().to_string()))
}

/// MOVE, or COPY + delete for servers without the MOVE extension.
async fn move_to(s: &mut Session, uid: u32, folder: &str) -> Result<()> {
    if with_timeout(s.uid_mv(uid.to_string(), folder)).await.is_ok() {
        return Ok(());
    }
    with_timeout(s.uid_copy(uid.to_string(), folder)).await.with_context(|| format!("Failed to copy to '{folder}'"))?;
    let _: Vec<_> = with_timeout(s.uid_store(uid.to_string(), "+FLAGS.SILENT (\\Deleted)")).await?.try_collect().await?;
    let _: Vec<_> = with_timeout(s.uid_expunge(uid.to_string())).await?.try_collect().await?;
    Ok(())
}

struct Archive(u32, String);
impl AsyncOp<String> for Archive {
    async fn call(&self, s: &mut Session) -> Result<String> {
        // Gmail has no archive folder; "All Mail" (\All) is where archived mails live.
        let folder = find_folder(s, &[NameAttribute::Archive, NameAttribute::All], &["archive", "archiv", "archives"], &self.1)
            .await?
            .context("No archive folder found on the server – create a folder named \"Archive\"")?;
        SetSeen(self.0, true).call(s).await?;
        move_to(s, self.0, &folder).await?;
        Ok(folder)
    }
}

struct Delete(u32, String);
impl AsyncOp<()> for Delete {
    async fn call(&self, s: &mut Session) -> Result<()> {
        match find_folder(s, &[NameAttribute::Trash], &[], &self.1).await? {
            Some(trash) => move_to(s, self.0, &trash).await,
            None => {
                let _: Vec<_> = with_timeout(s.uid_store(self.0.to_string(), "+FLAGS.SILENT (\\Deleted)")).await?.try_collect().await?;
                let _: Vec<_> = with_timeout(s.uid_expunge(self.0.to_string())).await?.try_collect().await?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a local Dovecot test server with the mails from the seed script
    /// (see DEVELOPMENT.md): `cargo test imap_commands -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn test_imap_commands_against_dovecot() {
        let account = AccountConfig {
            id: "t".into(),
            label: "Test".into(),
            server: "127.0.0.1".into(),
            port: 31993,
            allow_invalid_certs: true,
            user: "test@maillet.example".into(),
            pass: "testpass".into(),
            ..Default::default()
        };
        let mut s = connect(&account).await.unwrap();
        let mb = s.select("INBOX").await.unwrap();
        let all: Vec<u32> = s.uid_search("ALL").await.unwrap().into_iter().collect();
        let unseen = s.uid_search("UNSEEN").await.unwrap();
        assert!(all.len() >= 4, "seed mails missing: {all:?}");
        let origin = |uid| Origin { account_id: "t", account: "Test", uid_validity: mb.uid_validity.unwrap_or(0), uid };
        let summaries = fetch_summaries(&mut s, &all, &origin).await.unwrap();
        assert_eq!(summaries.len(), all.len());
        let jana = summaries.values().find(|m| m.from_name.as_deref() == Some("Jana Wolff")).expect("mail from Jana");
        assert!(jana.preview.starts_with("Hallo, anbei die Spezifikation für"), "{}", jana.preview);
        assert!(jana.has_attachments);
        assert!(summaries.values().any(|m| m.bulk));
        // Fetching the preview must not mark anything as read.
        assert_eq!(s.uid_search("UNSEEN").await.unwrap().len(), unseen.len());

        let cmds = Commands::default();
        let content = cmds.content(&account, jana.uid).await.unwrap();
        assert!(content.text.contains("Login-Flow (v0.9)"));
        assert_eq!(content.attachments, vec!["Atlas-Spec-v0.9.pdf".to_string()]);

        cmds.set_seen(&account, jana.uid, true).await.unwrap();
        s.noop().await.unwrap();
        assert!(!s.uid_search("UNSEEN").await.unwrap().contains(&jana.uid));
        cmds.set_seen(&account, jana.uid, false).await.unwrap();
        s.noop().await.unwrap();
        assert!(s.uid_search("UNSEEN").await.unwrap().contains(&jana.uid));

        // The reused command connection must keep working.
        let folder = cmds.archive(&account, jana.uid).await.unwrap();
        assert_eq!(folder, "Archive");
        s.noop().await.unwrap();
        assert!(!s.uid_search("ALL").await.unwrap().contains(&jana.uid));

        let tom = summaries.values().find(|m| m.subject == "Mittag heute?").unwrap();
        cmds.delete(&account, tom.uid).await.unwrap();
        s.noop().await.unwrap();
        assert!(!s.uid_search("ALL").await.unwrap().contains(&tom.uid));
        s.select("Trash").await.unwrap();
        assert_eq!(s.uid_search("ALL").await.unwrap().len(), 1);
        s.logout().await.ok();
    }

    /// Needs a running Proton Mail Bridge: `cargo test -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn test_starttls_proton_bridge() {
        let mut account = AccountConfig {
            label: "Bridge".into(),
            server: "127.0.0.1".into(),
            port: 1143,
            security: Security::Starttls,
            user: "nobody@example.com".into(),
            pass: "wrong".into(),
            ..Default::default()
        };
        let err = connect(&account).await.err().unwrap().to_string();
        assert!(err.contains("self-signed certificate?"), "{err}");
        account.security = Security::Ssl;
        let err = connect(&account).await.err().unwrap().to_string();
        assert!(err.contains("expects STARTTLS"), "{err}");
        account.security = Security::Starttls;
        account.allow_invalid_certs = true;
        let err = connect(&account).await.err().unwrap().to_string();
        assert!(err.contains("Login failed"), "{err}");
    }
}
