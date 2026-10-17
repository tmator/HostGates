use super::*;
impl HostGates {
    pub(super) fn chrome(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(crate::i18n::tr("Fichier"), |ui| {
                if ui
                    .button(crate::i18n::tr("Nouvelle session…    Ctrl+N"))
                    .clicked()
                {
                    self.new_session();
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Nouveau dossier…")).clicked() {
                    self.folder_editor = Some(FolderEditor::new(self.folder.clone()));
                    ui.close();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        !self.busy,
                        egui::Button::new(crate::i18n::tr("Importer MobaXterm…")),
                    )
                    .clicked()
                {
                    self.choose_import();
                    ui.close();
                }
                if ui
                    .button(crate::i18n::tr("Exporter les sessions…"))
                    .clicked()
                {
                    self.begin_export();
                    ui.close();
                }
                ui.separator();
                if ui.button(crate::i18n::tr("Quitter")).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(crate::i18n::tr("Sessions"), |ui| {
                if ui
                    .add_enabled(
                        self.selected.is_some(),
                        egui::Button::new(crate::i18n::tr("Se connecter")),
                    )
                    .clicked()
                {
                    if let Some(s) = self.selected_session() {
                        self.connect(&s, ui.ctx());
                    }
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Terminal local")).clicked() {
                    self.local_terminal(ui.ctx());
                    ui.close();
                }
                if ui
                    .button(crate::i18n::tr("Afficher les sessions"))
                    .clicked()
                {
                    self.catalog_open = true;
                    ui.close();
                }
            });
            ui.menu_button(crate::i18n::tr("Affichage"), |ui| {
                for (id, label) in [
                    ("light", "Clair"),
                    ("dark", "Sombre"),
                    ("system", "Système"),
                ] {
                    if ui
                        .add_enabled(!self.busy, egui::Button::new(crate::i18n::tr(label)))
                        .clicked()
                    {
                        self.send(Action::Theme(id.into()));
                        ui.close();
                    }
                }
            });
            ui.menu_button(crate::i18n::tr("Paramètres"), |ui| {
                if ui.button(crate::i18n::tr("Préférences…")).clicked() {
                    self.settings = true;
                    ui.close();
                }
            });
            ui.menu_button(crate::i18n::tr("Aide"), |ui| {
                for (i, title) in super::help::TOPICS.iter().enumerate() {
                    if ui.button(crate::i18n::tr(title.0)).clicked() {
                        self.help_topic = i;
                        self.help_open = true;
                        ui.close();
                    }
                }
                ui.separator();
                if ui
                    .button(crate::i18n::tr("À propos de HostGates…"))
                    .clicked()
                {
                    self.about_open = true;
                    ui.close();
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.strong(APP_NAME);
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            for (label, action) in [
                ("Session", 0),
                ("Dossier", 1),
                ("Terminal", 2),
                ("Importer", 3),
                ("Exporter", 4),
                ("Paramètres", 5),
                ("Aide", 6),
            ] {
                let enabled = !self.busy || [2, 5, 6].contains(&action);
                if toolbar_button(ui, &crate::i18n::tr(label), action, enabled).clicked() && enabled
                {
                    match action {
                        0 => self.new_session(),
                        1 => self.folder_editor = Some(FolderEditor::new(self.folder.clone())),
                        2 => self.local_terminal(ui.ctx()),
                        3 => self.choose_import(),
                        4 => self.begin_export(),
                        5 => self.settings = true,
                        _ => self.help_open = true,
                    }
                }
            }
        });
    }
}

fn toolbar_button(ui: &mut egui::Ui, label: &str, kind: usize, enabled: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(83.0, 40.0), egui::Sense::click());
    let painter = ui.painter();
    if response.hovered() && enabled {
        painter.rect_filled(rect, 3.0, ui.visuals().widgets.hovered.bg_fill);
    }
    let c = if !enabled {
        ui.visuals().weak_text_color()
    } else if kind == 1 {
        Color32::from_rgb(204, 153, 35)
    } else {
        ui.visuals().text_color()
    };
    let center = egui::pos2(rect.center().x, rect.top() + 10.0);
    let icon_vec = |x: f32, y: f32| egui::vec2(x * 0.5, y * 0.5);
    let stroke = egui::Stroke::new(1.0, c);
    let icon = egui::Rect::from_center_size(center, icon_vec(24.0, 21.0));
    match kind {
        0 => {
            painter.rect_stroke(icon, 1.0, stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [center + icon_vec(-6.0, 0.0), center + icon_vec(6.0, 0.0)],
                stroke,
            );
            painter.line_segment(
                [center + icon_vec(0.0, -6.0), center + icon_vec(0.0, 6.0)],
                stroke,
            );
        }
        1 => {
            painter.rect_filled(icon, 1.0, c);
            painter.rect_filled(
                egui::Rect::from_min_size(icon.min - icon_vec(0.0, 3.0), icon_vec(11.0, 5.0)),
                1.0,
                c,
            );
        }
        2 => {
            painter.rect_filled(icon, 1.0, c);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                ">_",
                egui::FontId::monospace(7.5),
                Color32::WHITE,
            );
        }
        3 | 4 => {
            let dir = if kind == 3 { 1.0 } else { -1.0 };
            painter.line_segment(
                [center - icon_vec(0.0, 9.0), center + icon_vec(0.0, 9.0)],
                stroke,
            );
            let tip = center + icon_vec(0.0, 9.0 * dir);
            painter.line_segment([tip, tip + icon_vec(-6.0, -6.0 * dir)], stroke);
            painter.line_segment([tip, tip + icon_vec(6.0, -6.0 * dir)], stroke);
        }
        5 => {
            painter.circle_stroke(center, 4.5, stroke);
            painter.circle_stroke(center, 1.5, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = egui::vec2(a.cos(), a.sin());
                painter.line_segment([center + d * 4.0, center + d * 6.5], stroke);
            }
        }
        _ => {
            painter.circle_filled(center, 6.0, c);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional(9.5),
                Color32::WHITE,
            );
        }
    }
    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 12.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        ui.visuals().text_color(),
    );
    if enabled {
        response
    } else {
        response.on_disabled_hover_text(crate::i18n::tr("Une opération est en cours"))
    }
}
