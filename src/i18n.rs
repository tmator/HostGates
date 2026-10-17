use std::{
    collections::BTreeMap,
    sync::{OnceLock, RwLock},
};
#[derive(Default)]
struct Catalog {
    language: String,
    strings: BTreeMap<String, String>,
}
static CATALOG: OnceLock<RwLock<Catalog>> = OnceLock::new();
pub const BUILTIN_LANGUAGES: &[(&str, &str, &str)] = &[
    ("fr", "Français", include_str!("../locales/fr.json")),
    ("en", "English", include_str!("../locales/en.json")),
    ("es", "Español", include_str!("../locales/es.json")),
    ("it", "Italiano", include_str!("../locales/it.json")),
    ("de", "Deutsch", include_str!("../locales/de.json")),
    (
        "pt",
        "Português (Brasil)",
        include_str!("../locales/pt.json"),
    ),
    ("ja", "日本語", include_str!("../locales/ja.json")),
    ("zh-CN", "简体中文", include_str!("../locales/zh-CN.json")),
    ("ko", "한국어", include_str!("../locales/ko.json")),
    ("ar", "العربية", include_str!("../locales/ar.json")),
    ("hi", "हिन्दी", include_str!("../locales/hi.json")),
    ("id", "Bahasa Indonesia", include_str!("../locales/id.json")),
];

pub fn language_name(code: &str) -> &str {
    let code = if code.is_empty() { "fr" } else { code };
    BUILTIN_LANGUAGES
        .iter()
        .find(|entry| entry.0 == code)
        .map_or(code, |entry| entry.1)
}

pub fn set_language(language: &str) -> bool {
    let language = if language.is_empty() { "fr" } else { language };
    let catalog = CATALOG.get_or_init(Default::default);
    if let Ok(mut c) = catalog.write() {
        if c.language == language {
            return false;
        }
        let embedded = BUILTIN_LANGUAGES
            .iter()
            .find(|entry| entry.0 == language)
            .map_or("{}", |entry| entry.2);
        let mut strings: BTreeMap<String, String> =
            serde_json::from_str(embedded).unwrap_or_default();
        if language
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            for folder in [
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.join("locales"))),
                Some(crate::ui::locale_dir()),
            ]
            .into_iter()
            .flatten()
            {
                if let Ok(bytes) = std::fs::read(folder.join(format!("{language}.json")))
                    && let Ok(extra) = serde_json::from_slice::<BTreeMap<String, String>>(&bytes)
                {
                    strings.extend(extra);
                }
            }
        }
        *c = Catalog {
            language: language.into(),
            strings,
        };
        return true;
    }
    false
}
pub fn tr(text: &str) -> String {
    let Some(catalog) = CATALOG.get().and_then(|c| c.read().ok()) else {
        return text.into();
    };
    if let Some(value) = catalog.strings.get(text) {
        return value.clone();
    }
    for (source, target) in &catalog.strings {
        if let Some(result) = translate_template(source, target, text) {
            return result;
        }
    }
    text.into()
}
// Match translated display templates after Rust formatting. Only fixed text is
// translated; captured names, IPs and paths are copied verbatim.
fn translate_template(source: &str, target: &str, text: &str) -> Option<String> {
    if !source.contains('{') {
        return None;
    }
    let mut literals = Vec::new();
    let mut names = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find('{') {
        literals.push(&rest[..start]);
        let end = rest[start..].find('}')? + start;
        names.push(&rest[start + 1..end]);
        rest = &rest[end + 1..];
    }
    literals.push(rest);
    if literals.iter().map(|s| s.len()).sum::<usize>() < 8 {
        return None;
    }
    let mut input = text.strip_prefix(literals[0])?;
    let mut values = Vec::new();
    for suffix in &literals[1..] {
        if suffix.is_empty() {
            values.push(input);
            input = "";
        } else {
            let end = input.find(suffix)?;
            values.push(&input[..end]);
            input = &input[end + suffix.len()..];
        }
    }
    if !input.is_empty() {
        return None;
    }
    let mut output = String::new();
    let mut rest = target;
    let mut ordinal = 0;
    while let Some(start) = rest.find('{') {
        output.push_str(&rest[..start]);
        let end = rest[start..].find('}')? + start;
        let name = &rest[start + 1..end];
        let index = if name.is_empty() {
            let n = names
                .iter()
                .enumerate()
                .filter(|(_, n)| n.is_empty())
                .nth(ordinal)?
                .0;
            ordinal += 1;
            n
        } else {
            names.iter().position(|n| *n == name)?
        };
        output.push_str(values.get(index)?);
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    Some(output)
}
pub fn languages() -> Vec<String> {
    let mut languages: Vec<String> = BUILTIN_LANGUAGES
        .iter()
        .map(|entry| entry.0.into())
        .collect();
    for folder in [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("locales"))),
        Some(crate::ui::locale_dir()),
    ]
    .into_iter()
    .flatten()
    {
        if let Ok(entries) = std::fs::read_dir(folder) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().is_some_and(|s| s == "json")
                    && let Some(name) = p.file_stem().and_then(|s| s.to_str())
                {
                    languages.push(name.into());
                }
            }
        }
    }
    languages.sort();
    languages.dedup();
    languages
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_catalogs_preserve_session_names_and_template_values() {
        let source = "{count} résultat(s) — tous les dossiers";
        for (code, _, json) in super::BUILTIN_LANGUAGES {
            let catalog: super::BTreeMap<String, String> =
                serde_json::from_str(json).expect("valid catalog");
            let target = catalog.get(source).expect("translated result count");
            let output =
                super::translate_template(source, target, "12 résultat(s) — tous les dossiers")
                    .expect("template matches");
            assert!(output.contains("12"), "{code}: {output}");
            assert!(!output.contains('{'), "{code}");
            assert!(catalog.contains_key("Déverrouiller le coffre"), "{code}");
        }
        assert_eq!(super::language_name(""), "Français");
        assert_eq!(super::language_name("ja"), "日本語");
        assert_eq!(super::language_name("custom"), "custom");
    }

    #[test]
    fn templates_preserve_values_and_require_full_match() {
        assert_eq!(
            super::translate_template(
                "{count} résultat(s) — tous les dossiers",
                "{count} result(s) — all folders",
                "12 résultat(s) — tous les dossiers"
            ),
            Some("12 result(s) — all folders".into())
        );
        assert!(
            super::translate_template(
                "{count} résultat(s) — tous les dossiers",
                "{count} results",
                "12 résultat(s) — tous les dossiers OTHER"
            )
            .is_none()
        );
    }
}
