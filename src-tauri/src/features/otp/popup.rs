//! The code popup: a small always-on-top window listing found codes and links.

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

pub const POPUP_WIDTH: f64 = 400.0;

/// Last received code, so the popup can fetch it after (re)loading.
#[derive(Default)]
pub struct LastCode(std::sync::Mutex<Option<String>>);

#[tauri::command]
pub fn get_last_code(last: State<LastCode>) -> Option<String> {
    last.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn copy_code(app: AppHandle, code: String) -> Result<(), String> {
    app.clipboard().write_text(code).map_err(|e| e.to_string())
}

/// Opens a link in the default browser (or a `mailto:` link in the mail program).
#[tauri::command]
pub fn open_link(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")) {
        return Err("Only http(s) and mailto links can be opened".into());
    }
    crate::base::open::xdg_open(&url).map_err(|e| format!("Failed to open browser: {e}"))
}

#[tauri::command]
pub fn hide_code_popup(app: AppHandle) {
    if let Some(popup) = app.get_webview_window("code") {
        let _ = popup.hide();
    }
}

/// The popup reports its content height when entries are added/removed.
#[tauri::command]
pub fn resize_code_popup(app: AppHandle, height: f64) {
    let Some(popup) = app.get_webview_window("code") else { return };
    let max = popup
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.work_area().size.height as f64 / m.scale_factor() * 0.85)
        .unwrap_or(900.0);
    let _ = popup.set_size(tauri::LogicalSize::new(POPUP_WIDTH, height.clamp(120.0, max)));
    center_popup(&popup);
}

/// Centers the popup on the primary monitor's work area.
fn center_popup(popup: &tauri::WebviewWindow) {
    if let (Ok(Some(monitor)), Ok(size)) = (popup.primary_monitor(), popup.outer_size()) {
        let area = monitor.work_area();
        let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
        let y = area.position.y + (area.size.height as i32 - size.height as i32) / 2;
        let _ = popup.set_position(tauri::PhysicalPosition::new(x, y));
    }
}

/// Adds an entry to the code popup, shows it and tells the main window.
pub fn show(app: &AppHandle, payload: String) {
    let _ = app.emit("auth-code", &payload);
    *app.state::<LastCode>().0.lock().unwrap() = Some(payload.clone());
    let Some(popup) = app.get_webview_window("code") else { return };
    let _ = popup.emit_to("code", "show-code", &payload);
    let _ = popup.show();
    // Position after show(): GTK ignores moves of hidden windows.
    center_popup(&popup);
    let _ = popup.set_always_on_top(true);
    let _ = popup.set_focus();
}
