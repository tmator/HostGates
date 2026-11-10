//! The subset HostGates needs: HTTP(S)/NTLM WinRM, fixed PowerShell commands,
//! JSON inventory and bounded execution. No Python or local PowerShell process.
use super::failure;
use crate::{
    error::Result,
    proxmox::{Server, Vm, VmKind},
};
use std::time::{Duration, Instant};
use winrm_rs::{
    AuthMethod, EncryptionMode, WinrmClient, WinrmConfig, WinrmCredentials, WinrmError,
};

const LIMIT: usize = 4 * 1024 * 1024;
const CLUSTER: &str = include_str!("hyperv_cluster.ps1");
const INVENTORY: &str = include_str!("hyperv_inventory.ps1");
const ACTION: &str = include_str!("hyperv_action.ps1");

fn credentials(identity: &str, secret: &str) -> WinrmCredentials {
    let (domain, user) = if let Some((domain, user)) = identity.split_once('\\') {
        (domain, user)
    } else if let Some((user, domain)) = identity.rsplit_once('@') {
        (domain, user)
    } else {
        ("", identity)
    };
    WinrmCredentials::new(user, secret, domain)
}

fn client(server: &Server, secret: &str) -> Result<WinrmClient> {
    let url = url::Url::parse(&server.url).map_err(|_| failure("Hyper-V : URL invalide."))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(failure("Hyper-V natif : HTTP ou HTTPS est requis."));
    }
    let tls = url.scheme() == "https";
    let config = WinrmConfig {
        port: Some(url.port().unwrap_or(if tls { 5986 } else { 5985 })),
        use_tls: tls,
        encryption: if tls {
            EncryptionMode::Auto
        } else {
            EncryptionMode::Always
        },
        accept_invalid_certs: tls && server.skip_tls_verification,
        auth_method: AuthMethod::Ntlm,
        connect_timeout_secs: 15,
        operation_timeout_secs: 20,
        max_retries: 0,
        max_output_bytes: Some(LIMIT),
        idle_timeout_secs: Some(60),
        ..Default::default()
    };
    let mut builder = WinrmClient::builder(config)
        .credentials(credentials(&server.identity, secret))
        .use_env_proxy(false)
        .platform_roots(true)
        .log_soap_bodies(false)
        .non_interactive_powershell(true);
    if tls && !server.skip_tls_verification && !server.ca_file.as_os_str().is_empty() {
        builder = builder.add_root_certificates_pem(&server.ca_file);
    }
    builder.build().map_err(|_| {
        failure("Hyper-V natif : configuration TLS invalide. Vérifiez le certificat CA PEM.")
    })
}

fn remote_error(error: WinrmError) -> crate::error::AppError {
    // The upstream errors may include server-controlled text. Never display it.
    match error {
        WinrmError::Timeout(_) | WinrmError::Cancelled => failure(
            "Hyper-V : délai dépassé. Une commande distante peut encore se terminer ; actualisez avant de réessayer.",
        ),
        WinrmError::Http(_) => failure(
            "Hyper-V natif : connexion WinRM impossible. Vérifiez HTTP/HTTPS, le port, le réseau et, en HTTPS, le certificat.",
        ),
        WinrmError::AuthFailed(message) => failure(&auth_diagnostic(&message)),
        WinrmError::Ntlm(winrm_rs::NtlmError::InvalidMessage(message)) => {
            failure(&ntlm_diagnostic(&message))
        }
        _ => failure(
            "Hyper-V natif : opération WinRM refusée. Vérifiez les droits WinRM/Hyper-V et les modules serveur.",
        ),
    }
}

// Translate only known library-generated categories. Never echo server response bodies,
// authentication headers, challenge blobs or arbitrary upstream error strings.
fn auth_diagnostic(message: &str) -> String {
    if message == "NTLM authentication rejected (bad credentials or CBT mismatch)" {
        return "Hyper-V [NTLM-AUTH-401] : le serveur a rejeté la réponse d’authentification. Vérifiez le nom NetBIOS du domaine, le compte, le mot de passe et les événements Windows 4625. En HTTPS, vérifier aussi la liaison au certificat (CBT).".into();
    }
    if let Some(status) = message
        .strip_prefix("expected 401 for NTLM negotiate, got ")
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|s| (100..=599).contains(s))
    {
        return format!(
            "Hyper-V [NTLM-NEGOTIATE-HTTP-{status}] : réponse inattendue avant authentification. Vérifiez le port WinRM, /wsman et un éventuel proxy HTTP."
        );
    }
    match message {
        "missing WWW-Authenticate header in 401"=>"Hyper-V [NTLM-NO-CHALLENGE] : réponse HTTP 401 sans méthode d’authentification. Vérifiez WinRM/Negotiate et le proxy.",
        "missing Negotiate prefix in challenge"=>"Hyper-V [NTLM-CHALLENGE-SCHEME] : la réponse ne contient pas le défi Negotiate attendu. Vérifiez les méthodes WinRM proposées et les en-têtes du proxy.",
        _=>"Hyper-V [NTLM-AUTH-PROTOCOL] : échange d’authentification incompatible ; aucun détail serveur sensible n’est affiché.",
    }.into()
}
fn ntlm_diagnostic(message: &str) -> String {
    for flag in [
        "NEGOTIATE_UNICODE",
        "NEGOTIATE_NTLM",
        "NEGOTIATE_EXTENDED_SESSIONSECURITY",
        "NEGOTIATE_128",
        "NEGOTIATE_SIGN",
        "NEGOTIATE_SEAL",
        "NEGOTIATE_KEY_EXCH",
    ] {
        if message == format!("NTLM Type 2: the server did not negotiate {flag}") {
            return format!(
                "Hyper-V [NTLM-FLAG-{flag}] : le serveur ne propose pas cette capacité NTLM requise. La réponse d’authentification n’a pas été envoyée. Vérifiez les politiques NTLM/signature/chiffrement ; ne désactivez pas le chiffrement HTTP."
            );
        }
    }
    for field in ["MsvAvNbComputerName", "MsvAvNbDomainName"] {
        if message == format!("NTLM Type 2: target info has no {field}") {
            return format!(
                "Hyper-V [NTLM-TARGET-{field}] : défi serveur incomplet pour le chiffrement WinRM. La réponse d’authentification n’a pas été envoyée."
            );
        }
    }
    if message.starts_with("NTLM over TLS:") || message.contains("channel binding") {
        return "Hyper-V [NTLM-CBT] : impossible de lier l’authentification au certificat de cette connexion TLS.".into();
    }
    if message.starts_with("HTTP 401 to a sealed NTLM request") {
        return "Hyper-V [NTLM-SESSION-401] : session authentifiée rejetée lors d’une commande. La commande n’a pas été relancée ; actualisez avant toute nouvelle action.".into();
    }
    if message.contains("closed") && message.contains("authentication") {
        return "Hyper-V [NTLM-CONNECTION-CLOSED] : connexion fermée pendant/après authentification. Vérifiez le maintien de connexion par WinRM ou le proxy.".into();
    }
    if message.contains("unsealed")
        || message.contains("seal")
        || message.contains("signature")
        || message.contains("checksum")
    {
        return "Hyper-V [NTLM-MESSAGE-INTEGRITY] : réponse chiffrée/signée incompatible ou non vérifiable. Aucune rétrogradation vers des commandes HTTP en clair.".into();
    }
    "Hyper-V [NTLM-INVALID-CHALLENGE] : négociation NTLM invalide ou incompatible ; ce message ne prouve pas un mauvais mot de passe.".into()
}

pub(super) fn run(server: &Server, secret: &str, action: &str, id: &str) -> Result<Vec<Vm>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| failure("Hyper-V : moteur réseau indisponible."))?;
    let deadline = Instant::now() + Duration::from_secs(120);
    run_with(server, action, id, |node, script| {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(remote_error(WinrmError::Timeout(120)));
        }
        // A client per target avoids sharing authentication context between nodes.
        let client = client(server, secret)?;
        let script = format!(
            "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); $ProgressPreference='SilentlyContinue'; try {{ {script} }} catch {{ exit 1 }}"
        );
        let output = runtime
            .block_on(client.run_powershell_with_timeout(node, &script, remaining))
            .map_err(remote_error)?;
        if output.exit_code != 0 || output.stdout.len() > LIMIT {
            return Err(failure(
                "Hyper-V : commande refusée ou résultat trop volumineux. Aucun arrêt forcé de secours n’a été demandé.",
            ));
        }
        Ok(output.stdout)
    })
}

fn node_valid(node: &str) -> bool {
    !node.is_empty()
        && node.len() <= 253
        && node
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
}

fn run_with(
    server: &Server,
    action: &str,
    id: &str,
    mut invoke: impl FnMut(&str, &str) -> Result<Vec<u8>>,
) -> Result<Vec<Vm>> {
    if !matches!(
        action,
        "list" | "console" | "start" | "shutdown" | "reboot" | "stop"
    ) {
        return Err(failure("Hyper-V : action invalide."));
    }
    let target = if action == "list" {
        None
    } else {
        Some(uuid::Uuid::parse_str(id).map_err(|_| failure("Hyper-V : identifiant invalide."))?)
    };
    let url = url::Url::parse(&server.url).map_err(|_| failure("Hyper-V : URL invalide."))?;
    let host = url
        .host_str()
        .ok_or_else(|| failure("Hyper-V : hôte absent."))?;
    if !matches!(url.scheme(), "http" | "https") || !node_valid(host) {
        return Err(failure("Hyper-V natif : hôte WinRM invalide."));
    }
    let nodes: Vec<String> = if server.cluster {
        serde_json::from_slice(&invoke(host, CLUSTER)?)
            .map_err(|_| failure("Hyper-V : inventaire du cluster invalide."))?
    } else {
        vec![host.into()]
    };
    let mut seen = std::collections::HashSet::new();
    if nodes.is_empty()
        || nodes.len() > 64
        || nodes
            .iter()
            .any(|n| !node_valid(n) || !seen.insert(n.to_ascii_lowercase()))
    {
        return Err(failure("Hyper-V : liste des nœuds invalide."));
    }
    let mut inventory = Vec::new();
    for node in nodes {
        let mut vms: Vec<Vm> = serde_json::from_slice(&invoke(&node, INVENTORY)?)
            .map_err(|_| failure("Hyper-V : inventaire invalide."))?;
        for vm in &mut vms {
            let guid = uuid::Uuid::parse_str(&vm.native_id)
                .map_err(|_| failure("Hyper-V : identifiant de VM invalide."))?;
            if vm.kind != VmKind::HyperV
                || vm.name.len() > 256
                || vm.name.chars().any(char::is_control)
            {
                return Err(failure("Hyper-V : machine invalide."));
            }
            vm.native_id = guid.to_string();
            vm.node = node.clone();
        }
        inventory.extend(vms);
        if inventory.len() > 10000 {
            return Err(failure("Hyper-V : inventaire trop volumineux."));
        }
    }
    inventory.sort_by(|a, b| a.name.cmp(&b.name).then(a.native_id.cmp(&b.native_id)));
    let Some(target) = target else {
        return Ok(inventory);
    };
    let mut selected: Vec<_> = inventory
        .into_iter()
        .filter(|v| v.native_id == target.to_string())
        .collect();
    if selected.len() != 1 {
        return Err(failure(
            "Hyper-V : VM déplacée ou indisponible. Actualisez l’inventaire.",
        ));
    }
    let vm = selected.remove(0);
    if action != "console" {
        // Only a parsed UUID and an allowlisted action enter this fixed script.
        invoke(
            &vm.node,
            &format!("$ErrorActionPreference='Stop'; $id='{target}'; $action='{action}'; {ACTION}"),
        )?;
    }
    Ok(vec![vm])
}

#[cfg(test)]
#[path = "hyperv_native_tests.rs"]
mod tests;
