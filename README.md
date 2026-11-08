# HostGates

Gestionnaire desktop en Rust natif (egui/eframe), conçu pour Windows et Linux.

## Fonctionnalités disponibles

- Sessions SSH/RDP/VNC/Web persistantes, dossiers imbriqués, favoris, tags, recherche, duplication et modification.
- Terminal SSH intégré via libssh2, avec mode OpenSSH externe facultatif : couleurs ANSI, UTF-8, redimensionnement, sélection, presse-papiers, défilement, recherche du texte visible et reconnexion.
- Terminal local PowerShell sous Windows, bash/sh sous Linux.
- RDP intégré dans les onglets sous Windows et Linux ; mstsc/FreeRDP externe reste une option. Voir [le guide RDP intégré](docs/RDP_INTEGRATED.md).
- Import MobaXterm INI/.mxtsessions et CSV : aperçu, sélection, dossiers, erreurs par ligne et gestion des doublons ; transaction atomique.
- Export CSV MobaXterm documenté et .mxtsessions de base, sans secrets. La compatibilité du format propriétaire avec toutes les versions MobaXterm n’est pas validée ; privilégier CSV.
- Aide hors ligne, dépannage et raccourcis ; thèmes clair/sombre/système.
- SQLite pour les métadonnées ; coffre maître chiffré ou coffre système pour les secrets. SSH intégré, MSTSC/FreeRDP et TigerVNC utilisent les identifiants sauvegardés ; voir les modalités dans [les fonctions avancées](docs/ADVANCED_FEATURES.md).

## Nouveautés de la bêta 0.9

- Choix de résolution avant une connexion MSTSC/FreeRDP.
- Serveurs Proxmox dépliables et VM dans le panneau latéral, avec consoles SPICE via remote-viewer externe. Installation du client, certificats et droits : [guide Proxmox/SPICE](docs/SPICE_PROXMOX.md).

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

La [préversion 0.9.0-beta.2](https://github.com/tmator/HostGates/releases/tag/v0.9.0-beta.2) fournit l'exécutable Windows x64, son archive avec documentation et licences, ainsi que les paquets Linux **DEB et RPM pour x86-64**, accompagnés de leurs empreintes SHA-256. Les instructions de [compilation Windows](docs/BUILD_WINDOWS.md) et [compilation Linux](docs/BUILD_LINUX.md) sont disponibles dans la documentation.

Pour Linux : **Debian 12+ / Ubuntu 22.04+** (DEB), **RHEL / Rocky / AlmaLinux 9+ et Fedora récent** (RPM). Installation et démarrage graphique de contrôle validés en conteneurs sur Debian 12, Ubuntu 22.04.5, Rocky 9.8 et Fedora 44. RHEL et AlmaLinux restent des cibles de compatibilité sans test direct. Voir le [guide des paquets Linux](docs/LINUX_PACKAGES.md).

## Licence

HostGates est sous [licence MIT](licence.md). Voir [licences tierces et compatibilité MobaXterm](docs/LICENSING.md) pour les conditions de redistribution. HostGates est un projet indépendant, sans affiliation à Mobatek.

Fermer les anciennes versions avant de lancer `target/release/HostGates-0.9.0-beta.2.exe`. Le coffre maître est verrouillé au démarrage et ferme les connexions après 10 minutes sans interaction. Les anciens binaires ne connaissent pas ce coffre : ne pas les utiliser après migration des secrets.

Compilation 0.3.1 : OpenSSL est compilé et lié statiquement avec SSH. Perl et un compilateur C sont nécessaires. Sous Windows, installer Strawberry Perl et le rendre disponible dans le PATH. Sous Linux, installer `perl`, `make` et `build-essential`. Ces outils ne sont pas nécessaires pour exécuter le binaire compilé.

Serveurs SSH anciens : clic droit sur la session → Options SSH / tunnel → Autoriser la clé hôte ssh-rsa (SHA-1, serveur ancien). Cette exception est désactivée par défaut ; voir [les notes 0.3.7](docs/RELEASE_0.3.7.md).

### Import RDP et RD Gateway

Le menu d’import accepte aussi les fichiers `.rdp` (UTF-8 / UTF-16 avec BOM) : adresse, port, utilisateur, domaine et RD Gateway. L’aperçu indique les limites ; les secrets et redirections de périphériques ne sont pas repris. RD Gateway se configure dans les options de session et fonctionne dans les onglets via IronRDP. MSTSC ou FreeRDP restent disponibles en option. Voir [le guide RDP](docs/RDP_INTEGRATED.md).

## Version 0.9 (bêta)

RDP intégré avec RD Gateway/RDS, import `.rdp`, plein écran et choix du client par défaut. Voir le [journal des versions](CHANGELOG.md) et les [notes détaillées](docs/RELEASE_0.9.0-beta.2.md).

### Hyper-V, actions Proxmox et LXC

Le panneau Proxmox / Hyper-V affiche les machines et leur état. Le clic droit permet de démarrer, arrêter proprement, redémarrer ou couper l’alimentation, avec confirmation. Les conteneurs LXC ouvrent leur console terminal Proxmox dans un onglet, sans SSH ni IP du conteneur. Hyper-V prend en charge les hôtes et clusters via WinRM : sous Windows avec VMConnect ; sous Linux avec Python/pypsrp et FreeRDP externes (depuis la bêta 0.9.0-beta.2).

Voir le [guide de configuration Hyper-V / Proxmox / LXC](docs/HYPERV_PROXMOX.md) pour les droits, certificats, clients et limites.

La résolution du bureau IronRDP se choisit dans **Nouvelle session / Modifier la session → Résolution IronRDP**. Elle est mémorisée par connexion et appliquée au prochain lancement, en direct ou via RD Gateway. Le choix MSTSC/FreeRDP reste proposé avant lancement du client externe.
