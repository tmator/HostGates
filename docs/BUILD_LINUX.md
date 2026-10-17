# Compiler et lancer HostGates sous Linux

Ce guide utilise Ubuntu/Debian. Exécutez les commandes dans un terminal Linux. La compilation et l’exécution Linux restent à valider : les tests locaux ont jusqu’ici été effectués sous Windows.

## 1. Copier le projet

Copiez le dossier du projet Windows vers Linux, par exemple dans `~/Documents/HostGates`.

Ne copiez pas `.tools` et `target` : ils contiennent la chaîne de compilation et les fichiers générés sous Windows.

Conservez les sources et fichiers du projet, notamment `Cargo.toml`, `Cargo.lock`, `src/`, `locales/`, `migrations/`, `docs/`, `tests/` et `scripts/`. Le dossier `docs/` est nécessaire car certains textes d’aide sont intégrés lors de la compilation.

## 2. Installer les dépendances de compilation

Sur Ubuntu/Debian :

```bash
sudo apt update
sudo apt install build-essential curl pkg-config \
  libdbus-1-dev libx11-dev libxkbcommon-dev libwayland-dev \
  libegl1-mesa-dev libgl1-mesa-dev libssl-dev zlib1g-dev
```

HostGates utilise Rust et une interface native egui/eframe. Node.js, Tauri et WebKit ne sont pas nécessaires. SQLite est embarqué par la dépendance Rust.

## 3. Installer Rust

Le projet demande Rust 1.95 minimum. Installez la version stable avec rustup :

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Acceptez l’installation par défaut, puis chargez l’environnement dans le terminal courant :

```bash
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

Si Rust est déjà installé avec rustup :

```bash
rustup update stable
```

Méthode officielle : https://rust-lang.org/tools/install/

N’exécutez pas rustup ou cargo avec sudo.

## 4. Compiler

Placez-vous dans le dossier contenant `Cargo.toml`. Adaptez ce chemin à l’emplacement choisi :

```bash
cd "$HOME/Documents/HostGates"
cargo build --release --locked
```

Le premier lancement télécharge les dépendances et peut prendre plusieurs minutes. Une connexion Internet est nécessaire si elles ne sont pas déjà en cache.

Le binaire produit est :

```text
target/release/hostgates
```

Il n’a pas d’extension `.exe`. Utilisez directement cargo sous Linux.

## 5. Lancer l’application

Depuis le dossier du projet, dans une session graphique Linux :

```bash
./target/release/hostgates
```

Pour compiler et lancer en mode développement :

```bash
cargo run --locked
```

Une session graphique et un pilote OpenGL compatible sont nécessaires. Les dialogues de fichiers utilisent le portail de bureau ; un service compatible doit être installé et actif. Le coffre de mots de passe nécessite DBus et un fournisseur Secret Service, par exemple GNOME Keyring. Si le coffre est indisponible, les connexions restent utilisables avec saisie du mot de passe dans le client.

## 6. Installer et configurer les clients de connexion

### SSH

Sur Ubuntu/Debian :

```bash
sudo apt install openssh-client
command -v ssh
```

Le chemin est souvent `/usr/bin/ssh`. Il faut le client OpenSSH sur le PC local ; le serveur SSH doit être disponible sur la machine distante.

### RDP

Installez le paquet fournissant le client FreeRDP X11 pour votre distribution et sa version. Son nom varie. L’exécutable recherché par HostGates est `xfreerdp3`, puis `xfreerdp` :

```bash
command -v xfreerdp3
command -v xfreerdp
```

Un affichage X11 ou XWayland compatible est nécessaire. Les demandes d’authentification apparaissent dans l’onglet HostGates et le bureau distant dans une fenêtre séparée.

### Chemins personnalisés

Dans **Paramètres → Applications externes** :

1. Laissez les champs vides pour rechercher les clients dans PATH, ou sélectionnez leur exécutable avec **Parcourir**.
2. Exemples : `/usr/bin/ssh`, `/usr/bin/xfreerdp3`, `/usr/bin/bash`.
3. Utilisez des chemins absolus, sans guillemets, arguments, `~` ou variables d’environnement.
4. Cliquez **Vérifier les chemins**, puis **Enregistrer les chemins**.

La vérification contrôle les fichiers sans les exécuter ; elle ne valide pas la connexion réseau. Les modifications s’appliquent aux prochaines connexions. Après une modification du PATH système, redémarrez HostGates.

Créez ensuite une session SSH ou RDP avec l’adresse DNS/IP, le port et l’utilisateur, puis double-cliquez dessus. Ports habituels : SSH 22, RDP 3389. Vérifiez les accès réseau/VPN, les droits du compte et le service distant. Les secrets du coffre HostGates ne remplissent pas automatiquement les invites des clients.

Voir également **Aide → Configurer les connexions** dans l’application et [le guide détaillé](CONNECTION_SETUP.md).

## 7. Vérifier la compilation et les tests

```bash
rustup component add rustfmt clippy
cargo fmt -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

Les tests standards ne nécessitent pas de serveur SSH distant ni de coffre actif. Deux tests d’intégration sont ignorés par défaut : le test coffre natif et le test SSH avec serveur local facultatif. Leur procédure est décrite dans les documents correspondants.

Une CI Windows/Linux est définie dans `.github/workflows/ci.yml`, mais sa présence ne signifie pas qu’elle a été exécutée avec succès.

## 8. Dépannage

- **cargo introuvable** : exécutez `source "$HOME/.cargo/env"` ou ouvrez un nouveau terminal.
- **Version Rust trop ancienne** : exécutez `rustup update stable`, puis vérifiez `rustc --version`.
- **Cargo.toml introuvable** : placez-vous à la racine du projet avec `cd`.
- **Erreur pkg-config, DBus, X11 ou Wayland** : vérifiez l’installation des dépendances de l’étape 2.
- **Compilation interrompue faute de mémoire** : essayez `cargo build --release --locked -j 1`.
- **Fenêtre impossible à ouvrir** : lancez depuis une session graphique, avec un pilote OpenGL compatible.
- **Dialogue de fichier indisponible** : vérifiez le portail de bureau de votre environnement graphique.
- **SSH/RDP introuvable** : installez le client, vérifiez son chemin et enregistrez-le dans les paramètres.
- **Connexion refusée ou délai dépassé** : vérifiez adresse, port, VPN, pare-feu et disponibilité du serveur.

## Autres distributions

Sur Fedora, les dépendances de compilation prévues sont :

```bash
sudo dnf install gcc gcc-c++ curl pkgconf-pkg-config dbus-devel \
  libX11-devel libxkbcommon-devel wayland-devel \
  mesa-libEGL-devel mesa-libGL-devel
```

Installez ensuite Rust et utilisez les mêmes commandes cargo. Cette procédure Fedora n’a pas été validée sur ce poste.

## Sources

- Installation officielle Rust : https://rust-lang.org/tools/install/
- Dialogues de fichiers rfd : https://docs.rs/rfd/0.17.2/rfd/
- Configuration du projet : `Cargo.toml`, `Cargo.lock` et `.github/workflows/ci.yml`.

Depuis 0.3.1, SSH embarque OpenSSL : installer aussi `perl` et `make` pour sa compilation (`sudo apt install perl make`). Aucun Perl nécessaire à l’exécution.
