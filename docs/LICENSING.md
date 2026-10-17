# Licence et interopérabilité

Vérification du 16 octobre 2026. Informations générales, pas un avis juridique individualisé.

## Choix pour HostGates

Le code propre à HostGates est publié sous [MIT](../licence.md), également déclarée dans Cargo.toml. Cette licence courte permet l'utilisation, la modification et la redistribution, commerciale ou non, en conservant la notice de copyright et la licence. Elle n'oblige pas les dérivés à publier leur code. C'est le choix retenu pour faciliter l'adoption ; si l'objectif était d'imposer le partage des modifications, une licence copyleft répondrait à un autre besoin. MIT n'accorde pas de licence explicite de brevets comme Apache-2.0.

Source : [Open Source Initiative, MIT](https://opensource.org/license/mit).

## Fichiers MobaXterm

Une implémentation indépendante qui lit et écrit un format de sessions n'est pas, de ce seul fait, soumise à la licence du logiciel d'origine. Dans l'Union européenne, la CJUE distingue les fonctionnalités et formats de fichiers de l'expression du programme protégée par le droit d'auteur des logiciels. Cette conclusion ne donne pas le droit de copier du code, des textes de manuel ou des ressources graphiques protégés ; d'autres droits et les conditions d'accès au logiciel restent à considérer.

Source : [CJUE, SAS Institute / World Programming, C-406/10, notamment points 39–46 et dispositif](https://eur-lex.europa.eu/legal-content/FR/TXT/?uri=CELEX:62010CJ0406).

HostGates utilise son propre parseur et ses icônes dessinées dans le code. Conserver cette indépendance : ne pas incorporer de code, logo, icône ou binaire MobaXterm. L'inspiration visuelle ne vaut pas autorisation de copier une interface à l'identique. Décrire le produit comme « HostGates, import/export de sessions compatible avec certains formats MobaXterm », sans suggérer une affiliation ou une certification par Mobatek. La compatibilité technique avec toutes les versions n'est pas garantie.

La [licence officielle MobaXterm Home](https://mobaxterm.mobatek.net/license.html) encadre son utilisation et reconnaît l'observation, l'étude ou le test du fonctionnement dans les conditions prévues. Elle n'autorise pas à republier ses composants propriétaires. Les conditions applicables à une édition Pro doivent être examinées séparément si elle sert à des travaux de rétro-ingénierie. Le parseur actuel ne nécessite pas de décompiler MobaXterm.

## Dépendances et distribution

L'[inventaire des licences déclarées](THIRD_PARTY_LICENSES.md) est généré à partir des métadonnées de Cargo.lock, toutes plateformes confondues. C'est un inventaire, pas un certificat de conformité ni un recueil complet des notices.

Les licences tierces continuent à s'appliquer à leurs composants ; la licence MIT de HostGates ne les remplace pas. Avant de distribuer un binaire, joindre les textes de licence et notices des composants effectivement embarqués, y compris leurs ressources et bibliothèques natives. Points particuliers relevés :

- `option-ext 0.2.0` est sous MPL-2.0 : pour une distribution l'incluant, informer les destinataires de la disponibilité des sources couvertes et fournir un moyen d'obtenir ces sources (y compris les modifications éventuelles) sous MPL. Cela n'impose pas de placer l'ensemble de HostGates sous MPL. Voir [FAQ officielle Mozilla, distribution d'un programme plus large](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) et [texte MPL, sections 3.2 et 3.3](https://www.mozilla.org/en-US/MPL/2.0/).
- `epaint_default_fonts` inclut des polices OFL-1.1 et Ubuntu-font-1.0 : conserver les notices correspondantes et respecter leurs restrictions de noms et de redistribution.
- Les composants Apache-2.0, BSD, Unicode-3.0, Boost (BSL-1.0), ISC et autres licences permissives gardent leurs obligations de notices. Pour les doubles licences, choisir une branche autorisée ; un `AND` impose les deux conditions.
- OpenSSH, mstsc, FreeRDP, TigerVNC et le navigateur sont des programmes externes, non distribués avec HostGates. Leur licence reste indépendante.

L'inventaire doit être régénéré et les notices revérifiées lors d'un changement de dépendances. Ne pas inclure les bases de sessions personnelles, secrets, exports réels ou répertoires de travail privés dans une publication.
