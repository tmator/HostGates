use eframe::egui::{Key, Modifiers};
use hostgates::{
    connections::{rdp, ssh_arguments},
    error::Result,
    importers::mobaxterm::parse,
    ui::terminal::key_bytes,
};
fn session() -> hostgates::models::SessionInput {
    parse(b"[Bookmarks]\ns=#109#0%host.example.org%2222%user")
        .rows
        .remove(0)
        .data
        .expect("session")
}
#[test]
fn ssh_strict_host_key_and_separate_arguments() -> Result<()> {
    let s = session();
    let args = ssh_arguments(&s)?;
    assert!(args.contains(&"StrictHostKeyChecking=ask".into()));
    assert_eq!(&args[args.len() - 2..], ["--", "host.example.org"]);
    assert!(!args.iter().any(|a| a.contains("password")));
    Ok(())
}
#[test]
fn malicious_host_is_rejected() {
    let mut s = session();
    s.hostname = "-oProxyCommand=bad".into();
    assert!(ssh_arguments(&s).is_err());
    assert!(rdp::arguments(&s, true).is_err());
}
#[test]
fn rdp_has_no_secret_or_cert_bypass() -> Result<()> {
    let mut s = session();
    s.hostname = "2001:db8::1".into();
    s.port = 3389;
    for windows in [false, true] {
        let args = rdp::arguments(&s, windows)?;
        assert!(args.contains(&"/v:[2001:db8::1]:3389".into()));
        assert!(
            !args
                .iter()
                .any(|a| a.starts_with("/p:") || a.contains("ignore"))
        );
    }
    Ok(())
}
#[test]
fn terminal_control_and_function_keys() {
    assert_eq!(key_bytes(Key::C, Modifiers::CTRL, false), Some(vec![3]));
    assert_eq!(key_bytes(Key::D, Modifiers::CTRL, false), Some(vec![4]));
    assert_eq!(
        key_bytes(Key::ArrowUp, Modifiers::NONE, true),
        Some(b"\x1bOA".to_vec())
    );
    assert_eq!(
        key_bytes(Key::F12, Modifiers::NONE, false),
        Some(b"\x1b[24~".to_vec())
    );
}
#[test]
fn terminal_utf8_ansi_and_resize() {
    let mut p = vt100::Parser::new(24, 80, 100);
    p.process("\x1b[31mBonjour été\x1b[0m".as_bytes());
    assert!(p.screen().contents().contains("Bonjour été"));
    assert_eq!(
        p.screen().cell(0, 0).expect("cell").fgcolor(),
        vt100::Color::Idx(1)
    );
    p.screen_mut().set_size(30, 100);
    assert_eq!(p.screen().size(), (30, 100));
}
#[test]
fn native_pty_output_and_exit() -> Result<()> {
    #[cfg(windows)]
    let program = hostgates::connections::find_program(&["powershell"]).expect("powershell");
    #[cfg(windows)]
    let args = vec![
        "-NoLogo".into(),
        "-NoProfile".into(),
        "-Command".into(),
        "Write-Output HOSTGATES_PTY_OK".into(),
    ];
    #[cfg(not(windows))]
    let program = std::path::PathBuf::from("/bin/echo");
    #[cfg(not(windows))]
    let args = vec!["HOSTGATES_PTY_OK".into()];
    let mut terminal = hostgates::connections::terminal::Terminal::launch(
        &program,
        &args,
        eframe::egui::Context::default(),
    )?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        terminal.poll();
        if terminal.closed
            && terminal
                .parser
                .screen()
                .contents()
                .contains("HOSTGATES_PTY_OK")
        {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("PTY did not produce expected output and exit before timeout")
}
