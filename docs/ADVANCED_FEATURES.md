# Connexions, fichiers et espace de travail

## SSH et transferts

Le mode SSH intégré utilise libssh2. À la connexion, renseigner l'utilisateur et le mot de passe, une clé privée ou laisser le mot de passe vide pour utiliser l'agent. Vérifier l'empreinte serveur avant d'accepter une nouvelle clé. Les clés modifiées sont bloquées.

Le bouton **SFTP** d'un volet SSH affiche ou masque l'explorateur à droite. Double-cliquer sur un dossier pour entrer, utiliser ↑ pour remonter et ↻ pour rafraîchir. Ctrl+clic sélectionne plusieurs éléments. **Envoyer** propose des fichiers ou un dossier ; **Télécharger** copie les éléments sélectionnés, dossiers inclus, dans le répertoire local choisi.

Le serveur doit autoriser SFTP et une connexion supplémentaire. Les transferts sont exécutés hors du thread graphique. Aucune commande shell n'est construite à partir des noms. Aucun écrasement automatique ; les liens symboliques sont refusés. Les interruptions peuvent laisser des fichiers partiels. Cette version ne propose pas de reprise de transfert ni de conservation des ACL/métadonnées complètes.

## Coffre

Créer le coffre dans **Paramètres > Coffre et mot de passe maître**. Les secrets déjà enregistrés sont migrés. Déverrouiller au lancement avant d'utiliser les identifiants sauvegardés. Dans une session, **Mot de passe…** permet d'enregistrer ou supprimer un secret ; la connexion SSH propose également de l'enregistrer.

Le bouton de verrouillage ferme les connexions. Un verrouillage automatique s'applique après 10 minutes sans interaction. La perte du mot de passe maître n'est pas récupérable. Les protections, algorithmes et limites sont décrits dans [securite.txt](../securite.txt).

## Passerelle SSH

Créer une session SSH pour la passerelle et la connecter directement une première fois pour valider sa clé et configurer son authentification. Sur une session cible SSH, RDP, VNC ou Web : **clic droit > Options SSH / tunnel**, puis sélectionner la passerelle. Sa destination est contactée depuis la passerelle. Les passerelles en cascade ne sont pas prises en charge.

Le tunnel reste lié à l'onglet et écoute uniquement sur l'interface locale. Pour Web, le navigateur ouvre une URL locale : HTTPS et les sites dépendant de leur nom de domaine peuvent nécessiter une configuration DNS/proxy externe. HostGates ne contourne pas les contrôles de certificat. Pour RDP/VNC, le client externe conserve sa fenêtre ; les identifiants enregistrés sont fournis selon les modalités ci-dessous.

## Terminaux multiples

**Volets visibles** choisit de 1 à 6 vues. Les autres onglets restent ouverts. Le menu ▾ de chaque onglet et de chaque volet permet de retrouver une connexion ouverte. Il évite d'afficher deux fois le même terminal dans la grille. Cliquer dans un terminal pour lui donner le focus clavier.

Le menu ▾ ou le clic droit permet de renommer un onglet pour l'exécution en cours. Pour renommer durablement une session sauvegardée, utiliser Modifier sur la session.

**Exécution multiple** ouvre la sélection des terminaux. Saisir les commandes, vérifier les destinataires, cocher la confirmation puis exécuter. Chaque ligne est envoyée avec Entrée. Les réponses restent dans les terminaux ; le programme ne détecte pas les invites ni la fin des commandes. Ne pas sélectionner un terminal attendant un mot de passe ou une confirmation.

## Langues

Choisir `fr` ou `en` dans Paramètres > Langue. Pour ajouter une langue sans recompiler, voir [LOCALES.md](LOCALES.md).

## Identifiants SSH / RDP / VNC / Web (0.3.1)

Dans **Nouvelle session** ou **Modifier**, cocher **Enregistrer / remplacer le mot de passe dans le coffre**. Le coffre doit être déverrouillé. Ne pas cocher conserve le secret existant. **Mot de passe… > Supprimer du coffre** le supprime explicitement.

- SSH intégré : la connexion utilise le secret après validation de la clé hôte. Le moteur OpenSSL embarqué prend en charge Ed25519 et ECDSA sous Windows. L'erreur de négociation `Session(-5)` arrive avant l'authentification ; changer le mot de passe ne la résout pas. Si elle persiste, contrôler la configuration serveur ou utiliser OpenSSH externe (authentification interactive).
- RDP : renseigner utilisateur et, si nécessaire, domaine. Windows fournit un fichier `.rdp` avec secret protégé par DPAPI à MSTSC. Linux fournit le secret par entrée standard à FreeRDP. Une politique serveur ou une MFA peut toujours demander une confirmation.
- VNC : configurer TigerVNC Viewer ou un client compatible `-PasswordFile`. La transmission automatique couvre VncAuth classique, jusqu'à 8 caractères ASCII. Pour d'autres modes, activer **Options SSH / tunnel > Saisir le mot de passe dans le client externe (RDP / VNC)**. Le mot de passe peut rester dans le coffre.
- Web : stockage possible, mais le navigateur gère l'authentification du site ; aucun secret dans l'URL.

Les fichiers temporaires sont réservés au compte utilisateur et supprimés à fermeture de l'onglet. Après un arrêt brutal, des fichiers `hostgates-auth-*` peuvent rester dans le répertoire temporaire. Voir `securite.txt`, notamment les limites du format VNC.

## Interface 0.3.2

La zone droite reste vide tant qu'aucune connexion n'est ouverte. Le catalogue est dans **Sessions > Afficher les sessions**, sans onglet permanent. Entrée dans le champ de mot de passe SSH déclenche **Connecter**, sans accepter automatiquement une clé serveur inconnue.

Les boutons **Volets visibles** vont jusqu'au nombre d'onglets ouverts, avec un maximum de six. Fermer des onglets réduit automatiquement le nombre de volets. Dans **Exécution multiple**, **Sélectionner tout** cible uniquement les terminaux actifs ; **Tout désélectionner** vide la sélection. Confirmer les destinataires avant l'envoi reste nécessaire.

## Coffre, clavier et passerelle manuelle (0.3.4)

Si un mot de passe maître est configuré, HostGates propose son déverrouillage au démarrage. Entrée valide ; Échap ferme la demande en laissant le coffre verrouillé. Sans mot de passe maître configuré, l'application continue à utiliser le coffre système. Une erreur de déverrouillage permet de réessayer.

Dans les fenêtres, Entrée lance l'action principale (enregistrer, connecter, déverrouiller, importer/exporter, confirmer la suppression). Les validations et les boutons désactivés restent respectés. Échap ferme la fenêtre au premier plan et abandonne son brouillon. Une liste déroulante ouverte se ferme d'abord. Dans les descriptions et commandes multilignes, Entrée insère une ligne ; Ctrl+Entrée déclenche l'action principale. L'exécution multiple exige toujours de cocher la confirmation des destinataires. Une opération déjà validée ou un réglage explicitement appliqué n'est pas annulé par Échap.

Dans **Options SSH / tunnel > Saisir une nouvelle passerelle SSH…**, renseigner adresse, port (22 par défaut), utilisateur et mot de passe. Un mot de passe vide utilise l'agent SSH. **Enregistrer** crée une session SSH réutilisable et l'associe à la cible, qui peut être SSH, RDP, VNC ou Web. Le mot de passe est enregistré dans le coffre actif, jamais dans preferences.json. Si son enregistrement échoue, la nouvelle session est retirée. La passerelle apparaît dans l'arbre et peut être modifiée ou utilisée par une autre cible.

Lors de la première connexion, vérifier l'empreinte de la passerelle auprès de son administrateur et l'accepter explicitement. Entrée ne fait jamais confiance automatiquement à une clé inconnue. Une clé connue modifiée reste bloquée. Les tunnels restent limités à un saut.
