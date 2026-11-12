# Current state

**As of:** 2026-11-07 · **Phase:** 1 · M0–M7 ✅ (M7.3 postponed) · R-001 ✅ · **M8.1/M8.2 installer ✅ (run #69)** · **M8.3 hardening ✅ (runs #70–#73)** · **Phase 2 search: S1.1 ✅ (run #78), S1.2 ✅ (run #80), S1.3 ✅ (run #83), S1.4 ✅ (run #90), S1.5 ✅ (runs #91/#93) → **S1 complete**, field test ✅ (Boss: “es funktioniert” – it works) · **Redesign: R0 ✅ (run #101), R1 ✅ (run #104), R2 ✅ (run #110), R3 ✅ (run #115), R4 ✅ (run #120), R5 ✅ (run #123), R6 ✅ (run #124), R7 ✅ (run #125), R8 ✅ (run #128), R9 ✅ (run #130), R10 ✅ (run #131) → **redesign complete**, next: field test by Boss (`docs/FIELD-TEST-REDESIGN.md`)** (`docs/PLAN-REDESIGN.md`)** (plan `docs/PLAN-SEARCH.md`) · **Branch:** `claude/eloquent-gauss-sd6qhp`

The app UI is German; UI labels and messages are quoted as they appear (with a translation where it helps).

## Documentation in English (2026-11-10, Boss: start brought forward from Thu 12.11.)
- All documentation translated to English, content 1:1 (D-004): `docs/*.md`, `CLAUDE.md`, `THIRD-PARTY.md`, `scripts/release-notes.md` (the next GitHub release gets English notes), README completed with five screenshots from the Linux gallery (`docs/screenshots/`, taken from a neutral path `/home/explorer`).
- Field-test checklists renamed: `PRAXISTEST-SUCHE.md` → `FIELD-TEST-SEARCH.md`, `PRAXISTEST-REDESIGN.md` → `FIELD-TEST-REDESIGN.md`.
- No code change; CI ignores doc-only commits (`paths-ignore`).

## Localisation L6 – Italian ✅ (2026-11-12)
- **Italiano (it-IT)** complete: all 433 texts with the Windows Explorer terms of Italian Windows (Cestino, Questo PC, Accesso rapido, Cartella di file, Data ultima modifica, Rinomina, Svuota Cestino …), informal “tu” in instructions and the infinitive in confirmation questions, like Windows; glossary **`docs/GLOSSARY-IT.md`**. Offered as “Italiano” and palette command “Language: Italiano”. Two plural forms (`n != 1`: “1 elemento”, “0 elementi”, “3 elementi”); “unità” stays the same in both forms. Quotes «…», access keys S/U/I/M/t. Size units stay B/KB/MB/GB/TB; the word is “byte”. Numbers `1.234,5` (already `separators("it")`).
- Undo and Cancel are both “Annulla”, Redo is “Ripeti”, as in Italian Windows.
- Local tests: i18n catalog (template, every complete language, compiled tables) and `conflict_keys_match_labels_and_differ` green. The gallery and layout gate run in CI for every language under `lang/`, including `it`. Native-speaker review open.
- **Next language: Chinese (Simplified).**

## Localisation L6 – Polish ✅ (CI run #157, 2026-11-11)
- **Polski (pl-PL)** complete: all 432 texts with the Windows Explorer terms of Polish Windows (Kosz, Ten komputer, Szybki dostęp, Folder plików, Data modyfikacji, Zmień nazwę …), informal “ty” like Windows; glossary **`docs/GLOSSARY-PL.md`**. Offered as “Polski” and palette command “Language: Polski”. Three plural forms with Polish rules (exactly 1 / 2–4 / the rest: “1 ukryty”, “3 170 bajtów”), quotes „…”, access keys Z/S/P/O/W. Layout check: every text fits on all 50 screens. Native-speaker review open.
- E2E [84c] assumed Ukrainian is still incomplete (automatic fallback to English) – with uk complete it is rightly picked → [84c] now uses a language we don't have (pt-BR → English) and the new **[84e]** checks that a complete system language (uk) is picked automatically, UI in Ukrainian. CI #156 (Ukrainian) was red on that one check.
- **CI #157 fully green** with ru, uk and pl: galleries + layout check for de, en, qps, es, fr, pl, ru, uk (50 screens each, every text fits), Windows incl. [G1]; Linux E2E locally 288/288.

## Localisation L6 – Ukrainian ✅ (CI run #157, 2026-11-11)
- **Українська (uk-UA)** complete: all 431 texts with the Windows Explorer terms of Ukrainian Windows (Кошик, Цей ПК, Швидкий доступ, Папка файлів, Дата змінення, Інсталювати …), formal “ви”; glossary **`docs/GLOSSARY-UK.md`**. Offered as “Українська” and palette command “Language: Українська”. Three plural forms (same rule as Russian), Cyrillic access keys (З/О/П/Б/В), units Б/КБ/МБ, “мс”. Layout check: every text fits on all 50 screens. Native-speaker review open.
- CI #155 (Russian) was red in a test that has nothing to do with languages: `search::refresh::tests::nothing_changed_nothing_read` read a folder again on Windows – NTFS updated the folder's last-write time a moment after the files were created. The test now does a first pass that takes such late times in, then checks that the second pass reads nothing.

## Localisation L6 – Russian ✅ (CI run #157, 2026-11-11)
- **Русский (ru-RU)** complete: all 430 texts with the Windows Explorer terms of Russian Windows (Корзина, Этот компьютер, Быстрый доступ, Папка с файлами, Дата изменения, view names Таблица/Крупные значки/Огромные значки …), formal “вы”, е instead of ё like Windows; glossary **`docs/GLOSSARY-RU.md`**. Offered as “Русский” and palette command “Language: Русский”. Native-speaker review of the gallery open.
- **Three plural forms** (1, 21 → элемент / 2–4 → элемента / 0, 5–20 → элементов) – the rule from L0 is used by every `tr_n!` (“15 элементов (1 скрыт)”, “Выбрано 3 элемента”).
- Cyrillic access keys in the conflict dialog (З/О/П/С/В, as Windows does with the Russian layout); key names stay Latin (Ctrl, Shift, Del, Enter); size units Б/КБ/МБ/ГБ/ТБ (“3,1 КБ (3 170 байт)”); quotes «…».
- The load time in the status line (“· 12 ms”) is a text now (“мс” in Russian).
- Layout check: every text fits on all 50 screens (Cyrillic is wider than Latin in places – still inside every box).

## Localisation L6 – French ✅ (CI run #154, 2026-11-11)
- **Français (fr-FR)** complete: all 428 texts with the Windows Explorer terms of French Windows (Corbeille, Ce PC, Accès rapide, Dossier de fichiers, Modifié le, Renommer, Afficher les éléments masqués …), formal “vous”, key names Maj/Suppr/Entrée; glossary **`docs/GLOSSARY-FR.md`**. Offered as “Français” and palette command “Language: Français”. Native-speaker review of the gallery open.
- **French typography**: no-break space before `:` `?` `!` `;` and inside « », ellipsis without a space (`Chargement…`); numbers `1 234,5` (from L2).
- **Size units are texts now** (`core::format_size`: `B/KB/MB/GB/TB` via `tr_noop!`): French Windows shows **o, Ko, Mo, Go, To** (“3,1 Ko (3 170 octets)”); German/Spanish/English keep B/KB/MB.
- Layout: “Rechercher dans tous les lecteurs” was 5 px too long for the search box in This PC / Recycle Bin → “Rechercher dans les lecteurs”; then every text fits on all 50 screens. French plural rule (0 and 1 singular) from L0.

## Localisation L6 – Spanish ✅ (CI run #153, 2026-11-11)
- **Español (es-ES)** complete: all 422 texts with the Windows Explorer terms of Spanish Windows (Papelera de reciclaje, Este equipo, Acceso rápido, Carpeta de archivos, Fecha de modificación, Cambiar nombre …), key names Mayús/Supr/Entrar, numbers `1.234,5`; glossary **`docs/GLOSSARY-ES.md`**. Offered in the language list and as palette command “Language: Español”. Native-speaker review of the gallery still open.
- **Access keys per language** (`core::mnemonic`): the conflict dialog no longer has R/S/B/A hard-wired in Slint; the letter comes from the catalog like Windows' `&` (`E&rsetzen`, `&Reemplazar`, `Co&mbinar`, `&Omitir`, `&Conservar ambos`, `&todos`). Slint asks Rust (`conflict-key`), German keeps r/s/b/a. Test `crates/core/tests/mnemonics.rs`: in every catalog the mnemonic without `&` equals the button text, and the keys of one dialog differ. The pseudo-locale does not count `&` (same padding as the label).
- **Column widths follow the language**: the default widths of “Date modified” and “Type” grow to the header / “File folder” text of the language (hidden probe texts in `MainWindow`, binding replaced once the user drags a column) – Spanish “Fecha de modificación” and “Carpeta de archivos” were elided before; German/English unchanged (130/110 px). The language list in the settings uses the full panel width (“Predeterminado del sistema (English)” was elided).
- Found in the Spanish gallery: “15 elementos (1 ocultos)” → the hidden count is a plural text now (`oculto`/`ocultos`).
- CI: one step makes a gallery for every further language in `lang/` (artifact **`ui-gallery-linux-l10n`**, one folder per language), the layout check covers them all – new languages are checked without touching the workflow.
- CI #153 fully green (es gallery 50 screens, every text fits; Windows [G1] green), Linux E2E locally 287/287.

## Localisation L5 – layout robustness ✅ (CI run #152, 2026-11-10)
- **Pseudo-locale `qps`** (`explorer_i18n::pseudo`): every English text with accented letters (`Settings` → `[Šéťťîñğš····]`), short texts +50 %, longer ones +40 %, in brackets (a cut-off end is visible), placeholders/line breaks untouched. `lang/qps` is generated by the `extract` tool, a catalog test checks it is current; it is in `LANGS` (never offered, only `EXPLORER2_LANG=qps` / `language = qps`), ~50 KB in the exe.
- **Overflow check in the UI dump** (`EXPLORER2_UI_OVERFLOW=1`, off by default: it measures every text): a 6th column marks texts that do not fit – `w` one line cut off, `h` wrapped lines cut off, `c` sticks out of a clipping parent (panel, scroll area, popup), `p` sticks out of the element right around it, `e` elided “…” on purpose. Texts without a label (inside std widgets) are listed when they overflow. Measured with Slint's own text layout (`layout_info`), no pixel guessing.
- **Gallery**: 9 more screens (sort menu, rename, conflict dialog, error, Inspector of a folder, This PC, Recycle Bin, many tabs, settings scrolled down) → 22 per theme, 50 per language; `overflow.txt` per gallery. CI runs it in **de, en, qps** (artifacts `ui-gallery-linux`, `-en`, `-qps`) and **`scripts/layout-check.sh`** fails on any `w/h/c/p` (elided texts listed only; the developer UI kit excluded). Windows: the gallery collects the same with Segoe UI and adds a qps pass (folder, Inspector, palette, settings, context menu) → check **[G1]**.
- **Found and fixed**: the indexed-drive check box with a long path (home folder) ran 400+ px out of the settings panel in **every** language; in qps “Single-click to open an item …” stuck out of its check box; the ⋮ menu elided “Command palette” → new kit block **`CheckRow`** (std check box + wrapping text, whole row toggles, same look) for all settings check boxes and the conflict dialog; the ⋮, sort and context menus are as wide as their longest entry (min. the old width, max. the window).
- Elided on purpose (listed by the check): file names in narrow columns, the column headers “Date modified”/“Date deleted”/“Original location” in qps (Explorer elides headers the same way).
- Result: de, en and qps – every text fits on all 50 screens; Windows [G1] (Segoe UI, de + qps) green from the first run.
- CI fonts are not the sandbox fonts: CI #150 found the Inspector key “Dimensions” in qps 3 px wider than the fixed 96 px key column (cut off) → the key column grows to its longest word. On CI the German column header “Änderungsdatum” is elided at the default width (listed, on purpose like Explorer). #151 still without that fix; **CI #152 fully green**, Linux E2E locally 287/287 (the [80] drive clicks scroll first: wrapped rows are taller).

## Localisation L4 – language setting ✅ (CI run #149, 2026-11-10)
- **Settings → “Sprache · Language”** at the top of the panel (the heading carries the English word whenever the UI is not English: findable in a language one cannot read). A `ComboBox` lists “System default (<language Windows would pick>)” first, then every **complete** language in its own name, sorted (`explorer_i18n::choices`; a chosen but incomplete language stays selectable). Today: Deutsch, English.
- **Live switch** (`lang::choose` → `lang::switch`): `explorer_i18n::set_language` + `slint::select_bundled_translation`, then every Rust-built text again – sidebar + drives, visible tabs reloaded (status line, type names, sizes, dates), tab titles, Inspector (`inspector::relabel`), search status (`search::relabel`: fixed texts are kept as `fn` pointers, the per-drive lines are rebuilt), integration/install status, and texts set once at start (renderer status, crash notice while shown) via **`lang::keep(win, key, f)`**. Menus, palette, quick search and dialogs build their texts when opened.
- A choice made in the UI wins over `EXPLORER2_LANG` for this run (the override only picks the start language); it is saved as `language = <code>` (`""` = System default) and used at the next start.
- **Palette**: “Language: System default/English/Deutsch” (`language-system`/`-en`/`-de`; a test demands a command for every offered language). The palette now also matches the **English source** of every command, so “language” works in every UI language.
- E2E [85]: open settings → list shows Deutsch/English/“Wie Windows (English)” → pick English → Slint and Rust texts English (“Settings”, “Recycle Bin”), saved → restart without override stays English → palette “language deutsch” switches back live, saved.
- CI #148 was red before any test ran: the examples `bench` of `explorer-fs` and `explorer-index` share `target/debug/examples/bench.exe`, and the Windows linker failed on the locked file (LNK1104; Cargo had warned about the collision for a while) → the index benchmark is now `index_bench`. CI #149 fully green, Linux E2E locally 287/287.

## Localisation L3 – English review + gallery ✅ (CI run #147, 2026-11-10)
- **Language-neutral test targets**: the UI scripts keep clicking German names; with `EXPLORER2_LANG=<code>` `scripts/ui-lib.sh` looks each name up first (`scripts/ui-lang-map.py`: German → English source via `lang/de`, → target via `lang/<code>`; 275 names). The gallery now runs in any language: CI artifact **`ui-gallery-linux-en`** next to `ui-gallery-linux`.
- English gallery checked screen by screen (32 images, light/dark): everything English, nothing clipped or overlapping; file type names (“TXT File”, “File folder”), sizes (“3.1 KB (3,170 bytes)”), Inspector groups GENERAL/DATE/LOCATION, dialogs, palette, quick search, settings.
- **Review of all 411 English texts** as a product language (Windows wording, US spelling, natural English) – ~40 changed, e.g. “Like Windows” → “System default”, “More options” → “Show more options” (Windows 11), “Search in X” → “Search X” (Explorer's search box), palette toggles “… on/off” → “Toggle …”, “Not possible here” → “Not available here”, “Opening failed” → “Couldn't open”, installer/setup/integration sentences rewritten. Every translation was carried over to the new source text; checked automatically: **the German texts are exactly the same as before** (multiset of all `msgstr`).
- Found during the review: without translation contexts one English text has one translation – renaming “Like Windows” to “System” merged it with the file attribute “System” (the Inspector would have shown “Attribute: Wie Windows”) → the colour scheme option is “System default”. Rule written down in CLAUDE.md.

## Localisation L2 – Rust texts ✅ (CI run #146, 2026-11-10)
- All user-visible texts in Rust are now `tr!`/`tr_n!` with English source – `core` (command names, type labels, sizes, attributes, places, renderer status, crash notice), `fs`/`ops`/`index`/`search`/`shell`/`integrate` (errors, drive names, copy/new-folder names, reset summaries), the app (status bar, search status, context menu, dialogs, job panel, Inspector, quick search, installer, integration), the setup and the recovery tool. **411 texts, `de.po` complete** → the German UI is unchanged; with `EXPLORER2_LANG=en` everything is English.
- `tr_noop!("…")` (gettext `N_`) marks texts in `const` tables (command directory) – translated where shown (`Command::title`/`shortcut`).
- Numbers per language: `explorer_i18n::format_integer`/`format_decimal`/`separators` (`1,234.5` en, `1.234,5` de/es/it, no-break space fr/pl/ru/uk) → `core::format_count`/`format_size`. Visible change for German: sizes now with a decimal comma (“3,1 KB” like Explorer, before “3.1 KB”).
- Logic no longer depends on UI texts: Inspector facts carry fixed ids (`size`, `modified`, `path` …) and group ids (`general`/`date`/`location`; Slint compares `r.gid`, not the German “ORT”); job/undo labels are fixed ids (`copy`, `move`, `rename`, `new-folder`, `delete` – never shown; Redo no longer matches German words).
- Language detection moved into `explorer-i18n` (`system_languages`, `init`), used by the app (also for `--install` console output), the setup and the recovery tool (its yes/no prompt accepts the current language's word plus `j/ja/y/yes`).
- **Deliberately not translated**: crash reports (a file for developers → fixed English: `Cause: Panic`), technical reasons inside “index damaged: …”, traces, and the **values written to the registry / Start menu** (folder verb texts, `.lnk` names) – the R-001 journal and the state detection compare them; translating them needs an R-001-safe relabel (own TODO).
- Bug found in the extractor: a byte string with `\x89` (PNG signature in `core::inspect`) made the strict string reader give up and **silently skip the rest of the file** (6 texts missing). Fix: other strings are only skipped (any escape), and `collect` now fails loudly if a file has more `tr!`/`tr_n!`/`tr_noop!`/`@tr` calls than were read.
- CI #145: the catalog test failed on Windows only – the recovery tool's multi-line usage text had `\r\n` in the CRLF checkout while rustc reads it with `\n` → the extractor normalises CRLF (checked against a CRLF copy of all sources). CI #146 fully green, Linux E2E locally 281/281.
- Tests: all unit tests on English source texts (core/fs/ops/index/search/integrate/app), E2E/Windows checks still on German (pinned); updated: [68d] `9,8 KB`, [73]/[C2] crash report `Cause: …`.

## Localisation L1 – Slint texts ✅ (CI run #144, 2026-11-10)
- All 89 UI texts of `ui/main.slint` are now `@tr("English")` (Windows Explorer terms, US spelling like Windows: “Color scheme”, “Date modified”, “Empty Recycle Bin”, “Show protected operating system files” …); shortcuts are texts too (`Ctrl+Shift+N` ↔ `Strg+Umschalt+N`, `Del` ↔ `Entf`). Identifiers (`command("…")`, `== "trash"`, theme ids, sort keys) stay untouched.
- `lang/de/…/explorer2.po` holds exactly the German texts the UI had before → **German UI unchanged** (gallery checked by eye); `de` is marked **complete** from now on, so the catalog test makes every new text bring its German translation, and German Windows users get German automatically.
- `EXPLORER2_LANG=en` shows the English source texts for everything that comes from Slint; texts built in Rust (column headers, status bar, menus, dialogs, Inspector facts) follow in L2.
- Not translated on purpose: the developer UI kit (`ui/kit.slint`, `EXPLORER2_UI_KIT=1`), product name, file paths.
- Found for L2: the Inspector compares a group by its German label (`r.group != "ORT"`) → needs a group id; the conflict dialog's letter keys R/S/B/A match the German and the English words, not other languages; the gallery script clicks German names → galleries per language need language-neutral targets (L5).
- CI #142: everything green except [57c] “empty the trash” – the purge (5 items incl. the 60 000-file job folder) was done (`trash Purge done ok=true`), just later than the fixed 2 s → the test now waits for the done line (max. 10 s). Unrelated timing flake.
- CI #143: Linux green; Windows W10 red + cascade W11–W22 – the click on “Weitere Optionen” got lost on the runner (screenshot: our menu still open, the item hovered, no `context menu run more` in the trace); the same app code was fully green on Windows in #142. W10 now clicks once more after 3 s when the click demonstrably got lost (logged as NOTE, never silent).
- Tests: unit `compiled_german_catalog_is_used` (Rust table answers from `lang/de`), E2E [84a] German texts come from the catalog, [84d] `EXPLORER2_LANG=en` shows “Settings”/“Quick access”.

## Localisation L0 – basics ✅ (CI run #141, 2026-11-10, plan `docs/PLAN-I18N.md`, D-029)
- Boss decisions (D-029): **10 languages** (de, en, es, fr, ru, uk, pl, it, zh Simplified, ja), **English source texts** in the code (German becomes `lang/de/…/explorer2.po`), **live** language switch.
- New crate **`explorer-i18n`** (no dependencies): `LANGS` (code, own name, plural header, `complete`), `tr!("…")` / `tr_n!(n, "…", "…")` with Slint-style placeholders (`{}`, `{0}`, `{n}`, `{{`), gettext plural rules for all 10 languages (Russian/Ukrainian/Polish three forms, French 0/1 singular, Chinese/Japanese one form), own `.po` reader/writer, `build.rs` compiles the `.po` files into static tables (only translated, non-fuzzy entries). `normalize`/`resolve`: `EXPLORER2_LANG` > setting `language` > first **complete** system language > English; Traditional Chinese is deliberately not mapped to Simplified.
- Texts are found in the sources (`@tr` in `.slint`, `tr!`/`tr_n!` in Rust, comments/strings skipped); `cargo run -p explorer-i18n --example extract [-- <code>…]` writes `lang/explorer2.pot` and merges every `.po` (keeps translations, adds new texts, drops gone ones). Test `catalogs`: the template is current, every `.po` has the right plural header, no stale texts, placeholders kept, complete languages fully translated, compiled tables = files on disk.
- System language: Windows `GetUserPreferredUILanguages` (`explorer_shell::ui_languages`), elsewhere `LANGUAGE`/`LC_ALL`/`LC_MESSAGES`/`LANG`. App `lang.rs`: `init` before the window, `apply_ui` → `slint::select_bundled_translation` after the window exists (Slint's rule); trace `language <code> (setting=… override=… system=…)`.
- Slint compiled with bundled translations (`lang/`, no translation contexts). `lang/de/` exists with the header only – the app looks exactly as before (source texts are still German until L1/L2).
- Tests pinned to German: `EXPLORER2_LANG=de` in `scripts/ui-lib.sh` (all UI scripts) and in the CI/release jobs. E2E [84]–[84c]: pinned German is bundled, the setting picks `pl`, an incomplete system language (`uk_UA`) falls back to English.
- CI #138: the template test failed on Windows only (CRLF checkout) → compared line-ending neutral, `lang/**` pinned to LF. CI #139/#140: Linux E2E red from [38] – every F2 press arrived as **Alt+F2**; cause is the runner, not L0: `ubuntu-latest` switched from 24.04 (#137) to **26.04** (#139), whose xdotool/xkb sends Alt with F2. Found with the raw key trace (`EXPLORER2_TRACE_KEYS`, kept); `e2e-linux` pinned to `ubuntu-24.04`, moving to 26.04 is a TODO.
- Tests: i18n 11 unit (plural rules incl. 11–14/21/22, placeholders, tags, resolution order, POSIX order, `.po` round trip, Slint/Rust scanners, merge) + 3 catalog tests; settings round trip with `language`; Windows cross-check of `shell`/`i18n`/`core` clean.

## M8.4 Release package (2026-11-08, Boss: “mach ein Release, sodass man bequem installieren kann” – make a release that's easy to install)
- **`Explorer2-Setup-0.1.0-x64.exe`**: one file, double-click → confirmation (target, without admin; “already installed → update?”) → installation via the app itself (`--install`, D-023) → “Jetzt starten?” (start now?). Errors as a dialog; `--quiet` without dialogs (exit 0/1/2/1223).
- Plus a portable ZIP (`explorer2.exe` + `explorer2-recover.exe`) and `SHA256SUMS.txt`; release notes in German including a SmartScreen notice (unsigned).
- `scripts/release-package.ps1` builds + checks the setup in **every** Windows CI run (artifact `explorer2-windows-x64` now contains the release files); `release.yml` publishes as soon as a version without a release is pushed (creates the tag itself; the proxy blocks tag pushes from the session).
- Version 0.0.1 → **0.1.0**.

## M5 – Shell integration (in progress, Boss approval 2026-11-04)
### CI run #37 – everything green ✅ (M5.0–M5.5)
- Windows tests including real icons, shell type names, context menu verbs, drop-target data path; clipboard interop; benchmark.
- UI matrix: drop target registered (1st attempt), icons + type names loaded, drag without a pressed button is skipped.
- **Windows E2E with real keyboard/mouse: 16/16** (selection, F2 + undo, new folder, Ctrl+C → Windows clipboard, Ctrl+V, Del → Recycle Bin + undo, real shell menu, Properties dialog, real OLE drag by mouse).
- Linux E2E 104/104. Exe Windows 16.5 MB.

### M5.0 Exe size
- Analysis (Linux build as an approximation): 22.9 MB total, of which Skia ≈ 8 MB (precompiled, not affected by `opt-level`), the rest Rust (std, Slint core/renderer, winit, text shaping harfrust/skrifa/parley, SVG usvg/resvg, image decoders). Wayland/X11/zbus/sctk-adwaita (including the embedded Cantarell font) are **Linux only**. Image formats/SVG come from Slint internals → cannot be switched off cleanly.
- Measure: `opt-level = "s"` → Linux **22.9 → 19.6 MB (−15 %)**. Bench 100k (2×9 runs): first rows and total load within noise, sorting 43 → ~50 ms. `"z"` saves hardly more and is slower.
- E2E with `"s"`: 100/100, first paint 19 ms.
- CI gate: Windows exe ≥ 20 MB → build red. **Windows exe now 16.47 MB** (run #29, previously 19.7 MB).

### M5.1 Icons / M5.2 Type names
- New crate `explorer-shell`: icon keys (per extension or per file for exe/lnk/ico/url/…, folders with the R/S attribute), thread-safe `IconRegistry`, `SHGetFileInfoW` → HICON → RGBA (alpha or mask), type names via `SHGFI_TYPENAME|USEFILEATTRIBUTES` with a cache.
- App: a row carries only an `icon` id, the UI holds an `[image]` model; a worker thread loads in batches, individual icons only for visible rows (viewport callback ±20 rows). The type column shows shell names, **sorting by type uses the displayed name** (like Explorer: .jpg/.jpeg together).
- Linux: no shell → fallback squares and our own type names as before.
- Tests: shell 8 unit (key rules, registry under concurrency, pixel conversion) + Windows test with real icons (16/32 px, folder ≠ txt, own exe), core +1 (type sorting), app +3; CI UI matrix checks “icons loaded” + shell type names.

### M5.3 Context menu / M5.4 Properties
- `explorer-shell::menu`: shell context menu via `IShellFolder::GetUIObjectOf` (selection) or `CreateViewObject` (folder background) → `IContextMenu`, submenus (Send to, Open with, New) via window subclassing + `IContextMenu2/3::HandleMenuMsg(2)`. Execution by position (`MAKEINTRESOURCE`).
- Own verbs instead of the shell: rename, delete, cut/copy/paste, new folder → run through our engine (undo, progress, conflict dialog). Everything else (third-party extensions) is executed by the shell, then a refresh.
- Right-click like Explorer: an unselected row gets selected, multi-selection stays; empty space = folder background; Shift+F10/menu key at the focused row; Alt+Enter = Properties (one file: `SHObjectProperties`, several: `SHMultiFileProperties`).
- Windows test: the menu is built (without showing it) and must contain the verbs open/delete/rename/properties; background menu not empty. E2E Linux 53–53d (UI path).
- CI run #29: tests + context menu test + release build on Windows green (the app compiles with HWND code); icons load in the UI matrix (2 icons after 23 ms). Only the `kinds` trace was red: the first 200 rows in the 100k folder are all folders → the trace now searches all rows (max. 10 names).
- CI run #28: shell icons and type names run on Windows (runner English: “File folder”); only one app test expected the Linux fallback hard → made platform-aware.

### M5.5 OLE drag & drop
- Dropping: own `IDropTarget` (`#[implement]`) replaces the winit handler via `RevokeDragDrop`/`RegisterDragDrop`; paths from `CF_HDROP` (`DragQueryFileW`). Target by hit test (Slint reports list geometry + scroll per pane) → folder row = into it, file/empty = the displayed folder.
- Default like Explorer: same drive = move, other = copy, Ctrl = copy, Shift = move, limited to what the source allows; never into itself. Executed through our engine (progress, conflicts, undo).
- Dragging: `SHDoDragDrop` with a shell data object (`GetUIObjectOf`) = Explorer-compatible including the drag image; started via a timer after the Slint callback (modal loop), then the pointer release is dispatched. Windows: in-app DnD via OLE too; Linux: Slint DragArea as before.
- Tests: Windows unit test pushes a real shell data object through our `IDropTarget` (paths including an umlaut, Ctrl → copy); CI UI matrix: drop target registered + test hook `EXPLORER2_TEST_OLE_DRAG` (modal drag loop in the running event loop, no crash/hang); app unit test hit test (panes, scroll, sidebar/header).

- **CI run #31 (red)**: the drop test compared `%TEMP%` short names (`RUNNER~1`) raw with the shell's long names → the test compares canonically. Found a real risk along the way: starting via a short-name path would have defeated drop comparisons (same folder/into itself) → **navigation now expands short names** (`explorer_fs::path::long_path`, `GetLongPathNameW`, junctions stay) and drop comparisons are case-insensitive on Windows.

- **Bug (CI #32/#33, UI matrix)**: the OLE drop target was never registered (“Invalid window handle”) – the timer ran before Slint had created the native window → dropping from other apps would have been dead. Fix: registration retried until the HWND exists (50 ms, max. 5 s), otherwise fallback to in-app DnD.
- New: **Windows E2E with real keyboard/mouse** (`scripts/e2e-windows.ps1`, CI step in the release job, screenshots on failure).

### Redesign R10 – Consistency + QA ✅ (CI run #131) → redesign R0–R10 complete
- **Old colour tokens removed**: `core::theme::Tokens` = scheme + colour roles (`Design`), the old 38-token set, `tint()` and the old default colours gone, as well as Slint `ThemeColors`/`Theme.c`; the trace reports accent/selection of the new roles (E2E [70]/[70h2] updated: light #2f6feb/#dce7fc, dark #4f8cff/#2a3750).
- **Display scaling** 125 %/150 % (`SLINT_SCALE_FACTOR`, window 1280×800 physical): E2E [83] – controls present, everything inside the window, the address bar ends before ⋮; at 150 % the breakpoints kick in (narrow sidebar, actions in the ⋮ menu). Gallery: `13-skalierung-150` (Linux), `07-skalierung-125` (Windows).
- **Gallery before/after**: state R0 (commit c50a5ce) rebuilt and photographed with the same gallery; 11 screens × light/dark side by side (sent to Boss).
- **Budgets** (CI #130 against #93 before the redesign): exe 18.08 → **19.31 MB** (early-warning gate 20 MB, D-015: quality over size – watch, don't cut blindly); normal start best-of-3 **unchanged** (#93: 54.8 ms, #131: 52.9 ms; #130 showed 87.8 ms = VM variance, budget 300 ms); first start #131 59.2 ms; 100k folder first rows 3 ms. Windows gallery 18 images (including 125 %).
- **Field-test checklist** `docs/FIELD-TEST-REDESIGN.md` (8 sections, including “Weitere Optionen” dark, 125/150 %, strong accent colours).
- Locally 276/276.

### Redesign R9 – States + motion ✅ (CI run #130)
- Pass over all controls: **pressed** (`pressed`) now also for menu entries, tabs and the sidebar (previously only buttons/header/breadcrumbs); **hover transitions 120 ms** also in menus and the palette; **appearance** uniform: ⋮ menu + sort menu fade in over 120 ms (like the context menu), conflict/confirmation dialogs fade in **and out** (stay visible only while fading out), settings panel 160 ms. Disabled = `faint` everywhere (buttons, menus, palette), focus = accent frame (fields, rows, tiles).
- Nothing waits for an animation: keyboard/Enter act immediately (dialogs take input from the first frame).
- **Pixel checks of the states** (E2E [20b]/[20c]): hover #2a2a2a, selection with focus #2a3750, without focus #2c2c2c (the address bar has the focus), Esc returns the focus; plus [19], [57a] (accent mark), [7e] (tab stroke), [70b/d], W37c/W38b/W39b pass. Locally 270/270.
- CI #129: Windows W10 red – the native menu behind “Weitere Optionen” took longer than 1.5 s to build, Esc went to our window → W10 now waits for the real menu window (`#32768`) before Esc; CI #130 fully green.

### Redesign R8 – Menus + dialogs ✅ (CI run #128)
- **R8a own context menu** (decision 2): compact menu in the new style as an overlay (appears in 120 ms, stays inside the window, keyboard: ↑/↓ skips separators + disabled entries, Enter, Esc, a click outside closes). Entries on items: Open · Cut/Copy/Paste/Rename (only for 1)/Delete · Properties; on the background: New folder · Paste · Refresh · Properties (of the folder). Same commands as bar/palette (`palette::execute`). Windows: **“Weitere Optionen” (more options)** opens the native shell menu with all extensions at the same place; native menus follow the dark scheme via uxtheme `SetPreferredAppMode`/`FlushMenuThemes` (ordinals 135/136, looked up at runtime → if missing, the menu stays light). Linux has a context menu for the first time. Tests: [53]–[53f] (Esc, background, click on an entry, Shift+F10 + keyboard, multi-selection), Windows W10a/W10/W10b, gallery images.
- **R8b**: conflict/confirmation dialog on `raised` with kit buttons (main action in accent, warning icon for the conflict), crash notice with a warning icon, jobs panel with thin progress bars (width animated), command palette + quick search with a magnifier row, accent selection + bar, footer, soft scrim and appearance in 160 ms, status bar on the window surface, sort menu made of kit `MenuItem`s (check marks instead of dots), settings panel in the new colours/font sizes (std widgets stay). **main.slint no longer uses any old `Theme.c` colours** (old tokens removed in R10).
- **Installer more robust** (flake [72c], CI #107/#126: under Xvfb the installed copy sometimes got no X connection and the old window was already closed): the window closes only once the installed copy is still running after 0.9 s; if it dies immediately, a second start; if that fails too, the window stays open with an error message (before: no window any more).
- Locally 268/268 (R8a + R8b), e2e-ops 159/159; CI #126 Windows: W10a/W10/W10b green (native menu dark), gallery 16 images; CI #127 + #128 fully green.

### Redesign R7 – Inspector ✅ (CI run #125)
- “Property Inspector” on `panel`: preview as a card (radius 8, border), title 14 px semibold + type dimmed, facts in **groups ALLGEMEIN / DATUM / ORT** (general / date / location) with small headers instead of a line per row; key left dimmed (96 px), value right (selectable and copyable); the **path** full width with a button **“Pfad kopieren”** (copy path) (`TextInput.copy()`); the content fades in over 180 ms when opened (the area itself is there immediately – the list never waits for an animation).
- Rust: `InfoRow` gets `group`/`first`; `inspector::arrange` sorts stably by group and marks the first entry (also for facts delivered later like Created/Dimensions), unit test.
- Tests: E2E [68h] group headers, [68i] “Pfad kopieren” → pasting into the search field lands as search text. Locally 265/265, CI #125 fully green.

### Redesign R6 – File list ✅ (CI run #124)
- **Column header** 24 px (previously 26) on the list surface, label small/dimmed, sorted column brighter + **drawn chevron** in accent instead of “▲/▼” characters (font glyphs are the known first-start trap), hover/pressed areas, column separators turn accent-coloured while dragging/hovering; the label lines up with the names.
- **Rows** 24 px as indented, rounded tiles (hit area stays full width): hover `hover`, selection `selected` (accent glaze) + **2 px accent bar** on the left, without focus `selected-idle`, focus frame 1 px accent, **drop target** = stronger accent glaze + frame; names 13 px, columns 12 px dimmed; hit highlighting stays bold. Transitions 120 ms.
- **Tiles** in the same style (radius 6, same states), names 12 px; rubber-band selection as an accent glaze with an accent border; inline rename on `raised` with an accent border; list on `surface`.
- Tests updated to the new values (measured by pixel): selection dark #2a3750 ([19], [70d]), list #1f1f1f/#ffffff ([70b/d], W37c/W38b), column header 24 px ([65]). Locally 263/263, CI #124 fully green (including Windows pixel tests W37c/W38b/W39b).

### Redesign R5 – Tabs ✅ (CI run #123)
- Tabs in IDE style: bar on the window surface (`app`), tabs 30 px with rounded top corners; the **current tab** carries the surface of the navigation row (both read as one surface) and – only in the **active view** – a 2 px accent stroke at the top (replaces the old accent line over the whole view); title dimmed except current/hover; **× only on hover or on the current tab** (line icon, space stays reserved → no jumping), middle-click closes as before; “+” as an icon button with the tooltip “Neuer Tab · Strg+T” (new tab · Ctrl+T).
- Tabs now have `tab` roles for screen readers with the title as the name. Old `NavButton` component removed (no longer used).
- Test: E2E [7e] measures the accent stroke on the current tab by pixel (the first attempt measured a list row with the same name → now measures the tab of the current folder). Locally 263/263, CI #123 fully green.

### Redesign R4 – Navigation ✅ (CI run #120)
- Navigation row 34 px (previously 40) on `panel` with a separator line: back/forward/up as line-icon buttons with tooltip + shortcut, **address bar** as calm segments (the way there dimmed, current folder bright, chevron icon instead of “›”, hover area per segment, the frame lifts on hover, accent while editing), a click into the free area = edit as before; **search field with magnifier** (accent on focus), same keys (Esc, ↑/↓ history, Enter).
- Logic unchanged (`navigate`, `start-edit`, `search-*`); names for tests unchanged (“Zurück”, “Adresszeile” …). E2E [7d] checks the sizes; locally 261/261, CI #120 fully green.

### Redesign R3 – Sidebar ✅ (CI run #115)
- New `Sidebar` only from `Theme.d` + `Ui`: group headers small/semibold (`SectionLabel`, “Dieser PC” (This PC) stays clickable = drive overview), entries 26 px with a **line icon** (Rust supplies `PlaceItem.glyph`: home/desktop/downloads/…/trash, drives drive/usb/network), active place with a **3 px accent mark** + calm surface (`selected-idle`), hover 120 ms, text dimmed until hover/active; drives with a **2 px fill bar** (red above 90 %) and free space small underneath. Every entry has its name as `accessible-label` (tests click by name as before).
- Colours sharpened: a light hover on the window surface was invisible (#f2f2f2 on #f0f0f0) → #e9e9e9; so that selection and hover can never be confused, a grey/pale accent selection gets more accent (`apart_from`, test: channel distance ≥ 15 for all 12 accents; the grey accent was previously identical to hover).
- Tests: E2E [57a] measures the accent mark by pixel, [70d] sidebar dark = #141414. Locally 261/261.
- CI #113: Windows W10 red – the result of the native context menu came later than the fixed 600 ms (it was in the trace) → W10 now waits up to 3 s for it; CI #115 fully green.

### Redesign R2 – Main layout ✅ (CI run #110)
- **Global bar removed** (−40 px height for files). Its actions do exactly the same as before (`command(...)`), but now sit where you work:
  - **File actions as icon buttons on the right of the navigation row of the active view**, per place as before (folder: New folder, Cut, Copy, Paste, Rename, Delete; search without New folder/Paste; Recycle Bin: Restore, Delete permanently, Empty Recycle Bin) + Undo/Redo. Invisible in the inactive view (the space stays → the address bar doesn't jump).
  - **⋮ menu (“Mehr” – more)**: two views/one view, show hidden items, Inspector, command palette, settings – with keyboard shortcuts.
  - **Status bar on the right**: sorting, details/icons, Inspector, settings (icon buttons).
- **Tooltips** with name + shortcut on all icon buttons (Slint 1.18 `Tooltip`, own look `TooltipBox`).
- **Breakpoints**: window < 900 px → sidebar 160 px, Inspector 240 px; view < 800 px wide → file actions move into the ⋮ menu (e.g. with two views). Values in `changed` handlers instead of bindings (a binding to the window width was a layout loop, Slint warns “binding loop … may panic”).
- **Dark = default for new installations** (`ThemeChoice::default()`), a saved choice stays; E2E [1d] checks this, the colour tests for “Wie Windows” set `theme = system` themselves.
- Tests: `ui_action`/`UiAction` (button or ⋮ menu), “Ausgeblendete” via the ⋮ menu, the std-widget pixel test now on the settings' “Schließen” (close) button, Windows W16 by name instead of coordinate (the last fixed click coordinate on the old bar). E2E [1e] checks the new layout. Gallery: ⋮ menu + tooltip.
- Locally: e2e-ops 153/153; in the first full run [49] (first drag) and [72c] (the installed copy got no X connection, also in CI #107 with old code) were red once, a single run + CI #110 green → watched as flaky. The first frame briefly started with a narrow sidebar (width 0 before the window) → fixed.
- CI #110: everything green, Windows gallery 14 images (including the ⋮ menu), W16 by name.
- Deviation from the plan: the Inspector does **not** close itself in a narrow window (that would change a saved state = behaviour), it only gets narrower.

### Redesign R1 – Design system ✅ (CI run #104)
- **Colour roles** `core::theme::Design` (field `ds` on `Tokens`, 26 roles: layers app/panel/surface/raised/overlay, states hover/pressed/selected/selected-idle, lines, text hierarchy text/text2/muted/faint, icons, accent + text on it, signals, folder/file, scrim/shadow). Dark = layered greys instead of black, accent calm blue `#4f8cff` (light `#2f6feb`). The Windows accent is adopted and pushed towards readability; the selection is a thin accent glaze that is thinned out so much for garish accents (yellow on dark) that it stays as bright as with the default blue.
- **Contrast test** for the default + 12 accents × light/dark: text ≥ 7 on every layer, text 2 ≥ 4.5, dimmed ≥ 4.5 (in selected rows ≥ 3), icons ≥ 4.5, accent ≥ 3, text on accent ≥ 4.5, signals ≥ 4.5, layers distinguishable. Several starting values failed this and were adjusted (e.g. dimmed on “pressed” 4.35 → brighter).
- **Slint**: `ui/theme.slint` (`ThemeColors` old, `DesignColors` new, `Theme.d`, global `Ui` with sizes/radii/font sizes/durations), `ui/icons.slint` (50 Lucide icons as path data, generated by `scripts/make-ui-icons.py`; bug found: a lower-case `m` at the start of a sub-path becomes relative after concatenation → becomes `M x y l …`), `ui/kit.slint` (`Icon`, `IconButton`, `ToolbarButton`, `MenuItem`, `MenuSeparator`, `SectionLabel`, `Field`, all with a meaningful name).
- **UI kit** `EXPLORER2_UI_KIT=1`: overview of all colour roles, font levels, row/menu patterns, controls in their states and all icons → gallery image `12-ui-kit` (Linux) or `06-ui-kit` (Windows), E2E [81]/[81b].
- Locally: Linux E2E 258/258 green; CI #104: everything green (tests Windows + Linux, E2E Linux, Windows release + E2E + gallery with the UI kit image).
- Visible look of the app unchanged (old `Theme.c` values untouched); icon license in `THIRD-PARTY.md`.

### Redesign R0 – Safeguards ✅ (CI run #101)
- Locally: Linux E2E fully green (after a popup fix: the sort menu lives in its own popup tree, the dump now walks it with an offset). CI #101: everything green, gallery artifacts `ui-gallery-linux` (22) and `ui-gallery-windows` (10, including the native context menu) = the “before” state.
- Boss: all 5 recommendations accepted (dark as default, own context menu + “Weitere Optionen”, dissolve the global bar, system font, calm blue).
- **UI dump** (`app/src/uidump.rs`): test mode `EXPLORER2_UI_DUMP=<file>` writes all named elements (`accessible-label`; Slint gives every `Text` its content as a label → also file rows, column headers, sidebar) with window position, on change and every second. Spike: the public Slint test API (`ElementHandle`) finds **nothing** without compiled-in debug info (`element_count()` = 0) → a direct walk through the item tree via `i-slint-core` (exact version like Slint, no debug info, no size growth).
- **Tests click by name**: `scripts/ui-lib.sh` (`ui_click "Einstellungen"`, `ui_click "Größe" last`, `ui_click_after "Adresszeile"`, `ui_box`, `ui_list_top`); all bar, panel, menu, sidebar and jobs clicks converted; rows still by index, but relative to the column header from the dump. Windows: `UiBox`/`UiClick`/`ListTop`, `RowY` relative.
- **Meaningful names** for icon buttons (they stay when the characters become symbols): “Zurück”, “Vorwärts”, “Nach oben”, “Neuer Tab”, “Tab schließen”, “Adresszeile” (back, forward, up, new tab, close tab, address bar) – for screen readers at the same time.
- **Gallery** `scripts/gallery.sh`: 11 screens × light/dark (folder, Inspector, multi-selection, two views, tiles, search, quick search, palette, settings, confirmation, narrow window) → CI artifact `ui-gallery-linux`; Windows 5 screens × 2 (including the native context menu) → `ui-gallery-windows`. That is the “before” state.
- E2E [7b]/[7c] check the dump itself.

### Redesign – plan created (2026-11-07)
- Boss: field test of the search on real Windows passed (“es funktioniert” – it works) → S2 not needed for now.
- Next project: UI/UX redesign in the File Pilot style, **design/feel only**. Plan with current-state analysis, design system, target layout, stages R0–R10 and 5 open decisions: `docs/PLAN-REDESIGN.md`.
- Most important finding: 140 px of chrome up to the first row (goal ~90 px), ~100 fixed click coordinates in the E2E tests → R0 makes click targets nameable first.

### Phase 2 – S1.5c Drive selection ✅ (CI run #93) → **S1.5 complete, S1 complete**
- CI #92 red only in the clipboard interop test (Windows): .NET `GetFileDropList` briefly saw the clipboard that had just been set as empty, our reading and `Get-Clipboard` saw the files → the test now reads with up to 5 attempts (like an app). #93 green, locally Linux E2E 254/254.
- **Next step: field test on real Windows** (`docs/FIELD-TEST-SEARCH.md`), then the decision on S2.
- `core::index_roots` (`Candidate`, `chosen`, `toggle`): setting `index_roots` empty = **automatic** (Windows: fixed local drives; Linux: home folder), otherwise a `;` list; `;` alone = explicitly none. If the choice becomes equal to the automatic one again, the setting goes back to empty (disks added later are included). Listed drives that are currently missing (USB unplugged) stay in the list but are not indexed.
- Settings “Suche” (search): all drives with check boxes (drive detection in the background), network drives/removable media/others can be added, DVD/BD never; **above** the status line (click targets stable). Changing a check box = save + restart the service (the old one stops and saves, then start with the new choice). `EXPLORER2_INDEX_ROOTS` still wins, `EXPLORER2_INDEX_CANDIDATES` replaces the drive detection in tests.
- A test found a real bug: `;` (explicitly none) was treated like “automatic”.
- Tests: unit (automatic, on/off, back to automatic, missing drives, Unix case), fuzz, E2E [80]–[80f] (both automatic, list in the settings, deselect → restart without it, its files no longer found, choice stays after restart, on again = automatic).

### Phase 2 – S1.5a/b Local time, watcher status, highlighting, search history ✅ (CI run #91)
- **Local time**: `core::local_offset_at(t)` (offset of *this* date → summer/winter time correct, via `chrono`, which is in the build anyway through Slint) + `format_local` for the list, Inspector, dialogs; `dm:` queries compute “today” in local time. Test with `TZ=Europe/Berlin`: January +1 h, July +2 h.
- **Watcher status**: `Pending::lost(reason)` counts missed events (buffer overflow, watcher errors) → status “Änderungen 2-mal verpasst, danach abgeglichen (zu viele Änderungen auf einmal)” (changes missed twice, reconciled afterwards (too many changes at once)); `watch::describe` turns notify errors into text (inotify limit: “fs.inotify.max_user_watches erhöhen” – increase fs.inotify.max_user_watches).
- **Highlighting**: `core::highlight` (`pieces` = literal pieces of the query without `ext:`/`!`/wildcards, `find`/`split` case-insensitive and character-exact) → the row gets `hl-pre/hl-hit/hl-post`, the hit is drawn **bold** (Slint cannot format text partially). In the random test: the pieces always reassemble the name.
- **Search history**: `core::searches` (`parse`/`serialize`/`push`/`step`, max. 20) in `search-history.txt`; saved on Enter in the search field, when opening a hit, in the quick search. ↑/↓ in the tab search field like in a shell (without changing the order), ↓ at the end = into the list. The quick search shows the last searches when the field is empty, Enter takes one into the field.
- Sorting by column header in hits: searches again in that order (it already did, now tested).
- Slint finding: a `let` variable used twice → the generated Rust code moves it (“borrow of moved value”) → second use via `self.text`.
- Disk full (sandbox): `target/debug` + old E2E working folders (100k files each) in the scratchpad deleted.
- Tests: E2E [79]–[79f], unit: local time (+ Berlin), highlighting, history, status line; fuzz: highlighting + history file.

### Phase 2 – S1.4c Global quick search ✅ (CI run #90) → **S1.4 complete**
- Ctrl+Shift+F (or palette “Alle Laufwerke durchsuchen” – search all drives): overlay like the command palette (`CommandPalette` with its own hint, footer, empty text and a second action Ctrl/Shift+Enter). Searches across **all** indexed drives after a 120 ms pause, older queries are discarded.
- Ranking of the best **50**: name = text (also without extension) → starts with it → shorter name; with query language (`ext:`, `*`, `|` …) the index order stays. Folder truncated on the left (“…/Dokumente/Bericht”), because the end makes the difference.
- Enter: open a folder in the active tab, a file with its program. Ctrl+Enter: show in its folder and select it (`nav::show_in_folder` via `pending_select`). Footer: “2 Laufwerke, 1.234.567 Einträge” (2 drives, 1,234,567 entries) or “Suchindex ist aus” (search index is off).
- **CI #86/#88/#89: [71e] red, locally always green.** The first suspicion (the dialog losing focus) was wrong; the end of the trace from CI (#89, new diagnostics) showed: the click on the reset button produced no dialog at all, but `selection … count=0` = a click **into the list**. Causes: (1) with the new section “Suche” above “Windows-Integration”, the button position depended on the search status line, which wraps with the runner's fonts (locally just one line) → click on text instead of the button; (2) **the panel let clicks on empty areas through to the list** (a real UX bug: a click into the panel cleared the selection); (3) [71d] was a **false green**: `last '…' >/dev/null` returns the exit code of `tail`, not of `grep` – the same pattern in 5 more checks. Fixes: the panel swallows clicks, “Suche” comes after “Windows-Integration” (click targets as proven in CI, 502), shorter status line, all `last … >/dev/null` → `grep -q`. The dialog additionally keeps the focus (doesn't hurt, but wasn't the cause). Windows: [W44] 7/7, [W45] new.
- Tests: E2E [78]–[78f] (open, `tief` → tief.pdf, Enter opens, `datei2` → datei2.txt before datei20–29, Ctrl+Enter shows it selected, Esc), Windows [W45]/[W45b], unit: ranking, left-side truncation.

### Phase 2 – S1.4b Search field per tab ✅ (CI run #90; Windows [W44] 7/7)
- Field to the right of the address bar (“Suchen in <Ordner>” – search in <folder>), Ctrl+F / Ctrl+E / F3 (also palette “Suchen”). Typing searches after a 250 ms pause; Enter immediately; ↓ into the list; Esc or an empty field leads back to the folder.
- A search is a navigation to the placeholder `::suche:<folder>\u{1f}<query>` (`core::places::search_path`/`search_of`). That gives history, tab title “Suche: …” and breadcrumb “Suche „…““ for free. The first search from a folder = one history step; typing further replaces it; leaving takes the folder off the stack again (`History::return_to`), so no trace. The session saves the folder, not the search.
- Hits: `VirtualData::Search { paths, total, walked }` with a full path per row (names repeat across folders). Thereby `selected_paths`/`entry_at`/`inspect_target`/open/thumbnails work → open, copy, cut, rename, delete, drag, context menu, Inspector. Paste and New folder are disabled in hits (`pane_dir` = None). Own bar for search hits.
- Column “Ordner” (folder) relative to the searched folder (`a/b`), across all drives the full path. At most **20,000** rows (sorted beforehand): “20.000 von 123.456 Treffern – Suche verfeinern” (20,000 of 123,456 hits – refine the search).
- `search::find`: the index where it covers the folder (`Volume::search_with`: hits + names/paths under the same read lock, the root and the searched folder itself are not hits). Otherwise (index off, building, folder on a non-indexed drive) the folder is walked with `walk::build` – same query language, cancellable when typing further; status “(Ordner durchsucht, ohne Index)” (folder searched, without index).
- Live: index change → visible search tabs refreshed (at most every 2 s, the last change always arrives). Without an index: `refresh_dirs` reloads walked searches when something below changes.
- Bugs found: Slint binding loop (width of the search field from the layout width → “Recursion detected”, panic on start) → preferred/minimum width. The target was made absolute before placeholder detection (`/home/…/::suche:…` → “Ordner nicht gefunden” – folder not found).
- Settings panel: “Schließen” (close) now sits fixed at the top right (reachable while scrolling, no longer moves). The E2E click aimed at the old position → cascade [63d]–[71f2] in the local run, fixed.
- Tests: E2E [76i] (walk without index), [77]–[77k] (Ctrl+F, one search per pause, columns, opening a hit from another folder, deleting into the Recycle Bin, hits follow the index, `ext:pdf`, Esc, no trace in the history, session); Windows [W44]; unit: status line, folder column, sorting of hits, `search_path` round trip, `History::return_to`.

### Phase 2 – S1.4a Search service in the app ✅ (CI run #90)
- `app/src/search.rs`: the service starts **3 s after the window** in its own thread (drive detection can block), never in the start path. Drives: fixed local ones (Windows), otherwise `$HOME`. On exit it waits until saved (the window is already closed by then).
- **Lock file `index/lock`** (`File::try_lock`): a second window (`--new-instance`, “single instance” off) does not co-maintain the index, otherwise two processes would write the same files. Off/on in quick succession: the new start waits until the old one has saved (never two services on the same files).
- Settings “Suche” (search): on/off (`index_enabled`), status per drive (“bereit, 1.012.345 Einträge (aufgebaut in 3,4 s)” – ready, 1,012,345 entries (built in 3.4 s), “wird aufgebaut … 120.000 Einträge” – building … 120,000 entries, without live updates + reason, save errors), button “Neu mit der Platte abgleichen” (reconcile with the disk again). The panel **scrolls** when the screen is too small (on Windows with the integration and installation blocks otherwise > 768 px).
- Palette: “Suchindex neu mit der Platte abgleichen” (reconcile the search index with the disk), “Suchindex ein/aus” (search index on/off).
- Tests: E2E [76]–[76h] (build after “event loop running”, file saved, palette reconcile/off/on → loaded, second window excluded by the lock, the next start loads + finds an offline change, `EXPLORER2_INDEX=off`). All other app tests run with `EXPLORER2_INDEX=off` (CI job-wide), so the index doesn't disturb their timing measurements.

### Phase 2 – S1.3 Build + reconciliation + live updates ✅ (CI run #83)
- **CI #83 measuring runs Windows (40 s each):**

  | Measuring run | Entries/s | CPU |
  |---|---|---|
  | `C:\Windows` cold | 5 363 | 10 % of one core |
  | `C:\Windows` **warm** | **206 928** | 305 % |
  | `Program Files` cold, normal priority + 32 threads | 3 133 | 4 % |
  | `Program Files (x86)` cold, normal priority | 5 023 | 7 % |

  The **cold VM disk (IOPS) is the brake**, not our code and not the priority → “below normal” stays. Warm, 1 M would take ~5 s. For very large drives S2 (MFT, sequential) remains the way.
- **Gate `D:\` (Windows):** live 204/203 ms, load 0 ms, reconciliation 22 ms.
- **Linux `$HOME`:** live best 29 ms, load 4 ms.
- New crate `explorer-search`:
  - `walk`: parallel initial build. Reader threads read folders, one collector writes into the index. Links are listed, never entered.
  - `refresh`: reconciliation after loading. A folder is only re-read if its modification time differs or is ≥ save time − 2 s; otherwise it costs one `stat`. Runs in slices so searches keep running.
  - `live`: a changed path → index. The disk decides, never the event type. From 32 events in one folder, it is read once instead of looking up every name individually.
  - `watch`: recursive watcher → set of changed paths (duplicates count once), debounce 200 ms / max. 600 ms. On overflow or watcher error, a reconciliation of the whole drive follows. More than 200,000 paths count as overflow.
  - `service`: one maintenance thread per drive, on Windows with priority “below normal” (the reader threads too). Sequence:
    - load the saved index: searchable immediately, then watcher, then reconciliation
    - otherwise: watcher, then initial build
    - afterwards apply changes in cycles, reorder from 5 % unsorted part, save every 5 min and on exit
- Locks: only the maintenance thread writes, in slices of max. 15 ms. Reordering and saving run under the read lock (`Index::reordered` builds a copy, then it is swapped). Ids change only with a new **generation**; `path_of(gen, id)` returns None for an old generation, then search again.
- **Only an exact state is saved.** If a change is cut off on exit (large folder half inserted), nothing is saved. The old file stays, the next start finds the difference. Save time = last exact moment − 10 s.
- Volume ID `fs::places::volume_tag`: Windows root + file system + serial number, Unix device number. A freshly formatted drive never gets a foreign index.
- Real race found (test under load, 1 in 20): notify/inotify reports a new folder **before** it watches it (`handle_inotify`: events first, then `add_watch`). Files from that moment never arrive as an event. Fix: on Linux, a folder in which something is new is reconciled once more 1 s later (`RECHECK_NEW`, subtree `Refresh`). Windows watches the tree with one handle and doesn't need this. Afterwards 72/72 runs with 6 processes in parallel green.
- Bug found and fixed: `Index::find` panicked on a path shorter than the root. New random test `random_paths`.
- Tests:
  - Service: build, save, load and reconcile on the next start = fresh build
  - Live (new, renamed, deleted, excluded, 96 files at once, folder with contents pushed in from outside, size changed) = fresh build
  - damaged / foreign drive / foreign folder → rebuild
  - reorder → new generation, `rescan`
  - exit immediately; root that doesn't exist
  - 25 runs in a row stable
- **CI #82 red:** the cold system disk of the Windows VM managed only ~1,100 entries/s during the initial build of `C:\` (1.01 M after 15 min, not finished). Linux CI: 540,000/s. `read_dir` is a plain `FindFirstFileExW`. Suspicion: IOPS limit of the VM disk (random reads per cold folder) or displacement by “below normal”. Now measured instead of guessed: measuring runs on `C:\` (cold / warm / normal priority + 32 threads, 40 s each, wall clock against process CPU); the gate runs on the drive of the working copy (`D:\`, local SSD).
- CI example `examples/drive.rs` (originally: Windows indexes all of `C:\`) (gate: 1 M < 2 min extrapolated, live change < 1 s, load < 1 s). Linux `$HOME` only with numbers. Locally (`/home/user`, 30k): live 202 ms, load 3 ms, reconciliation 102 ms.
- Linux: the recursive inotify watcher needs one watch per folder; setting it up took 5 s for 4,230 folders in the sandbox. That's why it starts only after loading.

### Phase 2 – S1.2 Saved index + exclusions ✅ (CI run #80: load 1 M Linux 88 ms / Windows 69 ms, save 461 / 862 ms)
- `persist.rs`: one file per drive (`file_name_for(root, case_insensitive)`), format `EXPL2IDX` + version + volume ID (`tag`, e.g. root + serial number) + save time (for the later reconciliation “what has changed since then”) + all columns raw (little-endian) + 64-bit checksum. Written atomically (temp file, `sync_all`, rename).
- **Loading checks everything** and says why (`LoadError::Missing/Outdated/OtherVolume/Damaged/Io`): checksum, every bound and length before allocating memory, every name between two separators, parents in range + alive, **no cycles** in the folders (otherwise `path()`/`is_under()` would hang), sorted part strictly increasing and congruent with the clean entries (the buffer search relies on that), extension ids, flags, roots. Child lists are rebuilt from the parents when loading.
- Tests: round trip (including changes after sorting: tail, renamed, deleted; saving what was loaded = the same bytes), file + ID, **every truncation and every flipped bit is rejected**, and “damaged, but checksum matches” (checksum bypassed) → either rejected or usable without panic/endless loop (locally 200,000 rounds).
- `exclude.rs`: entries with a separator = path (folder + everything below, prefix only at separators), without = name pattern (`node_modules`, `*.tmp`, `~$*`), Windows case-insensitive, `/` = `\`; defaults always included (Windows `$Recycle.Bin`, `System Volume Information`, `$WinREAgent`; Linux `/proc` `/sys` `/dev` `/run` `/snap`); setting `index_exclude`.
- Benchmark (1 M): file **69 MB**, save **179 ms**, load **116 ms** (CI gate < 1 s).

### Phase 2 – S1.1 Index core ✅ (CI run #78)
- New crate **`explorer-index`** (platform-neutral): entries as column arrays (parent index, name, size, date, flags, child list), lower-case names **contiguous in one buffer** (`\0` in between), original names only where they differ. Insert/change/rename/move/delete for live updates; `find(path)` (Windows case-insensitive).
- **Query language** (Everything subset): AND (space), OR `|` (binds tighter), NOT `!`, `"phrase"`, `*`/`?` (whole name), `ext:`, `size:` (units, ranges, `empty…gigantic`), `dm:` (`today`/`yesterday`/`thisweek`/`lastmonth`/`2026`/`2026-11-05`, `>`/`<`/`..`, local time), `file:`/`folder:`, `path:`, `case:`, `regex:` (its own `|` stays in the expression).
- **How fast, and why**: (1) data lies physically in **natural name order** (`rebuild_order`) → sequential memory access, and hits are ordered by name without sorting (previously 680 ms of sorting for 293,000 hits); (2) substring/prefix/suffix/`ext:` = **one SIMD pass** (`memmem`) over the name buffer, hit → entry by binary search, only those candidates are fully checked; (3) `path:word` without a separator = precomputed once per query (own name **or** an ancestor contains it, O(n) memoised) instead of building paths (1,742 → 21 ms); (4) parallel blocks across all cores. Changes after re-sorting go through a small unsorted rest that is merged in when searching.
- **Benchmark** (1 M realistic entries, depth ≤ 12, 4 cores, best of 9): index **82 MB**; name queries **0.7–5.3 ms** including name order (`bericht` 2.8 · `*.jpg` 3.7 · `urlaub|foto` 3.9 · `ext:jpg;png;mp4` 2.3 · `size:>32mb` 4.2); `path:garten` 20 ms, `regex:^img_\d{3}\.jpg$` 7.7 ms. CI gate: name queries < 10 ms, `path:`/`regex:` < 60 ms, < 100 MB.
- **CI #78 green** (1 M, best of 9): Linux worst name query **3.2 ms** (`b`), `size:>32mb` 2.8, `ext:` 2.3, `regex:` 6.1, `path:` 21; Windows **5.1 ms** (`b`), `size:` 4.5, `ext:` 3.5, `regex:` 8.4, `path:` 27 – budget 10 / 60 ms with headroom.
- **Windows endurance-test mystery solved (#75/#76/#78)**: “keys arrive, nothing happens” was **Notepad** (34× in the foreground): each round went one folder further, `Home` came only every 20 rounds → every 20th round the selection sat **behind** the 20 folders on a changing file, and `Enter` opened it (correctly) in the default program. A test bug, not the app: `Home` every 19 rounds (Linux + Windows), new check [S7] “no foreign window came to the front”.
- **CI #77**: Windows VM 2.5× slower than usual (build 543 instead of ~200 ms) → `size:>32mb` 10.32 ms. Budget not coupled to the VM (that would be loosening), but faster code: a single-thread measurement showed **~10 ns per entry** for a size comparison = iterator/closure load → column loop as a plain loop (single condition directly), buffer hits via continuous SIMD iterators (`memchr_iter`/`find_iter`, hit within an already covered name = one comparison) instead of restarting per hit, results reserved upfront → locally **all name queries ≤ 2.9 ms** (4 threads; `b` 2.9 · `size:>32mb` 2.6 · `ext:` 2.0 · `bericht` 0.8). Measuring switch `EXPLORER2_INDEX_THREADS`.
- **CI #75**: benchmark green on both (Linux worst 9.96 ms = `b`, Windows 6.3), but too tight → single-byte search via `memchr`, `folder:x`/`file:x` (kind AND text) drive via the text, wildcards with a fixed start (`img_1*.png`) via `\nimg_1`, mapping hit → entry galloping instead of binary search, and a query that is **exactly** one word/prefix/suffix takes over the buffer hits without re-checking → locally **all name queries ≤ 4.8 ms** (`b` 4.8 · `bericht` 1.3 · `img_1*.png` 0.7 · `folder:projekt` 1.5). Windows endurance test #75: after ~115 navigations no more keys arrived (focus gone, memory/handles clean) → check the foreground before every round + take the focus back, focus thieves are logged.
- **CI #74 red → improved**: Linux runner ~1.4× slower, `ext:jpg;png;mp4` 10.39 ms > 10 (Windows 6.8 green), `regex:` 50 ms tight. Budget not softened, instead: (a) **extension id** per entry (u16, +2 MB) → `ext:` compares numbers in an array (8.1 → 2.3 ms); (b) `regex:` via **mandatory literals** (`regex-syntax` extractor, ASCII only) as a prefilter in the name buffer, regex only on the candidates (18 → 7.7 ms). Measured and rejected: regex in multi-line mode over the whole buffer is 4× **slower** than individual checks (447 vs 117 ms). Separator in the buffer now `\n`, long `ſ` is folded to `s` for the search (otherwise the literal prefilter misses `(?i)s`).
- **Tests**: 11 unit (paths, logic, filters, scope, sorting, live changes, compaction), `tests/agree.rs` (fast paths = simple path across 5 trees, 41 queries, after rename/move/delete/create, with folder scope, after re-sorting again), `tests/fuzz.rs` (random queries: no panic + both paths equal; locally 90,000 rounds). Found along the way: `|` in a regular expression was split as OR → fixed.

### M8.3c Random-data tests + endurance test ✅ (CI runs #72/#73) → M8.3 complete
- **Random-data tests** (deterministic, own xorshift generator, no new dependency): all file parsers (settings, session, views, start guard, instance protocol) and text helpers (address bar, breadcrumb, palette search, display name/rename, image size, text preview, colour) with random text and damaged real files. Properties: no panic; what was read in survives save + load unchanged. Locally 420,000 rounds each over 6 seeds clean; CI 4000 per test (~1 s).
- **Journal (R-001)**: a real journal (all integrations on) – **every** truncated version is detected as damaged (except for the last `\n`, content + checksum are complete then), no modification gets through, reads back exactly; locally 400,000 rounds over 4 seeds.
- **Findings + fixes**: (1) `views.ini` with a single `\r` at the end of the file → path with `\r`, changed on the next save → truncated; (2) `session.ini`: tab paths with `\r` were read but never written → discarded when reading; (3) journal: content **after** the checksum line was unprotected (only possible by manual editing, our saving is atomic) → now counts as damaged; the checksum must be exactly in the written format (a leading zero previously counted as equal).
- **Endurance test** Linux (`scripts/e2e-stress.sh`, called from e2e.sh): 150 rounds of open folder/up, tab open/close, every 5 rounds tile ↔ details view, every 7 rounds a file change in the watched folder; measurement after 30 warm-up rounds vs. the end. Locally: 447 navigations in 75 s, **RSS +0.5 MiB (24.2 → 24.6 MiB), files ±0, threads ±0**. Windows (`scripts/m8-stress.ps1`, real keyboard): the same with private bytes, handles, threads, GDI/USER objects (icon leaks).
- **CI measurements**: Linux #72: RSS 25.2 → 25.5 MiB (448 navigations), files 20 ±0, threads 8 ±0. Windows #72: private bytes +14.2 MiB → looked at more closely: #73 with two measuring phases – warm-up 58.4 MiB (image cache 37.5), after 120 rounds 71.6 (cache 50.0), after 240 rounds 72.4 (cache 50.0) → **without the image cache phase A +0.7 MiB, phase B +0.8 MiB = no leak**; the growth was the tile image cache (LRU, budget 96 MiB, stops at 50). Handles 588 → 586, threads 16, GDI 53 ±0, USER +3.

### M8.3b Single instance ✅ (CI run #71)
- If a window is already running, a second start (folder double-click via M7.1, Win+E, Start menu, command line) hands its folder over to it: **new tab** in the active view (without a folder = like Ctrl+T), a minimised window comes back up and to the front (`AllowSetForegroundWindow` from the new process), the second process exits immediately – before window, renderer and start guard.
- Channel: Windows named pipe `\\.\pipe\explorer2-<session>-<hash>` (local clients only, default ACL = same user, the next pipe instance is created before the current one closes → the name never disappears), Linux Unix socket in the temp folder, permissions 0600. Name per config folder → tests/portable copies with their own `EXPLORER2_CONFIG_DIR` don't disturb each other. Protocol purely in `core::instance` (one line, anything foreign is discarded).
- **No lock-out**: “ok” comes only when the UI thread has really opened the tab; if the running instance doesn't reply (hangs), the second start opens its own window after 4 s. Whoever doesn't get the name (`--new-instance`, old instance just closing) retries every 2 s and takes over afterwards. Linux: SIGTERM frees the socket immediately.
- Can be switched off: Settings → Windows-Integration → “Ordner im offenen Fenster öffnen (als neuen Tab)” (open folders in the open window (as a new tab)) + palette; `--new-instance` / `EXPLORER2_NEW_INSTANCE=1` for a single start. The installer restart uses `--new-instance`.
- Tests: core 3 unit (protocol, foreign input, stable name), Linux E2E [74]–[74i] (hand-over < 3 s, new tab, without folder, instance stopped via SIGSTOP, `--new-instance`, switch, socket gone after kill), Windows `scripts/m8-instance.ps1` [N1]–[N9] (named pipe, minimised window, instance suspended via `NtSuspendProcess`, `--new-instance`, pipe free after the process ends).

### M8.3a Crash report + start guard ✅ (CI run #70)
- **Crash report**: a panic in any thread (panic hook) and on Windows unhandled exceptions (`SetUnhandledExceptionFilter`: code, address as `exe+0x…`) write `%APPDATA%\Explorer2\crash\crash-<UTC>-<pid>.txt` – version, cause, thread, message, location and the **last 200 log lines** (always kept in memory, even without `EXPLORER2_TRACE`). No “has stopped working” dialog. Max. 10 reports.
- Next start: notice card at the bottom (overlay, moves nothing) “… wurde beim letzten Mal unerwartet beendet” (… was closed unexpectedly last time) + “Bericht anzeigen” (show report; opens the folder); every report is reported once (`.seen.txt`).
- **Start guard** (`start-guard.txt`, logic purely in `core::guard`): a start that doesn't survive 5 s counts as aborted; after **2 in a row** the next start is **safe** (software rendering, no session, no GPU check) – a broken GPU driver or a session entry that crashes locks nobody out. If the safe start runs stably, the next one is normal again. A second running instance (same exe, file < 60 s old) is detected, PID reuse by foreign processes too (process name). Linux: SIGTERM (`kill`, logout) is an intended end, not a crash.
- **GPU check that takes the process down** (driver crash): marker `gpu-probe.txt` → on the next start the GPU is permanently “unsuitable” (otherwise an endless loop of crash – safe start – crash).
- Test hook `EXPLORER2_TEST_CRASH=panic|native|panic-later`. Tests: core 5 unit (counting, PID, UTC format, cleanup, texts), app 3 unit, Linux E2E [73]–[73j], Windows `scripts/m8-crash.ps1` [C1]–[C8] (real Windows exception via `RaiseException`, panic, safe start, GPU marker).

### M8.1/M8.2 Installer ✅ (CI run #69)
- Boss decision: **own installer in Rust** (D-023), per user, without administrator rights.
- `explorer2.exe --install [--quiet]` or Settings → “Installation” → “Installieren” (install; with confirmation): copies app + emergency tool to `%LOCALAPPDATA%\Programs\Explorer2`, creates “Explorer 2.0” and “Explorer 2.0 – Windows-Standard wiederherstellen” (restore Windows default) in the Start menu (`.lnk` via `IShellLinkW`), registers itself in **Apps & Features** (version, size, uninstall) and then continues running from the installation folder. Start menu + entry go **through the journal** – the one-button reset and `explorer2-recover` remove them as well.
- Integrations that were switched on from a downloaded copy are redirected to the installed exe (only clearly our own; ones taken over from others stay).
- Update over a running app: the running exe is renamed aside (`*.old-<pid>`), cleaned up on the next start.
- Uninstall (Apps & Features / `explorer2-recover --uninstall [--quiet]`): complete R-001 reset (all integrations, Start menu, entry), then a temp copy deletes the program folder – **only if it contains exclusively our files** (test with a foreign file) – and itself; settings stay.
- Tests: integrate 24 unit (install → uninstall = snapshot, update = one entry + the first original, integrations follow the installed exe, missing files → nothing journaled, a foreign folder is never deleted, entry in the manifest), Linux functional test + E2E [72] (palette → confirmation → the installed copy runs, the old one closes; uninstall cleans everything up), Windows CI `scripts/m8-install.ps1` [I1]–[U6] with the real default paths.
- CI #68: Windows fully green (I1–U6), Linux [72b–d] red – the E2E job only built `explorer2`; without `explorer2-recover` next to it the installer refuses (intended). Fix: the job builds `-p explorer-integrate --bins` too → #69 green.

### M7 ✅ complete (7.1 + 7.2); M7.3 IFEO postponed (Boss, D-022) – next step M8 installer + recovery

### M7.2 Win+E ✅ (CI run #67)
- Settings/palette: **“Win+E öffnet Explorer 2.0”** (Win+E opens Explorer 2.0) – `HKCU\Software\Classes\CLSID\{52205FD8-5DFB-447D-801A-D0B52F2E83E1}\shell\opennewwindow\command` = `"…\explorer2.exe"` (without a folder → last session) + empty `DelegateExecute` (otherwise Explorer's COM handler stays). Via journal/manifest like M7.1, off = exactly as before (`CLSID` itself is only removed if we created it and it is empty).
- Windows E2E W43–W43h: real Win+E via `keybd_event`; beforehand a check whether the runner shell processes Win+E at all (otherwise only a registry check + NOTE); on → Explorer 2.0 starts, no Explorer window; off → CLSID key identical, Win+E opens Windows' Explorer again.
- CI #65/#66: Win+E already started Explorer 2.0 correctly (process started by the shell, without a folder, no Explorer window); only test bugs were red – the trace file isn't inherited by the shell, PS 5.1 counts a single `CimInstance` as `$null`.

### M7.1 Open folders + context menu ✅ (CI run #64; functionally already #63)
- **CI #63**: Windows E2E **81/81** including W42–W42m: real ShellExecute on a folder and on `C:\` starts Explorer 2.0 there, the context menu shows “Öffnen (Explorer 2.0)” (default) + “In Explorer 2.0 öffnen”, `InvokeVerb` opens, `Folder` untouched, both off → `reg export` identical; R-001 snapshot including [R5]–[R5e]; Linux E2E 188/188. Only the first-start gate was red (D-018): 3305 ms at `probe_ms` **1189** (usually 70–170), `window created` already at 897 ms before any app code (usually ~150) → VM; normal start best-of-3 **86.8 ms** (budget 300). Boss: couple the gate to the VM (red from 2 s only at `probe_ms` < 300, otherwise a warning) → D-018 extended.
- CI #62: W42 didn't run (PS 5.1 + `reg export` on a missing key = abort, test script bug) → fixed; `[forced-gpu]` hung once after Skia's GL fallback (505 ms again in #63).
- Settings → “Windows-Integration”: **“Ordner mit Explorer 2.0 öffnen (Doppelklick, „Ordner öffnen“)”** (open folders with Explorer 2.0 (double-click, “Open folder”)) and **“„In Explorer 2.0 öffnen“ im Kontextmenü”** (“Open in Explorer 2.0” in the context menu), individually on/off (also via the palette); only visible on Windows. Off = exactly the state before (only the values of this function, the rest stays in the journal).
- Open folders: own verb `explorer2open` as the **default verb** of `Directory` and `Drive` (Windows ships `none` there; without our own default verb everything would fall back to `Folder` = Explorer). `Folder` (Control Panel, Recycle Bin, …) stays with Windows, Windows' `open` verb untouched.
- Safety: exe moved/deleted → on the next start the entries are redirected to the running exe (otherwise “Windows cannot find …” on every folder); when switching on, a copy of `explorer2-recover.exe` goes to `%LOCALAPPDATA%\Explorer2` (survives deletion of the app folder); notice “nicht verschieben” (don't move).
- Found: drives arrive as `"C:\"` → after Windows quoting `C:"` → `launch_arg` repairs it (test).
- Tests: integrate 18 unit (among others functions individually on/off = snapshot, moved exe, a default verb taken over from others stays, sweep without a journal cleans up all functions, `Folder` never touched, every change in the manifest); R-001 CI test [R5]–[R5e] (functions back with/without a journal = snapshot); Windows E2E W42–W42m: on via the palette → **a real ShellExecute on a folder starts Explorer 2.0 there**, drive `C:\` arrives intact, context menu entry visible + invokable via `InvokeVerb`, `Folder` untouched, both off → `reg export` identical to before.

### R-001 One-button reset (“emergency exit”) ✅ complete (CI runs #59–#61)
- New crate **`explorer-integrate`**: every Windows integration (from M7) goes only through `apply()`. Before every change the state before is in the **journal** (write-ahead, atomic with flush, mirror copy, checksum; registry values raw as type + bytes → bit-exact rollback; files with a backup).
- `reset()`: journal backwards – only where the value **still belongs to us**; if other software has changed it in the meantime, it stays and is in the report (“von anderer Software geändert – belassen” – changed by other software – left). Own keys only if empty. Afterwards a **sweep of the known integration locations** (folder verbs Folder/Directory/Drive, own context menu entries, Win+E, `HKCU\Software\Explorer2`; detected by the command containing `explorer2`) → works even without or with a broken journal. Errors (e.g. HKLM without admin) stay in the journal for the next attempt. Environment → `WM_SETTINGCHANGE`.
- **`explorer2-recover.exe`**: console, no UI/GPU (also runs in Safe Mode), `--status` (exit 1 = active), `--yes`, otherwise asks; as a second file in the Windows artifact.
- **Mandatory test in CI** (`scripts/r001-snapshot.ps1`, snapshot with Windows' own `reg.exe export`, independent of our code): integration on (folder verb, environment variable, overwritten foreign value, own keys, file) → reset → **snapshot identical** – (1) from the journal, (2) with a half-destroyed journal via the mirror, (3) entirely without a journal via the sweep; plus “second reset = nothing to do”. Run #59: 15/15, environment + foreign value bit-exact (same hash).
- Tests: 13 unit tests against the fake registry (among others a crash between journal and change, values changed by others, HKLM denied → second attempt, files, format damage) + a Windows unit test against the real registry (without the sweep, so a developer PC is never touched).
- Found while testing: a key under which a rollback failed dropped out of the journal as “not empty” → a second attempt would have left it; now it stays in.
- **CI #60 red**, two findings: (1) **real bug**: palette command with a dialog (reset, but also “Endgültig löschen”/“Papierkorb leeren”) – Enter in the dialog didn't arrive because the focus return of the closing palette competed with the dialog focus → commands now run one event-loop pass later; Linux E2E [71f2] covers the path. (2) W11 waited a fixed 2.5 s for the shell's Properties dialog; it came later, stayed open and took the focus from W12–W17 → the test now polls up to 10 s and waits for it to close.
- **CI #61 green**: Windows E2E 68/68 (W41: real integration reset via the app palette – 6 values/files, 7 keys, 0 errors, foreign value exactly back; W11 dialog after 261 ms), R-001 snapshot 15/15, Linux E2E 188/188.
- R-001c: Settings → **“Windows-Integration”** (status when opening, button “Alle Windows-Integrationen zurücksetzen” (reset all Windows integrations) only when something is active, confirmation with the state), palette command; Linux E2E [71]–[71g] (damaged journal reset via the button, 187/187), Windows E2E W41–W41h (real integration reset via the app palette).

### M6.4 Colour schemes ✅ (CI run #58) → **M6 complete**
- Settings → **Farbschema (colour scheme): Wie Windows / Hell / Dunkel** (like Windows / light / dark) (+ command palette “Farbschema: …”), check box **“Windows-Akzentfarbe verwenden”** (use Windows accent colour); saved in settings.ini (`theme`, `system_accent`).
- **All colours central** in `core::theme::tokens` (37 tokens) → Slint global `Theme`; main.slint no longer has any hex colour. Light = exactly the previous look (selection still #cce8ff).
- Accent: selection, hover, current place, drop target take over the **hue** of the Windows accent at the same brightness (yellow doesn't get garish); focus frame/active pane/progress use Windows' own shade (AccentPalette “Dark 1” light, “Light 2” dark), darkened/lightened to contrast ≥ 4.5 if needed.
- “Wie Windows” follows **live**: registry watcher (`RegNotifyChangeKeyValue` on Themes + DWM + Accent) → re-read → apply only on change. Std widgets (buttons, check boxes, scroll bars) via `Palette.color-scheme`, title bar via `DWMWA_USE_IMMERSIVE_DARK_MODE`.
- No light flash: the scheme is set before the first frame (registry read ~0.01 ms in the start path).
- Tests: core unit (WCAG contrast of all text/background pairs for 12 accents × light/dark including yellow/white/black, accent frame ≥ 3:1, HSL round trip, AccentPalette/ABGR format, settings round trip), Linux E2E [70]–[70k] by **pixel measurement** (list #fafafa → #191919, selection #1c3f63, sidebar #202020, std button dark, restart starts dark, system dark + green accent → green selection, accent check box off → blue), Windows E2E W37–W40 with the **real registry** (live dark/light, live accent change, title bar; registry restored exactly afterwards).
- **CI #57:** functionally everything green (Windows E2E including W37–W40, Linux E2E 180/180), only the first-start gate was red with 2709 ms. Cause: **the VM**, not M6.4. Already *before* the first theme code, `window created` was at 612 instead of 144 ms (#56), the background probe at 521 instead of 70 ms (7×) and run→event loop at 2011 instead of 406 ms. Theme init costs 0.49 ms, Linux first paint 10 → 9 ms. But the abort at the first start had prevented the real budget measurement (normal start < 300 ms) → CI now collects time gates and throws only at the end of the matrix, with the VM metric `probe_ms` in the log.
- **CI #58 green** (normal VM, `probe_ms` 84.5): first start 677 ms (#56 without M6.4: 633), normal start best-of-3 **102 ms** (#56: 94.5; budget 300), Windows E2E including W37–W40 green.

### M6.3 Command palette ✅ (CI run #55)
- **Ctrl+Shift+P** opens a search field at the top centre; ↑/↓/PgUp/PgDn select, Enter executes, Esc/a click outside closes.
- `core::commands` (pure, tested): **one** directory of all 45 commands (ID, German name, shortcut, condition: selection / real folder / paste possible / undo / redo / Recycle Bin), `fuzzy_score` (subsequence, umlauts folded, word starts + consecutive hits count), `rank` (recently used first), `push_recent`.
- Executed through the same routes as the keys: `nav::run_action` (extracted from `list_key`) or `fileops::command`; a unit test makes sure every command in the directory is executed.
- Unavailable commands greyed out (Enter → “Hier nicht möglich” – not possible here); recently used (8) in settings.ini (`recent_commands`).
- “>” mode: places from the sidebar, drives, virtual places and the history of all tabs.
- Tests: unit, Linux E2E [69]–[69g] (168/168), Windows E2E W36.
- CI #53 red at the first-start gate: run→first frame 1376 ms (#51: 456, #52: 592; VM additionally ~2× slower). The palette was invisible (`visible: false`) but **built and laid out** → now its own component under `if pal-open` (created only when opened, `init` focuses the field).
- #54 (same app code as #55) still red: 1178 ms; #55: **731 ms** with identical code → the first-start gate mainly measures the VM (background probe 170 vs 89 ms). CI now runs the Windows E2E even after a red time gate (the functional signal is no longer lost). Gate now a warning from 1 s, red from 2 s (Boss, D-018).

### M6.2 Inspector ✅ (CI run #52)
- Right sidebar (280 px), **Alt+P** or the “Details” button, state in settings.ini (`inspector`).
- Content: preview (Windows thumbnail 256 px or a large icon; text files: first 40 lines, UTF-8/BOM, binary detected), title + type, size (from 1 KB with bytes), modified, created, attributes, dimensions (from the file header, `core::inspect::image_size`), path as selectable text (copy).
- Multi-selection: count, size of the files, total including folder contents. Folders: contents + total size live in the background (`fs::dir_stats`: cancellable, links/junctions are not followed, unreadable folders counted, limit 500,000 entries).
- Deliberately without a disk walk: the *open* folder only shows its direct contents (otherwise opening `C:\` would count the whole disk); whole drives are never counted automatically.
- Debounced (120 ms), a generation counter discards stale results, a running count is cancelled on a new selection.
- Tests: unit (`core::inspect`, `fs::stats` including a symlink loop + limit), Linux E2E [68]–[68g] (161/161), Windows E2E W35/W35b (real thumbnail + dimensions of the PNG).

### M6.1c View per folder ✅ (CI run #51) → **M6.1 complete**
- `core::views::FolderViews` (pure, tested): folder → view, LRU up to 5000, Windows case-insensitive, tolerant parser; file `views.ini` (`mode<TAB>path`, written atomically).
- The view now belongs to the **tab** (when navigating: the remembered view of the folder, otherwise the default); `PaneData` carries the mode + tile sizes → left and right pane can look different. Switching remembers the view for the folder.
- Settings: “Aktuelle Ansicht für alle Ordner” (current view for all folders) → becomes the default (`view` in settings.ini), all remembered folders forgotten.
- Tests: unit, Linux E2E [67]–[67e] (154/154).

### M6.1b Sort menu + rubber-band selection ✅ (CI run #51)
- CI run #50 red at the **first-start gate** (fresh VM: 2140 ms > 1000 ms; window→event loop 1.6 s instead of ~0.4 s in #40/#48). Suspicion: the character “▾” in the new sort button is missing in Segoe UI → font fallback search at the first render. Fix: triangle and selection dot drawn instead of characters; new time marks `init done`/`run` in the trace. **Confirmed in #51**: window→event loop 1627 ms (#50) → 456 ms (#51, #48: 409 ms), first start 2140 → 702 ms; init itself only 8 ms, the time is in the first frame.
- “Sortieren ▾” (sort) in the status bar (popup): name/modified date/type/size (direction stays), ascending/descending; current choice with a dot. Tile view without column headers (like Explorer).
- Rubber-band selection: dragging on free space (below the rows, beside/between tiles) draws a frame; touched entries are selected live, with Ctrl in addition to the selection at the start. `core::view::band_hits` + `Selection::set_band` (pure, tested).
- Found: Slint fires `clicked` **before** the pointer-up event → the rubber band is finished in the pointer event, not in `clicked` (otherwise the end never arrived).
- Tests: unit (rubber-band hits details/grid, selection with/without base), Linux E2E [65]–[66d] (149/149), Windows E2E W34 (rubber band over two tiles with a real mouse).

### M6.1a Folder options ✅ (CI run #49)
- Settings panel, section “Ordneroptionen” (folder options) (in settings.ini: `show_extensions`, `show_protected`, `single_click`):
  - **Show file name extensions** (default on): off → the extension is hidden only for *known* types (Windows: `HKCR\.ext` exists, cached; folders, `.gitignore`, `ende.` never). F2 then edits only the displayed name, the extension is appended again on save (typing “x.txt” → “x.txt.txt”, like Explorer); on an error the typed text stays in the editor without the extension (no double extension).
  - **Show protected system files** (default off): hidden+system stays invisible even with “Ausgeblendete” (hidden items).
  - **Open with a single click**: releasing opens, hovering (600 ms) selects, Ctrl/Shift click only selects; clicks up to 600 ms after an open are ignored (otherwise the 2nd click of a double-click opens whatever lies under the pointer in the next folder); applies to rows and tiles.
- Tests: unit (`core::display`, settings round trip, Windows `registered_types`), Linux E2E [63]–[64e] (141/141), Windows E2E W31–W33b (second start with settings.ini: `.txt` hidden, an unknown extension stays, `attrib +h +s` file invisible, F2 keeps the extension).

### M5.7 Tile view + thumbnails ✅ (CI run #48; W28c: real PNG thumbnail, tile centre exactly R220 G20 B20) → **M5 complete**
- `core::view::ViewMode` (details/medium/large/extra large, 48/96/256 px, tile sizes, `step` for Ctrl+mouse wheel, Explorer shortcuts Ctrl+Shift+1/2/3/6 including US/DE shift characters), setting `view` in settings.ini.
- `shell::thumbnail(path, px)`: `IShellItemImageFactory::GetImage` (preview from the Windows thumbnail cache, otherwise a large shell icon), premultiplied BGRA → RGBA; Windows test with a self-written BMP.
- `app::thumbs`: 2 workers, only visible tiles, newest request first, scrolled-away ones are skipped, 96 MB limit (LRU, they come back when scrolling back), key including modification time (edited image → new preview).
- UI: grid as a second ListView over **stable** row/column models from Rust (an `int` repeater would have rebuilt all tiles on every selection change → double-click/hover broken, measured via `init` debug), tile = the same interaction as a row (click/Ctrl/Shift, dragging a multi-selection, right-click, double-click, Slint DnD/OLE drag), arrow keys in the grid, F2 field over the label, context menu via keyboard at the tile, OLE drop hit test with row height/columns.
- Ctrl+mouse wheel throttled (max. 1 step/250 ms, measured from the start): X11 delivers a wheel click twice, touchpads many small events.
- Tests: unit (ViewMode, keys in the grid, hit test in the grid, BGRA), Linux E2E [62]–[62h] (132/132), Windows E2E W28–W30 (real red PNG → red pixel in the tile).
- Runs #45–#47 red: the Windows Server runner has **no BMP preview provider** (the shell returned the icon) → `Thumb { image, preview }` (first `THUMBNAILONLY`, then `ICONONLY`), tests distinguish both cases; PNG previews exist there.

### M5.6d2 Libraries ✅ (CI run #44) → **M5.6 virtual places complete**
- `shell::libraries`/`library` (`IShellLibrary` via the `.library-ms` files in `FOLDERID_Libraries`): localised name, member folders (file system only), file for the icon.
- View “Bibliotheken” (Libraries) (`::bibliotheken`, aliases “Bibliotheken”, `shell:Libraries`, CLSID): rows with the real library icon and “N Ordner” (N folders); a library with **one** folder opens it directly, with several → `::bibliothek:<stem>` lists its folders (column “Ordner” (folder) = parent path).
- `shell:DocumentsLibrary`/`MusicLibrary`/`PicturesLibrary`/`VideosLibrary`/`CameraRoll`/`SavedPicturesLibrary` → library; Up from a library → “Bibliotheken”; breadcrumb “Bibliotheken › name”; the session remembers library tabs.
- Protection: all placeholders (`::…`) go through `places::is_placeholder` → no file operation, no watcher.
- Deliberately not yet: merged view (Explorer shows the contents of all folders mixed) → own TODO.
- Tests: unit (placeholders, names, `shell:` libraries), Windows unit `libraries_read_without_crashing`, Linux E2E [61]/[61b], Windows E2E W23–W27 with its own test library (2 folders).

### M5.6d1 Address bar + real shell icons ✅ (CI run #43)
- `core::address` (pure, tested): `%VAR%` (unknown ones stay, `100%` stays), `shell:<name>` with a sub-path (`shell:Downloads\\Fotos`), Windows canonical names + aliases (`Personal` = Documents), unknown name → clear error message; `~` only on Unix. Existing paths are never reinterpreted (a folder named `%x%` stays reachable).
- `fs::places::known_folder_path`: Windows `SHGetKnownFolderPath` (32 folders), Linux XDG counterparts where available.
- Icons: `IconKey::Shell` (parsing name → `SHParseDisplayName` + `SHGFI_PIDL`) for Recycle Bin/This PC; the sidebar shows real icons (folders with a desktop.ini icon, drives, Recycle Bin), “This PC” rows real drive icons; without a shell the replacement square stays.
- CI: pure doc commits (`docs/**`, `*.md`) no longer trigger a run.
- Tests: unit (address parsing, Windows: shell icons Recycle Bin/PC/C:\\), Linux E2E [60]–[60c], Windows E2E W20–W22 (`%TEMP%` with long names, `shell:Downloads`, unknown name).

### M5.6c Network ✅ (CI run #41, Windows E2E W18–W19c with a real SMB share green)
- `\\server` in the address bar → the server's shares as a folder list (`NetShareEnum` level 1); admin shares (`C$`, `ADMIN$`, `IPC$`), printers and `…$` hidden like in Explorer; the share comment is in “Typ” (type).
- Server wants a login → Windows login dialog (`WNetAddConnection3W`, window as owner, in a background thread), then reload; also applies to folders in shares (access denied / login failed) – at most once per path, no dialog loops.
- “Up” from `\\nas\film` → `\\nas` (share preselected), from the server → “Dieser PC” (This PC); breadcrumb `\\nas › film › …`; nothing can be created in the server folder.
- Tests: unit (path parsing, filter, error codes), Windows unit against `localhost`, Windows E2E W18–W19c with a real SMB share (`New-SmbShare`), Linux E2E [59] (clear error message).

### M5.6a/b Recycle Bin view + “Dieser PC” (This PC) ✅ (CI run #39, everything green including Windows E2E W15–W17b)
- `core::places::VirtualPlace` (placeholder paths `::papierkorb` / `::dieser-pc`, never confusable with real paths) → tabs, history, session, breadcrumb work unchanged; address aliases: “Papierkorb”, `shell:RecycleBinFolder`, “Dieser PC”, `shell:MyComputerFolder`, Explorer CLSIDs.
- `ops::trashview`: list with the real file name (Windows display names without extension → extension from `$R…`), equal names made unique (“a.txt (2)”), restore refuses to overwrite, delete permanently, empty.
- Safety: in virtual places `pane_dir`/`selected_paths`/`entry_at` return nothing → paste, new folder, drops, rename can never land on a placeholder path; disallowed commands report “Hier nicht möglich” (not possible here).
- Command bar in the Recycle Bin: Restore / Delete permanently / Empty Recycle Bin; columns “Gelöscht am” (deleted on), “Ursprünglicher Ort” (original location). Dragging files onto the Recycle Bin = delete. Every delete/undo refreshes open Recycle Bin tabs.
- “Dieser PC”: drives with “x GB frei von y GB” (x GB free of y GB), open → drive root, “Up” from a root → This PC.
- Tests: core +2, ops +3 (including a real Recycle Bin pass), E2E Linux **116/116** (new 55–58c), Windows E2E extended (W15–W17b).

### Renderer comparison Skia vs. FemtoVG (CI #35, Windows, D-015)
| | Skia (today) | FemtoVG |
|---|---|---|
| Exe size | 16.5 MB | **9.0 MB** |
| Picture (Mesa GL) | reference | practically the same; text minimally bolder, icons/layout identical |
| Cold start with OpenGL (Mesa llvmpipe, best of 3) | **982 ms** | 1245 ms (+27 %) |
| Without usable OpenGL, GPU forced | fallback, 408 ms | fallback, **225 ms** |
| Crashes | none | none |
- Assessment: picture equivalent, but on the only measurable GL path (software GL) FemtoVG starts slower → **Boss: stays with Skia** (D-016). Comparison job removed from CI, the script stays. Mesa is not a real GPU driver; decision only after a test on a real graphics card (CI artifact `explorer2-femtovg-test`).
- Linux comparison (Xvfb/Mesa) confirms: content identical, only text smoothing differs.
- Finding: an OLE drag without a pressed button (release between the drag threshold and the drag start) would have dropped “somewhere” on the next mouse move → drag now only starts while the primary button is pressed (takes swapped buttons into account).

## M4 – File operations
- **New crate `explorer-ops`** (testable platform-neutrally): `exec` (job engine, synchronous `run` + `spawn` in the background), `undo`, `simple` (rename, new folder), `names` (Windows naming rules, “ - Kopie”, “ (2)”), `clipboard` (CF_HDROP), `win_copy` (CopyFileExW with progress/cancel).
- **Safety rules**: copy into `.<name>.e2part` + rename; moving across drives deletes the source only after a complete, flushed copy; “Replace” = old file into the Recycle Bin (undoable); cancelling cleans up partial files; errors per item, the job keeps running.
- **App** (`src/fileops.rs`): clipboard (internal + Windows system clipboard), jobs + operations panel, conflict/confirmation dialog (modal, keys R/S/B/A/Esc or Enter/Esc), inline rename, new folder, undo/redo in the background, drag & drop (Slint 1.18 `DragArea`/`DropArea`), command bar. After paste/rename the new items are selected.
- **Tests locally**: ops 18 integration tests (+ disk full with a 1 MiB tmpfs), app 17 unit (keys, titles, progress text, DnD rules), E2E **100/100** (new 38–52: rename including undo/redo/Esc/error, new folder, copy/paste with conflict “Beide behalten” (keep both) and “Ersetzen” (replace) + undo, cut, Del + restore, Shift+Del with dialog, DnD move/Ctrl copy, 60,000-file job with panel/pause/cancel/undo).

### CI run #26 – everything green ✅
- E2E Linux **100/100** (M4 block 38–52 including DnD, conflicts, undo, 60,000-file job with pause/cancel).
- Windows: ops integration tests 19/19 (including Recycle Bin + restore, locked file), clipboard interop: Windows (.NET `GetFileDropList`) sees our files including an umlaut, we read Windows copies, the cut flag survives.
- Linux: disk full (1 MiB tmpfs) → error message, no partial file.
- Windows UI matrix: normal starts 44–45 ms to content, first start 636 ms (fresh VM), data: first rows 1.3 ms, all 100k 133–143 ms.
- Exe 19.7 MB (previously 17.2) – just under the 20 MB budget → noted as debt.
- The way there: run #21 Clippy 1.99 lint, #22 8.3 short names, #23 Recycle Bin display names, #24/#25 clipboard test cmdlet (see findings).

### Findings in M4
- **Bug (integration test, flaky 1/15)**: undo of “Replace” first put the new file into the Recycle Bin and then restored the *newest* Recycle Bin entry with the same path – i.e. the new file that had just been deleted instead of the old one. Fix: Recycle Bin snapshot **before** the undo, restore only from that.
- **Perf**: `fsync` per copied file would have multiplied mass copies → now only when moving (the source is deleted afterwards).
- **DnD**: a TouchArea as a sibling above the DragArea swallows the mouse press → the TouchArea must be a *child* of the DragArea.
- Multi-selection would have been lost when dragging (click on “down”) → on an already selected row the click is applied only on release (Explorer behaviour).
- **CI run #21**: new Clippy lint of the CI toolchain 1.99 (`chunks_exact_to_as_chunks`) → `as_chunks`, MSRV 1.88; locally now 1.99 as well.
- **Bug (CI run #22, Windows only)**: restoring from the Recycle Bin found nothing because our path contained the 8.3 short name `RUNNER~1`, but the Recycle Bin stores the long name. Would have hit real users the same way (short names from `%TEMP%`, case). Fix: canonicalise the parent folder + compare case-insensitively on Windows.
- **Bug (CI run #23, Windows only)**: folders were found now, files weren't: the Recycle Bin reports the *display name*, and that hides known extensions (“weg” instead of “weg.txt”). The restore would even have put the file back **without its extension**. Fix: match via name stem + extension of the `$R…` file, restore with the real original name.
- Test design: pausing *before* the conflict question already blocks the job before the question → the test now pauses after the question.

## M3 – Tabs, split view, sidebar, history, session
- **Architecture**: `App` → panes (1–2) → tabs; every tab has its own path/selection/loader/watcher/history. Background results are assigned via a stable tab ID (tabs can be closed in between) + generation counter.
- **Tabs**: +/Ctrl+T (same folder), ×/middle-click/Ctrl+W, Ctrl+(Shift+)Tab; inactive tabs load only when selected; a sort/filter change marks all as “stale”, visible ones reload immediately.
- **Split view**: “Zwei Ansichten” (two views)/Ctrl+\, F6 switches, a click activates, blue accent line = active pane, keyboard focus follows the active pane.
- **History**: ←/→ buttons, Alt+←/→, mouse side buttons.
- **Sidebar**: Quick access (Windows Known Folders → also redirected/OneDrive folders), “Dieser PC” (This PC) with drives, fill bars, “x GB frei von y GB”; query in the background, refresh every 15 s, no “insert disk” dialogs.
- **Session**: session.ini (debounced 400 ms + on close); start without a path → restore including the active pane; a path argument wins (like `explorer.exe C:\x`).

### CI run #19 – everything green ✅
- E2E Linux 60/60 (throttle: 6 refreshes / 3 s).
- Windows: places test with real Known Folders + drives (C:\ with fill level) green; drive query 0.17–0.34 ms (2 drives).
- Windows UI matrix: normal starts 66–69 ms (best 65.5 ms, budget 300), first start 255 ms, data: first rows 0.8–2.2 ms, all 100k 181–246 ms. (Mesa forced GPU: 992 ms cold start – purely a test path on software GL, auto correctly picks software there.)
- fs bench Windows: median 1.23 ms / 137 ms.

### Tests M3
- Unit: core 32 (+history, session, format_usage), fs 24 (+places: display names, fill level, XDG parser, real drives), app 13 (+keys for tabs/pane/history, tab titles, crumb budget).
- E2E 60/60 (new 27–37): sidebar click, Alt+←/→, mouse back button, Ctrl+T + independent navigation, Ctrl+Tab, split on/off, **focus moves into the new pane**, F6, a click activates a pane, session saved/restored (including the active pane), **lazy loading** of restored tabs, Ctrl+W, the last tab stays, a path argument beats the session.

### Findings in M3
- **Bug**: after a split the keyboard focus stayed in the left pane while the right one counted as active → F6 “jumped back”. Cause: a new repeater instance misses the focus order (`changed` doesn't fire on init). Fix: focus follows `active` (+ `init`). Proven via the key trace (`EXPLORER2_TRACE_KEYS`).
- **Bug**: restore ignored the saved active pane → fixed, E2E checks `active_pane=1`.
- **Bug (own code review)**: new tabs already have their path → breadcrumbs/watcher would never have been set up → setup now also runs when no watcher exists yet.
- Shortcuts didn't arrive while no list was focused → root FocusScope as a fallback.
- E2E diagnostics improved: an own log per block.
- **Bug (CI run #18)**: the refresh throttle only guaranteed the gap from the *end* of a refresh – events during a running refresh started a second one immediately (CI: 8 instead of ≤ 7 refreshes/3 s; since M3 more writes per refresh). Fix: minimum gap 500 ms from the *start*, after Done only extend. Measured: smallest gap 501 ms.

## M2 – Details view, selection, keyboard, open, breadcrumb
- **Details view**: column headers name / modified date / type / size, a click sorts (▲▼), widths draggable; sorting in settings.ini.
- **Selection** (`core::selection`): click / Ctrl / Shift like the Windows ListView (Ctrl+click sets the anchor even when deselecting), a click into empty space clears; carried over by name on auto-refresh/re-sorting.
- **Keyboard** (`keys::map_key`): arrows, PgUp/PgDn, Home/End, Ctrl+A, Ctrl+Space, Enter, Backspace/Alt+↑, F5, Ctrl+L/Alt+D, type-ahead.
- **Open**: navigate folders, files via `ShellExecuteW` (Windows) with the default app; a Windows test runs a real `.cmd` via the default verb.
- **Breadcrumb** (`core::breadcrumb`): Windows, UNC and Unix paths, “…” truncation on the left; Ctrl+L/a click into empty space = edit, Esc = cancel.
- After “Up” the folder we came from is selected (like Explorer).

### CI run #16 – everything green ✅ (Windows)
| Scenario | Window created | Cold start to content | Note |
|---|---|---|---|
| First start (no cache) | 134 ms | **221 ms** | software immediately, probe in the background (34 ms, “gdi generic”) |
| Normal start 1/2/3 (cache) | ~6.5 ms | **56 / 55 / 56 ms** | best 54.7 ms (budget 300) |
| forced GPU (without GPU) | 6.4 ms | 370 ms | Skia fallback |
| forced software / setting off | ~6.5 ms | 57 / 54 ms | |
| Mesa auto | 7.4 ms | 151 ms | the background probe detects llvmpipe (260 ms, not in the start path) |
| Mesa forced GPU | 7.6 ms | 458 ms | Skia OpenGL ok |

Data in all scenarios: first rows 0.8–1.6 ms, all 100k 162–285 ms. ShellExecute test (real `.cmd` via the default verb) green. fs bench: median 1.22 ms / 139 ms.

### Tests M2
- Unit: core 26 (selection, type-ahead, breadcrumb, type sorting, settings), app 10 (keys, status, selection remap, open).
- E2E 37/37 (new 12–26): sort orders via column header, direction change, persistence, arrows/Shift+End/Ctrl+A, **selection colour by pixel**, type-ahead, Enter opens a file (stub opener), Enter/double-click opens a folder, Backspace selects the folder we came from, Ctrl+L+Esc, breadcrumb click.

### Findings in M2
- **CI run #15 red (2 causes):**
  1. E2E [26]: test bug – the breadcrumb navigated correctly to the root `/` (short CI path → no “…”), the check pattern `//*` never matched → ancestor check corrected (counter-cases tested).
  2. Windows first start 1666 ms: GPU probe **581 ms** (previously 3–67 ms) + window creation 755 ms (fresh VM). → **Design fix**: the probe never runs in the start path any more (without a cache: software immediately, probe in the background, applies from the next start). New trace “window created”. Budget check: normal starts best-of-3 < 300 ms (hard), first start < 1 s (plausibility, reported).
- Test expectation wrong (not code): Ctrl+click sets the selection anchor even when deselecting – like the Windows ListView; test corrected.
- A supposed render bug (focused row “without” selection colour) was a reading error on a scaled-down screenshot – pixel measurement showed the correct colour; the “fix” was reverted, the selection colour is now checked by pixel in E2E.
- After the breadcrumb rework, a click on the address bar hit a segment → added the Explorer shortcuts Ctrl+L/Alt+D, E2E uses them.
- Ignored our own gotcha (`viewport-y` deprecated) → `content-y`.

## M1.5 – Renderer choice, settings, icon (Boss: option 2 + switch)
- **GPU probe** (`explorer-gpu`, Windows/WGL): hidden 1×1 window, legacy GL context, checks `GL_RENDERER`/`GL_VERSION`/`glCreateShader`. Software GL (GDI Generic, llvmpipe, Microsoft Basic Render) = unusable, because it's slower than our software renderer.
- **Decision** (`core::renderer::decide`): override `EXPLORER2_RENDERER` > setting off → software > probe (cached) → Skia GPU or software. `SLINT_BACKEND` set → the toolkit decides.
- **Cache**: result in `settings.ini` (per app version), background re-probe 5 s after start (driver new/removed → takes effect on the next start). The first start pays the probe once.
- **Settings panel**: “Hardwarebeschleunigung verwenden, wenn verfügbar” (use hardware acceleration when available) + status line (“Aktuell: Software-Rendering (keine geeignete GPU gefunden)” – currently: software rendering (no suitable GPU found)). `settings.ini` written atomically, read tolerantly; “Ausgeblendete” (hidden items) is saved as well.
- **Icon**: yellow folder + blue band on a light tile; exe icon (ICO 16–256) + window icon.

### Tested environments (locally, Linux/Xvfb)
| Environment | Result |
|---|---|
| without GL, auto | platform default, runs |
| without GL, forced GPU | Skia (fallback), runs |
| forced software | runs |
| Mesa GL (llvmpipe), auto / forced GPU | runs; GPU mode demonstrably loads EGL/Mesa (58–62 driver lines), software mode 0 |
| setting off | software, reason “disabled in settings” |
| E2E | 20/20 PASS (new: panel saves the setting, hidden items persisted, restart respects the setting, window icon set) |

### CI run #12 (Windows environment matrix)
| Scenario | Cold start to content | Result |
|---|---|---|
| Auto, first start (probe 3.4 ms → “gdi generic”) | ~50 ms | ✅ software (previously 488–679 ms) |
| Auto, from cache + background re-probe | 129 ms | ✅ |
| GPU forced (without GPU) | 355 ms | ✅ runs (Skia fallback) |
| Software forced | 45 ms | ✅ |
| Setting off | 45 ms | ✅ |
| Mesa GL | – | ❌ **test setup**: the Mesa 26.2.6 MSVC package contains no opengl32.dll (known upstream problem) → switched to the MinGW package |

### CI run #13 – everything green ✅
| Scenario (Windows) | Choice | Cold start to content |
|---|---|---|
| without GPU, 1st start (probe 67 ms) | software (“gdi generic”) | 270 ms |
| without GPU, cache | software | **55 ms** |
| forced GPU without GPU | Skia fallback, runs | 502 ms |
| forced software | software | 56 ms |
| setting off | software | 59 ms |
| Mesa, auto | software (llvmpipe detected) | 299 ms |
| Mesa, forced GPU | **Skia OpenGL surface ok** (GL path proven) | 485 ms |

Data budgets met in all scenarios (first rows 0.8–1.4 ms, all 100k 151–206 ms). Exe icon checked (centre pixel 255/203/69). Note: the probe costs 3–239 ms on the **first** start depending on the driver (cached afterwards) – real GPU values only measurable on Boss's PC.

### Bugs/findings in M1.5
- The E2E click coordinate for “Ausgeblendete” would have hit the wrong button after the new button → adjusted (x=880).
- The UI status line was English → `Reason` enum: English for traces, German (`status_de`) for the panel.
- A supposedly missing window icon was an `xprop` display problem (raw data: 256×256, yellow) → secured as E2E check [11].

## M1 – what was built
- `explorer-fs`: Windows fast path (`FindFirstFileExW` + Large Fetch), portable std path, verbatim long paths, background loader with batches/cancel, watcher with debounce.
- `explorer-core`: entry with hidden/system/readonly/is_link, `ViewFilter` (Explorer rules), faster natural sorting, `format_count`.
- `explorer2`: `nav.rs` controller – the UI thread does no I/O, generation counter, auto-refresh with adaptive throttle, status bar, error display, fallback when the folder is deleted, toggle “Ausgeblendete” (hidden items).

## Measurements (release, 100,000 entries)
| Metric | Windows (CI runner) | Linux (sandbox) | Budget |
|---|---|---|---|
| First batch (fs bench) | **1.3 ms** | ~6 ms | < 100 ms |
| Full load + sort (fs bench) | **130 ms** | ~250 ms | < 1 s |
| Sorting alone | 46 ms | 47 ms (previously 173 ms) | – |
| UI: loader done (100k including formatting) | 187 ms | 233 ms | < 1 s |
| UI: first rows ready (loader thread) | **0.9 ms** (default) / 1.1 ms (software) | 1–1.3 ms | < 100 ms |
| UI: all 100k ready (loader thread) | **140 ms** / 146 ms | 277–296 ms | < 1 s |
| UI: cold start to first content – default renderer (Skia, GL fails → fallback) | **488 ms** | 26 ms | M8: < 300 ms ❌ |
| UI: cold start to first content – software renderer | **43 ms** | 37 ms | M8: < 300 ms ✅ |

**Renderer finding (run #9):** on systems without GPU OpenGL (CI runners, VMs, RDP) the failed Skia GL attempt costs ~450 ms of start time. → Decision needed for M2/M8 (software renderer as default? check GL capability upfront? D3D first?).

**OS stalls on Windows CI (runs #8/#9):** individual runs take 168–170 ms already in the **raw** `FindFirstFileExW` call (our code's share ~0.2 ms) – sporadic (run #9: 2 of 6 runs), cause: foreign load on the runner (Defender/indexer after mass creation). In run #8 it hit the majority → the median tipped. Cushioned for users: the UI never blocks, shows “Lädt …” (loading …).

**Finding Windows CI:** the runner has no GPU OpenGL → Slint/Skia reports “Failed to initialize Skia opengl surface … Falling back”, the app still runs (the fallback works = good for VMs/RDP). The GL attempt costs start time → TODO: on Windows D3D first / avoid the GL attempt (M2/M8).

## Tests
- Unit/integration: 31 tests (core 7, fs 20, app 4) – green; fs Windows code clean via `clippy --target x86_64-pc-windows-msvc`.
- E2E under Xvfb (13 checks, all PASS): 100k folder, auto-refresh, deleted folder → parent folder, invalid path → error + address bar reset, toggle hidden items, up, the address bar follows after typing, refresh throttle, no crash/panic.
  - Script: `scripts/e2e.sh`, runs in CI as job `e2e-linux` (screenshots as an artifact on failure).

## Bugs found & fixed in M1
1. **Refresh endless loop**: a file that is constantly written (here: our own trace log) triggered a complete reload every ~100 ms → adaptive throttle (≥ 500 ms, 4× the last load time, max 10 s). Measured: 30 → 6 refreshes / 3 s.
2. **Address bar frozen**: the one-way binding `text: current-path` is dropped by Slint as soon as the user types → never updated afterwards (not even on up/double-click). Fix: two-way binding `<=>`.
3. Status bar “1 Elemente” → “1 Element” (singular).
4. Sorting too slow (173 ms / 100k) → allocation-free, 47 ms.
6. **Flaky perf gate**: the median over 5 runs tipped on sporadic OS stalls on the runner (proof: the raw OS query itself 168 ms) → best-of-N gate, 9 runs, stalls are counted and reported.
5. Measurement error: “first paint” on Windows (369 ms) included window/renderer start. The 1st attempt (marker “event loop running”) was wrong: run #7 showed that Slint initialises the renderer only **after** the loop starts, at the first frame (GL failed attempt + fallback blocks the UI thread ~490 ms). The rendering notifier is unsuitable as a marker (software renderer/Skia software fallback don't support it). **Solution:** data latency is measured in the loader thread (`data_ready=`), cold start to content separately (`at=`); CI compares the default vs. the software renderer.

## Open
- ✅ Run #10 green. Windows bench (9 runs): first batch best 1.1 ms / **median 1.17 ms** / max 1.52 s (OS stall); full+sorted best 101 ms / **median 115 ms**; OS stalls 2/9 (raw 176–314 ms) – the gate holds stably.
- UI smoke run #10: default renderer first rows 1.3 ms, all 100k 230 ms, **cold start 679 ms**; software renderer 1.3 ms / 190 ms / **cold start 54 ms**.
- Open for M2: **renderer decision** (Boss).
- Fix in the test itself: E2E regex adapted to the new `at=` field (locally 13/13 PASS).

## Next steps
1. ✅ M4 completed (run #26) → **approval of M5 (Boss)**; check the exe size beforehand (19.7 MB).
2. M5: shell integration (icons/thumbnails, context menu, properties, Recycle Bin, OLE drag & drop with other apps).
3. Boss: test the artifact on a real PC (GPU, copy/paste with the real Explorer).
