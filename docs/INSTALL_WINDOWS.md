# Installer sous Windows

La phase 1 fournit un exécutable de développement, pas encore un installateur. Compiler selon BUILD_WINDOWS.md, puis lancer `target/release/hostgates.exe`. Windows 11 est la cible prioritaire ; Windows 10 reste à valider sur une machine dédiée. Pilote graphique avec OpenGL requis pour le backend glow.

WebView2 n'est pas requis. OpenSSH et mstsc ne sont pas utilisés en phase 1. Les prochains providers pourront détecter OpenSSH (`ssh.exe`, `sftp.exe`) et le client RDP Windows (`mstsc.exe`). Le script de diagnostic ne les installe pas.

Le chemin exact de la base est affiché dans Paramètres ; il est sous le profil utilisateur local. Les secrets sont dans Windows Credential Manager, service `io.remotehub.desktop.credentials`, persistance locale. Pour supprimer une session et son secret, utiliser l'application. Retirer l'exécutable ne supprime pas les données personnelles. Une désinstallation complète et ses choix de conservation seront documentés avec l'installateur.
