use hostgates::{
    credentials::{CredentialStore, Secret},
    database::Database,
    error::{AppError, Result},
    models::*,
    services::SessionService,
    ui::matches_search,
};
use std::collections::HashMap;
use zeroize::Zeroizing;
fn session() -> SessionInput {
    SessionInput {
        name: "Serveur de test".into(),
        protocol: Protocol::SSH,
        hostname: "db.example.org".into(),
        port: 22,
        username: "admin".into(),
        domain: String::new(),
        folder_id: None,
        favorite: false,
        description: "Production".into(),
        color: "#7195ff".into(),
        icon: "server".into(),
        tags: vec!["linux".into()],
    }
}
fn folder(name: &str, parent_id: Option<String>) -> FolderInput {
    FolderInput {
        name: name.into(),
        parent_id,
        sort_order: 0,
        color: "#7195ff".into(),
        icon: "folder".into(),
    }
}
#[derive(Default)]
struct MockVault {
    values: HashMap<String, Secret>,
    fail: bool,
}
impl CredentialStore for MockVault {
    fn store_secret(&mut self, id: &str, s: &str) -> Result<()> {
        if self.fail {
            return Err(AppError::VaultUnavailable);
        }
        self.values.insert(id.into(), Zeroizing::new(s.into()));
        Ok(())
    }
    fn get_secret(&mut self, id: &str) -> Result<Option<Secret>> {
        if self.fail {
            return Err(AppError::VaultUnavailable);
        }
        Ok(self.values.get(id).map(|s| Zeroizing::new(s.to_string())))
    }
    fn delete_secret(&mut self, id: &str) -> Result<()> {
        if self.fail {
            return Err(AppError::VaultUnavailable);
        }
        self.values.remove(id);
        Ok(())
    }
}
#[test]
fn migrations_are_atomic_and_idempotent() -> Result<()> {
    let path = std::env::temp_dir().join(format!("hostgates-test-{}.db", uuid::Uuid::new_v4()));
    {
        let mut db = Database::open(&path)?;
        assert_eq!(db.schema_version()?, 2);
        db.save_session(None, &session())?;
    }
    {
        let db = Database::open(&path)?;
        assert_eq!(db.schema_version()?, 2);
        assert_eq!(db.snapshot()?.sessions.len(), 1);
    }
    std::fs::remove_file(path).expect("clean test database");
    Ok(())
}
#[test]
fn future_schema_is_rejected() {
    let path = std::env::temp_dir().join(format!("hostgates-test-{}.db", uuid::Uuid::new_v4()));
    {
        let conn = rusqlite::Connection::open(&path).expect("database");
        conn.pragma_update(None, "user_version", 999)
            .expect("version");
    }
    assert!(Database::open(&path).is_err());
    std::fs::remove_file(path).expect("cleanup");
}
#[test]
fn sessions_crud_and_tags() -> Result<()> {
    let mut db = Database::in_memory()?;
    let mut s = session();
    let id = db.save_session(None, &s)?;
    s.name = "Nouveau nom".into();
    s.tags = vec!["production".into(), "production".into()];
    s.favorite = true;
    db.save_session(Some(&id), &s)?;
    let saved = db.session(&id)?;
    assert_eq!(saved.data.name, s.name);
    assert_eq!(saved.data.tags, vec!["production"]);
    assert!(saved.data.favorite);
    assert!(saved.credential_id.is_none());
    db.delete_session(&id)?;
    assert!(db.session(&id).is_err());
    Ok(())
}
#[test]
fn rejects_injection_and_invalid_ports() {
    for host in [
        "-oProxyCommand=x",
        "host;rm -rf /",
        "host\nother",
        "user@host",
        "ssh://host",
        "",
    ] {
        let mut s = session();
        s.hostname = host.into();
        assert!(s.validate().is_err());
    }
    let mut s = session();
    s.port = 0;
    assert!(s.validate().is_err());
    s.port = 65535;
    s.hostname = "[2001:db8::1]".into();
    assert!(s.validate().is_ok());
}
#[test]
fn sql_values_are_parameters() -> Result<()> {
    let mut db = Database::in_memory()?;
    let mut s = session();
    s.name = "'; DROP TABLE sessions; --".into();
    db.save_session(None, &s)?;
    assert_eq!(db.snapshot()?.sessions[0].data.name, s.name);
    Ok(())
}
#[test]
fn folder_cycles_rejected_and_nonempty_protected() -> Result<()> {
    let mut db = Database::in_memory()?;
    let a = db.save_folder(None, &folder("A", None))?;
    let b = db.save_folder(None, &folder("B", Some(a.clone())))?;
    assert!(
        db.save_folder(Some(&a), &folder("A", Some(b.clone())))
            .is_err()
    );
    assert!(db.delete_folder(&a).is_err());
    let mut s = session();
    s.folder_id = Some(b.clone());
    let id = db.save_session(None, &s)?;
    assert!(db.delete_folder(&b).is_err());
    s.folder_id = None;
    db.save_session(Some(&id), &s)?;
    db.delete_folder(&b)?;
    db.delete_folder(&a)?;
    Ok(())
}
#[test]
fn missing_parent_rolls_back_session() -> Result<()> {
    let mut db = Database::in_memory()?;
    let mut s = session();
    s.folder_id = Some(uuid::Uuid::new_v4().to_string());
    assert!(db.save_session(None, &s).is_err());
    assert!(db.snapshot()?.sessions.is_empty());
    Ok(())
}
#[test]
fn secrets_are_separate_and_duplicates_have_none() -> Result<()> {
    let mut service = SessionService {
        db: Database::in_memory()?,
        vault: Box::<MockVault>::default(),
    };
    let id = service.save(None, &session())?;
    service.set_secret(&id, Zeroizing::new("test-only-value".into()))?;
    let credential = service.db.credential_reference(&id)?.expect("reference");
    assert!(service.vault.has_secret(&credential)?);
    let copied = service.save(None, &service.db.session(&id)?.data)?;
    assert!(service.db.credential_reference(&copied)?.is_none());
    service.delete(&id)?;
    assert!(!service.vault.has_secret(&credential)?);
    Ok(())
}
#[test]
fn vault_failure_does_not_create_false_reference() -> Result<()> {
    let mut service = SessionService {
        db: Database::in_memory()?,
        vault: Box::new(MockVault {
            fail: true,
            ..Default::default()
        }),
    };
    let id = service.save(None, &session())?;
    assert!(
        service
            .set_secret(&id, Zeroizing::new("test-value".into()))
            .is_err()
    );
    assert!(service.db.credential_reference(&id)?.is_none());
    Ok(())
}
#[test]
fn secret_deletion_failure_preserves_session() -> Result<()> {
    let mut db = Database::in_memory()?;
    let id = db.save_session(None, &session())?;
    db.link_credential(&id, Some(&uuid::Uuid::new_v4().to_string()))?;
    let mut service = SessionService {
        db,
        vault: Box::new(MockVault {
            fail: true,
            ..Default::default()
        }),
    };
    assert!(service.delete(&id).is_err());
    assert!(service.db.session(&id).is_ok());
    Ok(())
}
#[test]
fn search_includes_folder_tags_and_case() -> Result<()> {
    let mut db = Database::in_memory()?;
    let f = db.save_folder(None, &folder("Lorawan", None))?;
    let mut s = session();
    s.folder_id = Some(f);
    db.save_session(None, &s)?;
    let snap = db.snapshot()?;
    for q in ["LORAWAN", "linux", "ADMIN", "db.example", "production"] {
        assert!(matches_search(&snap.sessions[0], q, &snap));
    }
    assert!(!matches_search(&snap.sessions[0], "does-not-exist", &snap));
    Ok(())
}
#[test]
fn theme_is_validated_and_persisted() -> Result<()> {
    let mut db = Database::in_memory()?;
    db.set_theme("dark")?;
    assert_eq!(db.snapshot()?.theme, "dark");
    assert!(db.set_theme("unknown").is_err());
    Ok(())
}
#[test]
fn no_secret_value_columns_in_schema() {
    let conn = rusqlite::Connection::open_in_memory().expect("db");
    conn.execute_batch(include_str!("../migrations/001_initial.sql"))
        .expect("migration");
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .expect("tables");
    let tables = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("query")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("rows");
    for table in tables {
        let mut stmt = conn
            .prepare("SELECT name FROM pragma_table_info(?1)")
            .expect("schema");
        for col in stmt
            .query_map([table], |r| r.get::<_, String>(0))
            .expect("columns")
        {
            assert!(!["password", "secret", "passphrase"].contains(&col.expect("col").as_str()));
        }
    }
}
#[test]
#[ignore = "Uses a temporary entry in the OS credential store; run explicitly"]
fn native_vault_roundtrip() -> Result<()> {
    let mut vault = hostgates::credentials::NativeCredentialStore::initialize()?;
    let id = uuid::Uuid::new_v4().to_string();
    vault.store_secret(&id, "hostgates-integration-test")?;
    let read = vault.get_secret(&id);
    let deleted = vault.delete_secret(&id);
    deleted?;
    assert_eq!(
        read?.as_deref().map(|s| s.as_str()),
        Some("hostgates-integration-test")
    );
    assert!(!vault.has_secret(&id)?);
    Ok(())
}

#[test]
fn moving_session_preserves_metadata_and_secret_and_rejects_missing_folder() -> Result<()> {
    let mut db = Database::in_memory()?;
    let id = db.save_session(None, &session())?;
    let destination = db.save_folder(None, &folder("Destination", None))?;
    let credential = uuid::Uuid::new_v4().to_string();
    db.link_credential(&id, Some(&credential))?;
    db.move_session(&id, Some(&destination))?;
    let moved = db.session(&id)?;
    assert_eq!(moved.data.folder_id.as_deref(), Some(destination.as_str()));
    assert_eq!(moved.credential_id.as_deref(), Some(credential.as_str()));
    assert_eq!(moved.data.tags, session().tags);
    assert_eq!(moved.data.hostname, session().hostname);
    assert!(
        db.move_session(&id, Some(&uuid::Uuid::new_v4().to_string()))
            .is_err()
    );
    assert_eq!(db.session(&id)?.data.folder_id, Some(destination));
    db.move_session(&id, None)?;
    assert!(db.session(&id)?.data.folder_id.is_none());
    assert!(
        db.move_session(&uuid::Uuid::new_v4().to_string(), None)
            .is_err()
    );
    Ok(())
}

#[test]
fn search_matches_name_ip_and_ancestor_folder_with_trimmed_query() -> Result<()> {
    let mut db = Database::in_memory()?;
    let parent = db.save_folder(None, &folder("Production", None))?;
    let child = db.save_folder(None, &folder("Paris", Some(parent)))?;
    let mut data = session();
    data.name = "Soleil12".into();
    data.hostname = "192.0.2.12".into();
    data.description.clear();
    data.tags.clear();
    data.folder_id = Some(child);
    let id = db.save_session(None, &data)?;
    let snapshot = db.snapshot()?;
    let s = db.session(&id)?;
    for query in [" soleil12 ", "192.0.2.", "PRODUCTION", "paris"] {
        assert!(matches_search(&s, query, &snapshot), "{query}");
    }
    assert!(!matches_search(&s, "absent", &snapshot));
    Ok(())
}

#[test]
fn each_protocol_has_an_independent_replaceable_vault_secret() -> Result<()> {
    let mut service = SessionService {
        db: Database::in_memory()?,
        vault: Box::<MockVault>::default(),
    };
    let mut refs = std::collections::HashSet::new();
    for protocol in [Protocol::SSH, Protocol::RDP, Protocol::VNC, Protocol::WEB] {
        let mut data = session();
        data.protocol = protocol;
        if data.protocol == Protocol::WEB {
            data.hostname = "https://example.org".into();
            data.port = 443;
        }
        let id = service.save(None, &data)?;
        service.set_secret(&id, Zeroizing::new("first-fixture-secret".into()))?;
        let reference = service.db.credential_reference(&id)?.expect("reference");
        assert!(refs.insert(reference.clone()));
        service.set_secret(&id, Zeroizing::new("replacement-fixture-secret".into()))?;
        assert_eq!(
            service
                .vault
                .get_secret(&reference)?
                .as_deref()
                .map(|v| v.as_str()),
            Some("replacement-fixture-secret")
        );
        service.save(Some(&id), &data)?;
        assert_eq!(
            service.db.credential_reference(&id)?,
            Some(reference.clone())
        );
        service.remove_secret(&id)?;
        assert!(service.vault.get_secret(&reference)?.is_none());
    }
    Ok(())
}

#[test]
fn manual_gateway_uses_vault_and_rolls_back_on_secret_failure() -> Result<()> {
    let mut service = SessionService {
        db: Database::in_memory()?,
        vault: Box::<MockVault>::default(),
    };
    let data = session();
    let id = service.create_gateway(&data, Zeroizing::new("gateway-fixture-secret".into()))?;
    let reference = service
        .db
        .credential_reference(&id)?
        .expect("vault reference");
    assert_eq!(
        service
            .vault
            .get_secret(&reference)?
            .as_deref()
            .map(|v| v.as_str()),
        Some("gateway-fixture-secret")
    );
    assert_eq!(service.db.session(&id)?.data.port, data.port);
    let mut failing = SessionService {
        db: Database::in_memory()?,
        vault: Box::new(MockVault {
            fail: true,
            ..Default::default()
        }),
    };
    assert!(
        failing
            .create_gateway(&data, Zeroizing::new("fixture".into()))
            .is_err()
    );
    assert!(failing.db.snapshot()?.sessions.is_empty());
    let mut invalid = data;
    invalid.protocol = Protocol::RDP;
    assert!(
        service
            .create_gateway(&invalid, Zeroizing::new(String::new()))
            .is_err()
    );
    Ok(())
}
