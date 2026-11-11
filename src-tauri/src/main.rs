//! Maillet – a lightweight tray mail client.
//!
//! Layers (dependencies point downwards only):
//! - `app`      tray, windows, Tauri commands
//! - `features` look at new mails (OTP detection is one of them)
//! - `mail`     the core: check mailboxes, keep unread mails, read and move them
//! - `base`     configuration, keyring, log, status

mod app;
mod base;
mod features;
mod mail;

fn main() {
    app::run();
}
