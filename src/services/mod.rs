pub mod worker;
use crate::{
    credentials::{CredentialStore, Secret},
    database::Database,
    error::{AppError, Result},
    models::SessionInput,
};

pub struct SessionService {
    pub db: Database,
    pub vault: Box<dyn CredentialStore>,
}
impl SessionService {
    pub fn create_gateway(&mut self, data: &SessionInput, password: Secret) -> Result<String> {
        data.validate()?;
        if data.protocol != crate::models::Protocol::SSH || data.username.trim().is_empty() {
            return Err(AppError::Validation(
                "Passerelle SSH : utilisateur requis.".into(),
            ));
        }
        let id = self.save(None, data)?;
        if !password.is_empty()
            && let Err(e) = self.set_secret(&id, password)
        {
            self.delete(&id)?;
            return Err(e);
        }
        Ok(id)
    }

    pub fn save(&mut self, id: Option<&str>, data: &SessionInput) -> Result<String> {
        self.db.save_session(id, data)
    }
    // One credential per session. Duplication never copies credential references.
    pub fn set_secret(&mut self, session_id: &str, secret: Secret) -> Result<()> {
        if secret.is_empty() || secret.len() > 2048 {
            return Err(AppError::Validation(
                "Mot de passe vide ou trop long (2048 octets maximum).".into(),
            ));
        }
        let existing = self.db.credential_reference(session_id)?;
        let id = existing
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        self.vault.store_secret(&id, &secret)?;
        if let Err(error) = self.db.link_credential(session_id, Some(&id)) {
            if existing.is_none() {
                self.vault.delete_secret(&id)?;
            }
            return Err(error);
        }
        Ok(())
    }
    pub fn remove_secret(&mut self, session_id: &str) -> Result<()> {
        if let Some(id) = self.db.credential_reference(session_id)? {
            self.vault.delete_secret(&id)?;
            self.db.link_credential(session_id, None)?;
        }
        Ok(())
    }
    pub fn delete(&mut self, session_id: &str) -> Result<()> {
        self.remove_secret(session_id)?;
        self.db.delete_session(session_id)
    }
}
