# HostGates 0.3.5 — langues supplémentaires

Release Windows x64. Les sources restent compatibles Windows et Linux.

## Nouveautés

- Espagnol, italien, allemand, portugais du Brésil, japonais, chinois simplifié, coréen, arabe, hindi et indonésien, en plus du français et de l'anglais.
- 308 entrées par langue, intégrées à l'exécutable. Sélection dans Paramètres > Langue avec le nom de chaque langue dans son écriture.
- Polices libres Noto intégrées pour les écritures CJK, arabe et devanagari. Aucune installation de police ni connexion réseau nécessaire à l'utilisation.
- Fichiers JSON fournis pour adapter les traductions et contrôles de parité des clés, paramètres variables et caractères disponibles.

## Utilisation

Fermez HostGates puis lancez `HostGates-0.3.5.exe`. Les sessions, les paramètres et le coffre existants sont conservés. Choisissez la langue dans Paramètres > Langue ; elle est mémorisée.

L'archive contient l'exécutable, les catalogues JSON, la documentation et les notices de licence. L'exécutable seul contient également les langues et les polices. `SHA256SUMS-0.3.5.txt` contient les empreintes des deux fichiers.

## Portée et limites

Les dix nouveaux catalogues ont été produits par traduction automatique, avec corrections de libellés et préservation des noms techniques et paramètres. Ils n'ont pas été intégralement relus par des locuteurs natifs. Les fichiers Markdown et les diagnostics provenant du système ou des bibliothèques ne sont pas tous traduits.

Le moteur egui 0.36 forme les glyphes arabes et devanagari, mais ne fournit pas un traitement bidirectionnel complet. Les passages mêlant arabe, chiffres, chemins et texte latin peuvent présenter un ordre visuel imparfait. L'interface n'est pas entièrement inversée de droite à gauche. Les commandes, noms de sessions et adresses ne sont jamais réécrits pour contourner cette limite.

La compilation Linux reste disponible depuis les sources ; cette archive contient uniquement le binaire Windows. Aucun audit de sécurité indépendant n'est annoncé.

## Validation

Compilation Release Windows, formatage et Clippy validés. 72 tests réussis, dont les tests DPAPI et les contrôles de polices ; 4 tests d'intégration facultatifs ignorés. Les 12 catalogues de 308 entrées ont passé les contrôles des clés et paramètres variables. Pas de validation visuelle exhaustive ni de validation Linux dans cette release.
