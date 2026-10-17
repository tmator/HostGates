use super::*;
#[derive(Clone)]
struct SessionDrag(String);
impl HostGates {
    pub(super) fn sidebar_panel(&mut self, ui: &mut egui::Ui) -> egui::InnerResponse<()> {
        egui::Panel::left("sidebar")
            .default_size(260.0)
            .size_range(190.0..=480.0)
            .resizable(true)
            .show(ui, |ui| {
                self.sidebar(ui);
            })
    }
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
            ui.style_mut()
                .text_styles
                .insert(style, egui::FontId::proportional(12.0));
        }
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(10.0));
        ui.strong(crate::i18n::tr("Sessions utilisateur"));
        ui.add_space(5.0);
        let search = ui
            .horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text(crate::i18n::tr("Nom ou adresse IP…"))
                        .desired_width((ui.available_width() - 32.0).max(60.0)),
                );
                if ui
                    .small_button(crate::i18n::tr("×"))
                    .on_hover_text(crate::i18n::tr("Effacer la recherche"))
                    .clicked()
                {
                    self.search.clear();
                }
                response
            })
            .inner;
        if self.active_tab.is_none()
            && ui
                .ctx()
                .input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F))
        {
            search.request_focus();
        }
        ui.add_space(12.0);
        if ui
            .selectable_label(
                self.folder.is_none() && !self.favorites,
                crate::i18n::tr("Toutes les sessions"),
            )
            .clicked()
        {
            self.folder = None;
            self.favorites = false;
        }
        if ui
            .selectable_label(self.favorites, crate::i18n::tr("★ Favoris"))
            .clicked()
        {
            self.folder = None;
            self.favorites = true;
        }
        ui.add_space(16.0);
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.strong(crate::i18n::tr("Arborescence"));
            if ui.small_button(crate::i18n::tr("Tout ouvrir")).clicked() {
                self.tree_open = Some(true);
            }
            if ui.small_button(crate::i18n::tr("Replier")).clicked() {
                self.tree_open = Some(false);
            }
            if ui
                .add_enabled(!self.busy, egui::Button::new(crate::i18n::tr("+")))
                .on_hover_text(crate::i18n::tr("Nouveau dossier"))
                .clicked()
            {
                self.folder_editor = Some(FolderEditor::new(self.folder.clone()));
            }
        });
        if !self.search.trim().is_empty() {
            let count = self
                .snapshot
                .sessions
                .iter()
                .filter(|s| {
                    matches_search(s, &self.search, &self.snapshot)
                        && (!self.favorites || s.data.favorite)
                })
                .count();
            ui.label(crate::i18n::tr(&format!(
                "{count} résultat(s) — tous les dossiers"
            )));
            if count == 0 {
                ui.label(crate::i18n::tr("Aucune session correspondante."));
            }
        }
        ui.small(crate::i18n::tr(
            "Glisser une session sur un dossier pour la déplacer.",
        ));
        let root = ui.label(crate::i18n::tr("Racine (sans dossier)"));
        self.session_drop_target(ui, &root, None);
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_width(ui.available_width())
            .show(ui, |ui| self.folder_tree(ui, None, 0));
        self.tree_open = None;
    }
    fn session_drop_target(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        folder: Option<&str>,
    ) {
        if self.busy || self.editor.is_some() || self.folder_editor.is_some() {
            return;
        }
        if response.dnd_hover_payload::<SessionDrag>().is_some() {
            ui.painter().rect_stroke(
                response.rect.expand(2.0),
                2.0,
                egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Outside,
            );
        }
        if let Some(payload) = response.dnd_release_payload::<SessionDrag>()
            && self
                .snapshot
                .sessions
                .iter()
                .any(|s| s.uuid == payload.0 && s.data.folder_id.as_deref() != folder)
        {
            self.send(Action::MoveSession(
                payload.0.clone(),
                folder.map(str::to_owned),
            ));
        }
    }
    fn folder_tree(&mut self, ui: &mut egui::Ui, parent: Option<&str>, depth: usize) {
        if depth >= 64 {
            return;
        }
        let searching = !self.search.trim().is_empty();
        let folders: Vec<_> = self
            .snapshot
            .folders
            .iter()
            .filter(|f| !searching && f.data.parent_id.as_deref() == parent)
            .cloned()
            .collect();
        let sessions: Vec<_> = self
            .snapshot
            .sessions
            .iter()
            .filter(|s| {
                (searching || s.data.folder_id.as_deref() == parent)
                    && matches_search(s, &self.search, &self.snapshot)
                    && (!self.favorites || s.data.favorite)
            })
            .cloned()
            .collect();
        for s in sessions {
            let response = session_row(
                ui,
                &s.uuid,
                &s.data.name,
                &s.data.protocol,
                self.selected.as_deref() == Some(&s.uuid),
            )
            .on_hover_text(format!(
                "{} — {}:{}\nDouble-cliquer pour connecter",
                s.data.protocol.as_str(),
                s.data.hostname,
                s.data.port
            ));
            if searching {
                let folder = crate::importers::exchange::folder_path(
                    s.data.folder_id.as_deref(),
                    &self.snapshot,
                );
                ui.small(format!(
                    "{} · {}",
                    s.data.endpoint(),
                    if folder.is_empty() { "Racine" } else { &folder }
                ));
            }
            if response.clicked() {
                self.selected = Some(s.uuid.clone());
            }
            if response.double_clicked() {
                self.connect(&s, ui.ctx());
            }
            response.context_menu(|ui| {
                if ui.button(crate::i18n::tr("Connexion")).clicked() {
                    self.connect(&s, ui.ctx());
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Modifier / déplacer")).clicked() {
                    self.editor = Some(SessionEditor::from_session(&s));
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Options SSH / tunnel")).clicked() {
                    self.advanced.options_id = s.uuid.clone();
                    self.advanced.options = self
                        .advanced
                        .preferences
                        .sessions
                        .get(&s.uuid)
                        .cloned()
                        .unwrap_or_default();
                    self.advanced.options_open = true;
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Exporter…")).clicked() {
                    self.export_ids = vec![s.uuid.clone()];
                    self.export_open = true;
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Supprimer…")).clicked() {
                    self.confirm_delete = Some((s.uuid.clone(), false));
                    ui.close();
                }
            });
        }
        for f in folders {
            let header = egui::CollapsingHeader::new(&f.data.name)
                .id_salt(&f.uuid)
                .default_open(false)
                .open(self.tree_open)
                .show(ui, |ui| {
                    if ui
                        .selectable_label(
                            self.folder.as_deref() == Some(&f.uuid),
                            crate::i18n::tr("Afficher les sessions"),
                        )
                        .clicked()
                    {
                        self.folder = Some(f.uuid.clone());
                        self.favorites = false;
                    }
                    self.folder_tree(ui, Some(&f.uuid), depth + 1);
                });
            self.session_drop_target(ui, &header.header_response, Some(&f.uuid));
            header.header_response.context_menu(|ui| {
                if ui.button(crate::i18n::tr("Modifier / déplacer")).clicked() {
                    self.folder_editor = Some(FolderEditor::from_folder(&f));
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Nouveau sous-dossier")).clicked() {
                    self.folder_editor = Some(FolderEditor::new(Some(f.uuid.clone())));
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Supprimer…")).clicked() {
                    self.confirm_delete = Some((f.uuid.clone(), true));
                    ui.close();
                }
            });
        }
    }
}

fn protocol_icon(ui: &egui::Ui, rect: egui::Rect, protocol: &crate::models::Protocol) {
    let p = ui.painter();
    let origin = rect.min;
    let at = |x, y| origin + egui::vec2(x, y);
    let screen = |x, y, w, h| egui::Rect::from_min_size(at(x, y), egui::vec2(w, h));
    if matches!(
        protocol,
        crate::models::Protocol::WEB | crate::models::Protocol::VNC
    ) {
        let color = if *protocol == crate::models::Protocol::WEB {
            Color32::from_rgb(45, 150, 160)
        } else {
            Color32::from_rgb(160, 100, 190)
        };
        p.rect_filled(screen(1.0, 2.0, 22.0, 18.0), 3.0, color);
        p.text(
            at(12.0, 11.0),
            egui::Align2::CENTER_CENTER,
            if *protocol == crate::models::Protocol::WEB {
                "W"
            } else {
                "V"
            },
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
    } else if *protocol == crate::models::Protocol::SSH {
        let green = Color32::from_rgb(47, 139, 78);
        p.rect_filled(screen(1.0, 2.0, 21.0, 17.0), 2.0, green);
        let white = egui::Stroke::new(1.5, Color32::WHITE);
        p.line_segment([at(5.0, 6.0), at(9.0, 10.0)], white);
        p.line_segment([at(9.0, 10.0), at(5.0, 14.0)], white);
        p.line_segment([at(12.0, 14.0), at(18.0, 14.0)], white);
    } else {
        let blue = Color32::from_rgb(44, 124, 189);
        let stroke = egui::Stroke::new(1.5, blue);
        p.rect_stroke(
            screen(1.0, 1.0, 15.0, 11.0),
            1.0,
            stroke,
            egui::StrokeKind::Inside,
        );
        p.rect_filled(screen(7.0, 7.0, 16.0, 12.0), 1.0, blue);
        p.rect_stroke(
            screen(9.0, 9.0, 12.0, 7.0),
            0.0,
            egui::Stroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Inside,
        );
        p.line_segment([at(15.0, 19.0), at(15.0, 21.0)], stroke);
        p.line_segment([at(11.0, 21.0), at(19.0, 21.0)], stroke);
    }
}

// One widget owns clicks AND drags. A drag-only wrapper around clickable children
// steals their mouse events, and its floating layer can obscure drop targets.
fn session_row(
    ui: &mut egui::Ui,
    id: &str,
    name: &str,
    protocol: &crate::models::Protocol,
    selected: bool,
) -> egui::Response {
    ui.push_id(("session_row", id), |ui| {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width().max(40.0), 26.0),
            egui::Sense::click_and_drag(),
        );
        if selected || response.hovered() {
            ui.painter().rect_filled(
                rect,
                2.0,
                if selected {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().widgets.hovered.bg_fill
                },
            );
        }
        protocol_icon(
            ui,
            egui::Rect::from_min_size(rect.min + egui::vec2(2.0, 2.0), egui::vec2(24.0, 22.0)),
            protocol,
        );
        ui.painter()
            .with_clip_rect(rect.intersect(ui.clip_rect()))
            .text(
                rect.left_center() + egui::vec2(32.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("{} · {}", protocol.as_str(), name),
                egui::TextStyle::Body.resolve(ui.style()),
                if selected {
                    ui.visuals().selection.stroke.color
                } else {
                    ui.visuals().text_color()
                },
            );
        response.dnd_set_drag_payload(SessionDrag(id.to_owned()));
        response
    })
    .inner
}

#[cfg(test)]
#[path = "sidebar_tests.rs"]
mod tests;
