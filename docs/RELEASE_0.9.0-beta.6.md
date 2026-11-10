# HostGates 0.9.0 bêta 6 — diagnostic Linux

Cette bêta ajoute des diagnostics pour rechercher les échecs de connexion SPICE et Hyper-V. Elle ne garantit pas encore leur résolution sur les serveurs concernés.

- SPICE : panneau « Diagnostic de connexion » et bouton de copie, avec proxy utilisé, canal et catégorie d'erreur réseau native. Les tickets et mots de passe sont exclus.
- Hyper-V Linux : distingue le refus d'authentification, les défis NTLM incompatibles et les erreurs de session chiffrée. Bouton de copie sous l'erreur.
- Aucun assouplissement du chiffrement NTLM ou de la validation des certificats.

## Installation Linux x86-64

```bash
sudo apt install ./hostgates_0.9.0-beta.6-1_amd64.deb
```

Paquet vérifié sous Debian 12 et Ubuntu 22.04. Tests Rust Linux : 72 réussis ; test supplémentaire du refus de proxy SPICE natif réussi ; Clippy sans avertissement.

Après reproduction, copier le diagnostic de chaque connexion. Les adresses des serveurs peuvent être masquées avant partage. Ne pas transmettre de mot de passe ni de fichier contenant un ticket SPICE.

Cette publication contient uniquement le DEB et son fichier SHA-256. Les consoles SPICE nécessitent les bibliothèques natives indiquées dans la documentation du projet.
