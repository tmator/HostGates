# HostGates 0.3.6

Mise à jour de sécurité et de robustesse, Windows x64 et Linux x86-64.

- Filtrage des contrôles dans le collage terminal ; protection des exports CSV contre les formules.
- Une empreinte SSH enregistrée ne contourne plus un conflit known_hosts.
- Sauvegarde du secret de connexion SSH après authentification réussie.
- Adresses de bouclage distinctes pour les tunnels et leurs cookies Web.
- Limites SFTP de 10 Gio / 10 000 entrées par téléchargement ; imports accélérés.
- PATH relatif ignoré ; permissions des dossiers existants non modifiées sous Linux.

## FreeRDP et comptes locaux

Un PC hors domaine reste accessible. Pour une saisie interactive, activez « Saisir le mot de passe dans le client externe » dans les options de session. Pour transmettre un secret enregistré, renseignez un domaine explicite : le nom du PC cible pour un compte local (utilisateur séparé dans son champ), ou le domaine AD pour un compte de domaine. Ne renseignez pas un domaine fictif. Sans domaine explicite, HostGates refuse uniquement la transmission automatique du secret afin qu'il ne soit pas interprété comme le domaine par FreeRDP. Les politiques du serveur et la confiance du certificat restent applicables. Aucun serveur RDP réel n'a été validé pour cette release.

## Téléchargements

Windows : EXE portable ou ZIP avec documentation et licences.
Linux : DEB pour Debian 12+ / Ubuntu 22.04+ ; RPM pour RHEL/Rocky/AlmaLinux 9+ et Fedora récent. Binaire construit sur Rocky Linux 9, glibc 2.34 minimum. RHEL/AlmaLinux restent des cibles non testées directement. Empreintes SHA-256 fournies ; paquets non signés.

## Limites de sécurité

Le coffre n'authentifie pas preferences.json. Les tunnels locaux n'isolent pas les autres utilisateurs/processus du poste. Le collage multiligne sans bracketed paste peut exécuter des commandes. Les threads peuvent conserver des secrets jusqu'à leur arrêt effectif. Consultez securite.txt pour les limites et les corrections détaillées.

Les paragraphes d'aide modifiés peuvent revenir au français dans certaines langues, en attendant leur traduction.
