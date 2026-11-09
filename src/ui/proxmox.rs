use super::*;
use crate::{
    credentials::{CredentialStore, Secret},
    error::{AppError, Result},
    proxmox::{Api, AuthMode, PowerAction, Server, SpiceConfig, Vm, VmKind},
};
use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver},
    time::Instant,
};
use zeroize::Zeroizing;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_secret_is_rejected_before_vault_or_preferences_write() {
        let mut app = super::super::sidebar::tests::sidebar_app();
        let server = Server {
            name: "Test".into(),
            url: "https://localhost:8006".into(),
            identity: "test@pve".into(),
            ..Default::default()
        };
        for secret in ["x".repeat(16_385), "invalid\nsecret".into()] {
            let error = app
                .proxmox_save(server.clone(), &secret, true)
                .err()
                .expect("invalid secret");
            assert!(error.to_string().contains("secret manquant ou invalide"));
            assert!(app.advanced.preferences.proxmox_servers.is_empty());
        }
    }
    #[test]
    fn expanded_server_keeps_login_draft_and_escape_does_not_reopen_it() {
        let mut app = super::super::sidebar::tests::sidebar_app();
        app.busy = false;
        let server = Server {
            name: "Test".into(),
            url: "https://localhost:8006".into(),
            identity: "test@pve".into(),
            ..Default::default()
        };
        app.advanced.preferences.proxmox_servers.push(server);
        app.tree_open = Some(true);
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(Default::default(), |ui| app.proxmox_tree(ui));
        output.textures_delta.clear();
        app.proxmox
            .login
            .as_mut()
            .expect("login")
            .secret
            .push_str("draft-secret");
        let mut output = ctx.run_ui(Default::default(), |ui| app.proxmox_tree(ui));
        output.textures_delta.clear();
        assert_eq!(
            app.proxmox.login.as_ref().expect("login").secret.as_str(),
            "draft-secret"
        );
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.dialog_shortcuts(ui.ctx()));
        output.textures_delta.clear();
        assert!(app.proxmox.login.is_none());
        let mut output = ctx.run_ui(Default::default(), |ui| app.proxmox_tree(ui));
        output.textures_delta.clear();
        assert!(app.proxmox.login.is_none());
    }

    #[test]
    fn power_escape_cancels_without_sending_a_command() {
        let mut app = super::super::sidebar::tests::sidebar_app();
        app.busy = false;
        let vm: Vm =
            serde_json::from_value(serde_json::json!({"vmid":100,"node":"pve","status":"running"}))
                .expect("fixture");
        app.proxmox.power = Some(PowerConfirmation {
            server: Server::default(),
            vm,
            action: PowerAction::PowerOff,
        });
        let ctx = egui::Context::default();
        let input = |key| egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
            ..Default::default()
        };
        let mut out = ctx.run_ui(input(egui::Key::Escape), |ui| {
            app.dialog_shortcuts(ui.ctx());
            app.hypervisor_actions(ui.ctx());
        });
        out.textures_delta.clear();
        assert!(app.proxmox.power.is_none());
        assert!(app.proxmox.jobs.is_empty());
        assert!(app.proxmox.login.is_none());
    }

    #[test]
    fn long_vm_rows_keep_sidebar_width_stable_without_background_jobs() {
        let mut app = super::super::sidebar::tests::sidebar_app();
        app.busy = false;
        app.tree_open = Some(true);
        let server = Server {
            name: "Hyper-V lab".into(),
            hyperv: true,
            ..Default::default()
        };
        let vm:Vm=serde_json::from_value(serde_json::json!({"kind":"HyperV","vmid":0,"native_id":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa","node":"node.example.test","name":"Long machine name ".repeat(10),"status":"running"})).expect("vm");
        app.proxmox.cache.insert(
            server.id.clone(),
            Cache {
                attempted: true,
                updated: Some(Instant::now()),
                vms: Some(vec![vm]),
                error: None,
                ..Default::default()
            },
        );
        app.advanced.preferences.proxmox_servers.push(server);
        let ctx = egui::Context::default();
        let mut widths = Vec::new();
        for frame in 0..40 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::PointerMoved(egui::pos2(
                        30.0 + frame as f32,
                        250.0,
                    ))],
                    ..Default::default()
                },
                |ui| widths.push(app.sidebar_panel(ui).response.rect.width()),
            );
            out.textures_delta.clear();
        }
        assert!(widths[10] < 350.0);
        assert!(widths[10..].iter().all(|w| (w - widths[10]).abs() < 1.0));
        assert!(app.proxmox.jobs.is_empty());
        assert!(app.proxmox.login.is_none());
    }
    #[test]
    fn state_indicators_distinguish_power_disabled_and_unknown() {
        let mut vm: Vm =
            serde_json::from_value(serde_json::json!({"vmid":1,"node":"pve","status":"running"}))
                .expect("vm");
        assert_eq!(
            vm_state_color(&vm, false),
            egui::Color32::from_rgb(40, 180, 75)
        );
        vm.status = "stopped".into();
        assert_eq!(
            vm_state_color(&vm, false),
            egui::Color32::from_rgb(220, 55, 55)
        );
        vm.status = "disabled".into();
        assert_eq!(vm_state_color(&vm, false), egui::Color32::BLACK);
        vm.status = "unknown".into();
        assert_eq!(
            vm_state_color(&vm, false),
            egui::Color32::from_rgb(240, 145, 35)
        );
        vm.status = "running".into();
        assert_eq!(
            vm_state_color(&vm, true),
            egui::Color32::from_rgb(240, 145, 35)
        );
        assert!(matches!(
            console_operation(&Vm {
                kind: VmKind::Lxc,
                ..vm
            }),
            Operation::LxcConsole(_)
        ));
    }
    #[test]
    fn console_failure_keeps_inventory_and_allows_another_console_without_refresh() {
        let mut app = super::super::sidebar::tests::sidebar_app();
        app.busy = false;
        let server = Server {
            name: "Fixture".into(),
            ..Default::default()
        };
        let vm: Vm =
            serde_json::from_value(serde_json::json!({"vmid":100,"node":"pve","status":"running"}))
                .expect("vm");
        let ct = Vm {
            vmid: 101,
            kind: VmKind::Lxc,
            ..vm.clone()
        };
        app.advanced
            .preferences
            .proxmox_servers
            .push(server.clone());
        app.proxmox
            .cache
            .entry(server.id.clone())
            .or_default()
            .refreshed(vec![vm.clone(), ct.clone()]);
        let updated = app.proxmox.cache[&server.id].updated;
        for operation in [
            Operation::Console(vm.vmid),
            Operation::LxcConsole(ct.clone()),
        ] {
            let (tx, receiver) = mpsc::channel();
            app.proxmox
                .cache
                .get_mut(&server.id)
                .expect("cache")
                .started(&operation);
            app.proxmox.jobs.insert(
                server.id.clone(),
                Job {
                    receiver,
                    operation,
                },
            );
            assert!(!app.proxmox.cache[&server.id].stale(&vm));
            assert!(
                tx.send(Err(AppError::Operation("synthetic console failure".into())))
                    .is_ok()
            );
            app.proxmox_poll(&egui::Context::default());
            let cache = &app.proxmox.cache[&server.id];
            assert!(cache.error.is_some());
            assert!(!cache.stale(&vm));
            assert!(!cache.stale(&ct));
            assert_eq!(cache.updated, updated);
            assert_eq!(cache.vms.as_ref().expect("vms").len(), 2);
            assert!(app.proxmox.jobs.is_empty());
        }
    }
    #[test]
    fn only_inventory_errors_invalidate_every_machine_and_power_uncertainty_is_scoped() {
        let vm: Vm =
            serde_json::from_value(serde_json::json!({"vmid":100,"node":"pve","status":"running"}))
                .expect("vm");
        let other = Vm {
            vmid: 101,
            ..vm.clone()
        };
        let rows = vec![vm.clone(), other.clone()];
        let mut cache = Cache::default();
        cache.refreshed(rows.clone());
        cache.started(&Operation::Refresh);
        assert!(!cache.stale(&vm));
        cache.failed(&Operation::Refresh, "offline".into());
        assert!(cache.stale(&vm));
        assert!(cache.stale(&other));
        cache.refreshed(rows.clone());
        let action = Operation::Power(vm.clone(), PowerAction::Shutdown);
        cache.started(&action);
        assert!(cache.stale(&vm));
        assert!(!cache.stale(&other));
        cache.failed(&action, "timeout".into());
        assert!(!cache.stale(&other));
        cache.refreshed(rows);
        assert!(!cache.stale(&vm));
        assert!(!cache.stale(&other));
    }
    #[test]
    fn locking_drops_pending_results_and_runtime_secrets() {
        let mut state = State::default();
        let (tx, rx) = mpsc::channel();
        state.jobs.insert(
            "test".into(),
            Job {
                receiver: rx,
                operation: Operation::Refresh,
            },
        );
        state
            .secrets
            .insert("test".into(), Zeroizing::new("synthetic-secret".into()));
        state.clear_pending();
        assert!(state.secrets.is_empty());
        assert!(tx.send(Ok(Completed::Vms(Vec::new()))).is_err());
    }
}

#[derive(Clone)]
enum Operation {
    Refresh,
    Console(u32),
    HyperConsole(Vm),
    LxcConsole(Vm),
    Power(Vm, PowerAction),
}
enum Completed {
    Vms(Vec<Vm>),
    Power(Vec<Vm>, String),
    LxcConsole(Box<crate::connections::proxmox_terminal::Console>, String),
    HyperConsole(Vm, crate::hyperv::ConsoleLaunch),
    Console(SpiceConfig, String, std::path::PathBuf, Instant),
}
#[derive(Default)]
struct Cache {
    attempted: bool,
    updated: Option<Instant>,
    vms: Option<Vec<Vm>>,
    error: Option<String>,
    inventory_stale: bool,
    uncertain: std::collections::HashSet<String>,
}
fn machine_key(vm: &Vm) -> String {
    if vm.kind == VmKind::HyperV {
        vm.native_id.clone()
    } else {
        format!("{}:{}", vm.kind.api_name(), vm.vmid)
    }
}
impl Cache {
    fn stale(&self, vm: &Vm) -> bool {
        self.inventory_stale || self.uncertain.contains(&machine_key(vm))
    }
    fn started(&mut self, operation: &Operation) {
        self.error = None;
        if let Operation::Power(vm, _) = operation {
            self.uncertain.insert(machine_key(vm));
        }
    }
    fn failed(&mut self, operation: &Operation, message: String) {
        self.error = Some(message);
        match operation {
            Operation::Refresh => self.inventory_stale = true,
            Operation::Power(vm, _) => {
                self.uncertain.insert(machine_key(vm));
            }
            _ => {}
        }
    }
    fn refreshed(&mut self, vms: Vec<Vm>) {
        self.vms = Some(vms);
        self.updated = Some(Instant::now());
        self.inventory_stale = false;
        self.uncertain.clear();
        self.error = None;
    }
}
struct Job {
    receiver: Receiver<Result<Completed>>,
    operation: Operation,
}
pub(super) struct Editor {
    server: Server,
    secret: Secret,
    save: bool,
}
pub(super) struct Login {
    server: Server,
    operation: Operation,
    secret: Secret,
    save: bool,
}
pub(super) struct PowerConfirmation {
    server: Server,
    vm: Vm,
    action: PowerAction,
}

#[derive(Default)]
pub(super) struct State {
    pub power: Option<PowerConfirmation>,
    pub editor: Option<Editor>,
    pub login: Option<Login>,
    pub delete: Option<String>,
    cache: HashMap<String, Cache>,
    jobs: HashMap<String, Job>,
    secrets: HashMap<String, Secret>,
    open_next: Option<String>,
}
impl State {
    pub fn clear_pending(&mut self) {
        self.power = None;
        self.jobs.clear();
        self.secrets.clear();
        self.login = None;
        self.editor = None;
    }
    pub fn cancel_dialog(&mut self, target: &str) {
        match target {
            "vm_power" => self.power = None,
            "proxmox_editor" => self.editor = None,
            "proxmox_login" => self.login = None,
            "proxmox_delete" => self.delete = None,
            _ => {}
        }
    }
}

impl HostGates {
    pub(super) fn proxmox_tree(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Proxmox / Hyper-V");
            if ui
                .small_button(crate::i18n::tr("Ajouter un serveur"))
                .clicked()
            {
                self.proxmox.editor = Some(Editor {
                    server: Server::default(),
                    secret: Zeroizing::new(String::new()),
                    save: true,
                });
            }
        });
        let servers = self.advanced.preferences.proxmox_servers.clone();
        let search = self.search.trim().to_lowercase();
        let mut action = None;
        let can_auto_load = self.proxmox.login.is_none()
            && self.proxmox.editor.is_none()
            && self.proxmox.delete.is_none()
            && self.proxmox.power.is_none()
            && search.is_empty();
        for server in servers {
            let server_matches = search.is_empty()
                || server.name.to_lowercase().contains(&search)
                || server.url.to_lowercase().contains(&search);
            let loading = self.proxmox.jobs.contains_key(&server.id);
            let cache = self.proxmox.cache.entry(server.id.clone()).or_default();
            let force = if self.proxmox.open_next.as_deref() == Some(&server.id) {
                self.proxmox.open_next = None;
                Some(true)
            } else if !search.is_empty() {
                Some(true)
            } else {
                self.tree_open
            };
            let response = egui::CollapsingHeader::new(&server.name)
                .id_salt(("proxmox_server", &server.id))
                .default_open(false)
                .open(force)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!loading, egui::Button::new(crate::i18n::tr("Actualiser")))
                            .clicked()
                        {
                            action = Some((server.clone(), Operation::Refresh));
                        }
                        if ui
                            .add_enabled(!loading, egui::Button::new(crate::i18n::tr("Modifier")))
                            .clicked()
                        {
                            self.proxmox.editor = Some(Editor {
                                server: server.clone(),
                                secret: Zeroizing::new(String::new()),
                                save: true,
                            });
                        }
                        if ui
                            .add_enabled(!loading, egui::Button::new(crate::i18n::tr("Supprimer")))
                            .clicked()
                        {
                            self.proxmox.delete = Some(server.id.clone());
                        }
                    });
                    if loading {
                        ui.spinner();
                    }
                    if let Some(error) = &cache.error {
                        ui.label(crate::i18n::tr(error));
                    }
                    if cache.inventory_stale {
                        ui.small(crate::i18n::tr(
                            "Inventaire non actualisé : les états affichés peuvent avoir changé.",
                        ));
                    }
                    if cache.updated.is_some() {
                        ui.small(crate::i18n::tr("État à la dernière actualisation"));
                    }
                    if let Some(vms) = &cache.vms {
                        if vms.is_empty() {
                            ui.label(crate::i18n::tr("Aucune VM visible avec ces identifiants."));
                        }
                        for vm in vms {
                            if !server_matches
                                && !vm.label().to_lowercase().contains(&search)
                                && !vm.node.to_lowercase().contains(&search)
                            {
                                continue;
                            }
                            let stale = cache.stale(vm);
                            let row = ui
                                .horizontal(|ui| {
                                    let (rect, indicator) = ui.allocate_exact_size(
                                        egui::vec2(12.0, 18.0),
                                        egui::Sense::click(),
                                    );
                                    ui.painter().circle(
                                        rect.center(),
                                        4.5,
                                        vm_state_color(vm, stale),
                                        egui::Stroke::new(0.7, egui::Color32::GRAY),
                                    );
                                    let label = ui.selectable_label(
                                        false,
                                        format!(
                                            "{} · {} · {} ({})",
                                            crate::i18n::tr(if stale {
                                                "État inconnu"
                                            } else {
                                                vm.state_label()
                                            }),
                                            vm.kind.api_name(),
                                            vm.label(),
                                            vm.node
                                        ),
                                    );
                                    label.union(indicator)
                                })
                                .inner;
                            if row.double_clicked()
                                && !loading
                                && vm.connectable()
                                && !cache.stale(vm)
                            {
                                action = Some((server.clone(), console_operation(vm)));
                            }
                            row.context_menu(|ui| {
                                if ui
                                    .add_enabled(
                                        !loading && !cache.stale(vm) && vm.connectable(),
                                        egui::Button::new(crate::i18n::tr(
                                            if vm.kind == VmKind::Lxc {
                                                "Ouvrir la console du conteneur"
                                            } else if vm.kind == VmKind::HyperV {
                                                "Ouvrir VMConnect"
                                            } else {
                                                "Ouvrir la console SPICE"
                                            },
                                        )),
                                    )
                                    .clicked()
                                {
                                    action = Some((server.clone(), console_operation(vm)));
                                    ui.close();
                                }
                                ui.separator();
                                for command in [
                                    PowerAction::Start,
                                    PowerAction::Shutdown,
                                    PowerAction::Reboot,
                                    PowerAction::PowerOff,
                                ] {
                                    if ui
                                        .add_enabled(
                                            !loading && !cache.stale(vm) && command.available(vm),
                                            egui::Button::new(crate::i18n::tr(command.label())),
                                        )
                                        .clicked()
                                    {
                                        self.proxmox.power = Some(PowerConfirmation {
                                            server: server.clone(),
                                            vm: vm.clone(),
                                            action: command,
                                        });
                                        ui.close();
                                    }
                                }
                            });
                        }
                    } else if can_auto_load
                        && !cache.attempted
                        && !loading
                        && cache.error.is_none()
                        && action.is_none()
                    {
                        action = Some((server.clone(), Operation::Refresh));
                    }
                });
            response.header_response.on_hover_text(&server.url);
        }
        ui.separator();
        if let Some((server, operation)) = action {
            self.proxmox_request(server, operation, ui.ctx());
        }
    }

    fn proxmox_request(&mut self, server: Server, operation: Operation, ctx: &egui::Context) {
        if self.proxmox.jobs.contains_key(&server.id) {
            return;
        }
        self.proxmox
            .cache
            .entry(server.id.clone())
            .or_default()
            .attempted = true;
        let secret = if let Some(secret) = self.proxmox.secrets.get(&server.id) {
            Ok(Some(secret.clone()))
        } else if let Some(id) = &server.credential_id {
            crate::credentials::master::shared()
                .ok_or(AppError::VaultUnavailable)
                .and_then(|mut vault| vault.get_secret(id))
        } else {
            Ok(None)
        };
        match secret {
            Ok(Some(secret)) => self.proxmox_start(server, operation, secret, ctx),
            Ok(None) => {
                self.proxmox.login = Some(Login {
                    server,
                    operation,
                    secret: Zeroizing::new(String::new()),
                    save: false,
                })
            }
            Err(e) => self
                .proxmox
                .cache
                .entry(server.id)
                .or_default()
                .failed(&operation, e.to_string()),
        }
    }
    fn proxmox_start(
        &mut self,
        server: Server,
        operation: Operation,
        secret: Secret,
        ctx: &egui::Context,
    ) {
        let program = if matches!(operation, Operation::Console(_)) {
            match crate::connections::resolve_program(
                &self.snapshot.programs.spice,
                crate::connections::ProgramKind::Spice,
            ) {
                Ok(p) => Some(p),
                Err(e) => {
                    self.error = Some(e.to_string());
                    return;
                }
            }
        } else {
            None
        };
        let (tx, rx) = mpsc::channel();
        self.proxmox
            .cache
            .entry(server.id.clone())
            .or_default()
            .started(&operation);
        self.proxmox.jobs.insert(
            server.id.clone(),
            Job {
                receiver: rx,
                operation: operation.clone(),
            },
        );
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                if server.hyperv {
                    return match operation {
                        Operation::Refresh => Ok(Completed::Vms(crate::hyperv::run(
                            &server, &secret, "list", "",
                        )?)),
                        Operation::HyperConsole(vm) => {
                            let fresh =
                                crate::hyperv::run(&server, &secret, "console", &vm.native_id)?
                                    .into_iter()
                                    .next()
                                    .ok_or(AppError::NotFound)?;
                            let plan = crate::hyperv::console_launch(&server, &fresh, &secret)?;
                            Ok(Completed::HyperConsole(fresh, plan))
                        }
                        Operation::Power(vm, action) => {
                            crate::hyperv::run(&server, &secret, action.api_name(), &vm.native_id)?;
                            Ok(Completed::Power(
                                crate::hyperv::run(&server, &secret, "list", "")?,
                                "Commande Hyper-V terminée.".into(),
                            ))
                        }
                        _ => Err(AppError::NotFound),
                    };
                }
                let api = Api::login(&server, &secret)?;
                match operation {
                    Operation::HyperConsole(_) => Err(AppError::NotFound),
                    Operation::LxcConsole(vm) => Ok(Completed::LxcConsole(
                        Box::new(api.lxc_terminal(&server, &vm)?),
                        format!("{} · {}", server.name, vm.label()),
                    )),
                    Operation::Power(vm, action) => {
                        let task = api.power(&vm, action)?;
                        api.wait_power_task(&task)?;
                        Ok(Completed::Power(
                            api.vms()?,
                            "Proxmox : tâche terminée, état actualisé.".into(),
                        ))
                    }
                    Operation::Refresh => Ok(Completed::Vms(api.vms()?)),
                    Operation::Console(vmid) => {
                        let config = api.console(&server, vmid)?;
                        let received = Instant::now();
                        config.check_proxy()?;
                        Ok(Completed::Console(
                            config,
                            format!("{} · VM {vmid}", server.name),
                            program.ok_or(AppError::NotFound)?,
                            received,
                        ))
                    }
                }
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    fn proxmox_poll(&mut self, ctx: &egui::Context) {
        let completed: Vec<_> = self
            .proxmox
            .jobs
            .iter()
            .filter_map(|(id, job)| match job.receiver.try_recv() {
                Ok(result) => Some((id.clone(), result)),
                Err(mpsc::TryRecvError::Disconnected) => Some((
                    id.clone(),
                    Err(AppError::Operation(
                        "Proxmox : opération interrompue.".into(),
                    )),
                )),
                Err(mpsc::TryRecvError::Empty) => None,
            })
            .collect();
        for (id, result) in completed {
            let Some(job) = self.proxmox.jobs.remove(&id) else {
                continue;
            };
            if !self
                .advanced
                .preferences
                .proxmox_servers
                .iter()
                .any(|s| s.id == id)
            {
                continue;
            }
            match result {
                Ok(Completed::LxcConsole(console, title)) => {
                    let terminal =
                        crate::connections::terminal::Terminal::from_proxmox(*console, ctx.clone());
                    let tab_id = uuid::Uuid::new_v4().to_string();
                    self.active_tab = Some(tab_id.clone());
                    self.tabs.push(tabs::SessionTab {
                        id: tab_id,
                        title,
                        session: None,
                        files: None,
                        tunnel: None,
                        client: tabs::TabClient::Terminal(
                            Box::new(terminal),
                            super::terminal::TerminalView::default(),
                        ),
                    });
                }
                Ok(Completed::HyperConsole(vm, program)) => {
                    match crate::connections::rdp::RdpSession::launch_hyperv(program) {
                        Ok(client) => {
                            let tab_id = uuid::Uuid::new_v4().to_string();
                            self.active_tab = Some(tab_id.clone());
                            self.tabs.push(tabs::SessionTab {
                                id: tab_id,
                                title: vm.name,
                                session: None,
                                files: None,
                                tunnel: None,
                                client: tabs::TabClient::Rdp(client),
                            });
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
                Ok(Completed::Power(vms, message)) => {
                    self.status = message;
                    self.proxmox.cache.entry(id).or_default().refreshed(vms);
                }
                Ok(Completed::Vms(vms)) => self.proxmox.cache.entry(id).or_default().refreshed(vms),
                Ok(Completed::Console(config, title, program, received)) => {
                    let client = if received.elapsed().as_secs() >= 20 {
                        Err(AppError::Operation(
                            "SPICE : ticket expiré, relancez la connexion.".into(),
                        ))
                    } else {
                        config.launch(&program)
                    };
                    match client {
                        Ok(client) => {
                            let tab_id = uuid::Uuid::new_v4().to_string();
                            self.active_tab = Some(tab_id.clone());
                            self.tabs.push(tabs::SessionTab {
                                id: tab_id,
                                title,
                                session: None,
                                files: None,
                                tunnel: None,
                                client: tabs::TabClient::Rdp(client),
                            });
                        }
                        Err(e) => {
                            self.proxmox.cache.entry(id).or_default().error = Some(e.to_string())
                        }
                    }
                }
                Err(e) => self
                    .proxmox
                    .cache
                    .entry(id)
                    .or_default()
                    .failed(&job.operation, e.to_string()),
            }
        }
    }

    /// New secrets get a new vault reference. A failed preferences write never replaces
    /// an existing saved credential. The original reference remains valid on rollback.
    fn proxmox_save(&mut self, mut server: Server, secret: &str, save: bool) -> Result<Server> {
        server.validate()?;
        if secret.len() > 16_384 || secret.contains(['\r', '\n', '\0']) {
            return Err(AppError::Validation(
                "Proxmox : secret manquant ou invalide.".into(),
            ));
        }
        let old_servers = self.advanced.preferences.proxmox_servers.clone();
        let previous = old_servers.iter().find(|s| s.id == server.id);
        let old_id = previous.and_then(|s| s.credential_id.clone());
        let mut new_id = None;
        if !secret.is_empty() && save {
            let id = uuid::Uuid::new_v4().to_string();
            crate::credentials::master::shared()
                .ok_or(AppError::VaultUnavailable)?
                .store_secret(&id, secret)?;
            server.credential_id = Some(id.clone());
            new_id = Some(id);
        } else if !secret.is_empty() {
            server.credential_id = None;
        } else if previous.is_some_and(|p| {
            p.url != server.url
                || p.identity != server.identity
                || p.auth != server.auth
                || p.hyperv != server.hyperv
        }) {
            return Err(AppError::Validation(
                "Proxmox : ressaisissez le secret après un changement d’adresse ou de compte."
                    .into(),
            ));
        }
        self.advanced
            .preferences
            .proxmox_servers
            .retain(|s| s.id != server.id);
        self.advanced
            .preferences
            .proxmox_servers
            .push(server.clone());
        if let Err(e) = self.advanced.save() {
            self.advanced.preferences.proxmox_servers = old_servers;
            if let Some(id) = new_id
                && let Some(mut vault) = crate::credentials::master::shared()
            {
                let _ = vault.delete_secret(&id);
            }
            return Err(e);
        }
        if old_id != server.credential_id
            && let Some(id) = old_id
            && let Some(mut vault) = crate::credentials::master::shared()
        {
            // A failed cleanup leaves only an unreferenced encrypted entry, never a broken profile.
            if let Err(e) = vault.delete_secret(&id) {
                self.error = Some(e.to_string());
            }
        }
        if !secret.is_empty() && !save {
            self.proxmox
                .secrets
                .insert(server.id.clone(), Zeroizing::new(secret.to_owned()));
        } else {
            self.proxmox.secrets.remove(&server.id);
        }
        self.proxmox.jobs.remove(&server.id);
        self.proxmox.cache.remove(&server.id);
        Ok(server)
    }
    fn proxmox_remove(&mut self, id: &str) -> Result<()> {
        let previous = self.advanced.preferences.proxmox_servers.clone();
        let server = previous
            .iter()
            .find(|s| s.id == id)
            .ok_or(AppError::NotFound)?;
        // Check vault access before committing removal, so a locked vault does not hide the profile.
        if let Some(credential) = &server.credential_id {
            crate::credentials::master::shared()
                .ok_or(AppError::VaultUnavailable)?
                .get_secret(credential)?;
        }
        self.advanced
            .preferences
            .proxmox_servers
            .retain(|s| s.id != id);
        if let Err(e) = self.advanced.save() {
            self.advanced.preferences.proxmox_servers = previous;
            return Err(e);
        }
        if let Some(credential) = &server.credential_id
            && let Err(e) = crate::credentials::master::shared()
                .ok_or(AppError::VaultUnavailable)?
                .delete_secret(credential)
        {
            self.advanced.preferences.proxmox_servers = previous;
            self.advanced.save()?;
            return Err(e);
        }
        self.proxmox.jobs.remove(id);
        self.proxmox.cache.remove(id);
        self.proxmox.secrets.remove(id);
        Ok(())
    }

    pub(super) fn proxmox_dialogs(&mut self, ctx: &egui::Context) {
        self.proxmox_poll(ctx);
        self.hypervisor_actions(ctx);
        if let Some(mut draft) = self.proxmox.editor.take() {
            let mut open = true;
            let mut save = false;
            let mut cancel = false;
            egui::Window::new(crate::i18n::tr("Serveur de virtualisation"))
                .id(egui::Id::new("proxmox_editor"))
                .open(&mut open)
                .collapsible(false)
                .default_width(480.0)
                .show(ctx, |ui| {
                    ui.label(crate::i18n::tr("Nom"));
                    ui.text_edit_singleline(&mut draft.server.name);
                    ui.horizontal(|ui| {
                        if ui.selectable_value(&mut draft.server.hyperv, false, "Proxmox").changed() { draft.server.url = "https://".into(); draft.server.skip_tls_verification = false; }
                        if ui.selectable_value(&mut draft.server.hyperv, true, "Hyper-V").changed() { draft.server.url = "https://".into(); draft.server.skip_tls_verification = false; draft.server.auth = AuthMode::Password; }
                    });
                    ui.label(if draft.server.hyperv { if cfg!(windows) { "WinRM : https://hote:5986 (HTTP/Kerberos : http://hote:5985)" } else { "WinRM HTTPS : https://hote:5986 (NTLM)" } } else { "URL HTTPS (https://pve.example:8006)" });
                    ui.text_edit_singleline(&mut draft.server.url);
                    if !draft.server.hyperv { ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut draft.server.auth,
                            AuthMode::Password,
                            crate::i18n::tr("Mot de passe"),
                        );
                        ui.selectable_value(
                            &mut draft.server.auth,
                            AuthMode::ApiToken,
                            crate::i18n::tr("Jeton API"),
                        );
                    });
                    }
                    ui.label(if draft.server.hyperv { "DOMAINE\\utilisateur / utilisateur@domaine" } else if draft.server.auth == AuthMode::ApiToken {
                        "utilisateur@realm!jeton"
                    } else {
                        "utilisateur@realm"
                    });
                    ui.text_edit_singleline(&mut draft.server.identity);
                    ui.label(crate::i18n::tr(
                        "Secret (vide : conserver celui enregistré)",
                    ));
                    ui.add(egui::TextEdit::singleline(&mut *draft.secret).password(true));
                    ui.checkbox(
                        &mut draft.save,
                        crate::i18n::tr("Enregistrer dans le coffre"),
                    );
                    if draft.server.hyperv {
                        ui.checkbox(&mut draft.server.cluster, crate::i18n::tr("Lister tout le cluster depuis cet hôte"));
                        #[cfg(windows)]
                        {
                        ui.small(crate::i18n::tr("WinRM et les modules Hyper-V / FailoverClusters doivent être configurés sur les hôtes. VMConnect utilise le compte Windows courant."));
                        program_picker(ui, "VMConnect.exe", &mut draft.server.vmconnect);
                        }
                        #[cfg(not(windows))]
                        {
                        ui.small(crate::i18n::tr("Linux : WinRM HTTPS natif et FreeRDP. Aucun Python requis. Consultez l’aide Hyper-V."));
                        program_picker(ui, "FreeRDP (Hyper-V)", &mut draft.server.hyperv_freerdp);
                        program_picker(ui, &crate::i18n::tr("Certificat CA PEM (facultatif)"), &mut draft.server.ca_file);
                        }

                    } else {
                    ui.checkbox(&mut draft.server.skip_tls_verification, crate::i18n::tr("Ne pas vérifier le certificat HTTPS"));
                    if draft.server.skip_tls_verification {
                        ui.colored_label(ui.visuals().warn_fg_color, crate::i18n::tr("Attention : l’identité du serveur n’est plus vérifiée. Vos identifiants peuvent être interceptés. Le certificat CA configuré est ignoré."));
                    }
                    ui.add_enabled_ui(!draft.server.skip_tls_verification, |ui| {
                    ui.label(crate::i18n::tr("Certificat CA PEM (facultatif)"));
                    ui.horizontal(|ui| {
                        ui.label(draft.server.ca_file.display().to_string());
                        if ui.button(crate::i18n::tr("Parcourir…")).clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("PEM", &["pem", "crt"])
                                .pick_file()
                        {
                            draft.server.ca_file = path;
                        }
                        if ui.button(crate::i18n::tr("Effacer")).clicked() {
                            draft.server.ca_file.clear();
                        }
                    });
                    });
                    ui.label(crate::i18n::tr(
                        "Hôte du proxy SPICE (vide : hôte de l’API)",
                    ));
                    ui.text_edit_singleline(&mut draft.server.proxy_host);
                    ui.small(crate::i18n::tr(
                        "Droits : VM.Audit, VM.Console pour SPICE et VM.PowerMgmt pour les actions d’alimentation.",
                    ));
                    }
                    ui.horizontal(|ui| {
                        save = ui
                            .add_enabled(
                                !self.busy,
                                egui::Button::new(crate::i18n::tr("Enregistrer")),
                            )
                            .clicked()
                            || (!self.busy && super::shortcuts::submitted(ctx, "proxmox_editor"));
                        cancel = ui.button(crate::i18n::tr("Annuler")).clicked();
                    });
                });
            if open && !cancel {
                if save {
                    match self.proxmox_save(draft.server.clone(), &draft.secret, draft.save) {
                        Ok(server) => {
                            self.proxmox.open_next = Some(server.id.clone());
                            self.proxmox_request(server, Operation::Refresh, ctx);
                        }
                        Err(e) => {
                            self.error = Some(e.to_string());
                            self.proxmox.editor = Some(draft);
                        }
                    }
                } else {
                    self.proxmox.editor = Some(draft);
                }
            }
        }
        if let Some(mut login) = self.proxmox.login.take() {
            let mut open = true;
            let mut connect = false;
            egui::Window::new(crate::i18n::tr("Authentification de l’hyperviseur"))
                .id(egui::Id::new("proxmox_login"))
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(&login.server.name);
                    ui.label(&login.server.identity);
                    ui.add(egui::TextEdit::singleline(&mut *login.secret).password(true));
                    ui.checkbox(
                        &mut login.save,
                        crate::i18n::tr("Enregistrer dans le coffre"),
                    );
                    connect = ui
                        .add_enabled(
                            !self.busy && !login.secret.is_empty(),
                            egui::Button::new(crate::i18n::tr("Connexion")),
                        )
                        .clicked()
                        || (!self.busy
                            && !login.secret.is_empty()
                            && super::shortcuts::submitted(ctx, "proxmox_login"));
                });
            if open {
                if connect {
                    match self.proxmox_save(login.server.clone(), &login.secret, login.save) {
                        Ok(server) => {
                            self.proxmox_start(server, login.operation, login.secret, ctx)
                        }
                        Err(e) => {
                            self.error = Some(e.to_string());
                            self.proxmox.login = Some(login);
                        }
                    }
                } else {
                    self.proxmox.login = Some(login);
                }
            }
        }
        if let Some(id) = self.proxmox.delete.clone() {
            let mut open = true;
            let mut remove = false;
            egui::Window::new(crate::i18n::tr("Supprimer le serveur de virtualisation ?")).id(egui::Id::new("proxmox_delete")).open(&mut open).collapsible(false).show(ctx, |ui| {
                ui.label(crate::i18n::tr("Cette action retire la configuration locale et son secret, sans modifier les VM."));
                remove = ui.add_enabled(!self.busy,egui::Button::new(crate::i18n::tr("Supprimer"))).clicked() || (!self.busy && super::shortcuts::submitted(ctx,"proxmox_delete"));
            });
            if remove {
                if let Err(e) = self.proxmox_remove(&id) {
                    self.error = Some(e.to_string());
                } else {
                    open = false;
                }
            }
            if !open {
                self.proxmox.delete = None;
            }
        }
    }
}

fn console_operation(vm: &Vm) -> Operation {
    match vm.kind {
        VmKind::Qemu => Operation::Console(vm.vmid),
        VmKind::Lxc => Operation::LxcConsole(vm.clone()),
        VmKind::HyperV => Operation::HyperConsole(vm.clone()),
    }
}
impl HostGates {
    fn hypervisor_actions(&mut self, ctx: &egui::Context) {
        if let Some(confirm) = self.proxmox.power.take() {
            let mut open = true;
            let mut execute = false;
            egui::Window::new(crate::i18n::tr("Confirmer l’action sur la machine"))
                .id(egui::Id::new("vm_power")).open(&mut open).collapsible(false).show(ctx, |ui| {
                    ui.label(format!("{} — {} ({})", confirm.server.name, confirm.vm.label(), confirm.vm.node));
                    ui.strong(crate::i18n::tr(confirm.action.label()));
                    if confirm.action == PowerAction::PowerOff { ui.colored_label(ui.visuals().warn_fg_color, crate::i18n::tr("Coupure brutale : les données non enregistrées peuvent être perdues.")); }
                    execute = ui.add_enabled(!self.busy, egui::Button::new(crate::i18n::tr("Confirmer"))).clicked()
                        || (!self.busy && super::shortcuts::submitted(ctx,"vm_power"));
                });
            if open && execute {
                self.proxmox_request(
                    confirm.server,
                    Operation::Power(confirm.vm, confirm.action),
                    ctx,
                );
            } else if open {
                self.proxmox.power = Some(confirm);
            }
        }
    }
}

fn vm_state_color(vm: &Vm, stale: bool) -> egui::Color32 {
    if stale {
        return egui::Color32::from_rgb(240, 145, 35);
    }
    if vm.template != 0 || vm.status == "disabled" {
        return egui::Color32::BLACK;
    }
    match vm.status.as_str() {
        "running" => egui::Color32::from_rgb(40, 180, 75),
        "stopped" => egui::Color32::from_rgb(220, 55, 55),
        _ => egui::Color32::from_rgb(240, 145, 35),
    }
}

fn program_picker(ui: &mut egui::Ui, label: &str, path: &mut std::path::PathBuf) {
    ui.label(label);
    ui.horizontal(|ui| {
        ui.label(path.display().to_string());
        if ui.button(crate::i18n::tr("Parcourir…")).clicked()
            && let Some(selected) = rfd::FileDialog::new().pick_file()
        {
            *path = selected;
        }
        if ui.button(crate::i18n::tr("Effacer")).clicked() {
            path.clear();
        }
    });
}
