use serde::Serialize;
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "code", content = "message")]
pub enum AppError {
    #[error("{0}")]
    Operation(String),
    #[error("{0}")]
    Validation(String),
    #[error("Élément introuvable.")]
    NotFound,
    #[error("Opération impossible sur la base locale.")]
    Database,
    #[error(
        "Coffre système indisponible ou verrouillé. Vous pouvez enregistrer la session sans mot de passe."
    )]
    VaultUnavailable,
    #[error("Une opération est déjà en cours. Réessayez.")]
    Busy,
}
impl From<rusqlite::Error> for AppError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Database
    }
}
pub type Result<T> = std::result::Result<T, AppError>;
