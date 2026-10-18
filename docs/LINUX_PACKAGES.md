# Paquets Linux DEB et RPM

Les paquets sont destinés aux PC **x86-64/amd64**, avec une session graphique X11 ou Wayland. Ils contiennent le même binaire natif, les 12 langues, un lanceur de menu et les licences des composants embarqués. Les mots de passe et sessions restent dans le profil utilisateur, jamais dans le paquet.

## Compatibilité visée

| Format | Distributions visées |
|---|---|
| `.deb` | Debian 12 et suivantes ; Ubuntu 22.04 et suivantes |
| `.rpm` | RHEL, Rocky Linux et AlmaLinux 9 et suivantes ; Fedora récent |

La base de compilation du binaire distribué est Rocky Linux 9. Le générateur refuse un binaire exigeant une glibc supérieure à 2.34. Les dépendances graphiques chargées à l'exécution sont déclarées explicitement en plus des dépendances ELF détectées. Cela ne constitue pas une validation exhaustive sur chaque distribution et chaque bureau.

RHEL/Rocky/AlmaLinux 8, Debian 11, les distributions sans glibc telles qu'Alpine et les machines ARM ne sont pas visés par ces paquets. Un conteneur sans bureau ne suffit pas pour utiliser l'application graphique.

## Installer

Debian ou Ubuntu, dans le dossier contenant le paquet :

```bash
sudo apt install ./hostgates_0.3.5-1_amd64.deb
hostgates
```

RHEL, Rocky, AlmaLinux ou Fedora :

```bash
sudo dnf install ./hostgates-0.3.5-1.x86_64.rpm
hostgates
```

Utilisez `apt` ou `dnf` pour résoudre les dépendances ; évitez `dpkg -i` ou `rpm --nodeps` seuls sur votre poste. Les paquets locaux ne sont pas signés par une clé d'éditeur ; vérifiez le fichier SHA-256 fourni.

SSH et SFTP sont intégrés. Pour RDP, installez le client FreeRDP fourni par votre distribution ; pour VNC, un client compatible TigerVNC. Indiquez leurs chemins dans Paramètres > Applications externes si nécessaire. Les noms de paquets et exécutables diffèrent selon les versions des distributions.

Les dialogues de fichiers nécessitent `xdg-desktop-portal` et un backend adapté au bureau (GTK, KDE…). Le coffre système nécessite un service Secret Service disponible dans la session DBus, par exemple GNOME Keyring. Le coffre maître de HostGates est distinct. Le navigateur par défaut utilise les réglages du bureau.

## Construire les paquets

Compiler avec Rust 1.95 minimum sur une base glibc 2.34 ou antérieure, par exemple Rocky Linux 9. Les images officielles sont disponibles sur [Quay, espace Rocky Linux](https://quay.io/repository/rockylinux/rockylinux) ; cette base peut fonctionner dans un conteneur Podman à l'intérieur de WSL2.

```bash
cargo build --release --locked
cargo test --release --locked
```

Sur un environnement Debian/Ubuntu disposant de `rpm`, `dpkg-dev`, `python3`, `binutils` et `desktop-file-utils`, avec le binaire compilé et les sources :

```bash
python3 scripts/package-linux.py
```

`CARGO_TARGET_DIR` permet de désigner un dossier de compilation différent. Le générateur ne compile ni n'installe l'application ; il vérifie son ABI, collecte les licences et produit les deux formats dans `target/linux-release/`, avec un rapport des dépendances et des empreintes SHA-256. Il n'envoie rien sur GitHub.

Les sources `option-ext` non modifiées sont incluses dans la documentation du paquet conformément à sa licence MPL-2.0. Les polices Noto sont intégrées au binaire ; leurs notices OFL et celles des autres dépendances accompagnent les paquets.

## Désinstaller

```bash
sudo apt remove hostgates
# ou, sur une distribution RPM :
sudo dnf remove hostgates
```

La désinstallation ne supprime pas les sessions ni le coffre de votre profil utilisateur. Le chemin exact des données est affiché dans les paramètres de HostGates.

## Validation du 18 octobre 2026

Les mêmes paquets ont été installés avec résolution de leurs dépendances, puis lancés pendant 8 secondes sous Xvfb avec rendu OpenGL logiciel et un profil vierge :

| Distribution de test | Paquet | Installation | Démarrage graphique et création de la base |
|---|---|---|---|
| Debian 12 (Bookworm) | DEB | Réussie | Réussi |
| Ubuntu 22.04.5 LTS | DEB | Réussie | Réussi |
| Rocky Linux 9.8 | RPM | Réussie | Réussi |
| Fedora 44 | RPM | Réussie | Réussi |

Ces essais ont été réalisés dans des conteneurs isolés sous WSL2. Ils ne remplacent pas les tests sur un poste graphique réel, notamment pour Wayland, les portails de fichiers, le coffre système et les connexions réseau. RHEL et AlmaLinux n'ont pas été testés directement : la compatibilité visée repose sur la base ABI de la famille Enterprise Linux 9 et les tests Rocky.

## Compilation sous WSL2

Une distribution WSL2 dédiée permet de compiler et de tester sans modifier les données de l'application Windows. Conservez les sources et caches de compilation dans le système de fichiers Linux pour de meilleures performances. Les disques et outils WSL de développement ne doivent pas être ajoutés au dépôt Git ni aux paquets distribués.
