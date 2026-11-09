# HostGates 0.9 — bêta 4

Préversion **0.9.0-beta.4**, 9 novembre 2026. Correctifs depuis la bêta 3.

## SPICE

Suppression du test TCP préalable qui pouvait empêcher remote-viewer de démarrer alors que ce client dispose de son propre accès réseau. Le proxy automatique est conservé ; remote-viewer effectue directement la connexion. Les canaux TLS et la validation du certificat SPICE restent actifs. La connexion complète à une VM Proxmox réelle reste à retester sur votre infrastructure.

## Certificats Hyper-V

La case **Ne pas vérifier le certificat HTTPS** est désormais disponible dans le profil Hyper-V sous Windows et Linux. Elle est désactivée par défaut et s’applique uniquement au profil choisi et à ses nœuds de cluster, sans modifier le magasin de confiance du système.

Sous Linux, le transport Rust accepte alors le certificat non valide et ignore le CA du profil. Sous Windows, les contrôles CA, nom et révocation sont ignorés uniquement pour les sessions PowerShell de l’opération. Décocher la case rétablit la validation stricte. HTTPS reste obligatoire sous Linux ; les redirections restent refusées.

Avec cette exception, l’identité du serveur n’est plus vérifiée et les identifiants peuvent être interceptés. Le service WinRM doit toujours être accessible. L’option ne change pas la validation du certificat de la console FreeRDP.

## Validation

Tests TLS locaux avec certificat non approuvé : refus par défaut, exception explicite, retour à la validation stricte et refus des redirections. Tests des opérations PowerShell avec et sans exception, tests Rust Windows/Linux, analyse Clippy stricte et vérification des douze langues. Les connexions réelles Hyper-V et SPICE restent à valider dans votre environnement.

## Téléchargements

- Windows x64 : EXE et ZIP avec documentation et licences.
- DEB amd64 : Debian 12+ / Ubuntu 22.04+.
- RPM x86_64 : RHEL / Rocky / AlmaLinux 9+ et Fedora récent, base glibc 2.34.
- Empreintes SHA-256 des fichiers Windows et Linux.

Version interne des paquets : `0.9.0~beta.4-1`. Les fichiers ne sont pas signés par une clé d’éditeur. Aucun coffre, profil personnel ni outil local Codex n’est inclus.
