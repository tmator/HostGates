# HostGates 0.3.4

Gestionnaire desktop en Rust natif (egui/eframe), conçu pour Windows et Linux.

## Fonctionnalités disponibles

- Sessions SSH/RDP/VNC/Web persistantes, dossiers imbriqués, favoris, tags, recherche, duplication et modification.
- Terminal SSH intégré via libssh2, avec mode OpenSSH externe facultatif : couleurs ANSI, UTF-8, redimensionnement, sélection, presse-papiers, défilement, recherche du texte visible et reconnexion.
- Terminal local PowerShell sous Windows, bash/sh sous Linux.
- RDP via mstsc sous Windows ; FreeRDP sous Linux avec demandes interactives dans l’onglet et bureau dans une fenêtre externe.
- Import MobaXterm INI/.mxtsessions et CSV : aperçu, sélection, dossiers, erreurs par ligne et gestion des doublons ; transaction atomique.
- Export CSV MobaXterm documenté et .mxtsessions de base, sans secrets. La compatibilité du format propriétaire avec toutes les versions MobaXterm n’est pas validée ; privilégier CSV.
- Aide hors ligne avec neuf rubriques, dépannage et raccourcis ; thèmes clair/sombre/système.
- SQLite pour les métadonnées ; coffre maître chiffré ou coffre système pour les secrets. SSH intégré, MSTSC/FreeRDP et TigerVNC utilisent les identifiants sauvegardés ; voir les modalités dans [les fonctions avancées](docs/ADVANCED_FEATURES.md).

## Licence

HostGates est sous [licence MIT](licence.md). Voir [licences tierces et compatibilité MobaXterm](docs/LICENSING.md) pour les conditions de redistribution. HostGates est un projet indépendant, sans affiliation à Mobatek.

Fermer les anciennes versions avant de lancer `target/release/HostGates-0.3.4.exe`. Le coffre maître est verrouillé au démarrage et ferme les connexions après 10 minutes sans interaction. Les anciens binaires ne connaissent pas ce coffre : ne pas les utiliser après migration des secrets.

Compilation 0.3.1 : OpenSSL est compilé et lié statiquement avec SSH. Perl et un compilateur C sont nécessaires. Sous Windows, installer Strawberry Perl et le rendre disponible dans le PATH. Sous Linux, installer `perl`, `make` et `build-essential`. Ces outils ne sont pas nécessaires pour exécuter le binaire compilé.
