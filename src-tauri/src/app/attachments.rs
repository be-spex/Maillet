//! Attachments of the mail in the reader: view (temporary copy, opened with the
//! program registered for its type) or save (file dialog, Downloads by default).
//!
//! Viewing never touches the Downloads folder. The copy goes to a private
//! directory below `$XDG_RUNTIME_DIR` (a RAM disk, emptied at logout), falling
//! back to the system temp directory. Attachments above [`VIEW_LIMIT`] are
//! therefore only offered for saving. It is read-only, so nobody edits it by
//! mistake, and removed again: after a day, when Maillet starts and when it quits.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use tauri::{async_runtime, AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::mail::model::MailContent;
use crate::mail::parse;

/// Raw mails kept for their attachments: the open one plus a few before it …
const CACHE_SIZE: usize = 3;
/// … as long as they need no more memory than this together. The open one always stays.
const CACHE_BYTES: usize = 100 * 1024 * 1024;
/// Larger attachments are not copied for viewing (the copy lives in RAM), only saved.
pub const VIEW_LIMIT: usize = 25 * 1024 * 1024;
/// Viewed copies older than this are removed when the next one is opened.
const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// The raw mails last opened in the reader, newest last.
#[derive(Default)]
pub struct AttachmentCache(Mutex<VecDeque<(String, Vec<u8>)>>);

impl AttachmentCache {
    /// Takes the raw mail out of `content` and gives it a key the reader passes back.
    pub fn keep(&self, mut content: MailContent) -> MailContent {
        let raw = std::mem::take(&mut content.raw);
        if content.attachments.is_empty() {
            return content;
        }
        for a in &mut content.attachments {
            a.viewable = a.size <= VIEW_LIMIT;
        }
        content.key = uuid::Uuid::new_v4().to_string();
        let mut cache = self.0.lock().unwrap();
        cache.push_back((content.key.clone(), raw));
        while cache.len() > CACHE_SIZE || (cache.len() > 1 && cache.iter().map(|(_, r)| r.len()).sum::<usize>() > CACHE_BYTES) {
            cache.pop_front();
        }
        content
    }

    fn get(&self, key: &str, index: usize) -> Result<(String, Vec<u8>), String> {
        let cache = self.0.lock().unwrap();
        let (_, raw) = cache.iter().find(|(k, _)| k == key).ok_or_else(|| crate::base::i18n::t("att.expired", &[]))?;
        parse::attachment(raw, index).ok_or_else(|| crate::base::i18n::t("att.not_found", &[]))
    }
}

/// Opens an attachment with the program registered for its type, from a temporary copy.
#[tauri::command]
pub async fn view_attachment(cache: State<'_, AttachmentCache>, key: String, index: usize) -> Result<(), String> {
    let (name, data) = cache.get(&key, index)?;
    if data.len() > VIEW_LIMIT {
        return Err(crate::base::i18n::t("att.too_big", &[]));
    }
    let path = async_runtime::spawn_blocking(move || write_temp(&name, &data)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    crate::base::open::xdg_open(&path).map_err(|e| format!("xdg-open: {e}"))
}

/// Asks for a target file (Downloads by default) and saves the attachment there.
/// Returns the path, `None` if cancelled.
#[tauri::command]
pub async fn save_attachment(app: AppHandle, cache: State<'_, AttachmentCache>, key: String, index: usize) -> Result<Option<String>, String> {
    let (name, data) = cache.get(&key, index)?;
    let picked = async_runtime::spawn_blocking(move || {
        let mut dialog = app.dialog().file().set_file_name(safe_name(&name));
        if let Some(dir) = dirs::download_dir() {
            dialog = dialog.set_directory(dir);
        }
        dialog.blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = picked else { return Ok(None) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

/// Private directory for viewed attachments.
fn temp_root() -> PathBuf {
    match dirs::runtime_dir() {
        Some(dir) => dir.join("maillet-attachments"),
        // The temp directory is shared by all users.
        None => std::env::temp_dir().join(format!("maillet-attachments-{}", std::env::var("USER").unwrap_or_default())),
    }
}

/// Writes a read-only copy into its own subdirectory, so the file keeps its name.
fn write_temp(name: &str, data: &[u8]) -> std::io::Result<PathBuf> {
    let root = temp_root();
    create_private_dir(&root)?;
    remove_older_than(&root, MAX_AGE);
    let dir = root.join(uuid::Uuid::new_v4().to_string());
    create_private_dir(&dir)?;
    let path = dir.join(safe_name(name));
    std::fs::write(&path, data)?;
    set_mode(&path, 0o400)?;
    Ok(path)
}

/// Removes all viewed copies (at start and exit).
pub fn cleanup() {
    remove_older_than(&temp_root(), Duration::ZERO);
}

fn remove_older_than(root: &Path, age: Duration) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let old = entry.metadata().and_then(|m| m.modified()).ok().and_then(|t| SystemTime::now().duration_since(t).ok()).is_none_or(|d| d >= age);
        if old {
            let path = entry.path();
            // Read-only files in a writable directory can be removed.
            let _ = std::fs::remove_dir_all(&path).or_else(|_| std::fs::remove_file(&path));
        }
    }
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    set_mode(dir, 0o700)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_mode(_: &Path, _: u32) -> std::io::Result<()> {
    Ok(())
}

/// The attachment name as a plain file name: no directories, no control characters.
fn safe_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let clean: String = base.chars().map(|c| if c.is_control() { '_' } else { c }).collect();
    let clean = clean.trim().trim_start_matches('.').to_string();
    if clean.is_empty() { "Attachment".into() } else { clean.chars().take(200).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content(attachment_size: usize, raw_len: usize) -> MailContent {
        let a = crate::mail::model::Attachment { index: 0, name: "a.bin".into(), mime: "application/octet-stream".into(), size: attachment_size, viewable: true };
        MailContent { text: String::new(), html: None, remote_images: 0, attachments: vec![a], key: String::new(), raw: vec![0; raw_len] }
    }

    #[test]
    fn test_cache_limits_count_and_memory_but_keeps_the_open_mail() {
        let cache = AttachmentCache::default();
        let big = cache.keep(content(VIEW_LIMIT + 1, 1));
        assert!(!big.attachments[0].viewable && big.raw.is_empty());
        let keys: Vec<_> = (0..4).map(|_| cache.keep(content(1, 1)).key).collect();
        let held = |k: &str| cache.0.lock().unwrap().iter().any(|(key, _)| key == k);
        assert!(!held(&big.key) && !held(&keys[0]) && held(&keys[3]), "only the newest {CACHE_SIZE}");
        let huge = cache.keep(content(1, CACHE_BYTES + 1));
        assert!(held(&huge.key) && !held(&keys[3]), "a huge mail pushes the others out but stays itself");
    }

    #[test]
    fn test_safe_name() {
        assert_eq!(safe_name("Rechnung.pdf"), "Rechnung.pdf");
        assert_eq!(safe_name("../../.bashrc"), "bashrc");
        assert_eq!(safe_name("C:\\temp\\a.xml"), "a.xml");
        assert_eq!(safe_name("a\nb.txt"), "a_b.txt");
        assert_eq!(safe_name(".."), "Attachment");
        assert_eq!(safe_name(""), "Attachment");
    }

    #[test]
    fn test_write_temp_is_read_only_and_cleaned() {
        let path = write_temp("t.txt", b"hello").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        assert!(std::fs::metadata(&path).unwrap().permissions().readonly());
        assert!(path.starts_with(temp_root()));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn test_remove_older_than() {
        let root = std::env::temp_dir().join(format!("maillet-test-{}", uuid::Uuid::new_v4()));
        let dir = root.join("x");
        create_private_dir(&dir).unwrap();
        std::fs::write(dir.join("a.pdf"), b"%PDF").unwrap();
        set_mode(&dir.join("a.pdf"), 0o400).unwrap();
        remove_older_than(&root, MAX_AGE);
        assert!(dir.exists(), "fresh copies stay");
        remove_older_than(&root, Duration::ZERO);
        assert!(!dir.exists());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
