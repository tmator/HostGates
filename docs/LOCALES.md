# Langues de HostGates

Paramètres > Langue propose français (`fr`), anglais (`en`), espagnol (`es`), italien (`it`), allemand (`de`), portugais du Brésil (`pt`), japonais (`ja`), chinois simplifié (`zh-CN`), coréen (`ko`), arabe (`ar`), hindi (`hi`) et indonésien (`id`). Les catalogues et polices sont intégrés au binaire et fonctionnent hors ligne.

Les traductions sont des objets JSON UTF-8 : la clé française identifie le message, la valeur contient sa traduction. Ne traduisez pas les clés ni les paramètres entre accolades (`{count}`, `{}`, etc.). Conservez les noms de fichiers, chemins, protocoles et raccourcis techniques. Lancez `python scripts/check-locales.py` pour contrôler les catalogues distribués.

Pour personnaliser une langue, placez son fichier dans `locales/` à côté de l'exécutable, ou dans le sous-dossier `locales/` du répertoire de données de l'application. Le catalogue intégré est chargé d'abord, puis le dossier de l'exécutable, puis celui des données utilisateur. Les fichiers personnalisés sont prioritaires ; les entrées absentes restent celles du catalogue intégré. Pour une langue nouvelle, les clés absentes restent en français.

Pour ajouter une langue, copiez `en.json` sous un nouveau code (`nl.json`, par exemple), traduisez les valeurs, puis redémarrez. Une modification sur disque est relue lors d'un changement de langue ou au prochain lancement. Le nom du fichier doit contenir uniquement des lettres ASCII, chiffres, tirets ou traits de soulignement.

Les nouvelles traductions sont automatiques avec corrections ciblées, sans relecture native exhaustive. Voir [les notes 0.3.5](RELEASE_0.3.5.md) pour les limites du rendu arabe bidirectionnel. Les traductions ne modifient ni les commandes envoyées, ni les adresses, ni les secrets.
