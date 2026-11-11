//! Global shortcut that opens and closes the window (X11; Wayland compositors
//! do not allow global key grabs – there a desktop shortcut can start Maillet).

use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use super::windows;

pub const DEFAULT: &str = "Ctrl+Alt+M";

#[derive(Debug, Clone, Default, Serialize)]
pub struct Status {
    /// Configured keys; empty if switched off.
    pub keys: String,
    /// Why the shortcut could not be registered.
    pub error: Option<String>,
}

#[derive(Default)]
pub struct ShortcutStatus(Mutex<Status>);

impl ShortcutStatus {
    pub fn get(&self) -> Status {
        self.0.lock().unwrap().clone()
    }
}

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                windows::toggle_main(app.app_handle(), None);
            }
        })
        .build()
}

/// Registers the configured shortcut (or the default) and records the outcome.
pub fn register(app: &AppHandle, configured: Option<String>) {
    let keys = configured.unwrap_or_else(|| DEFAULT.to_string()).trim().to_string();
    let error = if keys.is_empty() {
        None
    } else {
        match keys.parse::<Shortcut>() {
            Err(e) => Some(format!("Invalid shortcut: {e}")),
            Ok(shortcut) => app.global_shortcut().register(shortcut).err().map(|e| e.to_string()),
        }
    };
    match &error {
        None if !keys.is_empty() => tracing::info!("Global shortcut {keys} registered"),
        Some(e) => tracing::warn!("Global shortcut {keys} not available: {e}"),
        _ => {}
    }
    *app.state::<ShortcutStatus>().0.lock().unwrap() = Status { keys, error };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_shortcut_parses() {
        assert!(DEFAULT.parse::<Shortcut>().is_ok());
        assert!("Super+Shift+F12".parse::<Shortcut>().is_ok());
        assert!("Ctrl+Alt+".parse::<Shortcut>().is_err());
    }
}
