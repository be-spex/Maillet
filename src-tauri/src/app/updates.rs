//! Updates for the AppImage: at start, Tauri's updater looks for a newer
//! signed release (latest.json of the GitHub release) and offers to install
//! it. Installed as a .deb, APT keeps the app current, so nothing is checked.

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

pub fn check_in_background(app: &AppHandle) {
    if std::env::var_os("APPIMAGE").is_none() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = offer(app).await {
            eprintln!("update check failed: {e}");
        }
    });
}

async fn offer(app: AppHandle) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    let name = app.config().product_name.clone().unwrap_or_default();
    let text = format!(
        "{name} {} is available (installed: {}).\n\nInstall it now? {name} restarts afterwards.",
        update.version, update.current_version
    );
    let handle = app.clone();
    app.dialog()
        .message(text)
        .title("Update available")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom("Install and Restart".into(), "Later".into()))
        .show(move |install| {
            if !install {
                return;
            }
            tauri::async_runtime::spawn(async move {
                match update.download_and_install(|_, _| {}, || {}).await {
                    Ok(()) => handle.restart(),
                    Err(e) => handle
                        .dialog()
                        .message(format!("The update could not be installed:\n{e}"))
                        .title("Update failed")
                        .kind(MessageDialogKind::Error)
                        .show(|_| {}),
                }
            });
        });
    Ok(())
}
