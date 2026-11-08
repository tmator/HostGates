//! Windows remoting adapter. Script is fixed; credentials travel only through stdin.
use crate::{
    error::{AppError, Result},
    proxmox::{Server, Vm},
};
use std::path::PathBuf;
fn failure(message: &str) -> AppError {
    AppError::Operation(message.into())
}
pub fn validate(server: &Server) -> Result<url::Url> {
    let url =
        url::Url::parse(server.url.trim()).map_err(|_| failure("Hyper-V : URL WinRM invalide."))?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
        || server.identity.trim().is_empty()
        || server.identity.chars().any(char::is_control)
        || server.identity.len() > 256
    {
        return Err(failure(
            "Hyper-V : utilisez https://hote:5986 et un compte DOMAINE\\utilisateur.",
        ));
    }
    if server.skip_tls_verification {
        return Err(failure(
            "Hyper-V : installez le certificat de confiance dans Windows. La vérification WinRM ne peut pas être désactivée.",
        ));
    }
    if !server.vmconnect.as_os_str().is_empty()
        && (!server.vmconnect.is_absolute() || !server.vmconnect.is_file())
    {
        return Err(failure(
            "Hyper-V : sélectionnez un chemin absolu existant vers vmconnect.exe.",
        ));
    }
    Ok(url)
}
pub fn run(server: &Server, secret: &str, action: &str, id: &str) -> Result<Vec<Vm>> {
    server.validate()?;
    if !matches!(
        action,
        "list" | "console" | "start" | "shutdown" | "reboot" | "stop"
    ) || (action != "list" && uuid::Uuid::parse_str(id).is_err())
        || secret.is_empty()
        || secret.len() > 16_384
        || secret.contains(['\0', '\r', '\n'])
    {
        return Err(failure("Hyper-V : paramètres invalides."));
    }
    #[cfg(not(windows))]
    {
        Err(failure(
            "La gestion Hyper-V nécessite actuellement HostGates sous Windows et PowerShell distant. Proxmox reste disponible sous Linux.",
        ))
    }
    #[cfg(windows)]
    {
        use std::{
            io::{Read, Write},
            os::windows::process::CommandExt,
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        let executable =
            PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
                .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        #[derive(serde::Serialize)]
        struct Request<'a> {
            url: &'a str,
            identity: &'a str,
            secret: &'a str,
            cluster: bool,
            action: &'a str,
            id: &'a str,
        }
        let payload = zeroize::Zeroizing::new(
            serde_json::to_vec(&Request {
                url: &server.url,
                identity: &server.identity,
                secret,
                cluster: server.cluster,
                action,
                id,
            })
            .map_err(|_| failure("Hyper-V : requête invalide."))?,
        );
        let mut child = Command::new(executable)
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                include_str!("hyperv.ps1"),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|_| failure("Hyper-V : PowerShell Windows indisponible."))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| failure("Hyper-V : sortie indisponible."))?;
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            output
                .take(4_194_305)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        });
        let write = child
            .stdin
            .take()
            .ok_or_else(|| failure("Hyper-V : entrée indisponible."))?
            .write_all(&payload);
        if write.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(failure("Hyper-V : envoi interrompu."));
        }
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if start.elapsed() < Duration::from_secs(120) => {
                    std::thread::sleep(Duration::from_millis(100))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(failure(
                        "Hyper-V : délai dépassé. Une commande distante peut encore se terminer ; actualisez avant de réessayer.",
                    ));
                }
            }
        };
        if !status.success() {
            return Err(failure(
                "Hyper-V : opération refusée. Vérifiez WinRM, le certificat, les identifiants, les droits Hyper-V et les modules Hyper-V / FailoverClusters. Aucun arrêt forcé de secours n’a été demandé.",
            ));
        }
        let bytes = reader
            .join()
            .map_err(|_| failure("Hyper-V : lecture interrompue."))?
            .map_err(|_| failure("Hyper-V : lecture impossible."))?;
        if bytes.len() > 4_194_304 {
            return Err(failure("Hyper-V : inventaire trop volumineux."));
        }
        let vms: Vec<Vm> = serde_json::from_slice(&bytes)
            .map_err(|_| failure("Hyper-V : inventaire invalide."))?;
        for vm in &vms {
            if uuid::Uuid::parse_str(&vm.native_id).is_err()
                || vm.node.is_empty()
                || !vm
                    .node
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
                || vm.name.len() > 256
                || vm.name.chars().any(char::is_control)
            {
                return Err(failure("Hyper-V : machine invalide."));
            }
        }
        Ok(vms)
    }
}
pub fn console_program(server: &Server) -> Result<PathBuf> {
    if !server.vmconnect.as_os_str().is_empty() {
        return Ok(server.vmconnect.clone());
    }
    #[cfg(windows)]
    {
        let path =
            PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
                .join("System32/vmconnect.exe");
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(failure(
        "Installez les outils de gestion Hyper-V et configurez vmconnect.exe dans le profil du serveur.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hyperv_profile_rejects_embedded_secrets_and_tls_bypass() {
        let mut server = Server {
            hyperv: true,
            name: "Lab".into(),
            url: "https://node.example:5986".into(),
            identity: "LAB\\tester".into(),
            ..Default::default()
        };
        assert!(server.validate().is_ok());
        for url in [
            "https://user:secret@node:5986",
            "https://node:5986/wsman",
            "https://node:5986?secret=bad",
            "file:///tmp/test",
        ] {
            server.url = url.into();
            assert!(server.validate().is_err());
        }
        server.url = "https://node.example:5986".into();
        server.skip_tls_verification = true;
        assert!(server.validate().is_err());
        server.skip_tls_verification = false;
        assert!(run(&server, "synthetic-secret", "stop", "not-a-guid").is_err());
        assert!(
            run(
                &server,
                "synthetic-secret",
                "delete",
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"
            )
            .is_err()
        );
    }
}
