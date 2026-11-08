# HostGates 0.9 — bêta 1

Préversion **0.9.0-beta.1**, 8 novembre 2026. Changements depuis 0.4.0.

## Nouveautés

- **Proxmox** : plusieurs serveurs dépliables dans le panneau latéral, inventaire et recherche des VM et conteneurs, authentification par mot de passe ou jeton API, secrets conservés dans le coffre.
- **SPICE** : ouverture des VM Proxmox dans remote-viewer sous Windows et Linux, chemin du client configurable, nouveaux tickets à chaque ouverture et prise en compte des migrations.
- **LXC** : console terminal Proxmox directement dans un onglet, via termproxy/WebSocket. Aucun serveur SSH ni adresse IP du conteneur n’est nécessaire.
- **Hyper-V sous Windows** : inventaire des hôtes et clusters via WinRM et ouverture des consoles dans VMConnect.
- **Alimentation** : démarrer, arrêter proprement, redémarrer ou couper brutalement l’alimentation des machines Hyper-V et Proxmox, avec confirmation et suivi des tâches Proxmox.
- **États visibles** : rond vert pour allumé, rouge pour arrêté, noir pour désactivé/modèle, orange pour inconnu ou intermédiaire. Actualisation manuelle, sans interrogation permanente au repos.
- **Résolution RDP** : choix mémorisé dans la configuration des sessions IronRDP, y compris avec RD Gateway/RDS. Menu de résolutions standard, plein écran ou valeur du client avant une connexion MSTSC/FreeRDP.
- Aide, guides de configuration et traductions des nouveaux contrôles dans les douze langues.

## Corrections

- Une erreur de console SPICE ou LXC ne transforme plus tout l’inventaire en état inconnu et ne bloque plus les autres consoles.
- Compatibilité des réponses Proxmox avec un port de console LXC numérique ou textuel.
- Séparation des erreurs de console, des inventaires périmés et des actions d’alimentation incertaines, limitées à la machine concernée.
- Diagnostics Proxmox plus précis pour les refus d’authentification et les réponses incompatibles, sans afficher les secrets ni les tickets.

## Configuration et limites de cette bêta

La validation TLS est activée par défaut. Un CA privé peut être configuré ; une exception explicite par serveur permet d’ignorer la vérification du certificat Proxmox. Cette exception retire l’authentification du serveur : préférer un CA de confiance. Voir [securite.txt](../securite.txt).

SPICE nécessite **remote-viewer**, installé séparément, et un affichage SPICE configuré sur la VM. Les consoles LXC nécessitent **VM.Console** et un accès WebSocket fonctionnel ; les actions d’alimentation nécessitent **VM.PowerMgmt**. Hyper-V nécessite Windows, WinRM et les droits appropriés ; VMConnect utilise le compte Windows courant et s’ouvre dans une fenêtre externe. Aucun support VMware n’est ajouté.

Le RDP intégré conserve les limites documentées pour l’audio, les périphériques, RemoteApp et les redirections avancées. La résolution choisie s’applique au prochain lancement. Cette bêta n’est pas une certification de toutes les infrastructures Hyper-V, Proxmox ou RDS. Voir les guides [Hyper-V / Proxmox / LXC](HYPERV_PROXMOX.md), [SPICE](SPICE_PROXMOX.md) et [RDP](RDP_INTEGRATED.md).

## Téléchargements

- Windows x64 : EXE et ZIP avec documentation et licences.
- Linux amd64 : DEB pour Debian 12+ / Ubuntu 22.04+.
- Linux x86_64 : RPM pour RHEL / Rocky / AlmaLinux 9+ et Fedora récent, base glibc 2.34.
- Fichiers d’empreintes SHA-256 pour vérifier les téléchargements.

Les paquets utilisent la version `0.9.0~beta.1-1`, antérieure à la future version stable `0.9.0-1`. Les fichiers ne sont pas signés par une clé d’éditeur. Aucun coffre, identifiant personnel, export de sessions ni outil local Codex n’est inclus.
