# HostGates 0.3.7

## Compatibilité avec les anciens serveurs SSH

Pour une erreur « no matching host key type found. Their offer: ssh-rsa » :

1. Clic droit sur la session → Options SSH / tunnel.
2. Cocher « Autoriser la clé hôte ssh-rsa (SHA-1, serveur ancien) ».
3. Enregistrer puis relancer la connexion. Vérifier l'empreinte du serveur.

Option désactivée par défaut et propre à chaque session. Disponible sous Windows et Linux pour SSH intégré, SFTP, tunnels et OpenSSH externe. Pour une passerelle ancienne, la cocher sur la session SSH de la passerelle. Les algorithmes modernes gardent la priorité ; les clés modifiées sont toujours refusées. Aucun changement global des politiques cryptographiques. Cela ne réactive pas les autres algorithmes anciens ni RSA/SHA-1 pour l'authentification par clé utilisateur. Certains environnements peuvent interdire SHA-1 indépendamment de cette option.

Les deux nouveaux textes d'interface sont traduits dans les 12 langues.

## Livrables

EXE Windows x64, ZIP avec documentation/licences, DEB amd64 (Debian 12+ / Ubuntu 22.04+) et RPM x86_64 (RHEL/Rocky/AlmaLinux 9+ / Fedora récent), avec empreintes SHA-256. Binaire Linux compilé sur Rocky Linux 9, glibc 2.34 minimum. RHEL et AlmaLinux restent des cibles sans test direct ; paquets non signés.

## RDP dans un onglet

Cette version conserve le client RDP externe. Un bureau RDP intégré nécessitera un moteur et une gestion dédiés de l'affichage, des entrées, des certificats et de l'authentification. IronRDP est une piste Rust pour une intégration multiplateforme ; aucune intégration RDP embarquée n'est annoncée comme fonctionnelle dans cette version.

Tests locaux SSH : le serveur de fixture nécessite `python -m pip install --target .tools/ssh-test-legacy paramiko==3.5.1` ; lancer ensuite `cargo test --test native_ssh -- --ignored`. Cette dépendance de test n’est pas embarquée dans l’application.
