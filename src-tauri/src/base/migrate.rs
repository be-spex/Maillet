//! One-time move of the app's directories after a change of its identifier
//! (the bundle identifier names the config, data and cache directories, e.g.
//! the webview's local storage). Runs before Tauri touches any of them.

use std::path::PathBuf;
use std::{env, fs};

/// `$XDG_CONFIG_HOME`, `$XDG_DATA_HOME` and `$XDG_CACHE_HOME` with their defaults.
fn base_dirs() -> Vec<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let xdg = |var: &str, default: &str| {
        env::var_os(var)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home.as_ref().map(|h| h.join(default)))
    };
    [
        xdg("XDG_CONFIG_HOME", ".config"),
        xdg("XDG_DATA_HOME", ".local/share"),
        xdg("XDG_CACHE_HOME", ".cache"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// In each base directory: if `<new>` does not exist yet, renames the first
/// existing `<old>` (in the order given) to it. Anything else stays untouched.
pub fn from_old_identifiers(new: &str, old: &[&str]) {
    for base in base_dirs() {
        let target = base.join(new);
        if target.exists() {
            continue;
        }
        if let Some(source) = old.iter().map(|o| base.join(o)).find(|p| p.is_dir()) {
            if let Err(e) = fs::rename(&source, &target) {
                eprintln!("{}: could not move {} to {}: {e}", new, source.display(), target.display());
            }
        }
    }
}
