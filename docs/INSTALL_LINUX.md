# Installer sous Linux

Cette première version doit être compilée selon BUILD_LINUX.md. Lancer `target/release/hostgates` depuis une session graphique X11 ou Wayland. `.deb` et AppImage prévus en phase 9.

Un service Secret Service actif est nécessaire pour sauvegarder les mots de passe : GNOME Keyring, ou KDE Wallet exposant cette API. Debian/Ubuntu : `sudo apt install gnome-keyring dbus-user-session` si ces composants ne sont pas déjà installés. Un coffre absent/verrouillé ne déclenche jamais de fichier de secours.

La base est sous le répertoire XDG utilisateur ; le chemin exact apparaît dans Paramètres. Le dossier est protégé en 0700. Le coffre reste indépendant de SQLite.

Pour les phases réseau futures : `openssh-client`, et FreeRDP (`freerdp3-x11` selon la distribution/version ; vérifier le paquet disponible). Fedora : `openssh-clients`, `freerdp`, `gnome-keyring`. Aucun de ces clients n'est utilisé par la phase 1.

Fedora et les sessions KDE nécessitent une validation dédiée. L'absence de coffre permet uniquement la gestion des métadonnées à ce stade ; la saisie de secrets éphémères à chaque connexion arrivera avec SSH/RDP.
