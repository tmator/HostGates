//! Proxmox termproxy wire protocol; no SSH, shell command or browser credential file.
use crate::{
    credentials::Secret,
    error::{AppError, Result},
    proxmox::Server,
};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime, pem::PemObject},
};
use std::{
    io::{ErrorKind, Read},
    net::{TcpStream, ToSocketAddrs},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError},
    },
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket, client::IntoClientRequest, stream::MaybeTlsStream};
use zeroize::Zeroizing;
type Socket = WebSocket<MaybeTlsStream<TcpStream>>;
fn failure(message: &str) -> AppError {
    AppError::Operation(message.into())
}
#[derive(serde::Deserialize)]
pub(crate) struct Ticket {
    pub user: String,
    pub ticket: Secret,
    #[serde(deserialize_with = "crate::proxmox::port_from_json")]
    pub port: u16,
}
pub(crate) struct Console {
    socket: Socket,
    initial: Vec<u8>,
}
#[derive(Debug)]
struct ExplicitCertificateException;
impl ServerCertVerifier for ExplicitCertificateException {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            signature,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            signature,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
fn tls_config(server: &Server) -> Result<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(cert);
    }
    if !server.skip_tls_verification && !server.ca_file.as_os_str().is_empty() {
        let mut pem = Vec::new();
        std::fs::File::open(&server.ca_file)
            .and_then(|f| f.take(262_145).read_to_end(&mut pem))
            .map_err(|_| failure("Console Proxmox : certificat CA illisible."))?;
        if pem.len() > 262_144 {
            return Err(failure("Console Proxmox : certificat CA trop volumineux."));
        }
        let mut count = 0;
        for cert in CertificateDer::pem_slice_iter(&pem) {
            roots
                .add(cert.map_err(|_| failure("Console Proxmox : certificat CA invalide."))?)
                .map_err(|_| failure("Console Proxmox : certificat CA invalide."))?;
            count += 1;
        }
        if count == 0 {
            return Err(failure("Console Proxmox : certificat CA manquant."));
        }
    }
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|_| failure("Console Proxmox : TLS indisponible."))?
    .with_root_certificates(roots)
    .with_no_client_auth();
    if server.skip_tls_verification {
        config
            .dangerous()
            .set_certificate_verifier(Arc::new(ExplicitCertificateException));
    }
    Ok(config)
}
fn tcp(socket: &Socket) -> Result<&TcpStream> {
    match socket.get_ref() {
        MaybeTlsStream::Rustls(stream) => Ok(&stream.sock),
        _ => Err(failure("Console Proxmox : connexion chiffrée requise.")),
    }
}
fn idle(error: &tungstenite::Error) -> bool {
    matches!(error,tungstenite::Error::Io(e) if matches!(e.kind(),ErrorKind::WouldBlock|ErrorKind::TimedOut))
}
fn message_bytes(message: Message) -> Option<Vec<u8>> {
    match message {
        Message::Binary(data) => Some(data.to_vec()),
        Message::Text(data) => Some(data.as_bytes().to_vec()),
        _ => None,
    }
}
pub(crate) fn connect(
    server: &Server,
    mut endpoint: url::Url,
    headers: reqwest::header::HeaderMap,
    ticket: Ticket,
) -> Result<Console> {
    let base = server.validate()?;
    if endpoint.origin() != base.origin()
        || endpoint.scheme() != "https"
        || ticket.user.is_empty()
        || ticket.user.len() > 256
        || ticket.user.contains([':', '\r', '\n', '\0'])
        || ticket.ticket.is_empty()
        || ticket.ticket.len() > 16_384
        || ticket.ticket.contains(['\r', '\n', '\0'])
        || !(5900..=5999).contains(&ticket.port)
    {
        return Err(failure("Console Proxmox : ticket ou destination invalide."));
    }
    endpoint
        .set_scheme("wss")
        .map_err(|_| failure("Console Proxmox : URL invalide."))?;
    endpoint
        .query_pairs_mut()
        .append_pair("port", &ticket.port.to_string())
        .append_pair("vncticket", &ticket.ticket);
    let mut request = endpoint
        .as_str()
        .into_client_request()
        .map_err(|_| failure("Console Proxmox : requête invalide."))?;
    for (name, value) in &headers {
        request.headers_mut().insert(name.clone(), value.clone());
    }
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        tungstenite::http::HeaderValue::from_static("binary"),
    );
    let host = endpoint
        .host_str()
        .ok_or_else(|| failure("Console Proxmox : hôte manquant."))?
        .trim_matches(['[', ']']);
    let port = endpoint.port_or_known_default().unwrap_or(443);
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|_| failure("Console Proxmox : résolution DNS impossible."))?;
    let start = Instant::now();
    let mut connected = None;
    for address in addresses.take(16) {
        let Some(remaining) = Duration::from_secs(10).checked_sub(start.elapsed()) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }
        if let Ok(stream) = TcpStream::connect_timeout(&address, remaining) {
            connected = Some(stream);
            break;
        }
    }
    let stream = connected.ok_or_else(|| failure("Console Proxmox : serveur inaccessible."))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|_| failure("Console Proxmox : socket indisponible."))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .map_err(|_| failure("Console Proxmox : socket indisponible."))?;
    let config = tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(1_048_576))
        .max_frame_size(Some(1_048_576))
        .max_write_buffer_size(2_097_152);
    let (mut socket,response)=tungstenite::client_tls_with_config(request,stream,Some(config),Some(tungstenite::Connector::Rustls(Arc::new(tls_config(server)?))))
        .map_err(|_| failure("Console Proxmox : connexion WebSocket refusée. Vérifiez le certificat, VM.Console et le passage WebSocket du reverse proxy. Les anciennes versions peuvent nécessiter une connexion par mot de passe plutôt qu’un jeton API."))?;
    if response
        .headers()
        .get("Sec-WebSocket-Protocol")
        .and_then(|v| v.to_str().ok())
        != Some("binary")
    {
        return Err(failure("Console Proxmox : protocole WebSocket inattendu."));
    }
    let auth = Zeroizing::new(format!("{}:{}\n", ticket.user, ticket.ticket.as_str()));
    socket
        .send(Message::Text(auth.as_str().into()))
        .map_err(|_| failure("Console Proxmox : authentification interrompue."))?;
    let mut initial = Vec::new();
    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(10) {
            return Err(failure(
                "Console Proxmox : délai d’authentification dépassé.",
            ));
        }
        let message = socket.read().map_err(|_| {
            failure("Console Proxmox : authentification refusée ou connexion interrompue.")
        })?;
        if matches!(message, Message::Close(_)) {
            return Err(failure("Console Proxmox : authentification refusée."));
        }
        if let Some(bytes) = message_bytes(message) {
            initial.extend(bytes);
        }
        if initial.len() >= 2 {
            if !initial.starts_with(b"OK") {
                return Err(failure("Console Proxmox : authentification refusée."));
            }
            initial.drain(..2);
            break;
        }
    }
    tcp(&socket)?
        .set_nonblocking(true)
        .map_err(|_| failure("Console Proxmox : socket indisponible."))?;
    Ok(Console { socket, initial })
}
fn input_packet(input: &[u8]) -> Vec<u8> {
    let mut packet = format!("0:{}:", input.len()).into_bytes();
    packet.extend_from_slice(input);
    packet
}
impl Console {
    pub(crate) fn run(
        mut self,
        input: Receiver<Vec<u8>>,
        output: SyncSender<Vec<u8>>,
        sizes: Receiver<(u16, u16)>,
        stopped: Arc<AtomicBool>,
        ctx: eframe::egui::Context,
    ) {
        let mut pending = std::collections::VecDeque::new();
        if !self.initial.is_empty() {
            pending.push_back(std::mem::take(&mut self.initial));
        }
        let mut ping = Instant::now();
        'session: while !stopped.load(Ordering::Acquire) {
            let writable = match self.socket.flush() {
                Ok(()) => true,
                Err(e) if idle(&e) => false,
                Err(_) => break,
            };
            if writable {
                let message = if let Ok(bytes) = input.try_recv() {
                    let bytes = Zeroizing::new(bytes);
                    Some(Message::Binary(input_packet(&bytes).into()))
                } else if let Some((rows, cols)) = sizes.try_iter().last() {
                    Some(Message::Text(format!("1:{cols}:{rows}:").into()))
                } else if ping.elapsed() >= Duration::from_secs(30) {
                    ping = Instant::now();
                    Some(Message::Text("2".into()))
                } else {
                    None
                };
                if let Some(message) = message {
                    // WouldBlock means tungstenite retained this frame; only flush, never resend.
                    if let Err(e) = self.socket.send(message)
                        && !idle(&e)
                    {
                        break;
                    }
                }
            }
            while let Some(bytes) = pending.pop_front() {
                match output.try_send(bytes) {
                    Ok(()) => ctx.request_repaint(),
                    Err(TrySendError::Full(bytes)) => {
                        pending.push_front(bytes);
                        break;
                    }
                    Err(TrySendError::Disconnected(_)) => break 'session,
                }
            }
            // Backpressure: never discard output and never block cancellation on a full UI queue.
            if pending.is_empty() {
                match self.socket.read() {
                    Ok(Message::Close(_)) => break,
                    Ok(message) => {
                        if let Some(bytes) = message_bytes(message) {
                            pending.push_back(bytes);
                        }
                    }
                    Err(e) if idle(&e) => {}
                    Err(_) => break,
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.socket.close(None);
        ctx.request_repaint();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_input_length_is_bytes_and_escape_data_is_not_reinterpreted() {
        assert_eq!(
            input_packet("é\u{1b}[A".as_bytes()),
            b"0:5:\xc3\xa9\x1b[A".to_vec()
        );
    }
}

#[cfg(test)]
#[path = "proxmox_terminal_tests.rs"]
mod network_tests;
