use super::*;

pub(super) fn submitted(ctx: &egui::Context, target: &str) -> bool {
    ctx.data(|d| d.get_temp::<String>(egui::Id::new("dialog_submit")))
        .as_deref()
        == Some(target)
}
impl HostGates {
    fn dialog_target(&self, ctx: &egui::Context) -> Option<&'static str> {
        // Follow the front window, including when the user brings an older dialog forward.
        if self.error.is_some() {
            return Some("error");
        }
        let focused = ctx.memory(|m| m.focused());
        let settings_target = if [
            Some(egui::Id::new("master_password")),
            Some(egui::Id::new("master_confirmation")),
        ]
        .contains(&focused)
        {
            "vault"
        } else {
            "settings"
        };
        let front = ctx.memory(|m| m.areas().top_layer_id(egui::Order::Middle));
        if let Some(front) = front {
            for (id, target, open) in [
                ("about_hostgates", "about", self.about_open),
                ("help_dialog", "help", self.help_open),
                ("startup_unlock", "unlock", self.unlock_open),
                ("delete_dialog", "delete", self.confirm_delete.is_some()),
                ("session_editor", "editor", self.editor.is_some()),
                ("folder_dialog", "folder", self.folder_editor.is_some()),
                ("import_dialog", "import", self.preview.is_some()),
                ("export_dialog", "export", self.export_open),
                ("rename_dialog", "rename", self.advanced.rename.is_some()),
                ("options_dialog", "options", self.advanced.options_open),
                ("secure_login", "login", self.advanced.login.is_some()),
                ("broadcast_dialog", "broadcast", self.advanced.broadcast),
                ("settings_dialog", settings_target, self.settings),
                ("catalog_dialog", "catalog", self.catalog_open),
            ] {
                if open && front.id == egui::Id::new(id) {
                    return Some(target);
                }
            }
        }
        if self.error.is_some() {
            Some("error")
        } else if self.about_open {
            Some("about")
        } else if self.help_open {
            Some("help")
        } else if self.unlock_open {
            Some("unlock")
        } else if self.confirm_delete.is_some() {
            Some("delete")
        } else if self.editor.is_some() {
            Some("editor")
        } else if self.folder_editor.is_some() {
            Some("folder")
        } else if self.preview.is_some() {
            Some("import")
        } else if self.export_open {
            Some("export")
        } else if self.advanced.rename.is_some() {
            Some("rename")
        } else if self.advanced.options_open {
            Some("options")
        } else if self.advanced.login.is_some() {
            Some("login")
        } else if self.advanced.broadcast {
            Some("broadcast")
        } else if self.settings {
            let focused = ctx.memory(|m| m.focused());
            if focused == Some(egui::Id::new("master_password"))
                || focused == Some(egui::Id::new("master_confirmation"))
            {
                Some("vault")
            } else {
                Some("settings")
            }
        } else if self.catalog_open {
            Some("catalog")
        } else {
            None
        }
    }
    pub(super) fn dialog_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.remove::<String>(egui::Id::new("dialog_submit")));
        if egui::Popup::is_any_open(ctx) {
            return;
        }
        let Some(target) = self.dialog_target(ctx) else {
            return;
        };
        let escape = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let multiline = ctx.memory(|m| m.focused()).is_some_and(|id| {
            [
                egui::Id::new("session_description"),
                egui::Id::new("multi_command"),
            ]
            .contains(&id)
        });
        let enter = !escape
            && ((!multiline
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)))
                || ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Enter)));
        if escape || (enter && ["error", "about", "help", "catalog"].contains(&target)) {
            match target {
                "error" => self.error = None,
                "about" => self.about_open = false,
                "help" => self.help_open = false,
                "unlock" => {
                    self.unlock_open = false;
                    self.unlock_password = zeroize::Zeroizing::new(String::new());
                }
                "delete" => self.confirm_delete = None,
                "editor" => self.editor = None,
                "folder" => self.folder_editor = None,
                "import" => self.preview = None,
                "export" => self.export_open = false,
                "rename" => self.advanced.rename = None,
                "options" => {
                    self.advanced.options_open = false;
                    self.advanced.gateway_draft = None;
                }
                "login" => self.advanced.cancel_connection(),
                "broadcast" => {
                    self.advanced.broadcast = false;
                    self.advanced.confirm_send = false;
                }
                "settings" | "vault" => {
                    self.settings = false;
                    self.programs_draft = None;
                    self.advanced.master = zeroize::Zeroizing::new(String::new());
                    self.advanced.confirmation = zeroize::Zeroizing::new(String::new());
                }
                "catalog" => self.catalog_open = false,
                _ => {}
            }
        } else if enter && !self.busy {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("dialog_submit"), target.to_string()));
        }
    }
    pub(super) fn unlock_window(&mut self, ctx: &egui::Context) {
        if !self.unlock_open {
            return;
        }
        let mut open = true;
        let mut unlock = false;
        egui::Window::new(crate::i18n::tr("Déverrouiller le coffre"))
            .id(egui::Id::new("startup_unlock")).open(&mut open).collapsible(false).resizable(false)
            .show(ctx, |ui| {
                ui.label(crate::i18n::tr("Saisissez votre mot de passe maître pour utiliser les identifiants enregistrés."));
                ui.add_enabled_ui(!self.busy, |ui| {
                    let r = ui.add(egui::TextEdit::singleline(&mut *self.unlock_password).id(egui::Id::new("startup_password")).password(true));
                    if !ctx.memory(|m| m.has_focus(r.id)) && !ctx.egui_wants_keyboard_input() { r.request_focus(); }
                    unlock = ui.add_enabled(!self.unlock_password.is_empty(), egui::Button::new(crate::i18n::tr("Déverrouiller"))).clicked()
                        || (submitted(ctx,"unlock") && !self.unlock_password.is_empty());
                    ui.small(crate::i18n::tr("Échap ferme cette fenêtre et laisse le coffre verrouillé."));
                });
                if self.busy { ui.spinner(); }
            });
        if !open {
            self.unlock_password = zeroize::Zeroizing::new(String::new());
        }
        self.unlock_open = open;
        if unlock && !self.busy {
            let secret = std::mem::replace(
                &mut self.unlock_password,
                zeroize::Zeroizing::new(String::new()),
            );
            self.send(Action::UnlockVault(secret));
        }
    }
}
