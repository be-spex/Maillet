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
    let Snapshot { accounts, mails } = state.store.snapshot();
    serde_json::json!({ "accounts": accounts, "mails": mails, "status": statuses.all() })
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

/// The window reports the height its content needs; the popup adapts to it.
#[tauri::command]
fn fit_main(app: AppHandle, height: f64) {
    windows::fit_content(&app, height);
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
        "tagline": "neue Mails im Blick, direkt im Tray",
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
                _ = stop_rx => tracing::info!("[{label}] watcher stopped"),
            }
        });
        tasks.push(WatchTask { stop_tx });
    }
}

/// Runs the watcher forever, reconnecting with exponential backoff.
/// A manual check skips the remaining wait. Notifies only on the first
/// failure of a streak to avoid notification spam.
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
            crate::base::notify::show(format!("Maillet – {label}"), format!("Verbindung fehlgeschlagen: {e:#}"));
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
    let mut text = match total {
        0 => "Maillet – keine neuen Mails".to_string(),
        1 => "Maillet – 1 neue Mail".to_string(),
        n => format!("Maillet – {n} neue Mails"),
    };
    let state = app.state::<AppState>();
    if state.tray_people_only.load(std::sync::atomic::Ordering::Relaxed) {
        text = format!("Maillet – {} von Menschen, {total} insgesamt", state.store.unread_people());
    }
    if snap.accounts.len() > 1 && total > 0 {
        let parts: Vec<String> = snap.accounts.iter().filter(|a| a.unread_total > 0).map(|a| format!("{} {}", a.account, a.unread_total)).collect();
        text += &format!(" ({})", parts.join(", "));
    }
    if errors > 0 {
        text += &format!(" · {errors} Konto/Konten mit Verbindungsproblem");
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
            list_mails, get_app_status, get_mail, set_seen, archive_mail, delete_mail, check_now, open_window, hide_main, fit_main, set_notifications, set_tray_people_only, set_sender_kind, archive_mails, unarchive_mails,
            get_accounts, add_account, update_account, remove_account, rename_tag, test_connection, restart_imap,
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
