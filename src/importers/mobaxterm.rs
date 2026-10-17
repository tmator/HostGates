use super::{DuplicatePolicy, ImportRow, Preview, map_row};

// Raw payloads live only in the parse call; unknown fields never reach SQLite.
pub fn parse(bytes: &[u8]) -> Preview {
    let mut preview = Preview::default();
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => std::borrow::Cow::Borrowed(s.trim_start_matches('\u{feff}')),
        Err(_) => {
            preview
                .warnings
                .push("Encodage ancien : lecture Windows-1252.".into());
            encoding_rs::WINDOWS_1252.decode(bytes).0
        }
    };
    // .moba exports contain a bare session line, without INI sections.
    let mut active = !text.lines().any(|line| line.trim_start().starts_with('['));
    let mut folder = String::new();
    let mut pending: Vec<(usize, String, String)> = vec![];
    fn flush(pending: &mut Vec<(usize, String, String)>, folder: &str, preview: &mut Preview) {
        for (line, name, value) in pending.drain(..) {
            let mut row = ImportRow {
                line,
                name: name.clone(),
                folder: folder.into(),
                kind: "?".into(),
                data: None,
                issue: None,
                selected: false,
                duplicate: false,
                policy: DuplicatePolicy::Skip,
            };
            // The first number is an icon; protocol is field zero in the next group.
            // A disabled reconnect prompt may prefix the record with '; logout'.
            let value = value
                .strip_prefix("; logout")
                .unwrap_or(&value)
                .trim_start();
            let header = value
                .strip_prefix('#')
                .and_then(|s| s.split_once('#'))
                .filter(|(icon, _)| !icon.is_empty() && icon.bytes().all(|c| c.is_ascii_digit()));
            if let Some((_icon, payload)) = header {
                let connection = payload.split('#').next().unwrap_or("");
                let fields: Vec<_> = connection.split('%').collect();
                let mode = fields.first().copied().unwrap_or("");
                let kind = match mode {
                    "0" => "SSH",
                    "4" => "RDP",
                    "5" => "VNC",
                    "11" => "WEB",
                    other => other,
                };
                if fields.len()
                    < (if kind == "WEB" {
                        2
                    } else if kind == "VNC" {
                        3
                    } else {
                        4
                    })
                    && ["SSH", "RDP", "VNC", "WEB"].contains(&kind)
                {
                    row.kind = kind.into();
                    row.issue=Some("Entrée tronquée : hôte, port et utilisateur requis (utilisateur vide accepté).".into());
                } else {
                    row = map_row(
                        line,
                        &name,
                        folder,
                        kind,
                        fields.get(1).copied().unwrap_or(""),
                        fields.get(2).copied().unwrap_or(""),
                        if ["VNC", "WEB"].contains(&kind) {
                            ""
                        } else {
                            fields.get(3).copied().unwrap_or("")
                        },
                    );
                }
            } else {
                row.issue = Some(
                    "En-tête de session invalide : attendu #ICÔNE#PROTOCOLE%hôte%port%utilisateur."
                        .into(),
                );
            }
            preview.rows.push(row);
        }
    }
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            flush(&mut pending, &folder, &mut preview);
            folder.clear();
            active = line
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .is_some_and(|s| {
                    s == "Bookmarks"
                        || s.strip_prefix("Bookmarks_")
                            .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
                });
            continue;
        }
        if !active {
            continue;
        }
        if let Some((name, value)) = raw.split_once('=') {
            match name.trim() {
                "SubRep" => folder = value.trim().into(),
                "ImgNum" => {}
                name => pending.push((index + 1, name.into(), value.trim().into())),
            }
        } else {
            preview.warnings.push(format!(
                "Ligne {} ignorée : séparateur '=' absent.",
                index + 1
            ));
        }
    }
    flush(&mut pending, &folder, &mut preview);
    if preview.rows.is_empty() {
        preview
            .warnings
            .push("Aucune session trouvée dans les sections Bookmarks.".into());
    }
    preview
}
