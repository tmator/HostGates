# Base locale

Migration initiale : `migrations/001_initial.sql`. Exécution dans une transaction puis mise à jour de `user_version`. WAL, synchronous FULL, foreign_keys ON et busy timeout 3 secondes. Base plus récente rejetée. Les tests rouvrent une base réelle temporaire et vérifient les contraintes.

Tables actives en phase 1 : folders, sessions, tags, session_tags, application_settings. Les paramètres SSH/RDP par défaut sont créés selon le protocole. Autres tables préparées : ssh_tunnels, known_hosts, recent_sessions, transfer_history, import_history. Leur présence ne signifie pas que leurs fonctions sont disponibles.

`credential_id` est un UUID identifiant une entrée dans le service de coffre stable ; ce n'est pas un secret. `passphrase_credential_id` est une référence future. Aucune colonne de valeur de mot de passe ou passphrase. Les données importées brutes ne sont pas conservées.

Les dates sont UTC ISO-8601. L'identité de session (protocole, hôte, port, utilisateur) n'est pas unique : les doublons peuvent être utiles et l'import devra demander une politique explicite. Dossiers non vides protégés. Effacement de session cascade sur tags, paramètres et tunnels ; l'orchestration de suppression du secret appartient au service.
