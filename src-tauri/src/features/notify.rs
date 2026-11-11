//! Desktop notifications for new mails from people. Newsletters and other
//! bulk mail stay silent, and mails the OTP feature already showed as a code
//! are not announced twice. A burst of mails is collapsed into one summary.

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use super::Feature;
use crate::app::windows;
use crate::base::config::AccountConfig;
use crate::base::notify;
use crate::mail::model::{Finding, MailBody, MailSummary};

/// At most this many single notifications per window …
const BURST: usize = 3;
const WINDOW: Duration = Duration::from_secs(30);
const BODY_LEN: usize = 140;

pub struct NotifyFeature {
    app: AppHandle,
    enabled: Arc<AtomicBool>,
    burst: Arc<Mutex<Burst>>,
}

#[derive(Default)]
struct Burst {
    started: Option<Instant>,
    shown: usize,
    /// Mails beyond the limit, announced together when the window ends.
    held_back: usize,
}

impl NotifyFeature {
    pub fn new(app: AppHandle, enabled: Arc<AtomicBool>) -> Self {
        Self { app, enabled, burst: Arc::default() }
    }

    /// True if the mail may get its own notification; otherwise it is counted
    /// for the summary.
    fn admit(&self) -> bool {
        let mut b = self.burst.lock().unwrap();
        if b.started.is_none_or(|t| t.elapsed() > WINDOW) {
            *b = Burst { started: Some(Instant::now()), ..Default::default() };
        }
        if b.shown < BURST {
            b.shown += 1;
            return true;
        }
        b.held_back += 1;
        if b.held_back == 1 {
            let (burst, app) = (self.burst.clone(), self.app.clone());
            let wait = WINDOW.saturating_sub(b.started.map_or(Duration::ZERO, |t| t.elapsed()));
            std::thread::spawn(move || {
                std::thread::sleep(wait);
                let n = std::mem::take(&mut burst.lock().unwrap().held_back);
                if n > 0 {
                    let text = if n == 1 { "1 weitere neue Mail".to_string() } else { format!("{n} weitere neue Mails") };
                    notify::show_clickable("Maillet", text, move || windows::show_main(&app, None));
                }
            });
        }
        false
    }
}

#[async_trait]
impl Feature for NotifyFeature {
    fn id(&self) -> &'static str {
        "notify"
    }

    async fn analyse(&self, account: &AccountConfig, mail: &MailSummary, _body: Option<&MailBody>) -> Vec<Finding> {
        let code_shown = mail.findings.iter().any(|f| f.feature == "otp");
        if !self.enabled.load(Ordering::Relaxed) || mail.bulk || code_shown || !self.admit() {
            return vec![];
        }
        let sender = mail.from_name.clone().or_else(|| mail.from_addr.clone()).unwrap_or_else(|| "Neue Mail".into());
        let subject = if mail.subject.is_empty() { "(kein Betreff)" } else { &mail.subject };
        let mut body = subject.to_string();
        if !mail.preview.is_empty() {
            let preview: String = mail.preview.chars().take(BODY_LEN).collect();
            body += &format!("\n{preview}{}", if mail.preview.chars().count() > BODY_LEN { "…" } else { "" });
        }
        let (app, id) = (self.app.clone(), mail.id.clone());
        notify::show_clickable(format!("{sender} · {}", account.label), body, move || {
            windows::show_main(&app, None);
            let _ = app.emit_to(windows::MAIN, "open-mail", id);
        });
        vec![]
    }
}
