# Proxmox et consoles SPICE

HostGates affiche plusieurs serveurs Proxmox dans le panneau latéral, avec les VM QEMU sous chaque serveur dépliable. Un double-clic ouvre la console SPICE dans **remote-viewer**, dans une fenêtre externe. Le client SPICE doit être installé séparément sous Windows et Linux. Le rendu SPICE intégré dans un onglet n’est pas implémenté.

## Configuration

1. Installer **Virt Viewer / remote-viewer** : [téléchargements SPICE](https://www.spice-space.org/download.html). Sous Debian/Ubuntu : `sudo apt install virt-viewer` ; sous Fedora/Rocky avec les dépôts appropriés : `sudo dnf install virt-viewer`.
2. Dans **Paramètres → Applications externes → SPICE (remote-viewer)**, sélectionner `remote-viewer.exe` ou `/usr/bin/remote-viewer` si le programme n’est pas dans le PATH. Vérifier puis enregistrer.
3. Dans le panneau latéral, cliquer sur **Proxmox / Hyper-V → Ajouter un serveur**. Renseigner un nom et l’URL de l’API, par exemple `https://pve.example:8006`.
4. Choisir **Mot de passe** avec un identifiant `utilisateur@realm` (par exemple `alice@pve`), ou **Jeton API** avec `utilisateur@realm!nom-du-jeton` et son secret. Un compte dédié à la consultation des VM est recommandé. La saisie MFA interactive et la connexion OIDC/WebAuthn ne sont pas implémentées : utiliser un jeton API dédié lorsque le compte exige une MFA.
5. Cocher **Enregistrer dans le coffre** pour conserver le secret dans le coffre existant. Sinon il reste en mémoire jusqu’au verrouillage ou à la fermeture de HostGates. Une modification de l’adresse ou du compte exige une nouvelle saisie du secret, pour ne pas envoyer un ancien secret à une autre destination.
6. Si le certificat HTTPS de Proxmox dépend d’une autorité privée, sélectionner son certificat CA au format PEM, obtenu auprès de l’administrateur par un canal fiable. Le nom DNS de l’URL doit correspondre au certificat. La vérification est activée par défaut. La case **Ne pas vérifier le certificat HTTPS**, propre à ce serveur, permet explicitement d’accepter un certificat autosigné, expiré ou dont le nom ne correspond pas. Elle ignore également le fichier CA configuré. Dans ce mode, le trafic reste chiffré mais l’identité du serveur n’est plus authentifiée : les identifiants et les tickets peuvent être interceptés. Préférer l’import du CA lorsque possible.
7. Laisser **Hôte du proxy SPICE** vide pour utiliser le nom de l’hôte API. Si l’API passe par un reverse proxy qui ne transporte pas SPICE, saisir ici le nom DNS ou l’IP du proxy SPICE réellement accessible, sans protocole ni port.

Les métadonnées sont stockées dans `preferences.json`. Aucun mot de passe ni secret de jeton API n’y est enregistré. Les secrets utilisent le coffre système ou le coffre maître déjà configuré ; ils sont également repris lors d’une migration ultérieure vers le coffre maître.

## Préparer Proxmox

- Accorder **VM.Audit** et **VM.Console** sur les VM autorisées (par exemple `/vms` avec propagation pour toutes les VM). Un jeton avec séparation des privilèges doit avoir les ACL appropriées, et son compte propriétaire aussi. La liste peut être vide si les droits de consultation sont absents.
- Dans **VM → Matériel → Affichage**, sélectionner un affichage compatible **SPICE**. Appliquer les changements nécessaires côté Proxmox et démarrer la VM.
- Autoriser l’accès réseau à l’API HTTPS (habituellement TCP 8006) et au proxy SPICE (habituellement TCP 3128). L’accès à l’interface Web seule ne garantit pas l’accès à la console SPICE.

HostGates propose le démarrage, l’arrêt propre, le redémarrage et la coupure forcée avec confirmation (droit VM.PowerMgmt). Les conteneurs LXC sont affichés et ouvrent une console terminal intégrée via WebSocket, sans SPICE ni SSH. Voir le [guide Hyper-V / Proxmox / LXC](HYPERV_PROXMOX.md).

## Utilisation

Déplier un serveur pour charger ses VM. **Actualiser** recharge la liste ; la recherche du panneau filtre les VM par nom, identifiant ou nœud. Un rond vert indique une machine allumée, rouge une machine arrêtée, noir un modèle ou une machine désactivée et orange un état inconnu ou intermédiaire. Double-cliquer sur une VM démarrée, ou utiliser **clic droit → Ouvrir la console SPICE**.

Chaque ouverture relit le nœud hébergeant la VM et demande un nouveau ticket : les migrations entre nœuds sont prises en compte. Proxmox rend le ticket SPICE utilisable pendant une courte durée (30 secondes dans l’implémentation consultée), d’où le lancement immédiat du client. Le ticket n’est jamais sauvegardé comme mot de passe permanent.

L’onglet HostGates suit le processus externe et permet son arrêt. Fermer cet onglet ou verrouiller le coffre ferme le processus lancé. **Supprimer** un serveur retire uniquement sa configuration locale et son secret, sans supprimer les VM. Les serveurs sont repliés au démarrage.

## Sécurité et dépannage

Les requêtes API utilisent HTTPS et refusent les redirections HTTP. La validation du certificat est activée par défaut ; l’exception explicite décrite ci-dessus ne concerne que le serveur configuré. Les mots de passe/jetons ne sont ni dans les URL ni dans les arguments des processus. La case ne désactive pas directement la vérification de remote-viewer. Toutefois, lorsque l’API n’est pas authentifiée, un intermédiaire pourrait remplacer la configuration SPICE qu’elle fournit. La configuration SPICE conserve le CA et le sujet du certificat fournis par Proxmox, impose le port TLS et n’accepte qu’une liste restreinte de propriétés.

Un fichier temporaire `.vv` contient le ticket en clair avec des droits limités au compte utilisateur (0600 sous Linux, ACL privée sous Windows). `remote-viewer` est invité à le supprimer après lecture ; HostGates le retire aussi lorsque l’onglet est fermé. Une fermeture brutale peut laisser un fichier résiduel, dont le ticket expire. Voir `securite.txt`. Ne pas partager de fichiers `.vv` ; ils sont exclus de Git.

- **Client introuvable** : configurer le chemin complet de remote-viewer.
- **Connexion HTTPS impossible** : vérifier URL, VPN, DNS, certificat CA et nom du serveur.
- **HTTP 401 [POST /access/ticket] — Connexion refusée** : vérifier le compte complet (`root@pam` pour root avec Linux PAM), le mode Mot de passe et les exigences MFA. Dans Modifier, retaper le secret : un champ vide conserve celui déjà enregistré. Vérifier que le même compte fonctionne dans l’interface Web sur la même adresse. L’API masque volontairement le motif précis du refus ; les journaux d’authentification Proxmox peuvent fournir ce motif à l’administrateur.
- **HTTP 401 [GET /cluster/resources] — Ticket refusé** : l’API a fourni un ticket, puis l’a refusé lors du chargement des VM. Vérifier les cookies transmis par le reverse proxy et les journaux serveur. Comparer avec l’adresse directe HTTPS du nœud en port 8006 si elle est accessible.
- **HTTP 401 — Jeton API refusé** : vérifier le realm, le nom du jeton, son secret et son expiration ; ne pas mettre le préfixe `PVEAPIToken=` dans l’identifiant. Vérifier la transmission de l’en-tête Authorization par un éventuel reverse proxy.
- **Droits insuffisants / liste vide** : vérifier les ACL du compte et du jeton.
- **Affichage SPICE absent** : modifier l’affichage côté Proxmox, puis actualiser/démarrer la VM.
- **Le client s’ouvre sans connexion** : vérifier le proxy SPICE et le port 3128. Relancer pour obtenir un ticket neuf. Les erreurs propres au client apparaissent dans sa fenêtre.

## Validation et périmètre

Tests automatisés avec une API HTTPS simulée : mot de passe, cookie/CSRF, jeton API, renouvellement de tickets, résolution du nœud courant, erreurs HTTP et refus des redirections, validation des propriétés `.vv`. Aucun serveur Proxmox réel ni console graphique distante n’était disponible pour la validation : le parcours complet reste à tester sur votre infrastructure.

SPICE est également utilisé avec QEMU/KVM et des plateformes fondées sur libvirt. Cette intégration d’inventaire et d’authentification cible spécifiquement l’API Proxmox ; elle n’ajoute pas encore de connexion générique libvirt/oVirt.

Références : [API Proxmox](https://pve.proxmox.com/pve-docs/api-viewer/), [implémentation spiceproxy](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm), [configuration remote-viewer Proxmox](https://github.com/proxmox/pve-access-control/blob/master/src/PVE/AccessControl.pm), [proxy SPICE](https://www.spice-space.org/spice-proxy.html).
