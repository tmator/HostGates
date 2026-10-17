# SFTP — phase 4

Contrat SftpBackend initial disponible ; implémentation à venir, aucun transfert actuellement. Double panneau local/distant, navigation, opérations sur fichiers, file de transferts indépendante de l'onglet, progression, annulation et retry. Pas de FTP/SMB. La future API devra préciser les événements de progression et la politique d'écrasement avant transfert. Suppression confirmée, pas de suivi aveugle de liens symboliques ni de chemins extraits non validés.
