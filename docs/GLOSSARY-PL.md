# Glossary: Polish (pl)

Localisation stage L6 (D-029, `docs/PLAN-I18N.md`). Catalog: `lang/pl/LC_MESSAGES/explorer2.po` (complete, offered as “Polski”).
Variant: **Polish (pl-PL)**, as Windows Explorer shows it; informal “ty” in instructions (“usuń …”, “wpisz > …”) like Windows 10/11.
**Native-speaker review still open**: please look over the CI artifact `ui-gallery-linux-l10n` (folder `pl`).

## Core terms (Windows Explorer pl-PL)
| English | Polski | Note |
|---|---|---|
| File Explorer | Eksplorator plików | our app keeps its name “Explorer 2.0” |
| Recycle Bin | Kosz | capitalised like Windows (“do Kosza”) |
| Empty Recycle Bin | Opróżnij Kosz | |
| This PC | Ten komputer | |
| Quick access | Szybki dostęp | |
| Home (user folder) | Folder osobisty | not “Narzędzia główne” (Windows 11 start page) |
| Desktop / Documents / Downloads / Pictures / Music / Videos | Pulpit / Dokumenty / Pobrane / Obrazy / Muzyka / Wideo | |
| Libraries | Biblioteki | |
| Local Disk / USB Drive / DVD Drive / Network Drive | Dysk lokalny / Dysk USB / Stacja dysków DVD / Dysk sieciowy | |
| File folder | Folder plików | type name |
| {} File | Plik {} | “Plik TXT” |
| Shortcut | Skrót | |
| Name / Date modified / Type / Size | Nazwa / Data modyfikacji / Typ / Rozmiar | column headers |
| Date deleted / Original location | Data usunięcia / Oryginalna lokalizacja | Recycle Bin columns |
| Cut / Copy / Paste / Rename / Delete | Wytnij / Kopiuj / Wklej / Zmień nazwę / Usuń | |
| Delete permanently | Usuń trwale | |
| Properties | Właściwości | |
| New folder / New tab | Nowy folder / Nowa karta | |
| Undo / Redo | Cofnij / Ponów | |
| Refresh | Odśwież | |
| Select all | Zaznacz wszystko | |
| Show hidden items | Pokaż ukryte elementy | |
| File name extensions | Rozszerzenia nazw plików | |
| Details / Large icons / Medium icons / Extra large icons | Szczegóły / Duże ikony / Średnie ikony / Bardzo duże ikony | |
| Sort / Ascending / Descending | Sortuj / Rosnąco / Malejąco | |
| Search | Wyszukaj | “Search X” → “Przeszukaj: X” (Explorer's search box) |
| Settings | Ustawienia | |
| Folder options | Opcje folderów | |
| Address bar | Pasek adresu | |
| Command palette | Paleta poleceń | |
| Inspector | Inspektor | “Toggle details pane” → “Pokaż lub ukryj okienko szczegółów” |
| Color scheme / Light / Dark / System default | Schemat kolorów / Jasny / Ciemny / Jak w systemie | |
| Replace / Merge / Skip / Keep both / Cancel | Zamień / Scal / Pomiń / Zachowaj oba / Anuluj | conflict dialog |
| OK / Close | OK / Zamknij | |
| Restore / Reset | Przywróć / Resetuj | |
| Windows integration | Integracja z systemem Windows | |
| item | element / elementy / elementów | three plural forms (exactly 1 / 2–4, 22–24 / the rest, 12–14, 21) |

## Typography
- Quotes „…” (low-high); dash “–” between parts of a sentence, “—” in copy names (`Nazwa — kopia (2)`).
- Ellipsis without a space: `Ładowanie…`.

## Keys
Key names as on Polish keyboards / Windows pl-PL: **Ctrl**, **Shift**, **Alt**, **Del**, **Enter**; arrows **W lewo/W prawo/W górę**.

Access keys in the conflict dialog (`core::mnemonic`): **Z**amień / **S**cal, **P**omiń, Zachowaj **o**ba, **w**szystkich (“Zastosuj do &wszystkich pozostałych konfliktów”). Esc = Anuluj.

Recovery tool prompt: `[t/N]`, accepts “t”, “tak” (and always y/yes/j/ja).

## Numbers and units
`1 234,5` (no-break space for thousands, comma for decimals); size units **B, KB, MB, GB, TB** like Polish Windows (“3,1 KB (3 170 bajtów)”). Dates are the Windows short format `dd.MM.yyyy HH:mm` (“15.01.2026 13:05”).

## Deliberately not translated
- The Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” and the folder verb texts in the registry stay German (R-001; TODO “R-001-safe relabel”).
- Technical names: `EXPLORER2_INDEX`, `SLINT_BACKEND`, `fs.inotify.max_user_watches`, `shell:`, `crash` folder.
