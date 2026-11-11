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
/// Pinned (flagged) mails shown per account.
const PIN_LIMIT: usize = 50;
/// Bytes of the body fetched for the preview.
const PREVIEW_BYTES: u32 = 8192;
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
    /// The user's person/bulk corrections; they override the header detection.
    pub kinds: Arc<super::classify::SenderKinds>,
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
    let mut flagged: Vec<u32> = with_timeout(session.uid_search("FLAGGED")).await.context("SEARCH FLAGGED failed")?.into_iter().collect();
    flagged.sort_unstable();
    let pinned: Vec<u32> = flagged.iter().rev().take(PIN_LIMIT).copied().collect();

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

    let known = w.store.known(&account.id);
    let to_fetch: Vec<u32> = shown.iter().chain(fresh.iter()).chain(pinned.iter()).copied().filter(|uid| !known.contains_key(uid)).collect::<HashSet<_>>().into_iter().collect();
    let mut fetched = fetch_summaries(session, &to_fetch, &origin).await?;
    for mail in fetched.values_mut() {
        if let Some(bulk) = mail.from_addr.as_deref().and_then(|a| w.kinds.get(a)) {
            mail.bulk = bulk;
        }
    }

    for uid in &fresh {
        let Some(mail) = fetched.get_mut(uid) else { continue };
        let body = if w.hook.wants_body(account, mail) { Some(fetch_body(session, *uid).await?) } else { None };
        w.hook.on_new_mail(account, mail, body.as_ref()).await;
        w.last_uid.store(*uid, Ordering::Relaxed);
    }

    let pick = |uid: &u32| known.get(uid).or_else(|| fetched.get(uid)).cloned();
    let mails: Vec<MailSummary> = shown.iter().filter_map(pick).collect();
    let pinned_mails: Vec<MailSummary> = pinned.iter().filter_map(pick).collect();
    w.store.set_account(&account.id, &account.label, total, mails, pinned_mails);

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
        let query = format!("(UID FLAGS BODY.PEEK[HEADER] BODY.PEEK[TEXT]<0.{PREVIEW_BYTES}>)");
        let stream = with_timeout(session.uid_fetch(&set, &query)).await.context("FETCH failed")?;
        let fetches: Vec<_> = tokio::time::timeout(COMMAND_TIMEOUT, stream.try_collect())
            .await
            .context("Server did not respond – connection lost")??;
        for f in fetches {
            let Some(uid) = f.uid else { continue };
            let mut summary = parse::summarize(&origin(uid), f.header().unwrap_or(&[]), f.text().unwrap_or(&[]));
            for flag in f.flags() {
                match flag {
                    async_imap::types::Flag::Seen => summary.seen = true,
                    async_imap::types::Flag::Flagged => summary.flagged = true,
                    _ => {}
                }
            }
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
    /// Resolved folder names per account, e.g. "Aufgaben" → "Folders/Aufgaben" on Proton.
    folders: Mutex<HashMap<(String, String), String>>,
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

    /// Pins (`\Flagged`, the star in most mail clients) or unpins a mail.
    pub async fn set_flagged(&self, account: &AccountConfig, uid: u32, flagged: bool) -> Result<()> {
        self.run(account, SetFlag(uid, "\\Flagged", flagged)).await
    }

    /// Reads a mail found by Message-ID in one of `folders` (e.g. after it was archived).
    pub async fn content_by_message_id(&self, account: &AccountConfig, folders: Vec<String>, message_id: &str) -> Result<MailContent> {
        self.run(account, ContentById { folders, message_id: message_id.to_string(), mailbox: account.mailbox.clone() }).await
    }

    /// Finds a folder by its last name part ("Aufgaben" matches "Folders/Aufgaben");
    /// with `create`, creates it if missing – at the top level, or below a
    /// "Folders" parent where the server allows no own top-level folders (Proton).
    pub async fn resolve_folder(&self, account: &AccountConfig, name: &str, create: bool) -> Result<Option<String>> {
        let key = (account.id.clone(), name.to_string());
        if let Some(found) = self.folders.lock().await.get(&key) {
            return Ok(Some(found.clone()));
        }
        let found = self.run(account, ResolveFolder { name: name.to_string(), create }).await?;
        if let Some(f) = &found {
            self.folders.lock().await.insert(key, f.clone());
        }
        Ok(found)
    }

    /// The subfolder `child` of `parent` (full name), created on request.
    pub async fn resolve_child(&self, account: &AccountConfig, parent: &str, child: &str, create: bool) -> Result<Option<String>> {
        let key = (account.id.clone(), format!("{parent}\u{0}{child}"));
        if let Some(found) = self.folders.lock().await.get(&key) {
            return Ok(Some(found.clone()));
        }
        let found = self.run(account, ResolveChild { parent: parent.to_string(), child: child.to_string(), create }).await?;
        if let Some(f) = &found {
            self.folders.lock().await.insert(key, f.clone());
        }
        Ok(found)
    }

    /// Moves a mail of the watched mailbox to `folder`, creating the folder if needed.
    pub async fn move_to_folder(&self, account: &AccountConfig, uid: u32, folder: &str) -> Result<()> {
        self.run(account, MoveUid { uid, folder: folder.to_string() }).await
    }

    /// Mails in another folder (e.g. the task folder); empty if it does not exist.
    pub async fn folder_mails(&self, account: &AccountConfig, folder: &str, limit: usize) -> Result<Vec<MailSummary>> {
        Ok(self.folder_mails_since(account, folder, None, limit).await?.0)
    }

    /// Mails received in the last `days` days (newest `limit` at most), filtered
    /// on the server. Also tells whether older mails exist.
    pub async fn folder_mails_since(&self, account: &AccountConfig, folder: &str, days: Option<u32>, limit: usize) -> Result<(Vec<MailSummary>, bool)> {
        let since = days.map(imap_date_days_ago);
        self.run(account, FolderMails { folder: folder.to_string(), limit, since, account_id: account.id.clone(), label: account.label.clone(), mailbox: account.mailbox.clone() }).await
    }

    /// Moves a mail found by Message-ID from `from` to `to` (`None` = the archive
    /// folder, then marked as read). Returns the target folder.
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn move_by_message_id(&self, account: &AccountConfig, from: &str, message_id: &str, to: Option<&str>) -> Result<String> {
        self.move_found(account, &[from.to_string()], message_id, to).await
    }

    /// Like [`Self::move_by_message_id`], but looks for the mail in several
    /// folders in order – a server may report a move with delay, so the mail
    /// may not be where the window last saw it. Already in the target: done.
    pub async fn move_found(&self, account: &AccountConfig, from: &[String], message_id: &str, to: Option<&str>) -> Result<String> {
        self.run(account, MoveById { from: from.to_vec(), message_id: message_id.to_string(), to: to.map(str::to_string), mailbox: account.mailbox.clone() }).await
    }

    /// Opens the command connection ahead of time, or keeps it alive with a
    /// NOOP, so that the first click in the window does not wait for a login.
    pub async fn keep_warm(&self, account: &AccountConfig) -> Result<()> {
        self.run(account, Noop).await
    }

    /// Moves archived mails (found by Message-ID) back to the watched mailbox
    /// as unread. Returns how many were found and moved.
    pub async fn unarchive(&self, account: &AccountConfig, folder: &str, message_ids: &[String]) -> Result<usize> {
        self.run(account, Unarchive { folder: folder.to_string(), message_ids: message_ids.to_vec(), mailbox: account.mailbox.clone() }).await
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

/// Creates a folder; "already exists" is fine.
async fn ensure_folder(s: &mut Session, folder: &str) -> Result<()> {
    match with_timeout(s.create(folder)).await {
        Ok(()) => Ok(()),
        Err(e) if already_exists(&e) => Ok(()),
        // Some servers refuse CREATE for existing special folders (e.g. INBOX) without ALREADYEXISTS.
        Err(e) => {
            let list: Vec<_> = with_timeout(s.list(Some(""), Some("*"))).await?.try_collect().await?;
            if list.iter().any(|n| n.name() == folder) { Ok(()) } else { Err(e).with_context(|| format!("Failed to create folder '{folder}'")) }
        }
    }
}

struct MoveUid {
    uid: u32,
    folder: String,
}
impl AsyncOp<()> for MoveUid {
    async fn call(&self, s: &mut Session) -> Result<()> {
        move_to(s, self.uid, &self.folder).await
    }
}

struct ResolveChild {
    parent: String,
    child: String,
    create: bool,
}
impl AsyncOp<Option<String>> for ResolveChild {
    async fn call(&self, s: &mut Session) -> Result<Option<String>> {
        let list: Vec<_> = with_timeout(s.list(Some(""), Some("*"))).await?.try_collect().await?;
        let delim = list.iter().find(|n| n.name() == self.parent).and_then(|n| n.delimiter()).unwrap_or("/").to_string();
        let full = format!("{}{delim}{}", self.parent, self.child);
        if list.iter().any(|n| n.name() == full) {
            return Ok(Some(full));
        }
        if !self.create {
            return Ok(None);
        }
        match with_timeout(s.create(&full)).await {
            Ok(()) => Ok(Some(full)),
            // Already there (e.g. created meanwhile by another client) – just as good.
            Err(e) if already_exists(&e) => Ok(Some(full)),
            Err(e) => Err(e).with_context(|| format!("Failed to create folder '{full}'")),
        }
    }
}

fn already_exists(e: &anyhow::Error) -> bool {
    let msg = format!("{e:#}").to_lowercase();
    msg.contains("alreadyexists") || msg.contains("already exists")
}

struct ResolveFolder {
    name: String,
    create: bool,
}
impl AsyncOp<Option<String>> for ResolveFolder {
    async fn call(&self, s: &mut Session) -> Result<Option<String>> {
        let list: Vec<_> = with_timeout(s.list(Some(""), Some("*"))).await?.try_collect().await?;
        let wanted = self.name.to_lowercase();
        let leaf = |n: &async_imap::types::Name| {
            let d = n.delimiter().unwrap_or("/");
            n.name().rsplit(d).next().unwrap_or("").to_lowercase()
        };
        if let Some(n) = list.iter().find(|n| leaf(n) == wanted && !n.attributes().contains(&NameAttribute::NoSelect)) {
            return Ok(Some(n.name().to_string()));
        }
        if !self.create {
            return Ok(None);
        }
        let top_level = with_timeout(s.create(&self.name)).await;
        if top_level.is_ok() || top_level.as_ref().is_err_and(already_exists) {
            return Ok(Some(self.name.clone()));
        }
        // Proton Mail Bridge: own folders only below "Folders/".
        if let Some(parent) = list.iter().find(|n| n.name().eq_ignore_ascii_case("Folders")) {
            let full = format!("{}{}{}", parent.name(), parent.delimiter().unwrap_or("/"), self.name);
            with_timeout(s.create(&full)).await.with_context(|| format!("Failed to create folder '{full}'"))?;
            return Ok(Some(full));
        }
        top_level.map(|_| None).with_context(|| format!("Failed to create folder '{}'", self.name))
    }
}

/// IMAP date (`10-Nov-2026`) of the day `days` days ago (UTC).
pub fn imap_date_days_ago(days: u32) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    imap_date(now / 86_400 - days as i64)
}

/// Days since 1970-01-01 → `DD-Mon-YYYY` (civil calendar, H. Hinnant's algorithm).
fn imap_date(days_since_epoch: i64) -> String {
    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{day:02}-{}-{year}", MONTHS[(month - 1) as usize])
}

struct FolderMails {
    folder: String,
    limit: usize,
    /// IMAP date: only mails received since then.
    since: Option<String>,
    account_id: String,
    label: String,
    mailbox: String,
}
impl AsyncOp<(Vec<MailSummary>, bool)> for FolderMails {
    async fn call(&self, s: &mut Session) -> Result<(Vec<MailSummary>, bool)> {
        // The watched mailbox is already selected on a command connection, but a
        // fresh SELECT is still needed for current UIDVALIDITY and EXISTS.
        let other = self.folder != self.mailbox;
        let Ok(mb) = with_timeout(s.select(&self.folder)).await else {
            if other {
                with_timeout(s.select(&self.mailbox)).await?;
            }
            return Ok((vec![], false));
        };
        let result = async {
            let query = self.since.as_ref().map_or("ALL".to_string(), |d| format!("SINCE {d}"));
            let mut uids: Vec<u32> = with_timeout(s.uid_search(&query)).await?.into_iter().collect();
            // More mails in the folder than found since the date: older ones exist.
            // Saves a second SEARCH round trip.
            let older = (mb.exists as usize) > uids.len();
            uids.sort_unstable();
            let newest: Vec<u32> = uids.iter().rev().take(self.limit).copied().collect();
            let uv = mb.uid_validity.unwrap_or(0);
            let origin = |uid| Origin { account_id: &self.account_id, account: &self.label, uid_validity: uv, uid };
            let mut mails: Vec<MailSummary> = fetch_summaries(s, &newest, &origin).await?.into_values().collect();
            mails.sort_by_key(|m| std::cmp::Reverse(m.uid));
            Ok::<_, anyhow::Error>((mails, older || uids.len() > self.limit))
        }
        .await;
        if other {
            with_timeout(s.select(&self.mailbox)).await?;
        }
        result
    }
}

struct Noop;
impl AsyncOp<()> for Noop {
    async fn call(&self, s: &mut Session) -> Result<()> {
        with_timeout(s.noop()).await?;
        Ok(())
    }
}

struct MoveById {
    from: Vec<String>,
    message_id: String,
    to: Option<String>,
    mailbox: String,
}
impl AsyncOp<String> for MoveById {
    async fn call(&self, s: &mut Session) -> Result<String> {
        let target = match &self.to {
            Some(f) => {
                ensure_folder(s, f).await?;
                f.clone()
            }
            None => find_folder(s, &[NameAttribute::Archive, NameAttribute::All], &["archive", "archiv", "archives"], &self.from[0])
                .await?
                .context("No archive folder found on the server – create a folder named \"Archive\"")?,
        };
        let id = self.message_id.trim_matches(['<', '>']).replace('"', "");
        let result = async {
            for folder in &self.from {
                if with_timeout(s.select(folder)).await.is_err() {
                    continue;
                }
                let Some(uid) = with_timeout(s.uid_search(format!("HEADER Message-ID \"{id}\""))).await?.into_iter().max() else { continue };
                if *folder == target {
                    return Ok(());
                }
                if self.to.is_none() {
                    SetSeen(uid, true).call(s).await?;
                }
                return move_to(s, uid, &target).await;
            }
            // Not in any source: maybe it already reached the target.
            if with_timeout(s.select(&target)).await.is_ok() && !with_timeout(s.uid_search(format!("HEADER Message-ID \"{id}\""))).await?.is_empty() {
                return Ok(());
            }
            bail!("Mail not found – it may have been moved or deleted elsewhere")
        }
        .await;
        with_timeout(s.select(&self.mailbox)).await?;
        result.map(|_| target)
    }
}

struct SetFlag(u32, &'static str, bool);
impl AsyncOp<()> for SetFlag {
    async fn call(&self, s: &mut Session) -> Result<()> {
        let op = format!("{}FLAGS.SILENT ({})", if self.2 { "+" } else { "-" }, self.1);
        let stream = with_timeout(s.uid_store(self.0.to_string(), &op)).await.context("STORE failed")?;
        let _: Vec<_> = stream.try_collect().await?;
        Ok(())
    }
}

struct ContentById {
    folders: Vec<String>,
    message_id: String,
    mailbox: String,
}
impl AsyncOp<MailContent> for ContentById {
    async fn call(&self, s: &mut Session) -> Result<MailContent> {
        let id = self.message_id.trim_matches(['<', '>']).replace('"', "");
        let mut found = None;
        for folder in &self.folders {
            if with_timeout(s.select(folder)).await.is_err() {
                continue;
            }
            let uids = with_timeout(s.uid_search(format!("HEADER Message-ID \"{id}\""))).await?;
            if let Some(uid) = uids.into_iter().max() {
                found = Some(fetch_raw(s, uid).await);
                break;
            }
        }
        // The command connection is expected to have the watched mailbox selected.
        with_timeout(s.select(&self.mailbox)).await?;
        match found {
            Some(raw) => Ok(parse::parse_content(&raw?)),
            None => bail!("Mail not found – it may have been deleted or moved"),
        }
    }
}

struct Unarchive {
    folder: String,
    message_ids: Vec<String>,
    mailbox: String,
}
impl AsyncOp<usize> for Unarchive {
    async fn call(&self, s: &mut Session) -> Result<usize> {
        with_timeout(s.select(&self.folder)).await.with_context(|| format!("Failed to open '{}'", self.folder))?;
        let mut moved = 0;
        let result = async {
            for id in &self.message_ids {
                let id = id.trim_matches(['<', '>']).replace('"', "");
                let uids = with_timeout(s.uid_search(format!("HEADER Message-ID \"{id}\""))).await?;
                for uid in uids {
                    SetSeen(uid, false).call(s).await?;
                    move_to(s, uid, &self.mailbox).await?;
                    moved += 1;
                }
            }
            Ok::<_, anyhow::Error>(())
        }
        .await;
        // The command connection is expected to have the watched mailbox selected.
        with_timeout(s.select(&self.mailbox)).await?;
        result.map(|_| moved)
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
        assert!(summaries.values().find(|m| m.subject == "Bereits gelesen").unwrap().seen, "\\Seen flag is fetched");
        assert!(!jana.seen || !unseen.is_empty());
        // Fetching the preview must not mark anything as read.
        assert_eq!(s.uid_search("UNSEEN").await.unwrap().len(), unseen.len());

        let cmds = Commands::default();
        // Period filter on the server: all seed mails are from today, nothing older.
        let (recent, older) = cmds.folder_mails_since(&account, "INBOX", Some(7), 500).await.unwrap();
        assert_eq!(recent.len(), all.len());
        assert!(!older);
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

        // Undo: found again by Message-ID, back in the inbox and unread.
        let mid = jana.message_id.clone().expect("seed mail has a Message-ID");
        assert_eq!(cmds.unarchive(&account, "Archive", &[mid]).await.unwrap(), 1);
        s.noop().await.unwrap();
        let unseen_now = s.uid_search("UNSEEN").await.unwrap();
        let jana_back = s.uid_search("FROM \"jana.wolff@firma.example\"").await.unwrap();
        assert_eq!(jana_back.len(), 1);
        assert!(unseen_now.contains(jana_back.iter().next().unwrap()));
        // The command connection still works on the inbox afterwards.
        cmds.set_seen(&account, *jana_back.iter().next().unwrap(), true).await.unwrap();

        let tom = summaries.values().find(|m| m.subject == "Mittag heute?").unwrap();
        cmds.delete(&account, tom.uid).await.unwrap();
        s.noop().await.unwrap();
        assert!(!s.uid_search("ALL").await.unwrap().contains(&tom.uid));
        s.select("Trash").await.unwrap();
        assert_eq!(s.uid_search("ALL").await.unwrap().len(), 1);
        s.logout().await.ok();
    }

    /// Needs the local Dovecot test server (see DEVELOPMENT.md):
    /// `cargo test tasks_against -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn test_pin_and_tasks_against_dovecot() {
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
        let origin = |uid| Origin { account_id: "t", account: "Test", uid_validity: mb.uid_validity.unwrap_or(0), uid };
        let summaries = fetch_summaries(&mut s, &all, &origin).await.unwrap();
        let malte = summaries.values().find(|m| m.from_addr.as_deref() == Some("m.mueller@nordwind.example")).unwrap().clone();
        let mid = malte.message_id.clone().unwrap();
        let cmds = Commands::default();

        // Pin = \Flagged
        cmds.set_flagged(&account, malte.uid, true).await.unwrap();
        s.noop().await.unwrap();
        assert!(s.uid_search("FLAGGED").await.unwrap().contains(&malte.uid));
        cmds.set_flagged(&account, malte.uid, false).await.unwrap();
        s.noop().await.unwrap();
        assert!(!s.uid_search("FLAGGED").await.unwrap().contains(&malte.uid));

        // Task: into the folder (created on demand), listed, readable there.
        assert_eq!(cmds.resolve_folder(&account, "Aufgaben", false).await.unwrap(), None);
        let folder = cmds.resolve_folder(&account, "Aufgaben", true).await.unwrap().unwrap();
        assert_eq!(folder, "Aufgaben");
        cmds.move_to_folder(&account, malte.uid, &folder).await.unwrap();
        s.noop().await.unwrap();
        assert!(!s.uid_search("ALL").await.unwrap().contains(&malte.uid));
        let tasks = cmds.folder_mails(&account, "Aufgaben", 100).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].message_id.as_deref(), Some(mid.as_str()));
        // States as subfolders: open → in progress → back to open.
        assert_eq!(cmds.resolve_child(&account, &folder, "In Arbeit", false).await.unwrap(), None);
        let progress = cmds.resolve_child(&account, &folder, "In Arbeit", true).await.unwrap().unwrap();
        assert_eq!(progress, "Aufgaben/In Arbeit");
        cmds.move_by_message_id(&account, &folder, &mid, Some(&progress)).await.unwrap();
        assert!(cmds.folder_mails(&account, &folder, 100).await.unwrap().is_empty(), "subfolder mails are not listed with the parent");
        assert_eq!(cmds.folder_mails(&account, &progress, 100).await.unwrap().len(), 1);
        // Stale view: the window thinks the mail is still open (base folder) – found anyway.
        cmds.move_found(&account, &[folder.clone(), progress.clone()], &mid, Some(&folder)).await.unwrap();
        assert_eq!(cmds.folder_mails(&account, &folder, 100).await.unwrap().len(), 1);
        // Already in the target: success without moving.
        cmds.move_found(&account, &[progress.clone()], &mid, Some(&folder)).await.unwrap();
        let text = cmds.content_by_message_id(&account, vec!["Aufgaben".into(), "INBOX".into()], &mid).await.unwrap().text;
        assert!(text.contains("drei Rückfragen"));

        // Done → archive, undo → back into the task folder, then back to the inbox.
        let folder = cmds.move_by_message_id(&account, "Aufgaben", &mid, None).await.unwrap();
        assert_eq!(folder, "Archive");
        assert!(cmds.folder_mails(&account, "Aufgaben", 100).await.unwrap().is_empty());
        cmds.move_by_message_id(&account, &folder, &mid, Some("Aufgaben")).await.unwrap();
        assert_eq!(cmds.folder_mails(&account, "Aufgaben", 100).await.unwrap().len(), 1);
        cmds.move_by_message_id(&account, "Aufgaben", &mid, Some("INBOX")).await.unwrap();
        s.noop().await.unwrap();
        assert_eq!(s.uid_search(format!("HEADER Message-ID \"{}\"", mid.trim_matches(['<', '>']))).await.unwrap().len(), 1);
        // A missing folder lists as empty, not as error.
        assert!(cmds.folder_mails(&account, "GibtEsNicht", 10).await.unwrap().is_empty());
        s.logout().await.ok();
    }

    #[test]
    fn test_imap_date() {
        assert_eq!(imap_date(0), "01-Jan-1970");
        assert_eq!(imap_date(20_402), "10-Nov-2025");
        assert_eq!(imap_date(19_782), "29-Feb-2024");
        assert_eq!(imap_date(20_768), "11-Nov-2026");
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
