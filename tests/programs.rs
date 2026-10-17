use hostgates::{
    connections::{ProgramKind, resolve_program},
    database::Database,
    models::ExternalPrograms,
};

#[test]
fn settings_survive_reopen_and_reset() {
    let dir = tempfile::tempdir().expect("test fixture must be valid");
    let path = dir.path().join("settings.db");
    let exe = std::env::current_exe()
        .expect("test fixture must be valid")
        .to_str()
        .expect("test fixture must be valid")
        .to_owned();
    let programs = ExternalPrograms {
        ssh: exe.clone(),
        rdp: exe.clone(),
        vnc: exe.clone(),
        terminal: exe,
    };
    {
        let mut db = Database::open(&path).expect("test fixture must be valid");
        assert_eq!(
            db.snapshot().expect("test fixture must be valid").programs,
            ExternalPrograms::default()
        );
        db.set_programs(&programs)
            .expect("test fixture must be valid");
    }
    let mut db = Database::open(&path).expect("test fixture must be valid");
    assert_eq!(
        db.snapshot().expect("test fixture must be valid").programs,
        programs
    );
    db.set_programs(&ExternalPrograms::default())
        .expect("test fixture must be valid");
    assert_eq!(
        db.programs().expect("test fixture must be valid"),
        ExternalPrograms::default()
    );
}

#[test]
fn invalid_override_never_falls_back_and_save_is_atomic() {
    let mut db = Database::in_memory().expect("test fixture must be valid");
    let exe = std::env::current_exe()
        .expect("test fixture must be valid")
        .to_str()
        .expect("test fixture must be valid")
        .to_owned();
    let previous = ExternalPrograms {
        ssh: exe.clone(),
        ..Default::default()
    };
    db.set_programs(&previous)
        .expect("test fixture must be valid");
    for invalid in ["missing-client", "./ssh", "ssh -v", "\"ssh\"", "bad\npath"] {
        assert!(resolve_program(invalid, ProgramKind::Ssh).is_err());
        let updated = ExternalPrograms {
            ssh: String::new(),
            rdp: invalid.into(),
            vnc: String::new(),
            terminal: exe.clone(),
        };
        assert!(db.set_programs(&updated).is_err());
        assert_eq!(db.programs().expect("test fixture must be valid"), previous);
    }
}

#[test]
fn explicit_path_with_spaces_and_removed_program() {
    let dir = tempfile::tempdir().expect("test fixture must be valid");
    let file = dir.path().join("test client.exe");
    std::fs::write(&file, b"test fixture, never executed").expect("test fixture must be valid");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700))
            .expect("test fixture must be valid");
    }
    assert_eq!(
        resolve_program(
            file.to_str().expect("test fixture must be valid"),
            ProgramKind::Ssh
        )
        .expect("test fixture must be valid"),
        file
    );
    std::fs::remove_file(&file).expect("test fixture must be valid");
    assert!(
        resolve_program(
            file.to_str().expect("test fixture must be valid"),
            ProgramKind::Ssh
        )
        .is_err()
    );
    assert!(
        resolve_program(
            dir.path().to_str().expect("test fixture must be valid"),
            ProgramKind::Ssh
        )
        .is_err()
    );
}

#[test]
fn empty_override_uses_path_lookup() {
    for kind in [ProgramKind::Ssh, ProgramKind::Rdp, ProgramKind::Terminal] {
        assert_eq!(
            resolve_program("", kind).ok(),
            hostgates::connections::find_program(kind.defaults())
        );
    }
}
