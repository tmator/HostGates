# SSH intégré et mode externe

Depuis 0.3.1, le moteur intégré utilise libssh2 avec OpenSSL embarqué, y compris sous Windows : Ed25519, ECDSA et RSA-SHA2. Il transmet le mot de passe du coffre après validation de la clé serveur. Session(-5) est une erreur de négociation avant authentification ; vérifier les algorithmes serveur si elle persiste.

## Mode OpenSSH externe

# SSH et terminal

OpenSSH installé sur le système assure le protocole, l’authentification et known_hosts. Arguments séparés, sans shell intermédiaire ; StrictHostKeyChecking=ask, timeout 15 secondes, keepalive 30 secondes et trois tentatives. Les clés et l’agent configurés dans OpenSSH restent utilisables ; le mot de passe se saisit dans le terminal. Aucune injection automatique depuis le coffre.

portable-pty fournit ConPTY sous Windows et un PTY Unix sous Linux ; vt100 interprète ANSI. Les threads de lecture/écriture utilisent des files bornées. Le rendu egui fournit sélection, copie Ctrl+Maj+C, collage Ctrl+V avec bracketed-paste si demandé, Ctrl+C distant, touches fonction, redimensionnement, scrollback et zoom. La recherche porte sur le contenu affiché. Une fermeture tue puis récupère le processus enfant. La clé modifiée est refusée par OpenSSH ; la clé inconnue exige confirmation explicite.

Le test tests/ssh_loopback.rs nécessite le serveur Paramiko local facultatif. Lancer scripts/ssh-test-server.py avec un dossier de sortie, définir HOSTGATES_SSH_FIXTURE vers ce dossier puis cargo test --test ssh_loopback -- --ignored. Ne pas utiliser de vrais secrets pour cette fixture.
