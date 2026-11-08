# HostGates 0.9 — bêta 2

Préversion **0.9.0-beta.2**, 8 novembre 2026. Changements depuis la bêta 1.

## Hyper-V depuis Linux

- Inventaire des hôtes et clusters Hyper-V, affichage des états et actions d’alimentation : démarrage, arrêt propre, redémarrage et coupure forcée avec confirmation.
- Gestion via WinRM HTTPS et un adaptateur Python/pypsrp embarqué dans HostGates ; PowerShell s’exécute sur les hôtes Windows, sans installation de PowerShell sur le poste Linux.
- Ouverture des consoles Hyper-V dans une fenêtre FreeRDP externe, suivie par un onglet HostGates. Le nœud propriétaire est recherché avant chaque connexion ; aucune IP ni serveur RDP dans la VM n’est nécessaire.
- Utilisation des identifiants du profil/coffre pour la console Linux, avec transmission du mot de passe par entrée standard, sans argument `/p:`.
- Chemins Python et FreeRDP et certificat CA WinRM configurables dans le profil du serveur. Les paramètres VMConnect Windows sont conservés séparément.
- Diagnostics spécifiques pour les dépendances manquantes, les certificats refusés, les refus NTLM et les hôtes inaccessibles. Aide, documentation et douze langues mises à jour.

## Prérequis et limites

Sous Linux, installer **Python 3 avec pypsrp >=0.8.1,<0.9** et **FreeRDP**. Ces dépendances sont facultatives pour les autres protocoles et ne sont pas installées automatiquement. Le [guide Hyper-V](HYPERV_PROXMOX.md#hyper-v-depuis-linux) détaille la création d’un environnement Python et la configuration du profil.

Cette première intégration utilise **NTLM sur WinRM HTTPS**, généralement sur le port **5986**. Les domaines interdisant NTLM ne sont pas pris en charge par cet adaptateur. Les hôtes nécessitent les droits et modules Hyper-V / FailoverClusters appropriés.

La console FreeRDP utilise le port **2179**, les droits de console du compte et un certificat approuvé par son propre magasin de confiance/système. Le CA WinRM du profil ne configure pas celui de FreeRDP. Les certificats non approuvés sont refusés ; aucun contournement TLS automatique n’est ajouté. La console est externe, pas intégrée au rendu IronRDP.

Les requêtes WinRM refusent les redirections et les relances automatiques. Les réponses et la durée des opérations sont bornées. Une action acceptée par l’hôte peut continuer après une déconnexion ou un délai dépassé : actualiser avant de réessayer.

Sous Windows, l’inventaire utilise toujours PowerShell/WinRM et VMConnect utilise le compte Windows courant. Proxmox, LXC, SSH et RDP conservent les fonctionnalités de la bêta 1.

## Validation

Tests Rust Windows/Linux, tests de routage et d’actions avec serveurs simulés, contrôle de syntaxe des scripts PowerShell, transport pypsrp réel contre un serveur HTTPS local, refus des certificats non approuvés et des redirections, limites cumulées des réponses, transmission du mot de passe FreeRDP uniquement par stdin. La validation d’une authentification et d’une console sur une infrastructure Hyper-V réelle reste nécessaire.

## Téléchargements

- Windows x64 : EXE et ZIP avec documentation et licences.
- Linux amd64 : DEB pour Debian 12+ / Ubuntu 22.04+.
- Linux x86_64 : RPM pour RHEL / Rocky / AlmaLinux 9+ et Fedora récent, base glibc 2.34.
- Empreintes SHA-256 des fichiers Windows et Linux.

Les paquets ont la version interne `0.9.0~beta.2-1`, antérieure à la future stable `0.9.0-1`. Leurs noms de téléchargement utilisent `0.9.0-beta.2`. Les fichiers ne sont pas signés par une clé d’éditeur. Aucun coffre, profil personnel ni outil local Codex n’est inclus.
