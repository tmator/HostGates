use super::*;
impl HostGates {
    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some((id, folder)) = self.confirm_delete.clone() {
            egui::Window::new(crate::i18n::tr("Confirmer la suppression"))
                .id(egui::Id::new("delete_dialog"))
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(crate::i18n::tr(if folder {
                        "Supprimer ce dossier ? Il doit être vide."
                    } else {
                        "Supprimer cette session et son mot de passe enregistré ?"
                    }));
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.busy,
                                egui::Button::new(crate::i18n::tr("Supprimer")),
                            )
                            .clicked()
                            || (!self.busy && super::shortcuts::submitted(ctx, "delete"))
                        {
                            self.send(if folder {
                                Action::DeleteFolder(id)
                            } else {
                                Action::DeleteSession(id)
                            });
                            self.confirm_delete = None;
                        }
                        if ui.button(crate::i18n::tr("Annuler")).clicked() {
                            self.confirm_delete = None;
                        }
                    });
                });
        }
        if !self.settings {
            self.advanced.master = zeroize::Zeroizing::new(String::new());
            self.advanced.confirmation = zeroize::Zeroizing::new(String::new());
            self.programs_draft = None;
            self.programs_check = None;
        }
        if self.settings {
            let mut programs = self
                .programs_draft
                .take()
                .unwrap_or_else(|| self.snapshot.programs.clone());
            let mut save_programs = false;
            let mut open = true;
            egui::Window::new(crate::i18n::tr("Paramètres")).id(egui::Id::new("settings_dialog")).open(&mut open).default_width(700.0).show(ctx,|ui| {
                egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| {
                ui.heading(crate::i18n::tr("Langue"));
                let previous=self.advanced.preferences.language.clone();
                egui::ComboBox::from_id_salt("locale").selected_text(crate::i18n::language_name(&previous)).show_ui(ui,|ui|{
                    for lang in crate::i18n::languages(){ui.selectable_value(&mut self.advanced.preferences.language,lang.clone(),crate::i18n::language_name(&lang));}
                });
                if previous!=self.advanced.preferences.language && let Err(e)=self.advanced.save(){self.error=Some(e.to_string());}
                self.vault_settings(ui);ui.separator();
                ui.heading(crate::i18n::tr("Applications externes"));
                ui.label(crate::i18n::tr("Champ vide : recherche automatique dans PATH. Sinon, chemin absolu de l’exécutable, sans guillemets ni arguments."));
                ui.add_enabled_ui(!self.busy, |ui| {
                    for (label, value) in [("SSH (OpenSSH)", &mut programs.ssh), (if cfg!(windows) { "RDP (mstsc)" } else { "RDP (FreeRDP)" }, &mut programs.rdp), ("VNC (TigerVNC ou compatible hôte::port)", &mut programs.vnc), ("Terminal local", &mut programs.terminal)] {
                        ui.label(crate::i18n::tr(label));
                        ui.horizontal(|ui| {
                            if ui.add(egui::TextEdit::singleline(value).desired_width(460.0).hint_text(crate::i18n::tr("Automatique (PATH)"))).changed() { self.programs_check = None; }
                            if ui.button(crate::i18n::tr("Parcourir…")).clicked() {
                                let picker = rfd::FileDialog::new().set_title(crate::i18n::tr("Choisir l’exécutable"));
                                #[cfg(windows)]
                                let picker = picker.add_filter("Exécutable", &["exe"]);
                                if let Some(path) = picker.pick_file() {
                                    *value = path.to_string_lossy().into_owned(); self.programs_check = None;
                                }
                            }
                            if ui.button(crate::i18n::tr("Auto")).on_hover_text(crate::i18n::tr("Revenir à la recherche dans PATH")).clicked() { value.clear(); self.programs_check = None; }
                        });
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(crate::i18n::tr("Vérifier les chemins")).clicked() {
                            use crate::connections::{resolve_program, ProgramKind};
                            self.programs_check = Some([(&programs.ssh, ProgramKind::Ssh), (&programs.rdp, ProgramKind::Rdp), (&programs.vnc, ProgramKind::Vnc), (&programs.terminal, ProgramKind::Terminal)].into_iter().map(|(value,kind)| match resolve_program(value,kind) {
                                Ok(path) => format!("{} : {}", kind.label(), path.display()), Err(e) => e.to_string(),
                            }).collect::<Vec<_>>().join("\n"));
                        }
                        if !self.busy && (ui.button(crate::i18n::tr("Enregistrer les chemins")).clicked() || super::shortcuts::submitted(ctx,"settings")) { save_programs = true; }
                        if ui.button(crate::i18n::tr("Annuler les modifications")).clicked() { programs = self.snapshot.programs.clone(); self.programs_check = None; }
                        if ui.button(crate::i18n::tr("Aide à la configuration")).clicked() { self.help_open = true; self.help_topic = super::help::TOPICS.len()-1; }
                    });
                });
                if let Some(report) = &self.programs_check { ui.label(report); }
                ui.small(crate::i18n::tr("La vérification contrôle les fichiers, sans les exécuter. Enregistrez pour appliquer aux prochaines connexions."));
                ui.separator();
                ui.heading(crate::i18n::tr("Apparence"));
                ui.horizontal(|ui| {for (id,label) in [("system","Système"),("dark","Sombre"),("light","Clair")] { if ui.add_enabled(!self.busy,egui::Button::new(crate::i18n::tr(label)).selected(self.snapshot.theme==id)).clicked(){self.send(Action::Theme(id.into()));} }});
                ui.separator();ui.heading(crate::i18n::tr("Données locales"));ui.label(&self.path);
                ui.label(crate::i18n::tr("SQLite contient les sessions, les dossiers et les préférences. Aucun mot de passe."));
                ui.separator();ui.heading(crate::i18n::tr("Coffre système"));ui.label(crate::i18n::tr(if self.vault_ready { "Fournisseur natif initialisé. Une opération peut nécessiter le déverrouillage du coffre." } else { "Indisponible. Les sessions restent utilisables sans enregistrement du mot de passe." }));
                ui.separator();ui.label(crate::i18n::tr("Ctrl+N : nouvelle session • Double-clic : connecter • Clic droit : actions"));
                ui.label(crate::i18n::tr("F1 : aide complète. Le SSH intégré utilise le coffre ; les clients externes gardent leur authentification."));
                });
            });
            if save_programs {
                self.send(Action::Programs(programs.clone()));
            }
            self.programs_draft = if open { Some(programs) } else { None };
            self.settings = open;
        }
    }
}
