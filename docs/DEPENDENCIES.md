# Dépendances

Versions exactes reproductibles dans `Cargo.lock`. Vérification initiale des API et licences via documentation officielle et métadonnées crates.io le 15 octobre 2026. Aucune dépendance git non épinglée. Recontrôler maintenance, avis RustSec et licences avant publication.

| Dépendance | Rôle | Licence annoncée | Source |
|---|---|---|---|
| eframe/egui 0.36 | interface native Windows/Linux | MIT OR Apache-2.0 | https://github.com/emilk/egui |
| rusqlite 0.38 | SQLite, bibliothèque embarquée | MIT | https://github.com/rusqlite/rusqlite |
| keyring-core 1 | abstraction coffre | MIT OR Apache-2.0 | https://docs.rs/keyring-core/ |
| windows-native-keyring-store 1 | Windows Credential Manager | MIT OR Apache-2.0 | https://docs.rs/windows-native-keyring-store/ |
| dbus-secret-service-keyring-store 1 | Secret Service Linux | MIT OR Apache-2.0 | https://docs.rs/dbus-secret-service-keyring-store/ |
| zeroize 1 | nettoyage des buffers applicatifs | Apache-2.0 OR MIT | https://docs.rs/zeroize/ |
| serde 1 | modèles sérialisables, futur import/export | MIT OR Apache-2.0 | https://serde.rs/ |
| uuid 1 | identifiants v4 | Apache-2.0 OR MIT | https://docs.rs/uuid/ |
| thiserror 2 | erreurs typées | MIT OR Apache-2.0 | https://docs.rs/thiserror/ |
| directories 6 | répertoires utilisateur standards | MIT OR Apache-2.0 | https://docs.rs/directories/ |

Le nouvel écosystème keyring recommande `keyring-core` et les fournisseurs spécifiques pour les applications qui contrôlent leur backend. Pas de dépendance au wrapper générique keyring 4 ni à son mode CLI. Les fournisseurs sont conditionnels à la cible. Le fournisseur Linux utilise `crypto-rust` pour la communication Secret Service ; ce n'est pas une implémentation cryptographique maison.

eframe 0.36 a renommé les panels en `Panel` et le callback applicatif en `App::ui` ; le code suit ces API et exige Rust 1.95. La chaîne locale est Rust 1.99.

Les contrats de futurs backends ne valent pas validation de dépendances supplémentaires. Les composants transitifs ont leurs propres licences ; voir l'[inventaire](THIRD_PARTY_LICENSES.md) et les [conditions de redistribution](LICENSING.md). Les notices complètes restent à réunir pour chaque paquet binaire distribué.

## Ajouts 0.2

portable-pty 0.9 (PTY système), vt100 0.16 (parseur terminal), encoding_rs 0.8 (Windows-1252), csv 1 (CSV), rfd 0.17 (dialogues natifs), tempfile 3 (exports atomiques). Versions exactes et transitives dans Cargo.lock. OpenSSH/mstsc/FreeRDP sont des exécutables système externes, non embarqués. Paramiko est uniquement utilisé par le serveur de test facultatif.

URL 2 (validation HTTP/HTTPS) et webbrowser 1 (ouverture par le navigateur par défaut) ajoutés pour les sessions Web ; versions exactes dans Cargo.lock.


## Ajouts 0.3

`ssh2 0.9.6` (MIT OR Apache-2.0) et libssh2 natif (BSD-3-Clause) pour SSH/SFTP/direct-tcpip ; backend cryptographique Windows CNG ou OpenSSL sous Linux. `argon2 0.5.3` (MIT OR Apache-2.0), `aes-gcm-siv 0.11.1` (MIT OR Apache-2.0), `rand_core 0.6.4` (MIT OR Apache-2.0) et `serde_json` pour le coffre versionné et les préférences/locales. Versions et dépendances transitives vérifiées dans Cargo.lock et l'inventaire régénéré le 17 octobre 2026. Ces choix ne constituent pas un audit de sécurité.

Depuis 0.3.1 : `ssh2` active `openssl-on-win32` et `vendored-openssl` pour les clés modernes sous Windows ; OpenSSL 3.6.3 embarqué (Apache-2.0), `des` 0.8.1 (MIT OR Apache-2.0) pour le format PasswordFile VNC, `windows-sys` 0.61 (MIT OR Apache-2.0) pour DPAPI et les ACL. La clé fixe et l'ordre des bits du format VNC sont des paramètres d'interopérabilité ; aucune implémentation DES de TigerVNC n'est copiée.
