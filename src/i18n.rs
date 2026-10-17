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
pub fn set_language(language: &str) {
    let catalog = CATALOG.get_or_init(Default::default);
    if let Ok(mut c) = catalog.write() {
        if c.language == language {
            return;
        }
        let embedded = if language == "en" {
            include_str!("../locales/en.json")
        } else {
            "{}"
        };
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
    }
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
    let mut languages = vec!["fr".into(), "en".into()];
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
