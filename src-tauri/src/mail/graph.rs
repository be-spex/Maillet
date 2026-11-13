//! Microsoft 365 and Outlook.com via Microsoft Graph, for mailboxes without
//! IMAP access. Offers the same operations as the IMAP command connection, so
//! the app does not care where a mail comes from.
//!
//! - **Sign-in:** OAuth device code flow with the user's own app registration.
//!   The user confirms in the browser; whether they may consent themselves is
//!   decided by their organisation. The refresh token lives in the keyring.
//! - **Ids:** Graph ids are strings; the app works with numeric uids. Each
//!   Graph id gets a uid on first sight. Immutable ids keep a mail's id when
//!   it moves between folders.
//! - **Watching:** Graph has no IDLE for desktop apps; the inbox is polled.
//! - **Folders:** addressed by path ("Tasks/In Progress"), like on IMAP.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::imap::Watch;
use super::model::{MailBody, MailContent, MailSummary};
use super::parse::{self, Origin};
use crate::base::config::{self, AccountConfig};
use crate::base::{log, status};

/// Delegated permissions Maillet asks for.
pub const SCOPES: &str = "offline_access User.Read Mail.ReadWrite";
/// How often the inbox is checked; "Check now" checks at once.
const POLL_INTERVAL: Duration = Duration::from_secs(60);
const LIST_LIMIT: usize = 200;
const PIN_LIMIT: usize = 50;
/// Newest inbox mails looked at for new arrivals on every check.
const FRESH_WINDOW: usize = 25;
const SELECT: &str = "id,subject,from,receivedDateTime,bodyPreview,hasAttachments,isRead,flag,internetMessageId,internetMessageHeaders,parentFolderId";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// One client for all Graph accounts.
pub fn shared() -> &'static Graph {
    static GRAPH: LazyLock<Graph> = LazyLock::new(|| Graph::new("https://login.microsoftonline.com", "https://graph.microsoft.com/v1.0"));
    &GRAPH
}

struct Token {
    access: String,
    expires: Instant,
    /// The newest refresh token (they rotate).
    refresh: String,
}

#[derive(Default)]
struct IdMap {
    by_uid: HashMap<u32, String>,
    by_id: HashMap<String, u32>,
    next: u32,
}

pub struct Graph {
    http: reqwest::Client,
    login_base: String,
    api_base: String,
    tokens: Mutex<HashMap<String, Token>>,
    ids: Mutex<HashMap<String, IdMap>>,
    /// (account, folder path) → Graph folder id.
    folders: Mutex<HashMap<(String, String), String>>,
}

/// What the device code flow shows the user.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub message: String,
    pub device_code: String,
    #[serde(default = "default_interval")]
    pub interval: u64,
    #[serde(default)]
    pub expires_in: u64,
}

fn default_interval() -> u64 {
    5
}

/// Result of a successful sign-in.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SignIn {
    #[serde(skip_serializing)]
    pub refresh_token: String,
    pub email: String,
    pub name: String,
}

impl Graph {
    pub fn new(login_base: &str, api_base: &str) -> Self {
        Self {
            http: reqwest::Client::builder().timeout(REQUEST_TIMEOUT).build().expect("HTTP client"),
            login_base: login_base.trim_end_matches('/').to_string(),
            api_base: api_base.trim_end_matches('/').to_string(),
            tokens: Mutex::default(),
            ids: Mutex::default(),
            folders: Mutex::default(),
        }
    }

    // ------------------------------------------------------------ sign-in

    fn token_url(&self, tenant: &str) -> String {
        format!("{}/{}/oauth2/v2.0/token", self.login_base, tenant_or_default(tenant))
    }

    /// Step 1: a code the user enters on the Microsoft page.
    pub async fn start_device_login(&self, client_id: &str, tenant: &str) -> Result<DeviceCode> {
        let url = format!("{}/{}/oauth2/v2.0/devicecode", self.login_base, tenant_or_default(tenant));
        let resp = self.http.post(url).form(&[("client_id", client_id), ("scope", SCOPES)]).send().await.context("Microsoft sign-in is not reachable")?;
        let body: Value = resp.json().await.context("Unexpected answer from Microsoft")?;
        if let Some(e) = oauth_error(&body) {
            bail!(e);
        }
        Ok(serde_json::from_value(body)?)
    }

    /// Step 2: waits until the user has confirmed (or refused) in the browser.
    pub async fn finish_device_login(&self, client_id: &str, tenant: &str, code: &DeviceCode) -> Result<SignIn> {
        let deadline = Instant::now() + Duration::from_secs(code.expires_in.max(60));
        let mut interval = Duration::from_secs(code.interval.max(1));
        loop {
            tokio::time::sleep(interval).await;
            if Instant::now() > deadline {
                bail!("The code has expired – start the sign-in again");
            }
            let form = [("grant_type", "urn:ietf:params:oauth:grant-type:device_code"), ("client_id", client_id), ("device_code", code.device_code.as_str())];
            let body: Value = self.http.post(self.token_url(tenant)).form(&form).send().await?.json().await?;
            match body.get("error").and_then(Value::as_str) {
                Some("authorization_pending") => continue,
                Some("slow_down") => {
                    interval += Duration::from_secs(5);
                    continue;
                }
                Some(_) => bail!(oauth_error(&body).unwrap_or_default()),
                None => {}
            }
            let access = str_field(&body, "access_token").context("No access token in the answer")?;
            let refresh_token = str_field(&body, "refresh_token").context("No refresh token – is offline_access allowed?")?;
            let me: Value = self.http.get(format!("{}/me?$select=mail,userPrincipalName,displayName", self.api_base)).bearer_auth(&access).send().await?.json().await?;
            let email = str_field(&me, "mail").or_else(|| str_field(&me, "userPrincipalName")).unwrap_or_default();
            return Ok(SignIn { refresh_token, email, name: str_field(&me, "displayName").unwrap_or_default() });
        }
    }

    /// A valid access token; renews it with the refresh token when needed.
    async fn token(&self, account: &AccountConfig) -> Result<String> {
        let mut tokens = self.tokens.lock().await;
        if let Some(t) = tokens.get(&account.id) {
            if t.expires > Instant::now() + Duration::from_secs(60) {
                return Ok(t.access.clone());
            }
        }
        let refresh = tokens.get(&account.id).map(|t| t.refresh.clone()).unwrap_or_else(|| account.pass.clone());
        if refresh.is_empty() {
            bail!("Not signed in – edit the account and sign in again");
        }
        let form = [("grant_type", "refresh_token"), ("client_id", account.client_id.as_str()), ("refresh_token", refresh.as_str()), ("scope", SCOPES)];
        let body: Value = self.http.post(self.token_url(&account.tenant)).form(&form).send().await.context("Microsoft sign-in is not reachable")?.json().await?;
        if let Some(e) = oauth_error(&body) {
            bail!("Sign-in expired or revoked – edit the account and sign in again ({e})");
        }
        let access = str_field(&body, "access_token").context("No access token in the answer")?;
        let expires = Instant::now() + Duration::from_secs(body.get("expires_in").and_then(Value::as_u64).unwrap_or(3600));
        let renewed = str_field(&body, "refresh_token").unwrap_or(refresh.clone());
        if renewed != refresh {
            let (id, secret) = (account.id.clone(), renewed.clone());
            if let Err(e) = config::off_thread(move || config::store_secret(&id, &secret)).await {
                tracing::warn!("[{}] could not save the renewed sign-in: {e:#}", account.label);
            }
        }
        tokens.insert(account.id.clone(), Token { access: access.clone(), expires, refresh: renewed });
        Ok(access)
    }

    /// Forgets the sign-in of an account (removed or signed in again).
    pub async fn forget(&self, account_id: &str) {
        self.tokens.lock().await.remove(account_id);
        self.folders.lock().await.retain(|(a, _), _| a != account_id);
    }

    // ------------------------------------------------------------ HTTP

    async fn request(&self, account: &AccountConfig, method: reqwest::Method, url: &str, body: Option<Value>) -> Result<reqwest::Response> {
        let url = if url.starts_with("http") { url.to_string() } else { format!("{}{url}", self.api_base) };
        for attempt in 0..2 {
            let token = self.token(account).await?;
            let mut req = self.http.request(method.clone(), &url).bearer_auth(token).header("Prefer", "IdType=\"ImmutableId\"");
            if let Some(b) = &body {
                req = req.json(b);
            }
            let resp = req.send().await.with_context(|| format!("Microsoft Graph is not reachable ({})", host(&url)))?;
            if resp.status() == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
                self.tokens.lock().await.get_mut(&account.id).map(|t| t.expires = Instant::now());
                continue;
            }
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let msg = serde_json::from_str::<Value>(&text)
                    .ok()
                    .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or(text);
                bail!("Microsoft Graph: {msg} ({status})");
            }
            return Ok(resp);
        }
        unreachable!()
    }

    async fn call(&self, account: &AccountConfig, method: reqwest::Method, url: &str, body: Option<Value>) -> Result<Value> {
        let resp = self.request(account, method, url, body).await?;
        if resp.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        let text = resp.text().await?;
        Ok(if text.trim().is_empty() { Value::Null } else { serde_json::from_str(&text)? })
    }

    async fn get(&self, account: &AccountConfig, url: &str) -> Result<Value> {
        self.call(account, reqwest::Method::GET, url, None).await
    }

    /// All items of a list, following the pages, up to `limit`.
    async fn list(&self, account: &AccountConfig, url: &str, limit: usize) -> Result<Vec<Value>> {
        let mut out = vec![];
        let mut next = Some(url.to_string());
        while let Some(u) = next.take() {
            let page = self.get(account, &u).await?;
            out.extend(page.get("value").and_then(Value::as_array).cloned().unwrap_or_default());
            if out.len() >= limit {
                out.truncate(limit);
                break;
            }
            next = str_field(&page, "@odata.nextLink");
        }
        Ok(out)
    }

    // ------------------------------------------------------------ ids

    async fn uid_for(&self, account_id: &str, graph_id: &str) -> u32 {
        let mut ids = self.ids.lock().await;
        let map = ids.entry(account_id.to_string()).or_default();
        if let Some(uid) = map.by_id.get(graph_id) {
            return *uid;
        }
        map.next += 1;
        let uid = map.next;
        map.by_id.insert(graph_id.to_string(), uid);
        map.by_uid.insert(uid, graph_id.to_string());
        uid
    }

    async fn graph_id(&self, account_id: &str, uid: u32) -> Result<String> {
        self.ids
            .lock()
            .await
            .get(account_id)
            .and_then(|m| m.by_uid.get(&uid).cloned())
            .ok_or_else(|| anyhow!("Mail no longer known – reload the list"))
    }

    async fn highest_uid(&self, account_id: &str) -> u32 {
        self.ids.lock().await.get(account_id).map_or(0, |m| m.next)
    }

    async fn summary(&self, account: &AccountConfig, v: &Value) -> MailSummary {
        let uid = self.uid_for(&account.id, &str_field(v, "id").unwrap_or_default()).await;
        to_summary(account, uid, v)
    }

    async fn summaries(&self, account: &AccountConfig, items: &[Value]) -> Vec<MailSummary> {
        let mut out = Vec::with_capacity(items.len());
        for v in items {
            out.push(self.summary(account, v).await);
        }
        out
    }

    // ------------------------------------------------------------ folders

    fn is_inbox(account: &AccountConfig, path: &str) -> bool {
        path.eq_ignore_ascii_case("inbox") || path == account.mailbox
    }

    /// Folder id of a path; with `create`, missing parts are created.
    async fn folder_id(&self, account: &AccountConfig, path: &str, create: bool) -> Result<Option<String>> {
        if Self::is_inbox(account, path) {
            return Ok(Some("inbox".into()));
        }
        if path.eq_ignore_ascii_case("archive") {
            return Ok(Some("archive".into()));
        }
        let key = (account.id.clone(), path.to_lowercase());
        if let Some(id) = self.folders.lock().await.get(&key) {
            return Ok(Some(id.clone()));
        }
        let mut parent: Option<String> = None;
        for part in path.split('/').filter(|p| !p.is_empty()) {
            let base = match &parent {
                None => "/me/mailFolders".to_string(),
                Some(id) => format!("/me/mailFolders/{id}/childFolders"),
            };
            let found = self.get(account, &format!("{base}?$filter=displayName eq '{}'&$select=id,displayName", odata(part))).await?;
            let id = match found.pointer("/value/0/id").and_then(Value::as_str) {
                Some(id) => id.to_string(),
                None if create => {
                    let made = self.call(account, reqwest::Method::POST, &base, Some(json!({ "displayName": part }))).await?;
                    str_field(&made, "id").context("Folder could not be created")?
                }
                None => return Ok(None),
            };
            parent = Some(id);
        }
        if let Some(id) = &parent {
            self.folders.lock().await.insert(key, id.clone());
        }
        Ok(parent)
    }

    pub async fn resolve_folder(&self, account: &AccountConfig, name: &str, create: bool) -> Result<Option<String>> {
        Ok(self.folder_id(account, name, create).await?.map(|_| name.to_string()))
    }

    pub async fn resolve_child(&self, account: &AccountConfig, parent: &str, child: &str, create: bool) -> Result<Option<String>> {
        let path = format!("{parent}/{child}");
        Ok(self.folder_id(account, &path, create).await?.map(|_| path))
    }

    /// Folder paths (two levels deep), for choosing a category target.
    pub async fn list_folders(&self, account: &AccountConfig) -> Result<Vec<String>> {
        let top = self.list(account, "/me/mailFolders?$select=id,displayName,childFolderCount&$top=100", 500).await?;
        let mut out = vec![];
        for f in &top {
            let name = str_field(f, "displayName").unwrap_or_default();
            out.push(name.clone());
            if f.get("childFolderCount").and_then(Value::as_u64).unwrap_or(0) > 0 {
                let id = str_field(f, "id").unwrap_or_default();
                for c in self.list(account, &format!("/me/mailFolders/{id}/childFolders?$select=displayName&$top=100"), 500).await? {
                    out.push(format!("{name}/{}", str_field(&c, "displayName").unwrap_or_default()));
                }
            }
        }
        out.sort();
        Ok(out)
    }

    // ------------------------------------------------------------ mails

    async fn raw(&self, account: &AccountConfig, graph_id: &str) -> Result<Vec<u8>> {
        let resp = self.request(account, reqwest::Method::GET, &format!("/me/messages/{graph_id}/$value"), None).await?;
        Ok(resp.bytes().await?.to_vec())
    }

    pub async fn content(&self, account: &AccountConfig, uid: u32) -> Result<MailContent> {
        let id = self.graph_id(&account.id, uid).await?;
        Ok(parse::parse_content(&self.raw(account, &id).await?))
    }

    async fn body(&self, account: &AccountConfig, uid: u32) -> Result<MailBody> {
        let id = self.graph_id(&account.id, uid).await?;
        Ok(parse::parse_body(&self.raw(account, &id).await?))
    }

    async fn patch(&self, account: &AccountConfig, graph_id: &str, body: Value) -> Result<()> {
        self.call(account, reqwest::Method::PATCH, &format!("/me/messages/{graph_id}"), Some(body)).await.map(|_| ())
    }

    async fn move_id(&self, account: &AccountConfig, graph_id: &str, folder_id: &str) -> Result<()> {
        self.call(account, reqwest::Method::POST, &format!("/me/messages/{graph_id}/move"), Some(json!({ "destinationId": folder_id }))).await.map(|_| ())
    }

    pub async fn set_seen(&self, account: &AccountConfig, uid: u32, seen: bool) -> Result<()> {
        let id = self.graph_id(&account.id, uid).await?;
        self.patch(account, &id, json!({ "isRead": seen })).await
    }

    pub async fn set_flagged(&self, account: &AccountConfig, uid: u32, flagged: bool) -> Result<()> {
        let id = self.graph_id(&account.id, uid).await?;
        self.patch(account, &id, json!({ "flag": { "flagStatus": if flagged { "flagged" } else { "notFlagged" } } })).await
    }

    pub async fn archive(&self, account: &AccountConfig, uid: u32) -> Result<String> {
        let id = self.graph_id(&account.id, uid).await?;
        self.patch(account, &id, json!({ "isRead": true })).await?;
        self.move_id(account, &id, "archive").await?;
        Ok("Archive".into())
    }

    pub async fn delete(&self, account: &AccountConfig, uid: u32) -> Result<()> {
        let id = self.graph_id(&account.id, uid).await?;
        self.move_id(account, &id, "deleteditems").await
    }

    /// Moves a mail out of the inbox (dealt with: read).
    pub async fn move_to_folder(&self, account: &AccountConfig, uid: u32, folder: &str) -> Result<()> {
        let id = self.graph_id(&account.id, uid).await?;
        let target = self.folder_id(account, folder, true).await?.context("Folder could not be created")?;
        self.patch(account, &id, json!({ "isRead": true })).await?;
        self.move_id(account, &id, &target).await
    }

    /// Mails of a folder, newest first; `has_older` if more exist before `days`.
    pub async fn folder_mails_since(&self, account: &AccountConfig, folder: &str, days: Option<u32>, limit: usize) -> Result<(Vec<MailSummary>, bool)> {
        let Some(fid) = self.folder_id(account, folder, false).await? else { return Ok((vec![], false)) };
        let since = days.map(iso_days_ago);
        // Graph wants a property of $orderby also in $filter.
        let filter = format!("receivedDateTime ge {}", since.as_deref().unwrap_or("1900-01-01T00:00:00Z"));
        let base = format!("/me/mailFolders/{fid}/messages?$select={SELECT}&$filter={filter}&$orderby=receivedDateTime desc&$top={}", limit.clamp(1, 100));
        let items = self.list(account, &base, limit + 1).await?;
        let more = items.len() > limit;
        let mut mails = self.summaries(account, &items[..items.len().min(limit)]).await;
        mails.sort_by_key(|m| std::cmp::Reverse(m.date.unwrap_or(0)));
        let older = match &since {
            Some(d) => more || !self.get(account, &format!("/me/mailFolders/{fid}/messages?$select=id&$filter=receivedDateTime lt {d}&$top=1")).await?.pointer("/value/0").is_none_or(Value::is_null),
            None => more,
        };
        Ok((mails, older))
    }

    async fn find_by_message_id(&self, account: &AccountConfig, message_id: &str) -> Result<Option<Value>> {
        let mid = format!("<{}>", message_id.trim().trim_matches(['<', '>']));
        let found = self.get(account, &format!("/me/messages?$filter=internetMessageId eq '{}'&$select=id,parentFolderId", odata(&mid))).await?;
        Ok(found.pointer("/value/0").cloned())
    }

    /// Moves a mail found by Message-ID to `to` (`None` = archive, then read).
    /// Back in the inbox it is unread again. Returns the target folder.
    pub async fn move_found(&self, account: &AccountConfig, message_id: &str, to: Option<&str>) -> Result<String> {
        let mail = self.find_by_message_id(account, message_id).await?.context("Mail not found – it may have been moved or deleted elsewhere")?;
        let id = str_field(&mail, "id").unwrap_or_default();
        let (target_id, target_name) = match to {
            None => ("archive".to_string(), "Archive".to_string()),
            Some(path) => (self.folder_id(account, path, true).await?.context("Folder could not be created")?, path.to_string()),
        };
        if to.is_none() {
            self.patch(account, &id, json!({ "isRead": true })).await?;
        }
        if to.is_some_and(|p| Self::is_inbox(account, p)) {
            self.patch(account, &id, json!({ "isRead": false })).await?;
        }
        let here = str_field(&mail, "parentFolderId").unwrap_or_default();
        let target_real = if target_id.contains('-') || target_id.len() > 20 { target_id.clone() } else { self.well_known_id(account, &target_id).await? };
        if here != target_real {
            self.move_id(account, &id, &target_id).await?;
        }
        Ok(target_name)
    }

    /// The real id of a well-known folder ("inbox", "archive").
    async fn well_known_id(&self, account: &AccountConfig, name: &str) -> Result<String> {
        let key = (account.id.clone(), format!("\u{0}{name}"));
        if let Some(id) = self.folders.lock().await.get(&key) {
            return Ok(id.clone());
        }
        let id = str_field(&self.get(account, &format!("/me/mailFolders/{name}?$select=id")).await?, "id").unwrap_or_default();
        self.folders.lock().await.insert(key, id.clone());
        Ok(id)
    }

    pub async fn content_by_message_id(&self, account: &AccountConfig, message_id: &str) -> Result<MailContent> {
        let mail = self.find_by_message_id(account, message_id).await?.context("Mail not found")?;
        Ok(parse::parse_content(&self.raw(account, &str_field(&mail, "id").unwrap_or_default()).await?))
    }

    /// Archived mails back to the inbox, unread. Returns how many were moved.
    pub async fn unarchive(&self, account: &AccountConfig, message_ids: &[String]) -> Result<usize> {
        let mut moved = 0;
        for mid in message_ids {
            if self.move_found(account, mid, Some("inbox")).await.is_ok() {
                moved += 1;
            }
        }
        Ok(moved)
    }

    /// For "Test connection": signs in and counts the unread inbox mails.
    pub async fn test_connection(&self, account: &AccountConfig) -> Result<String> {
        let inbox = self.get(account, "/me/mailFolders/inbox?$select=unreadItemCount,totalItemCount").await?;
        let n = |k: &str| inbox.get(k).and_then(Value::as_u64).unwrap_or(0);
        Ok(format!("Connected to Microsoft 365 – {} mails in the inbox, {} unread", n("totalItemCount"), n("unreadItemCount")))
    }

    pub async fn keep_warm(&self, account: &AccountConfig) -> Result<()> {
        self.token(account).await.map(|_| ())
    }

    // ------------------------------------------------------------ watching

    /// Signs in, loads the unread mails and checks again every minute.
    /// Returns only on errors; the caller reconnects.
    pub async fn run_watcher(&self, w: &Watch) -> Result<()> {
        let account = &w.account;
        status::set(&w.app, &account.id, "connecting", "Signing in to Microsoft …".to_string());
        self.token(account).await?;
        tracing::info!("[{}] Connected to Microsoft Graph – watching the inbox", account.label);
        log::event(&w.app, &account.id, &account.label, "info", "Connected to Microsoft 365 – checking every minute");
        loop {
            self.sync(w).await?;
            tokio::select! {
                _ = tokio::time::sleep(POLL_INTERVAL) => {}
                _ = w.check.notified() => log::event(&w.app, &account.id, &account.label, "info", "Manual check"),
            }
        }
    }

    async fn sync(&self, w: &Watch) -> Result<()> {
        let account = &w.account;
        let started = Instant::now();
        // Newest first: a mail that just arrived gets the highest uid.
        let recent = self.list(account, &format!("/me/mailFolders/inbox/messages?$select={SELECT}&$orderby=receivedDateTime desc&$top={FRESH_WINDOW}"), FRESH_WINDOW).await?;
        let recent = self.summaries(account, &recent).await;
        let unread = self
            .list(account, &format!("/me/mailFolders/inbox/messages?$select={SELECT}&$filter=receivedDateTime ge 1900-01-01T00:00:00Z and isRead eq false&$orderby=receivedDateTime desc&$top=100"), LIST_LIMIT)
            .await?;
        let mut mails = self.summaries(account, &unread).await;
        let flagged = self.list(account, &format!("/me/mailFolders/inbox/messages?$select={SELECT}&$filter=flag/flagStatus eq 'flagged'&$top={PIN_LIMIT}"), PIN_LIMIT).await?;
        let mut pinned = self.summaries(account, &flagged).await;
        let total = self.get(account, "/me/mailFolders/inbox?$select=unreadItemCount").await?.get("unreadItemCount").and_then(Value::as_u64).unwrap_or(mails.len() as u64) as usize;

        for m in mails.iter_mut().chain(pinned.iter_mut()) {
            if let Some(bulk) = m.from_addr.as_deref().and_then(|a| w.kinds.get(a)) {
                m.bulk = bulk;
            }
        }

        // Features only see mails arriving after the start.
        let seen = w.last_uid.load(Ordering::Relaxed);
        if seen == 0 {
            w.last_uid.store(self.highest_uid(&account.id).await.max(1), Ordering::Relaxed);
        } else {
            let mut fresh: Vec<MailSummary> = recent.into_iter().filter(|m| m.uid > seen).collect();
            fresh.sort_by_key(|m| m.uid);
            for mut mail in fresh {
                let body = if w.hook.wants_body(account, &mail) { Some(self.body(account, mail.uid).await?) } else { None };
                w.hook.on_new_mail(account, &mut mail, body.as_ref()).await;
                // Findings belong to the listed copy too.
                if let Some(listed) = mails.iter_mut().find(|m| m.uid == mail.uid) {
                    listed.findings = mail.findings.clone();
                }
                w.last_uid.store(mail.uid, Ordering::Relaxed);
            }
            let newest = self.highest_uid(&account.id).await;
            if newest > w.last_uid.load(Ordering::Relaxed) {
                // Mails first seen elsewhere (unread list, folder views) are not new arrivals.
                w.last_uid.store(newest, Ordering::Relaxed);
            }
        }

        let known: HashSet<u32> = mails.iter().map(|m| m.uid).collect();
        pinned.retain(|p| !known.contains(&p.uid) || p.flagged);
        w.store.set_account(&account.id, &account.label, total, mails, pinned);
        status::set(&w.app, &account.id, "ok", format!("Connected – {total} unread"));
        tracing::debug!("[{}] Graph check in {} ms", account.label, started.elapsed().as_millis());
        Ok(())
    }
}

// ---------------------------------------------------------------- helpers

fn tenant_or_default(tenant: &str) -> &str {
    if tenant.trim().is_empty() { "organizations" } else { tenant.trim() }
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

fn host(url: &str) -> &str {
    url.split("//").nth(1).and_then(|r| r.split('/').next()).unwrap_or(url)
}

/// OData string literal: single quotes doubled.
fn odata(s: &str) -> String {
    s.replace('\'', "''").replace('&', "%26").replace('#', "%23")
}

/// Readable OAuth error, e.g. "admin approval required".
fn oauth_error(body: &Value) -> Option<String> {
    let code = body.get("error").and_then(Value::as_str)?;
    let desc = body.get("error_description").and_then(Value::as_str).unwrap_or("");
    let first = desc.lines().next().unwrap_or("").trim();
    let hint = if desc.contains("AADSTS65001") || desc.contains("AADSTS90094") || desc.contains("AADSTS90095") || desc.contains("admin") {
        " – your organisation requires an administrator to approve Maillet"
    } else if code == "access_denied" || code == "authorization_declined" {
        " – the sign-in was declined"
    } else {
        ""
    };
    Some(format!("{}{hint}", if first.is_empty() { code } else { first }))
}

/// Converts a Graph message to the list entry. The original headers (if
/// Graph sends them) go through the same parser as IMAP mails, so sender,
/// subject and newsletter detection behave alike.
fn to_summary(account: &AccountConfig, uid: u32, v: &Value) -> MailSummary {
    let mut header = String::new();
    if let Some(hs) = v.get("internetMessageHeaders").and_then(Value::as_array) {
        for h in hs {
            let (Some(n), Some(val)) = (h.get("name").and_then(Value::as_str), h.get("value").and_then(Value::as_str)) else { continue };
            if n.eq_ignore_ascii_case("content-type") || n.eq_ignore_ascii_case("content-transfer-encoding") {
                continue;
            }
            header.push_str(&format!("{n}: {}\r\n", val.replace(['\r', '\n'], " ")));
        }
    }
    header.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
    let preview = str_field(v, "bodyPreview").unwrap_or_default();
    let origin = Origin { account_id: &account.id, account: &account.label, uid_validity: 0, uid };
    let mut m = parse::summarize(&origin, header.as_bytes(), preview.as_bytes());
    if let Some(s) = str_field(v, "subject") {
        m.subject = s;
    }
    if let Some(from) = v.pointer("/from/emailAddress") {
        m.from_addr = str_field(from, "address").filter(|a| !a.is_empty()).or(m.from_addr);
        m.from_name = str_field(from, "name").filter(|n| !n.trim().is_empty() && Some(n) != m.from_addr.as_ref()).or(m.from_name);
    }
    if let Some(t) = v.get("receivedDateTime").and_then(Value::as_str).and_then(parse_iso) {
        m.date = Some(t);
    }
    if let Some(mid) = str_field(v, "internetMessageId") {
        m.message_id = Some(mid.trim_matches(['<', '>']).to_string());
    }
    m.has_attachments = v.get("hasAttachments").and_then(Value::as_bool).unwrap_or(false);
    m.seen = v.get("isRead").and_then(Value::as_bool).unwrap_or(false);
    m.flagged = v.pointer("/flag/flagStatus").and_then(Value::as_str) == Some("flagged");
    m
}

/// `2026-11-11T14:40:05Z` (optionally with fractions) → Unix seconds.
fn parse_iso(s: &str) -> Option<i64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-').map(|x| x.parse::<i64>());
    let (y, mo, day) = (d.next()?.ok()?, d.next()?.ok()?, d.next()?.ok()?);
    let time = time.trim_end_matches('Z');
    let time = time.split(['+']).next()?;
    let mut t = time.split(':');
    let (h, mi) = (t.next()?.parse::<i64>().ok()?, t.next()?.parse::<i64>().ok()?);
    let sec = t.next().and_then(|x| x.split('.').next()).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
    Some(days_from_civil(y, mo, day) * 86_400 + h * 3600 + mi * 60 + sec)
}

/// Days since 1970-01-01 (H. Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// ISO date-time (UTC midnight) of the day `days` days ago.
fn iso_days_ago(days: u32) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    let mut z = now / 86_400 - days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    z -= era * 146_097;
    let yoe = (z - z / 1460 + z / 36_524 - z / 146_096) / 365;
    let doy = z - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T00:00:00Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> AccountConfig {
        AccountConfig { id: "g1".into(), label: "Work 365".into(), mailbox: "INBOX".into(), ..Default::default() }
    }

    #[test]
    fn test_parse_iso() {
        assert_eq!(parse_iso("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_iso("2026-11-11T14:40:05Z"), Some(1_794_408_005));
        assert_eq!(parse_iso("2026-11-11T14:40:05.1234567Z"), Some(1_794_408_005));
    }

    #[test]
    fn test_iso_days_ago_roundtrip() {
        let d = iso_days_ago(0);
        let ts = parse_iso(&d).unwrap();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        assert!(now - ts < 86_400 && now >= ts, "{d}");
    }

    #[test]
    fn test_summary_from_graph_message() {
        let v = json!({
            "id": "AAMk1", "subject": "Your invoice", "bodyPreview": "Thanks for your order. Total 12,90 €",
            "receivedDateTime": "2026-11-11T09:00:00Z", "hasAttachments": true, "isRead": false,
            "flag": { "flagStatus": "flagged" }, "internetMessageId": "<abc@shop.example>",
            "from": { "emailAddress": { "name": "Shop", "address": "billing@shop.example" } },
            "internetMessageHeaders": [
                { "name": "From", "value": "Shop <billing@shop.example>" },
                { "name": "List-Unsubscribe", "value": "<mailto:u@shop.example>" },
                { "name": "Content-Type", "value": "multipart/alternative; boundary=x" }
            ]
        });
        let m = to_summary(&account(), 7, &v);
        assert_eq!(m.id, "g1:0:7");
        assert_eq!(m.subject, "Your invoice");
        assert_eq!(m.from_addr.as_deref(), Some("billing@shop.example"));
        assert_eq!(m.from_name.as_deref(), Some("Shop"));
        assert_eq!(m.message_id.as_deref(), Some("abc@shop.example"));
        assert_eq!(m.date, Some(1_794_387_600));
        assert!(m.preview.starts_with("Thanks for your order"), "{}", m.preview);
        assert!(m.bulk && m.flagged && m.has_attachments && !m.seen);
    }

    #[test]
    fn test_summary_without_headers() {
        let v = json!({ "id": "AAMk2", "subject": "Hi", "bodyPreview": "Lunch?", "isRead": true,
            "from": { "emailAddress": { "name": "", "address": "tom@x.example" } } });
        let m = to_summary(&account(), 1, &v);
        assert_eq!(m.from_addr.as_deref(), Some("tom@x.example"));
        assert_eq!(m.from_name, None);
        assert!(m.seen && !m.bulk);
    }

    #[test]
    fn test_admin_approval_hint() {
        let e = oauth_error(&json!({ "error": "invalid_grant", "error_description": "AADSTS65001: The user or administrator has not consented to use the application.\r\nTrace ID: 1" })).unwrap();
        assert!(e.contains("administrator to approve"), "{e}");
        assert!(!e.contains("Trace ID"));
    }

    #[tokio::test]
    async fn test_uids_are_stable_per_graph_id() {
        let g = Graph::new("http://127.0.0.1:1", "http://127.0.0.1:1");
        let a = g.uid_for("acc", "X").await;
        let b = g.uid_for("acc", "Y").await;
        assert_eq!(g.uid_for("acc", "X").await, a);
        assert!(b > a);
        assert_eq!(g.graph_id("acc", b).await.unwrap(), "Y");
        assert!(g.graph_id("acc", 99).await.is_err());
    }

    /// A tiny stand-in for Microsoft: sign-in, inbox, folders, moving.
    mod fake {
        use axum::extract::{Path, RawQuery, State};
        use axum::routing::{get, patch, post};
        use axum::{Json, Router};
        use serde_json::{json, Value};
        use std::sync::{Arc, Mutex};

        #[derive(Default)]
        pub struct Box_ {
            pub folders: Vec<(String, String, Option<String>)>, // id, name, parent
            pub mails: Vec<Value>,
            pub calls: Vec<String>,
        }
        pub type S = Arc<Mutex<Box_>>;

        fn q(raw: &Option<String>) -> String {
            raw.clone().unwrap_or_default().replace("%20", " ").replace("%27", "'")
        }
        fn eq_value(query: &str, field: &str) -> Option<String> {
            let start = query.find(&format!("{field} eq '"))? + field.len() + 5;
            Some(query[start..].split('\'').next()?.to_string())
        }

        pub async fn serve(state: S) -> String {
            let app = Router::new()
                .route("/{tenant}/oauth2/v2.0/token", post(|| async { Json(json!({ "access_token": "AT", "expires_in": 3600, "refresh_token": "RT2" })) }))
                .route("/v1.0/me/mailFolders/{fid}/messages", get(|State(s): State<S>, Path(fid): Path<String>, RawQuery(raw): RawQuery| async move {
                    let st = s.lock().unwrap();
                    let query = q(&raw);
                    let want = if fid == "inbox" { "F-inbox".to_string() } else { fid };
                    let mut v: Vec<Value> = st.mails.iter().filter(|m| m["parentFolderId"] == want).cloned().collect();
                    if query.contains("isRead eq false") { v.retain(|m| m["isRead"] == false); }
                    if query.contains("receivedDateTime lt") { v.clear(); }
                    Json(json!({ "value": v }))
                }))
                .route("/v1.0/me/mailFolders/{fid}", get(|State(s): State<S>, Path(fid): Path<String>| async move {
                    let st = s.lock().unwrap();
                    let id = if fid == "inbox" { "F-inbox".to_string() } else { fid };
                    let unread = st.mails.iter().filter(|m| m["parentFolderId"] == id.as_str() && m["isRead"] == false).count();
                    Json(json!({ "id": id, "unreadItemCount": unread, "totalItemCount": st.mails.len() }))
                }))
                .route("/v1.0/me/mailFolders", get(|State(s): State<S>, RawQuery(raw): RawQuery| async move {
                    let st = s.lock().unwrap();
                    let name = eq_value(&q(&raw), "displayName");
                    let v: Vec<Value> = st.folders.iter().filter(|f| f.2.is_none() && name.as_ref().is_none_or(|n| n.eq_ignore_ascii_case(&f.1))).map(|f| json!({ "id": f.0, "displayName": f.1, "childFolderCount": 0 })).collect();
                    Json(json!({ "value": v }))
                }).post(|State(s): State<S>, Json(b): Json<Value>| async move {
                    let mut st = s.lock().unwrap();
                    let id = format!("F-{}", st.folders.len());
                    st.folders.push((id.clone(), b["displayName"].as_str().unwrap().to_string(), None));
                    st.calls.push(format!("create {}", b["displayName"]));
                    Json(json!({ "id": id }))
                }))
                .route("/v1.0/me/messages", get(|State(s): State<S>, RawQuery(raw): RawQuery| async move {
                    let st = s.lock().unwrap();
                    let mid = eq_value(&q(&raw).replace("%3C", "<").replace("%3E", ">"), "internetMessageId");
                    let v: Vec<Value> = st.mails.iter().filter(|m| mid.as_deref() == m["internetMessageId"].as_str()).cloned().collect();
                    Json(json!({ "value": v }))
                }))
                .route("/v1.0/me/messages/{id}", patch(|State(s): State<S>, Path(id): Path<String>, Json(b): Json<Value>| async move {
                    let mut st = s.lock().unwrap();
                    st.calls.push(format!("patch {id} {b}"));
                    if let Some(m) = st.mails.iter_mut().find(|m| m["id"] == id.as_str()) {
                        for (k, v) in b.as_object().unwrap() { m[k] = v.clone(); }
                    }
                    Json(json!({}))
                }))
                .route("/v1.0/me/messages/{id}/move", post(|State(s): State<S>, Path(id): Path<String>, Json(b): Json<Value>| async move {
                    let mut st = s.lock().unwrap();
                    let dest = match b["destinationId"].as_str().unwrap() { "inbox" => "F-inbox".to_string(), d => d.to_string() };
                    st.calls.push(format!("move {id} {dest}"));
                    if let Some(m) = st.mails.iter_mut().find(|m| m["id"] == id.as_str()) { m["parentFolderId"] = json!(dest); }
                    Json(json!({ "id": id }))
                }))
                .route("/v1.0/me/messages/{id}/$value", get(|| async { "From: Jana <jana@x.example>\r\nSubject: Hello\r\nContent-Type: text/plain\r\n\r\nHi there, the code is 123456.\r\n" }))
                .with_state(state);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            format!("http://{addr}")
        }
    }

    #[tokio::test]
    async fn test_against_fake_graph() {
        let state: fake::S = Default::default();
        {
            let mut st = state.lock().unwrap();
            st.folders.push(("F-inbox".into(), "Inbox".into(), None));
            st.mails.push(json!({ "id": "M1", "subject": "Hello", "bodyPreview": "Hi there", "isRead": false, "parentFolderId": "F-inbox",
                "receivedDateTime": "2026-11-11T09:00:00Z", "internetMessageId": "<m1@x.example>",
                "from": { "emailAddress": { "name": "Jana", "address": "jana@x.example" } } }));
        }
        let base = fake::serve(state.clone()).await;
        let g = Graph::new(&base, &format!("{base}/v1.0"));
        let mut acc = account();
        acc.pass = "RT1".into();
        acc.client_id = "client".into();

        // Inbox listing → uid; reading the mail via its MIME source.
        let (mails, older) = g.folder_mails_since(&acc, "INBOX", Some(7), 100).await.unwrap();
        assert_eq!(mails.len(), 1);
        assert!(!older);
        let uid = mails[0].uid;
        assert!(g.content(&acc, uid).await.unwrap().text.contains("123456"));
        assert!(g.test_connection(&acc).await.unwrap().contains("1 unread"));

        // Filing creates the folder, marks read and moves; the id stays the same.
        g.move_to_folder(&acc, uid, "Tasks").await.unwrap();
        {
            let st = state.lock().unwrap();
            assert!(st.calls.iter().any(|c| c.starts_with("create \"Tasks\"")), "{:?}", st.calls);
            assert_eq!(st.mails[0]["isRead"], true);
            assert_eq!(st.mails[0]["parentFolderId"], "F-1");
        }
        assert_eq!(g.folder_mails_since(&acc, "Tasks", None, 10).await.unwrap().0[0].uid, uid);

        // Undo: found by Message-ID, back in the inbox and unread again.
        g.move_found(&acc, "m1@x.example", Some("INBOX")).await.unwrap();
        let st = state.lock().unwrap();
        assert_eq!(st.mails[0]["parentFolderId"], "F-inbox");
        assert_eq!(st.mails[0]["isRead"], false);
    }

}
