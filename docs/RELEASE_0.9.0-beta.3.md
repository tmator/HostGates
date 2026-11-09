# HostGates 0.9 — bêta 3

Préversion **0.9.0-beta.3**, 9 novembre 2026. Changements depuis la bêta 2.

## Hyper-V Linux sans Python

- Transport WinRM intégré en Rust avec `winrm-rs` : suppression du processus Python et de la dépendance pypsrp.
- Inventaire des hôtes et clusters, états et actions d’alimentation conservés. Le propriétaire de la VM est relu avant une console ou une commande ; aucune mutation échouée n’est relancée automatiquement.
- Authentification NTLM sur HTTPS avec validation des certificats, CA PEM facultatif par profil, refus des redirections et sorties/durée bornées.
- FreeRDP reste nécessaire pour les consoles externes. L’ancien chemin Python est ignoré ; les profils et le coffre restent utilisables.
- Aide, douze traductions, documentation de sécurité et notices de dépendances mises à jour. Rust 1.98 minimum pour compiler.

Cette implémentation couvre les opérations utilisées par HostGates, pas toute l’API pypsrp. Les hôtes Windows doivent autoriser les shells WinRM/WinRS et PowerShell, disposer des modules Hyper-V / FailoverClusters et des droits nécessaires. NTLM doit être autorisé. Le client Linux n’a besoin ni de Python ni de PowerShell local. Sous Windows, l’adaptateur PowerShell/WinRM et VMConnect sont conservés.

## SPICE Proxmox

- Suppression du champ non TLS `port=-1` et canaux TLS obligatoires dans le fichier temporaire `.vv`.
- Vérification TCP du proxy avant lancement, avec diagnostic indiquant le port inaccessible, sans envoyer le ticket pendant ce contrôle.
- Le port SPICE (habituellement 3128) doit être joignable depuis le poste. L’accès à l’interface Web sur 8006 ou derrière un reverse proxy ne suffit pas.

Le fichier corrigé a été testé avec remote-viewer contre un proxy et un serveur TLS locaux jusqu’au début du protocole SPICE. L’ancien fichier passe également cet essai avec cette version du client : cette modification ne garantit donc pas de résoudre tous les messages « impossible de se connecter au serveur d’affichage ». La connexion complète à une VM Proxmox réelle reste à valider.

## Validation et limites

Tests unitaires Rust Windows/Linux, routage des clusters et mutations simulés, refus des certificats non approuvés et redirections sur HTTPS local, vérification des douze catalogues de traduction et analyse Clippy stricte. L’authentification complète et les consoles sur une infrastructure Hyper-V réelle restent à valider. Un délai dépassé ne garantit pas l’annulation d’une commande distante : actualiser avant de réessayer.

## Téléchargements

- Windows x64 : EXE et ZIP avec documentation et licences.
- Linux amd64 : DEB pour Debian 12+ / Ubuntu 22.04+.
- Linux x86_64 : RPM pour RHEL / Rocky / AlmaLinux 9+ et Fedora récent, base glibc 2.34.
- Empreintes SHA-256 Windows et Linux.

Les paquets ont la version interne `0.9.0~beta.3-1`, antérieure à la future stable `0.9.0-1`. Les fichiers ne sont pas signés par une clé d’éditeur. Aucun coffre, profil personnel ni outil local Codex n’est inclus.

Voir le [guide Hyper-V](HYPERV_PROXMOX.md#hyper-v-depuis-linux) et le [guide SPICE](SPICE_PROXMOX.md).
