# Hôtes Hyper-V, clusters et Proxmox

## Ajouter un serveur

Dans le panneau latéral **Proxmox / Hyper-V**, choisissez **Ajouter un serveur**, puis le type. Les profils existants restent des profils Proxmox. Les mots de passe peuvent être conservés dans le coffre commun ; déverrouillez-le avant de les enregistrer.

## Hyper-V (Windows)

- Renseignez un **hôte**, par exemple `https://hv01.example.test:5986`, et un compte `DOMAINE\utilisateur` ou `utilisateur@domaine`. Pour un cluster, cochez **Lister tout le cluster depuis cet hôte**. L’adresse est celle d’un nœud joignable, pas nécessairement le nom virtuel du cluster.
- Les hôtes doivent proposer PowerShell distant / WinRM, avec un certificat approuvé par Windows pour HTTPS. Le certificat doit couvrir le nom DNS utilisé. HostGates ne modifie ni WinRM, ni TrustedHosts, ni les politiques du système.
- Le compte doit disposer des droits de session distante et de gestion Hyper-V. Le module PowerShell Hyper-V est nécessaire sur chaque hôte ; FailoverClusters est nécessaire sur le nœud d’entrée d’un cluster. Tous les nœuds doivent être joignables avec le même compte et le même port WinRM.
- `http://hv01.example.test:5985` est disponible pour une infrastructure WinRM existante avec authentification Negotiate (Kerberos en domaine lorsque possible). Basic n’est pas utilisé. Privilégiez HTTPS, notamment hors domaine. La validation est activée par défaut. La case **Ne pas vérifier le certificat HTTPS** permet une exception limitée au profil, sans modifier le magasin de confiance Windows.
- Le double-clic ouvre **VMConnect dans une fenêtre externe**. Installez les outils de gestion Hyper-V Windows. Le chemin `vmconnect.exe` peut être sélectionné dans le profil ; par défaut, HostGates cherche dans `Windows/System32`.
- **VMConnect utilise le compte Windows courant**, qui doit être autorisé sur l’hôte, même si un autre compte est enregistré pour l’inventaire et les commandes. Aucun mot de passe n’est placé dans ses arguments.
- Sous Linux, le même inventaire et les mêmes actions utilisent le transport WinRM natif Rust et les consoles FreeRDP décrits ci-dessous. VMConnect reste propre à Windows.

Les machines sont ciblées par leur GUID. HostGates relit l’inventaire avant l’action pour retrouver le propriétaire actuel après une migration. En cas de nœud inaccessible, la lecture échoue explicitement : les anciens états ne sont pas considérés comme fiables et les actions sont désactivées jusqu’à une actualisation réussie.


## Hyper-V depuis Linux

Le même profil et les mêmes actions sont disponibles sous Linux. Le transport **WinRM est intégré en Rust**, avec `winrm-rs 2.0.0` : aucun Python, pypsrp ou PowerShell local n’est nécessaire. Ce remplacement couvre les opérations HostGates (inventaire, clusters, alimentation), pas toute l’API pypsrp. Il utilise WinRS pour exécuter les scripts PowerShell fixes sur les hôtes Windows. **NTLM sur HTTP ou HTTPS** : les domaines interdisant NTLM ne sont pas encore pris en charge sous Linux.

### Dépendances du poste

Installer **FreeRDP** pour les consoles uniquement : selon la distribution, `freerdp2-x11`, `freerdp3-x11` ou le paquet FreeRDP disponible dans ses dépôts. La gestion des VM n’a pas besoin de client externe.

Dans **Modifier le serveur → Hyper-V**, sélectionner `xfreerdp3` ou `xfreerdp` dans **FreeRDP (Hyper-V)**, ou laisser vide pour chercher dans le PATH. Le chemin VMConnect Windows reste conservé séparément. L’ancien chemin Python des profils est ignoré. Le fichier CA PEM du profil reste utilisé.

### Configuration des hôtes

- WinRM **HTTPS** est conseillé, généralement `https://hv01.example.test:5986`, doit être configuré sur chaque nœud avec un certificat dont le nom correspond au DNS utilisé. Le menu **Transport WinRM** permet aussi HTTP, habituellement `http://hv01.example.test:5985`. Le port reste libre : saisir `http://hv01.example.test:5895` si votre écouteur utilise 5895. Un changement de transport adapte seulement les ports standard 5985/5986 et préserve les ports personnalisés.
- Renseigner `DOMAINE\utilisateur`, `utilisateur@domaine` ou `HOTE\utilisateur` pour un compte local. Le service WinRM doit autoriser les shells distants WinRS et PowerShell. Le compte doit disposer des droits WinRM et Hyper-V ; les mêmes modules serveur que sous Windows sont nécessaires.
- Pour une autorité privée, choisir le fichier **Certificat CA PEM** dans le profil. Ce fichier concerne WinRM, et s’applique à chaque nœud du cluster. La case **Ne pas vérifier le certificat HTTPS** ignore explicitement les erreurs de certificat et le fichier CA pour ce profil, sur tous ses nœuds. L’identité du serveur n’est alors plus vérifiée ; les identifiants peuvent être interceptés. En HTTP, les options de certificat sont inactives et les messages SOAP sont obligatoirement signés/chiffrés par NTLM (sealing). Ne pas activer Basic ni AllowUnencrypted. HTTPS vérifié reste préférable pour authentifier le serveur. Cette exception concerne WinRM, pas le certificat de la console FreeRDP.
- Autoriser l’accès au port **2179** du nœud pour la console VMConnect/FreeRDP et les droits de console Hyper-V du compte. Le certificat de ce service est distinct de celui de WinRM : FreeRDP utilise son magasin de confiance/système. Faire approuver ce certificat par l’administrateur ; le CA WinRM du profil n’est pas transmis à FreeRDP. Les certificats non approuvés sont refusés sans acceptation automatique.
- En cluster, tous les nœuds doivent être accessibles directement, avec le même compte et port WinRM. HostGates inventorie les nœuds depuis le poste Linux, sans déléguer le mot de passe d’un hôte vers un autre. L’hôte propriétaire est relu avant une console ou une action. Une migration pendant l’opération peut encore imposer une actualisation.

Le double-clic ouvre **une fenêtre FreeRDP externe**, suivie par un onglet HostGates. Il utilise l’hôte et le GUID Hyper-V, sans dépendre d’un serveur RDP ou de l’adresse IP de la VM. Le mot de passe du profil est transmis par un pipe privé, jamais par `/p:`. Fermer l’onglet ou verrouiller le coffre ferme le processus console. Une erreur immédiate peut signaler un certificat de console non approuvé, un port fermé, des droits insuffisants ou un client FreeRDP incompatible. Pour diagnostiquer manuellement, utiliser FreeRDP avec `/v:hote:2179 /vmconnect:GUID /u:utilisateur /d:DOMAINE` et saisir le mot de passe dans son invite, sans l’ajouter à la commande.

Les états et actions d’alimentation suivent les mêmes règles qu’avec Windows. Aucune commande ayant échoué n’est relancée automatiquement. Après un délai dépassé, une action distante peut encore se terminer : actualiser avant de recommencer.

Tests : `cargo test --lib hyperv::` vérifie le routage, les actions, le refus des certificats non approuvés et des redirections avec un serveur HTTPS local. Ces tests ne remplacent pas une validation de l’authentification et des consoles sur votre infrastructure Hyper-V.

Références : [winrm-rs](https://github.com/muchiny/winrm-rs), [FreeRDP](https://github.com/FreeRDP/FreeRDP/wiki/CommandLineInterface).

## États et commandes

Dépliez un serveur pour voir les machines, leur type, leur hôte et leur état : allumée, arrêtée, en pause, sauvegardée, en transition, indisponible ou modèle. Ce sont les états de la **dernière actualisation**, pas une surveillance continue. Utilisez **Actualiser** pour les relire. Un état inconnu ne signifie pas que la machine est arrêtée ; « désactivée » n’est pas un état universel commun aux deux hyperviseurs.

Le clic droit propose :

- **Démarrer** : machine arrêtée (ou état sauvegardé Hyper-V).
- **Arrêter proprement** : demande au système invité de s’arrêter.
- **Redémarrer proprement** : commande de redémarrage Proxmox ; sous Hyper-V, arrêt propre puis démarrage uniquement après constat de l’état Off.
- **Couper l’alimentation** : arrêt forcé, équivalent à une coupure électrique. Les données non enregistrées peuvent être perdues.

Chaque action demande confirmation avec le nom du serveur et de la machine. Les modèles, états inconnus et transitions ne proposent pas d’actions incompatibles. Les opérations visent une seule machine, jamais un groupe sélectionné par nom. Un arrêt propre échoué n’est jamais remplacé automatiquement par une coupure.

Les actions Proxmox nécessitent **VM.PowerMgmt**, l’inventaire **VM.Audit**, et SPICE **VM.Console**. HostGates suit la tâche Proxmox avant d’actualiser l’état. Une erreur indique un échec ou un résultat non confirmé ; consultez le journal Proxmox. Hyper-V nécessite les services d’intégration pour l’arrêt propre de l’invité.

En cas de délai dépassé, ne relancez pas aveuglément : une commande déjà acceptée peut continuer côté serveur. Verrouiller ou fermer HostGates n’annule pas une action distante déjà soumise. Il n’y a pas de nouvelle tentative automatique des mutations.

## Console terminal des conteneurs LXC

Les conteneurs apparaissent avec les VM Proxmox. Double-cliquez un conteneur allumé ou choisissez **Ouvrir la console du conteneur** : le terminal s’ouvre dans un onglet HostGates, via les API Proxmox `termproxy` et `vncwebsocket`, comme la console xterm.js de l’interface Web.

Aucun serveur SSH ni adresse IP du conteneur n’est nécessaire. Le profil Proxmox authentifie l’accès à la console et doit disposer de **VM.Console** (ainsi que VM.Audit pour l’inventaire). Selon la configuration console du conteneur, Linux peut afficher sa propre invite de connexion, comme dans Proxmox. HostGates n’y saisit pas automatiquement le mot de passe du compte Proxmox.

Le redimensionnement, les touches de contrôle, le collage et le défilement utilisent le terminal intégré. Il n’y a pas de SFTP pour cette console. Les anciennes sessions SSH créées précédemment restent disponibles séparément ; elles ne sont plus utilisées par le double-clic sur les LXC.

Un reverse proxy doit laisser passer les WebSockets. Le certificat CA ou l’exception HTTPS du profil s’applique aussi à cette connexion WSS. Certains anciens serveurs n’autorisent pas les consoles avec un jeton API : utilisez alors l’authentification Proxmox par mot de passe. Un ticket temporaire est créé à chaque ouverture et n’est pas enregistré dans le profil. Fermer l’onglet ou verrouiller le coffre ferme la connexion locale ; cela n’arrête pas le conteneur et ne garantit pas la fermeture de son shell persistant côté Proxmox.

## Couleurs des états

- Rond vert : machine allumée.
- Rond rouge : machine arrêtée.
- Rond noir : machine désactivée ou modèle.
- Rond orange : état inconnu, inventaire périmé ou état intermédiaire (pause, sauvegarde, transition). Le libellé précise l’état connu.

Le contour permet de distinguer aussi le rond noir sur un thème sombre. Les états restent ceux de la dernière actualisation pendant l’ouverture d’une console ou un rafraîchissement en cours. Une erreur de console SPICE/LXC ne modifie pas les états et ne bloque pas les autres machines. Un échec de lecture de l’inventaire rend les états inconnus ; une commande d’alimentation en cours ou de résultat incertain ne concerne que sa machine cible.

## Vérification

Les tests HTTPS utilisent un serveur simulé local (routage QEMU/LXC, migration, tâches, erreurs, absence de coupure de secours, authentification et transport de console WebSocket, redimensionnement, fermeture et certificats). `tests/hyperv_mock.ps1` remplace entièrement les commandes distantes par des doubles de test et vérifie les actions et le propriétaire d’une VM de cluster. Aucune VM réelle n’est modifiée par ces tests. La validation dans une infrastructure Hyper-V réelle reste nécessaire.

Références : [gestion distante Hyper-V](https://learn.microsoft.com/en-us/windows-server/virtualization/hyper-v/manage/remotely-manage-hyper-v-hosts), [Stop-VM](https://learn.microsoft.com/en-us/powershell/module/hyper-v/stop-vm), [API Proxmox](https://pve.proxmox.com/pve-docs/api-viewer/).
