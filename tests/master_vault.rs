use hostgates::credentials::{CredentialStore, master::MasterVault};
#[test]
fn encrypted_vault_roundtrip_lock_password_change_and_tamper() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("vault.json");
    let mut vault = MasterVault::new(path.clone());
    let id = uuid::Uuid::new_v4().to_string();
    assert!(vault.create("short", &[]).is_err());
    assert!(!path.exists());
    vault
        .create("correct horse battery staple", &[])
        .expect("create");
    vault
        .store_secret(&id, "never-plaintext-SECRET")
        .expect("store");
    let bytes = std::fs::read(&path).expect("file");
    assert!(!String::from_utf8_lossy(&bytes).contains("never-plaintext"));
    assert_eq!(
        vault
            .get_secret(&id)
            .expect("read")
            .as_deref()
            .map(|s| s.as_str()),
        Some("never-plaintext-SECRET")
    );
    vault.lock();
    assert!(!vault.ready());
    assert!(vault.get_secret(&id).is_err());
    assert!(vault.store_secret(&id, "new").is_err());
    assert!(vault.unlock("wrong password").is_err());
    assert!(!vault.ready());
    vault
        .unlock("correct horse battery staple")
        .expect("unlock");
    vault
        .change_password("another long passphrase")
        .expect("change");
    vault.lock();
    assert!(vault.unlock("correct horse battery staple").is_err());
    vault
        .unlock("another long passphrase")
        .expect("new password");
    assert!(vault.get_secret(&id).expect("read").is_some());
    vault.delete_secret(&id).expect("delete");
    assert!(vault.get_secret(&id).expect("read").is_none());
    vault.lock();
    let original = std::fs::read(&path).expect("file");
    let mut envelope: serde_json::Value = serde_json::from_slice(&original).expect("json");
    let n = envelope["ciphertext"][0].as_u64().expect("byte");
    envelope["ciphertext"][0] = serde_json::json!(n ^ 1);
    std::fs::write(&path, serde_json::to_vec(&envelope).expect("json")).expect("tamper");
    assert!(vault.unlock("another long passphrase").is_err());
    assert!(!vault.ready());
    std::fs::remove_file(&path).expect("remove encrypted file");
    let mut reopened = MasterVault::new(path);
    assert!(
        reopened.configured(),
        "missing encrypted file must not fall back to native storage"
    );
    assert!(!reopened.ready());
    assert!(reopened.get_secret(&id).is_err());
}
#[test]
fn file_names_cannot_escape_download_destination() {
    use hostgates::connections::native::safe_name;
    use std::path::Path;
    for name in ["file.txt", "été.txt", "folder"] {
        assert!(safe_name(Path::new(name)).is_ok());
    }
    for name in ["..", ".", "/", ""] {
        assert!(safe_name(Path::new(name)).is_err());
    }
    #[cfg(windows)]
    for name in ["NUL", "CON.txt", "COM1", "foo.", "foo ", "file:stream"] {
        assert!(safe_name(Path::new(name)).is_err(), "{name}");
    }
}
