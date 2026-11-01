# HostGates 0.4.0 — RDP intégré et passerelles RDS

Publié le 1er novembre 2026. Changements depuis 0.3.7.

## Nouveautés

- Bureau RDP dans les onglets sous Windows et Linux, avec IronRDP : affichage, clavier, souris et Ctrl+Alt+Suppr.
- RD Gateway en HTTPS, avec transport RPC sur HTTPS (RPCH) pour les passerelles qui le nécessitent. Réutilisation des identifiants du bureau par défaut, avec possibilité d’utiliser un compte de passerelle distinct.
- Import de fichiers `.rdp` UTF-8/UTF-16 : adresse, port, utilisateur, domaine, passerelle et routage de collection RDS. Conservation des options au redémarrage.
- Suivi des redirections RDS sous TLS : serveur cible, identifiant de session et jeton de reconnexion, via la même passerelle.
- Plein écran pour une connexion RDP. Retour par le bouton dédié ou Ctrl+Alt+Entrée, sans fermer les autres onglets.
- Choix global du client dans Paramètres : IronRDP intégré, MSTSC sous Windows ou FreeRDP sous Linux. Chaque session peut suivre ce choix ou le remplacer.

## Corrections et performances

- Curseur RDP affiché localement ; regroupement des mouvements consécutifs et envoi des entrées par lots, sans réordonner les clics et les touches.
- Envoi des entrées sans attendre l’affichage suivant ; TCP_NODELAY sur les connexions RDP et de passerelle pour réduire le délai des petits paquets.
- Affichage des causes détaillées des erreurs de passerelle et des refus du serveur pendant l’activation.
- Correction de l’ouverture par MSTSC des fichiers temporaires d’authentification sous Windows.
- Conservation du routage de collection lors du lancement externe d’une session avec RD Gateway.
- Nouveaux contrôles traduits dans les 12 langues ; documentation de configuration et de sécurité actualisée.

## Sécurité et limites

La passerelle et chaque serveur RDP font l’objet de vérifications TLS distinctes. Un certificat non reconnu exige une acceptation explicite. Les redirections sont bornées et les boucles refusées. Aucun mot de passe n’est ajouté aux arguments du client intégré ; les jetons de reconnexion sont masqués dans les représentations de diagnostic.

Le bureau intégré conserve une résolution de 1280 × 720 ajustée à la fenêtre. Audio, imprimantes, lecteurs, presse-papiers distant, RemoteApp et authentification par carte à puce ne sont pas pris en charge par cette intégration. Les redirections RDSTLS ne sont pas prises en charge. Le client externe reste disponible pour les besoins avancés. Un tunnel SSH à destination fixe ne suit pas les redirections RDS.

La fluidité dépend aussi du réseau, de la passerelle et du serveur. Les essais locaux ne constituent pas une certification de toutes les infrastructures RDS.

## Téléchargements

- EXE Windows x64 et ZIP avec documentation et licences.
- DEB amd64 : Debian 12+ et Ubuntu 22.04+.
- RPM x86_64 : RHEL/Rocky/AlmaLinux 9+ et Fedora récent.
- Empreintes SHA-256 des fichiers distribués.

Les paquets Linux utilisent une base glibc 2.34. RHEL et AlmaLinux sont des cibles de compatibilité, sans validation directe. Les fichiers ne sont pas signés par une clé d’éditeur. Les données utilisateur et le coffre ne sont pas inclus dans les distributions.
