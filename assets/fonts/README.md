# Polices intégrées

HostGates utilise les polices Noto Sans CJK, Noto Sans Arabic et Noto Sans Devanagari pour les langues ne figurant pas dans les polices par défaut d'egui. Les fichiers sont intégrés au binaire et ne nécessitent aucune installation ou connexion réseau à l'exécution.

Les polices sont redistribuées sans modification sous SIL Open Font License 1.1. Leurs textes de licence et notices sont conservés dans ce dossier. `sources.json` contient les URL officielles et empreintes SHA-256 des fichiers récupérés.

La collection CJK fournit les variantes japonaise (index 0), coréenne (index 1) et chinoise simplifiée (index 2). Le choix suit la langue de l'interface. Le terminal garde sa police monospace principale, avec ces polices comme solutions de repli pour les caractères absents.
