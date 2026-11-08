#![allow(clippy::unwrap_used)]
use super::*;
use std::net::TcpListener;
use tungstenite::handshake::server::{Request, Response};
const SECRET: &str = "synthetic:+/=&?";
#[allow(clippy::result_large_err)] // tungstenite callback requires its concrete HTTP error response
fn fixture(
    trusted: bool,
    skip: bool,
    interactive: bool,
) -> (
    Server,
    tempfile::TempDir,
    std::thread::JoinHandle<bool>,
    std::sync::mpsc::Receiver<()>,
) {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
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
    let ca = dir.path().join("ca.pem");
    std::fs::write(&ca, cert.cert.pem()).unwrap();
    let server = Server {
        name: "Fixture".into(),
        url: format!(
            "https://localhost:{}",
            listener.local_addr().unwrap().port()
        ),
        identity: "test@pve".into(),
        ca_file: if trusted { ca } else { Default::default() },
        skip_tls_verification: skip,
        ..Default::default()
    };
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        let start = Instant::now();
        let stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e)
                    if e.kind() == ErrorKind::WouldBlock
                        && start.elapsed() < Duration::from_secs(10) =>
                {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(e) => panic!("fixture accept: {e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let stream = rustls::StreamOwned::new(
            rustls::ServerConnection::new(Arc::new(tls)).unwrap(),
            stream,
        );
        let ws = tungstenite::accept_hdr(stream, |request: &Request, mut response: Response| {
            assert_eq!(
                request.headers()["Cookie"],
                "PVEAuthCookie=synthetic-cookie"
            );
            assert_eq!(request.headers()["Sec-WebSocket-Protocol"], "binary");
            let url = url::Url::parse(&format!("https://localhost{}", request.uri())).unwrap();
            assert_eq!(url.path(), "/api2/json/nodes/new-node/lxc/100/vncwebsocket");
            assert!(
                url.query_pairs()
                    .any(|(k, v)| k == "vncticket" && v == SECRET)
            );
            response
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", "binary".parse().unwrap());
            Ok(response)
        });
        let Ok(mut ws) = ws else { return false };
        assert_eq!(
            ws.read().unwrap().into_text().unwrap(),
            format!("test@pve:{SECRET}\n")
        );
        ws.send(Message::Binary(b"O".to_vec().into())).unwrap();
        ws.send(Message::Binary(b"Kwelcome".to_vec().into()))
            .unwrap();
        if interactive {
            assert_eq!(
                ws.read().unwrap().into_data().as_ref(),
                input_packet("é".as_bytes())
            );
            ws.send(Message::Binary(b"echo".to_vec().into())).unwrap();
            assert_eq!(ws.read().unwrap().into_text().unwrap(), "1:120:40:");
            ws.send(Message::Binary(b"resized".to_vec().into()))
                .unwrap();
            ready_tx.send(()).unwrap();
            assert!(matches!(ws.read().unwrap(), Message::Close(_)));
        }
        true
    });
    (server, dir, thread, ready_rx)
}
fn open(server: &Server) -> Result<Console> {
    let endpoint = url::Url::parse(&format!(
        "{}/api2/json/nodes/new-node/lxc/100/vncwebsocket",
        server.url
    ))
    .unwrap();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Cookie", "PVEAuthCookie=synthetic-cookie".parse().unwrap());
    connect(
        server,
        endpoint,
        headers,
        serde_json::from_value(
            serde_json::json!({"user":"test@pve","ticket":SECRET,"port":"5900"}),
        )
        .unwrap(),
    )
}
#[test]
fn websocket_console_auth_input_resize_and_cancel_with_full_ui_queue() {
    let (server, _dir, thread, ready) = fixture(true, false, true);
    let console = open(&server).unwrap();
    let (input, rx) = std::sync::mpsc::channel();
    let (send, output) = std::sync::mpsc::sync_channel(1);
    let (resize, sizes) = std::sync::mpsc::channel();
    let stopped = Arc::new(AtomicBool::new(false));
    let stop = stopped.clone();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    input.send("é".as_bytes().to_vec()).unwrap();
    resize.send((40, 120)).unwrap();
    let worker = std::thread::spawn(move || {
        console.run(rx, send, sizes, stop, Default::default());
        done_tx.send(()).unwrap();
    });
    ready.recv_timeout(Duration::from_secs(3)).unwrap();
    stopped.store(true, Ordering::Release);
    done_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    worker.join().unwrap();
    assert_eq!(output.recv().unwrap(), b"welcome");
    assert!(thread.join().unwrap());
}
#[test]
fn websocket_tls_rejects_unknown_ca_before_auth_and_exception_is_profile_scoped() {
    for skip in [false, true] {
        let (server, _dir, thread, _ready) = fixture(false, skip, false);
        let result = open(&server);
        if skip {
            assert!(result.is_ok());
        } else {
            let error = result.as_ref().err().unwrap().to_string();
            assert!(!error.contains(SECRET));
            assert!(!error.contains("synthetic-cookie"));
        }
        drop(result);
        assert_eq!(thread.join().unwrap(), skip);
    }
}

#[test]
fn termproxy_accepts_integer_and_string_ports_but_rejects_invalid_values() {
    for value in [serde_json::json!(5900), serde_json::json!("5900")] {
        let ticket: Ticket = serde_json::from_value(
            serde_json::json!({"user":"test@pve","ticket":"synthetic-secret","port":value}),
        )
        .unwrap();
        assert_eq!(ticket.port, 5900);
    }
    for value in [
        serde_json::json!(null),
        serde_json::json!(true),
        serde_json::json!(-1),
        serde_json::json!(0),
        serde_json::json!(65536),
        serde_json::json!("invalid"),
        serde_json::json!(5900.5),
    ] {
        assert!(
            serde_json::from_value::<Ticket>(
                serde_json::json!({"user":"test@pve","ticket":"synthetic-secret","port":value})
            )
            .is_err()
        );
    }
}
