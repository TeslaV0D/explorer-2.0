# Glossary: Italian (it)

Localisation stage L6 (D-029, `docs/PLAN-I18N.md`). Catalog: `lang/it/LC_MESSAGES/explorer2.po` (complete, offered as “Italiano”).
Variant: **Italian (Italy), it-IT**, as Windows Explorer shows it. Instructions use the informal “tu” (“Non spostare …”, “digita > …”); confirmation questions use the infinitive (“Eliminare …?”, “Installare …?”), like Windows.
**Native-speaker review still open**: please look over the CI artifact `ui-gallery-linux-l10n` (folder `it`).

## Core terms (Windows Explorer it-IT)
| English | Italiano | Note |
|---|---|---|
| File Explorer | Esplora file | our app keeps its name “Explorer 2.0” |
| Recycle Bin | Cestino | |
| Empty Recycle Bin | Svuota Cestino | |
| This PC | Questo PC | |
| Quick access | Accesso rapido | |
| Home (user folder) | Cartella personale | not “Home” (that is the Windows 11 start page) |
| Desktop / Documents / Downloads / Pictures / Music / Videos | Desktop / Documenti / Download / Immagini / Musica / Video | |
| Libraries | Raccolte | |
| Local Disk / USB Drive / DVD Drive / Network Drive | Disco locale / Unità USB / Unità DVD / Unità di rete | “drive” = unità |
| File folder | Cartella di file | type name |
| {} File | File {} | “File TXT” |
| Shortcut | Collegamento | “Shortcut or link” → “Collegamento o link” |
| Name / Date modified / Type / Size | Nome / Data ultima modifica / Tipo / Dimensioni | column headers as in Explorer |
| Date deleted / Original location | Data eliminazione / Percorso originale | Recycle Bin columns |
| Cut / Copy / Paste / Rename / Delete | Taglia / Copia / Incolla / Rinomina / Elimina | |
| Delete permanently | Elimina definitivamente | |
| Properties | Proprietà | |
| New folder / New tab | Nuova cartella / Nuova scheda | |
| Undo / Redo | Annulla / Ripeti | “Annulla” is also Cancel, as in Windows |
| Refresh | Aggiorna | |
| Select all | Seleziona tutto | |
| Show hidden items | Mostra elementi nascosti | |
| File name extensions | Estensioni dei nomi dei file | |
| Details / Large icons / Medium icons / Extra large icons | Dettagli / Icone grandi / Icone medie / Icone molto grandi | |
| Sort / Ascending / Descending | Ordina / Crescente / Decrescente | |
| Search | Cerca | “Search X” → “Cerca in X” |
| Settings | Impostazioni | |
| Folder options | Opzioni cartella | |
| Address bar | Barra degli indirizzi | |
| Command palette | Tavolozza dei comandi | |
| Inspector | Ispettore | “Toggle details pane” → “Mostra o nascondi il riquadro dei dettagli” |
| Color scheme / Light / Dark / System default | Combinazione di colori / Chiaro / Scuro / Predefinito di sistema | |
| Replace / Merge / Skip / Keep both / Cancel | Sostituisci / Unisci / Ignora / Mantieni entrambi / Annulla | conflict dialog |
| OK / Close | OK / Chiudi | |
| Restore / Reset | Ripristina / Reimposta | |
| Windows integration | Integrazione con Windows | |
| item / items | elemento / elementi | “3 elementi selezionati”; 0 is plural |

## Keys
Key names as on an Italian keyboard / in Windows it-IT menus: **Ctrl**, **Maiusc** (Shift), **Alt**, **CANC** (Del), **INVIO** (Enter), arrows **Sinistra/Destra/Su**. The keys themselves never change (Ctrl+A stays Ctrl+A).

Access keys in the conflict dialog (`core::mnemonic`, the `&` in the catalog): **S**ostituisci / **U**nisci, **I**gnora, **M**antieni entrambi, **t**utti (“Esegui l'operazione per &tutti i conflitti rimanenti”). Esc = Annulla.

Recovery tool prompt: `[s/N]`, accepts “s”, “sì” (and always y/yes/j/ja).

## Numbers and units
`1.234,5` (point for thousands, comma for decimals), `explorer_i18n::separators("it")`. Size units stay **B, KB, MB, GB, TB** like Italian Windows; the spelled unit is “byte” (“3,1 KB (3.170 byte)”).

## Deliberately not translated
- The Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” and the folder verb texts written to the registry stay German (R-001 journal compares them; TODO “R-001-safe relabel”). Texts that name this entry quote it unchanged.
- Technical names: `EXPLORER2_INDEX`, `SLINT_BACKEND`, `fs.inotify.max_user_watches`, `shell:`, `crash` folder.
