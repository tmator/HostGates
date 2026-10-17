use super::*;
use crate::{
    connections::{
        native::{self, Auth, FileAction, FileBrowser, Tunnel},
        terminal::Terminal,
    },
    credentials::{CredentialStore, Secret},
    error::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};
use zeroize::Zeroizing;

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionOptions {
    pub key: PathBuf,
    pub fingerprint: String,
    pub gateway: Option<String>,
    pub external_ssh: bool,
    pub manual_external_auth: bool,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub language: String,
    pub sessions: BTreeMap<String, SessionOptions>,
}
pub fn data_dir() -> PathBuf {
    std::env::var_os("HOSTGATES_DATA_DIR")
        .or_else(|| std::env::var_os("REMOTEHUB_DATA_DIR"))
        .map(PathBuf::from)
        .or_else(|| {
            directories::ProjectDirs::from("io", "remotehub", crate::branding::STORAGE_ID)
                .map(|p| p.data_local_dir().to_owned())
        })
        .unwrap_or_else(|| PathBuf::from("."))
}
pub struct GatewayDraft {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Secret,
}
impl Default for GatewayDraft {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            user: String::new(),
            password: Zeroizing::new(String::new()),
        }
    }
}
pub struct Login {
    pub session: Session,
    pub password: Secret,
    pub save: bool,
    pub options: SessionOptions,
    pub message: String,
    pub offered_pin: Option<String>,
    pub offered_gateway: Option<(String, String)>,
}
pub struct Connected {
    pub session: Session,
    pub terminal: Option<Terminal>,
    pub files: Option<FileBrowser>,
    pub tunnel: Option<Tunnel>,
    pub target: Session,
}
pub struct Advanced {
    pub last_activity: std::time::Instant,
    pub preferences: Preferences,
    pub login: Option<Login>,
    pending: Option<Receiver<Result<Connected>>>,
    pub master: Secret,
    pub confirmation: Secret,
    pub panes: usize,
    pub visible: Vec<String>,
    pub rename: Option<(String, String)>,
    pub broadcast: bool,
    pub command: String,
    pub recipients: BTreeSet<String>,
    pub confirm_send: bool,
    pub options_open: bool,
    pub gateway_draft: Option<GatewayDraft>,
    pub options_id: String,
    pub options: SessionOptions,
}
impl Default for Advanced {
    fn default() -> Self {
        Self {
            last_activity: std::time::Instant::now(),
            preferences: Preferences::default(),
            login: None,
            pending: None,
            master: Zeroizing::new(String::new()),
            confirmation: Zeroizing::new(String::new()),
            panes: 1,
            visible: vec![],
            rename: None,
            broadcast: false,
            command: String::new(),
            recipients: Default::default(),
            confirm_send: false,
            options_open: false,
            gateway_draft: None,
            options_id: String::new(),
            options: Default::default(),
        }
    }
}
impl Advanced {
    pub(super) fn cancel_connection(&mut self) {
        self.login = None;
        self.pending = None;
    }
    pub fn load() -> Self {
        let preferences = std::fs::read(data_dir().join("preferences.json"))
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default();
        Self {
            preferences,
            ..Self::default()
        }
    }
    pub fn save(&self) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&self.preferences)
            .map_err(|e| AppError::Operation(e.to_string()))?;
        crate::importers::exchange::write_atomic(&data_dir().join("preferences.json"), &bytes)
    }
}
pub(super) fn saved_password(session: &Session) -> Result<Secret> {
    if let Some(id) = &session.credential_id {
        return crate::credentials::master::shared()
            .ok_or(AppError::VaultUnavailable)?
            .get_secret(id)
            .map(|p| p.unwrap_or_else(|| Zeroizing::new(String::new())));
    }
    Ok(Zeroizing::new(String::new()))
}
fn auth(s: &Session, options: &SessionOptions, password: Secret) -> Auth {
    Auth {
        host: s.data.hostname.trim_matches(['[', ']']).into(),
        port: s.data.port,
        user: s.data.username.clone(),
        password,
        key: options.key.clone(),
        fingerprint: options.fingerprint.clone(),
        endpoint: None,
    }
}
impl HostGates {
    pub(super) fn auto_lock(&mut self) {
        if self.advanced.last_activity.elapsed() < std::time::Duration::from_secs(600) || self.busy
        {
            return;
        }
        let ready = crate::credentials::master::shared()
            .is_some_and(|v| v.0.try_lock().is_ok_and(|v| v.configured() && v.ready()));
        if ready {
            self.tabs.clear();
            self.active_tab = None;
            self.advanced.login = None;
            self.advanced.pending = None;
            self.advanced.master = Zeroizing::new(String::new());
            self.advanced.confirmation = Zeroizing::new(String::new());
            self.send(Action::LockVault);
        }
    }

    pub(super) fn begin_connect(&mut self, s: &Session, ctx: &egui::Context) {
        if self.advanced.pending.is_some() {
            return;
        }
        let options = self
            .advanced
            .preferences
            .sessions
            .get(&s.uuid)
            .cloned()
            .unwrap_or_default();
        if s.data.protocol != crate::models::Protocol::SSH && options.gateway.is_none() {
            self.connect_external(s, ctx);
            return;
        }
        if options.external_ssh && options.gateway.is_none() {
            self.connect_external(s, ctx);
            return;
        }
        let (password, message) = match saved_password(s) {
            Ok(p) => (p, String::new()),
            Err(e) => (Zeroizing::new(String::new()), e.to_string()),
        };
        let start_immediately = message.is_empty()
            && (s.data.protocol != crate::models::Protocol::SSH
                || !s.data.username.trim().is_empty());
        self.advanced.login = Some(Login {
            session: s.clone(),
            password,
            save: false,
            options,
            message,
            offered_pin: None,
            offered_gateway: None,
        });
        if start_immediately {
            self.start_connection(ctx);
        }
    }
    fn start_connection(&mut self, ctx: &egui::Context) {
        let Some(login) = &self.advanced.login else {
            return;
        };
        let s = login.session.clone();
        let options = login.options.clone();
        let password = login.password.clone();
        let gateway = options
            .gateway
            .as_ref()
            .and_then(|id| self.snapshot.sessions.iter().find(|s| &s.uuid == id))
            .cloned();
        if options.gateway.is_some() && gateway.is_none() {
            self.error = Some("Passerelle SSH introuvable.".into());
            return;
        }
        let gateway_auth = if let Some(g) = &gateway {
            let opt = self
                .advanced
                .preferences
                .sessions
                .get(&g.uuid)
                .cloned()
                .unwrap_or_default();
            if g.data.protocol != crate::models::Protocol::SSH
                || opt.gateway.is_some()
                || g.uuid == s.uuid
            {
                self.error =
                    Some("Choisissez une passerelle SSH directe, différente de la cible.".into());
                return;
            }
            match saved_password(g) {
                Ok(p) => Some(auth(g, &opt, p)),
                Err(e) => {
                    self.error = Some(e.to_string());
                    return;
                }
            }
        } else {
            None
        };
        if login.save && !password.is_empty() {
            self.send(Action::SetSecret(s.uuid.clone(), password.clone()));
        }
        self.advanced
            .preferences
            .sessions
            .insert(s.uuid.clone(), options.clone());
        if let Err(e) = self.advanced.save() {
            self.error = Some(e.to_string());
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.advanced.pending = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<Connected> {
                let mut target = s.clone();
                let mut tunnel = None;
                if let Some(g) = gateway_auth {
                    let (host, port) = if s.data.protocol == crate::models::Protocol::WEB {
                        let u = crate::connections::web::validated_url(&s.data.hostname)?;
                        (
                            u.host_str().unwrap_or("").to_owned(),
                            u.port_or_known_default().unwrap_or(443),
                        )
                    } else {
                        (s.data.hostname.clone(), s.data.port)
                    };
                    let t = Tunnel::start(g, host.trim_matches(['[', ']']).into(), port)
                        .map_err(|e| AppError::Operation(format!("Passerelle SSH : {e}")))?;
                    if s.data.protocol == crate::models::Protocol::WEB {
                        let mut u = crate::connections::web::validated_url(&s.data.hostname)?;
                        u.set_host(Some("127.0.0.1"))
                            .map_err(|e| AppError::Operation(e.to_string()))?;
                        u.set_port(Some(t.port))
                            .map_err(|_| AppError::Validation("Port URL invalide.".into()))?;
                        target.data.hostname = u.to_string();
                    } else {
                        target.data.hostname = "127.0.0.1".into();
                    }
                    target.data.port = t.port;
                    tunnel = Some(t);
                }
                let (terminal, files) = if s.data.protocol == crate::models::Protocol::SSH {
                    let mut a = auth(&s, &options, password);
                    if tunnel.is_some() {
                        a.endpoint = Some((target.data.hostname.clone(), target.data.port));
                    }
                    let ssh = native::connect(&a)?;
                    (
                        Some(Terminal::from_ssh(ssh, ctx.clone())?),
                        Some(FileBrowser::start(a, ctx.clone())),
                    )
                } else {
                    (None, None)
                };
                Ok(Connected {
                    session: s,
                    terminal,
                    files,
                    tunnel,
                    target,
                })
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    pub(super) fn advanced_dialogs(&mut self, ctx: &egui::Context) {
        let result = self
            .advanced
            .pending
            .as_ref()
            .and_then(|rx| rx.try_recv().ok());
        if let Some(result) = result {
            self.advanced.pending = None;
            match result {
                Ok(mut c) => {
                    self.advanced.login = None;
                    if let Some(t) = c.terminal.take() {
                        let id = uuid::Uuid::new_v4().to_string();
                        self.active_tab = Some(id.clone());
                        self.tabs.push(tabs::SessionTab {
                            id,
                            title: c.session.data.name.clone(),
                            session: Some(c.session),
                            client: tabs::TabClient::Terminal(
                                Box::new(t),
                                terminal::TerminalView::default(),
                            ),
                            files: c.files,
                            tunnel: c.tunnel,
                        });
                    } else if c.target.data.protocol == crate::models::Protocol::WEB {
                        match crate::connections::web::open(&c.target.data.hostname) {
                            Ok(()) => {
                                let id = uuid::Uuid::new_v4().to_string();
                                self.active_tab = Some(id.clone());
                                self.tabs.push(tabs::SessionTab {
                                    id,
                                    title: c.session.data.name.clone(),
                                    session: Some(c.session),
                                    client: tabs::TabClient::Web,
                                    files: None,
                                    tunnel: c.tunnel,
                                });
                            }
                            Err(e) => self.error = Some(e.to_string()),
                        }
                    } else {
                        let before = self.tabs.len();
                        self.connect_external(&c.target, ctx);
                        if self.tabs.len() > before
                            && let Some(tab) = self.tabs.last_mut()
                        {
                            tab.session = Some(c.session);
                            tab.tunnel = c.tunnel;
                        }
                    }
                }
                Err(e) => {
                    if let Some(login) = &mut self.advanced.login {
                        login.message = e.to_string();
                        login.offered_pin = None;
                        login.offered_gateway = None;
                        if login.message.contains("HOSTKEY_UNKNOWN")
                            && login.message.contains("Passerelle SSH :")
                            && let Some(gateway) = login.options.gateway.as_ref().and_then(|id| {
                                self.snapshot.sessions.iter().find(|g| &g.uuid == id)
                            })
                            && login.message.contains(&format!(
                                " {}:{} ",
                                gateway.data.hostname.trim_matches(['[', ']']),
                                gateway.data.port
                            ))
                            && let Some(pin) = login.message.split("SHA256(hex)=").nth(1)
                        {
                            login.offered_gateway = Some((gateway.uuid.clone(), pin.to_owned()));
                        }
                        if login.offered_gateway.is_none()
                            && !login.message.contains("Passerelle SSH :")
                            && login.message.contains("HOSTKEY_UNKNOWN")
                            && login.message.contains(&format!(
                                " {}:{} ",
                                login.session.data.hostname, login.session.data.port
                            ))
                        {
                            login.offered_pin = login
                                .message
                                .split("SHA256(hex)=")
                                .nth(1)
                                .map(str::to_owned);
                        }
                    } else {
                        self.error = Some(e.to_string());
                    }
                }
            }
        }
        let mut start = super::shortcuts::submitted(ctx, "login")
            && self.advanced.pending.is_none()
            && !self.busy;
        let mut cancel = false;
        let mut accept_gateway = None;
        if let Some(login) = &mut self.advanced.login {
            egui::Window::new(crate::i18n::tr("Connexion sécurisée")).id(egui::Id::new("secure_login")).collapsible(false).show(ctx,|ui|{
                ui.label(crate::i18n::tr(&format!("{} — {}",login.session.data.name,login.session.data.endpoint())));
                ui.add_enabled_ui(self.advanced.pending.is_none() && !self.busy,|ui|{
                    if login.session.data.protocol==crate::models::Protocol::SSH {
                        ui.label(crate::i18n::tr("Utilisateur"));ui.text_edit_singleline(&mut login.session.data.username);
                        ui.label(crate::i18n::tr("Mot de passe / phrase secrète de clé (vide : agent SSH)"));let password_response = ui.add(egui::TextEdit::singleline(&mut *login.password).password(true));
                        if ui.is_enabled() && password_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { start = true; }
                        ui.checkbox(&mut login.save,crate::i18n::tr("Enregistrer dans le coffre"));
                        ui.label(crate::i18n::tr("Clé privée facultative"));ui.horizontal(|ui|{ui.label(login.options.key.display().to_string());if ui.button(crate::i18n::tr("Parcourir…")).clicked() && let Some(p)=rfd::FileDialog::new().pick_file(){login.options.key=p;}
if ui.small_button(crate::i18n::tr("×")).clicked(){login.options.key=PathBuf::new();}});
                    }
                    if login.options.gateway.is_some(){ui.label(crate::i18n::tr("La passerelle utilise sa clé/son agent ou son mot de passe enregistré. Connectez-la une fois pour valider son identité."));}
                    if login.session.data.protocol==crate::models::Protocol::WEB && login.options.gateway.is_some(){ui.label(crate::i18n::tr("Web via adresse locale : le certificat HTTPS doit correspondre. Ne désactivez pas sa vérification ; les sites exigeant leur nom DNS peuvent refuser cette URL."));}
                    if !login.message.is_empty(){ui.colored_label(Color32::LIGHT_RED,crate::i18n::tr(&login.message));}
                    if let Some((id,pin)) = login.offered_gateway.clone() {
                        ui.label(crate::i18n::tr("Vérifiez l'empreinte de la passerelle avec son administrateur."));
                        if ui.button(crate::i18n::tr("Faire confiance à la clé de la passerelle")).clicked() { accept_gateway = Some((id,pin)); }
                    }
                    if let Some(pin)=login.offered_pin.clone(){
                        ui.label(crate::i18n::tr("Vérifiez cette empreinte avec l'administrateur avant de l'accepter."));
                        if ui.button(crate::i18n::tr("Faire confiance à cette clé")).clicked(){login.options.fingerprint=pin;login.offered_pin=None;login.message.clear();}
                    }
                    ui.horizontal(|ui|{start |= ui.button(crate::i18n::tr("Connecter")).clicked();cancel=ui.button(crate::i18n::tr("Annuler")).clicked();});
                });if self.advanced.pending.is_some(){ui.spinner();cancel=ui.button(crate::i18n::tr("Annuler")).clicked();}
            });
        }
        if let Some((id, pin)) = accept_gateway {
            self.advanced
                .preferences
                .sessions
                .entry(id)
                .or_default()
                .fingerprint = pin;
            match self.advanced.save() {
                Ok(()) => {
                    if let Some(login) = &mut self.advanced.login {
                        login.offered_gateway = None;
                        login.message.clear();
                    }
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        if cancel {
            self.advanced.login = None;
            self.advanced.pending = None;
        }
        if start {
            self.start_connection(ctx);
        }
        self.connection_options(ctx);
        self.multi_commands(ctx);
        if let Some((id, mut name)) = self.advanced.rename.take() {
            let mut keep = true;
            egui::Window::new(crate::i18n::tr("Renommer l'onglet"))
                .id(egui::Id::new("rename_dialog"))
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut name);
                    ui.horizontal(|ui| {
                        if (ui.button(crate::i18n::tr("Enregistrer")).clicked()
                            || super::shortcuts::submitted(ctx, "rename"))
                            && crate::models::validate_name(&name).is_ok()
                        {
                            if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                                t.title = name.clone();
                            }
                            keep = false;
                        }
                        if ui.button(crate::i18n::tr("Annuler")).clicked() {
                            keep = false;
                        }
                    });
                });
            if keep {
                self.advanced.rename = Some((id, name));
            }
        }
    }
    pub(super) fn vault_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(crate::i18n::tr("Coffre et mot de passe maître"));
        let state = crate::credentials::master::shared()
            .and_then(|v| v.0.try_lock().ok().map(|v| (v.configured(), v.ready())));
        let Some((configured, ready)) = state else {
            ui.spinner();
            return;
        };
        ui.label(crate::i18n::tr(if configured {
            if ready {
                "Coffre déverrouillé"
            } else {
                "Coffre verrouillé"
            }
        } else {
            "Coffre système — protection maître non configurée"
        }));
        ui.add(
            egui::TextEdit::singleline(&mut *self.advanced.master)
                .id(egui::Id::new("master_password"))
                .password(true)
                .hint_text(crate::i18n::tr("Mot de passe maître")),
        );
        if !configured || ready {
            ui.add(
                egui::TextEdit::singleline(&mut *self.advanced.confirmation)
                    .id(egui::Id::new("master_confirmation"))
                    .password(true)
                    .hint_text(crate::i18n::tr("Confirmer le mot de passe")),
            );
        }
        ui.add_enabled_ui(!self.busy, |ui| {
            let label = if !configured {
                "Créer le coffre chiffré"
            } else if !ready {
                "Déverrouiller"
            } else {
                "Changer le mot de passe maître"
            };
            if !self.busy
                && (ui.button(crate::i18n::tr(label)).clicked()
                    || super::shortcuts::submitted(ui.ctx(), "vault"))
            {
                if (!configured || ready) && self.advanced.master != self.advanced.confirmation {
                    self.error = Some(crate::i18n::tr("Les mots de passe ne correspondent pas."));
                } else {
                    let secret =
                        std::mem::replace(&mut self.advanced.master, Zeroizing::new(String::new()));
                    self.advanced.confirmation = Zeroizing::new(String::new());
                    self.send(if !configured {
                        Action::CreateVault(secret)
                    } else if !ready {
                        Action::UnlockVault(secret)
                    } else {
                        Action::ChangeMaster(secret)
                    });
                }
            }
            if configured
                && ready
                && ui
                    .button(crate::i18n::tr("Verrouiller et fermer les connexions"))
                    .clicked()
            {
                self.advanced.pending = None;
                self.tabs.clear();
                self.active_tab = None;
                self.advanced.login = None;
                self.send(Action::LockVault);
            }
        });
        ui.small(crate::i18n::tr("Argon2id (64 Mio, 3 passes), AES-256-GCM-SIV. Aucun mot de passe maître récupérable. Conservez une sauvegarde du coffre."));
    }
    fn connection_options(&mut self, ctx: &egui::Context) {
        if !self.advanced.options_open {
            return;
        }
        let mut open = true;
        let mut save = false;
        egui::Window::new(crate::i18n::tr("Options SSH / tunnel")).id(egui::Id::new("options_dialog")).open(&mut open).show(ctx,|ui|{
            if ui.button(crate::i18n::tr("Saisir une nouvelle passerelle SSH…")).clicked() { self.advanced.gateway_draft = Some(GatewayDraft::default()); }
            if let Some(draft) = &mut self.advanced.gateway_draft {
                ui.label(crate::i18n::tr("Adresse de la passerelle")); ui.text_edit_singleline(&mut draft.host);
                ui.label(crate::i18n::tr("Port")); ui.add(egui::DragValue::new(&mut draft.port).range(1..=65535));
                ui.label(crate::i18n::tr("Utilisateur")); ui.text_edit_singleline(&mut draft.user);
                ui.label(crate::i18n::tr("Mot de passe")); ui.add(egui::TextEdit::singleline(&mut *draft.password).password(true));
                ui.small(crate::i18n::tr("Une session SSH réutilisable sera créée. Son mot de passe sera conservé dans le coffre."));
                if ui.button(crate::i18n::tr("Annuler la nouvelle passerelle")).clicked() { self.advanced.gateway_draft = None; }
            }
            let opt=&mut self.advanced.options;
            egui::ComboBox::from_id_salt("gateway").selected_text(opt.gateway.as_ref().and_then(|id|self.snapshot.sessions.iter().find(|s|&s.uuid==id)).map(|s|s.data.name.clone()).unwrap_or_else(||crate::i18n::tr("Connexion directe"))).show_ui(ui,|ui|{
                ui.selectable_value(&mut opt.gateway,None,crate::i18n::tr("Connexion directe"));
                for s in self.snapshot.sessions.iter().filter(|s|s.data.protocol==crate::models::Protocol::SSH&&s.uuid!=self.advanced.options_id){ui.selectable_value(&mut opt.gateway,Some(s.uuid.clone()),&s.data.name);}
            });
            ui.label(crate::i18n::tr("Passerelle SSH pour SSH, RDP, VNC ou Web. Un seul saut ; écoute locale uniquement."));
            ui.checkbox(&mut opt.manual_external_auth, crate::i18n::tr("Saisir le mot de passe dans le client externe (RDP / VNC)"));
            if self.snapshot.sessions.iter().any(|s|s.uuid==self.advanced.options_id&&s.data.protocol==crate::models::Protocol::SSH){ui.checkbox(&mut opt.external_ssh,crate::i18n::tr("Utiliser OpenSSH externe (sans volet SFTP intégré)"));}
            ui.label(crate::i18n::tr("Empreinte SHA-256 hexadécimale de la clé serveur"));ui.text_edit_singleline(&mut opt.fingerprint);
            ui.small(crate::i18n::tr("Ne modifiez une empreinte qu'après vérification indépendante de la nouvelle clé."));
            save=!self.busy && (ui.add_enabled(!self.busy,egui::Button::new(crate::i18n::tr("Enregistrer"))).clicked() || super::shortcuts::submitted(ctx,"options"));
        });
        if save {
            if let Some(draft) = &self.advanced.gateway_draft {
                let data = crate::models::SessionInput {
                    name: format!(
                        "Passerelle {}",
                        draft.host.chars().take(80).collect::<String>()
                    ),
                    protocol: crate::models::Protocol::SSH,
                    hostname: draft.host.trim().to_owned(),
                    port: draft.port,
                    username: draft.user.trim().to_owned(),
                    domain: String::new(),
                    folder_id: None,
                    favorite: false,
                    description: String::new(),
                    color: "#7195ff".into(),
                    icon: "server".into(),
                    tags: vec![],
                };
                if let Err(e) = data.validate() {
                    self.error = Some(e.to_string());
                    return;
                }
                if data.username.is_empty() {
                    self.error = Some(crate::i18n::tr("Utilisateur SSH requis."));
                    return;
                }
                if !draft.password.is_empty() && !self.vault_ready {
                    self.error = Some(crate::i18n::tr(
                        "Déverrouillez le coffre avant d'enregistrer le mot de passe.",
                    ));
                    return;
                }
                let secret = draft.password.clone();
                self.advanced.preferences.sessions.insert(
                    self.advanced.options_id.clone(),
                    self.advanced.options.clone(),
                );
                self.send(Action::CreateGateway(
                    self.advanced.options_id.clone(),
                    data,
                    secret,
                ));
                self.advanced.gateway_draft = None;
                self.advanced.options_open = false;
                return;
            }
            self.advanced.preferences.sessions.insert(
                self.advanced.options_id.clone(),
                self.advanced.options.clone(),
            );
            if let Err(e) = self.advanced.save() {
                self.error = Some(e.to_string());
            }
            open = false;
        }
        if !open {
            self.advanced.gateway_draft = None;
        }
        self.advanced.options_open = open;
    }
    fn multi_commands(&mut self, ctx: &egui::Context) {
        if !self.advanced.broadcast {
            return;
        }
        self.advanced.recipients.retain(|id| {
            self.tabs.iter().any(|t| {
                &t.id == id && !t.closed() && matches!(t.client, tabs::TabClient::Terminal(..))
            })
        });
        let mut open = true;
        let mut send = false;
        egui::Window::new(crate::i18n::tr("Exécution multiple")).id(egui::Id::new("broadcast_dialog")).open(&mut open).default_width(600.0).show(ctx,|ui|{
            ui.label(crate::i18n::tr("Sélectionnez les terminaux destinataires. Chaque ligne sera envoyée comme une commande."));
            ui.horizontal(|ui| {
                if ui.button(crate::i18n::tr("Sélectionner tout")).clicked() {
                    self.advanced.recipients = self.tabs.iter().filter(|t| matches!(t.client,tabs::TabClient::Terminal(..)) && !t.closed()).map(|t| t.id.clone()).collect();
                    self.advanced.confirm_send = false;
                }
                if ui.button(crate::i18n::tr("Tout désélectionner")).clicked() {
                    self.advanced.recipients.clear();
                    self.advanced.confirm_send = false;
                }
            });
            for t in self.tabs.iter().filter(|t|matches!(t.client,tabs::TabClient::Terminal(..))&&!t.closed()){
                let mut selected=self.advanced.recipients.contains(&t.id);if ui.checkbox(&mut selected,&t.title).changed(){self.advanced.confirm_send=false;if selected{self.advanced.recipients.insert(t.id.clone());}else{self.advanced.recipients.remove(&t.id);}}
            }
            if ui.add(egui::TextEdit::multiline(&mut self.advanced.command).id(egui::Id::new("multi_command")).desired_rows(6).desired_width(f32::INFINITY)).changed(){self.advanced.confirm_send=false;}
            ui.checkbox(&mut self.advanced.confirm_send,crate::i18n::tr("J'ai vérifié les commandes et leurs destinataires"));
            send=ui.add_enabled(self.advanced.confirm_send&&!self.advanced.command.trim().is_empty()&&!self.advanced.recipients.is_empty(),egui::Button::new(crate::i18n::tr("Exécuter sur la sélection"))).clicked() || (super::shortcuts::submitted(ctx,"broadcast") && self.advanced.confirm_send && !self.advanced.command.trim().is_empty() && !self.advanced.recipients.is_empty());
        });
        self.advanced.broadcast = open;
        if send {
            let text = self
                .advanced
                .command
                .replace("\r\n", "\n")
                .replace('\r', "\n");
            if text
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t')
                || text.len() > 65536
            {
                self.error = Some(crate::i18n::tr("Commande invalide ou trop longue."));
                return;
            }
            for t in &mut self.tabs {
                if self.advanced.recipients.contains(&t.id)
                    && !t.closed()
                    && let tabs::TabClient::Terminal(term, _) = &mut t.client
                {
                    term.write(
                        format!("{}\r", text.trim_end_matches('\n').replace('\n', "\r"))
                            .into_bytes(),
                    );
                }
            }
            self.advanced.confirm_send = false;
        }
    }
}
pub fn files_ui(ui: &mut egui::Ui, files: &mut FileBrowser) {
    files.poll();
    ui.heading(crate::i18n::tr("Fichiers distants"));
    ui.add_enabled_ui(!files.busy, |ui| {
        ui.horizontal(|ui| {
            ui.menu_button(crate::i18n::tr("Envoyer"), |ui| {
                if ui.button(crate::i18n::tr("Fichiers…")).clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_files() {
                        files.send(FileAction::Upload(p, files.path.clone()));
                    }
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Dossier…")).clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        files.send(FileAction::Upload(vec![p], files.path.clone()));
                    }
                    ui.close();
                }
            });
            if ui
                .add_enabled(
                    !files.selected.is_empty(),
                    egui::Button::new(crate::i18n::tr("Télécharger")),
                )
                .clicked()
                && let Some(dir) = rfd::FileDialog::new().pick_folder()
            {
                files.send(FileAction::Download(
                    files
                        .entries
                        .iter()
                        .filter(|e| files.selected.contains(&e.path))
                        .cloned()
                        .collect(),
                    dir,
                ));
            }
        });
        ui.horizontal(|ui| {
            if ui.button(crate::i18n::tr("↑")).clicked() {
                files.send(FileAction::List(
                    files
                        .path
                        .parent()
                        .unwrap_or(std::path::Path::new("/"))
                        .to_owned(),
                ));
            }
            if ui.button(crate::i18n::tr("↻")).clicked() {
                files.send(FileAction::List(files.path.clone()));
            }
        });
        ui.label(files.path.display().to_string());
        let mut enter = None;
        egui::ScrollArea::vertical()
            .id_salt("sftp_entries")
            .max_height((ui.available_height() - 70.0).max(80.0))
            .show(ui, |ui| {
                for e in &files.entries {
                    let r = ui.selectable_label(
                        files.selected.contains(&e.path),
                        format!(
                            "{} {}  {}",
                            if e.directory { "▸" } else { "·" },
                            e.name,
                            if e.directory {
                                String::new()
                            } else {
                                e.size.to_string()
                            }
                        ),
                    );
                    if r.clicked() {
                        if !ui.input(|i| i.modifiers.ctrl) {
                            files.selected.clear();
                        }
                        if !files.selected.insert(e.path.clone()) {
                            files.selected.remove(&e.path);
                        }
                    }
                    if e.directory && r.double_clicked() {
                        enter = Some(e.path.clone());
                    }
                }
            });
        if let Some(p) = enter {
            files.send(FileAction::List(p));
        }
    });
    if files.busy {
        ui.spinner();
        if ui.button(crate::i18n::tr("Annuler le transfert")).clicked() {
            files.cancel_operation();
        }
    }
    ui.label(crate::i18n::tr(&files.status));
    ui.small(crate::i18n::tr(
        "Aucun écrasement automatique. Les liens symboliques sont refusés.",
    ));
}
