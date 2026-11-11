//! One-time codes and login links: the original OTP Scout function, now one
//! feature among others. Rules, catalog, local API and the code popup.

pub mod api;
pub mod catalog;
pub mod popup;
pub mod rule_store;
pub mod rules;

use async_trait::async_trait;
use tauri::AppHandle;
use tokio::sync::watch;

use super::Feature;
use crate::base::config::AccountConfig;
use crate::base::log;
use crate::mail::model::{Finding, MailBody, MailSummary};
use rules::{CompiledRules, DetectionRule, HeaderMatch, RuleKind, Stage};

pub struct OtpFeature {
    app: AppHandle,
    rules: watch::Receiver<Vec<DetectionRule>>,
}

impl OtpFeature {
    pub fn new(app: AppHandle, rules: watch::Receiver<Vec<DetectionRule>>) -> Self {
        Self { app, rules }
    }

    /// The rules this account checks; the built-in default if none is assigned.
    fn rules_for(&self, account: &AccountConfig) -> CompiledRules {
        let own = rule_store::rules_for(&self.rules.borrow(), &account.tag);
        CompiledRules::compile(&if own.is_empty() { rules::default_rules() } else { own })
    }
}

#[async_trait]
impl Feature for OtpFeature {
    fn id(&self) -> &'static str {
        "otp"
    }

    fn wants_body(&self, account: &AccountConfig, mail: &MailSummary) -> bool {
        let from = mail.from_addr.as_deref().unwrap_or("");
        self.rules_for(account).rules.iter().any(|r| r.match_headers(from, &mail.subject) == Some(HeaderMatch::NeedsBody))
    }

    async fn analyse(&self, account: &AccountConfig, mail: &MailSummary, body: Option<&MailBody>) -> Vec<Finding> {
        let from = mail.from_addr.as_deref().unwrap_or("");
        let mut entry = log::MailEntry {
            uid: mail.uid,
            from: mail.from_addr.clone(),
            subject: Some(mail.subject.clone()),
            outcome: log::Outcome::NoMatch,
            kind: None,
            value: None,
            traces: vec![],
            timings: log::Timings::default(),
        };
        let mut found = vec![];
        for rule in &self.rules_for(account).rules {
            let trace = rule.trace(from, &mail.subject, body);
            let value = trace.value.clone();
            let headers_ok = trace.stages.iter().any(|s| s.stage == Stage::Subject && s.ok);
            entry.traces.push(trace);
            if let Some(code) = value {
                tracing::info!("[{}] OTP {:?} found (rule: {})", account.label, rule.kind, rule.label);
                popup::show(&self.app, payload(&code, rule.kind, &rule.label, account, mail));
                entry.outcome = log::Outcome::Found;
                entry.kind = Some(rule.kind);
                entry.value = Some(code.clone());
                let kind = if rule.kind == RuleKind::Link { "link" } else { "code" };
                found.push(Finding { feature: "otp".into(), kind: kind.into(), value: code, label: rule.label.clone() });
                break;
            }
            if headers_ok {
                entry.outcome = log::Outcome::Partial;
            }
        }
        log::mail(&self.app, &account.id, &account.label, entry);
        found
    }
}

/// What the code popup and the main window receive.
fn payload(code: &str, kind: RuleKind, rule: &str, account: &AccountConfig, mail: &MailSummary) -> String {
    serde_json::json!({
        "code": code,
        "kind": kind,
        "account": account.label,
        "account_id": account.id,
        "uid": mail.uid,
        "rule": rule,
        "from": mail.from_addr,
        "subject": mail.subject,
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
    })
    .to_string()
}
