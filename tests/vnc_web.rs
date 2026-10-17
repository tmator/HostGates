use hostgates::{
    connections::{vnc, web},
    database::Database,
    importers::{
        exchange::{self, ExportFormat},
        mobaxterm::parse,
    },
    models::Protocol,
};

#[test]
fn vnc_and_web_import_export_and_storage() {
    let preview = parse(b"[Bookmarks]\nDesktop=#70#5%192.0.2.8%5901%-1%0%\nPortal=#313#11%https://example.org:8443/admin?q=ok%-1%-1%0%");
    let vnc = preview.rows[0].data.as_ref().expect("vnc");
    assert_eq!(vnc.protocol, Protocol::VNC);
    assert!(vnc.username.is_empty());
    assert_eq!(vnc::arguments(vnc).expect("args"), vec!["192.0.2.8::5901"]);
    let web = preview.rows[1].data.as_ref().expect("web");
    assert_eq!(web.protocol, Protocol::WEB);
    assert_eq!(web.port, 8443);
    assert!(web.username.is_empty());
    let mut db = Database::in_memory().expect("db");
    let report = db.import(&preview).expect("import");
    assert_eq!((report.vnc, report.web), (1, 1));
    let snapshot = db.snapshot().expect("snapshot");
    let ids = snapshot
        .sessions
        .iter()
        .map(|s| s.uuid.clone())
        .collect::<Vec<_>>();
    for format in [ExportFormat::Csv, ExportFormat::Mxt] {
        let bytes = exchange::export(&snapshot, &ids, format).expect("export");
        let reread = if format == ExportFormat::Csv {
            exchange::parse_csv(&bytes).expect("csv")
        } else {
            parse(&bytes)
        };
        assert_eq!(reread.rows.len(), 2);
        assert!(reread.rows.iter().all(|r| r.data.is_some()));
        let url = reread
            .rows
            .iter()
            .find_map(|r| r.data.as_ref().filter(|d| d.protocol == Protocol::WEB))
            .expect("web");
        assert_eq!(url.hostname, web.hostname);
    }
}
#[test]
fn browser_rejects_non_web_schemes_and_embedded_secrets() {
    for value in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "data:text/plain,x",
        "https://user:password@example.org",
        "https://user@example.org",
        "https://example.org\n",
        "-bad",
        "example.org",
    ] {
        assert!(web::validated_url(value).is_err(), "{value}");
    }
    for value in [
        "https://example.org/a%20b?x=1&y=2#part",
        "http://192.0.2.1:8080/",
        "https://[::1]/",
    ] {
        assert!(web::validated_url(value).is_ok(), "{value}");
    }
}
#[test]
fn v1_database_migration_preserves_children_and_credentials() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("old.db");
    {
        let conn = rusqlite::Connection::open(&path).expect("open");
        conn.execute_batch(include_str!("../migrations/001_initial.sql"))
            .expect("v1");
        conn.execute_batch("INSERT INTO folders(uuid,name) VALUES ('folder','Existing'); INSERT INTO sessions(uuid,name,protocol,hostname,port,credential_id,folder_id) VALUES ('session','Existing','SSH','host',22,'vault-id','folder'); INSERT INTO tags VALUES ('prod'); INSERT INTO session_tags VALUES ('session','prod'); INSERT INTO ssh_settings(session_id) VALUES ('session'); INSERT INTO recent_sessions VALUES ('session','today'); PRAGMA user_version=1;").expect("data");
    }
    {
        let db = Database::open(&path).expect("migrate");
        assert_eq!(db.schema_version().expect("version"), 2);
        let s = db.session("session").expect("session");
        assert_eq!(s.credential_id.as_deref(), Some("vault-id"));
        assert_eq!(s.data.tags, vec!["prod"]);
        assert_eq!(s.data.folder_id.as_deref(), Some("folder"));
    }
    let conn = rusqlite::Connection::open(path).expect("open migrated");
    for table in ["ssh_settings", "recent_sessions"] {
        let count: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .expect("children");
        assert_eq!(count, 1);
    }
    let errors: i64 = conn
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .expect("fk check");
    assert_eq!(errors, 0);
}

#[test]
fn web_identity_keeps_case_sensitive_paths() {
    let a = parse(b"[Bookmarks]\nA=#313#11%https://EXAMPLE.org/Admin%-1%-1%\nB=#313#11%https://example.org/admin%-1%-1%");
    let first = a.rows[0].data.as_ref().expect("a");
    let second = a.rows[1].data.as_ref().expect("b");
    assert_ne!(exchange::identity(first), exchange::identity(second));
    let mut same = first.clone();
    same.hostname = "https://example.org/Admin".into();
    assert_eq!(exchange::identity(first), exchange::identity(&same));
}
