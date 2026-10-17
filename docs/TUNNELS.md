# Tunnels — phase 3

Schéma `ssh_tunnels` prêt pour LOCAL, REMOTE et DYNAMIC. Écoute par défaut 127.0.0.1, port validé. Implémentation et interface à venir. Les états de processus et sockets resteront en mémoire, jamais déduits d'une valeur persistée. Vue par session et vue globale, démarrage/arrêt, erreurs de port occupé. OpenSSH système est autorisé comme provider SOCKS si nécessaire, toujours avec arguments séparés et vérification des clés hôtes.
