//! Translations. The tables in `dist/i18n/*.json` are shared with the window;
//! English is the default and the fallback for missing keys.
//! Adding a language: copy `en.json`, translate, add it to `TABLES`.

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

/// Language code and table, in the order offered in the settings.
const TABLES: &[(&str, &str, &str)] = &[
    ("en", "English", include_str!("../../../dist/i18n/en.json")),
    ("de", "Deutsch", include_str!("../../../dist/i18n/de.json")),
];
pub const DEFAULT: &str = "en";

static PARSED: LazyLock<HashMap<&'static str, HashMap<String, String>>> = LazyLock::new(|| {
    TABLES.iter().map(|(code, _, json)| (*code, serde_json::from_str(json).expect("valid i18n table"))).collect()
});
static CURRENT: LazyLock<RwLock<String>> = LazyLock::new(|| RwLock::new(DEFAULT.to_string()));

/// Unknown codes fall back to the default.
pub fn set_language(code: &str) {
    let code = if PARSED.contains_key(code) { code } else { DEFAULT };
    *CURRENT.write().unwrap() = code.to_string();
}

pub fn language() -> String {
    CURRENT.read().unwrap().clone()
}

/// `(code, name)` of every available language.
pub fn languages() -> Vec<(&'static str, &'static str)> {
    TABLES.iter().map(|(c, n, _)| (*c, *n)).collect()
}

/// The table of the current language, completed with English for missing keys.
pub fn table() -> HashMap<String, String> {
    let mut t = PARSED[DEFAULT].clone();
    t.extend(PARSED[language().as_str()].clone());
    t
}

/// Text for `key`; `{name}` placeholders are filled from `vars`.
pub fn t(key: &str, vars: &[(&str, &str)]) -> String {
    let lang = language();
    let mut text = PARSED[lang.as_str()].get(key).or_else(|| PARSED[DEFAULT].get(key)).cloned().unwrap_or_else(|| key.to_string());
    for (name, value) in vars {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tables_have_the_same_keys() {
        let en: Vec<_> = { let mut k: Vec<_> = PARSED["en"].keys().collect(); k.sort(); k };
        for (code, _, _) in TABLES {
            let mut keys: Vec<_> = PARSED[code].keys().collect();
            keys.sort();
            assert_eq!(keys, en, "{code} differs from en");
        }
    }

    #[test]
    fn test_lookup_placeholders_and_fallback() {
        set_language("de");
        assert_eq!(t("tray.open_n", &[("n", "3")]), "Maillet öffnen (3 neu)");
        set_language("xx");
        assert_eq!(language(), "en");
        assert_eq!(t("tray.open_n", &[("n", "3")]), "Open Maillet (3 new)");
        assert_eq!(t("no.such.key", &[]), "no.such.key");
    }
}
