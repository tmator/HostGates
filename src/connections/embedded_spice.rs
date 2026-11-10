//! Native SPICE in egui. All GLib objects and calls stay on one worker thread.
mod native;
use crate::{
    error::{AppError, Result},
    proxmox::SpiceConfig,
};
use eframe::egui;
use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

pub fn runtime_path(configured: &str) -> Result<PathBuf> {
    #[cfg(windows)]
    let name = "libspice-client-glib-2.0-8.dll";
    #[cfg(not(windows))]
    let name = "libspice-client-glib-2.0.so.8";
    let mut dirs = Vec::new();
    if !configured.trim().is_empty() {
        let path = PathBuf::from(configured.trim());
        if !path.is_absolute() || configured.chars().any(char::is_control) {
            return Err(error("SPICE : chemin absolu requis."));
        }
        if path.is_dir() {
            dirs.push(path);
        } else if path.file_name().is_some_and(|v| v == name) && path.is_file() {
            return Ok(path);
        } else if path.is_file() {
            #[cfg(not(windows))]
            if path.file_name().is_some_and(|v| v == "remote-viewer") {
                return runtime_path("");
            }
            dirs.push(path.parent().unwrap_or(Path::new("")).to_owned());
        }
    } else {
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            dirs.push(dir.join("spice-runtime"));
        }
        #[cfg(windows)]
        if let Some(root) = std::env::var_os("ProgramFiles")
            && let Ok(entries) = std::fs::read_dir(root)
        {
            for entry in entries.flatten() {
                if entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .starts_with("virtviewer")
                {
                    dirs.push(entry.path().join("bin"));
                }
            }
        }
        #[cfg(not(windows))]
        for dir in [
            "/usr/lib/x86_64-linux-gnu",
            "/lib/x86_64-linux-gnu",
            "/usr/lib/aarch64-linux-gnu",
            "/usr/lib64",
            "/usr/lib",
        ] {
            dirs.push(dir.into());
        }
    }
    dirs.into_iter().map(|d| d.join(name)).find(|p| p.is_file()).ok_or_else(|| error("SPICE : bibliothèque native introuvable. Installez VirtViewer (Windows), libspice-client-glib-2.0-8 (Debian/Ubuntu) ou spice-glib (Fedora/Rocky). Paramètres > SPICE : sélectionnez la bibliothèque ou l’ancien remote-viewer."))
}
fn error(message: &str) -> AppError {
    AppError::Operation(message.into())
}
#[derive(Clone, Copy)]
enum Input {
    Position(i32, i32, i32),
    Button(i32, bool, i32),
    Key(u32, bool),
}
struct Shared {
    stop: AtomicBool,
    dirty: AtomicBool,
    state: Mutex<State>,
    input: Mutex<VecDeque<Input>>,
    ctx: egui::Context,
}
struct State {
    status: String,
    closed: bool,
    frame: Option<egui::ColorImage>,
    cursor: Option<Cursor>,
}
enum Cursor {
    Shape(egui::ColorImage, [f32; 2]),
    Hidden,
    Default,
}
impl Shared {
    fn status(&self, text: &str, closed: bool) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.status = text.into();
        s.closed = closed;
        drop(s);
        self.ctx.request_repaint();
    }
    fn send(&self, input: Input) {
        let mut q = self.input.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(input, Input::Position(..)) && matches!(q.back(), Some(Input::Position(..))) {
            q.pop_back();
        }
        if q.len() >= 1024 {
            self.stop.store(true, Ordering::Release);
            return;
        }
        q.push_back(input);
    }
}
pub struct EmbeddedSpice {
    shared: Arc<Shared>,
    texture: Option<egui::TextureHandle>,
    size: [usize; 2],
    cursor: Option<(egui::TextureHandle, [f32; 2])>,
    cursor_hidden: bool,
    keys: BTreeSet<u32>,
    buttons: i32,
    pub closed: bool,
    status: String,
}
impl EmbeddedSpice {
    pub fn new(config: SpiceConfig, path: &Path, ctx: &egui::Context) -> Result<Self> {
        config.validate()?;
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            dirty: AtomicBool::new(true),
            state: Mutex::new(State {
                status: "SPICE : connexion…".into(),
                closed: false,
                frame: None,
                cursor: None,
            }),
            input: Mutex::new(VecDeque::new()),
            ctx: ctx.clone(),
        });
        native::start(config, path.to_owned(), shared.clone())?;
        Ok(Self {
            shared,
            texture: None,
            size: [1, 1],
            cursor: None,
            cursor_hidden: false,
            keys: BTreeSet::new(),
            buttons: 0,
            closed: false,
            status: String::new(),
        })
    }
    pub fn poll(&mut self, ctx: &egui::Context) {
        let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        self.closed = state.closed;
        self.status.clone_from(&state.status);
        if let Some(cursor) = state.cursor.take() {
            match cursor {
                Cursor::Shape(image, hot) => {
                    self.cursor = Some((
                        ctx.load_texture("spice-cursor", image, egui::TextureOptions::LINEAR),
                        hot,
                    ));
                    self.cursor_hidden = false;
                }
                Cursor::Hidden => {
                    self.cursor = None;
                    self.cursor_hidden = true;
                }
                Cursor::Default => {
                    self.cursor = None;
                    self.cursor_hidden = false;
                }
            }
        }
        if let Some(frame) = state.frame.take() {
            self.size = frame.size;
            if let Some(texture) = &mut self.texture {
                texture.set(frame, egui::TextureOptions::LINEAR);
            } else {
                self.texture =
                    Some(ctx.load_texture("spice-display", frame, egui::TextureOptions::LINEAR));
            }
        }
    }
    fn key(&mut self, code: u32, down: bool) {
        if down {
            self.keys.insert(code);
        } else {
            self.keys.remove(&code);
        }
        self.shared.send(Input::Key(code, down));
    }
    pub fn release_input(&mut self) {
        for code in std::mem::take(&mut self.keys) {
            self.shared.send(Input::Key(code, false));
        }
        for button in 1..=3 {
            if self.buttons & (1 << (button - 1)) != 0 {
                self.shared.send(Input::Button(button, false, 0));
            }
        }
        self.buttons = 0;
    }
    pub fn stop(&mut self) {
        self.release_input();
        self.shared.stop.store(true, Ordering::Release);
        self.closed = true;
    }
    pub fn show(&mut self, ui: &mut egui::Ui, interactive: bool) {
        ui.label(crate::i18n::tr(&self.status));
        let Some(texture) = &self.texture else {
            return;
        };
        let original = egui::vec2(self.size[0] as f32, self.size[1] as f32);
        let available = ui.available_size();
        let scale = (available.x / original.x)
            .min(available.y / original.y)
            .max(0.01);
        let response = ui.add(
            egui::Image::new((texture.id(), original * scale)).sense(egui::Sense::click_and_drag()),
        );
        if response.hovered() && interactive && !self.closed {
            if let Some((cursor, hot)) = &self.cursor {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::None);
                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let rect = egui::Rect::from_min_size(
                        pos - egui::vec2(hot[0], hot[1]) * scale,
                        cursor.size_vec2() * scale,
                    );
                    ui.painter().with_clip_rect(response.rect).image(
                        cursor.id(),
                        rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                        egui::Color32::WHITE,
                    );
                }
            } else if self.cursor_hidden {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::None);
            }
        }
        if interactive
            && (response.clicked() || (response.hovered() && ui.input(|i| i.pointer.any_pressed())))
        {
            response.request_focus();
        }
        let focused =
            interactive && !self.closed && response.has_focus() && ui.input(|i| i.focused);
        if !focused {
            self.release_input();
            return;
        }
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            )
        });
        let modifiers = ui.input(|i| i.modifiers);
        for (code, down) in [
            (0x2a, modifiers.shift),
            (0x1d, modifiers.ctrl),
            (0x38, modifiers.alt),
            (0x15b, modifiers.mac_cmd),
        ] {
            if self.keys.contains(&code) != down {
                self.key(code, down);
            }
        }
        for event in ui.input(|i| i.events.clone()) {
            match event {
                egui::Event::Key {
                    key,
                    physical_key,
                    pressed,
                    ..
                } => {
                    if let Some(code) = scan_code(physical_key.unwrap_or(key)) {
                        self.key(code, pressed);
                    }
                }
                egui::Event::PointerMoved(pos) if response.hovered() || self.buttons != 0 => {
                    let p = (pos - response.rect.min) / scale;
                    self.shared.send(Input::Position(
                        (p.x as i32).clamp(0, self.size[0] as i32 - 1),
                        (p.y as i32).clamp(0, self.size[1] as i32 - 1),
                        self.buttons,
                    ));
                }
                egui::Event::PointerButton {
                    button,
                    pressed,
                    pos,
                    ..
                } if response.rect.contains(pos) || !pressed => {
                    let button = match button {
                        egui::PointerButton::Primary => 1,
                        egui::PointerButton::Middle => 2,
                        egui::PointerButton::Secondary => 3,
                        _ => continue,
                    };
                    let p = (pos - response.rect.min) / scale;
                    if pressed {
                        self.shared.send(Input::Position(
                            (p.x as i32).clamp(0, self.size[0] as i32 - 1),
                            (p.y as i32).clamp(0, self.size[1] as i32 - 1),
                            self.buttons,
                        ));
                    }
                    let mask = 1 << (button - 1);
                    if pressed {
                        self.buttons |= mask;
                    } else {
                        self.buttons &= !mask;
                    }
                    self.shared
                        .send(Input::Button(button, pressed, self.buttons));
                }
                egui::Event::MouseWheel { delta, .. } if response.hovered() && delta.y != 0.0 => {
                    let button = if delta.y > 0.0 { 4 } else { 5 };
                    self.shared.send(Input::Button(button, true, self.buttons));
                    self.shared.send(Input::Button(button, false, self.buttons));
                }
                _ => {}
            }
        }
    }
}
impl Drop for EmbeddedSpice {
    fn drop(&mut self) {
        self.stop();
    }
}
fn scan_code(key: egui::Key) -> Option<u32> {
    use egui::Key;
    let extra = match key {
        Key::Num1 => 2,
        Key::Num2 => 3,
        Key::Num3 => 4,
        Key::Num4 => 5,
        Key::Num5 => 6,
        Key::Num6 => 7,
        Key::Num7 => 8,
        Key::Num8 => 9,
        Key::Num9 => 10,
        Key::Num0 => 11,
        Key::Minus => 12,
        Key::Equals => 13,
        Key::OpenBracket => 26,
        Key::CloseBracket => 27,
        Key::Semicolon => 39,
        Key::Quote => 40,
        Key::Backtick => 41,
        Key::Backslash => 43,
        Key::Comma => 51,
        Key::Period => 52,
        Key::Slash => 53,
        _ => 0,
    };
    if extra != 0 {
        return Some(extra);
    }
    super::embedded_rdp::scan_code(key, true)
        .map(|(code, extended)| u32::from(code) | if extended { 0x100 } else { 0 })
}
