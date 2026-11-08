//! RDP desktop hosted by egui. Network/TLS work never runs on the UI thread.
use crate::{credentials::Secret, models::SessionInput};
use eframe::egui::{self, ColorImage, Event, Key, TextureHandle};
use ironrdp_client::{
    config::{ConfigBuilder, Destination},
    rdp::{RdpClient, RdpInputEvent, RdpOutputEvent},
};
use ironrdp_pdu::input::{
    fast_path::{FastPathInputEvent as Input, KeyboardFlags},
    mouse::{MousePdu, PointerFlags},
};
use sha2::{Digest, Sha256};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;
use zeroize::Zeroize;

struct CertificatePrompt {
    endpoint: String,
    fingerprint: String,
    reason: String,
    reply: mpsc::SyncSender<bool>,
}
pub struct EmbeddedRdp {
    requested_resolution: (u16, u16),
    gateway: Option<super::rdp::Gateway>,
    gateway_password: Secret,
    routing: super::rdp::Routing,
    same_credentials: bool,
    identity: SessionInput,
    target: SessionInput,
    password: Secret,
    input: Option<tokio::sync::mpsc::Sender<RdpInputEvent>>,
    output: Option<tokio::sync::mpsc::Receiver<RdpOutputEvent>>,
    cert_rx: mpsc::Receiver<CertificatePrompt>,
    cert_tx: mpsc::SyncSender<CertificatePrompt>,
    certificate: Option<CertificatePrompt>,
    stop: Arc<AtomicBool>,
    texture: Option<TextureHandle>,
    dimensions: [usize; 2],
    #[cfg(test)]
    frame_pixels: Vec<egui::Color32>,
    pending: std::collections::VecDeque<Input>,
    pointer_texture: Option<egui::TextureHandle>,
    pointer_hotspot: egui::Vec2,
    pointer_hidden: bool,
    pressed: std::collections::BTreeSet<(u8, bool)>,
    buttons: Vec<PointerFlags>,
    last_position: (u16, u16),
    modifiers: egui::Modifiers,
    focused: bool,
    pub closed: bool,
    pub status: String,
}
impl Drop for EmbeddedRdp {
    fn drop(&mut self) {
        self.stop();
    }
}
impl EmbeddedRdp {
    pub fn new(identity: SessionInput, target: SessionInput, password: Secret) -> Self {
        let (cert_tx, cert_rx) = mpsc::sync_channel(1);
        Self {
            requested_resolution: (1280, 720),
            gateway: None,
            gateway_password: Secret::new(String::new()),
            routing: Default::default(),
            same_credentials: true,
            identity,
            target,
            password,
            input: None,
            output: None,
            cert_rx,
            cert_tx,
            certificate: None,
            stop: Arc::new(AtomicBool::new(false)),
            texture: None,
            dimensions: [1280, 720],
            #[cfg(test)]
            frame_pixels: vec![],
            pending: Default::default(),
            pointer_texture: None,
            pointer_hotspot: egui::Vec2::ZERO,
            pointer_hidden: false,
            pressed: Default::default(),
            buttons: vec![],
            last_position: (0, 0),
            modifiers: Default::default(),
            focused: false,
            closed: false,
            status: String::new(),
        }
    }
    pub fn set_resolution(&mut self, resolution: Option<(u16, u16)>) -> crate::error::Result<()> {
        let (width, height) = resolution.unwrap_or((1280, 720));
        super::rdp::DisplaySize::Fixed { width, height }.validate()?;
        self.requested_resolution = (width, height);
        self.dimensions = [width as usize, height as usize];
        Ok(())
    }
    pub fn set_routing(&mut self, routing: super::rdp::Routing) {
        self.routing = routing;
    }
    pub fn set_gateway(&mut self, gateway: Option<super::rdp::Gateway>) {
        self.gateway = gateway;
    }
    pub fn stop(&mut self) {
        self.release_input();
        self.stop.store(true, Ordering::Release);
        if let Some(tx) = self.input.take() {
            let _ = tx.try_send(RdpInputEvent::Close);
        }
        self.certificate = None;
        self.password.zeroize();
        self.gateway_password.zeroize();
        self.closed = true;
    }
    fn send(&mut self, input: Input) {
        // Only replace consecutive absolute moves. Never cross a button/key event.
        if matches!(input, Input::MouseEvent(m) if m.flags == PointerFlags::MOVE)
            && self
                .pending
                .back()
                .is_some_and(|p| matches!(p, Input::MouseEvent(m) if m.flags == PointerFlags::MOVE))
        {
            *self.pending.back_mut().expect("pending move") = input;
            return;
        }
        if self.pending.len() >= 8192 {
            self.stop.store(true, Ordering::Release);
            self.input = None;
            self.pending.clear();
            self.closed = true;
            self.status = "RDP : trop d'entrées en attente, connexion arrêtée.".into();
        } else {
            self.pending.push_back(input);
        }
    }
    fn sync_modifiers(&mut self, modifiers: egui::Modifiers) {
        for (old, new, code, ext) in [
            (self.modifiers.ctrl, modifiers.ctrl, 0x1d, false),
            (self.modifiers.alt, modifiers.alt, 0x38, false),
            (self.modifiers.shift, modifiers.shift, 0x2a, false),
            (self.modifiers.mac_cmd, modifiers.mac_cmd, 0x5b, true),
        ] {
            if old != new {
                self.key(code, ext, new);
            }
        }
        self.modifiers = modifiers;
    }
    fn key(&mut self, code: u8, extended: bool, down: bool) {
        let mut flags = KeyboardFlags::empty();
        if extended {
            flags |= KeyboardFlags::EXTENDED;
        }
        if !down {
            flags |= KeyboardFlags::RELEASE;
            self.pressed.remove(&(code, extended));
        } else {
            self.pressed.insert((code, extended));
        }
        self.send(Input::KeyboardEvent(flags, code));
    }
    fn mouse(&mut self, flags: PointerFlags, wheel: i16) {
        self.send(Input::MouseEvent(MousePdu {
            flags,
            number_of_wheel_rotation_units: wheel,
            x_position: self.last_position.0,
            y_position: self.last_position.1,
        }));
    }
    pub(crate) fn release_input(&mut self) {
        for (code, ext) in std::mem::take(&mut self.pressed) {
            self.key(code, ext, false);
        }
        for flag in std::mem::take(&mut self.buttons) {
            self.mouse(flag, 0);
        }
        self.modifiers = Default::default();
        self.focused = false;
        self.flush_input();
    }
    fn flush_input(&mut self) {
        // One fast-path packet per batch, preserving click/key ordering.
        for _ in 0..8 {
            if self.pending.is_empty() {
                break;
            }
            let Some(tx) = self.input.as_ref() else {
                break;
            };
            let events = self.pending.iter().take(32).copied().collect();
            let count = self.pending.len().min(32);
            if tx.try_send(RdpInputEvent::FastPath(events)).is_err() {
                break;
            }
            self.pending.drain(..count);
        }
    }
    pub fn poll(&mut self, ctx: &egui::Context) {
        self.flush_input();
        if self.input.is_some()
            && self
                .output
                .as_ref()
                .is_some_and(|rx| rx.is_closed() && rx.is_empty())
        {
            self.status = "Connexion RDP terminée.".into();
            self.stop();
        }

        if self.certificate.is_none() {
            self.certificate = self.cert_rx.try_recv().ok();
        }
        while let Some(event) = self.output.as_mut().and_then(|rx| rx.try_recv().ok()) {
            match event {
                RdpOutputEvent::Image {
                    buffer,
                    width,
                    height,
                } => {
                    let size = [usize::from(width.get()), usize::from(height.get())];
                    if size[0] * size[1] > 4096 * 4096 || buffer.len() != size[0] * size[1] {
                        self.status = "RDP: image trop volumineuse ou invalide.".into();
                        self.stop();
                        break;
                    }
                    let pixels = buffer
                        .into_iter()
                        .map(|p| egui::Color32::from_rgb((p >> 16) as u8, (p >> 8) as u8, p as u8))
                        .collect();
                    let image = ColorImage::new(size, pixels);
                    #[cfg(test)]
                    {
                        self.frame_pixels = image.pixels.clone();
                    }
                    if let Some(texture) = &mut self.texture {
                        texture.set(image, egui::TextureOptions::LINEAR);
                    } else {
                        self.texture = Some(ctx.load_texture(
                            "rdp-desktop",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    self.dimensions = size;
                    self.status.clear();
                }
                RdpOutputEvent::ConnectionFailure(e) => {
                    // Display nested transport/authentication errors, not only
                    // the connector's generic "GW connect: custom error" wrapper.
                    self.status = format!("RDP : {}", e.report());
                    self.stop();
                }
                RdpOutputEvent::Terminated(_) => {
                    self.status = "Connexion RDP terminée.".into();
                    self.stop();
                }
                RdpOutputEvent::PointerDefault => {
                    self.pointer_texture = None;
                    self.pointer_hidden = false;
                }
                RdpOutputEvent::PointerHidden => {
                    self.pointer_hidden = true;
                }
                RdpOutputEvent::PointerBitmap(pointer) => {
                    let size = [usize::from(pointer.width), usize::from(pointer.height)];
                    if size[0] > 0
                        && size[1] > 0
                        && size[0] <= 384
                        && size[1] <= 384
                        && pointer.bitmap_data.len() == size[0] * size[1] * 4
                    {
                        self.pointer_texture = Some(ctx.load_texture(
                            "rdp-pointer",
                            ColorImage::from_rgba_unmultiplied(size, &pointer.bitmap_data),
                            egui::TextureOptions::LINEAR,
                        ));
                        self.pointer_hotspot =
                            egui::vec2(f32::from(pointer.hotspot_x), f32::from(pointer.hotspot_y));
                        self.pointer_hidden = false;
                    } else {
                        self.pointer_texture = None;
                        self.pointer_hidden = size == [0, 0];
                    }
                }
                // Local mouse position drives the cursor, without a network round trip.
                RdpOutputEvent::PointerPosition { .. } => {}
            }
        }
        if self.input.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        if self.focused
            && !ctx.memory(|m| {
                m.has_focus(egui::Id::new((
                    "rdp-input",
                    self.identity.name.clone(),
                    self as *const _ as usize,
                )))
            })
        {
            self.release_input();
        }
    }
    fn build_config(&self) -> Result<ironrdp_client::config::Config, Box<dyn std::error::Error>> {
        let host = self.target.hostname.trim_matches(['[', ']']);
        let address = if host.contains(':') {
            format!("[{host}]:{}", self.target.port)
        } else {
            format!("{host}:{}", self.target.port)
        };

        let mut builder = ConfigBuilder::new()
            .with_destination(Destination::new(address)?)
            .with_username(self.identity.username.clone())
            .with_domain(self.identity.domain.clone())
            .with_password(self.password.to_string())
            .with_desktop_width(self.requested_resolution.0)
            .with_desktop_height(self.requested_resolution.1)
            .with_client_name("HostGates")
            .with_client_build(26000)
            .with_client_dir("HostGates")
            .with_platform(if cfg!(windows) {
                ironrdp_pdu::rdp::capability_sets::MajorPlatformType::WINDOWS
            } else {
                ironrdp_pdu::rdp::capability_sets::MajorPlatformType::UNIX
            })
            .with_autologon(true)
            .with_credssp(true)
            .with_tls(true)
            .with_server_pointer(true)
            .with_pointer_software_rendering(false);
        if let Some(gateway) = &self.gateway {
            gateway.validate()?;
            if gateway.credential_source == 1 {
                return Err("RD Gateway : carte à puce non prise en charge dans les onglets. Sélectionnez le client externe.".into());
            }
            let username = if self.same_credentials {
                gateway_identity(&self.identity)
            } else {
                gateway.username.clone()
            };
            if username.trim().is_empty() {
                return Err("Utilisateur RD Gateway requis.".into());
            }
            builder = builder
                .with_transport(ironrdp_client::config::TransportKind::Gateway {
                    endpoint: gateway.host.clone(),
                })
                .with_gateway_username(username)
                .with_gateway_password(if self.same_credentials {
                    self.password.to_string()
                } else {
                    self.gateway_password.to_string()
                });
        }
        self.routing.validate()?;
        let mut config = builder.build()?;
        config.set_load_balance_info(&self.routing.load_balance_info)?;
        config.use_redirection_server_name = self.routing.use_redirection_server_name;
        // SSH forwarding has a fixed target; do not silently escape its tunnel.
        config.enable_server_redirection(
            self.identity.hostname == self.target.hostname
                && self.identity.port == self.target.port,
        );
        config.gateway_rpc = self.gateway.as_ref().is_some_and(|g| g.rpc_transport);
        config.tls_server_name = Some(self.identity.hostname.trim_matches(['[', ']']).to_owned());
        Ok(config)
    }
    fn start(&mut self, ctx: &egui::Context) {
        if self.identity.username.trim().is_empty() {
            self.status = "Utilisateur RDP requis.".into();
            return;
        }
        let config = self.build_config();
        let mut config = match config {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("RDP : {e}");
                return;
            }
        };
        self.certificate = None;
        while self.cert_rx.try_recv().is_ok() {}
        self.pending.clear();
        self.stop = Arc::new(AtomicBool::new(false));
        self.closed = false;
        self.texture = None;
        self.pointer_texture = None;
        self.pointer_hidden = false;
        let stop = self.stop.clone();
        let tx = self.cert_tx.clone();
        let repaint = ctx.clone();
        config.tls_callback = Some(Arc::new(move |der, endpoint, reason| {
            let fingerprint = Sha256::digest(der)
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(":");
            let (reply, answer) = mpsc::sync_channel(1);
            if tx
                .try_send(CertificatePrompt {
                    endpoint: endpoint.to_owned(),
                    fingerprint,
                    reason: reason.to_owned(),
                    reply,
                })
                .is_err()
            {
                return false;
            }
            repaint.request_repaint();
            for _ in 0..1200 {
                if stop.load(Ordering::Acquire) {
                    return false;
                }
                match answer.recv_timeout(Duration::from_millis(100)) {
                    Ok(yes) => return yes,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return false,
                    Err(_) => {}
                }
            }
            false
        }));
        let (tx, rx) = tokio::sync::mpsc::channel(2);
        let client = RdpClient::new(config, tx);
        self.input = Some(client.input_sender());
        self.output = Some(rx);
        self.password.zeroize();
        self.gateway_password.zeroize();
        self.status = "Connexion RDP…".into();
        let stop = self.stop.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                runtime.block_on(async {
                    tokio::select! { _=client.run()=>{}, _=async {while !stop.load(Ordering::Acquire){tokio::time::sleep(Duration::from_millis(100)).await;}}=>{} }
                });
            }
            ctx.request_repaint();
        });
    }
    pub fn show(&mut self, ui: &mut egui::Ui, interactive: bool) {
        use crate::i18n::tr;
        if let Some(prompt) = &self.certificate {
            ui.heading(tr("Certificat RDP non validé"));
            ui.label(&prompt.endpoint);
            ui.label(&prompt.reason);
            ui.label(tr(
                "Vérifiez cette empreinte SHA-256 auprès de l'administrateur avant d'accepter.",
            ));
            ui.monospace(&prompt.fingerprint);
            let accept = ui
                .button(tr("Accepter pour cette connexion uniquement"))
                .clicked();
            let reject =
                ui.button(tr("Refuser")).clicked() || ui.input(|i| i.key_pressed(Key::Escape));
            if (accept || reject)
                && let Some(prompt) = self.certificate.take()
            {
                let _ = prompt.reply.send(accept && !reject);
            }
            return;
        }
        if self.input.is_none() {
            ui.heading(tr("Connexion RDP intégrée"));
            ui.label(tr("Utilisateur"));
            ui.text_edit_singleline(&mut self.identity.username);
            ui.label(tr("Domaine"));
            ui.text_edit_singleline(&mut self.identity.domain);
            ui.label(tr("Mot de passe"));
            let edit = ui.add(egui::TextEdit::singleline(&mut *self.password).password(true));
            let mut gateway_enter = false;
            if let Some(gateway) = &mut self.gateway {
                ui.separator();
                ui.label(format!("RD Gateway : {}", gateway.host));
                ui.checkbox(
                    &mut self.same_credentials,
                    tr("Utiliser les identifiants du poste pour la passerelle"),
                );
                if !self.same_credentials {
                    ui.label(tr("Utilisateur de la passerelle"));
                    ui.text_edit_singleline(&mut gateway.username);
                    ui.label(tr("Mot de passe de la passerelle"));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut *self.gateway_password).password(true),
                    );
                    gateway_enter = (response.has_focus() || response.lost_focus())
                        && ui.input(|i| i.key_pressed(Key::Enter));
                }
                if gateway.usage == 2 {
                    ui.small(tr("Dans les onglets, la passerelle est toujours utilisée, sans tentative directe."));
                }
            }
            if gateway_enter
                || ui.button(tr("Connexion")).clicked()
                || ((edit.has_focus() || edit.lost_focus())
                    && ui.input(|i| i.key_pressed(Key::Enter)))
            {
                self.start(ui.ctx());
            }
            ui.add(egui::Label::new(&self.status).selectable(true).wrap());
            return;
        }
        ui.horizontal(|ui| {
            if ui.button("Ctrl+Alt+Suppr").clicked() {
                self.key(0x1d, false, true);
                self.key(0x38, false, true);
                self.key(0x53, true, true);
                self.key(0x53, true, false);
                self.key(0x38, false, false);
                self.key(0x1d, false, false);
            }
            ui.label(&self.status);
        });
        let Some(texture) = &self.texture else {
            ui.spinner();
            return;
        };
        let scale = (ui.available_width() / self.dimensions[0] as f32)
            .min(ui.available_height() / self.dimensions[1] as f32)
            .max(0.01);
        let size = egui::vec2(
            self.dimensions[0] as f32 * scale,
            self.dimensions[1] as f32 * scale,
        );
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter().image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
            egui::Color32::WHITE,
        );
        let id = egui::Id::new((
            "rdp-input",
            self.identity.name.clone(),
            self as *const _ as usize,
        ));
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());
        if response.clicked()
            || response.drag_started()
            || (response.hovered() && ui.input(|i| i.pointer.any_pressed()))
        {
            response.request_focus();
        }
        if interactive && response.hovered() {
            if self.pointer_hidden {
                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
            } else if let (Some(pointer), Some(pos)) =
                (&self.pointer_texture, ui.input(|i| i.pointer.hover_pos()))
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                let pointer_rect = egui::Rect::from_min_size(
                    pos - self.pointer_hotspot * scale,
                    pointer.size_vec2() * scale,
                );
                ui.painter().with_clip_rect(rect).image(
                    pointer.id(),
                    pointer_rect,
                    egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
                    egui::Color32::WHITE,
                );
            } else {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
            }
        }
        if !interactive || !response.has_focus() {
            self.release_input();
            return;
        }
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            )
        });
        self.focused = true;
        self.sync_modifiers(ui.input(|i| i.modifiers));
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                Event::PointerMoved(pos) if rect.contains(pos) || !self.buttons.is_empty() => {
                    self.last_position = map_position(pos, rect, self.dimensions);
                    self.mouse(PointerFlags::MOVE, 0);
                }
                Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    ..
                } if rect.contains(pos) || !pressed => {
                    self.last_position = map_position(pos, rect, self.dimensions);
                    let flag = match button {
                        egui::PointerButton::Primary => PointerFlags::LEFT_BUTTON,
                        egui::PointerButton::Secondary => PointerFlags::RIGHT_BUTTON,
                        egui::PointerButton::Middle => PointerFlags::MIDDLE_BUTTON_OR_WHEEL,
                        _ => continue,
                    };
                    if pressed {
                        if !self.buttons.contains(&flag) {
                            self.buttons.push(flag);
                        }
                    } else {
                        self.buttons.retain(|b| *b != flag);
                    }
                    self.mouse(
                        if pressed {
                            flag | PointerFlags::DOWN
                        } else {
                            flag
                        },
                        0,
                    );
                }
                Event::MouseWheel { delta, .. } if response.hovered() => self.mouse(
                    PointerFlags::VERTICAL_WHEEL,
                    delta.y.clamp(-255., 255.) as i16,
                ),
                Event::Text(text) | Event::Paste(text) => {
                    for c in text.encode_utf16().take(4096) {
                        self.send(Input::UnicodeKeyboardEvent(KeyboardFlags::empty(), c));
                        self.send(Input::UnicodeKeyboardEvent(KeyboardFlags::RELEASE, c));
                    }
                }
                Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } => {
                    for (old, new, code, ext) in [
                        (self.modifiers.ctrl, modifiers.ctrl, 0x1d, false),
                        (self.modifiers.alt, modifiers.alt, 0x38, false),
                        (self.modifiers.shift, modifiers.shift, 0x2a, false),
                        (self.modifiers.mac_cmd, modifiers.mac_cmd, 0x5b, true),
                    ] {
                        if old != new {
                            self.key(code, ext, new);
                        }
                    }
                    self.modifiers = modifiers;
                    if let Some((code, ext)) =
                        scan_code(key, modifiers.ctrl || modifiers.alt || modifiers.command)
                    {
                        self.key(code, ext, pressed);
                    }
                }
                Event::Copy | Event::Cut => {
                    self.key(0x1d, false, true);
                    let code = if matches!(event, Event::Copy) {
                        0x2e
                    } else {
                        0x2d
                    };
                    self.key(code, false, true);
                    self.key(code, false, false);
                    self.key(0x1d, false, false);
                }
                Event::WindowFocused(false) => self.release_input(),
                _ => {}
            }
        }
        self.flush_input();
    }
}
fn gateway_identity(identity: &SessionInput) -> String {
    if identity.domain.is_empty() || identity.username.contains(['\\', '@']) {
        identity.username.clone()
    } else {
        format!("{}\\{}", identity.domain, identity.username)
    }
}
fn map_position(pos: egui::Pos2, rect: egui::Rect, size: [usize; 2]) -> (u16, u16) {
    (
        ((pos.x - rect.left()) / rect.width() * size[0] as f32)
            .clamp(0., size[0].saturating_sub(1) as f32) as u16,
        ((pos.y - rect.top()) / rect.height() * size[1] as f32)
            .clamp(0., size[1].saturating_sub(1) as f32) as u16,
    )
}
fn scan_code(key: Key, shortcut: bool) -> Option<(u8, bool)> {
    Some(match key {
        Key::Enter => (0x1c, false),
        Key::Escape => (1, false),
        Key::Tab => (0xf, false),
        Key::Backspace => (0xe, false),
        Key::ArrowUp => (0x48, true),
        Key::ArrowDown => (0x50, true),
        Key::ArrowLeft => (0x4b, true),
        Key::ArrowRight => (0x4d, true),
        Key::Home => (0x47, true),
        Key::End => (0x4f, true),
        Key::PageUp => (0x49, true),
        Key::PageDown => (0x51, true),
        Key::Insert => (0x52, true),
        Key::Delete => (0x53, true),
        Key::F1 => (0x3b, false),
        Key::F2 => (0x3c, false),
        Key::F3 => (0x3d, false),
        Key::F4 => (0x3e, false),
        Key::F5 => (0x3f, false),
        Key::F6 => (0x40, false),
        Key::F7 => (0x41, false),
        Key::F8 => (0x42, false),
        Key::F9 => (0x43, false),
        Key::F10 => (0x44, false),
        Key::F11 => (0x57, false),
        Key::F12 => (0x58, false),
        _ if shortcut => (
            match key {
                Key::A => 0x1e,
                Key::B => 0x30,
                Key::C => 0x2e,
                Key::D => 0x20,
                Key::E => 0x12,
                Key::F => 0x21,
                Key::G => 0x22,
                Key::H => 0x23,
                Key::I => 0x17,
                Key::J => 0x24,
                Key::K => 0x25,
                Key::L => 0x26,
                Key::M => 0x32,
                Key::N => 0x31,
                Key::O => 0x18,
                Key::P => 0x19,
                Key::Q => 0x10,
                Key::R => 0x13,
                Key::S => 0x1f,
                Key::T => 0x14,
                Key::U => 0x16,
                Key::V => 0x2f,
                Key::W => 0x11,
                Key::X => 0x2d,
                Key::Y => 0x15,
                Key::Z => 0x2c,
                Key::Space => 0x39,
                _ => return None,
            },
            false,
        ),
        _ => return None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mouse_moves_are_coalesced_without_reordering_clicks_and_keys() {
        let preview =
            crate::importers::rdp::parse(b"full address:s:host\nusername:s:alice", "fixture")
                .expect("valid test fixture");
        let identity = preview.rows[0].data.clone().expect("valid test fixture");
        let mut tab = EmbeddedRdp::new(identity.clone(), identity, Secret::new(String::new()));
        for x in 0..1000 {
            tab.last_position = (x, 20);
            tab.mouse(PointerFlags::MOVE, 0);
        }
        tab.mouse(PointerFlags::LEFT_BUTTON | PointerFlags::DOWN, 0);
        for x in 1000..2000 {
            tab.last_position = (x, 30);
            tab.mouse(PointerFlags::MOVE, 0);
        }
        tab.mouse(PointerFlags::LEFT_BUTTON, 0);
        tab.key(0x1c, false, true);
        assert_eq!(tab.pending.len(), 5);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        tab.input = Some(tx);
        tab.poll(&egui::Context::default());
        assert!(tab.pending.is_empty());
        let RdpInputEvent::FastPath(events) = rx.try_recv().expect("valid test fixture") else {
            panic!("input batch expected");
        };
        assert_eq!(events.len(), 5);
        assert!(
            matches!(events[0], Input::MouseEvent(m) if m.x_position == 999 && m.flags == PointerFlags::MOVE)
        );
        assert!(
            matches!(events[1], Input::MouseEvent(m) if m.x_position == 999 && m.flags.contains(PointerFlags::DOWN))
        );
        assert!(
            matches!(events[2], Input::MouseEvent(m) if m.x_position == 1999 && m.flags == PointerFlags::MOVE)
        );
        assert!(
            matches!(events[3], Input::MouseEvent(m) if m.x_position == 1999 && m.flags == PointerFlags::LEFT_BUTTON)
        );
        assert!(matches!(events[4], Input::KeyboardEvent(_, 0x1c)));
        assert!(
            !tab.build_config()
                .expect("valid test fixture")
                .connector()
                .pointer_software_rendering
        );
    }

    #[test]
    fn gateway_transport_failure_displays_underlying_cause() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        drop(listener);
        let text = format!(
            "full address:s:desktop.example\nusername:s:alice\ndomain:s:LAB\ngatewayhostname:s:{address}"
        );
        let preview = crate::importers::rdp::parse(text.as_bytes(), "fixture").expect("parse");
        let row = &preview.rows[0];
        let identity = row.data.clone().expect("session");
        let mut tab = EmbeddedRdp::new(identity.clone(), identity, Secret::new(String::new()));
        tab.set_gateway(row.rdp_gateway.clone());
        let ctx = egui::Context::default();
        tab.start(&ctx);
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !tab.closed && std::time::Instant::now() < deadline {
            tab.poll(&ctx);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(tab.closed, "loopback failure must return");
        assert!(tab.status.contains("caused by:"), "{}", tab.status);
        assert_ne!(tab.status, "RDP : [GW connect] custom error");
    }
    #[test]
    fn configured_resolution_reaches_direct_and_gateway_negotiation() {
        let preview=crate::importers::rdp::parse(b"full address:s:desktop.example\nusername:s:alice\ndomain:s:LAB\ngatewayhostname:s:gw.example", "fixture").expect("parse");
        let row = &preview.rows[0];
        let data = row.data.clone().expect("data");
        let mut tab = EmbeddedRdp::new(data.clone(), data, Secret::new(String::new()));
        for gateway in [None, row.rdp_gateway.clone()] {
            tab.set_gateway(gateway);
            tab.set_resolution(Some((2560, 1440))).expect("resolution");
            let config = tab.build_config().expect("config");
            assert_eq!(config.connector().desktop_size.width, 2560);
            assert_eq!(config.connector().desktop_size.height, 1440);
            tab.dimensions = [640, 480]; // server framebuffer updates must not overwrite the next requested size
            assert_eq!(
                tab.build_config()
                    .expect("reconnect")
                    .connector()
                    .desktop_size
                    .width,
                2560
            );
        }
        assert!(tab.set_resolution(Some((65535, 65535))).is_err());
    }
    #[test]
    fn gateway_config_keeps_two_identities_target_port_and_clears_secrets() {
        let preview = crate::importers::rdp::parse(b"full address:s:desktop.example:3391\nusername:s:alice\ndomain:s:LAB\ngatewayhostname:s:gateway.example:444\ngatewayusername:s:GW\\bob", "fixture").expect("parse");
        let row = &preview.rows[0];
        let identity = row.data.clone().expect("session");
        let mut tab = EmbeddedRdp::new(
            identity.clone(),
            identity,
            Secret::new("desktop-secret".into()),
        );
        tab.set_gateway(row.rdp_gateway.clone());
        assert!(
            tab.same_credentials,
            "gateway must reuse desktop credentials by default"
        );
        let initial = tab.build_config().expect("default credentials");
        let ironrdp_client::config::Transport::Gateway(gw) = initial.transport() else {
            panic!("gateway required")
        };
        assert_eq!(gw.username, "LAB\\alice");
        assert_eq!(gw.password, "desktop-secret");
        tab.same_credentials = false;
        tab.gateway_password = Secret::new("gateway-secret".into());
        let config = tab.build_config().expect("config");
        assert_eq!(config.destination().port(), 3391);
        assert_eq!(config.tls_server_name.as_deref(), Some("desktop.example"));
        let ironrdp_client::config::Transport::Gateway(gw) = config.transport() else {
            panic!("gateway required")
        };
        assert_eq!(gw.endpoint, "gateway.example:444");
        assert_eq!(gw.username, "GW\\bob");
        assert_eq!(gw.password, "gateway-secret");
        assert!(!format!("{gw:?}").contains("gateway-secret"));
        tab.same_credentials = true;
        let config = tab.build_config().expect("shared credentials");
        let ironrdp_client::config::Transport::Gateway(gw) = config.transport() else {
            panic!("gateway required")
        };
        assert_eq!(gw.username, "LAB\\alice");
        assert_eq!(gw.password, "desktop-secret");
        tab.gateway.as_mut().expect("gateway").rpc_transport = true;
        assert!(tab.build_config().expect("RPC config").gateway_rpc);
        let json = serde_json::to_string(&tab.gateway).expect("serialize");
        let restored: Option<super::super::rdp::Gateway> =
            serde_json::from_str(&json).expect("reload");
        assert!(restored.expect("gateway").rpc_transport);
        tab.stop();
        assert!(tab.password.is_empty());
        assert!(tab.gateway_password.is_empty());
    }

    #[test]
    fn imported_rds_routing_reaches_x224_without_extra_cookie_prefix() {
        let preview = crate::importers::rdp::parse(b"full address:s:broker.example\nusername:s:alice\nloadbalanceinfo:s:tsv://MS Terminal Services Plugin.1.collection\nuse redirection server name:i:1", "fixture").expect("parse");
        let row = &preview.rows[0];
        let identity = row.data.clone().expect("session");
        let mut tab = EmbeddedRdp::new(identity.clone(), identity, Secret::new(String::new()));
        tab.set_routing(row.rdp_routing.clone());
        let config = tab.build_config().expect("config");
        let request = config.connector().request_data.as_ref().expect("routing");
        let mut bytes = vec![0; request.size()];
        request
            .write(&mut ironrdp_core::WriteCursor::new(&mut bytes))
            .expect("encode");
        assert_eq!(bytes, b"tsv://MS Terminal Services Plugin.1.collection\r\n");
        tab.set_routing(Default::default());
        assert!(
            tab.build_config()
                .expect("config")
                .connector()
                .request_data
                .is_none()
        );
    }
    #[test]
    fn coordinates_are_scaled_and_clamped() {
        let r = egui::Rect::from_min_size(egui::pos2(10., 20.), egui::vec2(640., 360.));
        assert_eq!(
            map_position(egui::pos2(330., 200.), r, [1280, 720]),
            (640, 360)
        );
        assert_eq!(
            map_position(egui::pos2(900., -1.), r, [1280, 720]),
            (1279, 0)
        );
    }
    #[test]
    fn text_is_unicode_and_navigation_uses_scancodes() {
        assert_eq!(scan_code(Key::A, false), None);
        assert_eq!(scan_code(Key::A, true), Some((0x1e, false)));
        assert_eq!(scan_code(Key::Delete, false), Some((0x53, true)));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    #[ignore = "Requires the isolated xrdp fixture on localhost:33989"]
    fn rdp_loopback_requires_certificate_consent_and_receives_desktop() {
        let password = std::fs::read_to_string("test-results/rdp-fixture-password.txt")
            .expect("fixture password");
        let s = SessionInput {
            name: "RDP fixture".into(),
            protocol: crate::models::Protocol::RDP,
            hostname: "127.0.0.1".into(),
            port: 33989,
            username: "hg-rdp-test".into(),
            domain: String::new(),
            folder_id: None,
            favorite: false,
            description: String::new(),
            color: "#7195ff".into(),
            icon: "server".into(),
            tags: vec![],
        };
        let ctx = egui::Context::default();
        for accept in [false, true] {
            let mut tab = EmbeddedRdp::new(
                s.clone(),
                s.clone(),
                zeroize::Zeroizing::new(password.clone()),
            );
            tab.start(&ctx);
            let deadline = std::time::Instant::now() + Duration::from_secs(45);
            let mut prompted = false;
            while std::time::Instant::now() < deadline {
                tab.poll(&ctx);
                if let Some(prompt) = tab.certificate.take() {
                    assert!(!prompt.fingerprint.is_empty());
                    prompted = true;
                    prompt.reply.send(accept).expect("decision");
                }
                if tab.closed || tab.texture.is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            assert!(
                prompted,
                "self-signed certificate must require consent: {}",
                tab.status
            );
            if accept {
                assert!(tab.texture.is_some(), "desktop must arrive: {}", tab.status);
                for _ in 0..100 {
                    tab.poll(&ctx);
                    std::thread::sleep(Duration::from_millis(50));
                }
                let mut ppm =
                    format!("P6\n{} {}\n255\n", tab.dimensions[0], tab.dimensions[1]).into_bytes();
                for pixel in &tab.frame_pixels {
                    ppm.extend_from_slice(&pixel.to_array()[..3]);
                }
                std::fs::write("test-results/rdp-test-frame.ppm", ppm).expect("frame");
                // The isolated xterm fixture writes proof that real RDP keyboard input arrived.
                for c in "printf verified > ~/hostgates-input-ok".encode_utf16() {
                    tab.send(Input::UnicodeKeyboardEvent(KeyboardFlags::empty(), c));
                    tab.send(Input::UnicodeKeyboardEvent(KeyboardFlags::RELEASE, c));
                }
                tab.key(0x1c, false, true);
                tab.key(0x1c, false, false);
                for _ in 0..40 {
                    tab.poll(&ctx);
                    std::thread::sleep(Duration::from_millis(50));
                }
            } else {
                assert!(tab.closed, "rejected certificate must fail");
                assert!(tab.texture.is_none());
            }
            tab.stop();
        }
    }
}
