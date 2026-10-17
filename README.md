# HostGates 0.3.4

Anciennement RemoteHub. Les sessions et le coffre existants sont conservés.

Gestionnaire desktop en Rust natif (egui/eframe), conçu pour Windows et Linux. Interface inspirée de l’organisation de MobaXterm : menus, barre d’outils, arborescence et onglets. Aucun navigateur ni Node.js requis.

## Fonctionnalités disponibles

- Sessions SSH/RDP/VNC/Web persistantes, dossiers imbriqués, favoris, tags, recherche, duplication et modification.
- Terminal SSH intégré via libssh2, avec mode OpenSSH externe facultatif : couleurs ANSI, UTF-8, redimensionnement, sélection, presse-papiers, défilement, recherche du texte visible et reconnexion.
- Terminal local PowerShell sous Windows, bash/sh sous Linux.
- RDP via mstsc sous Windows ; FreeRDP sous Linux avec demandes interactives dans l’onglet et bureau dans une fenêtre externe.
- Import MobaXterm INI/.mxtsessions et CSV : aperçu, sélection, dossiers, erreurs par ligne et gestion des doublons ; transaction atomique.
- Export CSV MobaXterm documenté et .mxtsessions de base, sans secrets. La compatibilité du format propriétaire avec toutes les versions MobaXterm n’est pas validée ; privilégier CSV.
- Aide hors ligne avec neuf rubriques, dépannage et raccourcis ; thèmes clair/sombre/système.
- SQLite pour les métadonnées ; coffre maître chiffré ou coffre système pour les secrets. SSH intégré, MSTSC/FreeRDP et TigerVNC utilisent les identifiants sauvegardés ; voir les modalités dans [les fonctions avancées](docs/ADVANCED_FEATURES.md).

## Lancer

Windows : `target/release/HostGates-0.3.4.exe` pour la compilation fournie. Créer ou importer une session puis double-cliquer pour connecter. F1 depuis l’accueil ouvre l’aide.

Avec Rust >=1.95 et les prérequis de compilation installés :

```powershell
cargo run --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt -- --check
cargo build --release --locked
```

OpenSSH doit être disponible dans PATH uniquement pour le mode externe. mstsc est utilisé sous Windows. Sous Linux, installer openssh-client et FreeRDP ainsi qu’un portail de bureau pour les dialogues de fichiers. Voir [compilation Linux](docs/BUILD_LINUX.md).

## État et limites

Cette version possède des connexions réelles et un import/export opérationnel. Le cahier des charges complet n’est pas achevé : La version 0.3 ajoute SFTP, tunnels SSH, multivues et exécution multiple ; verrouillage OS et installateurs restent à développer. Pas de validation sur un serveur RDP ni sur Linux depuis ce poste Windows. Les fixtures MobaXterm sont synthétiques ; aucun export utilisateur réel n’a été fourni.

Voir [import/export](docs/MOBAXTERM_IMPORT.md), [architecture](docs/ARCHITECTURE.md) et [sécurité](docs/SECURITY.md).

## Applications externes

Dans Paramètres, choisir les exécutables SSH, RDP et terminal local avec Parcourir ; vérifier puis enregistrer. Un champ vide conserve la recherche PATH. Les réglages persistent dans SQLite. Voir Aide > Configurer les connexions et [le guide](docs/CONNECTION_SETUP.md).

## Sessions VNC et Web

VNC (mode MobaXterm 5) utilise un client externe compatible TigerVNC configuré dans Applications externes. Web (mode 11) ouvre une URL HTTP/HTTPS dans le navigateur par défaut. Les deux types sont disponibles en création, import et export. Les sessions sont conservées lors de la migration automatique du schéma SQLite vers la version 2 ; fermez les anciennes versions de HostGates avant de démarrer cette version.

Les dossiers du panneau latéral sont repliés au démarrage. La recherche reste globale, même dans les dossiers fermés.

## Licence

HostGates est sous [licence MIT](licence.md). Voir [licences tierces et compatibilité MobaXterm](docs/LICENSING.md) pour les conditions de redistribution. HostGates est un projet indépendant, sans affiliation à Mobatek.


## Version 0.3 — fichiers, coffre et espace de travail

SSH intégré avec explorateur SFTP rétractable à droite (envoi/téléchargement de fichiers et dossiers), coffre maître Argon2id + AES-256-GCM-SIV, passerelles SSH pour les quatre protocoles, 1 à 6 volets visibles, menus de connexions, renommage des onglets et commandes multiples sur sélection.

Paramètres > Langue permet de choisir français/anglais. Les fichiers JSON dans `locales/` permettent d'ajouter une langue sans recompiler.

Voir le [guide des nouvelles fonctions](docs/ADVANCED_FEATURES.md), [les locales](docs/LOCALES.md) et [securite.txt](securite.txt). Le SSH intégré utilise les secrets du coffre ; les clients RDP/VNC/OpenSSH externes gardent leur authentification. Les tunnels Web utilisent une URL locale et ne contournent pas les contrôles HTTPS.

Fermer les anciennes versions avant de lancer `target/release/HostGates-0.3.4.exe`. Le coffre maître est verrouillé au démarrage et ferme les connexions après 10 minutes sans interaction. Les anciens binaires ne connaissent pas ce coffre : ne pas les utiliser après migration des secrets.

Compilation 0.3.1 : OpenSSL est compilé et lié statiquement avec SSH. Perl et un compilateur C sont nécessaires. Sous Windows, installer Strawberry Perl et le rendre disponible dans le PATH. Sous Linux, installer `perl`, `make` et `build-essential`. Ces outils ne sont pas nécessaires pour exécuter le binaire compilé.
