# HostGates 0.3.6

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

## Langues prises en charge

L'interface propose **12 langues**, disponibles dans **Paramètres → Langue** :

| Langue | Nom dans l'application |
|---|---|
| Français | Français |
| Anglais | English |
| Espagnol | Español |
| Italien | Italiano |
| Allemand | Deutsch |
| Portugais du Brésil | Português (Brasil) |
| Japonais | 日本語 |
| Chinois simplifié | 简体中文 |
| Coréen | 한국어 |
| Arabe | العربية |
| Hindi | हिन्दी |
| Indonésien | Bahasa Indonesia |

Les traductions et les polices sont intégrées à l'exécutable et fonctionnent hors ligne. Les fichiers JSON permettent de personnaliser ou d'ajouter une langue : voir [le guide des langues](docs/LOCALES.md).

Les dix nouvelles traductions sont automatiques avec corrections ciblées. Le rendu des passages mélangeant arabe et texte latin reste partiel ; voir [les notes de version 0.3.5](docs/RELEASE_0.3.5.md).

## Télécharger

La [release 0.3.6](https://github.com/tmator/HostGates/releases/tag/v0.3.6) fournit l'exécutable Windows x64, son archive avec documentation et licences, ainsi que les paquets Linux **DEB et RPM pour x86-64**, accompagnés de leurs empreintes SHA-256. Les instructions de [compilation Windows](docs/BUILD_WINDOWS.md) et [compilation Linux](docs/BUILD_LINUX.md) sont disponibles dans la documentation.

Pour Linux : **Debian 12+ / Ubuntu 22.04+** (DEB), **RHEL / Rocky / AlmaLinux 9+ et Fedora récent** (RPM). Installation et démarrage graphique de contrôle validés en conteneurs sur Debian 12, Ubuntu 22.04.5, Rocky 9.8 et Fedora 44. RHEL et AlmaLinux restent des cibles de compatibilité sans test direct. Voir le [guide des paquets Linux](docs/LINUX_PACKAGES.md).

## Licence

HostGates est sous [licence MIT](licence.md). Voir [licences tierces et compatibilité MobaXterm](docs/LICENSING.md) pour les conditions de redistribution. HostGates est un projet indépendant, sans affiliation à Mobatek.

Fermer les anciennes versions avant de lancer `target/release/HostGates-0.3.6.exe`. Le coffre maître est verrouillé au démarrage et ferme les connexions après 10 minutes sans interaction. Les anciens binaires ne connaissent pas ce coffre : ne pas les utiliser après migration des secrets.

Compilation 0.3.1 : OpenSSL est compilé et lié statiquement avec SSH. Perl et un compilateur C sont nécessaires. Sous Windows, installer Strawberry Perl et le rendre disponible dans le PATH. Sous Linux, installer `perl`, `make` et `build-essential`. Ces outils ne sont pas nécessaires pour exécuter le binaire compilé.
