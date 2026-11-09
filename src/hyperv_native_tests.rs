use super::*;
use serde_json::json;

fn profile() -> Server {
    Server {
        hyperv: true,
        cluster: true,
        name: "Test".into(),
        url: "https://node1.example.test:5986".into(),
        identity: "LAB\\test".into(),
        ..Default::default()
    }
}
fn inventory(node: &str) -> Vec<u8> {
    serde_json::to_vec(&if node.starts_with("node2") {
        json!([{"vmid":0,"native_id":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa","kind":"HyperV","node":"ignored","name":"VM ' $unsafe","status":"running"}])
    } else { json!([]) }).expect("native WinRM test fixture")
}
#[test]
fn native_cluster_routes_console_and_actions_to_current_owner_without_retry() {
    for action in ["console", "start", "shutdown", "reboot", "stop"] {
        let mut mutations = 0;
        let result = run_with(
            &profile(),
            action,
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            |node, script| {
                if script == CLUSTER {
                    return Ok(br#"["node1.example.test","node2.example.test"]"#.to_vec());
                }
                if script == INVENTORY {
                    return Ok(inventory(node));
                }
                mutations += 1;
                assert_eq!(node, "node2.example.test");
                assert!(script.contains(&format!("$action='{action}'")));
                assert!(!script.contains("$unsafe"));
                Err(failure("synthetic failed action"))
            },
        );
        if action == "console" {
            assert_eq!(
                result.expect("native WinRM test fixture")[0].node,
                "node2.example.test"
            );
            assert_eq!(mutations, 0);
        } else {
            assert!(result.is_err());
            assert_eq!(mutations, 1);
        }
    }
}
#[test]
fn native_rejects_invalid_ids_actions_and_cluster_nodes_before_mutation() {
    for (action, id) in [("delete", ""), ("stop", "'; Stop-Computer")] {
        assert!(run_with(&profile(), action, id, |_, _| panic!("must not connect")).is_err());
    }
    for nodes in [json!([]), json!(["node1", "NODE1"]), json!(["bad/path"])] {
        let mut calls = 0;
        assert!(
            run_with(&profile(), "list", "", |_, _| {
                calls += 1;
                Ok(serde_json::to_vec(&nodes).expect("native WinRM test fixture"))
            })
            .is_err()
        );
        assert_eq!(calls, 1);
    }
}
#[test]
fn native_errors_never_display_server_text_or_secrets() {
    let err = remote_error(WinrmError::AuthFailed("synthetic-secret response".into()));
    assert!(!err.to_string().contains("synthetic-secret"));
    let c = credentials("LAB\\alice", "synthetic-secret");
    assert_eq!(c.username, "alice");
    assert_eq!(c.domain, "LAB");
    assert!(!format!("{c:?}").contains("synthetic-secret"));
    let c = credentials("alice@example.test", "synthetic-secret");
    assert_eq!(c.domain, "example.test");
}
#[test]
fn native_client_refuses_plaintext_and_invalid_ca() {
    let mut server = profile();
    server.url = "http://node:5985".into();
    assert!(client(&server, "synthetic-secret").is_err());
    assert!(run(&server, "synthetic-secret", "list", "").is_err());
    server.url = "https://node:5986".into();
    server.ca_file = "missing-ca.pem".into();
    assert!(client(&server, "synthetic-secret").is_err());
}

#[test]
fn native_https_checks_ca_before_auth_and_refuses_redirects() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
    };
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])
        .expect("native WinRM test fixture");
    let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("native WinRM test fixture")
    .with_no_client_auth()
    .with_single_cert(
        vec![cert.cert.der().clone()],
        rustls::pki_types::PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into(),
    )
    .expect("native WinRM test fixture");
    let listener = TcpListener::bind("127.0.0.1:0").expect("native WinRM test fixture");
    let port = listener
        .local_addr()
        .expect("native WinRM test fixture")
        .port();
    listener
        .set_nonblocking(true)
        .expect("native WinRM test fixture");
    let stop = Arc::new(AtomicBool::new(false));
    let requests = Arc::new(AtomicUsize::new(0));
    let worker_stop = stop.clone();
    let worker_requests = requests.clone();
    let thread = std::thread::spawn(move || {
        let tls = Arc::new(tls);
        while !worker_stop.load(Ordering::Relaxed) {
            let Ok((socket, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            socket
                .set_nonblocking(false)
                .expect("native WinRM test fixture");
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("native WinRM test fixture");
            let mut stream = rustls::StreamOwned::new(
                rustls::ServerConnection::new(tls.clone()).expect("native WinRM test fixture"),
                socket,
            );
            let mut header = Vec::new();
            let mut byte = [0];
            while header.len() < 65536 && !header.ends_with(b"\r\n\r\n") {
                match stream.read(&mut byte) {
                    Ok(1) => header.push(byte[0]),
                    _ => break,
                }
            }
            if !header.ends_with(b"\r\n\r\n") {
                continue;
            }
            worker_requests.fetch_add(1, Ordering::Relaxed);
            let text = String::from_utf8_lossy(&header);
            let len = text
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|s| s.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            if len > 1_000_000 {
                continue;
            }
            let mut body = vec![0; len];
            let _ = stream.read_exact(&mut body);
            let _=stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: https://forbidden.invalid/wsman\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            let _ = stream.flush();
        }
    });
    let dir = tempfile::tempdir().expect("native WinRM test fixture");
    let ca = dir.path().join("ca.pem");
    std::fs::write(&ca, cert.cert.pem()).expect("native WinRM test fixture");
    let mut server = profile();
    server.cluster = false;
    server.url = format!("https://localhost:{port}");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("native WinRM test fixture");
    let untrusted = client(&server, "synthetic-secret").expect("native WinRM test fixture");
    assert!(
        runtime
            .block_on(untrusted.create_shell("localhost"))
            .is_err()
    );
    assert_eq!(requests.load(Ordering::Relaxed), 0);
    server.ca_file = ca;
    let trusted = client(&server, "synthetic-secret").expect("native WinRM test fixture");
    let result = runtime.block_on(trusted.create_shell("localhost"));
    assert!(result.is_err());
    stop.store(true, Ordering::Relaxed);
    thread.join().expect("native WinRM test fixture");
    assert_eq!(requests.load(Ordering::Relaxed), 1);
}
