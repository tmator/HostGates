//! Import a documented subset; never forward the original file to a client.
use super::{Preview, map_row};
use crate::error::{AppError, Result};
use std::collections::BTreeMap;

pub fn endpoint(value: &str, default_port: u16) -> Result<(String, u16)> {
    let value = value.trim();
    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let (host, tail) = rest.split_once(']').ok_or_else(invalid)?;
        let port = if tail.is_empty() {
            default_port
        } else {
            tail.strip_prefix(':')
                .ok_or_else(invalid)?
                .parse()
                .map_err(|_| invalid())?
        };
        (host.to_owned(), port)
    } else if value.matches(':').count() == 1 {
        let (host, port) = value.split_once(':').ok_or_else(invalid)?;
        (host.to_owned(), port.parse().map_err(|_| invalid())?)
    } else {
        (value.to_owned(), default_port)
    };
    if host.is_empty()
        || host.len() > 253
        || host.starts_with('-')
        || port == 0
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-_:".contains(c))
    {
        return Err(invalid());
    }
    Ok((host, port))
}
fn invalid() -> AppError {
    AppError::Validation("Fichier RDP : paramètre invalide ou non pris en charge.".into())
}

pub fn parse(bytes: &[u8], name: &str) -> Result<Preview> {
    let text = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if !bytes.len().is_multiple_of(2) {
            return Err(invalid());
        }
        let le = bytes[0] == 0xff;
        let words = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| {
                if le {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect::<Vec<_>>();
        String::from_utf16(&words).map_err(|_| invalid())?
    } else {
        std::str::from_utf8(bytes)
            .map_err(|_| invalid())?
            .trim_start_matches('\u{feff}')
            .to_owned()
    };
    let mut fields = BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.splitn(3, ':');
        let key = parts
            .next()
            .ok_or_else(invalid)?
            .trim()
            .to_ascii_lowercase();
        let kind = parts.next().ok_or_else(invalid)?;
        let value = parts.next().ok_or_else(invalid)?.trim();
        if value.chars().any(char::is_control) || fields.insert(key, (kind, value)).is_some() {
            return Err(invalid());
        }
    }
    let get = |key: &str, kind: &str, default: &str| -> Result<String> {
        match fields.get(key) {
            Some((k, v)) if *k == kind => Ok((*v).to_owned()),
            Some(_) => Err(invalid()),
            None => Ok(default.to_owned()),
        }
    };
    if get("remoteapplicationmode", "i", "0")? != "0"
        || !get("alternate shell", "s", "")?.is_empty()
    {
        return Err(AppError::Validation(
            "Import RemoteApp / commande de démarrage non pris en charge.".into(),
        ));
    }
    let port = get("server port", "i", "3389")?
        .parse()
        .map_err(|_| invalid())?;
    let (host, port) = endpoint(&get("full address", "s", "")?, port)?;
    let mut row = map_row(
        1,
        name,
        "",
        "RDP",
        &host,
        &port.to_string(),
        &get("username", "s", "")?,
    );
    row.rdp_configuration = true;
    row.rdp_routing.load_balance_info = get("loadbalanceinfo", "s", "")?;
    row.rdp_routing.use_redirection_server_name =
        match get("use redirection server name", "i", "0")?.as_str() {
            "0" => false,
            "1" => true,
            _ => return Err(invalid()),
        };
    row.rdp_routing.validate()?;
    let data = row.data.as_mut().ok_or_else(invalid)?;
    data.domain = get("domain", "s", "")?;
    data.validate()?;
    let gateway_host = get("gatewayhostname", "s", "")?;
    let usage = get(
        "gatewayusagemethod",
        "i",
        if gateway_host.is_empty() { "0" } else { "1" },
    )?;
    match usage.as_str() {
        "0" | "4" => {}
        "1" | "2" => {
            let gateway = crate::connections::rdp::Gateway {
                rpc_transport: false,
                host: gateway_host,
                username: get("gatewayusername", "s", "")?,
                usage: usage.parse().map_err(|_| invalid())?,
                credential_source: get("gatewaycredentialssource", "i", "4")?
                    .parse()
                    .map_err(|_| invalid())?,
            };
            gateway.validate()?;
            row.rdp_gateway = Some(gateway);
        }
        _ => return Err(AppError::Validation(
            "RDP : configurez une passerelle explicite (mode 1 ou 2), ou désactivez-la (mode 0)."
                .into(),
        )),
    }
    let mut warnings = vec!["Import RDP : adresse, port, utilisateur, domaine, RD Gateway et routage RDS. Mots de passe, signatures, redirections de périphériques, affichage et autres paramètres ne sont pas repris.".into()];
    if let Some(gateway) = &row.rdp_gateway {
        warnings.push(format!(
            "RD Gateway : {} ; utilisateur : {} ; mode : {}.",
            gateway.host, gateway.username, gateway.usage
        ));
        warnings.push(
            "RD Gateway : connexion dans un onglet par défaut ; authentification de la passerelle à renseigner avant connexion."
                .into(),
        );
    }
    if !row.rdp_routing.load_balance_info.is_empty() || row.rdp_routing.use_redirection_server_name
    {
        warnings.push("RDS : routage de collection et redirections sous TLS pris en charge via la même passerelle. Le certificat du serveur cible est vérifié. Modes carte à puce et RDSTLS non pris en charge.".into());
    }
    Ok(Preview {
        rows: vec![row],
        warnings,
    })
}
