use super::*;
impl HostGates {
    pub(super) fn content(&mut self, ui: &mut egui::Ui) {
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.heading(crate::i18n::tr(if self.favorites {
                "Favoris"
            } else {
                "Mes connexions"
            }));
            ui.label(RichText::new(format!("{} sessions", self.snapshot.sessions.len())).weak());
        });
        ui.label(
            RichText::new(crate::i18n::tr(
                "Double-cliquez une session pour ouvrir une connexion.",
            ))
            .weak(),
        );
        ui.add_space(18.0);
        let sessions: Vec<_> = self
            .snapshot
            .sessions
            .iter()
            .filter(|s| {
                (!self.favorites || s.data.favorite)
                    && (self.folder.is_none() || s.data.folder_id == self.folder)
                    && matches_search(s, &self.search, &self.snapshot)
            })
            .cloned()
            .collect();
        if sessions.is_empty() {
            ui.group(|ui| {
                ui.set_min_height(130.0);
                ui.heading(crate::i18n::tr(if self.snapshot.sessions.is_empty() {
                    "Bienvenue dans HostGates"
                } else {
                    "Aucune session correspondante"
                }));
                ui.label(crate::i18n::tr(
                    "Ajoutez une connexion SSH ou RDP pour organiser vos serveurs.",
                ));
                if ui
                    .button(crate::i18n::tr("Importer mes sessions MobaXterm…"))
                    .clicked()
                {
                    self.choose_import();
                }
                if ui.button(crate::i18n::tr("+ Créer une session")).clicked() {
                    self.new_session();
                }
            });
        }
        egui::ScrollArea::vertical()
            .id_salt("sessions")
            .max_height(260.0)
            .show(ui, |ui| {
                egui::Grid::new("session_table")
                    .striped(true)
                    .min_col_width(110.0)
                    .show(ui, |ui| {
                        ui.strong(crate::i18n::tr("Nom"));
                        ui.strong(crate::i18n::tr("Protocole"));
                        ui.strong(crate::i18n::tr("Adresse"));
                        ui.strong(crate::i18n::tr("Utilisateur"));
                        ui.end_row();
                        for s in &sessions {
                            let label = format!(
                                "{}{}",
                                if s.data.favorite { "★ " } else { "" },
                                s.data.name
                            );
                            let row = ui
                                .selectable_label(self.selected.as_deref() == Some(&s.uuid), label);
                            if row.clicked() {
                                self.selected = Some(s.uuid.clone());
                            }
                            if row.double_clicked() {
                                self.connect(s, ui.ctx());
                            }
                            row.context_menu(|ui| {
                                if ui
                                    .button(crate::i18n::tr("Connexion / nouvel onglet"))
                                    .clicked()
                                {
                                    self.connect(s, ui.ctx());
                                    ui.close();
                                }
                                if ui.button(crate::i18n::tr("Modifier")).clicked() {
                                    self.editor = Some(SessionEditor::from_session(s));
                                    ui.close();
                                }
                                if ui
                                    .button(crate::i18n::tr("Dupliquer sans mot de passe"))
                                    .clicked()
                                {
                                    let mut data = s.data.clone();
                                    data.name = format!("{} (copie)", data.name);
                                    self.editor = Some(SessionEditor::from_data(data));
                                    ui.close();
                                }
                                if ui
                                    .button(crate::i18n::tr(if s.data.favorite {
                                        "Retirer des favoris"
                                    } else {
                                        "Ajouter aux favoris"
                                    }))
                                    .clicked()
                                {
                                    let mut data = s.data.clone();
                                    data.favorite = !data.favorite;
                                    self.send(Action::SaveSession(Some(s.uuid.clone()), data));
                                    ui.close();
                                }
                                if ui.button(crate::i18n::tr("Supprimer…")).clicked() {
                                    self.confirm_delete = Some((s.uuid.clone(), false));
                                    ui.close();
                                }
                            });
                            ui.label(RichText::new(s.data.protocol.as_str()).color(
                                if s.data.protocol == crate::models::Protocol::SSH {
                                    Color32::from_rgb(96, 198, 180)
                                } else {
                                    Color32::from_rgb(145, 165, 255)
                                },
                            ));
                            ui.monospace(s.data.endpoint());
                            ui.label(&s.data.username);
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(12.0);
        if let Some(s) = self.selected_session() {
            self.details(ui, &s);
        } else {
            ui.label(
                RichText::new(crate::i18n::tr(
                    "Sélectionnez une session pour afficher ses propriétés.",
                ))
                .weak(),
            );
        }
    }
    fn details(&mut self, ui: &mut egui::Ui, s: &Session) {
        ui.horizontal(|ui| {
            ui.heading(&s.data.name);
            ui.label(RichText::new(crate::i18n::tr("● Hors connexion")).weak());
        });
        ui.label(&s.data.description);
        ui.horizontal(|ui| {
            for t in &s.data.tags {
                ui.label(RichText::new(format!("#{t}")).color(Color32::from_rgb(135, 164, 245)));
            }
        });
        ui.add_space(8.0);
        egui::Grid::new("details").num_columns(2).show(ui, |ui| {
            ui.label(crate::i18n::tr("Serveur"));
            ui.monospace(s.data.endpoint());
            ui.end_row();
            ui.label(crate::i18n::tr("Utilisateur"));
            ui.label(&s.data.username);
            ui.end_row();
            ui.label(crate::i18n::tr("Domaine"));
            ui.label(crate::i18n::tr(if s.data.domain.is_empty() {
                "—"
            } else {
                &s.data.domain
            }));
            ui.end_row();
            ui.label(crate::i18n::tr("Coffre système"));
            ui.label(crate::i18n::tr(if s.credential_id.is_some() {
                "Référence de mot de passe enregistrée"
            } else {
                "Aucun mot de passe enregistré"
            }));
            ui.end_row();
            ui.label(crate::i18n::tr("Créée le"));
            ui.label(&s.created_at);
            ui.end_row();
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button(crate::i18n::tr("▶ Connexion")).clicked() {
                self.connect(s, ui.ctx());
            }
            if ui.button(crate::i18n::tr("Modifier")).clicked() {
                self.editor = Some(SessionEditor::from_session(s));
            }
            if ui.button(crate::i18n::tr("Mot de passe…")).clicked() {
                let mut ed = SessionEditor::from_session(s);
                ed.secret_open = true;
                self.editor = Some(ed);
            }
            if ui.button(crate::i18n::tr("Supprimer…")).clicked() {
                self.confirm_delete = Some((s.uuid.clone(), false));
            }
        });
        ui.add_space(12.0);
        ui.group(|ui| {
            ui.label(crate::i18n::tr("Connexion sécurisée via les clients système"));
            ui.label(
                RichText::new(
                    crate::i18n::tr("SSH : terminal intégré et SFTP. RDP/VNC : fenêtre externe avec authentification propre."),
                )
                .weak(),
            );
        });
    }
}
