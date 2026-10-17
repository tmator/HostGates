mod advanced;
mod chrome;
mod shortcuts;
pub fn locale_dir() -> std::path::PathBuf {
    advanced::data_dir().join("locales")
}
mod dialogs;
mod editor;
mod exchange;
mod fonts;
mod help;
mod sessions;
mod sidebar;
mod tabs;
pub mod terminal;
use crate::{
    branding::APP_NAME,
    models::{Session, Snapshot},
    services::worker::{Action, Worker},
};
use editor::{FolderEditor, SessionEditor};
use eframe::egui::{self, Color32, RichText};

pub struct HostGates {
    advanced: advanced::Advanced,
    worker: Worker,
    snapshot: Snapshot,
    busy: bool,
    error: Option<String>,
    status: String,
    search: String,
    selected: Option<String>,
    folder: Option<String>,
    favorites: bool,
    tree_open: Option<bool>,
    editor: Option<SessionEditor>,
    folder_editor: Option<FolderEditor>,
    confirm_delete: Option<(String, bool)>,
    settings: bool,
    programs_draft: Option<crate::models::ExternalPrograms>,
    programs_check: Option<String>,
    path: String,
    vault_ready: bool,
    tabs: Vec<tabs::SessionTab>,
    active_tab: Option<String>,
    catalog_open: bool,
    preview: Option<crate::importers::Preview>,
    importing: bool,
    export_open: bool,
    export_ids: Vec<String>,
    export_format: crate::importers::exchange::ExportFormat,
    help_open: bool,
    about_open: bool,
    startup_checked: bool,
    unlock_open: bool,
    unlock_password: crate::credentials::Secret,
    help_topic: usize,
}
impl HostGates {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        fonts::install(&cc.egui_ctx, "fr");
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(7.0, 5.0);
            style.spacing.button_padding = egui::vec2(8.0, 4.0);
            let dark = style.visuals.dark_mode;
            let accent = Color32::from_rgb(0, 103, 192);
            style.visuals.panel_fill = if dark {
                Color32::from_gray(32)
            } else {
                Color32::from_rgb(249, 249, 249)
            };
            style.visuals.window_fill = if dark {
                Color32::from_gray(40)
            } else {
                Color32::WHITE
            };
            style.visuals.extreme_bg_color = if dark {
                Color32::from_gray(25)
            } else {
                Color32::WHITE
            };
            style.visuals.selection.bg_fill = if dark {
                Color32::from_rgb(35, 64, 88)
            } else {
                Color32::from_rgb(229, 241, 251)
            };
            style.visuals.selection.stroke = egui::Stroke::new(
                1.0,
                if dark {
                    Color32::from_rgb(120, 190, 250)
                } else {
                    accent
                },
            );
            style.visuals.widgets.inactive.bg_fill = if dark {
                Color32::from_gray(48)
            } else {
                Color32::from_gray(243)
            };
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(
                1.0,
                if dark {
                    Color32::from_gray(65)
                } else {
                    Color32::from_gray(215)
                },
            );
            style.visuals.widgets.hovered.bg_fill = if dark {
                Color32::from_gray(60)
            } else {
                Color32::from_gray(234)
            };
            style.visuals.widgets.active.bg_fill = if dark {
                Color32::from_gray(70)
            } else {
                Color32::from_gray(225)
            };
            style.visuals.window_corner_radius = egui::CornerRadius::same(8);
        });
        Self {
            advanced: advanced::Advanced::load(),
            worker: Worker::start(cc.egui_ctx.clone()),
            snapshot: Snapshot {
                programs: Default::default(),
                sessions: vec![],
                folders: vec![],
                theme: "system".into(),
            },
            busy: true,
            error: None,
            status: "Ouverture de la base locale…".into(),
            search: String::new(),
            selected: None,
            folder: None,
            favorites: false,
            tree_open: None,
            editor: None,
            folder_editor: None,
            confirm_delete: None,
            settings: false,
            programs_draft: None,
            programs_check: None,
            path: String::new(),
            vault_ready: false,
            tabs: vec![],
            active_tab: None,
            catalog_open: false,
            preview: None,
            importing: false,
            export_open: false,
            export_ids: vec![],
            export_format: crate::importers::exchange::ExportFormat::Csv,
            help_open: false,
            about_open: false,
            startup_checked: false,
            unlock_open: false,
            unlock_password: zeroize::Zeroizing::new(String::new()),
            help_topic: 0,
        }
    }
    fn send(&mut self, action: Action) {
        if self.busy {
            return;
        }
        match self.worker.tx.send(action) {
            Ok(()) => {
                self.busy = true;
                self.error = None;
            }
            Err(_) => {
                self.error = Some("Le service local s'est arrêté. Redémarrez HostGates.".into())
            }
        }
    }
    fn receive(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.worker.rx.try_recv() {
            self.busy = false;
            match result {
                Ok(update) => {
                    if self.importing {
                        self.preview = None;
                    }
                    if let Some(preview) = update.preview {
                        self.preview = Some(preview);
                    }
                    if let Some(id) = update.session_folder
                        && let Some(editor) = &mut self.editor
                    {
                        editor.select_created_folder(id);
                    }
                    self.snapshot = update.snapshot;
                    if let Some((target, gateway)) = update.gateway {
                        self.advanced
                            .preferences
                            .sessions
                            .entry(target)
                            .or_default()
                            .gateway = Some(gateway);
                        if let Err(e) = self.advanced.save() {
                            self.error = Some(e.to_string());
                        }
                    }
                    self.status = update.message;
                    if let Some(warning) = update.warning {
                        self.error = Some(warning);
                    }
                    self.path = update.path.display().to_string();
                    self.vault_ready = update.vault_ready;
                    if let Some(id) = update.saved_id {
                        self.selected = Some(id);
                        self.editor = None;
                    }
                    if self.status == "Dossier enregistré." {
                        self.folder_editor = None;
                    }
                    if self
                        .selected
                        .as_ref()
                        .is_some_and(|id| !self.snapshot.sessions.iter().any(|s| &s.uuid == id))
                    {
                        self.selected = None;
                    }
                    if self
                        .folder
                        .as_ref()
                        .is_some_and(|id| !self.snapshot.folders.iter().any(|f| &f.uuid == id))
                    {
                        self.folder = None;
                    }
                }
                Err(e) => self.error = Some(e.to_string()),
            }
            self.importing = false;
        }
        ctx.set_theme(match self.snapshot.theme.as_str() {
            "dark" => egui::ThemePreference::Dark,
            "light" => egui::ThemePreference::Light,
            _ => egui::ThemePreference::System,
        });
    }
    fn new_session(&mut self) {
        self.editor = Some(SessionEditor::new(self.folder.clone()));
    }
    fn selected_session(&self) -> Option<Session> {
        self.snapshot
            .sessions
            .iter()
            .find(|s| Some(&s.uuid) == self.selected.as_ref())
            .cloned()
    }
}

impl eframe::App for HostGates {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| !i.events.is_empty()) {
            self.advanced.last_activity = std::time::Instant::now();
        }
        self.auto_lock();
        if crate::i18n::set_language(&self.advanced.preferences.language) {
            fonts::install(&ctx, &self.advanced.preferences.language);
        }
        self.receive(&ctx);
        if !self.startup_checked
            && !self.busy
            && let Some(state) = crate::credentials::master::shared()
                .and_then(|v| v.0.try_lock().ok().map(|v| (v.configured(), v.ready())))
        {
            self.startup_checked = true;
            self.unlock_open = state.0 && !state.1;
        }
        if self.vault_ready {
            self.unlock_open = false;
        }
        self.dialog_shortcuts(&ctx);
        if self.active_tab.is_none()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::N))
            && !self.busy
        {
            self.new_session();
        }
        if self.active_tab.is_none()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F1))
        {
            self.help_open = true;
        }
        for tab in &mut self.tabs {
            match &mut tab.client {
                tabs::TabClient::Terminal(t, _) => t.poll(),
                tabs::TabClient::Rdp(r) => r.poll(),
                tabs::TabClient::Web => {}
            }
        }
        if !self.tabs.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(40));
        }
        egui::Panel::top("header").show(ui, |ui| self.chrome(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.busy {
                    ui.spinner();
                }
                ui.label(crate::i18n::tr(&self.status));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(crate::i18n::tr(&format!(
                        "{} onglet(s)  |  HostGates {}",
                        self.tabs.len(),
                        crate::branding::VERSION
                    )));
                });
            });
        });
        self.sidebar_panel(ui);
        egui::CentralPanel::default().show(ui, |ui| {
            self.workspace(ui);
        });
        if self.catalog_open {
            let mut open = true;
            egui::Window::new(crate::i18n::tr("Mes connexions"))
                .id(egui::Id::new("catalog_dialog"))
                .open(&mut open)
                .default_size([800.0, 500.0])
                .show(&ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.content(ui));
                });
            self.catalog_open = open;
        }
        self.advanced_dialogs(&ctx);
        if let Some(error) = self.error.clone() {
            egui::Window::new(crate::i18n::tr("Opération impossible"))
                .id(egui::Id::new("error_dialog"))
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.set_max_width(420.0);
                    ui.label(crate::i18n::tr(&error));
                    if ui.button(crate::i18n::tr("Fermer")).clicked() {
                        self.error = None;
                    }
                });
        }
        if let Some(mut editor) = self.editor.take() {
            let outcome = editor.show(&ctx, &self.snapshot.folders, self.busy, self.vault_ready);
            if !outcome.close {
                self.editor = Some(editor);
            }
            if let Some(action) = outcome.action {
                self.send(action);
            }
        }
        if let Some(mut editor) = self.folder_editor.take() {
            let outcome = editor.show(&ctx, &self.snapshot.folders, self.busy);
            if !outcome.close {
                self.folder_editor = Some(editor);
            }
            if let Some(action) = outcome.action {
                self.send(action);
            }
        }
        self.dialogs(&ctx);
        self.exchange_windows(&ctx);
        self.help_window(&ctx);
        self.about_window(&ctx);
        self.unlock_window(&ctx);
    }
}

pub fn matches_search(s: &Session, query: &str, snapshot: &Snapshot) -> bool {
    let query = query.trim().to_lowercase();
    let folder = crate::importers::exchange::folder_path(s.data.folder_id.as_deref(), snapshot);
    [
        &s.data.name,
        &s.data.hostname,
        &s.data.username,
        &s.data.description,
        &folder,
    ]
    .iter()
    .any(|v| v.to_lowercase().contains(&query))
        || s.data
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(&query))
}
