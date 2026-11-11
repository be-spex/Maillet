//! Application layer: state, Tauri commands, watcher tasks, tray and windows.

mod shortcut;
mod tray;
pub mod windows;

use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::{async_runtime, AppHandle, Emitter, Manager, RunEvent, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::{watch, Mutex, Notify};

use crate::base::config::{self, AccountConfig, SharedConfig};
use crate::base::{log, status};
use crate::features::otp::{self, api, catalog, popup, rule_store, rules};
use crate::features::Pipeline;
use crate::mail::imap::{self, Commands, MailHook, Watch};
use crate::mail::model::{MailContent, MailSummary};
use crate::mail::store::{MailStore, Snapshot};

struct WatchTask {
    stop_tx: tokio::sync::oneshot::Sender<()>,
}

pub struct AppState {
    pub config: SharedConfig,
    tasks: Mutex<Vec<WatchTask>>,
    /// Wakes all watchers for an immediate mailbox check.
    check_now: Arc<Notify>,
    /// Current OTP rules; the feature picks up changes without reconnecting.
    rules: watch::Sender<Vec<rules::DetectionRule>>,
    store: Arc<MailStore>,
    commands: Arc<Commands>,
    /// Set in `setup`, as features need the app handle.
    pipeline: OnceLock<Arc<Pipeline>>,
    /// Desktop notifications for new mails on/off.
    notifications: Arc<std::sync::atomic::AtomicBool>,
    /// Tray counts only mails from people.
    tray_people_only: Arc<std::sync::atomic::AtomicBool>,
    /// The user's person/bulk corrections per sender.
    kinds: Arc<crate::mail::classify::SenderKinds>,
}

impl AppState {
    fn publish_rules(&self, rules: Vec<rules::DetectionRule>) -> Vec<rules::DetectionRule> {
        self.rules.send_replace(rules.clone());
        rules
    }
}

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

// ---------------------------------------------------------------- Mails

#[tauri::command]
fn list_mails(state: State<AppState>, statuses: State<status::StatusStore>) -> serde_json::Value {
    let Snapshot { accounts, mails, pinned } = state.store.snapshot();
    serde_json::json!({ "accounts": accounts, "mails": mails, "pinned": pinned, "status": statuses.all() })
}

/// Account and UID of a mail id (`account:uidvalidity:uid`).
fn locate(state: &AppState, id: &str) -> Result<(AccountConfig, u32), String> {
    let (account_id, uid) = match state.store.find(id) {
        Some(MailSummary { account_id, uid, .. }) => (account_id, uid),
        None => {
            let mut parts = id.rsplitn(3, ':');
            let uid = parts.next().and_then(|u| u.parse().ok()).ok_or("Invalid mail id")?;
            let account_id = parts.nth(1).ok_or("Invalid mail id")?.to_string();
            (account_id, uid)
        }
    };
    let account = config::load_accounts_with_passwords()
        .map_err(err)?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;
    Ok((account, uid))
}

#[tauri::command]
async fn get_mail(state: State<'_, AppState>, id: String) -> Result<MailContent, String> {
    let (account, uid) = locate(&state, &id)?;
    state.commands.content(&account, uid).await.map_err(err)
}

#[tauri::command]
async fn set_seen(state: State<'_, AppState>, id: String, seen: bool) -> Result<(), String> {
    let (account, uid) = locate(&state, &id)?;
    state.commands.set_seen(&account, uid, seen).await.map_err(err)?;
    if seen {
        state.store.remove(&account.id, uid);
    } else {
        state.check_now.notify_waiters();
    }
    Ok(())
}

#[tauri::command]
async fn archive_mail(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let (account, uid) = locate(&state, &id)?;
    let folder = state.commands.archive(&account, uid).await.map_err(err)?;
    state.store.remove(&account.id, uid);
    Ok(folder)
}

// ---------------------------------------------------------------- Pinned & tasks

/// Server folder holding mails that still need something done. The full name
/// depends on the server, e.g. "Folders/Tasks" on Proton. Folder names are
/// English whatever the UI language, so they read the same in every mail client.
const TASK_FOLDER: &str = "Tasks";


fn account_by_id(id: &str) -> Result<AccountConfig, String> {
    config::load_accounts_with_passwords().map_err(err)?.into_iter().find(|a| a.id == id).ok_or_else(|| "Account not found".into())
}

/// The account's task folder; `create` makes it on first use.
async fn task_folder(state: &AppState, account: &AccountConfig, create: bool) -> Result<String, String> {
    state.commands.resolve_folder(account, TASK_FOLDER, create).await.map_err(err)?.ok_or_else(|| "No task folder on this account".into())
}

/// Task states and their folders: open = the task folder itself, the others
/// are subfolders (custom IMAP keywords are not supported by every server).
const STATES: [(&str, Option<&str>, usize); 3] = [("open", None, 200), ("progress", Some("In Progress"), 200), ("done", Some("Done"), 30)];

/// Folder of a task state; `None` if it does not exist and `create` is off.
async fn state_folder(state: &AppState, account: &AccountConfig, status: &str, create: bool) -> Result<Option<String>, String> {
    let (_, child, _) = STATES.iter().find(|(s, _, _)| *s == status).ok_or("Unknown task state")?;
    let Some(base) = state.commands.resolve_folder(account, TASK_FOLDER, create).await.map_err(err)? else { return Ok(None) };
    match child {
        None => Ok(Some(base)),
        Some(c) => state.commands.resolve_child(account, &base, c, create).await.map_err(err),
    }
}

/// Pins (star on the server) or unpins a mail.
#[tauri::command]
async fn set_pinned(state: State<'_, AppState>, id: String, pinned: bool) -> Result<(), String> {
    let (account, uid) = locate(&state, &id)?;
    state.commands.set_flagged(&account, uid, pinned).await.map_err(err)?;
    state.check_now.notify_waiters();
    Ok(())
}

/// A task is a mail in the server's task folder, found again by Message-ID.
#[derive(serde::Serialize)]
struct TaskRef {
    account_id: String,
    message_id: Option<String>,
}

/// Moves a mail from the inbox into the task folder (state "open").
#[tauri::command]
async fn make_task(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<TaskRef, String> {
    let message_id = state.store.find(&id).and_then(|m| m.message_id);
    let (account, uid) = locate(&state, &id)?;
    let folder = task_folder(&state, &account, true).await?;
    state.commands.move_to_folder(&account, uid, &folder).await.map_err(err)?;
    state.store.remove(&account.id, uid);
    let _ = app.emit("tasks-changed", ());
    Ok(TaskRef { account_id: account.id, message_id })
}

fn categories(state: &AppState) -> Vec<config::Category> {
    state.config.try_lock().ok().and_then(|c| c.categories.clone()).unwrap_or_else(config::default_categories)
}

#[derive(serde::Serialize)]
struct Filed {
    account_id: String,
    message_id: Option<String>,
    folder: String,
}

/// Files a mail in the folder of a category (created on first use).
#[tauri::command]
async fn file_mail(state: State<'_, AppState>, id: String, category: String) -> Result<Filed, String> {
    let cat = categories(&state).into_iter().find(|c| c.id == category).ok_or("Unknown category")?;
    let message_id = state.store.find(&id).and_then(|m| m.message_id);
    let (account, uid) = locate(&state, &id)?;
    let folder = state.commands.resolve_folder(&account, &cat.folder, true).await.map_err(err)?.ok_or("Folder could not be created")?;
    state.commands.move_to_folder(&account, uid, &folder).await.map_err(err)?;
    state.store.remove(&account.id, uid);
    Ok(Filed { account_id: account.id, message_id, folder })
}

/// Undo of [`file_mail`]: back to the inbox.
#[tauri::command]
async fn unfile_mail(state: State<'_, AppState>, account_id: String, message_id: String, folder: String) -> Result<(), String> {
    let account = account_by_id(&account_id)?;
    state.commands.move_found(&account, &[folder], &message_id, Some(&account.mailbox)).await.map_err(err)?;
    state.check_now.notify_waiters();
    Ok(())
}

#[tauri::command]
async fn set_categories(app: AppHandle, categories: Vec<config::Category>) -> Result<(), String> {
    let shared = app.state::<AppState>().config.clone();
    async_runtime::spawn_blocking(move || config::set_categories(&shared, categories)).await.map_err(|e| e.to_string())?.map_err(err)?;
    let _ = app.emit("settings-changed", ());
    Ok(())
}

/// Folder names of all accounts, for choosing an existing folder as target.
#[tauri::command]
async fn list_folders(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let accounts: Vec<_> = config::load_accounts_with_passwords().map_err(err)?.into_iter().filter(|a| !a.server.is_empty() && !a.pass.is_empty()).collect();
    let commands = &state.commands;
    let lists = futures_util::future::join_all(accounts.iter().map(|a| commands.list_folders(a))).await;
    let mut all: Vec<String> = lists.into_iter().filter_map(Result::ok).flatten().collect();
    all.sort();
    all.dedup();
    Ok(all)
}

/// Moves a mail to the trash (by its window id).
#[tauri::command]
async fn trash_mail(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let (account, uid) = locate(&state, &id)?;
    state.commands.delete(&account, uid).await.map_err(err)?;
    state.store.remove(&account.id, uid);
    Ok(())
}

#[tauri::command]
async fn set_mark_read_on_open(app: AppHandle, on: bool) -> Result<(), String> {
    let shared = app.state::<AppState>().config.clone();
    async_runtime::spawn_blocking(move || config::set_mark_read_on_open(&shared, on)).await.map_err(|e| e.to_string())?.map_err(err)?;
    let _ = app.emit("settings-changed", ());
    Ok(())
}

#[tauri::command]
async fn set_actions(app: AppHandle, actions: Vec<String>) -> Result<(), String> {
    let shared = app.state::<AppState>().config.clone();
    async_runtime::spawn_blocking(move || config::set_actions(&shared, actions)).await.map_err(|e| e.to_string())?.map_err(err)?;
    let _ = app.emit("settings-changed", ());
    Ok(())
}

#[derive(serde::Serialize)]
struct TaskItem {
    status: &'static str,
    mail: MailSummary,
}

/// All tasks of all accounts with their state, newest first.
#[tauri::command]
async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<TaskItem>, String> {
    let mut all = vec![];
    for account in config::load_accounts_with_passwords().map_err(err)? {
        if account.server.is_empty() || account.pass.is_empty() {
            continue;
        }
        for (status, _, limit) in STATES {
            let folder = match state_folder(&state, &account, status, false).await {
                Ok(Some(f)) => f,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!("[{}] task folder: {e}", account.label);
                    continue;
                }
            };
            match state.commands.folder_mails(&account, &folder, limit).await {
                Ok(mails) => all.extend(mails.into_iter().map(|mail| TaskItem { status, mail })),
                Err(e) => tracing::warn!("[{}] task folder {folder}: {e:#}", account.label),
            }
        }
    }
    all.sort_by_key(|t| std::cmp::Reverse(t.mail.date.unwrap_or(0)));
    Ok(all)
}

/// Moves a task between states. `from`/`to`: `open`, `progress`, `done`, plus
/// `to` = `archive` (out of the tasks) or `inbox` (no longer a task).
/// `from_folder` overrides the source folder (undo after archiving).
/// Returns the target folder, so the move can be undone.
#[tauri::command]
async fn move_task(app: AppHandle, state: State<'_, AppState>, account_id: String, message_id: String, from: String, to: String, from_folder: Option<String>) -> Result<String, String> {
    let account = account_by_id(&account_id)?;
    let source = match from_folder {
        Some(f) => f,
        None => state_folder(&state, &account, &from, false).await?.ok_or("Task folder not found")?,
    };
    let target = match to.as_str() {
        "archive" => None,
        "inbox" => Some(account.mailbox.clone()),
        status => Some(state_folder(&state, &account, status, true).await?.ok_or("Task folder not found")?),
    };
    // Where the window saw the mail first, then the other task folders and the inbox.
    let mut sources = vec![source];
    for (status, _, _) in STATES {
        if let Some(f) = state_folder(&state, &account, status, false).await? {
            if !sources.contains(&f) {
                sources.push(f);
            }
        }
    }
    if !sources.contains(&account.mailbox) {
        sources.push(account.mailbox.clone());
    }
    let moved = state.commands.move_found(&account, &sources, &message_id, target.as_deref()).await.map_err(err)?;
    if to == "inbox" {
        state.check_now.notify_waiters();
    }
    let _ = app.emit("tasks-changed", ());
    Ok(moved)
}

/// Days the inbox view shows by default.
const INBOX_DAYS: u32 = 7;
/// At most this many mails per account and load, whatever the period.
const INBOX_CAP: usize = 500;

#[derive(serde::Serialize)]
struct InboxPage {
    mails: Vec<MailSummary>,
    /// Older mails exist on at least one account.
    has_older: bool,
}

/// The whole inbox (read and unread) of the last `days` days, newest first.
#[tauri::command]
async fn list_inbox(state: State<'_, AppState>, days: u32) -> Result<InboxPage, String> {
    let days = days.clamp(1, 3650);
    let started = Instant::now();
    let accounts: Vec<_> = config::load_accounts_with_passwords().map_err(err)?.into_iter().filter(|a| !a.server.is_empty() && !a.pass.is_empty()).collect();
    // All accounts at once: the slowest one sets the waiting time, not the sum.
    let commands = &state.commands;
    let pages = futures_util::future::join_all(accounts.iter().map(|a| async move {
        let t = Instant::now();
        let page = commands.folder_mails_since(a, &a.mailbox, Some(days), INBOX_CAP).await;
        tracing::info!("[{}] inbox ({days} days) loaded in {} ms", a.label, t.elapsed().as_millis());
        page
    }))
    .await;
    let (mut all, mut has_older) = (vec![], false);
    for page in pages {
        let (mut mails, older) = page.map_err(err)?;
        has_older |= older;
        for m in &mut mails {
            if let Some(bulk) = m.from_addr.as_deref().and_then(|a| state.kinds.get(a)) {
                m.bulk = bulk;
            }
        }
        all.extend(mails);
    }
    all.sort_by_key(|m| std::cmp::Reverse(m.date.unwrap_or(0)));
    tracing::info!("inbox: {} mails in {} ms", all.len(), started.elapsed().as_millis());
    Ok(InboxPage { mails: all, has_older })
}

#[tauri::command]
async fn set_inbox_days(app: AppHandle, days: u32) -> Result<(), String> {
    let days = days.clamp(1, 365);
    let shared = app.state::<AppState>().config.clone();
    async_runtime::spawn_blocking(move || config::set_inbox_days(&shared, days)).await.map_err(|e| e.to_string())?.map_err(err)?;
    let _ = app.emit("settings-changed", ());
    Ok(())
}

/// Text of a task mail (it lives in one of the task folders).
#[tauri::command]
async fn get_task_mail(state: State<'_, AppState>, account_id: String, message_id: String) -> Result<MailContent, String> {
    let account = account_by_id(&account_id)?;
    let mut folders = vec![];
    for (status, _, _) in STATES {
        if let Some(f) = state_folder(&state, &account, status, false).await? {
            folders.push(f);
        }
    }
    folders.push(account.mailbox.clone());
    state.commands.content_by_message_id(&account, folders, &message_id).await.map_err(err)
}

/// Deletes the mail a code/link came from (code popup).
#[tauri::command]
async fn delete_mail(state: State<'_, AppState>, account_id: String, uid: u32) -> Result<(), String> {
    let account = config::load_accounts_with_passwords()
        .map_err(err)?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;
    state.commands.delete(&account, uid).await.map_err(err)?;
    state.store.remove(&account.id, uid);
    Ok(())
}

/// Version, accounts with connection status and the local API, for the status panel.
#[tauri::command]
fn get_app_status(app: AppHandle, state: State<AppState>, statuses: State<status::StatusStore>, api: State<api::ApiServer>) -> serde_json::Value {
    let all = statuses.all();
    let unread = state.store.snapshot().accounts;
    let accounts: Vec<_> = config::get_accounts(&state.config)
        .into_iter()
        .map(|a| {
            let st = all.iter().find(|s| s.account_id == a.id);
            serde_json::json!({
                "id": a.id,
                "label": a.label,
                "server": format!("{}:{}", a.server, a.port),
                "mailbox": a.mailbox,
                "state": st.map_or("connecting", |s| s.state),
                "message": st.map(|s| s.message.clone()).unwrap_or_default(),
                "timestamp": st.map(|s| s.timestamp),
                "unread": unread.iter().find(|u| u.account_id == a.id).map(|u| u.unread_total),
            })
        })
        .collect();
    serde_json::json!({
        "version": app.package_info().version.to_string(),
        "accounts": accounts,
        "api": api.status(),
        "shortcut": app.state::<shortcut::ShortcutStatus>().get(),
        "notifications": state.notifications.load(std::sync::atomic::Ordering::Relaxed),
        "tray_people_only": state.tray_people_only.load(std::sync::atomic::Ordering::Relaxed),
        "inbox_days": state.config.try_lock().ok().and_then(|c| c.inbox_days).unwrap_or(INBOX_DAYS),
        "actions": state.config.try_lock().ok().and_then(|c| c.actions.clone()).unwrap_or_else(|| config::DEFAULT_ACTIONS.map(String::from).to_vec()),
        "all_actions": config::ALL_ACTIONS,
        "categories": categories(&state),
        "mark_read_on_open": state.config.try_lock().ok().and_then(|c| c.mark_read_on_open).unwrap_or(true),
    })
}

#[tauri::command]
fn check_now(state: State<AppState>) {
    state.check_now.notify_waiters();
}

pub(crate) fn check_now_sync(app: &AppHandle) {
    app.state::<AppState>().check_now.notify_waiters();
}

#[tauri::command]
fn open_window(app: AppHandle, label: String) {
    windows::show(&app, &label);
}

/// Strings of the current language for the window, plus the available languages.
#[tauri::command]
fn get_i18n() -> serde_json::Value {
    use crate::base::i18n;
    serde_json::json!({
        "language": i18n::language(),
        "languages": i18n::languages().into_iter().map(|(code, name)| serde_json::json!({ "code": code, "name": name })).collect::<Vec<_>>(),
        "strings": i18n::table(),
    })
}

/// Switches the UI language at once (window, tray, notifications).
#[tauri::command]
async fn set_language(app: AppHandle, code: String) -> Result<(), String> {
    crate::base::i18n::set_language(&code);
    let state = app.state::<AppState>();
    state.store.touch(); // re-renders tray menu and tooltip
    let shared = state.config.clone();
    let lang = crate::base::i18n::language();
    async_runtime::spawn_blocking(move || config::set_language(&shared, &lang)).await.map_err(|e| e.to_string())?.map_err(err)?;
    let _ = app.emit("settings-changed", ());
    Ok(())
}

/// Sets the global shortcut; it takes effect at once. Returns the new status.
#[tauri::command]
async fn set_shortcut(app: AppHandle, keys: String) -> Result<shortcut::Status, String> {
    let status = shortcut::change(&app, &keys)?;
    let shared = app.state::<AppState>().config.clone();
    async_runtime::spawn_blocking(move || config::set_shortcut(&shared, &keys)).await.map_err(|e| e.to_string())?.map_err(err)?;
    let _ = app.emit("settings-changed", ());
    Ok(status)
}

#[tauri::command]
async fn set_tray_people_only(app: AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.tray_people_only.store(enabled, std::sync::atomic::Ordering::Relaxed);
    state.store.touch();
    let shared = state.config.clone();
    async_runtime::spawn_blocking(move || config::set_tray_people_only(&shared, enabled)).await.map_err(|e| e.to_string())?.map_err(err)
}

/// Marks a sender as person (`bulk = false`), bulk mail (`true`) or back to automatic (`None`).
#[tauri::command]
async fn set_sender_kind(app: AppHandle, addr: String, bulk: Option<bool>) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.kinds.set(&addr, bulk);
    if let Some(b) = bulk {
        state.store.reclassify(&addr, b);
    } else {
        state.check_now.notify_waiters();
    }
    let shared = state.config.clone();
    async_runtime::spawn_blocking(move || config::set_sender_kind(&shared, &addr, bulk)).await.map_err(|e| e.to_string())?.map_err(err)
}

/// Where an archived mail went – enough to bring it back.
#[derive(serde::Serialize, serde::Deserialize)]
struct Archived {
    account_id: String,
    folder: String,
    message_id: Option<String>,
}

/// Archives several mails (e.g. all bulk mail). Returns what can be undone.
#[tauri::command]
async fn archive_mails(state: State<'_, AppState>, ids: Vec<String>) -> Result<Vec<Archived>, String> {
    let mut done = vec![];
    let mut first_error = None;
    for id in ids {
        let message_id = state.store.find(&id).and_then(|m| m.message_id);
        let (account, uid) = locate(&state, &id)?;
        match state.commands.archive(&account, uid).await {
            Ok(folder) => {
                state.store.remove(&account.id, uid);
                done.push(Archived { account_id: account.id, folder, message_id });
            }
            Err(e) => first_error = first_error.or(Some(err(e))),
        }
    }
    match (done.is_empty(), first_error) {
        (true, Some(e)) => Err(e),
        _ => Ok(done),
    }
}

/// Brings archived mails back as unread.
#[tauri::command]
async fn unarchive_mails(state: State<'_, AppState>, items: Vec<Archived>) -> Result<usize, String> {
    let accounts = config::load_accounts_with_passwords().map_err(err)?;
    let mut groups: std::collections::BTreeMap<(String, String), Vec<String>> = Default::default();
    for item in items {
        if let Some(mid) = item.message_id {
            groups.entry((item.account_id, item.folder)).or_default().push(mid);
        }
    }
    let mut moved = 0;
    for ((account_id, folder), ids) in groups {
        let Some(account) = accounts.iter().find(|a| a.id == account_id) else { continue };
        moved += state.commands.unarchive(account, &folder, &ids).await.map_err(err)?;
    }
    state.check_now.notify_waiters();
    Ok(moved)
}

#[tauri::command]
async fn set_notifications(app: AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.notifications.store(enabled, std::sync::atomic::Ordering::Relaxed);
    let shared = state.config.clone();
    async_runtime::spawn_blocking(move || config::set_notifications(&shared, enabled)).await.map_err(|e| e.to_string())?.map_err(err)
}

#[tauri::command]
fn hide_main(app: AppHandle) {
    if let Some(w) = app.get_webview_window(windows::MAIN) {
        let _ = w.hide();
    }
}

// ---------------------------------------------------------------- Accounts

#[tauri::command]
fn get_accounts(state: State<AppState>) -> Vec<AccountConfig> {
    config::get_accounts(&state.config)
}

#[tauri::command]
fn add_account(state: State<AppState>, account: AccountConfig) -> Result<Vec<AccountConfig>, String> {
    config::add_account(&state.config, account).map_err(err)
}

#[tauri::command]
fn update_account(state: State<AppState>, account: AccountConfig) -> Result<Vec<AccountConfig>, String> {
    config::update_account(&state.config, account).map_err(err)
}

#[tauri::command]
fn remove_account(state: State<AppState>, id: String) -> Result<Vec<AccountConfig>, String> {
    let accounts = config::remove_account(&state.config, &id).map_err(err)?;
    state.publish_rules(config::get_rules(&state.config));
    Ok(accounts)
}

#[tauri::command]
fn rename_tag(state: State<AppState>, account_id: String, tag: String) -> Result<Vec<AccountConfig>, String> {
    let accounts = config::rename_tag(&state.config, &account_id, &tag).map_err(err)?;
    state.publish_rules(config::get_rules(&state.config));
    Ok(accounts)
}

#[tauri::command]
async fn test_connection(account: AccountConfig) -> Result<String, String> {
    let account = config::with_stored_password(account).map_err(err)?;
    imap::test_connection(&account).await.map_err(err)
}

/// A Microsoft sign-in in progress: client ID, tenant and the device code.
static GRAPH_LOGIN: std::sync::LazyLock<tokio::sync::Mutex<Option<(String, String, crate::mail::graph::DeviceCode)>>> = std::sync::LazyLock::new(Default::default);

#[derive(serde::Serialize)]
struct SignInCode {
    user_code: String,
    verification_uri: String,
    message: String,
}

/// Microsoft sign-in, step 1: the code the user enters in the browser.
#[tauri::command]
async fn graph_sign_in_start(client_id: String, tenant: String) -> Result<SignInCode, String> {
    let client_id = client_id.trim().to_string();
    if client_id.is_empty() {
        return Err("Enter the application (client) ID of your app registration".into());
    }
    let code = crate::mail::graph::shared().start_device_login(&client_id, &tenant).await.map_err(err)?;
    let view = SignInCode { user_code: code.user_code.clone(), verification_uri: code.verification_uri.clone(), message: code.message.clone() };
    *GRAPH_LOGIN.lock().await = Some((client_id, tenant, code));
    Ok(view)
}

/// Microsoft sign-in, step 2: waits for the confirmation in the browser, then
/// creates the account (or renews the sign-in of `account_id`). The refresh
/// token goes straight to the keyring and never to the window.
#[tauri::command]
async fn graph_sign_in_finish(app: AppHandle, label: String, account_id: Option<String>) -> Result<Vec<AccountConfig>, String> {
    let (client_id, tenant, code) = GRAPH_LOGIN.lock().await.clone().ok_or("No sign-in in progress")?;
    let graph = crate::mail::graph::shared();
    let signed = graph.finish_device_login(&client_id, &tenant, &code).await.map_err(err)?;
    *GRAPH_LOGIN.lock().await = None;
    let shared = app.state::<AppState>().config.clone();
    let accounts = match account_id {
        Some(id) => {
            config::store_secret(&id, &signed.refresh_token).map_err(err)?;
            graph.forget(&id).await;
            config::get_accounts(&shared)
        }
        None => {
            let account = AccountConfig {
                label: if label.trim().is_empty() { signed.email.clone() } else { label.trim().to_string() },
                server: "graph.microsoft.com".into(),
                port: 443,
                user: signed.email.clone(),
                pass: signed.refresh_token.clone(),
                mailbox: "INBOX".into(),
                kind: config::AccountKind::Graph,
                client_id,
                tenant,
                ..Default::default()
            };
            async_runtime::spawn_blocking(move || config::add_account(&shared, account)).await.map_err(|e| e.to_string())?.map_err(err)?
        }
    };
    let all = config::load_accounts_with_passwords().map_err(err)?;
    spawn_watchers(&app, all).await;
    Ok(accounts)
}

#[tauri::command]
async fn graph_sign_in_cancel() {
    *GRAPH_LOGIN.lock().await = None;
}

#[tauri::command]
async fn restart_imap(app: AppHandle) -> Result<(), String> {
    let accounts = config::load_accounts_with_passwords().map_err(err)?;
    spawn_watchers(&app, accounts).await;
    Ok(())
}

#[tauri::command]
fn get_status(store: State<status::StatusStore>) -> Vec<status::ConnStatus> {
    store.all()
}

#[tauri::command]
fn get_log(buffer: State<log::LogBuffer>) -> Vec<log::LogEntry> {
    buffer.all()
}

#[tauri::command]
fn clear_log(buffer: State<log::LogBuffer>) {
    buffer.clear();
}

// ---------------------------------------------------------------- OTP rules (settings)

#[tauri::command]
fn get_rules(state: State<AppState>) -> Vec<rules::DetectionRule> {
    config::get_rules(&state.config)
}

#[tauri::command]
fn save_rule(state: State<AppState>, rule: rules::DetectionRule) -> Result<Vec<rules::DetectionRule>, String> {
    config::save_rule(&state.config, rule).map(|r| state.publish_rules(r)).map_err(err)
}

#[tauri::command]
fn delete_rule(state: State<AppState>, id: String) -> Result<Vec<rules::DetectionRule>, String> {
    config::delete_rule(&state.config, &id).map(|r| state.publish_rules(r)).map_err(err)
}

#[tauri::command]
fn move_rule(state: State<AppState>, id: String, delta: i32) -> Result<Vec<rules::DetectionRule>, String> {
    config::move_rule(&state.config, &id, delta).map(|r| state.publish_rules(r)).map_err(err)
}

#[tauri::command]
fn export_rules(rules: Vec<rules::DetectionRule>) -> Result<String, String> {
    rule_store::export(&rules).map_err(err)
}

#[tauri::command]
fn preview_import(state: State<AppState>, json: String) -> Result<Vec<rule_store::ImportItem>, String> {
    config::preview_import(&state.config, &json).map_err(err)
}

#[tauri::command]
fn apply_import(state: State<AppState>, json: String, labels: Vec<String>, tags: Vec<String>) -> Result<Vec<rules::DetectionRule>, String> {
    config::apply_import(&state.config, &json, &labels, &tags).map(|r| state.publish_rules(r)).map_err(err)
}

#[tauri::command]
fn test_rule(rule: rules::DetectionRule, from: String, subject: String, body: String) -> Result<rules::MatchTrace, String> {
    rules::test_rule(&rule, &from, &subject, &body).map_err(err)
}

#[tauri::command]
fn get_default_rules() -> Vec<rules::DetectionRule> {
    rules::default_rules()
}

#[tauri::command]
async fn get_catalog() -> Result<catalog::LoadedCatalog, String> {
    catalog::load().await.map_err(err)
}

#[derive(serde::Serialize)]
struct ApiSettings {
    config: config::ApiConfig,
    token: String,
    status: api::ApiStatus,
}

#[tauri::command]
fn get_api_settings(state: State<AppState>, server: State<api::ApiServer>) -> Result<ApiSettings, String> {
    Ok(ApiSettings { config: config::get_api_config(&state.config), token: config::api_token().map_err(err)?, status: server.status() })
}

#[tauri::command]
async fn set_api_settings(app: AppHandle, api: config::ApiConfig) -> Result<(), String> {
    let shared = app.state::<AppState>().config.clone();
    async_runtime::spawn_blocking(move || config::set_api_config(&shared, api)).await.map_err(|e| e.to_string())?.map_err(err)?;
    api::restart(app).await;
    Ok(())
}

#[tauri::command]
fn regenerate_api_token(server: State<api::ApiServer>) -> Result<String, String> {
    let token = config::regenerate_api_token().map_err(err)?;
    server.set_token(token.clone());
    Ok(token)
}

// ---------------------------------------------------------------- Files & misc

/// Asks for a target file and writes `content`. Returns the path, `None` if cancelled.
#[tauri::command]
async fn save_json_file(app: AppHandle, file_name: String, content: String) -> Result<Option<String>, String> {
    let picked = async_runtime::spawn_blocking(move || app.dialog().file().add_filter("JSON", &["json"]).set_file_name(file_name).blocking_save_file())
        .await
        .map_err(|e| e.to_string())?;
    let Some(path) = picked else { return Ok(None) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

/// Asks for a JSON file and returns its content, `None` if cancelled.
#[tauri::command]
async fn open_json_file(app: AppHandle) -> Result<Option<String>, String> {
    let picked = async_runtime::spawn_blocking(move || app.dialog().file().add_filter("JSON", &["json"]).blocking_pick_file())
        .await
        .map_err(|e| e.to_string())?;
    let Some(path) = picked else { return Ok(None) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    std::fs::read_to_string(&path).map(Some).map_err(|e| format!("{}: {e}", path.display()))
}

#[tauri::command]
fn show_about(app: AppHandle) {
    windows::show(&app, "about");
}

/// Hides the calling window (Esc in About and Settings); closing only hides them anyway.
#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn get_app_info(app: AppHandle) -> serde_json::Value {
    serde_json::json!({
        "name": "Maillet",
        "tagline": crate::base::i18n::t("status.tagline", &[]),
        "version": app.package_info().version.to_string(),
    })
}

// ---------------------------------------------------------------- Watchers

const RETRY_MIN: Duration = Duration::from_secs(5);
const RETRY_MAX: Duration = Duration::from_secs(5 * 60);
/// A connection that lived this long counts as healthy; the backoff restarts.
const HEALTHY_AFTER: Duration = Duration::from_secs(60);

/// (Re)starts one watcher per account.
async fn spawn_watchers(app: &AppHandle, accounts: Vec<AccountConfig>) {
    let state = app.state::<AppState>();
    let mut tasks = state.tasks.lock().await;
    while let Some(task) = tasks.pop() {
        let _ = task.stop_tx.send(());
    }
    state.store.clear();
    let Some(pipeline) = state.pipeline.get().cloned() else { return };

    for account in accounts {
        if account.server.is_empty() || account.user.is_empty() {
            status::set(app, &account.id, "error", "Server or username missing");
            continue;
        }
        if account.pass.is_empty() {
            status::set(app, &account.id, "error", "No password stored – edit the account and enter it");
            continue;
        }
        let label = account.label.clone();
        let (commands, warm_account) = (state.commands.clone(), account.clone());
        let watch = Watch {
            app: app.clone(),
            account,
            check: state.check_now.clone(),
            last_uid: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            store: state.store.clone(),
            hook: pipeline.clone() as Arc<dyn MailHook>,
            kinds: state.kinds.clone(),
        };
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        async_runtime::spawn(async move {
            tokio::select! {
                _ = run_with_reconnect(watch) => {}
                _ = keep_warm(commands, warm_account) => {}
                _ = stop_rx => tracing::info!("[{label}] watcher stopped"),
            }
        });
        tasks.push(WatchTask { stop_tx });
    }
}

/// Runs the watcher forever, reconnecting with exponential backoff.
/// A manual check skips the remaining wait. Notifies only on the first
/// failure of a streak to avoid notification spam.
/// Keeps the command connection open (login right after start, then a NOOP
/// before the server or the idle limit drops it), so the window answers at once.
async fn keep_warm(commands: Arc<Commands>, account: config::AccountConfig) {
    tokio::time::sleep(Duration::from_secs(3)).await;
    loop {
        if let Err(e) = commands.keep_warm(&account).await {
            tracing::debug!("[{}] keep-warm failed: {e:#}", account.label);
        }
        tokio::time::sleep(Duration::from_secs(150)).await;
    }
}

async fn run_with_reconnect(watch: Watch) {
    let (app, label, id) = (watch.app.clone(), watch.account.label.clone(), watch.account.id.clone());
    let mut delay = RETRY_MIN;
    let mut failures = 0u32;
    loop {
        let started = Instant::now();
        let Err(e) = imap::run_watcher(&watch).await else { continue };
        if started.elapsed() > HEALTHY_AFTER {
            delay = RETRY_MIN;
            failures = 0;
        }
        failures += 1;
        tracing::error!("[{label}] connection failed (attempt {failures}): {e:#}");
        log::event(&app, &id, &label, "error", format!("{e:#} – retry {failures} in {}s", delay.as_secs()));
        if failures == 1 {
            crate::base::notify::show(format!("Maillet – {label}"), crate::base::i18n::t("notify.failed", &[("e", &format!("{e:#}"))]));
        }
        status::set(&app, &id, "error", format!("{e:#} – retrying in {}s", delay.as_secs()));
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = watch.check.notified() => log::event(&app, &id, &label, "info", "Manual check – reconnecting now"),
        }
        delay = (delay * 2).min(RETRY_MAX);
    }
}

/// Tooltip: total and per account, plus connection problems.
fn tray_tooltip(app: &AppHandle) -> String {
    let snap = app.state::<AppState>().store.snapshot();
    let total: usize = snap.accounts.iter().map(|a| a.unread_total).sum();
    let errors = app.state::<status::StatusStore>().all().into_iter().filter(|s| s.state == "error").count();
    use crate::base::i18n::t;
    let mut text = match total {
        0 => t("tip.none", &[]),
        1 => t("tip.one", &[]),
        n => t("tip.n", &[("n", &n.to_string())]),
    };
    let state = app.state::<AppState>();
    if state.tray_people_only.load(std::sync::atomic::Ordering::Relaxed) {
        text = t("tip.people", &[("p", &state.store.unread_people().to_string()), ("n", &total.to_string())]);
    }
    if snap.accounts.len() > 1 && total > 0 {
        let parts: Vec<String> = snap.accounts.iter().filter(|a| a.unread_total > 0).map(|a| format!("{} {}", a.account, a.unread_total)).collect();
        text += &format!(" ({})", parts.join(", "));
    }
    if errors > 0 {
        text += &t("tip.problems", &[("n", &errors.to_string())]);
    }
    text
}

/// Keeps tray and window in step with the mail store.
fn spawn_store_listener(app: AppHandle, tray: Option<tray::TrayHandle>) {
    let mut rx = app.state::<AppState>().store.subscribe();
    async_runtime::spawn(async move {
        loop {
            if rx.changed().await.is_err() {
                return;
            }
            let state = app.state::<AppState>();
            let count = if state.tray_people_only.load(std::sync::atomic::Ordering::Relaxed) { state.store.unread_people() } else { state.store.unread_total() };
            if let Some(tray) = &tray {
                tray::update(tray, count, tray_tooltip(&app)).await;
            }
            let _ = app.emit("mails-changed", count);
        }
    });
}

// ---------------------------------------------------------------- Start

pub fn run() {
    // `RUST_LOG=maillet=debug` shows IDLE wake-ups.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    let accounts = config::load_accounts_with_passwords().unwrap_or_default();
    let config = config::load_config().unwrap_or_default();
    crate::base::i18n::set_language(config.language.as_deref().unwrap_or(crate::base::i18n::DEFAULT));
    let (rules, _) = watch::channel(config.rules.clone());
    let notifications = Arc::new(std::sync::atomic::AtomicBool::new(config.notifications.unwrap_or(true)));
    let tray_people_only = Arc::new(std::sync::atomic::AtomicBool::new(config.tray_people_only.unwrap_or(false)));
    let kinds = Arc::new(crate::mail::classify::SenderKinds::new(config::sender_kinds(&config)));
    let state = AppState {
        config: Arc::new(Mutex::new(config)),
        tasks: Mutex::new(Vec::new()),
        check_now: Arc::new(Notify::new()),
        rules,
        store: Arc::new(MailStore::default()),
        commands: Arc::new(Commands::default()),
        pipeline: OnceLock::new(),
        notifications,
        tray_people_only,
        kinds,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(shortcut::plugin())
        .manage(state)
        .manage(shortcut::ShortcutStatus::default())
        .manage(status::StatusStore::default())
        .manage(log::LogBuffer::default())
        .manage(popup::LastCode::default())
        .manage(api::ApiServer::default())
        .manage(windows::MainWindowState::default())
        .invoke_handler(tauri::generate_handler![
            list_mails, get_app_status, get_mail, set_seen, archive_mail, delete_mail, check_now, open_window, hide_main, set_notifications, set_tray_people_only, set_sender_kind, archive_mails, unarchive_mails, set_shortcut, get_i18n, set_language, set_pinned, make_task, list_tasks, list_inbox, set_inbox_days, move_task, file_mail, unfile_mail, set_categories, list_folders, set_mark_read_on_open, trash_mail, set_actions, get_task_mail,
            get_accounts, add_account, update_account, remove_account, graph_sign_in_start, graph_sign_in_finish, graph_sign_in_cancel, rename_tag, test_connection, restart_imap,
            get_status, get_log, clear_log,
            get_rules, save_rule, delete_rule, move_rule, export_rules, preview_import, apply_import, test_rule,
            get_default_rules, get_catalog, get_api_settings, set_api_settings, regenerate_api_token,
            save_json_file, open_json_file, show_about, hide_window, get_app_info,
            popup::get_last_code, popup::copy_code, popup::open_link, popup::hide_code_popup, popup::resize_code_popup,
        ])
        .setup(move |app| {
            windows::create(app)?;
            let handle = app.handle().clone();
            let keys = handle.state::<AppState>().config.blocking_lock().shortcut.clone();
            shortcut::register(&handle, keys);

            // Features: OTP is one of them.
            let state = handle.state::<AppState>();
            // Features in order; notifications last, so they see what OTP found.
            let otp = otp::OtpFeature::new(handle.clone(), state.rules.subscribe());
            let notify = crate::features::notify::NotifyFeature::new(handle.clone(), state.notifications.clone());
            let _ = state.pipeline.set(Arc::new(Pipeline::new(vec![Arc::new(otp), Arc::new(notify)])));

            async_runtime::spawn(api::restart(handle.clone()));
            async_runtime::spawn(async move {
                let tray = tray::spawn(handle.clone()).await;
                if tray.is_none() {
                    tray::no_tray_fallback(&handle);
                }
                spawn_store_listener(handle.clone(), tray);
                spawn_watchers(&handle, accounts).await;
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let RunEvent::ExitRequested { .. } = event {}
        });
}
