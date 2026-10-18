use crate::{
    error::{AppError, Result},
    models::SessionInput,
};
use std::process::{Child, Command};
pub struct RdpSession {
    child: Child,
    // Keep authentication files alive even when a client hands off to another process.
    credential_file: Option<tempfile::NamedTempFile>,
    pub status: String,
    pub closed: bool,
}
pub fn arguments(s: &SessionInput, windows: bool) -> Result<Vec<String>> {
    s.validate()?;
    let host = if s.hostname.contains(':') && !s.hostname.starts_with('[') {
        format!("[{}]", s.hostname)
    } else {
        s.hostname.clone()
    };
    if windows {
        Ok(vec![format!("/v:{host}:{}", s.port), "/prompt".into()])
    } else {
        let mut args = vec![
            format!("/v:{host}:{}", s.port),
            "/dynamic-resolution".into(),
        ];
        if !s.username.is_empty() {
            args.push(format!("/u:{}", s.username));
        }
        if !s.domain.is_empty() {
            args.push(format!("/d:{}", s.domain));
        }
        Ok(args)
    }
}
impl RdpSession {
    pub fn launch(s: &SessionInput, configured: &str, password: &str) -> Result<Self> {
        let program = super::resolve_program(configured, super::ProgramKind::Rdp)?;
        s.validate()?;
        if password.is_empty() {
            return Self::launch_external(&program, arguments(s, cfg!(windows))?, "RDP");
        }
        if s.username.is_empty() {
            return Err(AppError::Validation(
                "Utilisateur RDP requis pour transmettre le mot de passe enregistré.".into(),
            ));
        }
        #[cfg(windows)]
        {
            let file = credential_file(s, password)?;
            let mut client = Self::launch_external(
                &program,
                vec![file.path().to_string_lossy().into_owned()],
                "RDP",
            )?;
            client.credential_file = Some(file);
            Ok(client)
        }
        #[cfg(not(windows))]
        {
            use std::io::Write;
            // Without /d FreeRDP may consume the password as the domain.
            if s.domain.trim().is_empty() {
                return Err(AppError::Validation("FreeRDP : renseignez explicitement le domaine avant de transmettre un mot de passe enregistré, ou utilisez une authentification manuelle.".into()));
            }
            if password.contains(['\r', '\n', '\0']) {
                return Err(AppError::Validation("FreeRDP ne peut pas recevoir ce mot de passe par entrée standard (caractère de contrôle).".into()));
            }
            let mut args = arguments(s, false)?;
            args.push("/from-stdin:force".into());
            let mut child = Command::new(program)
                .args(args)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|_| AppError::Operation("Impossible de lancer FreeRDP.".into()))?;
            let result = child
                .stdin
                .take()
                .ok_or_else(|| std::io::Error::other("stdin unavailable"))
                .and_then(|mut pipe| {
                    pipe.write_all(password.as_bytes())
                        .and_then(|_| pipe.write_all(b"\n"))
                });
            if result.is_err() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::Operation(
                    "Impossible de transmettre le mot de passe à FreeRDP.".into(),
                ));
            }
            Ok(Self {
                child,
                credential_file: None,
                status: "Client RDP lancé dans une fenêtre externe".into(),
                closed: false,
            })
        }
    }
    pub fn launch_external(
        program: &std::path::Path,
        args: Vec<String>,
        protocol: &str,
    ) -> Result<Self> {
        let child = Command::new(program).args(args).spawn().map_err(|_| {
            AppError::Operation(format!("Impossible de lancer le client {protocol}."))
        })?;
        Ok(Self {
            child,
            credential_file: None,
            status: format!("Client {protocol} lancé dans une fenêtre externe"),
            closed: false,
        })
    }
    pub(super) fn retain_credential_file(&mut self, file: tempfile::NamedTempFile) {
        self.credential_file = Some(file);
    }
    pub fn poll(&mut self) {
        if !self.closed {
            match self.child.try_wait() {
                Ok(Some(_)) => {
                    self.closed = true;
                    self.status =
                        "Client externe fermé (ou transféré à un processus existant)".into();
                }
                Ok(None) => {}
                Err(_) => self.status = "Suivi du client externe indisponible".into(),
            }
        }
    }
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        self.closed = true;
        self.status = "Arrêt du client externe demandé".into();
    }
}
impl Drop for RdpSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(windows)]
fn credential_file(s: &SessionInput, password: &str) -> Result<tempfile::NamedTempFile> {
    s.validate()?;
    if password.contains('\0') {
        return Err(AppError::Validation(
            "Mot de passe RDP contenant un caractère NUL non pris en charge.".into(),
        ));
    }
    let encrypted = super::secret_file::protect_password(password)?;
    let hex = encrypted
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<String>();
    let host = if s.hostname.contains(':') && !s.hostname.starts_with('[') {
        format!("[{}]", s.hostname)
    } else {
        s.hostname.clone()
    };
    let text = format!(
        "full address:s:{host}:{}\r\nusername:s:{}\r\ndomain:s:{}\r\nprompt for credentials:i:0\r\npassword 51:b:{hex}\r\n",
        s.port, s.username, s.domain
    );
    let bytes = std::iter::once(0xfeffu16)
        .chain(text.encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    super::secret_file::create(".rdp", &bytes)
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn rdp_file_contains_protected_password_and_separate_identity() {
        let data = crate::models::SessionInput {
            name: "fixture".into(),
            protocol: crate::models::Protocol::RDP,
            hostname: "host.example".into(),
            port: 3389,
            username: "alice".into(),
            domain: "LAB".into(),
            folder_id: None,
            favorite: false,
            description: String::new(),
            color: "#7195ff".into(),
            icon: "server".into(),
            tags: vec![],
        };
        let file = super::credential_file(&data, "never-in-plaintext").expect("rdp file");
        let bytes = std::fs::read(file.path()).expect("read");
        assert_eq!(&bytes[..2], &[0xff, 0xfe]);
        let text = String::from_utf16(
            &bytes[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )
        .expect("utf16");
        assert!(text.contains("username:s:alice\r\ndomain:s:LAB"));
        assert!(text.contains("prompt for credentials:i:0"));
        assert!(text.contains("password 51:b:01000000"));
        assert!(!text.contains("never-in-plaintext"));
        let mut invalid = data;
        invalid.username = "alice\r\nredirectclipboard:i:1".into();
        assert!(super::credential_file(&invalid, "fixture").is_err());
    }
}
