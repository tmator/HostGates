# Journal des versions

## 0.9.0-beta.1 — 8 novembre 2026 (bêta)

- Une erreur SPICE/LXC ne rend plus l’inventaire entier inconnu et ne bloque plus les autres consoles. Les erreurs d’inventaire restent distinctes ; une action d’alimentation incertaine ne concerne que sa machine cible.
- Compatibilité des tickets de console LXC avec un port JSON numérique ou textuel ; diagnostic de réponse incompatible précisant l’étape API, sans exposer les tickets.

- Résolution IronRDP configurable lors de la création/modification d’une session, avec tailles standard et mémorisation ; appliquée aussi aux passerelles RDS.
- Console LXC intégrée via termproxy/WebSocket Proxmox, remplaçant le raccourci SSH ; aucun SSH ni IP du conteneur nécessaire.
- Indicateurs d’état colorés : vert allumé, rouge arrêté, noir désactivé/modèle, orange inconnu ou intermédiaire.

- Hôtes et clusters Hyper-V sous Windows : inventaire via WinRM, état des VM, accès VMConnect avec chemin configurable et coffre pour les identifiants de gestion.
- Actions confirmées Hyper-V et Proxmox : démarrage, arrêt propre, redémarrage propre et coupure forcée distincte. Suivi des tâches Proxmox et vérification de l’hôte courant avant mutation.
- Inventaire des conteneurs LXC et console terminal intégrée authentifiée par le profil Proxmox.
- États explicites et indication des inventaires périmés, sans interrogation continue au repos. [Configuration et limites](docs/HYPERV_PROXMOX.md).

- Diagnostic Proxmox HTTP 401 distinct pour un refus de connexion, un ticket refusé après connexion ou un jeton API refusé, avec l’étape concernée et sans exposer les secrets.

- Option par serveur Proxmox pour ignorer explicitement la vérification du certificat HTTPS, désactivée par défaut et accompagnée d’un avertissement.

- Choix de résolution avant une connexion RDP externe (MSTSC / FreeRDP), avec
  résolutions standard, plein écran et mémorisation par session.
- Prise en compte du choix pour les connexions directes, RD Gateway/RDS et tunnels SSH.
- Serveurs Proxmox dépliables dans le panneau latéral, inventaire des VM et consoles SPICE via remote-viewer sous Windows et Linux.
- Authentification Proxmox par compte ou jeton API, secrets dans le coffre, certificats CA privés, actualisation et recherche des VM.
- Tickets SPICE renouvelés à chaque ouverture, prise en compte des migrations de VM et configuration du chemin du client externe.
- Aide, traductions et [guide Proxmox/SPICE](docs/SPICE_PROXMOX.md).

## 0.4.0 — 1er novembre 2026

- RDP intégré multiplateforme, RD Gateway HTTP/RPCH et redirections RDS.
- Import `.rdp`, conservation du routage de collection et des options de passerelle.
- Plein écran, curseur local, regroupement des mouvements et réduction des délais d’envoi.
- Choix global IronRDP/MSTSC/FreeRDP et exception par session.
- Diagnostics RDP détaillés, correction des fichiers temporaires Windows, traductions et documentation mises à jour.

[Notes complètes](docs/RELEASE_0.4.0.md)

## Versions précédentes

- [0.3.7](docs/RELEASE_0.3.7.md) : option de compatibilité ssh-rsa par session.
- [0.3.6](https://github.com/tmator/HostGates/releases/tag/v0.3.6) : corrections de sécurité et de robustesse.
- [0.3.5](https://github.com/tmator/HostGates/releases/tag/v0.3.5) : prise en charge de douze langues et distributions Linux DEB/RPM.
