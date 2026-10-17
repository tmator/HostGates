# Sécurité

## Mesures présentes

Les sessions résident dans `remotehub.db`, sous le répertoire utilisateur déterminé par `directories::ProjectDirs` avec l'identité `io.remotehub.desktop`. Le chemin exact apparaît dans Paramètres. Le dossier Linux est créé avec permissions 0700. Sous Windows, les ACL du profil utilisateur sont héritées. SQLite n'est pas chiffré : noms, adresses, descriptions, tags et identifiants utilisateur sont lisibles par les programmes ayant accès au profil.

Les mots de passe sont écrits dans Windows Credential Manager (persistance locale) ou Secret Service sous Linux avec communication DBus chiffrée par la bibliothèque. La base contient seulement un UUID `credential_id`. Aucun secret dans SQL, exports, arguments système ou logs. Le fournisseur est explicitement initialisé ; les mocks sont limités aux tests. L'échec du coffre est visible et ne provoque aucun stockage de substitution. La session peut être enregistrée sans mot de passe.

`CredentialStore` offre stockage, lecture, suppression et présence. L'interface n'offre pas de lecture/copie du mot de passe. Saisie masquée et buffer applicatif `Zeroizing<String>`, déplacé au worker sans clone. Les copies internes aux widgets, bibliothèques, allocateurs et OS ne peuvent pas être garanties effacées ; le programme ne promet pas une résistance à l'inspection mémoire par un processus privilégié. Aucun cache persistant egui n'est activé.

Les erreurs du coffre sont traduites en messages fixes sans sérialisation des erreurs natives, susceptibles de contenir des données. Pas de logger applicatif ni d'historique de commandes dans cette phase. Les futurs logs devront utiliser une liste de champs autorisés et un redactor central, sans secret ni terminal brut.

SQL utilise des paramètres, jamais une concaténation utilisateur. Les ports, adresses, noms et couleurs sont validés en Rust. Le worker sérialise mutations et accès au coffre. La duplication ne copie pas de référence au coffre. Suppression de session avec suppression du secret avant les métadonnées, et refus si le coffre échoue.

## Contrôles obligatoires avant connexion réelle

- SSH : vérification de clé par endpoint/algorithme, empreinte présentée au premier accès, choix une fois/toujours/refus. Changement de clé connue : avertissement et refus par défaut, jamais acceptation silencieuse. En 0.2, OpenSSH assure ces contrôles avec StrictHostKeyChecking=ask et son fichier known_hosts.
- RDP : certificats vérifiés ; aucune option globale pour ignorer les erreurs. Windows `mstsc`, Linux FreeRDP via arguments séparés, jamais shell. Aucun mot de passe en argv. Credential API TERMSRV Windows à traiter séparément des credentials génériques de phase 1 ; sous Linux utiliser un mécanisme sûr détecté ou la saisie FreeRDP.
- Presse-papiers : copie de secret non proposée. Toute future copie nécessitera avertissement et effacement conditionnel si la valeur n'a pas changé.
- Verrouillage : pas de faux verrouillage par simple masque d'interface. Une authentification OS et un contrôle dans le service seront nécessaires avant de proposer cette fonction.
- Multi-exec : confirmation supplémentaire pour commandes manifestement dangereuses ; pas un sandbox. Broadcast désactivé par défaut et signalé très visiblement.
- Export : sans secrets, ni payload MobaXterm brut. Export chiffré facultatif ultérieur : Argon2id + AEAD reconnu, avec format versionné.

## Limites et sauvegarde

Le coffre OS ne protège pas contre tout programme malveillant exécuté dans la session utilisateur ni contre un administrateur. Fermer HostGates puis sauvegarder la base ; une copie à chaud du seul `.db` peut perdre les transactions WAL. Le coffre doit être sauvegardé selon les mécanismes de l'OS. Les futurs exports seront distincts des sauvegardes du coffre. Ne pas saisir de secrets dans les descriptions/tags : ces champs sont des métadonnées ordinaires.

Pas d’audit externe réalisé. SSH utilise OpenSSH, RDP les clients du système ; SFTP graphique reste absent. Le refus d’une clé SSH modifiée a été testé sur un serveur local. La sortie du terminal est en mémoire et peut contenir des informations sensibles ; elle n’est pas journalisée par HostGates. Les shells peuvent toutefois conserver leur propre historique.
