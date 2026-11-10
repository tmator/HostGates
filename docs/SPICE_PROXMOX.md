# Proxmox et consoles SPICE

HostGates affiche plusieurs serveurs Proxmox dans le panneau latéral, avec les VM QEMU sous chaque serveur dépliable. Un double-clic ouvre la console SPICE **dans un onglet HostGates**. Le moteur est la bibliothèque native `libspice-client-glib` (spice-gtk), appelée depuis Rust via son ABI C. Aucun processus remote-viewer ni fichier `.vv` n’est utilisé.

## Configuration

1. Installer les bibliothèques natives 64 bits : **Virt Viewer** sous Windows ([téléchargements SPICE](https://www.spice-space.org/download.html)), `sudo apt install libspice-client-glib-2.0-8` sous Debian/Ubuntu, ou `sudo dnf install spice-glib` sous Fedora/Rocky (dépôts appropriés nécessaires). Les paquets HostGates DEB/RPM déclarent cette dépendance.
2. **Paramètres → Applications externes → SPICE** : laisser vide pour détecter VirtViewer dans Program Files ou les bibliothèques système Linux. On peut choisir `libspice-client-glib-2.0-8.dll`, `libspice-client-glib-2.0.so.8`, leur dossier, ou conserver l’ancien chemin remote-viewer. Sous Windows, toutes les DLL dépendantes doivent rester à côté de la DLL SPICE. Un dossier `spice-runtime` à côté de HostGates est également détecté. Redémarrer HostGates après un changement de bibliothèque. HostGates ne redistribue pas automatiquement ces bibliothèques ; leurs licences et mises à jour restent celles de leur fournisseur.
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

L’onglet affiche le bureau, transmet le clavier physique et la souris, et propose **Plein écran** (sortie : bouton ou Ctrl+Alt+Entrée). Cliquer dans le bureau pour lui donner le focus. Changer d’onglet libère les touches maintenues. Fermer cet onglet, Arrêter ou verrouiller le coffre déconnecte la session. **Supprimer** un serveur retire uniquement sa configuration locale et son secret, sans supprimer les VM. Les serveurs sont repliés au démarrage.

## Sécurité et dépannage

Les requêtes API utilisent HTTPS et refusent les redirections HTTP. La validation du certificat est activée par défaut ; l’exception explicite décrite ci-dessus ne concerne que le serveur configuré. Les mots de passe/jetons ne sont ni dans les URL ni dans les arguments des processus. La case ne désactive pas directement la vérification TLS de SPICE. Toutefois, lorsque l’API n’est pas authentifiée, un intermédiaire pourrait remplacer la configuration SPICE qu’elle fournit. La configuration SPICE conserve le CA et le sujet du certificat fournis par Proxmox, omet le port non chiffré (y compris la valeur invalide `port=-1`), impose `secure-channels=all` et le port TLS et n’accepte qu’une liste restreinte de propriétés.

Le ticket et le CA passent directement en mémoire à la bibliothèque native. **L’absence de fichier `.vv` est normale.** Les tickets ne sont ni enregistrés dans le coffre ni passés en arguments. Ne pas activer les traces de débogage SPICE en production : les bibliothèques tierces peuvent journaliser des métadonnées de connexion.

- **Bibliothèque introuvable/incompatible** : installer la bibliothèque et ses dépendances 64 bits ; vérifier son chemin dans les paramètres.
- **Connexion HTTPS impossible** : vérifier URL, VPN, DNS, certificat CA et nom du serveur.
- **HTTP 401 [POST /access/ticket] — Connexion refusée** : vérifier le compte complet (`root@pam` pour root avec Linux PAM), le mode Mot de passe et les exigences MFA. Dans Modifier, retaper le secret : un champ vide conserve celui déjà enregistré. Vérifier que le même compte fonctionne dans l’interface Web sur la même adresse. L’API masque volontairement le motif précis du refus ; les journaux d’authentification Proxmox peuvent fournir ce motif à l’administrateur.
- **HTTP 401 [GET /cluster/resources] — Ticket refusé** : l’API a fourni un ticket, puis l’a refusé lors du chargement des VM. Vérifier les cookies transmis par le reverse proxy et les journaux serveur. Comparer avec l’adresse directe HTTPS du nœud en port 8006 si elle est accessible.
- **HTTP 401 — Jeton API refusé** : vérifier le realm, le nom du jeton, son secret et son expiration ; ne pas mettre le préfixe `PVEAPIToken=` dans l’identifiant. Vérifier la transmission de l’en-tête Authorization par un éventuel reverse proxy.
- **Droits insuffisants / liste vide** : vérifier les ACL du compte et du jeton.
- **Affichage SPICE absent** : modifier l’affichage côté Proxmox, puis actualiser/démarrer la VM.
- **Connexion proxy/serveur impossible** : vérifier l’accès au proxy SPICE (habituellement 3128), le VPN et l’hôte configuré. Le client se connecte directement, sans test TCP préalable.
- **Certificat TLS refusé** : vérifier le CA et le sujet renvoyés par Proxmox. La vérification SPICE reste obligatoire.
- **Ticket refusé/expiré** : relancer la console pour obtenir un nouveau ticket.

## Validation et périmètre

Tests automatisés : API HTTPS simulée, validation des paramètres, conversion bornée des images, et test d’intégration avec QEMU local, proxy HTTP CONNECT, ticket synthétique et TLS. Ce dernier vérifie la réception du bureau, la déconnexion et le refus d’un mauvais sujet ou d’un CA inconnu. Aucun serveur Proxmox de production n’est utilisé par ces tests.

Cette première intégration affiche le premier écran (jusqu’à 4096 × 4096). Le clavier utilise la disposition configurée dans le système invité ; pas d’injection Unicode/IME. Audio, USB, cartes à puce, presse-papiers et multi-écrans SPICE ne sont pas implémentés. Pour une souris absolue confortable, activer une tablette et/ou installer l’agent SPICE dans la VM ; le mode relatif reste un mode de secours.

SPICE est également utilisé avec QEMU/KVM et des plateformes fondées sur libvirt. Cette intégration d’inventaire et d’authentification cible spécifiquement l’API Proxmox ; elle n’ajoute pas encore de connexion générique libvirt/oVirt.

Références : [API Proxmox](https://pve.proxmox.com/pve-docs/api-viewer/), [implémentation spiceproxy](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm), [configuration remote-viewer Proxmox](https://github.com/proxmox/pve-access-control/blob/master/src/PVE/AccessControl.pm), [proxy SPICE](https://www.spice-space.org/spice-proxy.html).

## Reproduire le test SPICE local

Sous Linux/WSL, installer QEMU (`qemu-system-x86`) et OpenSSL, puis lancer `python3 scripts/spice-test-server.py`. Le script démarre une VM BIOS sans disque, un proxy local et un certificat de test. Il affiche le chemin du fichier JSON synthétique. Dans un autre terminal, définir `HOSTGATES_SPICE_FIXTURE` vers ce chemin puis lancer `cargo test --lib native_qemu -- --ignored`. Pour tester le client Windows depuis un serveur WSL, utiliser `--directory` avec un dossier partagé et donner à la variable Windows le chemin Windows du JSON. Ctrl+C ferme le banc de test. Aucun serveur réel, coffre ou ticket de production n’est nécessaire.
