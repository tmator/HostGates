use super::*;
use crate::connections::rdp::DisplaySize;

fn label(display: DisplaySize) -> String {
    match display {
        DisplaySize::ClientDefault => crate::i18n::tr("Réglage du client"),
        DisplaySize::Fullscreen => crate::i18n::tr("Plein écran"),
        DisplaySize::Fixed { width, height } => format!("{width} × {height}"),
    }
}

impl HostGates {
    pub(super) fn rdp_display_dialog(&mut self, ctx: &egui::Context) {
        let Some((session, mut display)) = self.advanced.rdp_launch.take() else {
            return;
        };
        let mut open = true;
        let mut connect = false;
        let mut cancel = false;
        egui::Window::new(crate::i18n::tr("Affichage RDP"))
            .id(egui::Id::new("rdp_display_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(&session.data.name);
                ui.label(crate::i18n::tr("Résolution"));
                egui::ComboBox::from_id_salt("rdp_resolution")
                    .selected_text(label(display))
                    .show_ui(ui, |ui| {
                        for choice in [DisplaySize::ClientDefault, DisplaySize::Fullscreen]
                            .into_iter()
                            .chain(
                                DisplaySize::STANDARD
                                    .iter()
                                    .map(|&(width, height)| DisplaySize::Fixed { width, height }),
                            )
                        {
                            ui.selectable_value(&mut display, choice, label(choice));
                        }
                    });
                ui.small(crate::i18n::tr(
                    "Le dernier choix est mémorisé pour cette session.",
                ));
                ui.horizontal(|ui| {
                    connect = ui
                        .add_enabled(!self.busy, egui::Button::new(crate::i18n::tr("Connexion")))
                        .clicked()
                        || (!self.busy && super::shortcuts::submitted(ctx, "rdp_display"));
                    cancel = ui.button(crate::i18n::tr("Annuler")).clicked();
                });
            });
        if !open || cancel {
            return;
        }
        if connect {
            let previous = self
                .advanced
                .preferences
                .sessions
                .get(&session.uuid)
                .cloned();
            self.advanced
                .preferences
                .sessions
                .entry(session.uuid.clone())
                .or_default()
                .rdp_display = display;
            if let Err(e) = self.advanced.save() {
                if let Some(previous) = previous {
                    self.advanced
                        .preferences
                        .sessions
                        .insert(session.uuid.clone(), previous);
                } else {
                    self.advanced.preferences.sessions.remove(&session.uuid);
                }
                self.error = Some(e.to_string());
                self.advanced.rdp_launch = Some((session, display));
            } else {
                self.begin_connect_after_display(&session, ctx);
            }
        } else {
            self.advanced.rdp_launch = Some((session, display));
        }
    }
}

pub(super) fn embedded_resolution_selector(ui: &mut egui::Ui, resolution: &mut Option<(u16, u16)>) {
    egui::ComboBox::from_id_salt("embedded_resolution")
        .selected_text(
            resolution
                .map(|(w, h)| format!("{w} × {h}"))
                .unwrap_or_else(|| "1280 × 720".into()),
        )
        .show_ui(ui, |ui| {
            for &(w, h) in DisplaySize::STANDARD {
                ui.selectable_value(resolution, Some((w, h)), format!("{w} × {h}"));
            }
        });
}
