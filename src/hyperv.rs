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
            "Hyper-V : installez le certificat de confiance dans Windows. La vérification WinRM ne peut pas être désactivée. Sous Linux, configurez un CA PEM dans le profil.",
        ));
    }
    if cfg!(windows)
        && !server.vmconnect.as_os_str().is_empty()
        && (!server.vmconnect.is_absolute() || !server.vmconnect.is_file())
    {
        return Err(failure(
            "Hyper-V : sélectionnez un chemin absolu existant vers vmconnect.exe.",
        ));
    }
    #[cfg(not(windows))]
    {
        if url.scheme() != "https" {
            return Err(failure(
                "Hyper-V Linux : HTTPS WinRM est requis (port 5986).",
            ));
        }
        for path in [
            &server.hyperv_python,
            &server.hyperv_freerdp,
            &server.ca_file,
        ] {
            if !path.as_os_str().is_empty() && (!path.is_absolute() || !path.is_file()) {
                return Err(failure(
                    "Hyper-V Linux : sélectionnez un chemin absolu existant.",
                ));
            }
        }
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
    {
        use std::{
            io::{Read, Write},
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        #[cfg(windows)]
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
            ca_file: &'a std::path::Path,
        }
        let payload = zeroize::Zeroizing::new(
            serde_json::to_vec(&Request {
                url: &server.url,
                identity: &server.identity,
                secret,
                cluster: server.cluster,
                action,
                id,
                ca_file: &server.ca_file,
            })
            .map_err(|_| failure("Hyper-V : requête invalide."))?,
        );
        #[cfg(windows)]
        let mut command = {
            use std::os::windows::process::CommandExt;
            let mut command = Command::new(executable);
            command.args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                include_str!("hyperv.ps1"),
            ]);
            command.creation_flags(0x08000000);
            command
        };
        #[cfg(not(windows))]
        let mut command = {
            let python = linux_program(&server.hyperv_python, &["python3"])?;
            let mut command = Command::new(python);
            command.args(["-I", "-c", include_str!("hyperv_linux.py")]);
            command
        };
        let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped())
            .stderr(Stdio::null()).spawn()
            .map_err(|_| failure("Hyper-V : impossible de lancer PowerShell / Python. Vérifiez le chemin configuré et les dépendances."))?;
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
        if status.code() == Some(10) {
            return Err(failure(
                "Hyper-V Linux : pypsrp absent. Installez pypsrp>=0.8.1,<0.9 dans un environnement Python et sélectionnez son interpréteur dans le profil.",
            ));
        }
        if cfg!(not(windows)) {
            match status.code() {
                Some(11) => {
                    return Err(failure(
                        "Hyper-V Linux : certificat WinRM refusé. Vérifiez le CA PEM et le nom DNS du serveur.",
                    ));
                }
                Some(12) => {
                    return Err(failure(
                        "Hyper-V Linux : authentification NTLM refusée. Vérifiez le compte, le domaine et la politique NTLM du serveur.",
                    ));
                }
                Some(13) => {
                    return Err(failure(
                        "Hyper-V Linux : hôte WinRM inaccessible. Vérifiez HTTPS, le port, le réseau et le pare-feu.",
                    ));
                }
                _ => {}
            }
        }
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
    #[cfg(not(windows))]
    {
        linux_program(&server.hyperv_freerdp, &["xfreerdp3", "xfreerdp"])
    }
    #[cfg(windows)]
    {
        if !server.vmconnect.as_os_str().is_empty() {
            return Ok(server.vmconnect.clone());
        }
        #[cfg(windows)]
        {
            let path = PathBuf::from(
                std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
            )
            .join("System32/vmconnect.exe");
            if path.is_file() {
                return Ok(path);
            }
        }
        Err(failure(
            "Installez les outils de gestion Hyper-V et configurez vmconnect.exe dans le profil du serveur.",
        ))
    }
}

#[cfg(not(windows))]
fn linux_program(configured: &std::path::Path, names: &[&str]) -> Result<PathBuf> {
    if !configured.as_os_str().is_empty() {
        if configured.is_absolute() && configured.is_file() {
            return Ok(configured.to_owned());
        }
        return Err(failure("Hyper-V Linux : chemin de programme invalide."));
    }
    crate::connections::find_program(names).ok_or_else(|| failure("Hyper-V Linux : Python ou FreeRDP introuvable. Configurez leurs chemins dans le profil du serveur."))
}

pub struct ConsoleLaunch {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub password: Option<crate::credentials::Secret>,
}
pub fn console_launch(server: &Server, vm: &Vm, secret: &str) -> Result<ConsoleLaunch> {
    let program = console_program(server)?;
    #[cfg(windows)]
    {
        let _ = secret;
        Ok(ConsoleLaunch {
            program,
            args: vec![vm.node.clone(), "-G".into(), vm.native_id.clone()],
            password: None,
        })
    }
    #[cfg(not(windows))]
    {
        let args = freerdp_arguments(vm, &server.identity)?;
        Ok(ConsoleLaunch {
            program,
            args,
            password: Some(crate::credentials::Secret::new(secret.into())),
        })
    }
}
#[cfg(any(not(windows), test))]
fn freerdp_arguments(vm: &Vm, identity: &str) -> Result<Vec<String>> {
    if uuid::Uuid::parse_str(&vm.native_id).is_err()
        || vm.node.is_empty()
        || !vm
            .node
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
        || identity.chars().any(char::is_control)
    {
        return Err(failure("Hyper-V : identité de console invalide."));
    }
    let (domain, user) = if let Some((domain, user)) = identity.split_once('\\') {
        (domain, user)
    } else if let Some((user, domain)) = identity.rsplit_once('@') {
        (domain, user)
    } else {
        (vm.node.split('.').next().unwrap_or(&vm.node), identity)
    };
    if domain.is_empty() || user.is_empty() {
        return Err(failure("Hyper-V : utilisateur et domaine requis."));
    }
    Ok(vec![
        format!("/v:{}:2179", vm.node),
        format!("/vmconnect:{}", vm.native_id),
        format!("/u:{user}"),
        format!("/d:{domain}"),
        "/cert:deny".into(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(windows))]
    #[test]
    fn linux_profile_and_console_keep_secrets_in_stdin() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let program = dir.path().join("fake freerdp");
        let arguments = dir.path().join("arguments");
        let input = dir.path().join("input");
        std::fs::write(
            &program,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\ncat > '{}'\n",
                arguments.display(),
                input.display()
            ),
        )
        .expect("script");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
            .expect("permissions");
        let mut server = Server {
            hyperv: true,
            name: "Test".into(),
            url: "https://node.example:5986".into(),
            identity: "LAB\\alice".into(),
            hyperv_freerdp: program.clone(),
            ..Default::default()
        };
        assert!(server.validate().is_ok());
        let restored: Server =
            serde_json::from_str(&serde_json::to_string(&server).expect("serialize"))
                .expect("deserialize");
        assert_eq!(restored.hyperv_freerdp, program);
        server.url = "http://node.example:5985".into();
        assert!(server.validate().is_err());
        server.url = "https://node.example:5986".into();
        let vm: Vm = serde_json::from_value(
            serde_json::json!({"vmid":0,"native_id":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            "name":"Test","node":"node.example","status":"running","kind":"HyperV"}),
        )
        .expect("vm");
        let plan = console_launch(&server, &vm, "synthetic-secret").expect("plan");
        let _client = crate::connections::rdp::RdpSession::launch_hyperv(plan).expect("launch");
        let started = std::time::Instant::now();
        while std::fs::read_to_string(&input).unwrap_or_default() != "synthetic-secret\n" {
            assert!(
                started.elapsed() < std::time::Duration::from_secs(5),
                "stdin not delivered"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let args = std::fs::read_to_string(arguments).expect("args");
        assert!(args.contains("/from-stdin:force"));
        assert!(!args.contains("synthetic-secret"));
    }
    #[test]
    fn linux_console_uses_host_guid_and_explicit_identity_without_password_args() {
        let vm = Vm {
            native_id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".into(),
            node: "node2.example.test".into(),
            vmid: 0,
            name: "Test".into(),
            kind: crate::proxmox::VmKind::HyperV,
            status: "running".into(),
            template: 0,
        };
        for (identity, user, domain) in [
            ("LAB\\alice", "alice", "LAB"),
            ("alice@example.test", "alice", "example.test"),
            ("alice", "alice", "node2"),
        ] {
            let args = freerdp_arguments(&vm, identity).expect("valid identity");
            assert!(args.contains(&"/v:node2.example.test:2179".into()));
            assert!(args.contains(&format!("/vmconnect:{}", vm.native_id)));
            assert!(args.contains(&format!("/u:{user}")));
            assert!(args.contains(&format!("/d:{domain}")));
            assert!(args.contains(&"/cert:deny".into()));
            assert!(!args.iter().any(|a| a.starts_with("/p:")));
        }
        assert!(freerdp_arguments(&vm, "LAB\\").is_err());
        assert!(freerdp_arguments(&vm, "alice\n/p:bad").is_err());
    }
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
