#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    sync::Arc,
};

fn profile() -> Server {
    Server {
        name: "Lab".into(),
        url: "https://localhost:8006".into(),
        identity: "tester@pve!viewer".into(),
        auth: AuthMode::ApiToken,
        ..Default::default()
    }
}
fn configuration(ca: &str) -> serde_json::Value {
    json!({"type":"spice", "host":"pvespiceproxy:synthetic-ticket", "proxy":"http://localhost:3128", "tls-port":"61000", "host-subject":"CN=pve", "ca":ca.replace('\n',"\\n"), "password":"synthetic-password", "disable-effects":"all", "command":"untrusted-command", "tls-ciphers":"insecure"})
}

#[test]
fn profiles_reject_unsafe_endpoints_and_serialize_only_metadata() {
    let mut p = profile();
    assert!(p.validate().is_ok());
    for url in [
        "http://localhost:8006",
        "https://user:secret@localhost",
        "https://localhost/api",
        "https://localhost?secret=1",
        "https://localhost/#fragment",
    ] {
        p.url = url.into();
        assert!(p.validate().is_err());
    }
    p.url = "https://localhost:8006".into();
    for proxy in [
        "localhost:3128",
        "https://localhost",
        "host\npassword=bad",
        "../node",
    ] {
        p.proxy_host = proxy.into();
        assert!(p.validate().is_err());
    }
    p.proxy_host.clear();
    let value = serde_json::to_value(&p).unwrap();
    assert!(value.get("password").is_none());
    assert!(value.get("secret").is_none());
    p.identity = "tester@pve!viewer\r\nInjected: value".into();
    assert!(p.validate().is_err());
}

#[test]
fn vm_inventory_includes_containers_and_checks_node_paths() {
    let vms = parse_vms(json!([
        {"type":"node","node":"pve"}, {"type":"lxc","vmid":100,"node":"pve1","name":"Zcontainer","status":"running"},
        {"type":"qemu","vmid":102,"node":"pve2","name":"Zebra","status":"stopped"},
        {"type":"qemu","vmid":101,"node":"pve1","name":"Alpha","status":"running"}
    ]))
    .unwrap();
    assert_eq!(vms.len(), 3);
    assert_eq!(vms[0].vmid, 101);
    assert!(vms[0].connectable());
    assert!(!vms.iter().find(|v| v.vmid == 102).unwrap().connectable());
    assert_eq!(
        vms.iter().find(|v| v.vmid == 100).unwrap().kind,
        VmKind::Lxc
    );
    assert!(
        parse_vms(json!([{"type":"qemu","vmid":101,"node":"../access","status":"running"}]))
            .is_err()
    );
}

#[test]
fn spice_files_require_tls_and_cannot_inject_viewer_options() {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let original = configuration(&cert.cert.pem());
    let config: SpiceConfig = serde_json::from_value(original.clone()).unwrap();
    let text = config.file_text().unwrap();
    assert!(!text.lines().any(|line| line.starts_with("port=")));
    assert!(text.contains("\nsecure-channels=all\ntls-port=61000\nhost-subject=CN=pve\n"));
    assert!(text.contains("\npassword=synthetic-password\ndelete-this-file=1\n"));
    assert!(!text.contains("untrusted-command"));
    assert!(!text.contains("tls-ciphers"));
    for field in ["host", "password", "host-subject", "proxy"] {
        let mut bad = original.clone();
        bad[field] = json!("value\ndisable-channels=main");
        assert!(
            serde_json::from_value::<SpiceConfig>(bad)
                .unwrap()
                .file_text()
                .is_err()
        );
    }
    let mut bad = original.clone();
    bad["ca"] = json!("not a certificate");
    assert!(
        serde_json::from_value::<SpiceConfig>(bad)
            .unwrap()
            .file_text()
            .is_err()
    );
    let mut bad = original;
    bad["tls-port"] = json!(0);
    assert!(serde_json::from_value::<SpiceConfig>(bad).is_err());
}

#[test]
fn spice_proxy_diagnostic_never_sends_or_exposes_tickets() {
    use std::io::Read;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let mut data = configuration(&cert.cert.pem());
    data["proxy"] = json!(format!("http://{address}"));
    let config: SpiceConfig = serde_json::from_value(data).unwrap();
    config.check_proxy().unwrap();
    let (mut socket, _) = listener.accept().unwrap();
    let mut bytes = [0; 32];
    assert_eq!(socket.read(&mut bytes).unwrap(), 0);
    drop(socket);
    drop(listener);
    let error = config.check_proxy().unwrap_err().to_string();
    assert!(error.contains("inaccessible"));
    assert!(!error.contains("synthetic-ticket"));
    assert!(!error.contains("synthetic-password"));
}

// Real local TLS transport, synthetic credentials only. Each response closes its
// connection, exercising clients that cannot rely on connection reuse.
struct Mock {
    server: Server,
    ca: String,
    _dir: tempfile::TempDir,
    thread: std::thread::JoinHandle<Vec<String>>,
}
impl Mock {
    fn new(replies: Vec<(u16, serde_json::Value)>) -> Self {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = cert.cert.pem();
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.cert.der().clone()],
            rustls::pki_types::PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into(),
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ca.pem");
        std::fs::write(&path, &ca).unwrap();
        let mut server = profile();
        server.url = format!(
            "https://localhost:{}",
            listener.local_addr().unwrap().port()
        );
        server.ca_file = path;
        let thread = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for (code, body) in replies {
                let deadline = std::time::Instant::now() + Duration::from_secs(15);
                let stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && std::time::Instant::now() < deadline =>
                        {
                            std::thread::sleep(Duration::from_millis(5))
                        }
                        Err(e) => panic!("mock accept: {e}"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                let mut tls = rustls::StreamOwned::new(
                    rustls::ServerConnection::new(Arc::new(tls.clone())).unwrap(),
                    stream,
                );
                let mut reader = BufReader::new(&mut tls);
                let mut headers = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_err() {
                        return requests;
                    }
                    if line == "\r\n" {
                        break;
                    }
                    assert!(!line.is_empty());
                    headers.push_str(&line);
                }
                let size = headers
                    .lines()
                    .find_map(|s| {
                        s.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|s| s.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                let mut body_bytes = vec![0; size];
                reader.read_exact(&mut body_bytes).unwrap();
                drop(reader);
                headers.push_str(&String::from_utf8(body_bytes).unwrap());
                requests.push(headers);
                let body = body.to_string();
                write!(tls,"HTTP/1.1 {code} Test\r\nContent-Type: application/json\r\nLocation: https://localhost:1/forbidden\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                tls.flush().unwrap();
            }
            requests
        });
        Self {
            server,
            ca,
            _dir: dir,
            thread,
        }
    }
}

#[test]
fn https_password_authentication_sends_cookie_and_csrf() {
    let mut mock = Mock::new(vec![
        (
            200,
            json!({"data":{"ticket":"synthetic-cookie","CSRFPreventionToken":"synthetic-csrf"}}),
        ),
        (200, json!({"data":[]})),
    ]);
    mock.server.auth = AuthMode::Password;
    mock.server.identity = "tester@pve".into();
    let api = Api::login(&mock.server, "synthetic-secret&value").unwrap();
    assert!(api.vms().unwrap().is_empty());
    let requests = mock.thread.join().unwrap();
    assert!(requests[0].starts_with("POST /api2/json/access/ticket "));
    assert!(requests[0].contains("password=synthetic-secret%26value"));
    assert!(requests[1].contains("PVEAuthCookie=synthetic-cookie"));
    assert!(
        requests[1]
            .to_lowercase()
            .contains("csrfpreventiontoken: synthetic-csrf")
    );
}

#[test]
fn https_token_auth_refreshes_migrated_node_and_fetches_new_console_ticket() {
    let rows = json!({"data":[{"type":"qemu","vmid":100,"node":"new-node","status":"running","name":"Test VM"}]});
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let replies = (0..2)
        .flat_map(|_| {
            [
                (200, rows.clone()),
                (200, json!({"data":{"spice":1}})),
                (200, json!({"data":configuration(&cert.cert.pem())})),
            ]
        })
        .collect();
    let mock = Mock::new(replies);
    assert!(mock.ca.contains("BEGIN CERTIFICATE"));
    let api = Api::login(&mock.server, "synthetic-token").unwrap();
    for _ in 0..2 {
        assert!(api.console(&mock.server, 100).unwrap().file_text().is_ok());
    }
    let requests = mock.thread.join().unwrap();
    assert_eq!(requests.len(), 6);
    for request in &requests {
        assert!(request.contains("PVEAPIToken=tester@pve!viewer=synthetic-token"));
    }
    for i in [2, 5] {
        assert!(requests[i].starts_with("POST /api2/json/nodes/new-node/qemu/100/spiceproxy "));
        assert!(requests[i].ends_with("proxy=localhost"));
    }
}

#[test]
fn api_errors_never_expose_response_secrets_or_follow_redirects() {
    for status in [401, 403, 302] {
        let mock = Mock::new(vec![(status, json!({"data":"DO-NOT-EXPOSE"}))]);
        let api = Api::login(&mock.server, "synthetic-token").unwrap();
        let error = api.vms().unwrap_err().to_string();
        assert!(!error.contains("DO-NOT-EXPOSE"));
        assert!(!error.contains("synthetic-token"));
        assert_eq!(mock.thread.join().unwrap().len(), 1);
    }
}

#[test]
fn https_rejects_untrusted_certificates_and_wrong_hostnames_before_authentication() {
    for wrong_name in [false, true] {
        let mut mock = Mock::new(vec![(200, json!({"data":[]}))]);
        if wrong_name {
            mock.server.url = mock.server.url.replace("localhost", "127.0.0.1");
        } else {
            mock.server.ca_file.clear();
        }
        let api = Api::login(&mock.server, "synthetic-token").unwrap();
        assert!(api.vms().is_err());
        assert!(mock.thread.join().unwrap().is_empty());
    }
}

#[test]
fn tls_exception_is_opt_in_persisted_and_scoped_to_one_server() {
    let mut original = serde_json::to_value(profile()).unwrap();
    original
        .as_object_mut()
        .unwrap()
        .remove("skip_tls_verification");
    let restored: Server = serde_json::from_value(original).unwrap();
    assert!(!restored.skip_tls_verification);
    let mut mock = Mock::new(vec![(200, json!({"data":[]}))]);
    // Exercise both an unknown CA and an incorrect certificate hostname.
    mock.server.url = mock.server.url.replace("localhost", "127.0.0.1");
    mock.server.ca_file = mock._dir.path().join("missing-ca.pem");
    mock.server.skip_tls_verification = true;
    let restored: Server =
        serde_json::from_value(serde_json::to_value(&mock.server).unwrap()).unwrap();
    assert!(restored.skip_tls_verification);
    assert!(
        Api::login(&restored, "synthetic-token")
            .unwrap()
            .vms()
            .unwrap()
            .is_empty()
    );
    assert_eq!(mock.thread.join().unwrap().len(), 1);
    // Creating the relaxed client must not change another server's TLS policy.
    let mut strict = Mock::new(vec![(200, json!({"data":[]}))]);
    strict.server.ca_file.clear();
    assert!(
        Api::login(&strict.server, "synthetic-token")
            .unwrap()
            .vms()
            .is_err()
    );
    assert!(strict.thread.join().unwrap().is_empty());
}

#[test]
fn authentication_diagnostics_distinguish_login_ticket_and_api_token_without_secrets() {
    let mut login = Mock::new(vec![(401, json!({"data":"DO-NOT-EXPOSE"}))]);
    login.server.auth = AuthMode::Password;
    login.server.identity = "root@pam".into();
    let error = Api::login(&login.server, "synthetic-password")
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("POST /access/ticket"));
    assert!(error.contains("Connexion refusée"));
    assert!(!error.contains("DO-NOT-EXPOSE"));
    assert!(!error.contains("synthetic-password"));
    let requests = login.thread.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("username=root%40pam"));
    let mut ticket = Mock::new(vec![
        (
            200,
            json!({"data":{"ticket":"synthetic-cookie","CSRFPreventionToken":"synthetic-csrf"}}),
        ),
        (401, json!({"data":"DO-NOT-EXPOSE"})),
    ]);
    ticket.server.auth = AuthMode::Password;
    ticket.server.identity = "root@pam".into();
    let error = Api::login(&ticket.server, "synthetic-password")
        .unwrap()
        .vms()
        .unwrap_err()
        .to_string();
    assert!(error.contains("GET /cluster/resources"));
    assert!(error.contains("Ticket refusé"));
    assert!(!error.contains("synthetic-cookie"));
    assert_eq!(ticket.thread.join().unwrap().len(), 2);
    let token = Mock::new(vec![(401, json!({"data":"DO-NOT-EXPOSE"}))]);
    let error = Api::login(&token.server, "synthetic-token")
        .unwrap()
        .vms()
        .unwrap_err()
        .to_string();
    assert!(error.contains("Jeton API refusé"));
    assert!(!error.contains("synthetic-token"));
    assert_eq!(token.thread.join().unwrap().len(), 1);
}

#[test]
fn lxc_addresses_exclude_local_and_invalid_networks() {
    let values = vec![
        json!({"inet":"127.0.0.1/8","inet6":"::1/128"}),
        json!({"inet":"192.168.1.5/24","inet6":"fe80::1/64"}),
        json!({"inet":"192.168.1.5/24","inet6":"2001:db8::1/64"}),
        json!({"inet":"host;bad","inet6":"::/128"}),
    ];
    assert_eq!(
        interface_addresses(&values),
        vec!["192.168.1.5", "2001:db8::1"]
    );
}
#[test]
fn power_actions_reject_templates_unknown_states_and_wrong_transitions() {
    let mut vm: Vm =
        serde_json::from_value(json!({"vmid":1,"node":"pve","status":"stopped"})).unwrap();
    assert!(PowerAction::Start.available(&vm));
    assert!(!PowerAction::PowerOff.available(&vm));
    vm.status = "unknown".into();
    for action in [
        PowerAction::Start,
        PowerAction::Shutdown,
        PowerAction::Reboot,
        PowerAction::PowerOff,
    ] {
        assert!(!action.available(&vm));
    }
    vm.status = "running".into();
    assert!(PowerAction::Shutdown.available(&vm));
    vm.template = 1;
    assert!(!PowerAction::Shutdown.available(&vm));
}

#[test]
fn power_action_uses_current_node_and_tracks_task_without_forced_fallback() {
    let selected: Vm =
        serde_json::from_value(json!({"vmid":100,"node":"old","status":"running"})).unwrap();
    let task = "UPID:new-node:00000001:00000002:00000003:qmshutdown:100:test@pve:";
    let mock = Mock::new(vec![
        (
            200,
            json!({"data":[{"type":"qemu","vmid":100,"node":"new-node","status":"running"}]}),
        ),
        (200, json!({"data":task})),
        (
            200,
            json!({"data":{"status":"stopped","exitstatus":"ERROR synthetic-private-data"}}),
        ),
    ]);
    let api = Api::login(&mock.server, "synthetic-token").unwrap();
    let accepted = api.power(&selected, PowerAction::Shutdown).unwrap();
    let err = api.wait_power_task(&accepted).unwrap_err().to_string();
    assert!(!err.contains("synthetic-private-data"));
    let requests = mock.thread.join().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests[1].starts_with("POST /api2/json/nodes/new-node/qemu/100/status/shutdown "));
    assert!(
        !requests
            .iter()
            .any(|s| s.contains("forceStop") || s.contains("/status/stop "))
    );
}
#[test]
fn lxc_discovery_and_power_use_container_routes() {
    let selected: Vm =
        serde_json::from_value(json!({"kind":"Lxc","vmid":100,"node":"old","status":"running"}))
            .unwrap();
    let rows = json!({"data":[{"type":"lxc","vmid":100,"node":"new-node","status":"running"}]});
    let task = "UPID:new-node:00000001:00000002:00000003:vzstop:100:test@pve:";
    let mock = Mock::new(vec![
        (200, rows.clone()),
        (
            200,
            json!({"data":[{"inet":"10.2.3.4/24","inet6":"fe80::1/64"}]}),
        ),
        (200, rows),
        (200, json!({"data":task})),
        (200, json!({"data":{"status":"stopped","exitstatus":"OK"}})),
    ]);
    let api = Api::login(&mock.server, "synthetic-token").unwrap();
    assert_eq!(api.lxc_addresses(&selected).unwrap(), vec!["10.2.3.4"]);
    let task = api.power(&selected, PowerAction::PowerOff).unwrap();
    api.wait_power_task(&task).unwrap();
    let requests = mock.thread.join().unwrap();
    assert!(requests[1].starts_with("GET /api2/json/nodes/new-node/lxc/100/interfaces "));
    assert!(requests[3].starts_with("POST /api2/json/nodes/new-node/lxc/100/status/stop "));
}
#[test]
fn stale_or_template_machine_is_rejected_before_power_post() {
    let selected: Vm =
        serde_json::from_value(json!({"vmid":100,"node":"old","status":"running"})).unwrap();
    for row in [
        json!({"type":"qemu","vmid":100,"node":"new-node","status":"stopped"}),
        json!({"type":"qemu","vmid":100,"node":"new-node","status":"running","template":1}),
    ] {
        let mock = Mock::new(vec![(200, json!({"data":[row]}))]);
        let api = Api::login(&mock.server, "synthetic-token").unwrap();
        assert!(api.power(&selected, PowerAction::PowerOff).is_err());
        assert_eq!(mock.thread.join().unwrap().len(), 1);
    }
}

#[test]
fn lxc_terminal_requests_ticket_on_current_node_and_rejects_invalid_reply() {
    let selected: Vm = serde_json::from_value(
        json!({"kind":"Lxc","vmid":100,"node":"old-node","status":"running"}),
    )
    .unwrap();
    let mock = Mock::new(vec![
        (
            200,
            json!({"data":[{"type":"lxc","vmid":100,"node":"new-node","status":"running"}]}),
        ),
        (
            200,
            json!({"data":{"user":"test@pve","ticket":"private-do-not-print","port":0}}),
        ),
    ]);
    let api = Api::login(&mock.server, "synthetic-token").unwrap();
    let error = api
        .lxc_terminal(&mock.server, &selected)
        .err()
        .expect("invalid ticket")
        .to_string();
    assert!(!error.contains("private-do-not-print"));
    let requests = mock.thread.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].starts_with("POST /api2/json/nodes/new-node/lxc/100/termproxy "));
    assert!(requests[1].contains("PVEAPIToken=tester@pve!viewer=synthetic-token"));
}
