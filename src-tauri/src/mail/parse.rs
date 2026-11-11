//! Turns raw IMAP data into the mail model (folded headers, RFC 2047, MIME, charsets).

use mail_parser::{Message, MessageParser, MimeHeaders};

use super::model::{MailBody, MailContent, MailSummary};

const PREVIEW_LEN: usize = 220;

/// Where a mail lives; part of its id.
pub struct Origin<'a> {
    pub account_id: &'a str,
    pub account: &'a str,
    pub uid_validity: u32,
    pub uid: u32,
}

/// Builds the list entry from the header and the first bytes of the body.
/// `text_start` may be cut off anywhere; the parser copes with that.
pub fn summarize(origin: &Origin, header: &[u8], text_start: &[u8]) -> MailSummary {
    let mut raw = Vec::with_capacity(header.len() + text_start.len() + 128);
    raw.extend_from_slice(header);
    raw.extend_from_slice(text_start);
    close_multiparts(&mut raw);
    let parsed = MessageParser::default().parse(&raw);
    let from = parsed.as_ref().and_then(|m| m.from()).and_then(|a| a.first());
    MailSummary {
        id: format!("{}:{}:{}", origin.account_id, origin.uid_validity, origin.uid),
        account_id: origin.account_id.to_string(),
        account: origin.account.to_string(),
        uid: origin.uid,
        from_name: from.and_then(|a| a.name()).map(str::to_string).filter(|n| !n.trim().is_empty()),
        from_addr: from.and_then(|a| a.address()).map(str::to_string),
        subject: parsed.as_ref().and_then(|m| m.subject()).unwrap_or("").to_string(),
        date: parsed.as_ref().and_then(|m| m.date()).map(|d| d.to_timestamp()),
        preview: parsed.as_ref().map(preview).unwrap_or_default(),
        has_attachments: parsed.as_ref().is_some_and(looks_like_attachments),
        seen: false,
        flagged: false,
        message_id: parsed.as_ref().and_then(|m| m.message_id()).map(str::to_string),
        bulk: parsed.as_ref().is_some_and(is_bulk),
        findings: vec![],
    }
}

/// A cut-off body lacks the closing MIME boundaries, and the parser then
/// finds no text part. Closes every boundary seen, innermost first.
fn close_multiparts(raw: &mut Vec<u8>) {
    static BOUNDARY: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r#"(?i)boundary\s*=\s*"?([^";\r\n]+)"?"#).unwrap());
    let text = String::from_utf8_lossy(raw).into_owned();
    let mut seen: Vec<String> = vec![];
    for cap in BOUNDARY.captures_iter(&text) {
        let b = cap[1].trim().to_string();
        if !seen.contains(&b) && !text.contains(&format!("--{b}--")) {
            seen.push(b);
        }
    }
    for b in seen.iter().rev() {
        raw.extend_from_slice(format!("\r\n--{b}--\r\n").as_bytes());
    }
}

/// First readable words of the text, quotes and line breaks removed.
fn preview(msg: &Message) -> String {
    let text = match msg.text_part(0) {
        Some(part) if !part.is_text_html() => msg.body_text(0).map(|t| t.into_owned()).unwrap_or_default(),
        _ => msg.body_html(0).map(|h| crate::base::html::strip_html(&h)).unwrap_or_default(),
    };
    let joined = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('>'))
        .collect::<Vec<_>>()
        .join(" ");
    let mut out: String = joined.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.chars().count() > PREVIEW_LEN {
        out = out.chars().take(PREVIEW_LEN).collect::<String>() + "…";
    }
    out
}

/// Newsletters, mailing lists, automatic mails.
fn is_bulk(msg: &Message) -> bool {
    let precedence = msg.header_raw("Precedence").unwrap_or("").trim().to_ascii_lowercase();
    let auto = msg.header_raw("Auto-Submitted").unwrap_or("no").trim().to_ascii_lowercase();
    msg.header_raw("List-Unsubscribe").is_some()
        || msg.header_raw("List-Id").is_some()
        || matches!(precedence.as_str(), "bulk" | "list" | "junk")
        || !auto.starts_with("no")
}

/// Only the header is known for sure: `multipart/mixed` usually carries attachments.
fn looks_like_attachments(msg: &Message) -> bool {
    msg.content_type()
        .is_some_and(|ct| ct.c_type.eq_ignore_ascii_case("multipart") && ct.subtype().is_some_and(|s| s.eq_ignore_ascii_case("mixed")))
}

/// Decodes the subject (folded lines, RFC 2047 encoded words, any charset).
#[cfg(test)]
pub fn parse_subject(header_bytes: &[u8]) -> Option<String> {
    let msg = MessageParser::default().parse_headers(header_bytes)?;
    msg.subject().map(str::to_string)
}

#[cfg(test)]
pub fn parse_from(header_bytes: &[u8]) -> Option<String> {
    let msg = MessageParser::default().parse_headers(header_bytes)?;
    msg.from()?.first()?.address().map(str::to_string)
}

/// Decodes a full RFC 822 message (multipart, quoted-printable, base64, charsets).
pub fn parse_body(raw: &[u8]) -> MailBody {
    let Some(msg) = MessageParser::default().parse(raw) else {
        return MailBody { text: String::from_utf8_lossy(raw).into_owned(), html: String::new() };
    };
    let html = msg.body_html(0).map(|h| h.into_owned()).unwrap_or_default();
    let text = match msg.text_part(0) {
        Some(part) if !part.is_text_html() => msg.body_text(0).map(|t| t.into_owned()).unwrap_or_default(),
        _ => crate::base::html::strip_html(&html),
    };
    MailBody { text, html }
}

/// Text and attachment names for the reader.
pub fn parse_content(raw: &[u8]) -> MailContent {
    let body = parse_body(raw);
    let parsed = MessageParser::default().parse(raw);
    let attachments = parsed
        .as_ref()
        .map(|m| m.attachments().filter(|a| a.content_id().is_none() || a.attachment_name().is_some()).map(|a| a.attachment_name().unwrap_or("Attachment").to_string()).collect())
        .unwrap_or_default();
    let safe = parsed.as_ref().filter(|_| !body.html.trim().is_empty()).map(|m| super::sanitize::sanitize(&body.html, &super::sanitize::inline_images(m)));
    MailContent {
        text: body.text,
        remote_images: safe.as_ref().map_or(0, |s| s.remote_images),
        html: safe.map(|s| s.html),
        attachments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: Origin = Origin { account_id: "a1", account: "Arbeit", uid_validity: 7, uid: 42 };

    #[test]
    fn test_parse_headers_folded_utf8_subject() {
        let raw = b"From: Spotify <no-reply@alerts.spotify.com>\r\n\
Subject: =?UTF-8?Q?913088=C2=A0=E2=80=93_dein_Spotify?=\r\n =?UTF-8?Q?_Anmeldecode?=\r\n\r\n";
        let subject = parse_subject(raw).unwrap();
        assert_eq!(subject, "913088\u{a0}\u{2013} dein Spotify Anmeldecode");
        assert_eq!(parse_from(raw).as_deref(), Some("no-reply@alerts.spotify.com"));
    }

    #[test]
    fn test_parse_body_quoted_printable_multipart() {
        let raw = "From: no-reply@mail.anthropic.com\r\n\
Subject: Secure link to log in to Claude.ai\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/alternative; boundary=\"b1\"\r\n\
\r\n\
--b1\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
Your code: 123456 =E2=80=93 thanks\r\n\
--b1\r\n\
Content-Type: text/html; charset=utf-8\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
<a href=3D\"https://claude.ai/magic-link#abcdefghijklmnopqrstuvwxyz0123456789abcdefgh=\r\n\
ijklmnop\">Sign in</a>\r\n\
--b1--\r\n";
        let body = parse_body(raw.as_bytes());
        assert!(body.text.contains("Your code: 123456 – thanks"));
        assert!(body.html.contains("https://claude.ai/magic-link#abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnop\""));
    }

    #[test]
    fn test_summary_from_header_and_cut_off_multipart_text() {
        let header = b"From: =?UTF-8?Q?Jana_W=C3=B6lff?= <jana@firma.example>\r\n\
Subject: Atlas: Review bis Freitag\r\n\
Date: Tue, 10 Nov 2026 09:14:00 +0100\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"x\"\r\n\r\n";
        // Body cut off in the middle, like a partial IMAP fetch.
        let text = b"--x\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nHallo,\r\n\r\n> altes Zitat\r\nanbei die Spezifikation f\xc3\xbcr den neuen Login-Fl";
        let s = summarize(&ORIGIN, header, text);
        assert_eq!(s.id, "a1:7:42");
        assert_eq!(s.from_name.as_deref(), Some("Jana Wölff"));
        assert_eq!(s.from_addr.as_deref(), Some("jana@firma.example"));
        assert_eq!(s.subject, "Atlas: Review bis Freitag");
        assert_eq!(s.date, Some(1794298440));
        assert!(s.preview.starts_with("Hallo, anbei die Spezifikation für den neuen Login-Fl"), "{}", s.preview);
        assert!(!s.preview.contains("Zitat"));
        assert!(s.has_attachments);
        assert!(!s.bulk);
    }

    #[test]
    fn test_summary_nested_multipart_cut_off() {
        let header = b"From: a@b.example\r\nSubject: x\r\nContent-Type: multipart/mixed; boundary=outer\r\n\r\n";
        let text = b"--outer\r\nContent-Type: multipart/alternative; boundary=\"inner\"\r\n\r\n--inner\r\n\
Content-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\nGr=C3=BC=C3=9Fe aus Ham";
        assert_eq!(summarize(&ORIGIN, header, text).preview, "Grüße aus Ham");
    }

    #[test]
    fn test_content_html_is_sanitized_with_inline_image() {
        let raw = "From: a@b.example\r\nSubject: Newsletter\r\nMIME-Version: 1.0\r\n\
Content-Type: multipart/related; boundary=\"r\"\r\n\r\n\
--r\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
<h1 onclick=\"x()\">Hallo</h1><img src=\"cid:logo\"><img src=\"https://t.example/p.gif\"><script>evil()</script>\r\n\
--r\r\nContent-Type: image/png\r\nContent-ID: <logo>\r\nContent-Transfer-Encoding: base64\r\n\r\niVBORw0KGgo=\r\n--r--\r\n";
        let c = parse_content(raw.as_bytes());
        let html = c.html.expect("html part");
        assert!(html.contains("<h1>Hallo</h1>"), "{html}");
        assert!(html.contains("src=\"data:image/png;base64,"), "{html}");
        assert!(!html.contains("evil") && !html.contains("onclick"));
        assert_eq!(c.remote_images, 1);
        assert!(c.attachments.is_empty(), "inline image is no attachment: {:?}", c.attachments);
        assert!(c.text.contains("Hallo"));
    }

    #[test]
    fn test_summary_html_only_and_bulk() {
        let header = b"From: news@shop.example\r\nSubject: Angebote\r\nList-Unsubscribe: <mailto:off@shop.example>\r\n\
Content-Type: text/html; charset=utf-8\r\n\r\n";
        let s = summarize(&ORIGIN, header, b"<html><body><p>Nur <b>heute</b>: alles billiger</p></body></html>");
        assert!(s.bulk);
        assert_eq!(s.from_name, None);
        assert_eq!(s.preview, "Nur heute: alles billiger");
    }

    #[test]
    fn test_auto_submitted_counts_as_bulk() {
        let header = b"From: jira@firma.example\r\nSubject: [ATLAS-1] Update\r\nAuto-Submitted: auto-generated\r\n\r\n";
        assert!(summarize(&ORIGIN, header, b"x").bulk);
    }
}
