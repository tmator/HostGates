use super::*;
use crate::importers::{DuplicatePolicy, exchange::ExportFormat};
impl HostGates {
    pub(super) fn choose_import(&mut self) {
        if self.busy {
            return;
        }
        if let Some(file) = rfd::FileDialog::new()
            .set_title(crate::i18n::tr("Importer des sessions MobaXterm"))
            .add_filter("Sessions MobaXterm", &["mxtsessions", "moba", "ini", "csv"])
            .pick_file()
        {
            self.send(Action::PreviewImport(file));
        }
    }
    pub(super) fn exchange_windows(&mut self, ctx: &egui::Context) {
        if let Some(mut preview) = self.preview.take() {
            let mut open = true;
            let mut import = false;
            egui::Window::new(crate::i18n::tr("Importer des sessions — aperçu")).id(egui::Id::new("import_dialog")).open(&mut open).default_width(960.0).default_height(500.0).resizable(true).show(ctx,|ui|{
                ui.label(crate::i18n::tr("Aucun mot de passe n'est importé. Les clés et paramètres avancés inconnus sont ignorés."));
                for warning in &preview.warnings{ui.colored_label(Color32::from_rgb(190,125,35),warning);}
                ui.horizontal(|ui|{
                    if ui.button(crate::i18n::tr("Tout sélectionner")).clicked(){for r in &mut preview.rows{r.selected=r.data.is_some();}}
                    if ui.button(crate::i18n::tr("Tout désélectionner")).clicked(){for r in &mut preview.rows{r.selected=false;}}
                    ui.separator();ui.label(crate::i18n::tr("Doublons, appliquer à tous :"));
                    for (policy,label) in [(DuplicatePolicy::Skip,"Ignorer"),(DuplicatePolicy::Import,"Importer quand même"),(DuplicatePolicy::Replace,"Remplacer")]{if ui.button(crate::i18n::tr(label)).clicked(){for row in &mut preview.rows{row.policy=policy;}}}
                });
                egui::ScrollArea::both().max_height(340.0).show(ui,|ui|{egui::Grid::new("import_preview").striped(true).show(ui,|ui|{
                    for h in ["Choix","Nom","Dossier","Type","Serveur","Port","Utilisateur","État / doublon"]{ui.strong(crate::i18n::tr(h));}ui.end_row();
                    for (i,row) in preview.rows.iter_mut().enumerate(){
                        ui.add_enabled(row.data.is_some(),egui::Checkbox::without_text(&mut row.selected));ui.label(&row.name);ui.label(crate::i18n::tr(if row.folder.is_empty(){"Racine"}else{&row.folder}));ui.label(&row.kind);
                        ui.label(row.data.as_ref().map_or("—",|d|&d.hostname));ui.label(row.data.as_ref().map_or(String::from("—"),|d|d.port.to_string()));ui.label(row.data.as_ref().map_or("—",|d|&d.username));
                        if row.duplicate{egui::ComboBox::from_id_salt(("duplicate",i)).selected_text(crate::i18n::tr(policy_label(row.policy))).show_ui(ui,|ui|{for p in [DuplicatePolicy::Skip,DuplicatePolicy::Import,DuplicatePolicy::Replace]{ui.selectable_value(&mut row.policy,p,crate::i18n::tr(policy_label(p)));}});}else{ui.label(row.issue.as_deref().unwrap_or("OK"));}ui.end_row();
                    }
                });});
                let count=preview.rows.iter().filter(|r|r.selected&&r.data.is_some()).count();
                ui.label(crate::i18n::tr(&format!("{count} sélectionnée(s) sur {}. Remplacer met à jour les métadonnées et conserve le secret existant.",preview.rows.len())));
                if ui.add_enabled(!self.busy&&count>0,egui::Button::new(crate::i18n::tr("Importer les sessions sélectionnées"))).clicked() || (!self.busy && count>0 && super::shortcuts::submitted(ctx,"import")){import=true;}
            });
            if import {
                self.send(Action::Import(preview.clone()));
                self.importing = self.busy;
                self.preview = Some(preview);
            } else if open {
                self.preview = Some(preview);
            }
        }
        if self.export_open {
            let mut open = true;
            let mut save = false;
            egui::Window::new(crate::i18n::tr("Exporter des sessions")).id(egui::Id::new("export_dialog")).open(&mut open).default_width(540.0).show(ctx,|ui|{
                ui.label(crate::i18n::tr("Export sans mots de passe, passphrases ni références au coffre."));
                ui.radio_value(&mut self.export_format,ExportFormat::Csv,crate::i18n::tr("CSV MobaXterm — format d'échange documenté (recommandé)"));
                ui.radio_value(&mut self.export_format,ExportFormat::Mxt,crate::i18n::tr(".mxtsessions — champs de connexion de base"));
                if self.export_format==ExportFormat::Mxt{ui.label(crate::i18n::tr("Le format propriétaire complet n'est pas documenté. Les paramètres avancés ne sont pas exportés ; utiliser CSV si votre version refuse ce fichier."));}
                ui.horizontal(|ui|{if ui.button(crate::i18n::tr("Tout")).clicked(){self.export_ids=self.snapshot.sessions.iter().map(|s|s.uuid.clone()).collect();}
if ui.button(crate::i18n::tr("Aucun")).clicked(){self.export_ids.clear();}});
                egui::ScrollArea::vertical().max_height(240.0).show(ui,|ui|{for s in &self.snapshot.sessions{let mut yes=self.export_ids.contains(&s.uuid);if ui.checkbox(&mut yes,format!("{}  •  {}",s.data.name,s.data.protocol.as_str())).changed(){if yes{self.export_ids.push(s.uuid.clone())}else{self.export_ids.retain(|id|id!=&s.uuid);}}}});
                if ui.add_enabled(!self.busy&&!self.export_ids.is_empty(),egui::Button::new(crate::i18n::tr("Choisir le fichier et exporter…"))).clicked() || (!self.busy && !self.export_ids.is_empty() && super::shortcuts::submitted(ctx,"export")){save=true;}
            });
            self.export_open = open;
            if save {
                let ext = if self.export_format == ExportFormat::Csv {
                    "csv"
                } else {
                    "mxtsessions"
                };
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(format!("HostGates Sessions.{ext}"))
                    .add_filter("Sessions", &[ext])
                    .save_file()
                {
                    self.send(Action::Export(
                        path,
                        self.export_ids.clone(),
                        self.export_format,
                    ));
                    self.export_open = false;
                }
            }
        }
    }
    pub(super) fn begin_export(&mut self) {
        self.export_ids = self
            .snapshot
            .sessions
            .iter()
            .map(|s| s.uuid.clone())
            .collect();
        self.export_open = true;
    }
}
fn policy_label(p: DuplicatePolicy) -> &'static str {
    match p {
        DuplicatePolicy::Skip => "Doublon : ignorer",
        DuplicatePolicy::Import => "Doublon : importer",
        DuplicatePolicy::Replace => "Doublon : remplacer",
    }
}
