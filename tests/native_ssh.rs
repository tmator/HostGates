use hostgates::connections::{
    native::{self, Auth, FileAction, FileBrowser, Tunnel},
    terminal::Terminal,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
struct Fixture(std::process::Child);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn wait(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !condition() {
        assert!(Instant::now() < deadline, "fixture timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn finish(files: &mut FileBrowser) {
    wait(|| {
        files.poll();
        !files.busy
    });
    assert_eq!(files.status, "Terminé", "SFTP operation: {}", files.status);
}
#[test]
#[ignore = "Requires Python and Paramiko; loopback fixture only"]
fn native_terminal_sftp_roundtrip_and_tunnel() {
    let dir = tempfile::tempdir().expect("tempdir");
    let _fixture = Fixture(
        std::process::Command::new("python")
            .arg("scripts/native-ssh-fixture.py")
            .arg(dir.path())
            .env("PYTHONPATH", ".tools/ssh-test")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("python"),
    );
    wait(|| dir.path().join("port").exists());
    let port = std::fs::read_to_string(dir.path().join("port"))
        .expect("port")
        .parse()
        .expect("number");
    let pin = std::fs::read_to_string(dir.path().join("fingerprint")).expect("pin");
    let mut auth = Auth {
        host: "127.0.0.1".into(),
        port,
        user: "fixture".into(),
        password: zeroize::Zeroizing::new("fixture-secret".into()),
        key: PathBuf::new(),
        fingerprint: "wrong".into(),
        endpoint: None,
    };
    assert!(native::connect(&auth).is_err());
    auth.fingerprint = pin;
    let ctx = eframe::egui::Context::default();
    let mut terminal =
        Terminal::from_ssh(native::connect(&auth).expect("SSH"), ctx.clone()).expect("terminal");
    wait(|| {
        terminal.poll();
        terminal
            .parser
            .screen()
            .contents()
            .contains("NATIVE_SSH_READY")
    });
    terminal.write(b"COMMAND_ECHO\r".to_vec());
    wait(|| {
        terminal.poll();
        terminal.parser.screen().contents().contains("COMMAND_ECHO")
    });
    terminal.resize(30, 100);
    let mut files = FileBrowser::start(auth.clone(), ctx);
    files.send(FileAction::List(".".into()));
    finish(&mut files);
    let upload = dir.path().join("upload");
    std::fs::create_dir_all(upload.join("nested")).expect("folder");
    std::fs::write(
        upload.join("nested/data.bin"),
        (0..=255u8).cycle().take(100_000).collect::<Vec<_>>(),
    )
    .expect("file");
    files.send(FileAction::Upload(vec![upload.clone()], files.path.clone()));
    finish(&mut files);
    let entry = files
        .entries
        .iter()
        .find(|e| e.name == "upload")
        .expect("uploaded folder")
        .clone();
    assert!(entry.directory);
    let downloads = dir.path().join("downloads");
    std::fs::create_dir(&downloads).expect("downloads");
    files.send(FileAction::Download(vec![entry.clone()], downloads.clone()));
    finish(&mut files);
    assert_eq!(
        std::fs::read(upload.join("nested/data.bin")).expect("source"),
        std::fs::read(downloads.join("upload/nested/data.bin")).expect("destination")
    );
    files.send(FileAction::Download(vec![entry], downloads.clone()));
    wait(|| {
        files.poll();
        !files.busy
    });
    assert_ne!(files.status, "Terminé", "must not overwrite existing files");
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("echo");
    let echo_port = listener.local_addr().expect("addr").port();
    let echo = std::thread::spawn(move || {
        let (mut c, _) = listener.accept().expect("accept");
        let mut bytes = Vec::new();
        c.read_to_end(&mut bytes).expect("read");
        c.write_all(&bytes).expect("echo");
    });
    let tunnel = Tunnel::start(auth, "127.0.0.1".into(), echo_port).expect("tunnel");
    let local_port = tunnel.port;
    let local_host = tunnel.host.clone();
    let mut stream =
        std::net::TcpStream::connect((local_host.as_str(), local_port)).expect("connect tunnel");
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .expect("timeout");
    stream.write_all(b"FORWARDED").expect("send");
    stream
        .shutdown(std::net::Shutdown::Write)
        .expect("half close");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("receive");
    assert_eq!(bytes, b"FORWARDED");
    echo.join().expect("echo thread");
    drop(tunnel);
    wait(|| std::net::TcpStream::connect((local_host.as_str(), local_port)).is_err());
    terminal.stop();
}

#[test]
#[ignore = "Requires Python and Paramiko; loopback fixture only"]
fn modern_host_keys_negotiate_and_bad_password_is_rejected() {
    for profile in ["ed25519", "ecdsa"] {
        let dir = tempfile::tempdir().expect("tempdir");
        let _fixture = Fixture(
            std::process::Command::new("python")
                .args(["scripts/native-ssh-fixture.py"])
                .arg(dir.path())
                .arg(profile)
                .env("PYTHONPATH", ".tools/ssh-test")
                .stdout(std::process::Stdio::null())
                .spawn()
                .expect("fixture"),
        );
        wait(|| dir.path().join("port").exists());
        let mut auth = Auth {
            host: "127.0.0.1".into(),
            port: std::fs::read_to_string(dir.path().join("port"))
                .expect("port")
                .parse()
                .expect("number"),
            user: "fixture".into(),
            password: zeroize::Zeroizing::new("incorrect".into()),
            key: PathBuf::new(),
            fingerprint: std::fs::read_to_string(dir.path().join("fingerprint")).expect("pin"),
            endpoint: None,
        };
        let err = native::connect(&auth)
            .err()
            .expect("wrong password rejected")
            .to_string();
        assert!(!err.contains("encryption keys"), "{profile}: {err}");
        auth.password = zeroize::Zeroizing::new("fixture-secret".into());
        let session = native::connect(&auth).unwrap_or_else(|e| panic!("{profile}: {e}"));
        assert!(session.authenticated());
        assert_eq!(
            session.methods(ssh2::MethodType::HostKey),
            Some(if profile == "ed25519" {
                "ssh-ed25519"
            } else {
                "ecdsa-sha2-nistp256"
            })
        );
    }
}
