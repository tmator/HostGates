# Multi-exec — phase 5

Non implémenté en phase 1. Prévoir sélection de sessions/dossiers/tags/favoris et connexions ouvertes, concurrence bornée, annulation, timeout, stdout/stderr séparés, code retour et durée. Ne pas conserver les commandes dans l'historique par défaut.

La détection de reboot, shutdown, poweroff, halt, rm -rf, mkfs, fdisk, parted et dd déclenchera une confirmation supplémentaire ; elle ne pourra jamais garantir la sûreté d'une commande. Le broadcast est un mécanisme distinct, explicitement activé, destinations visibles et bannière rouge persistante. Il doit être désarmé lors de changements inattendus de destinations.
