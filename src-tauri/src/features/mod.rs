//! Features look at new mails and enrich them with findings. OTP detection is
//! one of them; topics, status cards or AI will plug in the same way.

pub mod notify;
pub mod otp;

use async_trait::async_trait;
use std::sync::Arc;

use crate::base::config::AccountConfig;
use crate::mail::imap::MailHook;
use crate::mail::model::{Finding, MailBody, MailSummary};

#[async_trait]
pub trait Feature: Send + Sync {
    /// Short id, e.g. `otp`; also used in findings.
    fn id(&self) -> &'static str;
    /// Headers not enough? The pipeline then loads the body – once for all features.
    fn wants_body(&self, _account: &AccountConfig, _mail: &MailSummary) -> bool {
        false
    }
    /// Looks at a new mail. Async so that slow features (e.g. AI) fit in later.
    async fn analyse(&self, account: &AccountConfig, mail: &MailSummary, body: Option<&MailBody>) -> Vec<Finding>;
}

/// All active features, called in order for every new mail.
pub struct Pipeline {
    features: Vec<Arc<dyn Feature>>,
}

impl Pipeline {
    pub fn new(features: Vec<Arc<dyn Feature>>) -> Self {
        Self { features }
    }
}

#[async_trait]
impl MailHook for Pipeline {
    fn wants_body(&self, account: &AccountConfig, mail: &MailSummary) -> bool {
        self.features.iter().any(|f| f.wants_body(account, mail))
    }

    async fn on_new_mail(&self, account: &AccountConfig, mail: &mut MailSummary, body: Option<&MailBody>) {
        for feature in &self.features {
            let found = feature.analyse(account, mail, body).await;
            if !found.is_empty() {
                tracing::debug!("[{}] feature {} found {} item(s) in UID {}", account.label, feature.id(), found.len(), mail.uid);
            }
            mail.findings.extend(found);
        }
    }
}
