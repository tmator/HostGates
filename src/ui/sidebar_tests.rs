use super::*;
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};

struct Harness {
    ctx: egui::Context,
    time: f64,
    row: Rect,
    folder: Rect,
    root: Rect,
    clicks: usize,
    connects: usize,
    drops: Vec<(String, bool)>,
    source_first: bool,
    open: bool,
}
impl Harness {
    fn new(source_first: bool, open: bool) -> Self {
        let mut h = Self {
            ctx: Default::default(),
            time: 0.0,
            row: Rect::NOTHING,
            folder: Rect::NOTHING,
            root: Rect::NOTHING,
            clicks: 0,
            connects: 0,
            drops: vec![],
            source_first,
            open,
        };
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }
    fn source(&mut self, ui: &mut egui::Ui) {
        let r = session_row(
            ui,
            "session-id",
            "Serveur",
            &crate::models::Protocol::SSH,
            false,
        );
        self.row = r.rect;
        self.clicks += usize::from(r.clicked());
        self.connects += usize::from(r.double_clicked());
    }
    fn destination(&mut self, ui: &mut egui::Ui) {
        let r = egui::CollapsingHeader::new("Destination")
            .id_salt("destination")
            .open(Some(self.open))
            .show(ui, |ui| {
                ui.label("Contenu");
            })
            .header_response;
        self.folder = r.rect;
        if let Some(p) = r.dnd_release_payload::<SessionDrag>() {
            self.drops.push((p.0.clone(), false));
        }
    }
    fn frame(&mut self, events: Vec<Event>) {
        self.time += 0.05;
        let input = egui::RawInput {
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let ctx = self.ctx.clone();
        let mut output = ctx.run_ui(input, |ui| {
            ui.set_width(250.0);
            let root = ui.label("Racine (sans dossier)");
            self.root = root.rect;
            if let Some(p) = root.dnd_release_payload::<SessionDrag>() {
                self.drops.push((p.0.clone(), true));
            }
            if self.source_first {
                self.source(ui);
                self.destination(ui);
            } else {
                self.destination(ui);
                self.source(ui);
            }
        });
        output.textures_delta.clear();
    }
    fn button(&mut self, pos: Pos2, pressed: bool) {
        self.frame(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            },
        ]);
    }
    fn drag_to(&mut self, target: Pos2) {
        let from = self.row.left_center() + egui::vec2(50.0, 0.0);
        self.button(from, true);
        self.frame(vec![Event::PointerMoved(from + egui::vec2(12.0, 0.0))]);
        self.frame(vec![Event::PointerMoved(target)]);
        self.button(target, false);
    }
}
#[test]
fn double_click_connects_from_text_and_icon() {
    for x in [10.0, 60.0] {
        let mut h = Harness::new(true, false);
        let pos = h.row.left_center() + egui::vec2(x, 0.0);
        h.button(pos, true);
        h.button(pos, false);
        h.button(pos, true);
        h.button(pos, false);
        assert_eq!(h.clicks, 2);
        assert_eq!(h.connects, 1);
        assert!(!egui::DragAndDrop::has_any_payload(&h.ctx));
    }
}
#[test]
fn dropping_on_open_or_closed_folder_works_in_both_render_orders() {
    for source_first in [true, false] {
        for open in [true, false] {
            let mut h = Harness::new(source_first, open);
            h.drag_to(h.folder.center());
            assert_eq!(
                h.drops,
                vec![("session-id".into(), false)],
                "source_first={source_first}, open={open}"
            );
            assert_eq!(h.connects, 0);
            assert_eq!(h.clicks, 0);
        }
    }
}
#[test]
fn dropping_at_root_and_outside() {
    let mut h = Harness::new(true, false);
    h.drag_to(h.root.center());
    assert_eq!(h.drops, vec![("session-id".into(), true)]);
    h.drops.clear();
    h.drag_to(egui::pos2(600.0, 600.0));
    assert!(h.drops.is_empty());
    assert!(!egui::DragAndDrop::has_any_payload(&h.ctx));
    assert_eq!(h.connects, 0);
}

#[test]
fn cancelled_drag_does_not_move_and_following_double_click_connects() {
    let mut h = Harness::new(true, false);
    let from = h.row.left_center() + egui::vec2(60.0, 0.0);
    h.button(from, true);
    h.frame(vec![Event::PointerMoved(from + egui::vec2(12.0, 0.0))]);
    h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.button(h.folder.center(), false);
    assert!(h.drops.is_empty());
    assert!(!egui::DragAndDrop::has_any_payload(&h.ctx));
    h.time += 1.0;
    h.button(from, true);
    h.button(from, false);
    h.button(from, true);
    h.button(from, false);
    assert_eq!(h.connects, 1);
}

fn sidebar_app() -> HostGates {
    HostGates {
        advanced: super::super::advanced::Advanced::default(),
        worker: crate::services::worker::Worker {
            tx: std::sync::mpsc::channel().0,
            rx: std::sync::mpsc::channel().1,
        },
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
        startup_checked: true,
        unlock_open: false,
        unlock_password: zeroize::Zeroizing::new(String::new()),
        help_topic: 0,
    }
}

#[test]
fn six_panes_keep_separate_terminals_and_only_focused_pane_gets_input() {
    let mut app = sidebar_app();
    app.busy = false;
    app.advanced.panes = 6;
    let mut inputs = Vec::new();
    for i in 0..8 {
        let (t, rx) = crate::connections::terminal::Terminal::fixture();
        inputs.push(rx);
        app.tabs.push(super::super::tabs::SessionTab {
            id: i.to_string(),
            title: format!("Terminal {i}"),
            session: None,
            client: super::super::tabs::TabClient::Terminal(Box::new(t), Default::default()),
            files: None,
            tunnel: None,
        });
    }
    app.active_tab = Some("0".into());
    let ctx = egui::Context::default();
    let pos = egui::pos2(150.0, 200.0);
    for frame in 0..8 {
        let events = match frame {
            3 => vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            4 => vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
            6 => vec![Event::Text("FOCUS_TEST".into())],
            _ => vec![],
        };
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(frame as f64 / 10.0),
                events,
                ..Default::default()
            },
            |ui| {
                ui.set_max_size(egui::vec2(1200.0, 900.0));
                app.workspace(ui);
            },
        );
        output.textures_delta.clear();
    }
    assert_eq!(app.tabs.len(), 8);
    assert_eq!(app.advanced.visible.len(), 6);
    assert_eq!(
        app.advanced
            .visible
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        6
    );
    let receivers = inputs
        .iter()
        .filter(|rx| rx.try_iter().any(|v| v == b"FOCUS_TEST"))
        .count();
    assert_eq!(
        receivers, 1,
        "exactly the focused pane receives keyboard input"
    );
    for id in &app.advanced.visible {
        let t = app.tabs.iter().find(|t| &t.id == id).expect("tab");
        if let super::super::tabs::TabClient::Terminal(t, _) = &t.client {
            assert!(t.size.1 < 100, "pane must fit in the grid");
        }
    }
}

#[test]
fn sidebar_width_stays_constant_when_pointer_moves() {
    let mut app = sidebar_app();
    let mut db = crate::database::Database::in_memory().expect("db");
    let input = format!(
        "[Bookmarks]\nSubRep=Parent/Child\n{}=#70#0%192.0.2.1%22%user",
        "Long name ".repeat(10)
    );
    db.import(&crate::importers::mobaxterm::parse(input.as_bytes()))
        .expect("fixture");
    app.snapshot = db.snapshot().expect("snapshot");
    app.busy = false;
    let ctx = egui::Context::default();
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(7.0, 5.0);
        s.spacing.button_padding = egui::vec2(9.0, 5.0);
    });
    for (pass, search) in ["", "192.0.2."].into_iter().enumerate() {
        app.search = search.into();
        let mut widths = Vec::new();
        for frame in 0..80 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some((pass * 80 + frame) as f64 / 60.0),
                    events: vec![Event::PointerMoved(egui::pos2(
                        25.0 + (frame % 12) as f32 * 15.0,
                        40.0 + frame as f32 * 3.0,
                    ))],
                    ..Default::default()
                },
                |ui| {
                    widths.push(app.sidebar_panel(ui).response.rect.width());
                },
            );
            output.textures_delta.clear();
        }
        let reference = widths[10];
        assert!(reference < 350.0, "unexpected expansion: {reference}");
        assert!(
            widths[10..].iter().all(|w| (w - reference).abs() < 1.0),
            "width drift: {widths:?}"
        );
    }
}

#[test]
fn workspace_clamps_panes_to_open_tabs_and_clears_when_empty() {
    let mut app = sidebar_app();
    app.busy = false;
    app.advanced.panes = 6;
    for i in 0..3 {
        let (terminal, _) = crate::connections::terminal::Terminal::fixture();
        app.tabs.push(super::super::tabs::SessionTab {
            id: i.to_string(),
            title: format!("Terminal {i}"),
            session: None,
            client: super::super::tabs::TabClient::Terminal(Box::new(terminal), Default::default()),
            files: None,
            tunnel: None,
        });
    }
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        app.workspace(ui);
    });
    output.textures_delta.clear();
    assert_eq!(app.advanced.panes, 3);
    assert_eq!(app.advanced.visible.len(), 3);
    app.tabs.truncate(1);
    let mut output = ctx.run_ui(Default::default(), |ui| {
        app.workspace(ui);
    });
    output.textures_delta.clear();
    assert_eq!(app.advanced.panes, 1);
    assert_eq!(app.advanced.visible.len(), 1);
    app.tabs.clear();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        app.workspace(ui);
    });
    output.textures_delta.clear();
    assert!(app.active_tab.is_none());
    assert!(app.advanced.visible.is_empty());
    assert!(
        output
            .shapes
            .iter()
            .all(|s| !matches!(s.shape, egui::epaint::Shape::Text(_))),
        "empty workspace must not display catalog or tabs"
    );
}

fn shortcut_frame(app: &mut super::super::HostGates, ctx: &egui::Context, key: egui::Key) {
    let mut output = ctx.run_ui(
        egui::RawInput {
            events: vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ui| {
            app.dialog_shortcuts(ui.ctx());
        },
    );
    output.textures_delta.clear();
}
#[test]
fn escape_discards_only_front_dialog_and_enter_does_not_fall_through() {
    let mut app = sidebar_app();
    app.busy = false;
    app.editor = Some(super::super::SessionEditor::new(None));
    app.help_open = true;
    let ctx = egui::Context::default();
    shortcut_frame(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.help_open);
    assert!(app.editor.is_some());
    assert!(!super::super::shortcuts::submitted(&ctx, "editor"));
    shortcut_frame(&mut app, &ctx, egui::Key::Enter);
    assert!(super::super::shortcuts::submitted(&ctx, "editor"));
    shortcut_frame(&mut app, &ctx, egui::Key::Escape);
    assert!(app.editor.is_none());
    assert!(!super::super::shortcuts::submitted(&ctx, "editor"));
    app.unlock_open = true;
    app.unlock_password = zeroize::Zeroizing::new("fixture-master".into());
    shortcut_frame(&mut app, &ctx, egui::Key::Enter);
    assert!(super::super::shortcuts::submitted(&ctx, "unlock"));
    shortcut_frame(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.unlock_open);
    assert!(app.unlock_password.is_empty());
}
#[test]
fn escape_discards_manual_gateway_password_and_busy_blocks_enter() {
    let mut app = sidebar_app();
    app.advanced.options_open = true;
    app.advanced.gateway_draft = Some(super::super::advanced::GatewayDraft {
        password: zeroize::Zeroizing::new("fixture".into()),
        ..Default::default()
    });
    let ctx = egui::Context::default();
    app.busy = true;
    shortcut_frame(&mut app, &ctx, egui::Key::Enter);
    assert!(!super::super::shortcuts::submitted(&ctx, "options"));
    shortcut_frame(&mut app, &ctx, egui::Key::Escape);
    assert!(app.advanced.gateway_draft.is_none());
    assert!(!app.advanced.options_open);
}

#[test]
fn shortcuts_follow_front_window_and_preserve_multiline_enter() {
    let mut app = sidebar_app();
    app.busy = false;
    app.editor = Some(super::super::SessionEditor::new(None));
    app.help_open = true;
    let ctx = egui::Context::default();
    let mut text = String::new();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        egui::Window::new("Editor")
            .id(egui::Id::new("session_editor"))
            .show(ui.ctx(), |ui| {
                ui.label("Fixture");
            });
        egui::Window::new("Help")
            .id(egui::Id::new("help_dialog"))
            .show(ui.ctx(), |ui| {
                ui.label("Fixture");
            });
    });
    output.textures_delta.clear();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        ui.ctx().move_to_top(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("session_editor"),
        ));
    });
    output.textures_delta.clear();
    shortcut_frame(&mut app, &ctx, egui::Key::Enter);
    assert!(super::super::shortcuts::submitted(&ctx, "editor"));
    assert!(app.help_open);
    app.help_open = false;
    let mut output = ctx.run_ui(Default::default(), |ui| {
        ui.add(egui::TextEdit::multiline(&mut text).id(egui::Id::new("session_description")))
            .request_focus();
    });
    output.textures_delta.clear();
    shortcut_frame(&mut app, &ctx, egui::Key::Enter);
    assert!(!super::super::shortcuts::submitted(&ctx, "editor"));
}
