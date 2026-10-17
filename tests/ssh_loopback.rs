use hostgates::{
    connections::{ssh_arguments, terminal::Terminal},
    error::Result,
    importers::mobaxterm::parse,
};
fn wait_for(t: &mut Terminal, needle: &str) -> bool {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while std::time::Instant::now() < until {
        t.poll();
        if t.parser.screen().contents().contains(needle) {
            return true;
        }
        if t.closed {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    false
}
#[test]
#[ignore = "Requires scripts/ssh-test-server.py; loopback only"]
fn openssh_password_and_changed_host_key() -> Result<()> {
    let fixture = std::path::PathBuf::from(
        std::env::var_os("HOSTGATES_SSH_FIXTURE").expect("fixture directory"),
    );
    let port = std::fs::read_to_string(fixture.join("port"))
        .expect("port")
        .parse::<u16>()
        .expect("port number");
    let mut session = parse(b"[Bookmarks]\ns=#109#0%127.0.0.1%22%hostgates-test")
        .rows
        .remove(0)
        .data
        .expect("session");
    session.port = port;
    let program = hostgates::connections::find_program(&["ssh"]).expect("OpenSSH");
    let options = |known: &std::path::Path| {
        let mut args = ssh_arguments(&session).expect("args");
        let at = args.len() - 2;
        args.splice(
            at..at,
            [
                "-F".into(),
                if cfg!(windows) {
                    "NUL".into()
                } else {
                    "/dev/null".into()
                },
                "-o".into(),
                format!(
                    "UserKnownHostsFile=\"{}\"",
                    known.display().to_string().replace('\\', "/")
                ),
                "-o".into(),
                "GlobalKnownHostsFile=none".into(),
            ],
        );
        args
    };
    let known = fixture.join("accepted_known_hosts");
    let mut t = Terminal::launch(&program, &options(&known), eframe::egui::Context::default())?;
    assert!(
        wait_for(&mut t, "Are you sure"),
        "Expected an explicit unknown host prompt"
    );
    t.write(b"yes\r".to_vec());
    assert!(wait_for(&mut t, "password:"), "Expected password prompt");
    t.write(b"test-fixture-only\r".to_vec());
    assert!(
        wait_for(&mut t, "HOSTGATES_SSH_OK"),
        "Authenticated shell not reached"
    );
    assert!(t.parser.screen().contents().contains("été"));
    t.resize(32, 110);
    t.write(b"hostgates-check\r".to_vec());
    assert!(wait_for(&mut t, "HOSTGATES_INPUT_OK"));
    drop(t);
    let mut t = Terminal::launch(
        &program,
        &options(&fixture.join("wrong_known_hosts")),
        eframe::egui::Context::default(),
    )?;
    assert!(
        wait_for(&mut t, "REMOTE HOST IDENTIFICATION HAS CHANGED"),
        "Changed host key must be rejected"
    );
    assert!(!t.parser.screen().contents().contains("HOSTGATES_SSH_OK"));
    Ok(())
}
