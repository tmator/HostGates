use super::*;
pub const TOPICS: &[(&str, &str, &str)] = &[
    (
        "Clavier, coffre et passerelles",
        "Si un mot de passe maître est configuré, le déverrouillage est proposé au démarrage. Entrée déclenche l'action principale de la fenêtre active ; Échap ferme la fenêtre et abandonne son brouillon. Dans les champs multilignes, Entrée ajoute une ligne et Ctrl+Entrée valide. Les confirmations et validations restent obligatoires. Options SSH / tunnel > Saisir une nouvelle passerelle SSH permet de fournir adresse, port, utilisateur et mot de passe. Enregistrer crée une session SSH réutilisable, stocke le secret dans le coffre et associe cette passerelle à la cible. Un mot de passe vide utilise l'agent SSH. Vérifiez la clé de la passerelle à la première connexion : Entrée ne l'accepte jamais automatiquement.",
        "If a master password is configured, unlocking is offered at startup. Enter triggers the active window's default action; Escape closes it and discards its draft. In multiline fields, Enter adds a line and Ctrl+Enter submits. Validation and confirmations still apply. SSH / tunnel options > Enter a new SSH gateway lets you provide host, port, username and password. Save creates a reusable SSH session, stores its secret in the vault and assigns the gateway to the target. An empty password uses the SSH agent. Verify the gateway host key on first connection: Enter never trusts it automatically.",
    ),
    (
        "Mots de passe et connexion",
        "Dans Nouvelle session ou Modifier, cochez Enregistrer / remplacer le mot de passe dans le coffre. Déverrouillez d'abord le coffre. SSH intégré utilise ce secret après vérification de la clé hôte ; l'erreur Session(-5) concerne la négociation, avant le mot de passe. MSTSC reçoit un fichier RDP protégé par DPAPI ; FreeRDP reçoit le mot de passe via son entrée standard. TigerVNC utilise PasswordFile (VncAuth classique, 8 caractères ASCII maximum). Les politiques serveur peuvent imposer une nouvelle saisie. Dans Options SSH / tunnel, activez Saisir le mot de passe dans le client externe pour un client incompatible. OpenSSH externe reste interactif. Web conserve le secret dans le coffre mais le navigateur gère l'authentification du site. Aucun mot de passe n'est ajouté à la ligne de commande ni à l'URL. Les fichiers temporaires VNC sont seulement obfusqués et protégés par les droits du compte ; voir securite.txt.",
        "In New session or Edit, select Save / replace password in the vault. Unlock the vault first. Built-in SSH uses the secret after host key verification; Session(-5) is a negotiation failure before password authentication. MSTSC receives a DPAPI-protected RDP file; FreeRDP receives the password through standard input. TigerVNC uses PasswordFile (classic VncAuth, up to 8 ASCII characters). Server policy may still require another prompt. In SSH / tunnel options, enable manual external authentication for incompatible clients. External OpenSSH remains interactive. Web can store the secret in the vault, but the browser handles website sign-in. No password is added to the command line or URL. Temporary VNC files are only obfuscated and protected by account permissions; see securite.txt.",
    ),
    (
        "Bien démarrer",
        "Créez ou importez une session, puis double-cliquez pour vous connecter. Les dossiers sont fermés au démarrage. La recherche par nom ou IP traverse les dossiers. Glissez une session sur un dossier pour la déplacer. Pour créer un dossier pendant la création d'une session : + Nouveau dossier, puis Créer et sélectionner.",
        "Create or import a session, then double-click to connect. Folders start collapsed. Name/IP search includes all folders. Drag a session onto a folder to move it. In the session editor, use + New folder, then Create and select.",
    ),
    (
        "SSH et fichiers",
        "SSH intégré utilise le mot de passe saisi, le coffre, une clé privée ou l'agent SSH. Vérifiez les nouvelles empreintes auprès de l'administrateur. Une clé modifiée est bloquée. Le bouton SFTP ouvre le volet droit. Double-clic sur un dossier pour entrer ; ↑ pour remonter ; Ctrl+clic pour sélectionner plusieurs éléments. Envoyer propose fichiers ou dossier. Télécharger copie la sélection dans un dossier local, récursivement. Aucun écrasement ni suivi de lien symbolique. Une erreur ou une interruption peut laisser une copie partielle : retirez-la avant de réessayer. SFTP nécessite que le serveur autorise ce sous-système et une connexion SSH supplémentaire. Le mode OpenSSH externe reste disponible dans Options SSH / tunnel, sans explorateur intégré.",
        "Integrated SSH uses a password, the vault, a private key or the SSH agent. Verify new server fingerprints independently. Changed keys are blocked. SFTP opens the right panel. Double-click a folder to enter, use ↑ to go up, Ctrl+click to select multiple items. Upload accepts files or a folder. Download recursively copies the selection into a local folder. Existing items and symbolic links are rejected. An error or interruption may leave partial files: remove these before retrying. SFTP requires server support and another SSH connection. External OpenSSH is available in SSH / tunnel options without the integrated file browser.",
    ),
    (
        "Coffre et sécurité",
        "Paramètres > Coffre et mot de passe maître permet de créer le coffre chiffré et de migrer les secrets existants. Minimum 12 caractères ; utilisez une longue phrase unique. Le coffre est verrouillé au démarrage. Déverrouillez-le dans Paramètres. Mot de passe… dans une session enregistre ou supprime son secret ; la connexion SSH propose aussi Enregistrer dans le coffre. Le verrouillage manuel ferme les connexions. Après 10 minutes sans interaction, le coffre maître se verrouille et ferme les connexions. Ce délai n'est pas un verrouillage OS. Sauvegardez vault.json, vault.enabled, remotehub.db et preferences.json application fermée. Le mot de passe maître perdu n'est pas récupérable. Voir securite.txt pour les algorithmes et limites.",
        "Settings > Vault and master password creates the encrypted vault and migrates existing secrets. Minimum 12 characters; use a long unique passphrase. The vault starts locked. Unlock it in Settings. Password… on a session saves/deletes its secret; the SSH login also offers Save in vault. Manual locking closes connections. After 10 minutes without interaction, the master vault locks and closes connections. This is not OS lock detection. Back up vault.json, vault.enabled, remotehub.db and preferences.json with the app closed. A lost master password cannot be recovered. See securite.txt for algorithms and limitations.",
    ),
    (
        "Tunnels SSH",
        "Créez une session SSH pour la passerelle, connectez-la pour vérifier son empreinte et enregistrez son mot de passe ou configurez sa clé/son agent. Sur la cible : clic droit > Options SSH / tunnel > choisissez la passerelle. Un seul saut est pris en charge. SSH, RDP, VNC et Web utilisent un port local aléatoire lié uniquement à 127.0.0.1. La passerelle doit autoriser le transfert TCP. Gardez l'onglet ouvert. Web utilise une URL locale : les certificats HTTPS et les sites exigeant leur nom DNS peuvent être incompatibles ; aucune vérification TLS n'est désactivée. Les paramètres de tunnels propriétaires MobaXterm ne sont pas importés.",
        "Create an SSH gateway session, connect once to verify its fingerprint, and save its password or configure a key/agent. Right-click the target > SSH / tunnel options > select the gateway. One hop is supported. SSH, RDP, VNC and Web use a random port bound only to 127.0.0.1. The gateway must allow TCP forwarding. Keep the tab open. Web uses a local URL: HTTPS certificates and sites requiring their DNS hostname may be incompatible; TLS validation is never disabled. Proprietary MobaXterm tunnel settings are not imported.",
    ),
    (
        "Onglets et commandes multiples",
        "Volets visibles affiche 1 à 6 connexions ; les autres restent ouvertes. Chaque volet et chaque onglet a un menu ▾ pour sélectionner une connexion ouverte. Renommer l'onglet change son titre pendant cette exécution. Cliquez dans le terminal pour lui donner le clavier. Exécution multiple permet de sélectionner les terminaux, saisir une ou plusieurs lignes, vérifier les destinataires, puis exécuter. Chaque ligne est envoyée avec Entrée ; les résultats restent dans chaque terminal. Cela ne détecte pas les invites : ne sélectionnez pas un terminal attendant un mot de passe ou une confirmation.",
        "Visible panes displays 1–6 connections; other tabs stay open. Each pane and tab has a ▾ menu to select an open connection. Rename tab changes its title for this run. Click inside a terminal to focus keyboard input. Multi-execution lets you select terminals, enter one or more lines, check recipients, then execute. Each line is sent with Enter; output stays in each terminal. Prompts are not detected: do not select a terminal waiting for a password or confirmation.",
    ),
    (
        "RDP, VNC et Web",
        "RDP lance mstsc sous Windows ou FreeRDP sous Linux. VNC utilise un client compatible TigerVNC hôte::port. Configurez leurs chemins dans Paramètres > Applications externes si nécessaire. Ils gardent leur fenêtre et leur authentification : HostGates n'injecte pas de mot de passe dans leurs arguments. Web ouvre http(s) dans le navigateur par défaut. Le coffre peut conserver un secret RDP/VNC, mais ne remplit pas ces clients externes.",
        "RDP launches mstsc on Windows or FreeRDP on Linux. VNC uses a TigerVNC-compatible host::port client. Configure paths in Settings > External applications if needed. These clients keep their own window and authentication: HostGates never passes passwords in arguments. Web opens HTTP(S) in the default browser. RDP/VNC passwords can be stored in the vault but are not automatically filled into external clients.",
    ),
    (
        "Import et export",
        "Fichier > Importer MobaXterm : formats .mxtsessions, .moba, .ini et CSV. Modes SSH 0, RDP 4, VNC 5, Web 11. Vérifiez l'aperçu et les doublons avant d'importer. Les mots de passe MobaXterm ne sont pas déchiffrés. Export CSV recommandé ; .mxtsessions limité aux champs connus. Aucun secret ni configuration de passerelle HostGates n'est exporté. Un export n'est pas une sauvegarde complète.",
        "File > Import MobaXterm supports .mxtsessions, .moba, .ini and CSV. SSH mode 0, RDP 4, VNC 5, Web 11. Review the preview and duplicates before importing. MobaXterm passwords are not decrypted. CSV export is recommended; .mxtsessions covers known fields only. Secrets and HostGates gateway settings are never exported. An export is not a complete backup.",
    ),
    (
        "Langues et raccourcis",
        "Paramètres > Langue permet de choisir parmi 12 langues : français, anglais, espagnol, italien, allemand, portugais du Brésil, japonais, chinois simplifié, coréen, arabe, hindi et indonésien. Les traductions et polices sont intégrées. Pour ajouter une langue, copiez locales/en.json, traduisez uniquement les valeurs et placez le fichier dans le dossier locales à côté de l’exécutable ou dans le dossier de données. Redémarrez puis sélectionnez la langue. Les clés manquantes restent en français. Ctrl+N : nouvelle session ; Ctrl+F : recherche ; F1 : aide.",
        "Settings > Language offers 12 languages: French, English, Spanish, Italian, German, Brazilian Portuguese, Japanese, Simplified Chinese, Korean, Arabic, Hindi and Indonesian. Translations and fonts are bundled. To add a language, copy locales/en.json, translate only the values and place the file in the locales folder next to the executable or inside the data directory. Restart and select the language. Missing keys fall back to French. Ctrl+N: new session; Ctrl+F: search; F1: help.",
    ),
    (
        "Configurer les connexions",
        "SSH intégré ne nécessite pas ssh.exe, mais un serveur SSH accessible et un utilisateur valide. Authentification par mot de passe, clé privée ou agent compatible libssh2 ; certaines clés/agents Windows peuvent nécessiter OpenSSH externe. SFTP et tunnels suivent la même politique de clé serveur. Pour RDP/VNC/terminal local, configurez l'exécutable dans Paramètres > Applications externes, vérifiez puis enregistrez. En cas d'échec : vérifiez VPN, pare-feu, port, compte, permissions SFTP et autorisation de forwarding. La version Linux doit être compilée avec OpenSSL et pkg-config installés. Aucune certification de sécurité ; validation Linux réelle encore nécessaire.",
        "Integrated SSH needs no ssh.exe, but requires a reachable SSH server and a valid username. Password, private key and libssh2-compatible agent authentication are supported; some Windows keys/agents may require external OpenSSH. SFTP and tunnels use the same host-key policy. Configure RDP/VNC/local terminal executables in Settings > External applications, check and save. On failure, check VPN, firewall, port, account, SFTP permissions and forwarding authorization. Linux builds require OpenSSL and pkg-config development packages. No security certification; testing on a real Linux desktop is still required.",
    ),
];
impl HostGates {
    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        if !self.help_open {
            return;
        }
        let mut open = true;
        egui::Window::new(crate::i18n::tr("Aide HostGates"))
            .id(egui::Id::new("help_dialog"))
            .open(&mut open)
            .default_size([850.0, 540.0])
            .show(ctx, |ui| {
                ui.columns(2, |cols| {
                    cols[0].set_max_width(240.0);
                    for (i, (title, _, _)) in TOPICS.iter().enumerate() {
                        if cols[0]
                            .selectable_label(self.help_topic == i, crate::i18n::tr(title))
                            .clicked()
                        {
                            self.help_topic = i;
                        }
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("help_content")
                        .show(&mut cols[1], |ui| {
                            let topic = &TOPICS[self.help_topic.min(TOPICS.len() - 1)];
                            ui.label(if self.advanced.preferences.language == "en" {
                                topic.2.to_owned()
                            } else {
                                crate::i18n::tr(topic.1)
                            });
                        });
                });
            });
        self.help_open = open;
    }
}

impl HostGates {
    pub(super) fn about_window(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new(crate::i18n::tr("À propos de HostGates"))
            .id(egui::Id::new("about_hostgates")).open(&mut open)
            .collapsible(false).resizable(false).default_width(380.0)
            .show(ctx, |ui| {
                ui.heading(crate::branding::APP_NAME);
                ui.label(format!("Version {}", crate::branding::VERSION));
                ui.separator();
                ui.label(crate::i18n::tr("Gestionnaire de connexions distantes pour Windows et Linux, développé en Rust."));
                ui.add_space(8.0);
                ui.label(crate::i18n::tr("Développé avec l'aide de l'intelligence artificielle (OpenAI Codex)."));
                ui.add_space(8.0);
                ui.label(crate::i18n::tr("Logiciel libre sous licence MIT."));
                ui.separator();
                close = ui.button(crate::i18n::tr("Fermer")).clicked();
            });
        self.about_open = open && !close;
    }
}
