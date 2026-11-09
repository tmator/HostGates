use super::*;
pub const TOPICS: &[(&str, &str, &str)] = &[
    (
        "Proxmox et SPICE",
        "Installez remote-viewer (Virt Viewer), puis configurez son chemin dans Paramètres > Applications externes > SPICE. Panneau latéral > Proxmox / SPICE > Ajouter un serveur : URL HTTPS, utilisateur@realm et mot de passe, ou utilisateur@realm!jeton et secret API. Les secrets peuvent être conservés dans le coffre. Pour un certificat privé, sélectionnez le CA PEM fourni par votre administrateur ; le nom DNS doit correspondre. La case Ne pas vérifier le certificat HTTPS désactive cette protection pour ce serveur seulement, avec un risque d’interception des identifiants ; préférez configurer le CA. Droits nécessaires : VM.Audit et VM.Console, sur le compte et le jeton. Activez un affichage SPICE dans le matériel de la VM et démarrez-la. Dépliez le serveur, puis double-cliquez sur une VM. La console s’ouvre dans une fenêtre externe. API habituellement sur 8006, proxy SPICE sur 3128. Le champ proxy permet un hôte différent de l’API. Actualiser recharge les VM ; Supprimer retire seulement la configuration locale. Les LXC et la saisie MFA interactive ne sont pas pris en charge. Voir docs/SPICE_PROXMOX.md et securite.txt.",
        "Install remote-viewer (Virt Viewer), then configure its path in Settings > External applications > SPICE. Sidebar > Proxmox / SPICE > Add server: HTTPS URL, user@realm and password, or user@realm!token and API secret. Secrets can be saved in the vault. For private certificates, select the CA PEM supplied by your administrator; the DNS name must match. Do not verify the HTTPS certificate disables this protection for this server only and risks credential interception; prefer configuring the CA. Required privileges: VM.Audit and VM.Console for both account and token. Enable SPICE display in the VM hardware and start it. Expand the server, then double-click a VM. The console opens in an external window. API typically uses port 8006; SPICE proxy uses 3128. The proxy field allows a host different from the API. Refresh reloads the VMs; Remove only deletes local configuration. LXC and interactive MFA are not supported. See docs/SPICE_PROXMOX.md and securite.txt.",
    ),
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
        "Créez une session SSH pour la passerelle, connectez-la pour vérifier son empreinte et enregistrez son mot de passe ou configurez sa clé/son agent. Sur la cible : clic droit > Options SSH / tunnel > choisissez la passerelle. Un seul saut est pris en charge. SSH, RDP, VNC et Web utilisent un port local aléatoire lié à une adresse de bouclage 127.x distincte. La passerelle doit autoriser le transfert TCP. Gardez l'onglet ouvert. Web utilise une URL locale : les certificats HTTPS et les sites exigeant leur nom DNS peuvent être incompatibles ; aucune vérification TLS n'est désactivée. Les paramètres de tunnels propriétaires MobaXterm ne sont pas importés.",
        "Create an SSH gateway session, connect once to verify its fingerprint, and save its password or configure a key/agent. Right-click the target > SSH / tunnel options > select the gateway. One hop is supported. SSH, RDP, VNC and Web use a random port bound to a distinct 127.x loopback address. The gateway must allow TCP forwarding. Keep the tab open. Web uses a local URL: HTTPS certificates and sites requiring their DNS hostname may be incompatible; TLS validation is never disabled. Proprietary MobaXterm tunnel settings are not imported.",
    ),
    (
        "Onglets et commandes multiples",
        "Volets visibles affiche 1 à 6 connexions ; les autres restent ouvertes. Chaque volet et chaque onglet a un menu ▾ pour sélectionner une connexion ouverte. Renommer l'onglet change son titre pendant cette exécution. Cliquez dans le terminal pour lui donner le clavier. Exécution multiple permet de sélectionner les terminaux, saisir une ou plusieurs lignes, vérifier les destinataires, puis exécuter. Chaque ligne est envoyée avec Entrée ; les résultats restent dans chaque terminal. Cela ne détecte pas les invites : ne sélectionnez pas un terminal attendant un mot de passe ou une confirmation.",
        "Visible panes displays 1–6 connections; other tabs stay open. Each pane and tab has a ▾ menu to select an open connection. Rename tab changes its title for this run. Click inside a terminal to focus keyboard input. Multi-execution lets you select terminals, enter one or more lines, check recipients, then execute. Each line is sent with Enter; output stays in each terminal. Prompts are not detected: do not select a terminal waiting for a password or confirmation.",
    ),
    (
        "Connexion RDP intégrée",
        "RDP s'ouvre dans un onglet sous Windows et Linux sans client externe. Saisissez utilisateur, domaine facultatif et mot de passe, puis Connexion. Le coffre préremplit le mot de passe enregistré. Cliquez dans le bureau pour utiliser clavier et souris ; Ctrl+Alt+Suppr est disponible au-dessus. Le bureau 1280×720 est ajusté au volet. Les tunnels SSH sont pris en charge. Un certificat non reconnu exige une vérification de son empreinte et une acceptation explicite valable seulement pour cette connexion. RD Gateway se configure dans les options et fonctionne dans les onglets. Saisissez les identifiants de la passerelle ou cochez leur réutilisation depuis le poste. Le client externe reste optionnel. Importez aussi les fichiers .rdp depuis Fichier > Importer. Les paramètres de base et RD Gateway sont repris, sans mots de passe, redirections ni RemoteApp. Pour les fonctions avancées (audio, imprimantes, disques, presse-papiers distant), cochez Utiliser le client RDP externe dans les options de la session. Voir docs/RDP_INTEGRATED.md.",
        "RDP opens in a tab on Windows and Linux without an external client. Enter username, optional domain and password, then Connect. The vault prefills a saved password. Click the desktop to send keyboard and mouse input; Ctrl+Alt+Delete is available above it. The 1280×720 desktop scales to the pane. SSH tunnels are supported. Untrusted certificates require fingerprint verification and explicit acceptance for this connection only. Configure RD Gateway in session options; it works in tabs. Enter separate gateway credentials or explicitly reuse the desktop credentials. The external client remains optional. File > Import also accepts .rdp files, preserving basic connection and RD Gateway settings, without passwords, redirections or RemoteApp. For advanced features (audio, printers, drives, remote clipboard), enable Use external RDP client in session options. See docs/RDP_INTEGRATED.md.",
    ),
    (
        "RDP, VNC et Web",
        "Le mode RDP externe optionnel lance mstsc sous Windows ou FreeRDP sous Linux. VNC utilise un client compatible TigerVNC hôte::port. Configurez leurs chemins dans Paramètres > Applications externes si nécessaire. Ils gardent leur fenêtre et leur authentification : HostGates n'injecte pas de mot de passe dans leurs arguments. Web ouvre http(s) dans le navigateur par défaut. Le coffre transmet les secrets RDP via DPAPI sous Windows ou stdin sous Linux (domaine explicite requis), et VNC via un fichier temporaire obfusqué DES, limité à 8 caractères ASCII. Aucun secret ne passe dans les arguments.",
        "Optional external RDP mode launches mstsc on Windows or FreeRDP on Linux. VNC uses a TigerVNC-compatible host::port client. Configure paths in Settings > External applications if needed. These clients keep their own window and authentication: HostGates never passes passwords in arguments. Web opens HTTP(S) in the default browser. RDP secrets use DPAPI on Windows or stdin on Linux (explicit domain required). VNC uses a temporary DES-obfuscated password file, limited to 8 ASCII characters. No secret is passed in arguments.",
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
        "Hyper-V / Proxmox / LXC",
        "Ajoutez un serveur Proxmox ou Hyper-V dans le panneau latéral. Hyper-V fonctionne sous Windows via WinRM et sous Linux via WinRM natif Rust (HTTPS :5986 requis sous Linux). Configurez FreeRDP et le CA PEM dans le profil Linux. FreeRDP ouvre la console externe sur :2179 avec le compte du profil ; son certificat doit être approuvé dans le système. Utilisez un compte de gestion ; cochez le cluster pour lister ses nœuds. Les hôtes nécessitent les modules Hyper-V / FailoverClusters. VMConnect utilise le compte Windows courant, avec un chemin configurable dans le profil. Clic droit sur une machine : démarrer, arrêter proprement, redémarrer ou couper l’alimentation, avec confirmation. Proxmox nécessite VM.PowerMgmt pour ces actions. Les états sont ceux de la dernière actualisation ; utilisez Actualiser. Double-cliquez un LXC allumé pour ouvrir sa console terminal Proxmox dans un onglet, sans SSH ni IP du conteneur. Le droit VM.Console est requis. Un délai dépassé n’annule pas forcément une commande distante. Guide détaillé : docs/HYPERV_PROXMOX.md.",
        "Add a Proxmox or Hyper-V server in the sidebar. Hyper-V uses WinRM on Windows and native Rust WinRM on Linux (HTTPS :5986 required on Linux). Configure FreeRDP and the PEM CA in the Linux profile. FreeRDP opens an external console on :2179 with profile credentials and requires a system-trusted certificate, with Hyper-V / FailoverClusters modules on hosts. Enable cluster inventory to list its nodes. VMConnect uses the current Windows account; its executable path is configurable in the profile. Right-click a machine to start, shut down gracefully, restart or power off forcibly, with confirmation. Proxmox power actions require VM.PowerMgmt. States reflect the last refresh; use Refresh. Double-click a running LXC to open its Proxmox terminal console in a tab, without SSH or a container IP. VM.Console permission is required. A timeout does not necessarily cancel a remote operation. See docs/HYPERV_PROXMOX.md.",
    ),
    (
        "Configurer les connexions",
        "SSH intégré ne nécessite pas ssh.exe, mais un serveur SSH accessible et un utilisateur valide. Authentification par mot de passe, clé privée ou agent compatible libssh2 ; certaines clés/agents Windows peuvent nécessiter OpenSSH externe. SFTP et tunnels suivent la même politique de clé serveur. Pour RDP externe/VNC/terminal local, configurez l'exécutable dans Paramètres > Applications externes, vérifiez puis enregistrez. En cas d'échec : vérifiez VPN, pare-feu, port, compte, permissions SFTP et autorisation de forwarding. La version Linux doit être compilée avec OpenSSL et pkg-config installés. Aucune certification de sécurité ; validation Linux réelle encore nécessaire.",
        "Integrated SSH needs no ssh.exe, but requires a reachable SSH server and a valid username. Password, private key and libssh2-compatible agent authentication are supported; some Windows keys/agents may require external OpenSSH. SFTP and tunnels use the same host-key policy. Configure external RDP/VNC/local terminal executables in Settings > External applications, check and save. On failure, check VPN, firewall, port, account, SFTP permissions and forwarding authorization. Linux builds require OpenSSL and pkg-config development packages. No security certification; testing on a real Linux desktop is still required.",
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
