use crate::{
    branding::STORAGE_ID,
    credentials::{Secret, master},
    database::Database,
    error::{AppError, Result},
    models::{FolderInput, SessionInput, Snapshot},
    services::SessionService,
};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

pub enum Action {
    CreateGateway(String, SessionInput, Secret),
    UnlockVault(Secret),
    CreateVault(Secret),
    ChangeMaster(Secret),
    LockVault,
    PreviewImport(PathBuf),
    Import(crate::importers::Preview),
    Export(
        PathBuf,
        Vec<String>,
        crate::importers::exchange::ExportFormat,
    ),
    Refresh,
    MoveSession(String, Option<String>),
    CreateSessionFolder(FolderInput),
    SaveSession(Option<String>, SessionInput),
    SaveSessionSecret(Option<String>, SessionInput, Secret),
    DeleteSession(String),
    SaveFolder(Option<String>, FolderInput),
    DeleteFolder(String),
    SetSecret(String, Secret),
    RemoveSecret(String),
    Theme(String),
    Programs(crate::models::ExternalPrograms),
}
pub struct Update {
    pub gateway: Option<(String, String)>,
    pub warning: Option<String>,
    pub session_folder: Option<String>,
    pub preview: Option<crate::importers::Preview>,
    pub snapshot: Snapshot,
    pub message: String,
    pub saved_id: Option<String>,
    pub path: PathBuf,
    pub vault_ready: bool,
}
pub struct Worker {
    pub tx: Sender<Action>,
    pub rx: Receiver<Result<Update>>,
}
impl Worker {
    pub fn start(ctx: eframe::egui::Context) -> Self {
        let (tx, requests) = mpsc::channel();
        let (results, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let setup = (|| -> Result<_> {
                let dirs = directories::ProjectDirs::from("io", "remotehub", STORAGE_ID)
                    .ok_or(AppError::Database)?;
                let data_dir = std::env::var_os("HOSTGATES_DATA_DIR")
                    .or_else(|| std::env::var_os("REMOTEHUB_DATA_DIR"))
                    .map(PathBuf::from)
                    .unwrap_or_else(|| dirs.data_local_dir().to_owned());
                #[cfg(unix)]
                let existed = data_dir.exists();
                std::fs::create_dir_all(&data_dir).map_err(|_| AppError::Database)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if existed {
                        if std::fs::metadata(&data_dir)
                            .map_err(|_| AppError::Database)?
                            .permissions()
                            .mode()
                            & 0o077
                            != 0
                        {
                            return Err(AppError::Operation("Le dossier de données doit être privé (permissions 0700). Choisissez un dossier dédié à HostGates.".into()));
                        }
                    } else {
                        std::fs::set_permissions(&data_dir, std::fs::Permissions::from_mode(0o700))
                            .map_err(|_| AppError::Database)?;
                    }
                }
                let path = data_dir.join("remotehub.db");
                let instance = std::fs::OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .read(true)
                    .write(true)
                    .open(data_dir.join("instance.lock"))
                    .map_err(|_| AppError::Database)?;
                instance.try_lock().map_err(|_|AppError::Operation("Une autre instance utilise déjà ces données. Fermez-la avant de continuer.".into()))?;
                let db = Database::open(&path)?;
                let vault = master::install(data_dir.join("vault.json"));
                Ok((
                    SessionService {
                        db,
                        vault: Box::new(vault),
                    },
                    path,
                    instance,
                ))
            })();
            let (mut service, path, _instance) = match setup {
                Ok(x) => x,
                Err(e) => {
                    let _ = results.send(Err(e));
                    ctx.request_repaint();
                    return;
                }
            };
            for action in requests {
                let result = (|| -> Result<Update> {
                    let mut saved_id = None;
                    let mut gateway = None;
                    let mut warning = None;
                    let mut session_folder = None;
                    let mut preview = None;
                    let mut custom_message = None;
                    let message = match action {
                        Action::UnlockVault(secret) => {
                            master::shared()
                                .ok_or(AppError::VaultUnavailable)?
                                .0
                                .lock()
                                .map_err(|_| AppError::VaultUnavailable)?
                                .unlock(&secret)?;
                            "Coffre déverrouillé."
                        }
                        Action::CreateVault(secret) => {
                            let ids = service
                                .db
                                .snapshot()?
                                .sessions
                                .into_iter()
                                .filter_map(|s| s.credential_id)
                                .collect::<Vec<_>>();
                            master::shared()
                                .ok_or(AppError::VaultUnavailable)?
                                .0
                                .lock()
                                .map_err(|_| AppError::VaultUnavailable)?
                                .create(&secret, &ids)?;
                            "Coffre chiffré créé ; secrets existants migrés."
                        }
                        Action::ChangeMaster(secret) => {
                            master::shared()
                                .ok_or(AppError::VaultUnavailable)?
                                .0
                                .lock()
                                .map_err(|_| AppError::VaultUnavailable)?
                                .change_password(&secret)?;
                            "Mot de passe maître modifié."
                        }
                        Action::LockVault => {
                            master::shared()
                                .ok_or(AppError::VaultUnavailable)?
                                .0
                                .lock()
                                .map_err(|_| AppError::VaultUnavailable)?
                                .lock();
                            "Coffre verrouillé."
                        }
                        Action::PreviewImport(file) => {
                            preview = Some(crate::importers::exchange::read_preview(
                                &file,
                                &service.db.snapshot()?,
                            )?);
                            "Aperçu prêt : vérifiez la sélection et les doublons."
                        }
                        Action::Import(data) => {
                            custom_message = Some(service.db.import(&data)?.summary());
                            "Import terminé."
                        }
                        Action::Export(file, ids, format) => {
                            let bytes = crate::importers::exchange::export(
                                &service.db.snapshot()?,
                                &ids,
                                format,
                            )?;
                            crate::importers::exchange::write_atomic(&file, &bytes)?;
                            custom_message = Some(format!(
                                "{} sessions exportées sans secrets vers {}.",
                                ids.len(),
                                file.display()
                            ));
                            "Export terminé."
                        }
                        Action::Refresh => "Données chargées.",
                        Action::MoveSession(id, folder) => {
                            service.db.move_session(&id, folder.as_deref())?;
                            "Session déplacée."
                        }
                        Action::CreateSessionFolder(data) => {
                            session_folder = Some(service.db.save_folder(None, &data)?);
                            "Dossier créé et sélectionné dans la session."
                        }
                        Action::SaveSession(id, data) => {
                            saved_id = Some(service.save(id.as_deref(), &data)?);
                            "Session enregistrée."
                        }
                        Action::SaveSessionSecret(id, data, password) => {
                            let id = service.save(id.as_deref(), &data)?;
                            if let Err(e) = service.set_secret(&id, password) {
                                warning = Some(format!(
                                    "Session enregistrée, mais le mot de passe n'a pas été enregistré : {e}"
                                ));
                            }
                            saved_id = Some(id);
                            "Session enregistrée."
                        }
                        Action::CreateGateway(target, data, password) => {
                            let id = service.create_gateway(&data, password)?;
                            gateway = Some((target, id));
                            "Passerelle SSH enregistrée."
                        }
                        Action::DeleteSession(id) => {
                            service.delete(&id)?;
                            "Session supprimée."
                        }
                        Action::SaveFolder(id, data) => {
                            service.db.save_folder(id.as_deref(), &data)?;
                            "Dossier enregistré."
                        }
                        Action::DeleteFolder(id) => {
                            service.db.delete_folder(&id)?;
                            "Dossier supprimé."
                        }
                        Action::SetSecret(id, secret) => {
                            service.set_secret(&id, secret)?;
                            "Mot de passe enregistré dans le coffre système."
                        }
                        Action::RemoveSecret(id) => {
                            service.remove_secret(&id)?;
                            "Mot de passe supprimé du coffre."
                        }
                        Action::Programs(programs) => {
                            service.db.set_programs(&programs)?;
                            "Applications externes enregistrées."
                        }
                        Action::Theme(theme) => {
                            service.db.set_theme(&theme)?;
                            "Thème enregistré."
                        }
                    };
                    Ok(Update {
                        gateway,
                        warning,
                        session_folder,
                        preview,
                        snapshot: service.db.snapshot()?,
                        message: custom_message.unwrap_or_else(|| message.into()),
                        saved_id,
                        path: path.clone(),
                        vault_ready: master::shared()
                            .is_some_and(|v| v.0.lock().is_ok_and(|v| v.ready())),
                    })
                })();
                if results.send(result).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        let _ = tx.send(Action::Refresh);
        Self { tx, rx }
    }
}
