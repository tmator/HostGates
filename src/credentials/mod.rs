pub mod master;
use crate::{
    branding::CREDENTIAL_SERVICE,
    error::{AppError, Result},
    models::validate_id,
};
use zeroize::Zeroizing;

// Deliberately no Debug/Serialize on secrets or stores.
pub type Secret = Zeroizing<String>;
pub trait CredentialStore: Send {
    fn store_secret(&mut self, id: &str, secret: &str) -> Result<()>;
    fn get_secret(&mut self, id: &str) -> Result<Option<Secret>>;
    fn delete_secret(&mut self, id: &str) -> Result<()>;
    fn has_secret(&mut self, id: &str) -> Result<bool> {
        Ok(self.get_secret(id)?.is_some())
    }
}
pub struct NativeCredentialStore;
impl NativeCredentialStore {
    pub fn initialize() -> Result<Self> {
        #[cfg(windows)]
        keyring_core::set_default_store(
            windows_native_keyring_store::Store::new().map_err(|_| AppError::VaultUnavailable)?,
        );
        #[cfg(target_os = "linux")]
        keyring_core::set_default_store(
            dbus_secret_service_keyring_store::Store::new()
                .map_err(|_| AppError::VaultUnavailable)?,
        );
        #[cfg(not(any(windows, target_os = "linux")))]
        return Err(AppError::VaultUnavailable);
        Ok(Self)
    }
    fn entry(id: &str) -> Result<keyring_core::Entry> {
        validate_id(id)?;
        #[cfg(windows)]
        let entry = keyring_core::Entry::new_with_modifiers(
            CREDENTIAL_SERVICE,
            id,
            &std::collections::HashMap::from([("persistence", "local")]),
        );
        #[cfg(not(windows))]
        let entry = keyring_core::Entry::new(CREDENTIAL_SERVICE, id);
        entry.map_err(|_| AppError::VaultUnavailable)
    }
}
impl CredentialStore for NativeCredentialStore {
    fn store_secret(&mut self, id: &str, secret: &str) -> Result<()> {
        Self::entry(id)?
            .set_password(secret)
            .map_err(|_| AppError::VaultUnavailable)
    }
    fn get_secret(&mut self, id: &str) -> Result<Option<Secret>> {
        match Self::entry(id)?.get_password() {
            Ok(s) => Ok(Some(Zeroizing::new(s))),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(_) => Err(AppError::VaultUnavailable),
        }
    }
    fn delete_secret(&mut self, id: &str) -> Result<()> {
        match Self::entry(id)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(_) => Err(AppError::VaultUnavailable),
        }
    }
}
pub struct UnavailableStore;
impl CredentialStore for UnavailableStore {
    fn store_secret(&mut self, _: &str, _: &str) -> Result<()> {
        Err(AppError::VaultUnavailable)
    }
    fn get_secret(&mut self, _: &str) -> Result<Option<Secret>> {
        Err(AppError::VaultUnavailable)
    }
    fn delete_secret(&mut self, _: &str) -> Result<()> {
        Err(AppError::VaultUnavailable)
    }
}
