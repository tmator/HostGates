use super::terminal::TerminalView;
use super::*;
use crate::{
    connections::{rdp::RdpSession, terminal::Terminal},
    models::Protocol,
};
pub enum TabClient {
    EmbeddedSpice(Box<crate::connections::embedded_spice::EmbeddedSpice>),
    EmbeddedRdp(Box<crate::connections::embedded_rdp::EmbeddedRdp>),
    Web,
    Terminal(Box<Terminal>, TerminalView),
    Rdp(RdpSession),
}
impl TabClient {
    fn is_desktop(&self) -> bool {
        matches!(self, Self::EmbeddedRdp(r) if !r.closed)
            || matches!(self, Self::EmbeddedSpice(r) if !r.closed)
    }
    fn release_input(&mut self) {
        match self {
            Self::EmbeddedRdp(r) => r.release_input(),
            Self::EmbeddedSpice(r) => r.release_input(),
            _ => {}
        }
    }
    fn show_desktop(&mut self, ui: &mut egui::Ui, interactive: bool) {
        match self {
            Self::EmbeddedRdp(r) => r.show(ui, interactive),
            Self::EmbeddedSpice(r) => r.show(ui, interactive),
            _ => {}
        }
    }
}
pub struct SessionTab {
    pub files: Option<crate::connections::native::FileBrowser>,
    pub tunnel: Option<crate::connections::native::Tunnel>,
    pub id: String,
    pub title: String,
    pub session: Option<Session>,
    pub client: TabClient,
}
impl SessionTab {
    pub fn closed(&self) -> bool {
        match &self.client {
            TabClient::Web => false,
            TabClient::EmbeddedRdp(r) => r.closed,
            TabClient::EmbeddedSpice(r) => r.closed,
            TabClient::Terminal(t, _) => t.closed,
            TabClient::Rdp(r) => r.closed,
        }
    }
}
impl HostGates {
    pub(super) fn uses_external_rdp(&self, id: &str) -> bool {
        let global = self.advanced.preferences.default_external_rdp;
        self.advanced
            .preferences
            .sessions
            .get(id)
            .map_or(global, |o| o.uses_external_rdp(global))
    }

    fn workspace_interactive(&self) -> bool {
        self.editor.is_none()
            && self.proxmox.editor.is_none()
            && self.proxmox.login.is_none()
            && self.proxmox.delete.is_none()
            && self.proxmox.power.is_none()
            && self.folder_editor.is_none()
            && !self.catalog_open
            && !self.settings
            && !self.unlock_open
            && !self.about_open
            && !self.help_open
            && self.preview.is_none()
            && !self.export_open
            && self.error.is_none()
            && self.advanced.login.is_none()
            && self.advanced.rdp_launch.is_none()
            && self.advanced.rename.is_none()
            && !self.advanced.broadcast
            && !self.advanced.options_open
    }
    pub(super) fn enter_rdp_fullscreen(&mut self, id: String, ctx: &egui::Context) {
        if self.fullscreen_tab.is_none() {
            self.restore_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        }
        self.fullscreen_tab = Some(id);
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
    }
    pub(super) fn leave_rdp_fullscreen(&mut self, ctx: &egui::Context) {
        self.fullscreen_tab = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.restore_fullscreen));
    }
    pub(super) fn fullscreen_workspace(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(id) = self.fullscreen_tab.clone() else {
            return false;
        };
        let valid = self
            .tabs
            .iter()
            .any(|t| t.id == id && t.client.is_desktop());
        let shortcut = ui.ctx().input_mut(|i| {
            i.consume_key(
                egui::Modifiers::CTRL | egui::Modifiers::ALT,
                egui::Key::Enter,
            )
        });
        if !valid || !self.workspace_interactive() || shortcut {
            for tab in &mut self.tabs {
                tab.client.release_input();
            }
            self.leave_rdp_fullscreen(ui.ctx());
            return false;
        }
        for tab in &mut self.tabs {
            if tab.id != id {
                tab.client.release_input();
            }
        }
        let mut leave = false;
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
                ui.horizontal(|ui| {
                    ui.label(&tab.title);
                    leave = ui
                        .button(crate::i18n::tr("Quitter le plein écran"))
                        .on_hover_text(crate::i18n::tr("Ctrl+Alt+Entrée"))
                        .clicked();
                });
                tab.client.show_desktop(ui, !leave);
            }
        });
        if leave {
            self.leave_rdp_fullscreen(ui.ctx());
        }
        true
    }

    pub(super) fn connect(&mut self, s: &Session, ctx: &egui::Context) {
        self.begin_connect(s, ctx);
    }
    pub(super) fn connect_embedded_rdp(&mut self, original: &Session, target: &Session) {
        let password = match super::advanced::saved_password(original) {
            Ok(p) => p,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        let mut rdp = crate::connections::embedded_rdp::EmbeddedRdp::new(
            original.data.clone(),
            target.data.clone(),
            password,
        );
        if let Err(e) = rdp.set_resolution(
            self.advanced
                .preferences
                .sessions
                .get(&original.uuid)
                .and_then(|o| o.embedded_resolution),
        ) {
            self.error = Some(e.to_string());
            return;
        }
        rdp.set_routing(
            self.advanced
                .preferences
                .sessions
                .get(&original.uuid)
                .map(|o| o.rdp_routing.clone())
                .unwrap_or_default(),
        );
        rdp.set_gateway(
            self.advanced
                .preferences
                .sessions
                .get(&original.uuid)
                .and_then(|o| o.rdp_gateway.clone()),
        );
        let id = uuid::Uuid::new_v4().to_string();
        self.active_tab = Some(id.clone());
        self.tabs.push(SessionTab {
            id,
            title: original.data.name.clone(),
            session: Some(original.clone()),
            files: None,
            tunnel: None,
            client: TabClient::EmbeddedRdp(Box::new(rdp)),
        });
    }
    pub(super) fn connect_external(&mut self, s: &Session, ctx: &egui::Context) {
        let display = self
            .advanced
            .preferences
            .sessions
            .get(&s.uuid)
            .map(|o| o.rdp_display)
            .unwrap_or_default();
        if s.data.protocol == Protocol::RDP && !self.uses_external_rdp(&s.uuid) {
            self.connect_embedded_rdp(s, s);
            return;
        }
        if s.data.protocol == Protocol::RDP
            && let Some(gateway) = self
                .advanced
                .preferences
                .sessions
                .get(&s.uuid)
                .and_then(|o| o.rdp_gateway.clone())
        {
            let routing = self
                .advanced
                .preferences
                .sessions
                .get(&s.uuid)
                .map(|o| o.rdp_routing.clone())
                .unwrap_or_default();
            #[cfg(windows)]
            let client = RdpSession::launch_gateway_with_display(
                &s.data,
                &self.snapshot.programs.rdp,
                &gateway,
                &routing,
                display,
            )
            .map(TabClient::Rdp);
            #[cfg(not(windows))]
            let client = crate::connections::resolve_program(
                &self.snapshot.programs.rdp,
                crate::connections::ProgramKind::Rdp,
            )
            .and_then(|program| {
                gateway
                    .arguments_with_routing(&s.data, &routing)
                    .and_then(|mut args| {
                        display.apply_arguments(&mut args, false)?;
                        Terminal::launch(&program, &args, ctx.clone())
                    })
            })
            .map(|t| TabClient::Terminal(Box::new(t), TerminalView::default()));
            match client {
                Ok(client) => {
                    let id = uuid::Uuid::new_v4().to_string();
                    self.active_tab = Some(id.clone());
                    self.tabs.push(SessionTab {
                        id,
                        title: s.data.name.clone(),
                        session: Some(s.clone()),
                        client,
                        files: None,
                        tunnel: None,
                    });
                }
                Err(e) => self.error = Some(e.to_string()),
            }
            return;
        }

        if s.data.protocol == Protocol::WEB {
            match crate::connections::web::open(&s.data.hostname) {
                Ok(()) => self.status = "Lien transmis au navigateur par défaut.".into(),
                Err(e) => self.error = Some(e.to_string()),
            }
            return;
        }
        let manual = self
            .advanced
            .preferences
            .sessions
            .get(&s.uuid)
            .is_some_and(|o| o.manual_external_auth);
        let password = match if manual || s.data.protocol == Protocol::SSH {
            Ok(zeroize::Zeroizing::new(String::new()))
        } else {
            super::advanced::saved_password(s)
        } {
            Ok(value) => value,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        let client = match s.data.protocol {
            Protocol::WEB => unreachable!(),
            Protocol::VNC => {
                crate::connections::vnc::launch(&s.data, &self.snapshot.programs.vnc, &password)
                    .map(TabClient::Rdp)
            }
            Protocol::SSH => {
                let program = match crate::connections::resolve_program(
                    &self.snapshot.programs.ssh,
                    crate::connections::ProgramKind::Ssh,
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        self.error = Some(e.to_string());
                        return;
                    }
                };
                crate::connections::ssh_arguments_with_legacy(
                    &s.data,
                    self.advanced
                        .preferences
                        .sessions
                        .get(&s.uuid)
                        .is_some_and(|o| o.legacy_ssh_rsa),
                )
                .and_then(|args| Terminal::launch(&program, &args, ctx.clone()))
                .map(|t| TabClient::Terminal(Box::new(t), TerminalView::default()))
            }
            Protocol::RDP => {
                #[cfg(windows)]
                {
                    RdpSession::launch_with_display(
                        &s.data,
                        &self.snapshot.programs.rdp,
                        &password,
                        display,
                    )
                    .map(TabClient::Rdp)
                }
                #[cfg(not(windows))]
                {
                    if !password.is_empty() {
                        RdpSession::launch_with_display(
                            &s.data,
                            &self.snapshot.programs.rdp,
                            &password,
                            display,
                        )
                        .map(TabClient::Rdp)
                    } else {
                        let program = match crate::connections::resolve_program(
                            &self.snapshot.programs.rdp,
                            crate::connections::ProgramKind::Rdp,
                        ) {
                            Ok(p) => p,
                            Err(e) => {
                                self.error = Some(e.to_string());
                                return;
                            }
                        };
                        crate::connections::rdp::arguments(&s.data, false)
                            .and_then(|mut args| {
                                display.apply_arguments(&mut args, false)?;
                                Terminal::launch(&program, &args, ctx.clone())
                            })
                            .map(|t| TabClient::Terminal(Box::new(t), TerminalView::default()))
                    }
                }
            }
        };
        match client {
            Ok(client) => {
                let id = uuid::Uuid::new_v4().to_string();
                self.active_tab = Some(id.clone());
                self.tabs.push(SessionTab {
                    files: None,
                    tunnel: None,
                    id,
                    title: s.data.name.clone(),
                    session: Some(s.clone()),
                    client,
                });
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    pub(super) fn local_terminal(&mut self, ctx: &egui::Context) {
        let program = match crate::connections::resolve_program(
            &self.snapshot.programs.terminal,
            crate::connections::ProgramKind::Terminal,
        ) {
            Ok(p) => p,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        match Terminal::launch(&program, &[], ctx.clone()) {
            Ok(t) => {
                let id = uuid::Uuid::new_v4().to_string();
                self.active_tab = Some(id.clone());
                self.tabs.push(SessionTab {
                    files: None,
                    tunnel: None,
                    id,
                    title: "Terminal local".into(),
                    session: None,
                    client: TabClient::Terminal(Box::new(t), TerminalView::default()),
                });
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    pub(super) fn workspace(&mut self, ui: &mut egui::Ui) {
        use crate::i18n::tr;
        if self.tabs.is_empty() {
            self.active_tab = None;
            self.advanced.visible.clear();
            self.advanced.panes = 1;
            return;
        }
        if !self
            .tabs
            .iter()
            .any(|t| Some(&t.id) == self.active_tab.as_ref())
        {
            self.active_tab = self.tabs.first().map(|t| t.id.clone());
        }
        let mut close = None;
        let mut move_tab = None;
        let choices = self
            .tabs
            .iter()
            .map(|t| (t.id.clone(), t.title.clone()))
            .collect::<Vec<_>>();
        egui::ScrollArea::horizontal()
            .id_salt("tabbar")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, tab) in self.tabs.iter().enumerate() {
                        ui.push_id(&tab.id, |ui| {
                            ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
                            ui.spacing_mut().item_spacing.x = 3.0;
                            let selected = self.active_tab.as_deref() == Some(&tab.id);
                            let fill = if selected {
                                ui.visuals().selection.bg_fill
                            } else {
                                ui.visuals().widgets.inactive.bg_fill
                            };
                            let r = ui.add_sized(
                                [150.0, 24.0],
                                egui::Button::new(
                                    egui::RichText::new(format!(
                                        "{} {}",
                                        if tab.closed() { "○" } else { "●" },
                                        tab.title
                                    ))
                                    .size(12.0),
                                )
                                .fill(fill)
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if selected {
                                        ui.visuals().selection.stroke.color
                                    } else {
                                        ui.visuals().widgets.inactive.bg_stroke.color
                                    },
                                ))
                                .corner_radius(4)
                                .truncate(),
                            );
                            r.clone().on_hover_text(&tab.title);
                            if r.clicked() {
                                self.active_tab = Some(tab.id.clone());
                            }
                            r.context_menu(|ui| {
                                if ui.button(tr("Renommer l'onglet")).clicked() {
                                    self.advanced.rename =
                                        Some((tab.id.clone(), tab.title.clone()));
                                    ui.close();
                                }
                                if ui.button(tr("Déplacer à gauche")).clicked() && i > 0 {
                                    move_tab = Some((i, i - 1));
                                    ui.close();
                                }
                                if ui.button(tr("Déplacer à droite")).clicked()
                                    && i + 1 < self.tabs.len()
                                {
                                    move_tab = Some((i, i + 1));
                                    ui.close();
                                }
                            });
                            ui.menu_button(crate::i18n::tr("▾"), |ui| {
                                if ui.button(tr("Renommer l'onglet")).clicked() {
                                    self.advanced.rename =
                                        Some((tab.id.clone(), tab.title.clone()));
                                    ui.close();
                                }
                                ui.separator();
                                for (id, title) in &choices {
                                    if ui
                                        .selectable_label(
                                            self.active_tab.as_ref() == Some(id),
                                            title,
                                        )
                                        .clicked()
                                    {
                                        self.active_tab = Some(id.clone());
                                        ui.close();
                                    }
                                }
                            });
                            if ui
                                .small_button(crate::i18n::tr("×"))
                                .on_hover_text(tr("Fermer la session"))
                                .clicked()
                            {
                                close = Some(tab.id.clone());
                            }
                            ui.separator();
                        });
                    }
                });
            });
        if let Some((a, b)) = move_tab {
            self.tabs.swap(a, b);
        }
        if let Some(id) = close {
            self.tabs.retain(|t| t.id != id);
            if self.active_tab.as_ref() == Some(&id) {
                self.active_tab = self.tabs.last().map(|t| t.id.clone());
            }
        }
        ui.separator();
        if self.active_tab.is_none() {
            self.advanced.visible.clear();
            self.advanced.panes = 1;
            return;
        }
        self.advanced.panes = self.advanced.panes.clamp(1, self.tabs.len().clamp(1, 6));
        ui.horizontal(|ui| {
            ui.label(tr("Volets visibles"));
            for n in 1..=self.tabs.len().min(6) {
                ui.selectable_value(&mut self.advanced.panes, n, n.to_string());
            }
            if ui.button(tr("Exécution multiple")).clicked() {
                self.advanced.broadcast = true;
            }
        });
        let interactive = self.workspace_interactive();
        self.advanced
            .visible
            .retain(|id| choices.iter().any(|(tid, _)| tid == id));
        if let Some(active) = &self.active_tab {
            if self.advanced.panes == 1 {
                self.advanced.visible = vec![active.clone()];
            } else if !self.advanced.visible.contains(active) {
                self.advanced.visible.insert(0, active.clone());
            }
        }
        for (id, _) in &choices {
            if self.advanced.visible.len() >= self.advanced.panes {
                break;
            }
            if !self.advanced.visible.contains(id) {
                self.advanced.visible.push(id.clone());
            }
        }
        self.advanced.visible.truncate(self.advanced.panes);
        let ids = self.advanced.visible.clone();
        for tab in &mut self.tabs {
            if !interactive || !ids.contains(&tab.id) {
                tab.client.release_input();
            }
        }
        let count = ids.len();
        if count == 0 {
            return;
        }
        let cols = if count == 1 { 1 } else { 2 };
        let rows = count.div_ceil(cols);
        let rect = ui.available_rect_before_wrap();
        let width = (rect.width() - (cols - 1) as f32 * 6.0) / cols as f32;
        let height = (rect.height() - (rows - 1) as f32 * 6.0) / rows as f32;
        let mut fullscreen = None;
        let mut reconnect = None;
        let mut switch = None;
        for (index, id) in ids.iter().enumerate() {
            let Some(tab) = self.tabs.iter_mut().find(|t| &t.id == id) else {
                continue;
            };
            let pane = egui::Rect::from_min_size(
                rect.min
                    + egui::vec2(
                        (index % cols) as f32 * (width + 6.0),
                        (index / cols) as f32 * (height + 6.0),
                    ),
                egui::vec2(width, height),
            );
            ui.scope_builder(egui::UiBuilder::new().id_salt(id).max_rect(pane),|ui|{
                ui.set_clip_rect(pane.intersect(ui.clip_rect()));
                ui.horizontal(|ui|{
                    ui.menu_button(format!("{} ▾",tab.title),|ui|{
                        for(other,title)in &choices {if ui.selectable_label(other==id,title).clicked(){switch=Some((index,other.clone()));ui.close();}}
                    });
                    if let Some(s)=&tab.session && ui.small_button(crate::i18n::tr("↻")).on_hover_text(tr("Reconnecter / nouvelle connexion")).clicked(){reconnect=Some(s.clone());}
                    if let Some(files)=&mut tab.files && ui.selectable_label(files.open,crate::i18n::tr("SFTP")).clicked(){files.open= !files.open;if files.open&&files.entries.is_empty(){files.send(crate::connections::native::FileAction::List(files.path.clone()));}}
                    if tab.client.is_desktop()
                        && ui.small_button(tr("Plein écran")).clicked() { fullscreen = Some(id.clone()); }
                    if ui.small_button(tr("Arrêter")).clicked(){match &mut tab.client{TabClient::Terminal(t,_)=>t.stop(),TabClient::Rdp(r)=>r.stop(),TabClient::EmbeddedRdp(r)=>r.stop(),TabClient::EmbeddedSpice(r)=>r.stop(),TabClient::Web=>{}}tab.tunnel=None;tab.files=None;}
                });
                if let Some(files)=&mut tab.files && files.open {
                    egui::Panel::right(egui::Id::new(("files",id))).default_size((width*0.4).clamp(180.0,300.0)).size_range(150.0..=400.0).resizable(true).show(ui,|ui|super::advanced::files_ui(ui,files));
                }
                match &mut tab.client {
                    TabClient::Terminal(t,v)=>{if count>1{v.focus=false;}v.show(ui,t,interactive);},
                    TabClient::Rdp(r)=>{ui.heading(tr("Bureau distant RDP / VNC / SPICE"));ui.label(crate::i18n::tr(&r.status));ui.label(tr("Le client distant utilise sa propre fenêtre. Utilisez Alt+Tab pour la retrouver."));},
                    TabClient::EmbeddedRdp(r)=>r.show(ui,interactive),
                    TabClient::EmbeddedSpice(r)=>r.show(ui,interactive),
                    TabClient::Web=>{ui.heading(tr("Tunnel Web actif"));ui.label(tr("Gardez cet onglet ouvert pendant la navigation. Le fermer arrête le tunnel."));}
                }
            });
        }
        if let Some((index, id)) = switch {
            if let Some(other) = self.advanced.visible.iter().position(|v| v == &id) {
                self.advanced.visible.swap(index, other);
            } else {
                self.advanced.visible[index] = id.clone();
            }
            self.active_tab = Some(id);
        }
        if let Some(id) = fullscreen {
            self.enter_rdp_fullscreen(id, ui.ctx());
        }
        if let Some(s) = reconnect {
            self.connect(&s, ui.ctx());
        }
    }
}
