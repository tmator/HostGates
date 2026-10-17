use hostgates::{
    database::Database,
    error::Result,
    importers::{
        DuplicatePolicy,
        exchange::{self, ExportFormat},
        mobaxterm::parse,
    },
};
const SSH: &[u8] = include_bytes!("fixtures/moba_ssh_basic.mxtsessions");
const RDP: &[u8] = include_bytes!("fixtures/moba_rdp_basic.mxtsessions");
#[test]
fn basic_ssh_and_rdp() {
    let p = parse(SSH);
    let s = p.rows[0].data.as_ref().expect("ssh");
    assert_eq!(s.hostname, "192.0.2.10");
    assert_eq!(s.port, 22);
    assert!(p.rows[0].folder.is_empty());
    assert_eq!(parse(RDP).rows[0].kind, "RDP");
}
#[test]
fn short_empty_user_supported() {
    let p = parse(include_bytes!(
        "fixtures/moba_short_putty_session.mxtsessions"
    ));
    assert_eq!(p.rows[0].data.as_ref().expect("short").username, "");
}
#[test]
fn unknown_and_wsl_not_imported() {
    let p = parse(include_bytes!("fixtures/moba_unknown_type.mxtsessions"));
    assert_eq!(p.rows.len(), 3);
    assert_eq!(p.rows[0].kind, "data");
    assert!(p.rows.iter().all(|r| !r.selected && r.data.is_none()));
}
#[test]
fn malformed_is_reported_without_panic() {
    let p = parse(include_bytes!("fixtures/moba_malformed.mxtsessions"));
    assert_eq!(p.rows.len(), 3);
    assert!(p.rows.iter().all(|r| r.issue.is_some()));
    assert_eq!(p.warnings.len(), 1);
}
#[test]
fn utf8_bom_crlf_and_windows_1252() {
    let p = parse(b"\xef\xbb\xbf[Bookmarks]\r\nSubRep=\r\nSSH=#109#0%host%22%user\r\n");
    assert_eq!(p.rows.len(), 1);
    let p = parse(b"[Bookmarks]\nCaf\xe9=#109#0%host%22%user\n");
    assert_eq!(p.rows[0].name, "Café");
    assert_eq!(p.warnings.len(), 1);
}
#[test]
fn section_metadata_order_and_unrelated_credentials() {
    let p=parse(b"[Passwords]\npassword=not-a-session\n[Bookmarks_1]\nS=#109#0%host%22%user\nSubRep=Folder\n[Other]\nx=ignore\n");
    assert_eq!(p.rows.len(), 1);
    assert_eq!(p.rows[0].folder, "Folder");
}
#[test]
fn import_nested_folders_and_duplicate_skip() -> Result<()> {
    let mut db = Database::in_memory()?;
    let p = parse(include_bytes!("fixtures/moba_folders.mxtsessions"));
    let report = db.import(&p)?;
    assert_eq!(report.ssh, 2);
    assert_eq!(db.snapshot()?.folders.len(), 2);
    assert_eq!(db.import(&p)?.skipped, 2);
    Ok(())
}
#[test]
fn duplicate_replace_preserves_secret_reference() -> Result<()> {
    let mut db = Database::in_memory()?;
    let mut p = parse(SSH);
    db.import(&p)?;
    let s = db.snapshot()?.sessions.remove(0);
    let credential = uuid::Uuid::new_v4().to_string();
    db.link_credential(&s.uuid, Some(&credential))?;
    p.rows[0].policy = DuplicatePolicy::Replace;
    p.rows[0].data.as_mut().expect("data").name = "Updated".into();
    assert_eq!(db.import(&p)?.replaced, 1);
    assert_eq!(db.session(&s.uuid)?.data.name, "Updated");
    assert_eq!(db.credential_reference(&s.uuid)?, Some(credential));
    Ok(())
}
#[test]
fn duplicate_import_creates_new_without_credential() -> Result<()> {
    let mut db = Database::in_memory()?;
    let mut p = parse(SSH);
    db.import(&p)?;
    p.rows[0].policy = DuplicatePolicy::Import;
    db.import(&p)?;
    assert_eq!(db.snapshot()?.sessions.len(), 2);
    assert!(
        db.snapshot()?
            .sessions
            .iter()
            .all(|s| s.credential_id.is_none())
    );
    Ok(())
}
#[test]
fn failed_batch_rolls_back_earlier_rows_and_folders() -> Result<()> {
    let mut db = Database::in_memory()?;
    let mut p = parse(SSH);
    let mut bad = p.rows[0].clone();
    bad.data.as_mut().expect("data").hostname = "--bad".into();
    bad.folder = "temp/sub".into();
    bad.policy = DuplicatePolicy::Import;
    p.rows.push(bad);
    assert!(db.import(&p).is_err());
    assert!(db.snapshot()?.sessions.is_empty());
    assert!(db.snapshot()?.folders.is_empty());
    Ok(())
}
#[test]
fn ambiguous_replace_is_rejected() -> Result<()> {
    let mut db = Database::in_memory()?;
    let mut p = parse(SSH);
    p.rows[0].policy = DuplicatePolicy::Import;
    db.import(&p)?;
    db.import(&p)?;
    p.rows[0].policy = DuplicatePolicy::Replace;
    assert!(db.import(&p).is_err());
    assert_eq!(db.snapshot()?.sessions.len(), 2);
    Ok(())
}
#[test]
fn mxt_and_csv_roundtrips_no_secrets() -> Result<()> {
    let mut db = Database::in_memory()?;
    db.import(&parse(include_bytes!("fixtures/moba_folders.mxtsessions")))?;
    db.import(&parse(RDP))?;
    let snap = db.snapshot()?;
    let ids: Vec<_> = snap.sessions.iter().map(|s| s.uuid.clone()).collect();
    for format in [ExportFormat::Mxt, ExportFormat::Csv] {
        let bytes = exchange::export(&snap, &ids, format)?;
        assert!(!String::from_utf8_lossy(&bytes).contains("credential"));
        let p = if format == ExportFormat::Mxt {
            parse(&bytes)
        } else {
            exchange::parse_csv(&bytes)?
        };
        let mut copy = Database::in_memory()?;
        copy.import(&p)?;
        assert_eq!(copy.snapshot()?.sessions.len(), 3);
        assert_eq!(copy.snapshot()?.folders.len(), 2);
    }
    Ok(())
}
#[test]
fn csv_quotes_names_and_alias_headers() -> Result<()> {
    let p = exchange::parse_csv(
        b"Session type,Remote Host,Username,Port,Session name\nSSH,host,user,22,\"Name, quoted\"\n",
    )?;
    assert_eq!(p.rows[0].name, "Name, quoted");
    assert!(p.rows[0].data.is_some());
    Ok(())
}
#[test]
fn mxt_refuses_lossy_names_csv_preserves() -> Result<()> {
    let mut db = Database::in_memory()?;
    let mut p = parse(SSH);
    p.rows[0].data.as_mut().expect("data").name = "a=b".into();
    db.import(&p)?;
    let snap = db.snapshot()?;
    let ids = vec![snap.sessions[0].uuid.clone()];
    assert!(exchange::export(&snap, &ids, ExportFormat::Mxt).is_err());
    let bytes = exchange::export(&snap, &ids, ExportFormat::Csv)?;
    assert_eq!(exchange::parse_csv(&bytes)?.rows[0].name, "a=b");
    Ok(())
}
#[test]
fn preview_marks_in_file_duplicates() -> Result<()> {
    let db = Database::in_memory()?;
    let temp = tempfile::Builder::new()
        .suffix(".mxtsessions")
        .tempfile()
        .expect("temp");
    std::fs::write(
        temp.path(),
        b"[Bookmarks]\na=#109#0%host%22%u\nb=#109#0%HOST%22%u\n",
    )
    .expect("fixture");
    let p = exchange::read_preview(temp.path(), &db.snapshot()?)?;
    assert!(!p.rows[0].duplicate);
    assert!(p.rows[1].duplicate);
    Ok(())
}
#[test]
fn export_atomic_replaces_file() -> Result<()> {
    let dir = tempfile::tempdir().expect("dir");
    let p = dir.path().join("out.csv");
    exchange::write_atomic(&p, b"one")?;
    exchange::write_atomic(&p, b"two")?;
    assert_eq!(std::fs::read(p).expect("read"), b"two");
    Ok(())
}
#[test]
fn parser_handles_arbitrary_bytes() {
    for i in 0..256u16 {
        let bytes = vec![i as u8; 100];
        let _ = parse(&bytes);
    }
    assert!(
        parse(b"[Bookmarks]\na=#109#0%-oBad%22%u").rows[0]
            .data
            .is_none()
    );
}

#[test]
fn mxt_rejects_reserved_metadata_names() -> Result<()> {
    for name in ["SubRep", "ImgNum"] {
        let mut db = Database::in_memory()?;
        let mut p = parse(SSH);
        p.rows[0].data.as_mut().expect("data").name = name.into();
        db.import(&p)?;
        let snap = db.snapshot()?;
        let ids = vec![snap.sessions[0].uuid.clone()];
        assert!(exchange::export(&snap, &ids, ExportFormat::Mxt).is_err());
        assert!(exchange::export(&snap, &ids, ExportFormat::Csv).is_ok());
    }
    Ok(())
}

#[test]
fn protocol_comes_from_mode_not_icon() {
    for icon in ["70", "109", "91", "149", "105", "151", "9999"] {
        for (mode, expected) in [("0", "SSH"), ("4", "RDP")] {
            let input = format!("[Bookmarks]\nExample=#{icon}#{mode}%192.0.2.9%2200%user%\n");
            let preview = parse(input.as_bytes());
            assert_eq!(preview.rows[0].kind, expected);
            assert_eq!(
                preview.rows[0].data.as_ref().expect("valid protocol").port,
                2200
            );
        }
    }
}
#[test]
fn soleil_rdp_layouts_with_custom_icon() {
    // User-provided layouts; host addresses anonymized for the fixture.
    let input = "[Bookmarks]\nSoleil2=#70#4%192.0.2.220%3389%username%0%-1%-1%-1%-1%0%0%-1%%%%%0%0%%-1%%-1%-1%0%-1%0%-1%0%0%0%0#MobaFont%10%0%0%-1%15%236,236,236%30,30,30%180,180,192%0%-1%0%%xterm%-1%0%_Std_Colors_0_%80%24%0%1%-1%<none>%%0%0%-1%0#0# #-1\nSoleil12=#70#4%192.0.2.1%3389%username%0%-1%-1%-1%-1%0%0%-1%%%%%0%0%%-1%%-1%-1%0%-1%0%-1%0%0%0%0%#MobaFont%10%0%0%-1%15%236,236,236%30,30,30%180,180,192%0%-1%0%%xterm%-1%0%_Std_Colors_0_%80%24%0%1%-1%<none>%%0%0%-1%0%#0# #-1";
    let preview = parse(input.as_bytes());
    assert_eq!(preview.rows.len(), 2);
    for row in &preview.rows {
        assert!(row.selected);
        assert_eq!(row.kind, "RDP");
        assert_eq!(row.data.as_ref().expect("RDP").port, 3389);
        assert_eq!(row.data.as_ref().expect("RDP").username, "username");
    }
    let mut db = Database::in_memory().expect("database");
    assert_eq!(db.import(&preview).expect("import").rdp, 2);
}
#[test]
fn misleading_icons_do_not_convert_other_protocols() {
    for mode in ["1", "2", "3", "6", "7", "8", "12", "99", ""] {
        let input = format!("[Bookmarks]\nUnknown=#109#{mode}%192.0.2.1%22%user%\n");
        assert!(parse(input.as_bytes()).rows[0].data.is_none());
    }
}
#[test]
fn bare_moba_and_logout_prefix_are_supported_without_executing_prefix() {
    let preview = parse(b"Session=; logout#70#4%192.0.2.1%3389%%#MobaFont%10");
    assert_eq!(preview.rows[0].kind, "RDP");
    assert_eq!(
        preview.rows[0]
            .data
            .as_ref()
            .expect("bare session")
            .username,
        ""
    );
    assert!(
        parse(b"Session=anything#70#4%host%3389%u").rows[0]
            .data
            .is_none()
    );
}
