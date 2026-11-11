# HostGates 0.9

Le support de SPICE est ajouté sur Windows et Linux. Les consoles des VM Proxmox s’ouvrent dans un onglet HostGates, avec le clavier, la souris et le plein écran. Un affichage SPICE doit être configuré pour la VM. Sous Windows, installer les bibliothèques Virt Viewer de même architecture que HostGates. Le DEB installe libspice-client-glib. Le RPM la recommande. Sur les distributions où elle est absente, les consoles SPICE nécessitent son installation séparée.

Le support Hyper-V est toujours **en développement**, notamment la connexion WinRM depuis Linux.

Le panneau de diagnostic SPICE et les diagnostics NTLM de la bêta 6 ont été retirés. Les erreurs de connexion restent affichées.

Téléchargements x86-64 disponibles en EXE et ZIP pour Windows, DEB pour Debian et Ubuntu, RPM pour les distributions compatibles. Les fichiers SHA-256 permettent de vérifier chaque téléchargement.
