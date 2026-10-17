# Mise à jour 0.3 : SSH intégré et fonctions avancées

Le SSH intégré fonctionne désormais sans exécutable OpenSSH : renseigner l'utilisateur, vérifier l'empreinte, puis fournir un mot de passe, une clé privée ou un agent compatible. Le mode OpenSSH externe reste disponible via clic droit > Options SSH / tunnel. Il conserve les limitations d'intégration du coffre décrites plus bas. Le volet SFTP et les tunnels intégrés utilisent libssh2.

Pour les transferts de fichiers, le mot de passe maître, les passerelles et les commandes multiples, consulter [ADVANCED_FEATURES.md](ADVANCED_FEATURES.md). La section suivante reste utile pour configurer les programmes externes ; les modalités actuelles de transmission des secrets RDP/VNC sont détaillées dans ADVANCED_FEATURES.md (0.3.1).

# Configurer les applications externes

CONFIGURER LES CONNEXIONS

1. Ouvrez Paramètres > Applications externes. Les champs concernent les programmes installés sur ce PC, pas les chemins sur le serveur distant.
2. Laissez un champ vide pour chercher dans PATH, ou utilisez Parcourir pour sélectionner l’exécutable. Les espaces sont acceptés : ne mettez pas de guillemets, arguments, %VARIABLE% ou ~.
3. Cliquez Vérifier les chemins, puis Enregistrer les chemins. La vérification contrôle seulement la présence du fichier et ses permissions, pas sa version ni la connexion réseau. Utilisez un programme de confiance correspondant au client indiqué. PuTTY/plink n’est pas interchangeable avec OpenSSH ; Remmina n’est pas interchangeable avec FreeRDP.
4. Les changements s’appliquent aux nouvelles connexions, sans interrompre celles ouvertes. Auto efface le chemin personnalisé ; enregistrez ensuite. Après une modification du PATH système, redémarrez HostGates.

WINDOWS
SSH : installer Client OpenSSH dans les fonctionnalités facultatives de Windows. Le serveur OpenSSH local n’est pas nécessaire pour se connecter à une autre machine. Exemple habituel : C:\Windows\System32\OpenSSH\ssh.exe.
RDP : choisir mstsc.exe, habituellement C:\Windows\System32\mstsc.exe. Si un secret est enregistré dans le coffre, il est transmis via un fichier RDP protégé par DPAPI ; sinon le client demande le mot de passe.
Terminal local : PowerShell 7 (pwsh.exe) ou Windows PowerShell, par exemple C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe. Aucun terminal local n’est requis pour SSH : configurez ssh.exe directement.
Ces chemins sont des exemples ; adaptez-les à votre installation.

LINUX
Installer le client OpenSSH (paquet openssh-client sur Debian/Ubuntu), FreeRDP avec son client X11 et bash/sh. Le paquet FreeRDP dépend de la distribution/version. Sélectionner par exemple /usr/bin/ssh, /usr/bin/xfreerdp3 ou /usr/bin/xfreerdp et /usr/bin/bash. Les fichiers doivent être exécutables. FreeRDP nécessite un affichage graphique compatible X11/XWayland. Ses demandes de mot de passe/certificat apparaissent dans l’onglet et le bureau distant dans une fenêtre séparée.

CRÉER ET OUVRIR UNE SESSION
SSH : choisir SSH, nom, adresse DNS/IP sans ssh://, port (22 par défaut) et utilisateur. Le serveur doit fournir SSH et être joignable via le réseau/VPN. Double-cliquer la session. Vérifier l’empreinte de la clé hôte puis saisir le mot de passe ou la passphrase si demandé. Les clés et agents configurés dans OpenSSH restent utilisables.
RDP : choisir RDP, adresse sans rdp:// et port (3389 par défaut). Le service Bureau à distance doit être activé sur la machine cible et le compte autorisé. Renseigner utilisateur/domaine dans la session pour la transmission des identifiants à MSTSC/FreeRDP.
Le coffre transmet le secret à SSH intégré, MSTSC/FreeRDP et TigerVNC compatible PasswordFile. OpenSSH externe et les sites Web restent interactifs. Une option de saisie manuelle est disponible dans Options SSH / tunnel.

EN CAS D’ÉCHEC
Programme introuvable : vérifier le chemin, enregistrer puis reconnecter. Un chemin personnalisé invalide ne bascule pas silencieusement vers PATH.
Fichier trouvé mais lancement impossible : vérifier que c’est le bon client, son architecture, ses permissions et ses dépendances. La vérification du chemin ne lance aucun programme.
Délai dépassé/refus : vérifier hôte, port, VPN, pare-feu et service distant. Échec d’authentification : vérifier compte, domaine, clé et droits. Une clé hôte modifiée exige une vérification auprès de l’administrateur.

Sources :
- https://learn.microsoft.com/en-us/windows-server/administration/openssh/openssh_install_firstuse
- https://github.com/FreeRDP/FreeRDP/wiki/CommandLineInterface

## VNC et Web

VNC : installez TigerVNC Viewer (ou un client acceptant hôte::port), puis sélectionnez son exécutable dans Paramètres > Applications externes > VNC. Champ vide : recherche vncviewer/tigervncviewer/xtigervncviewer dans PATH. Port habituel 5900 ; authentification dans le client externe.

Web : URL complète HTTP/HTTPS, ouverte par le navigateur par défaut. Aucun chemin à configurer. Les URL avec identifiant/mot de passe sont refusées.

Import MobaXterm : mode 5 = VNC (le champ 3 est un réglage d’affichage, pas un utilisateur), mode 11 = Web (le champ 2 est un réglage d’affichage, pas un port). Passerelles/proxys non repris.
