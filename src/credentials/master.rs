//! Versioned, authenticated vault. No password or derived key is persisted.
use super::{CredentialStore, NativeCredentialStore, Secret};
use crate::error::{AppError, Result};
use aes_gcm_siv::{
    Aes256GcmSiv, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};
use zeroize::Zeroizing;

const AAD: &[u8] = b"RemoteHub vault v1 Argon2id m=65536,t=3,p=1 AES-256-GCM-SIV";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u8,
    salt: [u8; 16],
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}
pub struct MasterVault {
    path: PathBuf,
    key: Option<Zeroizing<[u8; 32]>>,
    native: Option<NativeCredentialStore>,
}
#[derive(Clone)]
pub struct SharedVault(pub Arc<Mutex<MasterVault>>);
static VAULT: OnceLock<SharedVault> = OnceLock::new();
pub fn shared() -> Option<SharedVault> {
    VAULT.get().cloned()
}
pub fn install(path: PathBuf) -> SharedVault {
    VAULT
        .get_or_init(|| SharedVault(Arc::new(Mutex::new(MasterVault::new(path)))))
        .clone()
}
fn failure() -> AppError {
    AppError::Operation("Coffre verrouillé, mot de passe incorrect ou fichier endommagé.".into())
}
fn derive(password: &str, salt: &[u8; 16]) -> Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(65536, 3, 1, Some(32)).map_err(|_| failure())?,
    )
    .hash_password_into(password.as_bytes(), salt, key.as_mut())
    .map_err(|_| failure())?;
    Ok(key)
}
impl MasterVault {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            key: None,
            native: NativeCredentialStore::initialize().ok(),
        }
    }
    pub fn configured(&self) -> bool {
        self.path.exists() || self.path.with_extension("enabled").exists()
    }
    pub fn ready(&self) -> bool {
        if self.configured() {
            self.key.is_some()
        } else {
            self.native.is_some()
        }
    }
    pub fn lock(&mut self) {
        self.key = None;
    }
    fn envelope(&self) -> Result<Envelope> {
        let metadata = std::fs::metadata(&self.path).map_err(|_| failure())?;
        if metadata.len() > 16 * 1024 * 1024 {
            return Err(failure());
        }
        let bytes = std::fs::read(&self.path).map_err(|_| failure())?;
        let value: Envelope = serde_json::from_slice(&bytes).map_err(|_| failure())?;
        if value.version != 1 {
            return Err(failure());
        }
        Ok(value)
    }
    fn decrypt(envelope: &Envelope, key: &[u8; 32]) -> Result<BTreeMap<String, Secret>> {
        let cipher = Aes256GcmSiv::new_from_slice(key).map_err(|_| failure())?;
        let bytes = Zeroizing::new(
            cipher
                .decrypt(
                    Nonce::from_slice(&envelope.nonce),
                    Payload {
                        msg: &envelope.ciphertext,
                        aad: AAD,
                    },
                )
                .map_err(|_| failure())?,
        );
        serde_json::from_slice(&bytes).map_err(|_| failure())
    }
    fn persist(
        &self,
        values: &BTreeMap<String, Secret>,
        key: &[u8; 32],
        salt: [u8; 16],
    ) -> Result<()> {
        let mut nonce = [0u8; 12];
        OsRng.try_fill_bytes(&mut nonce).map_err(|_| failure())?;
        let plain = Zeroizing::new(serde_json::to_vec(values).map_err(|_| failure())?);
        let cipher = Aes256GcmSiv::new_from_slice(key).map_err(|_| failure())?;
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &plain,
                    aad: AAD,
                },
            )
            .map_err(|_| failure())?;
        let bytes = serde_json::to_vec(&Envelope {
            version: 1,
            salt,
            nonce,
            ciphertext,
        })
        .map_err(|_| failure())?;
        crate::importers::exchange::write_atomic(&self.path, &bytes)
    }
    pub fn unlock(&mut self, password: &str) -> Result<()> {
        self.lock();
        let envelope = self.envelope()?;
        let key = derive(password, &envelope.salt)?;
        Self::decrypt(&envelope, &key)?;
        self.key = Some(key);
        Ok(())
    }
    pub fn create(&mut self, password: &str, ids: &[String]) -> Result<()> {
        if self.configured() {
            return Err(AppError::Operation("Le coffre existe déjà.".into()));
        }
        if password.chars().count() < 12 {
            return Err(AppError::Validation(
                "Utilisez au moins 12 caractères pour le mot de passe maître.".into(),
            ));
        }
        let mut values = BTreeMap::new();
        for id in ids {
            if let Some(secret) = self.get_secret(id)? {
                values.insert(id.clone(), secret);
            }
        }
        let mut salt = [0u8; 16];
        OsRng.try_fill_bytes(&mut salt).map_err(|_| failure())?;
        let key = derive(password, &salt)?;
        self.persist(&values, &key, salt)?;
        crate::importers::exchange::write_atomic(
            &self.path.with_extension("enabled"),
            b"RemoteHub master vault v1\n",
        )?;
        self.key = Some(key);
        // The durable encrypted copy exists before deleting any legacy secret.
        if let Some(native) = &mut self.native {
            for id in values.keys() {
                native.delete_secret(id).map_err(|_|AppError::Operation("Coffre chiffré créé, mais nettoyage du coffre système incomplet. Les anciennes copies doivent être supprimées du gestionnaire d'identifiants du système.".into()))?;
            }
        }
        Ok(())
    }
    pub fn change_password(&mut self, password: &str) -> Result<()> {
        if password.chars().count() < 12 {
            return Err(AppError::Validation(
                "Utilisez au moins 12 caractères.".into(),
            ));
        }
        let envelope = self.envelope()?;
        let values = Self::decrypt(&envelope, self.key.as_ref().ok_or_else(failure)?)?;
        let mut salt = [0u8; 16];
        OsRng.try_fill_bytes(&mut salt).map_err(|_| failure())?;
        let key = derive(password, &salt)?;
        self.persist(&values, &key, salt)?;
        self.key = Some(key);
        Ok(())
    }
}
impl CredentialStore for MasterVault {
    fn get_secret(&mut self, id: &str) -> Result<Option<Secret>> {
        crate::models::validate_id(id)?;
        if !self.configured() {
            return self.native.as_mut().ok_or_else(failure)?.get_secret(id);
        }
        Ok(Self::decrypt(&self.envelope()?, self.key.as_ref().ok_or_else(failure)?)?.remove(id))
    }
    fn store_secret(&mut self, id: &str, secret: &str) -> Result<()> {
        crate::models::validate_id(id)?;
        if !self.configured() {
            return self
                .native
                .as_mut()
                .ok_or_else(failure)?
                .store_secret(id, secret);
        }
        let envelope = self.envelope()?;
        let key = self.key.as_ref().ok_or_else(failure)?;
        let mut values = Self::decrypt(&envelope, key)?;
        values.insert(id.into(), Zeroizing::new(secret.into()));
        self.persist(&values, key, envelope.salt)
    }
    fn delete_secret(&mut self, id: &str) -> Result<()> {
        crate::models::validate_id(id)?;
        if !self.configured() {
            return self.native.as_mut().ok_or_else(failure)?.delete_secret(id);
        }
        let envelope = self.envelope()?;
        let key = self.key.as_ref().ok_or_else(failure)?;
        let mut values = Self::decrypt(&envelope, key)?;
        values.remove(id);
        self.persist(&values, key, envelope.salt)
    }
}
impl CredentialStore for SharedVault {
    fn get_secret(&mut self, id: &str) -> Result<Option<Secret>> {
        self.0.lock().map_err(|_| failure())?.get_secret(id)
    }
    fn store_secret(&mut self, id: &str, s: &str) -> Result<()> {
        self.0.lock().map_err(|_| failure())?.store_secret(id, s)
    }
    fn delete_secret(&mut self, id: &str) -> Result<()> {
        self.0.lock().map_err(|_| failure())?.delete_secret(id)
    }
}
