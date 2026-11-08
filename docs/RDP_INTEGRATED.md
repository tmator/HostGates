# RDP intégré dans HostGates

Les sessions RDP s'affichent désormais dans un onglet, avec le moteur Rust IronRDP. Le client externe reste disponible via clic droit sur la session → Options SSH / tunnel → Utiliser le client RDP externe.

## Connexion

Ouvrez une session RDP, vérifiez utilisateur et domaine, puis saisissez le mot de passe et cliquez sur Connexion (ou Entrée dans le champ du mot de passe). Un secret déjà enregistré dans le coffre est prérempli après déverrouillage. Un domaine vide est autorisé. Le mot de passe saisi dans cet onglet n'est pas enregistré automatiquement : utilisez l'éditeur de session pour le conserver dans le coffre.

Le bureau est affiché en 1280 × 720 et ajusté visuellement à la taille du volet. Cliquez dans le bureau pour lui donner le clavier. Les caractères saisis sont transmis en Unicode ; navigation, touches de fonction, raccourcis Ctrl/Alt et souris sont pris en charge. Un bouton Ctrl+Alt+Suppr envoie cette combinaison au serveur. Le collage transmet du texte (maximum 4096 unités UTF-16 par événement).

## Certificat et tunnels

La chaîne de certification et le nom du serveur sont vérifiés avant l'authentification. En cas de certificat autosigné/invalide, un écran présente l'empreinte SHA-256 : vérifiez-la par un autre moyen auprès de l'administrateur, puis acceptez explicitement pour cette connexion uniquement, ou refusez. Entrée ne valide pas ce consentement. Aucun bouton ne désactive globalement la vérification TLS. Une fermeture ou annulation interrompt aussi une attente de certificat.

Les tunnels SSH continuent d'être gérés par HostGates. La connexion TCP passe par le tunnel ; la vérification TLS conserve le nom du serveur RDP d'origine. L'acceptation d'un certificat n'est pas enregistrée dans preferences.json.

## Portée de cette intégration

Bureau distant, saisie clavier/souris, défilement, affichage dans les volets, arrêt/reconnexion et collage de texte. Pas de redirection audio, disque, imprimante, carte à puce, presse-papiers bidirectionnel, RemoteApp ou SSO Kerberos garanti dans cette première intégration. Utiliser le client externe pour ces fonctions et les scénarios non pris en charge.

Les images et entrées transitent par des files bornées. La fermeture arrête la tâche réseau et libère ses ressources ; le moteur RDP peut conserver des copies internes des identifiants pendant la connexion. Aucun secret n'est transmis en argument de processus.

## Sources et licences

IronRDP client 0.1.0 est intégré sous MIT/Apache-2.0 dans vendor/ironrdp-client. Adaptations locales documentées dans HOSTGATES_PATCH.md : validation TLS stricte, décision explicite par empreinte, nom d'origine derrière tunnel et file d'entrée bornée. Les autres composants IronRDP sont verrouillés dans Cargo.lock. Les licences upstream sont conservées.

## Validation du 31 octobre 2026

Compilation et analyse Clippy sans avertissements sous Windows et Linux (WSL Ubuntu 22.04). Suite standard : 79 tests réussis sous Windows, 77 sous Linux ; les tests nécessitant des serveurs dédiés restent ignorés par défaut. Test RDP séparé réussi sur les deux plateformes contre un serveur xrdp isolé : refus du certificat, acceptation explicite, authentification, image du bureau et exécution effective d’une commande par saisie Unicode. Le test RDP est ignoré par défaut et nécessite sa fixture locale sur 127.0.0.1:33989.

La connexion à un serveur Windows imposant NLA, l’utilisation sur un bureau Linux natif et les environnements de domaine restent à valider sur les machines cibles. Le binaire Windows fourni dans target/debug est une compilation de test, sans nouvelle publication GitHub.

## Passerelles RD Gateway et import .rdp

Fichier → Importer MobaXterm / RDP permet de sélectionner un fichier .rdp (UTF-8 ou UTF-16 avec BOM). L’aperçu reprend l’adresse, le port, l’utilisateur, le domaine et la passerelle RD Gateway. Les mots de passe, signatures, réglages d’affichage et redirections ne sont pas importés. Les fichiers RemoteApp ou demandant une commande au démarrage sont refusés. Il ne s’agit pas d’un import intégral de toutes les propriétés Microsoft.

Dans les options de la session, activez « Utiliser une passerelle RD Gateway », renseignez son adresse (port HTTPS 443 par défaut) et éventuellement son utilisateur. Laissez « Utiliser le client RDP externe » décoché pour ouvrir le bureau dans un onglet IronRDP, sous Windows ou Linux. Les sessions précédemment importées réutilisent leurs paramètres de passerelle.

Dans l’onglet, renseignez les identifiants du poste distant. Décochez la réutilisation seulement si la passerelle utilise un autre compte. La case « Utiliser les identifiants du poste pour la passerelle » est cochée par défaut et réutilise utilisateur, domaine et mot de passe, y compris le secret du poste prérempli depuis le coffre. Le mot de passe distinct de la passerelle est conservé seulement en mémoire pendant la connexion, puis effacé du formulaire ; il n’est pas enregistré automatiquement dans le coffre. Les identifiants peuvent être de la forme DOMAINE\utilisateur ou utilisateur@domaine.

Les certificats HTTPS de la passerelle et RDP du poste sont vérifiés séparément. Une exception demande une acceptation explicite pour le serveur affiché ; elle n’est pas mémorisée. Les transports WebSocket et HTTP à deux canaux sont disponibles, avec authentification HTTP Negotiate/NTLM ou Basic sur TLS vérifié. Le port du poste distant est conservé. Aucun client MSTSC/FreeRDP n’est lancé dans ce mode.

Limites : pas de carte à puce, de dialogue MFA interactif spécifique dans cette intégration. Le mode importé « utiliser la passerelle si la connexion directe échoue » utilise toujours la passerelle dans les onglets, sans essai direct ; le formulaire le signale. Pour conserver exactement ce comportement, ou utiliser les fonctions non prises en charge, cochez le client externe. Un tunnel SSH et une RD Gateway ne peuvent pas être combinés.

La configuration est conservée dans preferences.json, à sauvegarder avec remotehub.db. Les exports MobaXterm/CSV ne contiennent pas RD Gateway. Les tests automatisés couvrent l’import, la sélection d’un onglet intégré, les identités séparées, le port cible, les protocoles et la validation TLS. Ils ne remplacent pas un essai sur votre passerelle Windows et sa politique d’authentification.

Le transport utilise ironrdp-mstsgu 0.1.0 adapté localement (voir vendor/ironrdp-mstsgu/HOSTGATES_PATCH.md). Les tests upstream utilisent uniquement des certificats de test publics et des serveurs de boucle locale.

### Vérifications RD Gateway

Le 1er novembre 2026 : tests applicatifs Windows réussis, dont sélection du client intégré après import, séparation des identifiants et effacement des secrets du formulaire. Les 31 tests ciblés du transport sous Linux couvrent les échanges HTTP, les premiers échanges NTLM/Negotiate, le port cible et les certificats des transports WebSocket / double HTTP. Un certificat refusé bloque la connexion avant les identifiants HTTP. Ces essais utilisent des serveurs locaux simulés, pas une RD Gateway Windows de production.

Pour tester votre session existante, fermez puis relancez HostGates avec le nouvel exécutable, sélectionnez « RDP intégré (IronRDP) » dans les options, puis ouvrez la session. Le formulaire « Connexion RDP intégrée » doit apparaître dans l’onglet, avec les champs de la passerelle. En cas d’échec, recopiez le message affiché dans l’onglet sans mot de passe.

L’import `.rdp` suit le choix global du client RDP pour la connexion enregistrée. En choisissant « Remplacer » pour un doublon, l’ancien choix du client externe et l’éventuelle passerelle SSH sont réinitialisés ; la RD Gateway du fichier est reprise. Choisir « Ignorer » conserve la session existante inchangée. Les préférences sont conservées au redémarrage.

Si une passerelle coupe la connexion pendant la négociation HTTP/WebSocket, HostGates effectue une seule nouvelle tentative en HTTPS classique à deux canaux, avec de nouvelles connexions TLS vérifiées. Un refus explicite d’authentification ou une erreur de certificat ne déclenche pas cette tentative. Cette compatibilité est couverte par un serveur de test local ; une coupure peut aussi venir d’un proxy, d’une politique serveur ou d’une authentification refusée sans réponse HTTP.

### Passerelles nécessitant RPC sur HTTPS

Le transport RPCH (MS-RPCH / TS Proxy) est désormais raccordé au bureau intégré. Il vérifie TLS, authentifie les canaux HTTPS et négocie l’intégrité des paquets RPC. Après une coupure des tentatives HTTP RD Gateway, il est essayé automatiquement ; un refus explicite de mot de passe ou de certificat ne déclenche pas ce changement de transport.

Pour une passerelle qui fonctionne dans MSTSC mais ferme les échanges HTTP dans HostGates, cochez « Utiliser directement RPC sur HTTPS (RPCH) » dans les options de la session, puis reconnectez-la. Ce réglage évite les essais HTTP précédents et persiste au redémarrage. Sélectionnez le client intégré dans les options de la session. Les erreurs de ce transport commencent par « GW RPCH connect ».

Des tests locaux couvrent les échanges TS Proxy avec authentification NTLM, le refus de paquets non signés/altérés, le renouvellement des canaux, le transfert et la fermeture. La compatibilité avec la passerelle de déploiement reste à vérifier.

### Message « Data PDU » pendant l’activation

Le connecteur distingue désormais les notifications Status Info (quatre octets)
et Set Error Info sans erreur, qui laissent l’activation continuer, des refus
du serveur. Un refus est affiché avec son code symbolique et sa description,
au lieu du message générique « got Data PDU ». Les autres messages inattendus
restent rejetés, avec leur type précis. Aucun contenu de session ni identifiant
n’est ajouté au diagnostic. Trois tests couvrent notifications, refus et
messages inattendus ou malformés. Cette correction ne prouve pas à elle seule
que le serveur de déploiement acceptera la connexion.

### Collections RDS et courtier de connexions

L’import conserve désormais `loadbalanceinfo` dans les préférences de la session
et le transmet dans la demande X.224, tel quel avec le terminateur CRLF. Les
routes de collection `tsv://MS Terminal Services Plugin.1.…` ne reçoivent pas
de préfixe `Cookie: msts=` supplémentaire. Les valeurs non ASCII, contenant
des caractères de contrôle ou dépassant 200 octets sont rejetées.

Réimportez le fichier `.rdp` et choisissez **Remplacer** pour mettre à jour une
connexion existante ; **Ignorer** conserve ses paramètres précédents. Les tests
couvrent ces deux cas, la sauvegarde/relecture et les octets de routage émis.

Le client intégré annonce maintenant la capacité de redirection RDS (version 4),
traite les Enhanced Security Server Redirection PDUs avant ou après activation,
et reconnecte la destination choisie via la même passerelle. Le port cible est
conservé. `use redirection server name:i:1` donne priorité au FQDN fourni par le
courtier, puis au nom NetBIOS ; sinon, priorité à son adresse réseau.
Le certificat TLS est vérifié à nouveau pour cette destination. Une nouvelle
exception de certificat nécessite une acceptation explicite.

La reconnexion transmet l’identifiant de session et le cookie opaque fourni par
le courtier. Ce cookie reste distinct du mot de passe CredSSP et du secret de la
passerelle. Les boucles et plus de quatre redirections sont refusées. Les paquets
malformés, modes carte à puce et RDSTLS ne sont pas acceptés. La redirection est
désactivée avec un tunnel SSH à destination fixe, pour ne pas sortir du tunnel.
Ces comportements sont testés avec des paquets synthétiques ; la validation sur
une ferme RDS Windows réelle reste nécessaire. Un refus ServerInsufficientPrivileges
reste un refus du serveur et ne déclenche aucun contournement des droits.

### Souris et plein écran

Le curseur RDP est affiché localement à partir de sa forme et de son point actif,
sans redessiner toute l’image du bureau à chaque mouvement. Les mouvements absolus
consécutifs en attente sont regroupés ; aucun clic, relâchement ou événement
clavier n’est supprimé ni déplacé. Les entrées sont envoyées par lots bornés.
La latence de la connexion et la charge du serveur peuvent encore ralentir les
réponses du bureau distant.

Cliquez sur **Plein écran** dans la barre du volet RDP pour afficher cette seule
connexion. **Quitter le plein écran** ou **Ctrl+Alt+Entrée** rétablit la disposition
précédente, sans fermer les autres sessions. Échap est transmis au bureau distant.
Une déconnexion ou une boîte de dialogue de HostGates fait sortir de cette vue.
Le bureau conserve sa résolution négociée et est ajusté à la taille de l’écran.

### Choix global du client RDP

Dans Paramètres, choisissez **Client RDP par défaut** : RDP intégré (IronRDP),
ou MSTSC sous Windows / FreeRDP sous Linux. Cliquez sur **Enregistrer**.
Ce réglage vaut pour les nouvelles connexions ; les sessions ouvertes continuent.
Le chemin du client externe reste configurable dans Applications externes.

Les options d’une session permettent **Utiliser le réglage général**, IronRDP,
ou le client externe. Les anciens choix explicites du client externe sont
conservés. Les imports RDP suivent désormais le choix général ; le routage de
collection est aussi transmis à MSTSC / FreeRDP avec une RD Gateway.

Les entrées RDP intégrées sont envoyées dès leur traitement, sans attendre
l’affichage suivant. TCP_NODELAY est activé sur les connexions TCP directes et
vers les passerelles/proxys afin de limiter les délais des petits paquets.
La surbrillance d’un élément nécessite toujours un aller-retour vers le serveur.

## Résolution du client externe

Avant chaque lancement de MSTSC (Windows) ou FreeRDP (Linux), la fenêtre
**Affichage RDP** propose une liste de résolutions standard, de 1024 × 768 à
3840 × 2160, ainsi que **Plein écran** et **Réglage du client**. Ce dernier
conserve le comportement habituel du client externe.

Le choix est mémorisé pour cette session lorsque vous cliquez sur **Connexion**
ou appuyez sur Entrée. Annuler, Échap ou fermer la fenêtre ne modifie pas le
dernier choix et ne lance aucune connexion. Le choix intervient avant
l’ouverture éventuelle du tunnel SSH ; il est aussi appliqué aux sessions
importées utilisant RD Gateway et le routage RDS.

MSTSC reçoit les dimensions et le mode fenêtré/plein écran dans un fichier RDP
temporaire protégé. Les mots de passe enregistrés restent protégés par DPAPI.
FreeRDP reçoit `/size:largeurxhauteur` ou `/f`. Le serveur peut imposer ses
propres limites. Ce réglage ne modifie pas la résolution du client IronRDP intégré.

## Résolution du bureau

Dans **Nouvelle session** ou **Modifier la session**, sélectionnez **Résolution IronRDP**. Le choix est conservé par session dans preferences.json et prend effet à la prochaine connexion. Les résolutions standard vont de 1024×768 à 3840×2160 ; sans configuration, le bureau demandé reste 1280×720. Le réglage s’applique en direct, via tunnel SSH et via RD Gateway/RDS. Le serveur peut négocier une autre taille. Le plein écran agrandit la zone d’affichage ; il ne modifie pas le choix enregistré de résolution.
