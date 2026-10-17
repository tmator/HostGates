use crate::{
    models::{Folder, FolderInput, Protocol, Session, SessionInput},
    services::worker::Action,
};
use eframe::egui;
use zeroize::Zeroizing;

pub struct Outcome {
    pub close: bool,
    pub action: Option<Action>,
}
pub struct SessionEditor {
    id: Option<String>,
    data: SessionInput,
    tags: String,
    new_folder: Option<String>,
    password: Zeroizing<String>,
    save_password: bool,
    pub secret_open: bool,
    error: Option<String>,
}
impl SessionEditor {
    pub fn new(folder_id: Option<String>) -> Self {
        Self::from_data(SessionInput {
            name: String::new(),
            protocol: Protocol::SSH,
            hostname: String::new(),
            port: 22,
            username: String::new(),
            domain: String::new(),
            folder_id,
            favorite: false,
            description: String::new(),
            color: "#7195ff".into(),
            icon: "server".into(),
            tags: vec![],
        })
    }
    pub fn from_data(data: SessionInput) -> Self {
        Self {
            id: None,
            tags: data.tags.join(", "),
            new_folder: None,
            data,
            password: Zeroizing::new(String::new()),
            secret_open: false,
            save_password: false,
            error: None,
        }
    }
    pub fn select_created_folder(&mut self, id: String) {
        self.data.folder_id = Some(id);
        self.new_folder = None;
        self.error = None;
    }
    pub fn from_session(s: &Session) -> Self {
        let mut e = Self::from_data(s.data.clone());
        e.id = Some(s.uuid.clone());
        e
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        folders: &[Folder],
        busy: bool,
        vault_ready: bool,
    ) -> Outcome {
        let mut out = Outcome {
            close: false,
            action: None,
        };
        let mut open = true;
        egui::Window::new(crate::i18n::tr(if self.id.is_some(){"Modifier la session"}else{"Nouvelle session"})).id(egui::Id::new("session_editor")).open(&mut open).collapsible(false).resizable(false).default_width(500.0).show(ctx,|ui| {
            ui.add_enabled_ui(!busy,|ui| {
                if self.secret_open {
                    ui.label(crate::i18n::tr("Le mot de passe sera enregistré dans le coffre actif (système ou chiffré par mot de passe maître)."));
                    ui.add(egui::TextEdit::singleline(&mut *self.password).password(true).hint_text(crate::i18n::tr("Mot de passe")).desired_width(400.0));
                    if !vault_ready { ui.label(crate::i18n::tr("Coffre indisponible : enregistrement désactivé. Aucun repli vers un fichier.")); }
                    if let Some(id)=&self.id {
                        ui.horizontal(|ui| {
                            if ui.add_enabled(vault_ready&&!self.password.is_empty(),egui::Button::new(crate::i18n::tr("Enregistrer dans le coffre"))).clicked() || (vault_ready && !self.password.is_empty() && super::shortcuts::submitted(ctx,"editor")) { out.action=Some(Action::SetSecret(id.clone(),std::mem::replace(&mut self.password,Zeroizing::new(String::new()))));out.close=true; }
                            if ui.button(crate::i18n::tr("Supprimer du coffre")).clicked(){out.action=Some(Action::RemoveSecret(id.clone()));out.close=true;}
                        });
                    }
                    if ui.button(crate::i18n::tr("Retour")).clicked(){self.secret_open=false;self.password=Zeroizing::new(String::new());}
                    return;
                }
                egui::Grid::new("session_fields").num_columns(2).spacing([16.0,12.0]).show(ui,|ui|{
                    ui.label(crate::i18n::tr("Nom *"));ui.text_edit_singleline(&mut self.data.name);ui.end_row();
                    ui.label(crate::i18n::tr("Protocole"));ui.horizontal(|ui|{
                        if ui.selectable_value(&mut self.data.protocol,Protocol::SSH,crate::i18n::tr("SSH")).changed(){self.data.port=22;}
                        if ui.selectable_value(&mut self.data.protocol,Protocol::RDP,crate::i18n::tr("RDP")).changed(){self.data.port=3389;}
                        if ui.selectable_value(&mut self.data.protocol,Protocol::VNC,crate::i18n::tr("VNC")).changed(){self.data.port=5900;}
                        if ui.selectable_value(&mut self.data.protocol,Protocol::WEB,crate::i18n::tr("Web")).changed(){self.data.port=443;}
                    });ui.end_row();
                    ui.label(crate::i18n::tr(if self.data.protocol==Protocol::WEB { "URL http(s) *" } else { "Serveur *" }));ui.add(egui::TextEdit::singleline(&mut self.data.hostname).hint_text(if self.data.protocol==Protocol::WEB { "https://serveur.exemple.fr:8443/" } else { "serveur.exemple.fr ou 192.168.1.10" }));ui.end_row();
                    if self.data.protocol!=Protocol::WEB { ui.label(crate::i18n::tr("Port *"));ui.add(egui::DragValue::new(&mut self.data.port).range(1..=65535));ui.end_row(); }
                    ui.label(crate::i18n::tr("Utilisateur"));ui.text_edit_singleline(&mut self.data.username);ui.end_row();
                    if self.data.protocol==Protocol::RDP {ui.label(crate::i18n::tr("Domaine"));ui.text_edit_singleline(&mut self.data.domain);ui.end_row();}
                    ui.label(crate::i18n::tr("Dossier"));ui.vertical(|ui| {
                        folder_select(ui,"session_folder",&mut self.data.folder_id,folders,None);
                        if self.new_folder.is_none() && ui.button(crate::i18n::tr("+ Nouveau dossier…")).clicked() { self.new_folder = Some(String::new()); }
                        if let Some(name) = &mut self.new_folder {
                            ui.label(crate::i18n::tr("Créer dans le dossier sélectionné (ou à la racine) :"));
                            ui.text_edit_singleline(name);
                            let mut cancel = false;
                            ui.horizontal(|ui| {
                                if ui.button(crate::i18n::tr("Créer et sélectionner")).clicked() || super::shortcuts::submitted(ctx,"editor") {
                                    match crate::models::validate_name(name) {
                                        Ok(()) => out.action = Some(Action::CreateSessionFolder(FolderInput { name: name.trim().into(), parent_id: self.data.folder_id.clone(), sort_order: 0, color: "#7195ff".into(), icon: "folder".into() })),
                                        Err(e) => self.error = Some(e.to_string()),
                                    }
                                }
                                if ui.button(crate::i18n::tr("Annuler le dossier")).clicked() { cancel = true; }
                            });
                            if cancel { self.new_folder = None; }
                        }
                    });ui.end_row();
                    ui.label(crate::i18n::tr("Tags"));ui.add(egui::TextEdit::singleline(&mut self.tags).hint_text(crate::i18n::tr("production, linux, réseau")));ui.end_row();
                    ui.label(crate::i18n::tr("Description"));ui.add(egui::TextEdit::multiline(&mut self.data.description).id(egui::Id::new("session_description")));ui.end_row();
                    ui.label(crate::i18n::tr("Favori"));ui.checkbox(&mut self.data.favorite,crate::i18n::tr("Ajouter aux favoris"));ui.end_row();
                });
                ui.add_space(10.0);
                ui.checkbox(&mut self.save_password, crate::i18n::tr("Enregistrer / remplacer le mot de passe dans le coffre"));
                if self.save_password {
                    ui.add(egui::TextEdit::singleline(&mut *self.password).password(true).hint_text(crate::i18n::tr("Mot de passe")));
                    if !vault_ready { ui.label(crate::i18n::tr("Déverrouillez le coffre avant d'enregistrer le mot de passe.")); }
                }
                ui.label(crate::i18n::tr(match self.data.protocol {
                    Protocol::SSH => "SSH intégré : mot de passe transmis après vérification de la clé serveur. OpenSSH externe : saisie dans le terminal.",
                    Protocol::RDP => "RDP : mot de passe fourni à MSTSC (Windows) ou FreeRDP (Linux). Une politique serveur peut imposer une nouvelle saisie.",
                    Protocol::VNC => "VNC : transmission via PasswordFile (TigerVNC, authentification classique, 8 caractères ASCII maximum).",
                    Protocol::WEB => "Web : le mot de passe peut être conservé dans le coffre. La connexion au site reste gérée par le navigateur.",
                }));
                if let Some(error)=&self.error {ui.colored_label(egui::Color32::LIGHT_RED,crate::i18n::tr(error));}
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.new_folder.is_none() && (!self.save_password || (vault_ready && !self.password.is_empty())), egui::Button::new(crate::i18n::tr("Enregistrer"))).clicked() || (self.new_folder.is_none() && (!self.save_password || (vault_ready && !self.password.is_empty())) && super::shortcuts::submitted(ctx,"editor")){
                        self.data.hostname=self.data.hostname.trim().to_owned();
                        if self.data.protocol==Protocol::WEB && let Ok(url)=crate::connections::web::validated_url(&self.data.hostname) { self.data.port=url.port_or_known_default().unwrap_or(443); }
                        self.data.tags=self.tags.split(',').map(str::trim).filter(|s|!s.is_empty()).map(str::to_owned).collect();
                        match self.data.validate(){Ok(())=>out.action=Some(if self.save_password {
                            Action::SaveSessionSecret(self.id.clone(),self.data.clone(),std::mem::replace(&mut self.password,Zeroizing::new(String::new())))
                        } else { Action::SaveSession(self.id.clone(),self.data.clone()) }),Err(e)=>self.error=Some(e.to_string())}
                    }
                    if ui.button(crate::i18n::tr("Annuler")).clicked(){out.close=true;}
                });
            });
        });
        out.close |= !open;
        out
    }
}
pub struct FolderEditor {
    id: Option<String>,
    data: FolderInput,
}
impl FolderEditor {
    pub fn new(parent_id: Option<String>) -> Self {
        Self {
            id: None,
            data: FolderInput {
                name: String::new(),
                parent_id,
                sort_order: 0,
                color: "#7195ff".into(),
                icon: "folder".into(),
            },
        }
    }
    pub fn from_folder(f: &Folder) -> Self {
        Self {
            id: Some(f.uuid.clone()),
            data: f.data.clone(),
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, folders: &[Folder], busy: bool) -> Outcome {
        let mut out = Outcome {
            close: false,
            action: None,
        };
        let mut open = true;
        egui::Window::new(crate::i18n::tr("Dossier"))
            .id(egui::Id::new("folder_dialog"))
            .open(&mut open)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!busy, |ui| {
                    ui.label(crate::i18n::tr("Nom"));
                    ui.text_edit_singleline(&mut self.data.name);
                    ui.label(crate::i18n::tr("Dossier parent"));
                    folder_select(
                        ui,
                        "parent_folder",
                        &mut self.data.parent_id,
                        folders,
                        self.id.as_deref(),
                    );
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::tr("Enregistrer")).clicked()
                            || super::shortcuts::submitted(ctx, "folder")
                        {
                            out.action =
                                Some(Action::SaveFolder(self.id.clone(), self.data.clone()));
                        }
                        if ui.button(crate::i18n::tr("Annuler")).clicked() {
                            out.close = true;
                        }
                    });
                });
            });
        out.close |= !open;
        out
    }
}
fn folder_select(
    ui: &mut egui::Ui,
    salt: &str,
    selected: &mut Option<String>,
    folders: &[Folder],
    exclude: Option<&str>,
) {
    let label = folders
        .iter()
        .find(|f| Some(&f.uuid) == selected.as_ref())
        .map_or("Racine", |f| f.data.name.as_str());
    egui::ComboBox::from_id_salt(salt)
        .selected_text(label)
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, None, crate::i18n::tr("Racine"));
            for folder in folders.iter().filter(|f| Some(f.uuid.as_str()) != exclude) {
                ui.selectable_value(
                    selected,
                    Some(folder.uuid.clone()),
                    folder_path(folder, folders),
                );
            }
        });
}
fn folder_path(folder: &Folder, folders: &[Folder]) -> String {
    let mut names = vec![folder.data.name.as_str()];
    let mut parent = folder.data.parent_id.as_deref();
    for _ in 0..64 {
        let Some(p) = parent else { break };
        let Some(f) = folders.iter().find(|f| f.uuid == p) else {
            break;
        };
        names.push(&f.data.name);
        parent = f.data.parent_id.as_deref();
    }
    names.reverse();
    names.join(" / ")
}
