//! Desktop notifications.
//!
//! Shown on a plain thread: notify-rust talks D-Bus via a blocking call that
//! starts its own async runtime, which panics inside a Tokio worker (zbus
//! uses Tokio here because ksni enables it). tauri-plugin-notification calls
//! it from a Tokio task, so it is not used for showing notifications.

pub fn show(title: impl Into<String>, body: impl Into<String>) {
    let (title, body) = (title.into(), body.into());
    std::thread::spawn(move || {
        if let Err(e) = notify_rust::Notification::new().appname("Maillet").summary(&title).body(&body).icon("mail-unread").show() {
            tracing::warn!("Notification failed: {e}");
        }
    });
}

/// Notification that runs `on_click` when it is clicked.
pub fn show_clickable(title: impl Into<String>, body: impl Into<String>, on_click: impl FnOnce() + Send + 'static) {
    let (title, body) = (title.into(), body.into());
    std::thread::spawn(move || {
        let shown = notify_rust::Notification::new()
            .appname("Maillet")
            .summary(&title)
            .body(&escape(&body))
            .icon("mail-unread")
            .action("default", &super::i18n::t("notify.open", &[]))
            .show();
        match shown {
            // Blocks this thread until the notification is clicked or closed.
            Ok(handle) => handle.wait_for_action(|action| {
                if action == "default" {
                    on_click();
                }
            }),
            Err(e) => tracing::warn!("Notification failed: {e}"),
        }
    });
}

/// Notification servers interpret a little HTML in the body.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
