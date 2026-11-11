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
