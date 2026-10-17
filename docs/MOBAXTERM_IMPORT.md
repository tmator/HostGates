# Import et export MobaXterm

Importer accepte .mxtsessions, .ini et .csv. Un aperçu permet de sélectionner les lignes et choisir ignorer/importer/remplacer pour les doublons (protocole, hôte, port, utilisateur). Remplacer conserve la référence au secret existant et échoue en cas de correspondance ambiguë. L’import entier est transactionnel. Le rapport distingue SSH, RDP, remplacements, ignorés, types non supportés et invalides.

INI : sections Bookmarks et Bookmarks_N, SubRep pour la hiérarchie, UTF-8/BOM et fallback Windows-1252. Le premier nombre entre # est une icône, jamais un protocole. Le champ suivant avant % définit le protocole : 0 SSH, 4 RDP. Toutes les icônes sont acceptées, dont 70, 149 et 105. Les autres protocoles ne sont pas convertis. Seuls hôte/port/utilisateur sont mappés ; aucun déchiffrement, aucune conservation du payload brut. Limites : 8 Mio, 10 000 lignes de sessions, profondeur 64.

CSV recommandé : colonnes Folder name, Session name, Session type, Hostname, Username, Port number. Dans MobaXterm utiliser User sessions > Import sessions from third-party programs puis l’import CSV. Source officielle : https://blog.mobatek.net/post/mobaxterm-import-csv/

L’export .mxtsessions écrit les champs de connexion de base. Le format propriétaire complet n’est pas documenté : aucun paramètre avancé n’est inventé, et la relecture par toutes les versions MobaXterm n’a pas été vérifiée. Les valeurs impossibles à représenter sont refusées. Utiliser CSV si votre version refuse le fichier.

Les exports excluent mots de passe, passphrases et références au coffre ; ils ne constituent pas une sauvegarde complète de HostGates. Écriture par fichier temporaire et remplacement atomique. Les 18 tests couvrent fixtures synthétiques, encodages, fichiers malformés, doublons, rollback et aller-retour. Aucun fichier utilisateur réel n’a été fourni.

Correction du 16 octobre 2026 : reconnaissance selon le protocole réel, prise en charge des fichiers .moba sans section et du préfixe de présentation « ; logout » (jamais exécuté). Tests des deux structures Soleil fournies, avec adresses anonymisées. Les clés privées, passerelles, proxys et autres réglages avancés ne sont toujours pas importés.

Source de la distinction icône/protocole : étude du format par Ruzgfpegk, https://gist.github.com/Ruzgfpegk/ab597838e4abbe8de30d7224afd062ea .

Modes 5 (VNC) et 11 (Web) désormais pris en charge. VNC : hôte/port, sans interpréter le champ d’affichage comme utilisateur. Web : URL HTTP/HTTPS et port dérivé de l’URL, sans interpréter le champ de barre d’outils comme port. Les préférences de navigateur embarqué ne sont pas reprises.
