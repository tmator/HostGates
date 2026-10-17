use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Protocol {
    SSH,
    RDP,
    VNC,
    WEB,
}
impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SSH => "SSH",
            Self::RDP => "RDP",
            Self::VNC => "VNC",
            Self::WEB => "WEB",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionInput {
    pub name: String,
    pub protocol: Protocol,
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub domain: String,
    pub folder_id: Option<String>,
    pub favorite: bool,
    pub description: String,
    pub color: String,
    pub icon: String,
    pub tags: Vec<String>,
}
impl SessionInput {
    pub fn endpoint(&self) -> String {
        if self.protocol == Protocol::WEB {
            self.hostname.clone()
        } else {
            format!("{}:{}", self.hostname, self.port)
        }
    }

    pub fn validate(&self) -> Result<()> {
        validate_name(&self.name)?;
        let host = &self.hostname;
        if self.protocol == Protocol::WEB {
            crate::connections::web::validated_url(host)?;
        } else if host.is_empty()
            || host.len() > 253
            || host.starts_with('-')
            || !host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".-:[]_".contains(c))
        {
            return Err(AppError::Validation(
                "Adresse invalide : utilisez un nom DNS ou une adresse IP sans protocole.".into(),
            ));
        }
        if self.port == 0 {
            return Err(AppError::Validation(
                "Le port doit être compris entre 1 et 65535.".into(),
            ));
        }
        for value in [&self.username, &self.domain, &self.icon] {
            if value.len() > 256 || value.chars().any(char::is_control) {
                return Err(AppError::Validation("Champ invalide ou trop long.".into()));
            }
        }
        if self.description.len() > 8192 || self.tags.len() > 32 {
            return Err(AppError::Validation(
                "Description ou liste de tags trop longue.".into(),
            ));
        }
        validate_color(&self.color)?;
        for tag in &self.tags {
            validate_name(tag)?;
        }
        if let Some(id) = &self.folder_id {
            validate_id(id)?;
        }
        Ok(())
    }
}
pub fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "Le nom doit contenir entre 1 et 128 caractères sans caractère de contrôle.".into(),
        ));
    }
    Ok(())
}
pub fn validate_color(color: &str) -> Result<()> {
    if color.len() != 7
        || !color.starts_with('#')
        || !color[1..].bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(AppError::Validation(
            "Couleur hexadécimale invalide.".into(),
        ));
    }
    Ok(())
}
pub fn validate_id(id: &str) -> Result<()> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| AppError::Validation("Identifiant invalide.".into()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub uuid: String,
    #[serde(flatten)]
    pub data: SessionInput,
    pub credential_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_connected_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderInput {
    pub name: String,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    pub color: String,
    pub icon: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub uuid: String,
    #[serde(flatten)]
    pub data: FolderInput,
}
#[derive(Serialize)]
pub struct Snapshot {
    pub programs: ExternalPrograms,
    pub sessions: Vec<Session>,
    pub folders: Vec<Folder>,
    pub theme: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExternalPrograms {
    pub ssh: String,
    pub rdp: String,
    pub vnc: String,
    pub terminal: String,
}
