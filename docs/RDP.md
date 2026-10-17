# RDP

Windows : MSTSC reçoit un fichier .rdp temporaire avec utilisateur, domaine et mot de passe protégé par DPAPI quand le coffre contient un secret. Sans secret (ou en mode saisie manuelle), /v:serveur:port et /prompt sont utilisés. Les politiques serveur peuvent imposer une nouvelle saisie.

Linux : FreeRDP reçoit le mot de passe par /from-stdin:force et un pipe, sans /p:secret dans les arguments. Sans secret, il est relié au terminal intégré pour ses demandes interactives. Le bureau apparaît dans une fenêtre graphique externe. Pour une invite supplémentaire ou un client incompatible, activer la saisie manuelle dans Options SSH / tunnel. Aucun contournement de validation de certificat.

Le statut « client lancé » ne prouve pas l’authentification. MSTSC peut transférer la connexion à un processus existant ; HostGates ne peut alors pas fermer cette fenêtre transférée. Le fichier temporaire reste jusqu’à la fermeture de l’onglet, puis est supprimé. Les identifiants TERMSRV existants ne sont pas modifiés. Les fichiers DPAPI et les arguments ont été testés ; aucun serveur RDP réel n’était disponible pour valider l’authentification de bout en bout. Voir securite.txt pour les limites.
