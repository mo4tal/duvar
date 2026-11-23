//! Multi-language UI catalog loaded from `/locales/*.json` (default: Turkish `tr`).
use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageInfo {
    pub code: &'static str,
    pub native_name: &'static str,
    pub english_name: &'static str,
    pub rtl: bool,
}

pub const LANGUAGES: &[LanguageInfo] = &[
    LanguageInfo {
        code: "tr",
        native_name: "Türkçe",
        english_name: "Turkish",
        rtl: false,
    },
    LanguageInfo {
        code: "en",
        native_name: "English",
        english_name: "English",
        rtl: false,
    },
    LanguageInfo {
        code: "es",
        native_name: "Español",
        english_name: "Spanish",
        rtl: false,
    },
    LanguageInfo {
        code: "de",
        native_name: "Deutsch",
        english_name: "German",
        rtl: false,
    },
    LanguageInfo {
        code: "fr",
        native_name: "Français",
        english_name: "French",
        rtl: false,
    },
    LanguageInfo {
        code: "pt",
        native_name: "Português (Brasil)",
        english_name: "Portuguese (Brazil)",
        rtl: false,
    },
    LanguageInfo {
        code: "ru",
        native_name: "Русский",
        english_name: "Russian",
        rtl: false,
    },
    LanguageInfo {
        code: "ja",
        native_name: "日本語",
        english_name: "Japanese",
        rtl: false,
    },
    LanguageInfo {
        code: "zh",
        native_name: "简体中文",
        english_name: "Chinese (Simplified)",
        rtl: false,
    },
    LanguageInfo {
        code: "ar",
        native_name: "العربية",
        english_name: "Arabic",
        rtl: true,
    },
];

const RAW_BUNDLES: &[(&str, &str)] = &[
    ("tr", include_str!("../locales/tr.json")),
    ("en", include_str!("../locales/en.json")),
    ("es", include_str!("../locales/es.json")),
    ("de", include_str!("../locales/de.json")),
    ("fr", include_str!("../locales/fr.json")),
    ("pt", include_str!("../locales/pt.json")),
    ("ru", include_str!("../locales/ru.json")),
    ("ja", include_str!("../locales/ja.json")),
    ("zh", include_str!("../locales/zh.json")),
    ("ar", include_str!("../locales/ar.json")),
];

type FlatMap = HashMap<String, String>;

fn parse_flat(raw: &str) -> FlatMap {
    let mut out = FlatMap::new();
    let Ok(serde_json::Value::Object(root)) = serde_json::from_str::<serde_json::Value>(raw) else {
        return out;
    };
    for (section, val) in root {
        if section.starts_with('_') {
            continue;
        }
        if let serde_json::Value::Object(entries) = val {
            for (k, v) in entries {
                if let Some(text) = v.as_str() {
                    out.insert(format!("{section}.{k}"), text.to_owned());
                }
            }
        }
    }
    out
}

fn catalogs() -> &'static HashMap<&'static str, FlatMap> {
    static CATALOGS: OnceLock<HashMap<&'static str, FlatMap>> = OnceLock::new();
    CATALOGS.get_or_init(|| {
        RAW_BUNDLES
            .iter()
            .map(|(code, raw)| (*code, parse_flat(raw)))
            .collect()
    })
}

fn active_cell() -> &'static RwLock<&'static str> {
    static ACTIVE: OnceLock<RwLock<&'static str>> = OnceLock::new();
    ACTIVE.get_or_init(|| RwLock::new("tr"))
}

pub fn normalize_code(code: &str) -> &'static str {
    let trimmed = code.trim().to_ascii_lowercase();
    let base = trimmed.split(['_', '-']).next().unwrap_or("tr");
    LANGUAGES
        .iter()
        .find(|l| l.code == base)
        .map(|l| l.code)
        .unwrap_or("tr")
}

pub fn init_from_settings(settings: Option<&gio::Settings>) -> &'static str {
    use gio::prelude::SettingsExt;
    if let Ok(env_lang) = std::env::var("DUVAR_LANG")
        && !env_lang.trim().is_empty()
    {
        return set_language(&env_lang);
    }
    if let Some(s) = settings
        && s.settings_schema()
            .is_some_and(|schema| schema.has_key("ui-language"))
    {
        let value = s.string("ui-language");
        return set_language(value.as_str());
    }
    set_language("tr")
}

pub fn set_language(code: &str) -> &'static str {
    let normalized = normalize_code(code);
    if let Ok(mut guard) = active_cell().write() {
        *guard = normalized;
    }
    normalized
}

pub fn current_language() -> &'static str {
    active_cell().read().map(|g| *g).unwrap_or("tr")
}

pub fn language_index(code: &str) -> u32 {
    let normalized = normalize_code(code);
    LANGUAGES
        .iter()
        .position(|l| l.code == normalized)
        .unwrap_or(0) as u32
}

pub fn language_display_label(info: &LanguageInfo) -> String {
    if info.code == "tr" {
        format!("{} ({})", info.native_name, tr("settings.lang_default"))
    } else {
        format!("{} — {}", info.native_name, info.english_name)
    }
}

pub fn tr(key: &str) -> String {
    let lang = current_language();
    let map = catalogs();
    if let Some(text) = map.get(lang).and_then(|m| m.get(key)) {
        return text.clone();
    }
    if let Some(text) = map.get("tr").and_then(|m| m.get(key)) {
        return text.clone();
    }
    key.to_owned()
}

pub fn tr_fmt(key: &str, vars: &[(&str, &str)]) -> String {
    let mut text = tr(key);
    for (name, value) in vars {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn placeholders(text: &str) -> BTreeSet<String> {
        let mut set = BTreeSet::new();
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{'
                && let Some(end) = text[i + 1..].find('}')
            {
                let token = &text[i + 1..i + 1 + end];
                if !token.is_empty()
                    && token
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    set.insert(token.to_owned());
                }
                i += end + 2;
                continue;
            }
            i += 1;
        }
        set
    }

    #[test]
    fn all_ten_locales_have_complete_keys_and_placeholders() {
        let map = catalogs();
        assert_eq!(map.len(), 10, "Expected exactly 10 bundled locales");
        let base = map.get("tr").expect("Default tr locale must exist");
        assert!(
            base.len() >= 140,
            "Locale catalog unexpectedly small: {}",
            base.len()
        );

        for info in LANGUAGES {
            let bundle = map
                .get(info.code)
                .unwrap_or_else(|| panic!("Missing locale {}", info.code));
            for (k, tr_val) in base {
                let val = bundle
                    .get(k)
                    .unwrap_or_else(|| panic!("Locale {} missing key {k}", info.code));
                assert!(
                    !val.trim().is_empty(),
                    "Locale {} has empty translation for {k}",
                    info.code
                );
                assert!(
                    !val.contains('&'),
                    "Locale {} key {k} contains raw '&' which breaks Libadwaita Pango markup: {val:?}",
                    info.code
                );
                assert_eq!(
                    placeholders(tr_val),
                    placeholders(val),
                    "Placeholder mismatch in locale {} for key {k}: tr={tr_val:?} vs {val:?}",
                    info.code
                );
            }
            for k in bundle.keys() {
                assert!(
                    base.contains_key(k),
                    "Locale {} has extra undocumented key {k}",
                    info.code
                );
            }
        }
    }

    #[test]
    fn default_language_is_turkish_and_switching_works() {
        assert_eq!(normalize_code("unknown"), "tr");
        assert_eq!(normalize_code("en_US"), "en");
        set_language("en");
        assert_eq!(tr("tabs.explore"), "Explore");
        assert_eq!(tr("tabs.colors"), "Colors");
        set_language("tr");
        assert_eq!(tr("tabs.explore"), "Keşfet");
        assert_eq!(tr("tabs.colors"), "Renkler");
    }
}
