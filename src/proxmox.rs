//! Proxmox API and restricted SPICE connection files. Secrets never implement Debug.
use crate::{
    credentials::Secret,
    error::{AppError, Result},
};
use reqwest::{
    blocking::{Client, Response},
    header::{AUTHORIZATION, COOKIE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::{io::Read, path::PathBuf, time::Duration};
use zeroize::Zeroizing;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMode {
    #[default]
    Password,
    ApiToken,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Server {
    pub hyperv: bool,
    pub cluster: bool,
    pub vmconnect: PathBuf,
    pub hyperv_freerdp: PathBuf,
    pub id: String,
    pub name: String,
    pub url: String,
    pub identity: String,
    pub auth: AuthMode,
    pub credential_id: Option<String>,
    pub ca_file: PathBuf,
    /// Explicit per-server exception; never enabled for existing profiles.
    pub skip_tls_verification: bool,
    /// Optional hostname/IP reachable by the viewer, not an arbitrary URL.
    pub proxy_host: String,
}
impl Default for Server {
    fn default() -> Self {
        Self {
            hyperv: false,
            cluster: false,
            vmconnect: PathBuf::new(),
            hyperv_freerdp: PathBuf::new(),
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            url: "https://".into(),
            identity: String::new(),
            auth: AuthMode::default(),
            credential_id: None,
            ca_file: PathBuf::new(),
            skip_tls_verification: false,
            proxy_host: String::new(),
        }
    }
}
fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}
fn failure(message: &str) -> AppError {
    AppError::Operation(message.into())
}
impl Server {
    pub fn validate(&self) -> Result<url::Url> {
        crate::models::validate_id(&self.id)?;
        crate::models::validate_name(&self.name)?;
        if let Some(id) = &self.credential_id {
            crate::models::validate_id(id)?;
        }
        if self.hyperv {
            return crate::hyperv::validate(self);
        }
        let url = url::Url::parse(self.url.trim())
            .map_err(|_| invalid("Proxmox : URL HTTPS invalide."))?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !matches!(url.path(), "" | "/")
        {
            return Err(invalid(
                "Proxmox : utilisez https://serveur:8006 sans identifiants ni chemin.",
            ));
        }
        let id = self.identity.trim();
        if id.is_empty()
            || id.len() > 256
            || id
                .chars()
                .any(|c| c.is_control() || c.is_whitespace() || c == '=')
            || !id.contains('@')
            || (self.auth == AuthMode::ApiToken && !id.contains('!'))
            || (self.auth == AuthMode::Password && id.contains('!'))
        {
            return Err(invalid(
                "Proxmox : renseignez utilisateur@realm ou utilisateur@realm!jeton.",
            ));
        }
        if !self.proxy_host.is_empty() {
            validate_proxy_host(&self.proxy_host)?;
        }
        if !self.skip_tls_verification
            && !self.ca_file.as_os_str().is_empty()
            && (!self.ca_file.is_absolute() || !self.ca_file.is_file())
        {
            return Err(invalid(
                "Proxmox : sélectionnez un certificat CA PEM existant.",
            ));
        }
        Ok(url)
    }
}
fn validate_proxy_host(host: &str) -> Result<()> {
    let host = host.trim_matches(['[', ']']);
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Ok(());
    }
    if host.is_empty()
        || host.len() > 253
        || host.starts_with('-')
        || host
            .split('.')
            .any(|s| s.is_empty() || s.len() > 63 || s.starts_with('-') || s.ends_with('-'))
        || !host
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-'))
    {
        return Err(invalid(
            "Proxmox : hôte du proxy SPICE invalide (nom DNS ou IP, sans port).",
        ));
    }
    Ok(())
}
fn node_valid(node: &str) -> bool {
    !node.is_empty()
        && !matches!(node, "." | "..")
        && node.len() <= 253
        && node
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_'))
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum VmKind {
    #[default]
    Qemu,
    Lxc,
    HyperV,
}
impl VmKind {
    pub fn api_name(self) -> &'static str {
        match self {
            Self::Qemu => "qemu",
            Self::Lxc => "lxc",
            Self::HyperV => "hyperv",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerAction {
    Start,
    Shutdown,
    Reboot,
    PowerOff,
}
impl PowerAction {
    pub fn api_name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Shutdown => "shutdown",
            Self::Reboot => "reboot",
            Self::PowerOff => "stop",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Start => "Démarrer",
            Self::Shutdown => "Arrêter proprement",
            Self::Reboot => "Redémarrer proprement",
            Self::PowerOff => "Couper l’alimentation",
        }
    }
    pub fn available(self, vm: &Vm) -> bool {
        vm.template == 0
            && match self {
                Self::Start => matches!(vm.status.as_str(), "stopped" | "saved"),
                _ => vm.status == "running",
            }
    }
}
#[derive(Clone, Debug, Deserialize)]
pub struct Vm {
    #[serde(default)]
    pub kind: VmKind,
    #[serde(default)]
    pub native_id: String,
    pub vmid: u32,
    #[serde(default)]
    pub name: String,
    pub node: String,
    pub status: String,
    #[serde(default)]
    pub template: u8,
}
impl Vm {
    pub fn state_label(&self) -> &'static str {
        if self.template != 0 {
            return "Modèle";
        }
        match self.status.as_str() {
            "disabled" => "Désactivée",
            "running" => "Allumée",
            "stopped" => "Arrêtée",
            "paused" => "En pause",
            "saved" => "Sauvegardée",
            "starting" | "stopping" | "saving" => "En transition",
            _ => "Indisponible",
        }
    }
    pub fn label(&self) -> String {
        if self.kind == VmKind::HyperV {
            return self.name.clone();
        }
        format!(
            "{} · {}",
            self.vmid,
            if self.name.is_empty() {
                "VM"
            } else {
                &self.name
            }
        )
    }
    pub fn connectable(&self) -> bool {
        self.status == "running" && self.template == 0
    }
}
#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}
#[derive(Deserialize)]
struct Resource {
    #[serde(rename = "type")]
    kind: String,
    #[serde(flatten)]
    vm: serde_json::Value,
}
pub fn parse_vms(value: serde_json::Value) -> Result<Vec<Vm>> {
    let rows: Vec<Resource> =
        serde_json::from_value(value).map_err(|_| failure("Proxmox : liste des VM invalide."))?;
    let mut result = Vec::new();
    for row in rows
        .into_iter()
        .filter(|r| matches!(r.kind.as_str(), "qemu" | "lxc"))
    {
        let mut vm: Vm =
            serde_json::from_value(row.vm).map_err(|_| failure("Proxmox : VM invalide."))?;
        vm.kind = if row.kind == "lxc" {
            VmKind::Lxc
        } else {
            VmKind::Qemu
        };
        if vm.vmid == 0
            || !node_valid(&vm.node)
            || vm.name.len() > 256
            || vm.name.chars().any(char::is_control)
        {
            return Err(failure("Proxmox : VM invalide."));
        }
        result.push(vm);
    }
    result.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.vmid.cmp(&b.vmid))
    });
    Ok(result)
}

pub struct Api {
    auth: AuthMode,
    client: Client,
    base: url::Url,
    headers: HeaderMap,
}
fn network_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        failure("Proxmox : délai de connexion dépassé.")
    } else {
        failure(
            "Proxmox : connexion HTTPS impossible. Vérifiez l’adresse, le réseau et le certificat CA (nom du serveur compris).",
        )
    }
}
fn read_data<T: serde::de::DeserializeOwned>(
    response: Response,
    operation: &str,
    auth: Option<AuthMode>,
) -> Result<T> {
    match response.status().as_u16() {
        200..=299 => {}
        401 => {
            let explanation = match auth {
                None => {
                    "Connexion refusée. Vérifiez le compte complet (ex. root@pam), le mot de passe et les exigences MFA. Dans Modifier, ressaisissez le secret pour remplacer celui du coffre."
                }
                Some(AuthMode::Password) => {
                    "Ticket refusé après obtention du ticket de connexion. Vérifiez le reverse proxy et les journaux Proxmox ; le cookie peut être supprimé ou le ticket invalidé."
                }
                Some(AuthMode::ApiToken) => {
                    "Jeton API refusé. Vérifiez utilisateur@realm!nom-du-jeton, son secret, son expiration et la transmission de l’en-tête Authorization par le reverse proxy."
                }
            };
            return Err(failure(&format!(
                "Proxmox : HTTP 401 [{operation}]. {explanation}"
            )));
        }
        403 => {
            return Err(failure(
                "Proxmox : droits insuffisants (VM.Audit / VM.Console / VM.PowerMgmt selon l’action).",
            ));
        }
        _ => {
            return Err(failure(
                "Proxmox : requête refusée. Vérifiez les droits, l’état et la configuration de la machine.",
            ));
        }
    }
    let mut bytes = Zeroizing::new(Vec::new());
    response
        .take(2_097_153)
        .read_to_end(&mut bytes)
        .map_err(|_| failure("Proxmox : réponse interrompue."))?;
    if bytes.len() > 2_097_152 {
        return Err(failure("Proxmox : réponse trop volumineuse."));
    }
    let result: Envelope<T> = serde_json::from_slice(&bytes).map_err(|e| {
        let kind = if e.is_data() {
            "format de réponse incompatible"
        } else {
            "réponse JSON invalide"
        };
        // Do not print serde's error: it can contain ticket or credential values.
        failure(&format!("Proxmox : {kind} [{operation}]."))
    })?;
    Ok(result.data)
}
fn secret_header(value: &str) -> Result<HeaderValue> {
    let mut header = HeaderValue::from_str(value)
        .map_err(|_| invalid("Proxmox : identifiant ou secret invalide."))?;
    header.set_sensitive(true);
    Ok(header)
}
#[derive(Deserialize)]
struct Ticket {
    ticket: Secret,
    #[serde(rename = "CSRFPreventionToken")]
    csrf: Secret,
    #[serde(rename = "NeedTFA", default)]
    need_tfa: Option<serde_json::Value>,
}
impl Api {
    pub fn login(server: &Server, secret: &str) -> Result<Self> {
        if server.hyperv {
            return Err(invalid("Utilisez le connecteur Hyper-V pour ce profil."));
        }
        let base = server.validate()?;
        if secret.is_empty() || secret.len() > 16_384 || secret.contains(['\r', '\n', '\0']) {
            return Err(invalid("Proxmox : secret manquant ou invalide."));
        }
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .https_only(true)
            .danger_accept_invalid_certs(server.skip_tls_verification);
        if !server.skip_tls_verification && !server.ca_file.as_os_str().is_empty() {
            let file = std::fs::File::open(&server.ca_file)
                .map_err(|_| invalid("Proxmox : certificat CA illisible."))?;
            let mut pem = Vec::new();
            file.take(262_145)
                .read_to_end(&mut pem)
                .map_err(|_| invalid("Proxmox : certificat CA illisible."))?;
            if pem.len() > 262_144 {
                return Err(invalid("Proxmox : certificat CA trop volumineux."));
            }
            let certs = reqwest::Certificate::from_pem_bundle(&pem)
                .map_err(|_| invalid("Proxmox : certificat CA PEM invalide."))?;
            if certs.is_empty() {
                return Err(invalid("Proxmox : certificat CA PEM invalide."));
            }
            for cert in certs {
                builder = builder.add_root_certificate(cert);
            }
        }
        let client = builder.build().map_err(network_error)?;
        let mut api = Self {
            auth: server.auth,
            client,
            base,
            headers: HeaderMap::new(),
        };
        match server.auth {
            AuthMode::ApiToken => {
                let value =
                    Zeroizing::new(format!("PVEAPIToken={}={secret}", server.identity.trim()));
                api.headers.insert(AUTHORIZATION, secret_header(&value)?);
            }
            AuthMode::Password => {
                let response = api
                    .client
                    .post(api.endpoint("access/ticket")?)
                    .form(&[("username", server.identity.trim()), ("password", secret)])
                    .send()
                    .map_err(network_error)?;
                let ticket: Ticket = read_data(response, "POST /access/ticket", None)?;
                if ticket.ticket.starts_with("PVE:!tfa!")
                    || ticket
                        .need_tfa
                        .is_some_and(|v| v != false && v != 0 && !v.is_null())
                {
                    return Err(failure(
                        "Proxmox : ce compte exige une MFA. Utilisez un jeton API dédié avec les droits VM.Audit et VM.Console.",
                    ));
                }
                let cookie = Zeroizing::new(format!("PVEAuthCookie={}", ticket.ticket.as_str()));
                api.headers.insert(COOKIE, secret_header(&cookie)?);
                api.headers
                    .insert("CSRFPreventionToken", secret_header(&ticket.csrf)?);
            }
        }
        Ok(api)
    }
    fn endpoint(&self, path: &str) -> Result<url::Url> {
        self.base
            .join(&format!("api2/json/{path}"))
            .map_err(|_| invalid("Proxmox : URL invalide."))
    }
    pub fn vms(&self) -> Result<Vec<Vm>> {
        let response = self
            .client
            .get(self.endpoint("cluster/resources")?)
            .query(&[("type", "vm")])
            .headers(self.headers.clone())
            .send()
            .map_err(network_error)?;
        parse_vms(read_data(
            response,
            "GET /cluster/resources",
            Some(self.auth),
        )?)
    }
    fn locate(&self, selected: &Vm) -> Result<Vm> {
        self.vms()?
            .into_iter()
            .find(|vm| vm.vmid == selected.vmid && vm.kind == selected.kind)
            .ok_or_else(|| failure("Proxmox : machine introuvable ou non autorisée."))
    }
    pub fn power(&self, selected: &Vm, action: PowerAction) -> Result<String> {
        let vm = self.locate(selected)?;
        if !action.available(&vm) {
            return Err(failure(
                "Action indisponible dans l’état actuel de la machine. Actualisez la liste.",
            ));
        }
        let path = format!(
            "nodes/{}/{}/{}/status/{}",
            vm.node,
            vm.kind.api_name(),
            vm.vmid,
            action.api_name()
        );
        let response = self
            .client
            .post(self.endpoint(&path)?)
            .headers(self.headers.clone())
            .send()
            .map_err(network_error)?;
        // Proxmox returns a task identifier: acceptance is not completion.
        let task: String = read_data(response, "POST power action", Some(self.auth))?;
        if !task.starts_with("UPID:") {
            return Err(failure(
                "Proxmox : identifiant de tâche invalide. Vérifiez l’état avant de réessayer.",
            ));
        }
        Ok(task)
    }
    pub fn wait_power_task(&self, task: &str) -> Result<()> {
        let fields: Vec<_> = task.split(':').collect();
        if fields.len() < 3
            || !node_valid(fields[1])
            || !task.bytes().all(|b| {
                b.is_ascii_alphanumeric() || matches!(b, b':' | b'-' | b'_' | b'.' | b'@' | b'!')
            })
        {
            return Err(failure(
                "Proxmox : suivi de tâche impossible. Vérifiez le résultat dans Proxmox.",
            ));
        }
        let started = std::time::Instant::now();
        loop {
            let response = self
                .client
                .get(self.endpoint(&format!("nodes/{}/tasks/{task}/status", fields[1]))?)
                .headers(self.headers.clone())
                .send()
                .map_err(network_error)?;
            let status: serde_json::Value =
                read_data(response, "GET task status", Some(self.auth))?;
            if status["status"] == "stopped" {
                return if status["exitstatus"] == "OK" {
                    Ok(())
                } else {
                    Err(failure(
                        "Proxmox : la tâche a échoué. Consultez son journal dans Proxmox ; aucun arrêt forcé de secours n’a été demandé.",
                    ))
                };
            }
            if started.elapsed() >= Duration::from_secs(90) {
                return Err(failure(
                    "Proxmox : tâche encore en cours. Actualisez ou consultez Proxmox avant de réessayer.",
                ));
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    pub(crate) fn lxc_terminal(
        &self,
        server: &Server,
        selected: &Vm,
    ) -> Result<crate::connections::proxmox_terminal::Console> {
        let vm = self.locate(selected)?;
        if vm.kind != VmKind::Lxc || !vm.connectable() {
            return Err(failure("Démarrez le conteneur avant d’ouvrir sa console."));
        }
        let path = format!("nodes/{}/lxc/{}", vm.node, vm.vmid);
        let response = self
            .client
            .post(self.endpoint(&format!("{path}/termproxy"))?)
            .headers(self.headers.clone())
            .send()
            .map_err(network_error)?;
        let ticket: crate::connections::proxmox_terminal::Ticket =
            read_data(response, "POST LXC termproxy (VM.Console)", Some(self.auth))?;
        crate::connections::proxmox_terminal::connect(
            server,
            self.endpoint(&format!("{path}/vncwebsocket"))?,
            self.headers.clone(),
            ticket,
        )
    }
    pub fn lxc_addresses(&self, selected: &Vm) -> Result<Vec<String>> {
        let vm = self.locate(selected)?;
        if vm.kind != VmKind::Lxc || !vm.connectable() {
            return Err(failure("Démarrez le conteneur avant la connexion SSH."));
        }
        let response = self
            .client
            .get(self.endpoint(&format!("nodes/{}/lxc/{}/interfaces", vm.node, vm.vmid))?)
            .headers(self.headers.clone())
            .send()
            .map_err(network_error)?;
        let rows: Vec<serde_json::Value> =
            read_data(response, "GET LXC interfaces", Some(self.auth))?;
        Ok(interface_addresses(&rows))
    }
    pub fn console(&self, server: &Server, vmid: u32) -> Result<SpiceConfig> {
        // Refresh the node at connect time: a VM may have migrated since the last listing.
        let vm = self
            .vms()?
            .into_iter()
            .find(|v| v.vmid == vmid)
            .ok_or_else(|| failure("Proxmox : VM introuvable ou non autorisée."))?;
        if vm.kind != VmKind::Qemu || !vm.connectable() {
            return Err(failure(
                "Proxmox : démarrez la VM depuis Proxmox avant d’ouvrir sa console.",
            ));
        }
        let prefix = format!("nodes/{}/qemu/{vmid}", vm.node);
        let response = self
            .client
            .get(self.endpoint(&format!("{prefix}/status/current"))?)
            .headers(self.headers.clone())
            .send()
            .map_err(network_error)?;
        let status: serde_json::Value = read_data(
            response,
            "GET /nodes/{node}/qemu/{vmid}/status/current",
            Some(self.auth),
        )?;
        if status.get("spice").is_none_or(|v| v != true && v != 1) {
            return Err(failure(
                "Proxmox : activez un affichage SPICE pour cette VM dans Matériel > Affichage.",
            ));
        }
        let proxy = if server.proxy_host.is_empty() {
            self.base
                .host_str()
                .unwrap_or_default()
                .trim_matches(['[', ']'])
        } else {
            server.proxy_host.trim_matches(['[', ']'])
        };
        let response = self
            .client
            .post(self.endpoint(&format!("{prefix}/spiceproxy"))?)
            .headers(self.headers.clone())
            .form(&[("proxy", proxy)])
            .send()
            .map_err(network_error)?;
        read_data(
            response,
            "POST /nodes/{node}/qemu/{vmid}/spiceproxy",
            Some(self.auth),
        )
    }
}

#[derive(Deserialize)]
pub struct SpiceConfig {
    #[serde(rename = "type")]
    kind: String,
    host: Secret,
    proxy: String,
    #[serde(rename = "tls-port", deserialize_with = "port_from_json")]
    tls_port: u16,
    #[serde(rename = "host-subject")]
    subject: String,
    ca: String,
    password: Secret,
}
pub(crate) fn port_from_json<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u16, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    let port = value
        .as_u64()
        .and_then(|v| u16::try_from(v).ok())
        .or_else(|| value.as_str().and_then(|v| v.parse().ok()));
    port.filter(|p| *p > 0)
        .ok_or_else(|| serde::de::Error::custom("invalid port"))
}
fn scalar(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 16_384 || value.chars().any(char::is_control) {
        return Err(failure("SPICE : configuration de console invalide."));
    }
    Ok(())
}
fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
fn valid_ca_pem(pem: &[u8]) -> bool {
    use rustls::pki_types::pem::PemObject;
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls::pki_types::CertificateDer::pem_slice_iter(pem) {
        let Ok(cert) = cert else {
            return false;
        };
        if roots.add(cert).is_err() {
            return false;
        }
    }
    !roots.is_empty()
}
impl SpiceConfig {
    /// Run on the connection worker, never on the UI thread. No tickets are sent.
    pub fn check_proxy(&self) -> Result<()> {
        use std::net::{TcpStream, ToSocketAddrs};
        self.file_text()?;
        let proxy = url::Url::parse(&self.proxy).map_err(|_| failure("SPICE : proxy invalide."))?;
        let host = proxy
            .host_str()
            .unwrap_or_default()
            .trim_matches(['[', ']']);
        let port = proxy.port_or_known_default().unwrap_or(3128);
        let addresses = (host, port).to_socket_addrs().map_err(|_| failure(
            "SPICE : nom du proxy introuvable. Configurez son adresse accessible dans le profil Proxmox."))?;
        let deadline = std::time::Instant::now() + Duration::from_secs(4);
        for address in addresses.take(4) {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            if TcpStream::connect_timeout(&address, remaining.min(Duration::from_secs(1))).is_ok() {
                return Ok(());
            }
        }
        Err(failure(&format!(
            "SPICE : proxy {host}:{port} inaccessible. Vérifiez le port SPICE (habituellement 3128), le VPN et le champ Hôte du proxy SPICE. L’accès HTTPS à Proxmox ne suffit pas."
        )))
    }
    pub fn file_text(&self) -> Result<Secret> {
        if self.kind != "spice" {
            return Err(failure("SPICE : type de console invalide."));
        }
        scalar(&self.host)?;
        scalar(&self.password)?;
        scalar(&self.subject)?;
        scalar(&self.proxy)?;
        let proxy = url::Url::parse(&self.proxy).map_err(|_| failure("SPICE : proxy invalide."))?;
        if !["http", "https"].contains(&proxy.scheme())
            || proxy.host_str().is_none()
            || !proxy.username().is_empty()
            || proxy.password().is_some()
            || proxy.query().is_some()
            || proxy.fragment().is_some()
            || !matches!(proxy.path(), "" | "/")
        {
            return Err(failure("SPICE : proxy invalide."));
        }
        let ca = self.ca.replace("\\n", "\n");
        if ca.len() > 262_144 || !valid_ca_pem(ca.as_bytes()) {
            return Err(failure("SPICE : certificat CA invalide."));
        }
        // Only allowlisted fields reach remote-viewer. Never pass server-provided commands,
        // arbitrary file paths, redirection options or TLS-verification overrides.
        Ok(Zeroizing::new(format!(
            "[virt-viewer]\ntype=spice\nhost={}\nproxy={}\nsecure-channels=all\ntls-port={}\nhost-subject={}\nca={}\npassword={}\ndelete-this-file=1\n",
            escape(&self.host),
            escape(&self.proxy),
            self.tls_port,
            escape(&self.subject),
            escape(&ca),
            escape(&self.password)
        )))
    }
    pub fn launch(&self, program: &std::path::Path) -> Result<crate::connections::rdp::RdpSession> {
        let text = self.file_text()?;
        let file = crate::connections::secret_file::create(".vv", text.as_bytes())?;
        let mut client = crate::connections::rdp::RdpSession::launch_external(
            program,
            vec![file.path().to_string_lossy().into_owned()],
            "SPICE",
        )?;
        client.retain_credential_file(file);
        Ok(client)
    }
}

#[cfg(test)]
#[path = "proxmox_tests.rs"]
mod tests;

fn interface_addresses(rows: &[serde_json::Value]) -> Vec<String> {
    let mut result = Vec::new();
    for row in rows {
        for key in ["inet", "inet6"] {
            if let Some(value) = row.get(key).and_then(|v| v.as_str()) {
                let addr = value.split('/').next().unwrap_or("");
                if let Ok(ip) = addr.parse::<std::net::IpAddr>() {
                    let link_local = match ip {
                        std::net::IpAddr::V4(a) => a.is_link_local(),
                        std::net::IpAddr::V6(a) => (a.segments()[0] & 0xffc0) == 0xfe80,
                    };
                    if !ip.is_loopback()
                        && !ip.is_unspecified()
                        && !ip.is_multicast()
                        && !link_local
                    {
                        result.push(ip.to_string());
                    }
                }
            }
        }
    }
    result.sort();
    result.dedup();
    result.sort_by_key(|s| s.contains(':'));
    result
}
