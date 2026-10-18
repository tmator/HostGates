//! SSH transport for terminal, SFTP and loopback-only forwarding.
use crate::{
    credentials::Secret,
    error::{AppError, Result},
};
use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Clone)]
pub struct Auth {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Secret,
    pub key: PathBuf,
    pub fingerprint: String,
    pub endpoint: Option<(String, u16)>,
}
fn error(e: impl std::fmt::Display) -> AppError {
    AppError::Operation(e.to_string())
}
pub fn connect(auth: &Auth) -> Result<ssh2::Session> {
    if auth.user.is_empty() {
        return Err(error("Utilisateur SSH requis."));
    }
    let (host, port) = auth
        .endpoint
        .as_ref()
        .map(|(h, p)| (h.as_str(), *p))
        .unwrap_or((&auth.host, auth.port));
    let mut tcp = None;
    for addr in (host, port).to_socket_addrs().map_err(error)? {
        if let Ok(socket) = TcpStream::connect_timeout(&addr, Duration::from_secs(10)) {
            tcp = Some(socket);
            break;
        }
    }
    let tcp = tcp.ok_or_else(|| error("Connexion SSH impossible (délai 10 s par adresse)."))?;
    let mut session = ssh2::Session::new().map_err(error)?;
    session.method_pref(ssh2::MethodType::Kex,"curve25519-sha256,curve25519-sha256@libssh.org,ecdh-sha2-nistp256,ecdh-sha2-nistp384,ecdh-sha2-nistp521,diffie-hellman-group16-sha512,diffie-hellman-group18-sha512,diffie-hellman-group14-sha256,diffie-hellman-group-exchange-sha256").map_err(error)?;
    for method in [ssh2::MethodType::CryptCs, ssh2::MethodType::CryptSc] {
        session.method_pref(method,"chacha20-poly1305@openssh.com,aes256-gcm@openssh.com,aes128-gcm@openssh.com,aes256-ctr,aes128-ctr").map_err(error)?;
    }
    for method in [ssh2::MethodType::MacCs, ssh2::MethodType::MacSc] {
        session.method_pref(method,"hmac-sha2-256-etm@openssh.com,hmac-sha2-512-etm@openssh.com,hmac-sha2-256,hmac-sha2-512").map_err(error)?;
    }
    session.method_pref(ssh2::MethodType::HostKey,"ssh-ed25519,ecdsa-sha2-nistp256,ecdsa-sha2-nistp384,ecdsa-sha2-nistp521,rsa-sha2-512,rsa-sha2-256").map_err(error)?;
    session.set_tcp_stream(tcp);
    session.set_timeout(15000);
    session.handshake().map_err(|e| {
        if e.code() == ssh2::ErrorCode::Session(-5) {
            error("Négociation cryptographique SSH impossible. Vérifiez les algorithmes et les clés hôte du serveur (Ed25519, ECDSA ou RSA-SHA2). Un serveur ancien ou utilisant uniquement des certificats hôte peut nécessiter OpenSSH externe dans les options de la session. Aucun mot de passe n'a encore été envoyé.")
        } else {
            error(e)
        }
    })?;
    let digest = session
        .host_key_hash(ssh2::HashType::Sha256)
        .ok_or_else(|| error("Empreinte SSH indisponible."))?;
    let fingerprint = digest
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect::<String>();
    let (hostkey, _) = session
        .host_key()
        .ok_or_else(|| error("Clé SSH absente."))?;
    let mut known = session.known_hosts().map_err(error)?;
    if let Some(base) = directories::BaseDirs::new() {
        let file = base.home_dir().join(".ssh/known_hosts");
        if file.exists() {
            known
                .read_file(&file, ssh2::KnownHostFileKind::OpenSSH)
                .map_err(error)?;
        }
    }
    let check = known.check_port(&auth.host, auth.port, hostkey);
    // An explicit pin also detects changes after an earlier accepted connection.
    if matches!(
        check,
        ssh2::CheckResult::Mismatch | ssh2::CheckResult::Failure
    ) {
        return Err(error(format!(
            "HOSTKEY_CHANGED {}:{} SHA256(hex)={fingerprint}",
            auth.host, auth.port
        )));
    }
    if !auth.fingerprint.is_empty() {
        if auth.fingerprint != fingerprint {
            return Err(error(format!(
                "HOSTKEY_CHANGED {}:{} SHA256(hex)={fingerprint}",
                auth.host, auth.port
            )));
        }
    } else if !matches!(check, ssh2::CheckResult::Match) {
        let code = if matches!(check, ssh2::CheckResult::Mismatch) {
            "HOSTKEY_CHANGED"
        } else {
            "HOSTKEY_UNKNOWN"
        };
        return Err(error(format!(
            "{code} {}:{} SHA256(hex)={fingerprint}",
            auth.host, auth.port
        )));
    }
    if !auth.key.as_os_str().is_empty() {
        session
            .userauth_pubkey_file(
                &auth.user,
                None,
                &auth.key,
                if auth.password.is_empty() {
                    None
                } else {
                    Some(&auth.password)
                },
            )
            .map_err(error)?;
    } else if !auth.password.is_empty() {
        session
            .userauth_password(&auth.user, &auth.password)
            .map_err(error)?;
    } else {
        session.userauth_agent(&auth.user).map_err(error)?;
    }
    if !session.authenticated() {
        return Err(error("Authentification SSH refusée."));
    }
    Ok(session)
}

pub struct Tunnel {
    pub host: String,
    pub port: u16,
    stop: Arc<AtomicBool>,
}
impl Drop for Tunnel {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}
impl Tunnel {
    pub fn start(auth: Auth, target: String, port: u16) -> Result<Self> {
        // Authenticate and verify the gateway before exposing a listener.
        let initial = connect(&auth)?;
        // A distinct loopback host separates browser cookies between tunnels.
        // This is not authentication against other local processes.
        let bytes = *uuid::Uuid::new_v4().as_bytes();
        let host = format!("127.{}.{}.{}", bytes[0], bytes[1], bytes[2].max(1));
        let listener = std::net::TcpListener::bind((host.as_str(), 0)).map_err(error)?;
        let local = listener.local_addr().map_err(error)?.port();
        listener.set_nonblocking(true).map_err(error)?;
        let stop = Arc::new(AtomicBool::new(false));
        let cancel = stop.clone();
        std::thread::spawn(move || {
            let mut first = Some(initial);
            let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            while !cancel.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        if count.load(Ordering::Acquire) >= 32 {
                            continue;
                        }
                        count.fetch_add(1, Ordering::AcqRel);
                        let session = first.take();
                        let auth = auth.clone();
                        let target = target.clone();
                        let cancel = cancel.clone();
                        let count = count.clone();
                        std::thread::spawn(move || {
                            let _ = (|| -> Result<()> {
                                let session = match session {
                                    Some(s) => s,
                                    None => connect(&auth)?,
                                };
                                let mut channel = session
                                    .channel_direct_tcpip(&target, port, None)
                                    .map_err(error)?;
                                session.set_blocking(false);
                                socket.set_nonblocking(true).map_err(error)?;
                                let mut to_remote = Vec::new();
                                let mut to_local = Vec::new();
                                let mut buf = [0u8; 32768];
                                let mut local_eof = false;
                                let mut sent_eof = false;
                                while !cancel.load(Ordering::Acquire) {
                                    if !local_eof && to_remote.len() < 65536 {
                                        match socket.read(&mut buf) {
                                            Ok(0) => local_eof = true,
                                            Ok(n) => to_remote.extend_from_slice(&buf[..n]),
                                            Err(e)
                                                if e.kind() == std::io::ErrorKind::WouldBlock => {}
                                            Err(e) => return Err(error(e)),
                                        }
                                    }
                                    if !to_remote.is_empty() {
                                        match channel.write(&to_remote) {
                                            Ok(n) => {
                                                to_remote.drain(..n);
                                            }
                                            Err(e)
                                                if e.kind() == std::io::ErrorKind::WouldBlock => {}
                                            Err(e) => return Err(error(e)),
                                        }
                                    }
                                    if local_eof && to_remote.is_empty() && !sent_eof {
                                        sent_eof = channel.send_eof().is_ok();
                                    }
                                    if to_local.len() < 65536 {
                                        match channel.read(&mut buf) {
                                            Ok(n) => to_local.extend_from_slice(&buf[..n]),
                                            Err(e)
                                                if e.kind() == std::io::ErrorKind::WouldBlock => {}
                                            Err(e) => return Err(error(e)),
                                        }
                                    }
                                    if !to_local.is_empty() {
                                        match socket.write(&to_local) {
                                            Ok(n) => {
                                                to_local.drain(..n);
                                            }
                                            Err(e)
                                                if e.kind() == std::io::ErrorKind::WouldBlock => {}
                                            Err(e) => return Err(error(e)),
                                        }
                                    }
                                    if channel.eof() && to_local.is_empty() {
                                        break;
                                    }
                                    std::thread::sleep(Duration::from_millis(5));
                                }
                                Ok(())
                            })();
                            count.fetch_sub(1, Ordering::AcqRel);
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(20))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            host,
            port: local,
            stop,
        })
    }
}

#[derive(Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub directory: bool,
    pub size: u64,
}
pub enum FileAction {
    List(PathBuf),
    Upload(Vec<PathBuf>, PathBuf),
    Download(Vec<Entry>, PathBuf),
}
pub struct FileBrowser {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: std::collections::BTreeSet<PathBuf>,
    pub status: String,
    pub busy: bool,
    pub open: bool,
    tx: std::sync::mpsc::Sender<FileAction>,
    rx: std::sync::mpsc::Receiver<Result<(PathBuf, Vec<Entry>)>>,
    cancel: Arc<AtomicBool>,
}
impl Drop for FileBrowser {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}
impl FileBrowser {
    pub fn start(auth: Auth, ctx: eframe::egui::Context) -> Self {
        let (tx, commands) = std::sync::mpsc::channel();
        let (results, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancelled = cancel.clone();
        std::thread::spawn(move || {
            let mut path = PathBuf::from(".");
            for action in commands {
                if cancelled.load(Ordering::Acquire) {
                    break;
                }
                let result = (|| -> Result<_> {
                    let session = connect(&auth)?;
                    let sftp = session.sftp().map_err(error)?;
                    match action {
                        FileAction::List(p) => path = sftp.realpath(&p).map_err(error)?,
                        FileAction::Upload(files, dir) => {
                            for file in files {
                                let name = safe_name(&file)?;
                                upload(&sftp, &file, &dir.join(name), &cancelled, 0)?;
                            }
                            path = dir;
                        }
                        FileAction::Download(entries, dir) => {
                            let mut budget = DownloadBudget::default();
                            for entry in entries {
                                let name = safe_name(&entry.path)?;
                                download(
                                    &sftp,
                                    &entry.path,
                                    &dir.join(name),
                                    &cancelled,
                                    0,
                                    &mut budget,
                                )?;
                            }
                        }
                    }
                    let mut entries = Vec::new();
                    for (p, stat) in sftp.readdir(&path).map_err(error)? {
                        let Ok(name) = safe_name(&p) else {
                            continue;
                        };
                        entries.push(Entry {
                            path: p.clone(),
                            name,
                            directory: stat.is_dir(),
                            size: stat.size.unwrap_or(0),
                        });
                    }
                    entries.sort_by(|a, b| b.directory.cmp(&a.directory).then(a.name.cmp(&b.name)));
                    Ok((path.clone(), entries))
                })();
                if results.send(result).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            path: PathBuf::from("."),
            entries: vec![],
            selected: Default::default(),
            status: String::new(),
            busy: false,
            open: false,
            tx,
            rx,
            cancel,
        }
    }
    pub fn send(&mut self, action: FileAction) {
        if !self.busy {
            self.cancel.store(false, Ordering::Release);
            self.busy = self.tx.send(action).is_ok();
            self.status = "SFTP…".into();
        }
    }
    pub fn poll(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            self.busy = false;
            match result {
                Ok((p, e)) => {
                    self.path = p;
                    self.entries = e;
                    self.selected.clear();
                    self.status = "Terminé".into();
                }
                Err(e) => self.status = e.to_string(),
            }
        }
    }
    pub fn cancel_operation(&self) {
        self.cancel.store(true, Ordering::Release);
    }
}
pub fn safe_name(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| error("Nom de fichier non UTF-8 ou invalide."))?;
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.chars().any(|c| c.is_control() || "/\\:".contains(c))
    {
        return Err(error("Nom distant non sûr."));
    }
    #[cfg(windows)]
    if name.ends_with(['.', ' '])
        || name.contains(['<', '>', '"', '|', '?', '*'])
        || matches!(
            name.split('.').next().unwrap_or("").to_uppercase().as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
    {
        return Err(error("Nom non représentable sous Windows."));
    }
    Ok(name.into())
}
fn check(cancel: &AtomicBool, depth: usize) -> Result<()> {
    if cancel.load(Ordering::Acquire) || depth > 64 {
        Err(error("Transfert annulé ou profondeur excessive."))
    } else {
        Ok(())
    }
}
fn copy(mut from: impl Read, mut to: impl Write, cancel: &AtomicBool) -> Result<()> {
    let mut b = [0u8; 65536];
    loop {
        check(cancel, 0)?;
        let n = from.read(&mut b).map_err(error)?;
        if n == 0 {
            break;
        }
        to.write_all(&b[..n]).map_err(error)?;
    }
    to.flush().map_err(error)
}
fn upload(
    sftp: &ssh2::Sftp,
    local: &Path,
    remote: &Path,
    cancel: &AtomicBool,
    depth: usize,
) -> Result<()> {
    check(cancel, depth)?;
    let meta = std::fs::symlink_metadata(local).map_err(error)?;
    if meta.file_type().is_symlink() {
        return Err(error("Liens symboliques non transférés."));
    }
    if meta.is_dir() {
        sftp.mkdir(remote, 0o700).map_err(error)?;
        for item in std::fs::read_dir(local).map_err(error)? {
            let file = item.map_err(error)?.path();
            upload(
                sftp,
                &file,
                &remote.join(safe_name(&file)?),
                cancel,
                depth + 1,
            )?;
        }
    } else if meta.is_file() {
        let mut dest = sftp
            .open_mode(
                remote,
                ssh2::OpenFlags::WRITE | ssh2::OpenFlags::CREATE | ssh2::OpenFlags::EXCLUSIVE,
                0o600,
                ssh2::OpenType::File,
            )
            .map_err(error)?;
        copy(
            std::fs::File::open(local).map_err(error)?,
            &mut dest,
            cancel,
        )?;
        dest.close().map_err(error)?;
    } else {
        return Err(error("Fichier spécial non transféré."));
    }
    Ok(())
}
#[derive(Default)]
struct DownloadBudget {
    entries: u64,
    bytes: u64,
}
impl DownloadBudget {
    fn consume(&mut self, entries: u64, bytes: u64) -> Result<()> {
        if entries > 10000 - self.entries || bytes > 10 * 1024 * 1024 * 1024 - self.bytes {
            return Err(error(
                "Limite de téléchargement atteinte (10 000 entrées ou 10 Gio par opération).",
            ));
        }
        self.entries += entries;
        self.bytes += bytes;
        Ok(())
    }
}
fn download(
    sftp: &ssh2::Sftp,
    remote: &Path,
    local: &Path,
    cancel: &AtomicBool,
    depth: usize,
    budget: &mut DownloadBudget,
) -> Result<()> {
    check(cancel, depth)?;
    budget.consume(1, 0)?;
    let meta = sftp.lstat(remote).map_err(error)?;
    if meta.is_dir() {
        std::fs::create_dir(local).map_err(error)?;
        for (p, _) in sftp.readdir(remote).map_err(error)? {
            let name = safe_name(&p)?;
            download(sftp, &p, &local.join(name), cancel, depth + 1, budget)?;
        }
    } else if meta.is_file() {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(local)
            .map_err(error)?;
        let mut file = file;
        let mut source = sftp.open(remote).map_err(error)?;
        let mut buffer = [0u8; 65536];
        loop {
            check(cancel, depth)?;
            let count = source.read(&mut buffer).map_err(error)?;
            if count == 0 {
                break;
            }
            budget.consume(0, count as u64)?;
            file.write_all(&buffer[..count]).map_err(error)?;
        }
        file.flush().map_err(error)?;
    } else {
        return Err(error("Lien symbolique ou fichier spécial non transféré."));
    }
    Ok(())
}

#[cfg(test)]
mod budget_tests {
    #[test]
    fn download_limits_are_cumulative_and_fail_before_writing() {
        let mut budget = super::DownloadBudget::default();
        budget
            .consume(9999, 10 * 1024 * 1024 * 1024 - 1)
            .expect("within limit");
        budget.consume(1, 1).expect("at limit");
        assert!(budget.consume(1, 0).is_err());
        assert!(budget.consume(0, 1).is_err());
    }
}
