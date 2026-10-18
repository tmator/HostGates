use super::{Preview, map_row};
use crate::{
    error::{AppError, Result},
    models::{Session, Snapshot},
};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Read, Write},
    path::Path,
};
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;
#[derive(Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Mxt,
    Csv,
}
pub fn read_preview(path: &Path, snapshot: &Snapshot) -> Result<Preview> {
    let file = std::fs::File::open(path)
        .map_err(|_| AppError::Operation("Impossible de lire le fichier sélectionné.".into()))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::Operation("Lecture du fichier impossible.".into()))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(AppError::Validation(
            "Fichier trop volumineux (8 Mio maximum).".into(),
        ));
    }
    let mut preview = if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
    {
        parse_csv(&bytes)?
    } else {
        super::mobaxterm::parse(&bytes)
    };
    if preview.rows.len() > 10000 {
        return Err(AppError::Validation(
            "Maximum 10 000 sessions par import.".into(),
        ));
    }
    let mut seen: HashSet<_> = snapshot
        .sessions
        .iter()
        .map(|s| identity(&s.data))
        .collect();
    for row in &mut preview.rows {
        if let Some(data) = &row.data {
            row.duplicate = !seen.insert(identity(data));
        }
    }
    Ok(preview)
}
pub fn identity(s: &crate::models::SessionInput) -> (String, String, u16, String) {
    (
        s.protocol.as_str().into(),
        if s.protocol == crate::models::Protocol::WEB {
            crate::connections::web::validated_url(&s.hostname)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| s.hostname.clone())
        } else {
            s.hostname.to_ascii_lowercase()
        },
        s.port,
        s.username.clone(),
    )
}
pub fn parse_csv(bytes: &[u8]) -> Result<Preview> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AppError::Validation("Le CSV doit être encodé en UTF-8.".into()))?
        .trim_start_matches('\u{feff}');
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());
    let headers = rdr
        .headers()
        .map_err(|_| AppError::Validation("En-tête CSV invalide.".into()))?
        .clone();
    let col = |names: &[&str]| {
        headers
            .iter()
            .position(|h| names.iter().any(|n| h.trim().eq_ignore_ascii_case(n)))
    };
    let kind = col(&["Session type"]);
    let host = col(&["Hostname", "Remote Host"]);
    let port = col(&["Port number", "Port"]);
    let user = col(&["Username"]);
    let name = col(&["Session name"]);
    let folder = col(&["Folder name"]);
    if kind.is_none() || host.is_none() {
        return Err(AppError::Validation(
            "Colonnes requises : Session type et Hostname (ou Remote Host).".into(),
        ));
    }
    let mut preview = Preview::default();
    for (i, result) in rdr.records().enumerate() {
        let record = result.map_err(|_| {
            AppError::Validation(format!("CSV invalide à l'enregistrement {}.", i + 2))
        })?;
        let get = |index: Option<usize>| index.and_then(|j| record.get(j)).unwrap_or("");
        let host = get(host);
        let kind = get(kind).to_ascii_uppercase();
        let port = get(port);
        let port = if port.is_empty() {
            match kind.as_str() {
                "RDP" => "3389",
                "VNC" => "5900",
                "WEB" | "BROWSER" => "443",
                _ => "22",
            }
        } else {
            port
        };
        preview.rows.push(map_row(
            i + 2,
            if get(name).is_empty() {
                host
            } else {
                get(name)
            },
            get(folder),
            &kind,
            host,
            port,
            get(user),
        ));
    }
    Ok(preview)
}
pub fn folder_path(id: Option<&str>, snapshot: &Snapshot) -> String {
    let mut names = vec![];
    let mut cursor = id;
    for _ in 0..64 {
        let Some(f) = snapshot
            .folders
            .iter()
            .find(|f| Some(f.uuid.as_str()) == cursor)
        else {
            break;
        };
        names.push(f.data.name.as_str());
        cursor = f.data.parent_id.as_deref();
    }
    names.reverse();
    names.join("\\")
}
pub fn export(snapshot: &Snapshot, ids: &[String], format: ExportFormat) -> Result<Vec<u8>> {
    let sessions: Vec<&Session> = snapshot
        .sessions
        .iter()
        .filter(|s| ids.contains(&s.uuid))
        .collect();
    if sessions.is_empty() {
        return Err(AppError::Validation(
            "Sélectionnez au moins une session à exporter.".into(),
        ));
    }
    match format {
        ExportFormat::Csv => {
            let mut writer = csv::WriterBuilder::new()
                .terminator(csv::Terminator::CRLF)
                .from_writer(Vec::new());
            writer
                .write_record([
                    "Folder name",
                    "Session name",
                    "Session type",
                    "Hostname",
                    "Username",
                    "Port number",
                ])
                .map_err(|_| AppError::Database)?;
            for s in sessions {
                writer
                    .write_record(
                        [
                            folder_path(s.data.folder_id.as_deref(), snapshot),
                            s.data.name.clone(),
                            s.data.protocol.as_str().into(),
                            s.data.hostname.clone(),
                            s.data.username.clone(),
                            s.data.port.to_string(),
                        ]
                        .map(|value| spreadsheet_cell(&value)),
                    )
                    .map_err(|_| AppError::Database)?;
            }
            writer
                .into_inner()
                .map_err(|_| AppError::Operation("Export CSV impossible.".into()))
        }
        ExportFormat::Mxt => {
            let mut groups: BTreeMap<String, Vec<&Session>> = BTreeMap::new();
            for s in sessions {
                groups
                    .entry(folder_path(s.data.folder_id.as_deref(), snapshot))
                    .or_default()
                    .push(s);
            }
            let mut out = String::from("\u{feff}");
            for (i, (folder, sessions)) in groups.into_iter().enumerate() {
                if folder.contains(['\r', '\n']) {
                    return Err(AppError::Validation(
                        "Nom de dossier incompatible avec le format INI.".into(),
                    ));
                }
                out.push_str(&format!(
                    "[Bookmarks{}]\r\nSubRep={}\r\nImgNum=42\r\n",
                    if i == 0 {
                        String::new()
                    } else {
                        format!("_{i}")
                    },
                    folder
                ));
                let mut names = HashSet::new();
                for s in sessions {
                    if s.data.name.contains(['=', '\r', '\n'])
                        || ["subrep", "imgnum"].contains(&s.data.name.to_lowercase().as_str())
                        || s.data.name.trim() != s.data.name
                        || s.data.name.starts_with([';', '#', '['])
                        || [&s.data.hostname, &s.data.username]
                            .iter()
                            .any(|v| v.contains(['%', '#', '\r', '\n']))
                        || !names.insert(s.data.name.clone())
                    {
                        return Err(AppError::Validation("Nom ou utilisateur non représentable sans perte en .mxtsessions (ou noms identiques dans un dossier). Utilisez l'export CSV.".into()));
                    }
                    let (kind, mode) = match s.data.protocol {
                        crate::models::Protocol::SSH => ("109", "0"),
                        crate::models::Protocol::RDP => ("91", "4"),
                        crate::models::Protocol::VNC => ("128", "5"),
                        crate::models::Protocol::WEB => ("313", "11"),
                    };
                    if s.data.protocol == crate::models::Protocol::WEB {
                        out.push_str(&format!(
                            "{}=#{}#{}%{}%-1%-1%\r\n",
                            s.data.name, kind, mode, s.data.hostname
                        ));
                        continue;
                    }
                    let user = if s.data.protocol == crate::models::Protocol::VNC {
                        "-1"
                    } else {
                        &s.data.username
                    };
                    out.push_str(&format!(
                        "{}=#{}#{}%{}%{}%{}%\r\n",
                        s.data.name, kind, mode, s.data.hostname, s.data.port, user
                    ));
                }
                out.push_str("\r\n");
            }
            Ok(out.into_bytes())
        }
    }
}
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| AppError::Operation("Impossible de créer le fichier d'export.".into()))?;
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| AppError::Operation("Écriture de l'export impossible.".into()))?;
    file.persist(path).map_err(|_| {
        AppError::Operation(
            "Impossible de remplacer le fichier d'export. Vérifiez ses permissions.".into(),
        )
    })?;
    Ok(())
}

// CSV quoting does not prevent spreadsheet formula evaluation.
fn spreadsheet_cell(value: &str) -> String {
    if value.trim_start().starts_with(['=', '+', '-', '@']) || value.starts_with(['\t', '\r', '\n'])
    {
        format!("'{value}")
    } else {
        value.to_owned()
    }
}
#[cfg(test)]
mod security_tests {
    #[test]
    fn formulas_are_text() {
        for value in ["=1+1", "+CMD", "-1", "@SUM(A1)", "  =1", "\tfoo"] {
            assert!(super::spreadsheet_cell(value).starts_with('\''));
        }
        assert_eq!(super::spreadsheet_cell("server"), "server");
    }
}
