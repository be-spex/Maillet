//! Makes mail HTML safe to show: no scripts, forms, frames or event handlers,
//! no remote content unless the user asks for it. The window additionally
//! shows the result in a sandboxed frame with a strict content security
//! policy, so a gap here still cannot run code or load anything.

use base64::Engine;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SafeHtml {
    pub html: String,
    /// Images from the internet, blocked until the user loads them.
    pub remote_images: usize,
}

static CSS_URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)url\s*\([^)]*\)"#).unwrap());
static CSS_BAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(expression\s*\(|@import|behavior\s*:|-moz-binding)"#).unwrap());
static REMOTE_IMG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(<img\b[^>]*?)\ssrc="(https?://[^"]*)""#).unwrap());
static CID_IMG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\ssrc="cid:([^"]*)""#).unwrap());
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<a\s([^>]*?)href="([^"]*)""#).unwrap());

/// Inline CSS may stay, but must not load anything or run code.
fn clean_css(css: &str) -> String {
    CSS_BAD.replace_all(&CSS_URL.replace_all(css, "none"), "").into_owned()
}

/// `inline` maps Content-IDs (without `<>`) to `data:` URLs of embedded images.
pub fn sanitize(html: &str, inline: &HashMap<String, String>) -> SafeHtml {
    let mut b = ammonia::Builder::default();
    b.add_generic_attributes(["style", "align", "valign", "bgcolor", "width", "height", "border", "cellpadding", "cellspacing", "dir"])
        .add_tags(["font", "center"])
        .add_tag_attributes("font", ["color", "face", "size"])
        .url_schemes(["http", "https", "mailto", "cid"].into_iter().collect())
        .link_rel(Some("noopener noreferrer"))
        .attribute_filter(|_element, attribute, value| match attribute {
            "style" => Some(Cow::Owned(clean_css(value))),
            _ => Some(Cow::Borrowed(value)),
        });
    let clean = b.clean(html).to_string();

    // Remote images keep their address in data-remote-src; the window sets it on request.
    let mut remote_images = 0;
    let clean = REMOTE_IMG.replace_all(&clean, |c: &regex::Captures| {
        remote_images += 1;
        format!(r#"{} data-remote-src="{}""#, &c[1], &c[2])
    });
    // Embedded images (cid:) become data: URLs; unknown ones are dropped.
    let clean = CID_IMG.replace_all(&clean, |c: &regex::Captures| match inline.get(&c[1]) {
        Some(data) => format!(r#" src="{data}""#),
        None => String::new(),
    });
    // The link target as tooltip, so it is visible before clicking.
    let clean = LINK.replace_all(&clean, |c: &regex::Captures| {
        if c[1].contains("title=") { c[0].to_string() } else { format!(r#"<a {}title="{}" href="{}""#, &c[1], &c[2], &c[2]) }
    });
    SafeHtml { html: clean.into_owned(), remote_images }
}

/// Embedded images by Content-ID, as `data:` URLs (large ones are left out).
pub fn inline_images(msg: &mail_parser::Message) -> HashMap<String, String> {
    use mail_parser::MimeHeaders;
    const MAX_ONE: usize = 3 * 1024 * 1024;
    const MAX_ALL: usize = 10 * 1024 * 1024;
    let mut total = 0;
    let mut out = HashMap::new();
    for part in &msg.parts {
        let (Some(cid), Some(ct)) = (part.content_id(), part.content_type()) else { continue };
        if !ct.c_type.eq_ignore_ascii_case("image") {
            continue;
        }
        let bytes = part.contents();
        if bytes.len() > MAX_ONE || total + bytes.len() > MAX_ALL {
            continue;
        }
        total += bytes.len();
        let mime = format!("image/{}", ct.subtype().unwrap_or("png"));
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        out.insert(cid.trim_matches(['<', '>']).to_string(), format!("data:{mime};base64,{data}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(html: &str) -> SafeHtml {
        sanitize(html, &HashMap::new())
    }

    #[test]
    fn test_scripts_handlers_forms_frames_are_removed() {
        let s = clean(r#"<p onclick="evil()">Hi<script>alert(1)</script></p><form action="https://x"><input name="pw"></form><iframe src="https://x"></iframe><a href="javascript:evil()">x</a>"#).html;
        for bad in ["script", "onclick", "alert", "<form", "<input", "iframe", "javascript:"] {
            assert!(!s.to_lowercase().contains(bad), "{bad} survived: {s}");
        }
        assert!(s.contains("Hi"));
    }

    #[test]
    fn test_remote_images_are_blocked_and_counted() {
        let s = clean(r#"<img src="https://tracker.example/p.gif" width="1"><img src="http://x.example/a.png" alt="A">"#);
        assert_eq!(s.remote_images, 2);
        assert!(!s.html.contains(r#" src="http"#), "{}", s.html);
        assert!(s.html.contains(r#"data-remote-src="https://tracker.example/p.gif""#));
    }

    #[test]
    fn test_inline_css_cannot_load_or_run() {
        let s = clean(r#"<div style="color:red; background:url(https://x.example/t.gif); width:expression(alert(1))">x</div>"#).html;
        assert!(s.contains("color:red"));
        assert!(!s.contains("x.example") && !s.contains("expression"), "{s}");
    }

    #[test]
    fn test_cid_images_and_link_tooltips() {
        let inline = HashMap::from([("logo@m".to_string(), "data:image/png;base64,AAAA".to_string())]);
        let s = sanitize(r#"<img src="cid:logo@m"><img src="cid:missing"><a href="https://example.com/x">Link</a>"#, &inline).html;
        assert!(s.contains(r#"src="data:image/png;base64,AAAA""#), "{s}");
        assert!(!s.contains("cid:"));
        assert!(s.contains(r#"title="https://example.com/x""#), "{s}");
        assert!(s.contains("noopener"));
    }

    #[test]
    fn test_tables_and_fonts_stay() {
        let s = clean(r##"<table width="100%" bgcolor="#eee"><tr><td align="center"><font color="red">Sale</font></td></tr></table>"##).html;
        assert!(s.contains("<table") && s.contains(r##"bgcolor="#eee""##) && s.contains(r#"<font color="red">"#), "{s}");
    }
}
