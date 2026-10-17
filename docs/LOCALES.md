# Ajouter une langue

Les traductions sont des dictionnaires JSON UTF-8 : la clé est le texte français de référence, la valeur est le texte traduit. `fr` et `en` sont disponibles ; l'anglais est embarqué dans le binaire. Le dossier `locales` du projet contient les catalogues complets.

1. Copier `locales/en.json` vers `locales/de.json` (ou un autre code, par exemple `pt-BR`).
2. Traduire les **valeurs**, conserver les clés et les paramètres `{count}`, `{}` à l'identique. Ne pas traduire les identifiants, chemins ou adresses contenus dans les paramètres.
3. Placer le fichier dans `locales` à côté de l'exécutable, ou dans le sous-dossier `locales` des données utilisateur (chemin de la base visible dans Paramètres).
4. Redémarrer, ouvrir Paramètres > Langue et choisir le code. Le choix est enregistré dans `preferences.json`.

Les catalogues utilisateur complètent ou remplacent les textes embarqués. Une clé absente revient au français. Les messages produits par les serveurs et les clients externes gardent leur propre langue. Les exemples de commandes ne sont pas traduits.

Pour ajouter du texte dans le code Rust : `crate::i18n::tr("Texte français")`, puis ajouter la clé aux catalogues. Pour un texte formaté, traduire le résultat complet et ajouter le modèle correspondant (exemple `{count} occurrence(s)`). Les modèles doivent contenir suffisamment de texte fixe pour éviter de traiter une adresse ou un nom utilisateur comme une phrase d'interface.
