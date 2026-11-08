# Hôtes Hyper-V, clusters et Proxmox

## Ajouter un serveur

Dans le panneau latéral **Proxmox / Hyper-V**, choisissez **Ajouter un serveur**, puis le type. Les profils existants restent des profils Proxmox. Les mots de passe peuvent être conservés dans le coffre commun ; déverrouillez-le avant de les enregistrer.

## Hyper-V (Windows)

- Renseignez un **hôte**, par exemple `https://hv01.example.test:5986`, et un compte `DOMAINE\utilisateur` ou `utilisateur@domaine`. Pour un cluster, cochez **Lister tout le cluster depuis cet hôte**. L’adresse est celle d’un nœud joignable, pas nécessairement le nom virtuel du cluster.
- Les hôtes doivent proposer PowerShell distant / WinRM, avec un certificat approuvé par Windows pour HTTPS. Le certificat doit couvrir le nom DNS utilisé. HostGates ne modifie ni WinRM, ni TrustedHosts, ni les politiques du système.
- Le compte doit disposer des droits de session distante et de gestion Hyper-V. Le module PowerShell Hyper-V est nécessaire sur chaque hôte ; FailoverClusters est nécessaire sur le nœud d’entrée d’un cluster. Tous les nœuds doivent être joignables avec le même compte et le même port WinRM.
- `http://hv01.example.test:5985` est disponible pour une infrastructure WinRM existante avec authentification Negotiate (Kerberos en domaine lorsque possible). Basic n’est pas utilisé. Privilégiez HTTPS, notamment hors domaine. Aucune exception de certificat WinRM n’est créée par HostGates.
- Le double-clic ouvre **VMConnect dans une fenêtre externe**. Installez les outils de gestion Hyper-V Windows. Le chemin `vmconnect.exe` peut être sélectionné dans le profil ; par défaut, HostGates cherche dans `Windows/System32`.
- **VMConnect utilise le compte Windows courant**, qui doit être autorisé sur l’hôte, même si un autre compte est enregistré pour l’inventaire et les commandes. Aucun mot de passe n’est placé dans ses arguments.
- L’inventaire, les commandes et VMConnect Hyper-V sont actuellement disponibles sous **Windows uniquement**. Les profils restent lisibles sous Linux ; le connecteur signale cette limite. Proxmox reste utilisable sous Windows et Linux.

Les machines sont ciblées par leur GUID. HostGates relit l’inventaire avant l’action pour retrouver le propriétaire actuel après une migration. En cas de nœud inaccessible, la lecture échoue explicitement : les anciens états ne sont pas considérés comme fiables et les actions sont désactivées jusqu’à une actualisation réussie.

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
