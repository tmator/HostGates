# Architecture

## Décision de pile

La demande initiale proposait Tauri/React. La préférence explicite suivante pour une application Rust conduit à une interface **egui/eframe native**. Le code applicatif, l'interface, la validation et la persistance sont en Rust. SQLite et certaines dépendances système comportent du code natif non Rust. Le projet ne dépend plus de Tauri, React, TypeScript ou xterm.js.

egui/eframe supporte Windows et Linux (X11/Wayland). Le renderer OpenGL `glow` évite un backend WebView ; un pilote graphique compatible reste nécessaire. La version 0.36 requiert Rust 1.95 minimum. Le terminal utilise portable-pty et le parseur existant vt100 avec un rendu egui. OpenSSH assure le protocole SSH.

## Organisation actuelle

```text
src/
  main.rs              démarrage de la fenêtre
  branding.rs          nom affiché et identifiants persistants
  models/              Session, Folder, DTO et validations
  database/            SQLite, transactions, migrations, requêtes paramétrées
  credentials/         CredentialStore et fournisseurs natifs
  services/            orchestration session/coffre
    worker.rs          file d'actions et résultats vers l'interface
  backends/            contrats SshBackend, RdpBackend, SftpBackend
  ui/                  shell, navigation, propriétés, formulaires
migrations/            SQL versionné
tests/                 tests sans serveur distant et test natif facultatif
scripts/               diagnostics et outils de compilation
docs/                  décisions et feuille de route
```

Les futurs modules `sftp`, `tunnels`, `multiexec` et `platform` seront créés avec leurs implémentations respectives ; pas de providers factices. Les interfaces sont des contrats initiaux, à étendre notamment pour progression, reconnexion et annulation.

## Flux

Interface → Action typée → worker unique → service → SQLite/coffre → Snapshot sans secrets → interface. Le worker détient seul la connexion SQLite et le CredentialStore et sérialise leur accès. Une opération en cours bloque les mutations supplémentaires, mais laisse la fenêtre répondre. Les secrets ne figurent jamais dans Snapshot. Les futures connexions réseau disposeront de tâches asynchrones séparées ; elles ne doivent pas bloquer ce worker.

La base assure les relations, contraintes de ports/protocoles et transactions. Le service valide les entrées avant SQL. Les dossiers sont limités à 64 niveaux et les cycles rejetés, y compris déplacement d'un sous-arbre. Suppression d'un dossier non vide refusée. Suppression session : coffre d'abord, base ensuite. Si le coffre refuse la suppression, conserver la session pour permettre de réessayer.

Le coffre et SQLite ne partagent pas de transaction distribuée. Une interruption entre écriture au coffre et liaison SQLite peut laisser une entrée orpheline. Une erreur de liaison déclenche un nettoyage compensatoire. Une interruption de suppression peut laisser une référence à une entrée absente, traitée comme absence de mot de passe. Un mécanisme de réconciliation est prévu au durcissement.

## Identité et évolution

Le nom affiché est centralisé dans `branding.rs`. L'identifiant de stockage est stable : le renommer nécessite une migration explicite des données et références au coffre. Les migrations sont ordonnées via `PRAGMA user_version`, atomiques, jamais modifiées après distribution. Une base plus récente est refusée.

Les états actifs (connexions, tunnels et transferts en cours) restent en mémoire ; SQLite ne prouve jamais qu'un processus ou tunnel est connecté. Les protocoles futurs auront leurs providers propres. Seuls SSH/RDP sont acceptés actuellement.

## Découpage des phases

0 architecture ; 1 gestion locale ; 2 SSH et terminal ; 3 onglets/tunnels ; 4 SFTP ; 5 multi-exec/broadcast ; 6 RDP externe ; 7 MobaXterm ; 8 export/sauvegardes ; 9 packaging ; 10 durcissement.

## Ajouts 0.2

connections/ contient les clients système SSH/RDP et le PTY. importers/ contient les parseurs MobaXterm/CSV et les exports ; database/import.rs assure la transaction globale. ui/ contient terminal, onglets, aide et aperçu. Les threads PTY sont indépendants du worker SQLite/coffre. Les contrats backends/ ne sont pas les implémentations des clients système actuels.
