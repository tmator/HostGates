pub mod native;
pub mod rdp;
mod secret_file;
pub mod terminal;
pub mod vnc;
pub mod web;
use std::path::PathBuf;
pub fn find_program(names: &[&str]) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for name in names {
        for dir in std::env::split_paths(&paths) {
            let path = dir.join(name);
            if path.is_file() {
                return Some(path);
            }
            #[cfg(windows)]
            {
                let path = dir.join(format!("{name}.exe"));
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }
    None
}
pub fn ssh_arguments(s: &crate::models::SessionInput) -> crate::error::Result<Vec<String>> {
    s.validate()?;
    let mut args = vec![
        "-tt".into(),
        "-o".into(),
        "StrictHostKeyChecking=ask".into(),
        "-o".into(),
        "ServerAliveInterval=30".into(),
        "-o".into(),
        "ServerAliveCountMax=3".into(),
        "-o".into(),
        "ConnectTimeout=15".into(),
        "-p".into(),
        s.port.to_string(),
    ];
    if !s.username.is_empty() {
        args.extend(["-l".into(), s.username.clone()]);
    }
    args.extend([
        "--".into(),
        s.hostname
            .trim_start_matches('[')
            .trim_end_matches(']')
            .into(),
    ]);
    Ok(args)
}

#[derive(Clone, Copy)]
pub enum ProgramKind {
    Ssh,
    Rdp,
    Vnc,
    Terminal,
}
impl ProgramKind {
    pub fn defaults(self) -> &'static [&'static str] {
        match self {
            Self::Ssh => &["ssh"],
            Self::Vnc => &["vncviewer", "tigervncviewer", "xtigervncviewer"],
            Self::Rdp => {
                if cfg!(windows) {
                    &["mstsc"]
                } else {
                    &["xfreerdp3", "xfreerdp"]
                }
            }
            Self::Terminal => {
                if cfg!(windows) {
                    &["pwsh", "powershell"]
                } else {
                    &["bash", "sh"]
                }
            }
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Ssh => "OpenSSH",
            Self::Rdp => "Client RDP",
            Self::Vnc => "Client VNC (TigerVNC)",
            Self::Terminal => "Terminal local",
        }
    }
}
pub fn resolve_program(configured: &str, kind: ProgramKind) -> crate::error::Result<PathBuf> {
    use crate::error::AppError;
    if configured.is_empty() {
        return find_program(kind.defaults()).ok_or_else(|| AppError::Operation(format!(
            "{} introuvable dans PATH. Ouvrez Paramètres > Applications externes et sélectionnez son exécutable. Consultez Aide > Configurer les connexions.", kind.label())));
    }
    let path = PathBuf::from(configured);
    let invalid = || {
        AppError::Validation(format!(
            "{} : sélectionnez un fichier exécutable existant avec un chemin absolu, sans guillemets ni arguments.",
            kind.label()
        ))
    };
    if configured.chars().any(char::is_control) || !path.is_absolute() || !path.is_file() {
        return Err(invalid());
    }
    #[cfg(windows)]
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
    {
        return Err(invalid());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::fs::metadata(&path)
            .map_err(|_| invalid())?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(invalid());
        }
    }
    Ok(path)
}
impl crate::models::ExternalPrograms {
    pub fn validate(&self) -> crate::error::Result<()> {
        for (value, kind) in [
            (&self.ssh, ProgramKind::Ssh),
            (&self.rdp, ProgramKind::Rdp),
            (&self.vnc, ProgramKind::Vnc),
            (&self.terminal, ProgramKind::Terminal),
        ] {
            if !value.is_empty() {
                resolve_program(value, kind)?;
            }
        }
        Ok(())
    }
}
