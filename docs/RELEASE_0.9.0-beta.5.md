# HostGates 0.9 — bêta 5

Cette publication fournit le paquet **DEB amd64** pour Debian 12+ et Ubuntu 22.04+.

- Consoles SPICE Proxmox intégrées aux onglets via libspice-client-glib : clavier, souris, curseur et plein écran, sans remote-viewer externe ni fichier .vv.
- Hyper-V : choix HTTP/HTTPS et port personnalisable, dont 5895. Sous Linux, les commandes HTTP utilisent obligatoirement le chiffrement NTLM des messages.
- Dépendance SPICE native installée automatiquement par APT ; aide, documentation et traductions des nouveaux réglages actualisées.

Installation : télécharger le DEB puis exécuter `sudo apt install ./hostgates_0.9.0-beta.5-1_amd64.deb`.
Pour WinRM sur le port 5895, choisir HTTP et renseigner `http://votre-hote:5895`.

SPICE : premier écran jusqu’à 4096 × 4096 ; audio, USB, presse-papiers et multi-écrans non implémentés. La disposition du clavier est celle de la VM. HTTPS vérifié reste préférable pour WinRM.

Validation : tests Rust Windows/Linux, Clippy, test QEMU local avec proxy CONNECT et TLS, refus des certificats invalides, test des messages WinRM HTTP chiffrés. Les connexions à votre infrastructure restent à valider.
