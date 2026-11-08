use crate::{
    error::{AppError, Result},
    models::SessionInput,
};
use std::process::{Child, Command};

/// External-client display choice. Only known sizes can reach client arguments/files.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DisplaySize {
    #[default]
    ClientDefault,
    Fullscreen,
    Fixed {
        width: u16,
        height: u16,
    },
}
impl DisplaySize {
    pub const STANDARD: &[(u16, u16)] = &[
        (1024, 768),
        (1280, 720),
        (1280, 800),
        (1280, 1024),
        (1366, 768),
        (1440, 900),
        (1600, 900),
        (1680, 1050),
        (1920, 1080),
        (1920, 1200),
        (2560, 1440),
        (2560, 1600),
        (3440, 1440),
        (3840, 2160),
    ];
    pub fn validate(self) -> Result<()> {
        if let Self::Fixed { width, height } = self
            && !Self::STANDARD.contains(&(width, height))
        {
            return Err(AppError::Validation("Résolution RDP invalide.".into()));
        }
        Ok(())
    }
    pub fn file_settings(self) -> Result<String> {
        self.validate()?;
        Ok(match self {
            Self::ClientDefault => String::new(),
            Self::Fullscreen => "screen mode id:i:2\r\nuse multimon:i:0\r\n".into(),
            Self::Fixed { width, height } => format!(
                "screen mode id:i:1\r\nuse multimon:i:0\r\ndesktopwidth:i:{width}\r\ndesktopheight:i:{height}\r\n"
            ),
        })
    }
    pub fn apply_arguments(self, args: &mut Vec<String>, windows: bool) -> Result<()> {
        self.validate()?;
        match self {
            Self::ClientDefault => {}
            Self::Fullscreen => args.push("/f".into()),
            Self::Fixed { width, height } => {
                if windows {
                    args.extend([format!("/w:{width}"), format!("/h:{height}")]);
                } else {
                    args.retain(|a| a != "/dynamic-resolution");
                    args.push(format!("/size:{width}x{height}"));
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Gateway {
    pub rpc_transport: bool,
    pub host: String,
    pub username: String,
    pub usage: u8,
    pub credential_source: u8,
}
impl Default for Gateway {
    fn default() -> Self {
        Self {
            rpc_transport: false,
            host: String::new(),
            username: String::new(),
            usage: 1,
            credential_source: 4,
        }
    }
}
impl Gateway {
    pub fn arguments_with_routing(
        &self,
        s: &SessionInput,
        routing: &Routing,
    ) -> Result<Vec<String>> {
        routing.validate()?;
        let mut args = self.arguments(s)?;
        if !routing.load_balance_info.is_empty() {
            args.push(format!("/load-balance-info:{}", routing.load_balance_info));
        }
        Ok(args)
    }

    pub fn file_text_with_routing(&self, s: &SessionInput, routing: &Routing) -> Result<String> {
        routing.validate()?;
        let mut text = self.file_text(s)?;
        text.push_str(&format!(
            "loadbalanceinfo:s:{}\r\nuse redirection server name:i:{}\r\n",
            routing.load_balance_info,
            u8::from(routing.use_redirection_server_name)
        ));
        Ok(text)
    }
    pub fn validate(&self) -> Result<()> {
        crate::importers::rdp::endpoint(&self.host, 443)?;
        if ![1, 2].contains(&self.usage)
            || ![0, 1, 4].contains(&self.credential_source)
            || self.username.len() > 256
            || self.username.chars().any(char::is_control)
        {
            return Err(AppError::Validation(
                "Paramètres RD Gateway invalides.".into(),
            ));
        }
        Ok(())
    }
    pub fn arguments(&self, s: &SessionInput) -> Result<Vec<String>> {
        self.validate()?;
        if self.credential_source == 1 {
            return Err(AppError::Validation(
                "RD Gateway : authentification par carte à puce non configurée pour FreeRDP."
                    .into(),
            ));
        }
        let mut args = arguments(s, false)?;
        args.push(format!("/g:{}", self.host.trim()));
        args.push(format!(
            "/gateway-usage-method:{}",
            if self.usage == 2 { "detect" } else { "direct" }
        ));
        if !self.username.is_empty() {
            args.push(format!("/gu:{}", self.username));
        }
        Ok(args)
    }
    pub fn file_text(&self, s: &SessionInput) -> Result<String> {
        self.validate()?;
        s.validate()?;
        let host = if s.hostname.contains(':') && !s.hostname.starts_with('[') {
            format!("[{}]", s.hostname)
        } else {
            s.hostname.clone()
        };
        Ok(format!(
            "full address:s:{host}:{}\r\nusername:s:{}\r\ndomain:s:{}\r\ngatewayhostname:s:{}\r\ngatewayusagemethod:i:{}\r\ngatewayprofileusagemethod:i:1\r\ngatewaycredentialssource:i:{}\r\ngatewayusername:s:{}\r\npromptcredentialonce:i:0\r\nprompt for credentials:i:1\r\nauthentication level:i:1\r\nredirectclipboard:i:0\r\nredirectprinters:i:0\r\n",
            s.port,
            s.username,
            s.domain,
            self.host.trim(),
            self.usage,
            self.credential_source,
            self.username
        ))
    }
}
pub struct RdpSession {
    child: Child,
    // Keep authentication files alive even when a client hands off to another process.
    credential_file: Option<super::secret_file::ClientFile>,
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
    pub fn launch_gateway(s: &SessionInput, configured: &str, gateway: &Gateway) -> Result<Self> {
        Self::launch_gateway_with_routing(s, configured, gateway, &Routing::default())
    }
    pub fn launch_gateway_with_routing(
        s: &SessionInput,
        configured: &str,
        gateway: &Gateway,
        routing: &Routing,
    ) -> Result<Self> {
        Self::launch_gateway_with_display(s, configured, gateway, routing, DisplaySize::default())
    }
    pub fn launch_gateway_with_display(
        s: &SessionInput,
        configured: &str,
        gateway: &Gateway,
        routing: &Routing,
        display: DisplaySize,
    ) -> Result<Self> {
        let program = super::resolve_program(configured, super::ProgramKind::Rdp)?;
        let mut text = gateway.file_text_with_routing(s, routing)?;
        text.push_str(&display.file_settings()?);
        let bytes = std::iter::once(0xfeffu16)
            .chain(text.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let file = super::secret_file::create(".rdp", &bytes)?;
        let mut client = Self::launch_external(
            &program,
            vec![file.path().to_string_lossy().into_owned()],
            "RDP / RD Gateway",
        )?;
        client.credential_file = Some(file);
        Ok(client)
    }
    pub fn launch(s: &SessionInput, configured: &str, password: &str) -> Result<Self> {
        Self::launch_with_display(s, configured, password, DisplaySize::default())
    }
    pub fn launch_with_display(
        s: &SessionInput,
        configured: &str,
        password: &str,
        display: DisplaySize,
    ) -> Result<Self> {
        display.validate()?;
        let program = super::resolve_program(configured, super::ProgramKind::Rdp)?;
        s.validate()?;
        if password.is_empty() && (!cfg!(windows) || display == DisplaySize::ClientDefault) {
            let mut args = arguments(s, cfg!(windows))?;
            display.apply_arguments(&mut args, cfg!(windows))?;
            return Self::launch_external(&program, args, "RDP");
        }
        if !password.is_empty() && s.username.is_empty() {
            return Err(AppError::Validation(
                "Utilisateur RDP requis pour transmettre le mot de passe enregistré.".into(),
            ));
        }
        #[cfg(windows)]
        {
            let file = connection_file(s, password, display)?;
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
            display.apply_arguments(&mut args, false)?;
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
    pub(crate) fn retain_credential_file(&mut self, file: super::secret_file::ClientFile) {
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

#[cfg(all(windows, test))]
fn credential_file(s: &SessionInput, password: &str) -> Result<super::secret_file::ClientFile> {
    connection_file(s, password, DisplaySize::default())
}
#[cfg(windows)]
fn connection_file(
    s: &SessionInput,
    password: &str,
    display: DisplaySize,
) -> Result<super::secret_file::ClientFile> {
    s.validate()?;
    if password.contains('\0') {
        return Err(AppError::Validation(
            "Mot de passe RDP contenant un caractère NUL non pris en charge.".into(),
        ));
    }
    let authentication = if password.is_empty() {
        "prompt for credentials:i:1\r\n".to_owned()
    } else {
        let encrypted = super::secret_file::protect_password(password)?;
        let hex = encrypted
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>();
        format!("prompt for credentials:i:0\r\npassword 51:b:{hex}\r\n")
    };
    let host = if s.hostname.contains(':') && !s.hostname.starts_with('[') {
        format!("[{}]", s.hostname)
    } else {
        s.hostname.clone()
    };
    let mut text = format!(
        "full address:s:{host}:{}\r\nusername:s:{}\r\ndomain:s:{}\r\n{authentication}",
        s.port, s.username, s.domain
    );
    text.push_str(&display.file_settings()?);
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
        let display = super::DisplaySize::Fixed {
            width: 1920,
            height: 1080,
        };
        let file = super::connection_file(&data, "never-in-plaintext", display).expect("rdp file");
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
        assert!(text.contains("screen mode id:i:1\r\nuse multimon:i:0\r\ndesktopwidth:i:1920\r\ndesktopheight:i:1080\r\n"));
        let manual = super::connection_file(&data, "", display).expect("manual file");
        let bytes = std::fs::read(manual.path()).expect("read manual");
        let manual_text = String::from_utf16(
            &bytes[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes(*b))
                .collect::<Vec<_>>(),
        )
        .expect("utf16");
        assert!(manual_text.contains("prompt for credentials:i:1"));
        assert!(manual_text.contains("desktopwidth:i:1920"));
        assert!(!manual_text.contains("password 51:"));
        let mut invalid = data;
        invalid.username = "alice\r\nredirectclipboard:i:1".into();
        assert!(super::credential_file(&invalid, "fixture").is_err());
    }
}
#[cfg(test)]
mod display_tests {
    use super::*;
    #[test]
    fn display_choices_are_validated_and_client_specific() {
        for &(width, height) in DisplaySize::STANDARD {
            let display = DisplaySize::Fixed { width, height };
            let mut windows = vec!["/v:host:3389".into(), "/prompt".into()];
            display
                .apply_arguments(&mut windows, true)
                .expect("windows");
            assert_eq!(
                windows[2..],
                [format!("/w:{width}"), format!("/h:{height}")]
            );
            let mut linux = vec!["/v:host:3389".into(), "/dynamic-resolution".into()];
            display.apply_arguments(&mut linux, false).expect("linux");
            assert_eq!(
                linux,
                ["/v:host:3389".to_owned(), format!("/size:{width}x{height}")]
            );
            let file = display.file_settings().expect("file settings");
            assert!(file.contains("screen mode id:i:1"));
            assert!(file.contains(&format!(
                "desktopwidth:i:{width}\r\ndesktopheight:i:{height}"
            )));
            assert_eq!(
                serde_json::from_str::<DisplaySize>(
                    &serde_json::to_string(&display).expect("serialize")
                )
                .expect("reload"),
                display
            );
        }
        assert!(
            DisplaySize::ClientDefault
                .file_settings()
                .expect("default")
                .is_empty()
        );
        assert!(
            DisplaySize::Fullscreen
                .file_settings()
                .expect("fullscreen")
                .contains("screen mode id:i:2")
        );
        let invalid = DisplaySize::Fixed {
            width: 0,
            height: 65535,
        };
        let mut args = vec!["/v:host".into()];
        assert!(invalid.apply_arguments(&mut args, true).is_err());
        assert_eq!(args, ["/v:host"]);
        assert!(invalid.file_settings().is_err());
    }
}
/// Routing settings imported independently of RD Gateway.
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Routing {
    pub load_balance_info: String,
    pub use_redirection_server_name: bool,
}

impl Routing {
    pub fn validate(&self) -> crate::error::Result<()> {
        // X.224 has a one-byte length indicator. Leave space for its headers,
        // the routing-cookie prefix, CRLF and RDP negotiation request.
        if self.load_balance_info.len() > 200
            || !self.load_balance_info.is_ascii()
            || self.load_balance_info.chars().any(char::is_control)
        {
            return Err(crate::error::AppError::Validation(
                "RDP : information de routage invalide (ASCII, 200 octets maximum, sans caractères de contrôle).".into(),
            ));
        }
        Ok(())
    }
}
