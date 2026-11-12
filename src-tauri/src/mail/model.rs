//! Data model of the mail core: what Maillet knows about a mail.

use serde::Serialize;

/// Decoded mail text, as far as features and the reader need it.
/// `html` still contains the `href` targets (needed for link detection).
#[derive(Debug, Default, Clone)]
pub struct MailBody {
    pub text: String,
    pub html: String,
}

/// Result of a feature looking at a mail, e.g. a one-time code.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Feature id, e.g. `otp`.
    pub feature: String,
    /// What was found, e.g. `code` or `link`.
    pub kind: String,
    pub value: String,
    /// Human-readable source, e.g. the rule name.
    pub label: String,
}

/// Everything the mail list needs, without the full text.
#[derive(Debug, Clone, Serialize)]
pub struct MailSummary {
    /// Stable within a mailbox generation: `account:uidvalidity:uid`.
    pub id: String,
    pub account_id: String,
    pub account: String,
    pub uid: u32,
    pub from_name: Option<String>,
    pub from_addr: Option<String>,
    pub subject: String,
    /// Unix timestamp (seconds) from the `Date` header, if parseable.
    pub date: Option<i64>,
    pub preview: String,
    pub has_attachments: bool,
    /// Read (`\Seen`) on the server.
    pub seen: bool,
    /// Pinned (`\Flagged`) on the server.
    pub flagged: bool,
    /// `Message-ID` header, used to find the mail again after moving it.
    pub message_id: Option<String>,
    /// Newsletter or other bulk mail (`List-Unsubscribe`, `Precedence: bulk`, …).
    pub bulk: bool,
    pub findings: Vec<Finding>,
}

/// An attachment as the reader lists it; `index` addresses it for opening and saving.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Attachment {
    pub index: usize,
    pub name: String,
    /// MIME type, e.g. `application/pdf`.
    pub mime: String,
    /// Decoded size in bytes.
    pub size: usize,
    /// Small enough to be viewed from a temporary copy; larger ones can only be saved.
    pub viewable: bool,
}

/// Full content for the reader.
#[derive(Clone, Serialize)]
pub struct MailContent {
    pub text: String,
    /// Sanitized HTML, if the mail has an HTML part.
    pub html: Option<String>,
    /// Remote images in `html`, blocked until loaded on request.
    pub remote_images: usize,
    pub attachments: Vec<Attachment>,
    /// Key of the raw mail in the app's attachment cache (set by the app layer).
    pub key: String,
    /// The raw mail, kept so attachments can be opened without fetching it again.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

impl std::fmt::Debug for MailContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailContent").field("text", &self.text).field("attachments", &self.attachments).field("raw_len", &self.raw.len()).finish_non_exhaustive()
    }
}
