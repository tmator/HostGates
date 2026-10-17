# Compiler sous Windows

Windows 10/11 x64, Rust stable >=1.95 avec cible MSVC, Visual Studio Build Tools 2022 avec outils C++ et SDK Windows. Aucun Node.js ou WebView2.

```powershell
./scripts/check-windows.ps1
cargo fmt -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```


Le binaire est `target/release/hostgates.exe`. Pour le développement : `cargo run --locked`. Le test coffre natif est facultatif : `cargo test --locked native_vault_roundtrip -- --ignored`. Il écrit une entrée UUID temporaire puis la supprime, sans consulter les entrées existantes.

Les MSI/setup.exe seront ajoutés en phase 9 avec signature et tests d'installation/désinstallation. La compilation ne constitue pas une validation sur toutes les éditions Windows.

Depuis 0.3.1, installer Strawberry Perl et le rendre disponible dans le PATH avant compilation. OpenSSL est compilé statiquement ; la première compilation est sensiblement plus longue, aucune DLL OpenSSL ni installation Perl requise pour les utilisateurs du binaire.
