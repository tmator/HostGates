use crate::connections::terminal::Terminal;
use eframe::egui::{self, Color32, Event, FontId, Key};

#[derive(Default)]
pub struct TerminalView {
    pub selection: Option<((u16, u16), (u16, u16))>,
    pub focus: bool,
    pub font_size: f32,
    pub search: String,
    pub search_open: bool,
}
pub fn key_bytes(key: Key, modifiers: egui::Modifiers, application: bool) -> Option<Vec<u8>> {
    if modifiers.ctrl && !modifiers.alt && !modifiers.shift {
        let c = match key {
            Key::A => 1,
            Key::B => 2,
            Key::C => 3,
            Key::D => 4,
            Key::E => 5,
            Key::F => 6,
            Key::G => 7,
            Key::H => 8,
            Key::I => 9,
            Key::J => 10,
            Key::K => 11,
            Key::L => 12,
            Key::M => 13,
            Key::N => 14,
            Key::O => 15,
            Key::P => 16,
            Key::Q => 17,
            Key::R => 18,
            Key::S => 19,
            Key::T => 20,
            Key::U => 21,
            Key::V => 22,
            Key::W => 23,
            Key::X => 24,
            Key::Y => 25,
            Key::Z => 26,
            _ => return None,
        };
        return Some(vec![c]);
    }
    let seq = match key {
        Key::Enter => "\r",
        Key::Backspace => "\x7f",
        Key::Tab => {
            if modifiers.shift {
                "\x1b[Z"
            } else {
                "\t"
            }
        }
        Key::Escape => "\x1b",
        Key::ArrowUp => {
            if application {
                "\x1bOA"
            } else {
                "\x1b[A"
            }
        }
        Key::ArrowDown => {
            if application {
                "\x1bOB"
            } else {
                "\x1b[B"
            }
        }
        Key::ArrowRight => {
            if application {
                "\x1bOC"
            } else {
                "\x1b[C"
            }
        }
        Key::ArrowLeft => {
            if application {
                "\x1bOD"
            } else {
                "\x1b[D"
            }
        }
        Key::Home => "\x1b[H",
        Key::End => "\x1b[F",
        Key::Insert => "\x1b[2~",
        Key::Delete => "\x1b[3~",
        Key::PageUp => "\x1b[5~",
        Key::PageDown => "\x1b[6~",
        Key::F1 => "\x1bOP",
        Key::F2 => "\x1bOQ",
        Key::F3 => "\x1bOR",
        Key::F4 => "\x1bOS",
        Key::F5 => "\x1b[15~",
        Key::F6 => "\x1b[17~",
        Key::F7 => "\x1b[18~",
        Key::F8 => "\x1b[19~",
        Key::F9 => "\x1b[20~",
        Key::F10 => "\x1b[21~",
        Key::F11 => "\x1b[23~",
        Key::F12 => "\x1b[24~",
        _ => return None,
    };
    Some(seq.as_bytes().to_vec())
}
fn color(c: vt100::Color, default: Color32) -> Color32 {
    match c {
        vt100::Color::Default => default,
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
        vt100::Color::Idx(i) => {
            const BASE: [[u8; 3]; 16] = [
                [25, 28, 32],
                [205, 77, 77],
                [116, 185, 88],
                [218, 184, 83],
                [100, 151, 218],
                [181, 120, 205],
                [85, 185, 190],
                [218, 221, 225],
                [100, 107, 117],
                [246, 115, 115],
                [156, 218, 120],
                [250, 215, 116],
                [147, 186, 244],
                [217, 163, 239],
                [130, 218, 219],
                [250, 250, 250],
            ];
            let [r, g, b] = if i < 16 {
                BASE[i as usize]
            } else if i >= 232 {
                let v = 8 + (i - 232) * 10;
                [v, v, v]
            } else {
                let n = i - 16;
                let scale = |x: u8| if x == 0 { 0 } else { 55 + 40 * x };
                [scale(n / 36), scale((n / 6) % 6), scale(n % 6)]
            };
            Color32::from_rgb(r, g, b)
        }
    }
}
impl TerminalView {
    pub fn show(&mut self, ui: &mut egui::Ui, term: &mut Terminal, interactive: bool) {
        if self.font_size == 0.0 {
            self.font_size = 14.0;
            self.focus = true;
        }
        ui.horizontal(|ui| {
            ui.label(crate::i18n::tr(&term.status));
            if ui.small_button(crate::i18n::tr("Copier l'écran")).clicked() {
                ui.ctx().copy_text(term.parser.screen().contents());
            }
            if ui.small_button(crate::i18n::tr("Rechercher")).clicked() {
                self.search_open = !self.search_open;
            }
            if ui.small_button(crate::i18n::tr("−")).clicked() {
                self.font_size = (self.font_size - 1.0).max(10.0);
            }
            if ui.small_button(crate::i18n::tr("+")).clicked() {
                self.font_size = (self.font_size + 1.0).min(26.0);
            }
        });
        if self.search_open {
            ui.horizontal(|ui| {
                ui.label(crate::i18n::tr(
                    "Recherche dans l'écran / historique affiché",
                ));
                ui.text_edit_singleline(&mut self.search);
                let count = if self.search.is_empty() {
                    0
                } else {
                    term.parser
                        .screen()
                        .contents()
                        .matches(&self.search)
                        .count()
                };
                ui.label(crate::i18n::tr(&format!("{count} occurrence(s)")));
            });
        }
        let font = FontId::monospace(self.font_size);
        let glyph = ui
            .painter()
            .layout_no_wrap("M".into(), font.clone(), Color32::WHITE);
        let cw = glyph.size().x;
        let ch = glyph.size().y.max(self.font_size + 2.0);
        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());
        if interactive && (self.focus || response.clicked()) {
            response.request_focus();
            self.focus = false;
        }
        if response.has_focus() && interactive {
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
        }
        term.resize(
            (rect.height() / ch).floor().max(2.0) as u16,
            (rect.width() / cw).floor().max(10.0) as u16,
        );
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.5 {
                let previous = term.parser.screen().scrollback();
                let next = if scroll > 0.0 {
                    previous.saturating_add(3)
                } else {
                    previous.saturating_sub(3)
                };
                term.parser.screen_mut().set_scrollback(next);
            }
        }
        let position = |p: egui::Pos2| {
            (
                ((p.y - rect.top()) / ch)
                    .floor()
                    .clamp(0.0, (term.size.0 - 1) as f32) as u16,
                ((p.x - rect.left()) / cw)
                    .floor()
                    .clamp(0.0, (term.size.1 - 1) as f32) as u16,
            )
        };
        if response.drag_started()
            && let Some(p) = response.interact_pointer_pos()
        {
            let p = position(p);
            self.selection = Some((p, p));
        }
        if response.dragged()
            && let (Some((start, _)), Some(p)) = (self.selection, response.interact_pointer_pos())
        {
            self.selection = Some((start, position(p)));
        }
        if response.clicked() {
            self.selection = None;
        }
        if response.has_focus() && interactive {
            let events = ui.input(|i| i.events.clone());
            for event in events {
                match event {
                    Event::Copy => {
                        if ui.input(|i| i.modifiers.shift) {
                            ui.ctx().copy_text(self.selected_text(term));
                        } else {
                            term.write(vec![3]);
                        }
                    }
                    Event::Cut => {
                        term.write(vec![24]);
                    }
                    Event::Paste(text) => {
                        let text = sanitized_paste(&text);
                        let bytes = if term.parser.screen().bracketed_paste() {
                            format!("\x1b[200~{text}\x1b[201~").into_bytes()
                        } else {
                            text.into_bytes()
                        };
                        term.write(bytes);
                    }
                    Event::Text(text) => {
                        term.parser.screen_mut().set_scrollback(0);
                        term.write(text.into_bytes());
                    }
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        if modifiers.ctrl && modifiers.shift && key == Key::C {
                            ui.ctx().copy_text(self.selected_text(term));
                            continue;
                        }
                        if modifiers.ctrl && modifiers.shift && key == Key::F {
                            self.search_open = true;
                            continue;
                        }
                        if modifiers.shift && [Key::PageUp, Key::PageDown].contains(&key) {
                            let p = term.parser.screen().scrollback();
                            term.parser
                                .screen_mut()
                                .set_scrollback(if key == Key::PageUp {
                                    p + 20
                                } else {
                                    p.saturating_sub(20)
                                });
                            continue;
                        }
                        if let Some(bytes) =
                            key_bytes(key, modifiers, term.parser.screen().application_cursor())
                        {
                            term.parser.screen_mut().set_scrollback(0);
                            term.write(bytes);
                        }
                    }
                    _ => {}
                }
            }
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(25, 28, 32));
        let screen = term.parser.screen();
        for row in 0..term.size.0 {
            for col in 0..term.size.1 {
                let Some(cell) = screen.cell(row, col) else {
                    continue;
                };
                if cell.is_wide_continuation() {
                    continue;
                }
                let pos = rect.min + egui::vec2(col as f32 * cw, row as f32 * ch);
                let cell_rect = egui::Rect::from_min_size(
                    pos,
                    egui::vec2(cw * if cell.is_wide() { 2.0 } else { 1.0 }, ch),
                );
                let mut fg = color(cell.fgcolor(), Color32::from_rgb(220, 224, 230));
                let mut bg = color(cell.bgcolor(), Color32::from_rgb(25, 28, 32));
                if cell.inverse() {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if self.selection.is_some_and(|(a, b)| {
                    let (start, end) = if a <= b { (a, b) } else { (b, a) };
                    (row, col) >= start && (row, col) <= end
                }) {
                    bg = Color32::from_rgb(57, 81, 104);
                }
                painter.rect_filled(cell_rect, 0.0, bg);
                let contents = cell.contents();
                if !contents.is_empty() {
                    painter.text(pos, egui::Align2::LEFT_TOP, contents, font.clone(), fg);
                }
            }
        }
        if !screen.hide_cursor() && screen.scrollback() == 0 && !term.closed {
            let (r, c) = screen.cursor_position();
            let pos = rect.min + egui::vec2(c as f32 * cw, r as f32 * ch);
            painter.rect_stroke(
                egui::Rect::from_min_size(pos, egui::vec2(cw, ch)),
                0.0,
                egui::Stroke::new(1.0, Color32::LIGHT_GREEN),
                egui::StrokeKind::Inside,
            );
        }
        response.context_menu(|ui| {
            if ui.button(crate::i18n::tr("Copier la sélection")).clicked() {
                ui.ctx().copy_text(self.selected_text(term));
                ui.close();
            }
            ui.label(crate::i18n::tr("Coller : Ctrl+V • Interrompre : Ctrl+C"));
        });
    }
    fn selected_text(&self, term: &Terminal) -> String {
        let Some((a, b)) = self.selection else {
            return String::new();
        };
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        let mut out = String::new();
        for r in start.0..=end.0 {
            let from = if r == start.0 { start.1 } else { 0 };
            let to = if r == end.0 { end.1 } else { term.size.1 - 1 };
            let mut line = String::new();
            for c in from..=to {
                if let Some(cell) = term.parser.screen().cell(r, c)
                    && !cell.is_wide_continuation()
                {
                    let text = cell.contents();
                    if text.is_empty() {
                        line.push(' ')
                    } else {
                        line.push_str(text)
                    }
                }
            }
            out.push_str(line.trim_end());
            if r < end.0 {
                out.push('\n');
            }
        }
        out
    }
}

fn sanitized_paste(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\t' | '\n' | '\r'))
        .collect()
}
#[cfg(test)]
mod paste_security_tests {
    #[test]
    fn clipboard_cannot_close_bracketed_paste_or_send_controls() {
        assert_eq!(
            super::sanitized_paste("a\x1b[201~\x03\u{009b}b\n\t"),
            "a[201~b\n\t"
        );
    }
}
