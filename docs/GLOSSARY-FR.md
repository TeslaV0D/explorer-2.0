# Glossary: French (fr)

Localisation stage L6 (D-029, `docs/PLAN-I18N.md`). Catalog: `lang/fr/LC_MESSAGES/explorer2.po` (complete, offered as “Français”).
Variant: **French (France), fr-FR**, as Windows Explorer shows it; formal “vous” in instructions (“supprimez …”, “tapez > …”) like Windows.
**Native-speaker review still open**: please look over the CI artifact `ui-gallery-linux-l10n` (folder `fr`).

## Core terms (Windows Explorer fr-FR)
| English | Français | Note |
|---|---|---|
| File Explorer | Explorateur de fichiers | our app keeps its name “Explorer 2.0” |
| Recycle Bin | Corbeille | |
| Empty Recycle Bin | Vider la Corbeille | |
| This PC | Ce PC | |
| Quick access | Accès rapide | |
| Home (user folder) | Dossier personnel | not “Accueil” (Windows 11 start page) |
| Desktop / Documents / Downloads / Pictures / Music / Videos | Bureau / Documents / Téléchargements / Images / Musique / Vidéos | |
| Libraries | Bibliothèques | |
| Local Disk / USB Drive / DVD Drive / Network Drive | Disque local / Lecteur USB / Lecteur de DVD / Lecteur réseau | “drive” = lecteur |
| File folder | Dossier de fichiers | type name |
| {} File | Fichier {} | “Fichier TXT” |
| Shortcut | Raccourci | |
| Name / Date modified / Type / Size | Nom / Modifié le / Type / Taille | column headers as in Explorer |
| Date deleted / Original location | Date de suppression / Emplacement d'origine | Recycle Bin columns |
| Cut / Copy / Paste / Rename / Delete | Couper / Copier / Coller / Renommer / Supprimer | |
| Delete permanently | Supprimer définitivement | |
| Properties | Propriétés | |
| New folder / New tab | Nouveau dossier / Nouvel onglet | |
| Undo / Redo | Annuler / Rétablir | “Annuler” is also Cancel, as in Windows |
| Refresh | Actualiser | |
| Select all | Sélectionner tout | |
| Show hidden items | Afficher les éléments masqués | “hidden” = masqué |
| File name extensions | Extensions de noms de fichiers | |
| Details / Large icons / Medium icons / Extra large icons | Détails / Grandes icônes / Icônes moyennes / Très grandes icônes | |
| Sort / Ascending / Descending | Trier / Croissant / Décroissant | |
| Search | Rechercher | “Search X” → “Rechercher dans X”; “Search all drives” → “Rechercher dans les lecteurs” (fits the search box) |
| Settings | Paramètres | |
| Folder options | Options des dossiers | |
| Address bar | Barre d'adresse | |
| Command palette | Palette de commandes | |
| Inspector | Inspecteur | “Toggle details pane” → “Afficher ou masquer le volet des détails” |
| Color scheme / Light / Dark / System default | Jeu de couleurs / Clair / Sombre / Par défaut du système | |
| Replace / Merge / Skip / Keep both / Cancel | Remplacer / Fusionner / Ignorer / Conserver les deux / Annuler | conflict dialog |
| OK / Close | OK / Fermer | |
| Restore / Reset | Restaurer / Réinitialiser | |
| Windows integration | Intégration à Windows | |
| item / items | élément / éléments | French plural: 0 and 1 are singular (“0 élément”) |

## Typography
- Non-breaking space (U+00A0) before `:` `?` `!` `;` and inside guillemets: `Langue : Deutsch`, `« notizen.txt »` (the generator `fr.py` applies it; the list of forbidden file-name characters stays as it is).
- Ellipsis without a space: `Chargement…`.
- Typographic apostrophe is not used (Windows uses the straight `'`).

## Keys
Key names as in Windows fr-FR: **Ctrl**, **Maj** (Shift), **Alt**, **Suppr** (Del), **Entrée** (Enter), arrows **Gauche/Droite/Haut**. The keys themselves never change.

Access keys in the conflict dialog (`core::mnemonic`): **R**emplacer / **F**usionner, **I**gnorer, Conserver les **d**eux, **t**ous (“Faire de même pour &tous les conflits restants”). Esc = Annuler.

Recovery tool prompt: `[o/N]`, accepts “o”, “oui” (and always y/yes/j/ja).

## Numbers and units
`1 234,5` (no-break space for thousands, comma for decimals); size units **o, Ko, Mo, Go, To** like French Windows (“3,1 Ko (3 170 octets)”). Dates are the Windows short format `dd/MM/yyyy HH:mm` (“15/01/2026 13:05”).

## Deliberately not translated
- The Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” and the folder verb texts in the registry stay German (R-001; TODO “R-001-safe relabel”).
- Technical names: `EXPLORER2_INDEX`, `SLINT_BACKEND`, `fs.inotify.max_user_watches`, `shell:`, `crash` folder.
