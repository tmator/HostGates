pub mod exchange;
pub mod mobaxterm;
use crate::{
    error::{AppError, Result},
    models::SessionInput,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum DuplicatePolicy {
    #[default]
    Skip,
    Import,
    Replace,
}
#[derive(Clone)]
pub struct ImportRow {
    pub line: usize,
    pub name: String,
    pub folder: String,
    pub kind: String,
    pub data: Option<SessionInput>,
    pub issue: Option<String>,
    pub selected: bool,
    pub duplicate: bool,
    pub policy: DuplicatePolicy,
}
#[derive(Clone, Default)]
pub struct Preview {
    pub rows: Vec<ImportRow>,
    pub warnings: Vec<String>,
}
pub fn folder_parts(path: &str) -> Result<Vec<&str>> {
    let parts: Vec<_> = path.split(['\\', '/']).filter(|p| !p.is_empty()).collect();
    if parts.len() > 64 {
        return Err(AppError::Validation("Dossier trop profond.".into()));
    }
    for p in &parts {
        crate::models::validate_name(p)?;
        if [".", ".."].contains(p) {
            return Err(AppError::Validation("Chemin de dossier ambigu.".into()));
        }
    }
    Ok(parts)
}
pub fn map_row(
    line: usize,
    name: &str,
    folder: &str,
    kind: &str,
    host: &str,
    port: &str,
    user: &str,
) -> ImportRow {
    let mut row = ImportRow {
        line,
        name: name.into(),
        folder: folder.into(),
        kind: kind.into(),
        data: None,
        issue: None,
        selected: false,
        duplicate: false,
        policy: DuplicatePolicy::Skip,
    };
    let protocol = match kind {
        "SSH" => crate::models::Protocol::SSH,
        "RDP" => crate::models::Protocol::RDP,
        "VNC" => crate::models::Protocol::VNC,
        "WEB" | "BROWSER" => crate::models::Protocol::WEB,
        _ => {
            row.issue = Some(format!("{kind} — actuellement non pris en charge"));
            return row;
        }
    };
    let web_url = if protocol == crate::models::Protocol::WEB {
        crate::connections::web::validated_url(host).ok()
    } else {
        None
    };
    let parsed = if let Some(url) = &web_url {
        Ok(url.port_or_known_default().unwrap_or(443))
    } else {
        port.parse::<u16>()
    };
    let data = SessionInput {
        name: name.into(),
        protocol,
        hostname: host.into(),
        port: parsed.unwrap_or(0),
        username: user.into(),
        domain: String::new(),
        folder_id: None,
        favorite: false,
        description: String::new(),
        color: "#7195ff".into(),
        icon: "server".into(),
        tags: vec![],
    };
    match data
        .validate()
        .and_then(|_| folder_parts(folder).map(|_| ()))
    {
        Ok(()) => {
            row.selected = true;
            row.data = Some(data)
        }
        Err(e) => row.issue = Some(e.to_string()),
    }
    row
}
