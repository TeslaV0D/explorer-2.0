# CLAUDE.md – Projekt-Memory (wird automatisch geladen)

> Dieses File ist das Langzeitgedächtnis. **Nach jeder Arbeitseinheit aktualisieren:**
> 1. `docs/STATUS.md` (Zwischenstand) 2. `docs/TODO.md` (Haken setzen / neue Tasks) 3. dieses File bei neuen Entscheidungen/Erkenntnissen.

## Projekt in einem Satz
**Explorer 2.0** – vollwertiger, schneller, robuster Ersatz für den Windows Explorer (Windows 10/11 x64). Richtung: File Pilot (filepilot.tech).

## Dokumente
| Datei | Inhalt |
|---|---|
| `docs/PROJECT.md` | Projektbeschreibung, Vision, Ziele, Nicht-Ziele |
| `docs/PLAN-BASICS.md` | Plan für Phase 1 (Basis-Features, Architektur, Meilensteine) |
| `docs/WINDOWS-INTEGRATION.md` | Recherche: wie wir den Explorer systemweit ersetzen |
| `docs/TODO.md` | Aufgabenliste inkl. Phase 2 (Suche) + Phase 3 (Shortcuts) |
| `docs/STATUS.md` | Aktueller Zwischenstand |

## Entscheidungen (Decision Log)
| ID | Entscheidung | Status |
|---|---|---|
| D-001 | Stack: **Rust + Slint** (Skia-OpenGL-Renderer, Software-Fallback) + `windows`-crate (Win32/COM) für Shell-Integration | **fix (Boss, 2026-11-03)** |
| D-002 | Reihenfolge: Basis → Suche (voidtools-Stil) → Datei-Shortcuts (Sweetch-Stil) | fix (Boss) |
| D-003 | Explorer-Ersatz in Stufen, jede Stufe reversibel; riskante Hooks (IFEO) zuletzt + opt-in | vorgeschlagen |
| D-004 | Dokumentation auf Deutsch, Code/Identifier auf Englisch | fix |
| D-005 | Enumeration Windows: `FindFirstFileExW` (Basic + Large Fetch) statt `NtQueryDirectoryFile` – dokumentierte API, Messung zeigt Budget-Reserve; NtQuery nur bei Bedarf | fix (M1) |
| D-006 | Watcher: `notify`-Crate (ReadDirectoryChangesW/inotify), eigener Debounce + adaptive Drossel statt notify-debouncer | fix (M1) |
| D-007 | UI-Sprache vorerst Deutsch hart codiert; i18n später (Slint `@tr`) | vorgeschlagen |
| D-009 | Renderer: **GPU-Probe vor Fensterstart** (WGL: GL ≥ 3.0 + `glCreateShader`, Software-GL wie GDI Generic/llvmpipe = unbrauchbar) → Skia (GPU) oder Software. **Probe nie im Startpfad**: ohne Cache Start mit Software + Probe sofort im Hintergrund (gilt ab nächstem Start); mit Cache Re-Probe 5 s nach Start. Ergebnis in settings.ini (pro App-Version). Einstellung „Hardwarebeschleunigung verwenden, wenn verfügbar“ (wie Browser), Override `EXPLORER2_RENDERER=auto|gpu|software`; `SLINT_BACKEND` hat Vorrang | fix (Boss, 2026-11-03) |
| D-010 | App-Icon: gelber Ordner + blaues Band (Windows-11-Explorer-Anmutung) auf heller abgerundeter Kachel (Apple-Files-Anmutung), Vorwärts-Chevrons = „2.0“; 16–32 px vereinfachte Variante ohne Kachel | fix (M1.5) |
| D-011 | Dateioperationen datensicher: Temp-Datei + Rename, Quelle erst nach vollständiger + geflushter Kopie löschen, „Ersetzen“ = Papierkorb, Undo aus Report-Journal; fsync nur bei Verschieben | fix (M4) |
| D-012 | In-App-DnD über Slint 1.18 `DragArea`/`DropArea` (Payload = Pane/Zeile, Auflösung zur Drop-Zeit); OLE-DnD mit anderen Apps erst M5 | fix (M4) |
| D-013 | Exe-Größe: `opt-level="s"` (−15 %, Sortieren 100k +6–10 ms, Budgets weit weg), CI-Gate < 20 MB; Skia ≈ 8 MB, Rest Rust (Slint/Winit/Text), Bildformate/SVG kommen aus Slint-Interna (nicht abschaltbar) | fix (M5.0) |
| D-014 | Icons: Schlüssel pro Endung (wenige) bzw. pro Datei für exe/lnk/ico/url/… und Ordner mit R/S-Attribut (desktop.ini); threadsichere `IconRegistry` (Schlüssel → id), Loader-Thread schreibt nur `icon-id` in die Zeile, UI hält `[image]` indexiert per id → nachgeladene Icons ohne Zeilen-Durchlauf; Einzel-Icons nur für sichtbare Zeilen | fix (M5.1) |
| D-015 | **Qualität vor Größe**: Hauptziel = gutes, fehlerfreies Produkt; kleine Exe ist Nebenziel. Größen-Optimierungen nur, wenn sie nichts verschlechtern (Schrift, Ladezeiten, Animationen, Bugs) – sonst nicht machen. Die 20-MB-Grenze ist nur unser eigenes Frühwarn-Gate, keine technische Grenze | **fix (Boss, 2026-11-04)** |
| D-016 | GPU-Renderer bleibt **Skia** (FemtoVG: 9,0 statt 16,5 MB, Bild gleichwertig, aber Start mit GL +27 % auf Mesa → D-015). Feature `gpu-femtovg` + `scripts/renderer-compare.ps1` bleiben für einen späteren Test auf echter GPU | **fix (Boss, 2026-11-04)** |
| D-017 | Kachel-Ansicht: 3 Größen wie Explorer (48/96/256 px) + Details, global gespeichert (pro Ordner später); Vorschaubilder nur über die Shell (`IShellItemImageFactory`, nutzt den Windows-Thumbnail-Cache), kein eigener Platten-Cache; Speicher-LRU 96 MB | fix (Boss: Weg 1, 2026-11-04) |
| D-018 | Erststart-Gate (einmaliger Start auf frischer CI-VM): **Warnung ab 1 s, rot ab 2 s** – schwankt bei gleichem Code 0,6–1,2 s; echtes Startbudget bleibt der normale Start < 300 ms best-of-3; Sonderzeichen-Regel verhindert die bekannte Fallback-Falle an der Quelle | **fix (Boss, 2026-11-04)** |
| D-019 | Farbschemata: **alle Farben aus `core::theme::tokens`** (rein, WCAG-Kontrast-getestet für 12 Akzente × hell/dunkel) → Slint-Global `Theme` (`Theme.c.<token>`), keine Hex-Farben mehr in main.slint; Hell = Original-Look; Auswahl-Familie übernimmt Farbton des Akzents bei **gleicher Luminanz**; Akzent für Rahmen = Windows-Schattierung (AccentPalette Dark 1 / Light 2), notfalls auf Kontrast ≥ 4,5 geschoben. „Wie Windows“ liest `AppsUseLightTheme` + Akzent selbst und folgt live (`RegNotifyChangeKeyValue`), Std-Widgets per `Palette.color-scheme`, Titelleiste per DWM | fix (M6.4) |
| D-020 | R-001-Umsetzung: Crate `explorer-integrate`; **jede** Integration nur über `apply()` (Write-Ahead-Journal: Zustand vorher, atomar, Spiegel, Prüfsumme; Registry-Werte roh = Typ + Bytes). `reset()` stellt nur zurück, was **noch uns gehört** (fremd geänderte Werte bleiben + Bericht), löscht eigene Schlüssel nur leer, danach Sweep der bekannten Stellen (`manifest`, Erkennung am Text `explorer2`) = Rückbau auch ohne/mit kaputtem Journal; HKLM-Fehler bleiben im Journal für den nächsten Versuch. Recovery = `explorer2-recover.exe` (Konsole, `--status`/`--yes`) | fix (R-001) |
| D-021 | M7.1 Ordner öffnen: nur `Directory`/`Drive` (nie `Folder` = virtuelle Ordner), **eigenes Verb `explorer2open` als Standard-Verb** der Klasse (Windows hat `none`), Windows' `open` bleibt; Kontextmenü-Verb `explorer2`; jede Funktion einzeln schaltbar (`features::enable`/`disable` über `revert_within` + `Scope` = Teilbaum oder nur Werte eines geteilten Schlüssels); Exe weg → beim nächsten Start neu verknüpft; Notfall-Werkzeug-Kopie in `%LOCALAPPDATA%\Explorer2` (App-Daten, nicht im Journal) | fix (M7.1) |
| D-008 | Perf-Gates in CI auf **best-of-N** (≥ 9 Läufe), Median/Max/OS-Stalls nur ausweisen – Shared-Runner haben sporadische OS-Stalls (gemessen 168 ms im rohen FindFirstFileExW); Budgets selbst unverändert | fix (M1) |

## Arbeitsumgebung / Constraints
- Dev-Sandbox ist **Linux**, Zielplattform **Windows**. Windows-Code kann hier nicht ausgeführt werden → Build/Test via GitHub Actions (`windows-latest`); Kernlogik plattformneutral halten, damit sie auch unter Linux testbar ist.
- Branch: `claude/relaxed-pasteur-ufhh1p`. Kein PR ohne explizite Ansage.
- User-Präferenz: Anrede „Boss“, Bro-Ton, auf Deutsch antworten, kurz & präzise.

## Harte Anforderungen (Boss)
- **R-001 Ein-Knopf-Reset / „Notausgang“** (2026-11-03, Lehre aus File Pilot, wo nur Windows-Neuinstallation half): Jede Windows-Integration (Win+E, Ordner-Verben/Default-Handler, explorer.exe-Proxy/IFEO, Kontextmenü-Einträge, Autostart, Umgebungsvariablen/PATH, Verknüpfungen, Dateizuordnungen, Hotkeys) muss **mit einem Knopf vollständig rückgängig** zu machen sein – in der App **und** über ein eigenständiges Recovery-Tool, das auch funktioniert, wenn die App selbst nicht startet (inkl. Abgesichertem Modus). Jede Änderung wird vorher mit Originalwert in einem Journal gesichert; Reset stellt exakt den Ursprungszustand her. Pflicht-Test: Integration an → Reset → Registry/Umgebung identisch zum Snapshot vorher. Gilt ab M7, Architektur schon vorher berücksichtigen.

## Meilenstein-Stand
- M0 ✅ · M1 ✅ (Run #10) · M1.5 ✅ Renderer-Wahl + Einstellungen + Icon (Run #13) · M2 ✅ (Run #16) · M3 ✅ (Run #19) · M4 ✅ (Run #26) · **M5 ✅: M5.0–M5.5 ✅ (Run #37), M5.6a/b ✅ (Run #39), M5.6c ✅ (Run #41), M5.6 ✅ komplett (Papierkorb, Dieser PC, Netzwerk, Adresszeile/Icons, Bibliotheken; Run #39–#44), M5.7 ✅ Kachel-Ansicht + Vorschaubilder (Run #48) → M5 komplett**, **M6 ✅ komplett: M6.1 ✅ (Ordneroptionen, Sortier-Menü, Rahmen-Auswahl, Ansicht pro Ordner; Run #49–#51), M6.2 ✅ Inspector (Run #52), M6.3 ✅ Befehlspalette (Run #55), M6.4 ✅ Farbschemata (Run #58)**, **R-001 ✅ Ein-Knopf-Reset (Journal, `explorer2-recover.exe`, App-Knopf, CI-Snapshot-Test; Run #59–#61)**, nächster Schritt M7 Windows-Integration (erst nach Freigabe Boss)
- Arbeitsregel (Boss): nächste Stufe erst, wenn die aktuelle vollständig abgeschlossen, getestet und CI-grün ist.

## Code-Layout (seit M0)
- `crates/core` (`explorer-core`): plattformneutrales Modell, Sortierung (natürlich, Ordner zuerst), Unit-Tests
- `crates/fs` (`explorer-fs`): `enumerate`/`read_dir` (Windows: `win.rs` FindFirstFileExW; sonst `portable.rs`), `path.rs` (Verbatim/FILETIME, plattformneutral getestet), `loader.rs` (Hintergrund-Load, Batches, Cancel), `watch.rs` (notify + `Debounce`), Benchmark `examples/bench.rs`
- `crates/gpu` (`explorer-gpu`): `probe_opengl()` – Windows-WGL-Probe (verstecktes Fenster, Legacy-Kontext), sonst `NotSupported`; ohne Skia → lokal per `--target x86_64-pc-windows-msvc` prüfbar
- `crates/core`: `history.rs` (Zurück/Vor, max. 100), `session.rs` (session.ini: Panes/Tabs/Split/aktive Pane, tolerant)
- `crates/fs`: `places.rs` – Schnellzugriff (Windows `SHGetKnownFolderPath`, Linux XDG `user-dirs.dirs`), Laufwerke (`GetLogicalDrives`/`GetVolumeInformationW`/`GetDiskFreeSpaceExW`, `SEM_FAILCRITICALERRORS` gegen „Datenträger einlegen“-Dialoge) – `drives()` kann blockieren → nur im Hintergrund
- `crates/core`: zusätzlich `settings.rs` (key=value, tolerant, inkl. Sortierung), `renderer.rs` (`classify_gl`, `decide`, `Reason`, `status_de`), `selection.rs` (Explorer-Auswahlmodell, `remap` für Refresh), `typeahead.rs`, `breadcrumb.rs` (`crumbs`/`fit`, Win/UNC/Unix); `type_label`/`extension`, `SortKey::Type`
- `crates/ops` (`explorer-ops`): `exec.rs` (Job-Engine: `run` synchron/testbar, `spawn` Thread + `JobHandle` cancel/pause/answer/is_finished, Events Started/Progress/Conflict/Finished, `Report.done` = Undo-Journal), `undo.rs` (`entry_from_report`, `undo`, `TrashSnapshot`, `UndoStack`), `simple.rs` (rename/new_folder), `names.rs`, `clipboard.rs` (CF_HDROP, `examples/clip.rs` für CI-Interop), `win_copy.rs`; Tests `tests/ops.rs`
- `crates/shell` (`explorer-shell`): `icons.rs` (`icon_key`, `IconRegistry`, `bgra_to_rgba`, `load_icon`), `types.rs` (`type_name`, Cache pro Endung), `win.rs` (`SHGetFileInfoW`, HICON→RGBA, `CoInitializeEx`) – andere Plattformen liefern `None` → UI-Fallback
- Adresszeile: `core::address` (`expand_env`, `expand_home`, `shell_folder` → `KnownFolder`), `fs::places::known_folder_path`, nav `resolve_input` (nur Adresszeile/Seitenleiste; existierende Pfade unverändert)
- Netzwerk: `fs::network` (`server_root`/`share_of`/`share_root_parent` rein + getestet, `shares` = `NetShareEnum` Level 1, `connect` = Anmeldedialog `WNetAddConnection3W`); nav: Server-Pfad `\\nas` → `share-loader`-Thread, kein Watcher, `pane_dir` = None (nichts anlegen); `FsErrorKind::LoginRequired`, `ask_login` einmal pro Pfad (`Tab.login_tried`)
- Befehlspalette: `core::commands` (`ALL`, `Need`, `fuzzy_score`, `rank`, `push_recent`), `app/src/palette.rs` (Ziele Cmd/Go, `execute` → `nav::run_action`/`fileops::command`), nav `palette_places`, `History::recent`; **neue Befehle immer ins Verzeichnis + `execute`** (Test prüft das)
- R-001 Integration/Reset: `crates/integrate` (`model` KeyPath/RegData roh, `registry` Trait + `MemRegistry` (Tests, Snapshot per `same_content`), `win.rs` `WinRegistry` (64-bit-Sicht), `journal` (Format, `load` → None/Ok/Damaged, `save` Spiegel zuerst), `manifest::points()` (**jede neue Integration hier eintragen!**), `apply`/`reset`/`reset_with`/`sweep`/`status`), Bin `explorer2-recover`, Example `r001_selftest` (nur CI), `scripts/r001-snapshot.ps1` (reg.exe-Export vorher/nachher), App `src/integration.rs` (Status beim Öffnen der Einstellungen, Knopf/Palette → `Confirm::IntegrationReset`)
- M7 Integrationen: `integrate::features` (`Feature` FolderOpen/ContextMenu, `changes(exe)`, `scopes()`, `state` On/OnElsewhere/Partial/Off, `enable`/`disable`), `core::address::launch_arg` (`"C:\"` → `C:"`-Reparatur), App `integration.rs` (`set_feature`/`toggle_feature`/`startup` Neu-Verknüpfen), Trace für von Windows gestartete Instanzen: `EXPLORER2_TRACE_FILE`; Windows-E2E W42 (ShellExecute auf Ordner/Laufwerk, `InvokeVerb`)
- Farbschemata: `core::theme` (`ThemeChoice`, `Accent`, `tokens`, `tint`, `readable`, `contrast`), `shell::system_theme`/`watch_system_theme`/`set_dark_title_bar` (`win_theme.rs`; Overrides `EXPLORER2_SYSTEM_THEME=dark|light`, `EXPLORER2_ACCENT=#rrggbb`), `app/src/theme.rs` (State, `apply` → `Theme`-Global, Titelleiste mit HWND-Wiederholung); **neue UI-Farbe = neues Token in core + ThemeColors (Slint) + `colors()`** (Compiler erzwingt Vollständigkeit)
- Inspector: `core::inspect` (`image_size`, `text_preview`, `attributes_de`, `totals`, `count_de`), `fs::dir_stats` (+`_limited`, `MAX_ENTRIES`), `app/src/inspector.rs` (Entprellung, Generation, Hintergrund-Job, `InfoRow`-Modell), nav `inspect_target`, Hook in `sync_window`
- Ordneroptionen: `core::display` (`display_name`/`hidden_ext`/`rename_result`, rein), `ViewFilter.show_extensions`/`show_protected_os`, `shell::known_type` (HKCR, gecacht), `RowLabels::display_name` (Zeilenname = Anzeigename!, echte Namen nur in `Tab.entries`), `fileops::Rename.hidden_ext`, nav `set_option`/`click_open`; Slint `Timer` für Markieren durch Zeigen
- Kachel-Ansicht: `core::view` (`ViewMode`, `grid_columns`), `shell::thumbnail`, `app/src/thumbs.rs` (Worker + LRU, `want(keys)` → ids), nav `set_view`/`zoom`/`refresh_thumbs`/`set_cols`, `Tab.grid` + `Pane.cols_model` (stabile Raster-Modelle), `keys::map_key_grid`, `dnd::Geom` mit `line_h`/`cols`/`item_w`; Slint `Tile`-Komponente + zweite ListView `grid`
- Bibliotheken: `shell::libraries`/`library` (`IShellLibrary`, COM pro Loader-Thread via `init_thread`), Platzhalter `::bibliotheken` + `::bibliothek:<Stamm>` (`places::library_path`/`library_of`), nav `load_library` (1 Ordner → `Update::Redirect`), `VirtualData::Targets`/`Library`; Schutz-Check generell `places::is_placeholder`
- Virtuelle Orte: `core::places::VirtualPlace` (Platzhalter-Pfade), `ops::trashview` (Papierkorb), nav: `load_virtual`, `VirtualData` am Tab, `pane_place`/`selected_trash`; **in virtuellen Orten liefern `pane_dir`/`selected_paths`/`entry_at` None/leer** (Schutz vor Dateioperationen auf Platzhaltern)
- `crates/app/src/fileops.rs`: UI-Seite der Dateioperationen (Clipboard, Jobs/Panel, Dialoge, Rename, Undo/Redo, DnD); nav.rs liefert Helfer (`selected_paths`, `refresh_dirs` mit Nach-Auswahl/-Umbenennen, `show_rename`, `set_cut`)
- E2E M4: `scripts/e2e-ops.sh` (von e2e.sh aufgerufen, einzeln lauffähig)
- `crates/app` (`explorer2`): Slint-UI (`ui/main.slint`, Einstellungs-Panel), `src/nav.rs` = Fenster-Controller: `App` → `Pane` (1–2) → `Tab` (Pfad, Einträge, Auswahl, Loader, Watcher, Verlauf); Async-Ergebnisse per **stabiler Tab-ID** + Generationszähler; inaktive/restaurierte Tabs laden lazy, `stale` nach Sortier-/Filterwechsel; Session-Save entprellt 400 ms + beim Schließen, `src/keys.rs` = Tastenbelegung (`map_key`, rein/testbar), `src/open.rs` = Datei öffnen (`ShellExecuteW` / `xdg-open`, Override `EXPLORER2_OPEN_CMD`), `src/gfx.rs` = Renderer-Wahl vor Fenster, `src/config.rs` = settings.ini (`%APPDATA%\Explorer2` / XDG, Override `EXPLORER2_CONFIG_DIR`), atomar + Mutex
- Icon: `crates/app/res/icon/*.svg` → `scripts/make-icons.sh` → PNGs + `explorer2.ico` (Exe-Ressource) + `icon-256.png` (Fenster-Icon)
- CI: `.github/workflows/ci.yml` (fmt, clippy -D warnings, test auf Windows+Linux; Release-Artefakt `explorer2.exe`)
- E2E lokal: `xvfb-run -a -s "-screen 0 1280x800x24" scripts/e2e.sh $PWD/target/release/explorer2 <workdir>`
- Lokal prüfen: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- Slint-Gotcha: Widgets (`Button`, `ListView`) brauchen `import { … } from "std-widgets.slint";`

## Erkenntnisse / Gotchas
- Release-Exe ist GUI-Subsystem: nie `println!` (panikt bei fehlendem stdout, `panic=abort` killt App) → `let _ = writeln!(...)`.
- PowerShell `& exe` wartet bei GUI-Exes nicht → in CI `Start-Process -Wait -RedirectStandardOutput`.
- Sandbox: für Xvfb-UI-Test `libxkbcommon-x11-0` nötig; Start mit `SLINT_BACKEND=winit-software xvfb-run ...`.
- Windows-Cross-Check lokal möglich für reine Rust-Crates: `cargo clippy -p explorer-core -p explorer-fs --target x86_64-pc-windows-msvc` (App-Crate nicht, wegen Skia/MSVC).
- CI-Status per GitHub-MCP (`actions_list` / `get_job_logs`), kein `gh`.
- Slint: `text: prop` auf TextInput ist Einweg und **bricht beim Tippen** → für editierbare Felder immer `text <=> prop` (in-out).
- Slint: `viewport-y` ist deprecated → `content-y`.
- Background→UI: `slint::invoke_from_event_loop` + Generationszähler; UI-State im `thread_local!` (Rc nicht Send).
- `cargo fmt` bricht lange Zeilen um → Python-String-Replaces danach prüfen (`assert s.count(old)==1`).
- E2E unter Xvfb ohne Window-Manager: `xdotool windowfocus` statt `windowactivate`; Debug-Trace via `EXPLORER2_TRACE=1` (stderr).
- Trace-Log nie in beobachteten Ordner schreiben, außer zum Testen der Drossel.
- Trace-Zeilen: `at=` = ms seit Prozessstart (Kaltstart-Messung), `data_ready=` = ms navigate→Daten fertig, gemessen im Loader-Thread (Daten-Latenz, Budget-Check). Slint initialisiert den Renderer erst beim ersten Frame **nach** Loop-Start → „event loop running“ ist KEIN Fenster-bereit-Marker; `set_rendering_notifier` ist bei Software-Renderern nicht verfügbar → nicht als Marker nutzen.
- GitHub-Runner (Windows) haben kein GPU-OpenGL → Skia fällt zurück (funktioniert, kostet **~450 ms** Startzeit; Software-Renderer startet in 43 ms).
- Windows-Runner: sporadische OS-Stalls im Dateisystem bis 1,5 s (Defender/Indexer nach Massen-Anlage, ~2 von 9 Läufen) → Perf immer mit roher OS-Messung daneben auswerten (`bench` gibt pro Lauf `raw: first_entry` aus).
- Slint: Renderer per `slint::BackendSelector::new().backend_name("winit").renderer_name("skia"|"software").select()` **vor** dem ersten Fenster; `"skia"` fällt selbst zurück, `"skia-opengl"` nicht. `SLINT_BACKEND` gewinnt immer.
- `xprop` zeigt `_NET_WM_ICON` im Pretty-Modus leer → mit `-notype 32c` Rohdaten lesen (Icon war gesetzt).
- Mesa für Windows-GL-Tests: **MinGW**-Paket nehmen (26.x MSVC-Paket hat kein Software-OpenGL); App-lokale `opengl32.dll` wird von Probe + Skia genutzt.
- GPU-Probe-Kosten schwanken stark (Runner 3–581 ms, Mesa-llvmpipe 239 ms) → deshalb läuft sie nur im Hintergrund.
- Erststart-Gate schwankt bei **gleichem Code** stark mit der VM (#54: 1178 ms, #55: 731 ms; #57: 2709 ms bei `probe_ms` 521 vs #58: 677 ms bei 84,5) → bei Rot erst Zahlen vergleichen (`window created` liegt vor jedem App-Code, `probe_ms` steht als VM-Kennzahl im Log), nicht blind optimieren; Zeit-Gates werden gesammelt und erst am Matrix-Ende geworfen, E2E läuft auch nach rotem Gate.
- Allererster Start auf frischer Windows-VM zahlt Einmalkosten (Fenster-Erstellung 755 ms statt ~5 ms: Dateicache/Fonts) → Kaltstart-Budget 300 ms wird an normalen Starts (Cache) best-of-3 geprüft, Erststart nur Plausibilität < 1 s und ausgewiesen.
- **UI-Zustände per Pixel messen** (`convert img -format '%[pixel:p{x,y}]' info:`), nie per Augenmaß auf verkleinerten Screenshots – ein vermeintlicher Render-Bug (fokussierte Zeile „ohne“ Auswahlfarbe) war ein Lesefehler.
- Slint: `changed prop` feuert **nicht** bei der Initialisierung → eine neu instanziierte Repeater-Instanz (z. B. neue Split-Pane) verpasst einen vorher gesetzten Sequenz-Auftrag. Fokus deshalb an Zustand koppeln (`changed active` + `init`), nicht nur an Zähler.
- Slint: Funktionen von Repeater-Instanzen sind aus Rust nicht aufrufbar → Aufträge (Scroll, Edit, Fokus) über `*-seq`-Zähler in den Modelldaten + `changed`.
- Slint: Root-`FocusScope` mit `key-pressed` als Fallback, damit Shortcuts auch ohne fokussierte Liste greifen.
- Drosseln/Rate-Limits immer ab **Start** einer Aktion messen, nicht ab Ende (sonst Überlappung bei langsamer Maschine).
- E2E: pro Block eigenes Log (`trace-<n>.log`), sonst sind Fehler nachträglich nicht analysierbar; `EXPLORER2_TRACE_KEYS=1` loggt jede Taste mit Pane.
- Sicherheitscheck blockt `rm -rf $VAR` in `bash -c`-Strings → eindeutige Temp-Ordner statt Löschen.
- Lange CI-Logs: `get_job_logs` mit `return_content=false` → URL per `curl` laden und greppen (tail wird von Cache-Upload-Zeilen geflutet).

- Slint DnD: TouchArea muss **Kind** der `DragArea` sein (als Geschwister darüber schluckt sie den Druck); `DragAction` ist in Rust nicht öffentlich → Callbacks liefern int (0/1/2), Slint mappt. `pure callback` für `data:`-Bindings.
- Python-Replaces in Dateien mit Windows-Pfaden: **raw strings** (`r"..."`) – `\a`/`\b` werden sonst zu Steuerzeichen.
- PowerShell 7 hat kein `Get-Clipboard -Format FileDropList` → `shell: powershell` (5.1) für Clipboard-Tests.
- Windows PowerShell 5.1 `Get-Clipboard -Format FileDropList` lieferte in CI leer, obwohl `FileDrop` auf dem Clipboard lag → für Tests `[System.Windows.Forms.Clipboard]::GetFileDropList()` (= Weg anderer Apps) nutzen.
- CI nutzt das **neueste** stable (neue Clippy-Lints, z. B. 1.99 `chunks_exact_to_as_chunks`) → vor dem Push `rustup update stable` lokal.
- Windows: Pfade können **8.3-Kurznamen** enthalten (`C:\Users\RUNNER~1\…`, z. B. aus `%TEMP%`), Shell/Papierkorb liefern Langnamen → Pfade nie roh vergleichen: Elternordner kanonisieren + case-insensitiv (`undo::same_location`).
- Windows-Papierkorb (`trash`-Crate): `TrashItem.name` ist der **Anzeigename** (Endung bei Standard-Einstellung ausgeblendet: „weg“ statt „weg.txt“) und wird beim Restore als Zielname benutzt → Abgleich über Stamm + Endung der `$R…`-Datei (`item.id`), vor `restore_all` `item.name` auf den echten Namen setzen.
- `windows`-Crate `#[implement]` expandiert zu `windows_core::…` → `windows-core` als **direkte** Abhängigkeit nötig.
- Modale Windows-Schleifen (`SHDoDragDrop`, `TrackPopupMenuEx`) nie direkt aus einem Slint-Callback, der danach noch Slint-State anfasst → Start per `slint::Timer::single_shot(ZERO)`; nach `DoDragDrop` Pointer-Release per `dispatch_event` nachreichen (OLE schluckt das Loslassen).
- Slint erzeugt das native Fenster erst **irgendwann nach** Event-Loop-Start (nicht zwingend im ersten Timer) → HWND-abhängiges Setup (Drop-Target) mit Wiederholung, bis `window_handle` ≠ 0 (CI #32/#33: „Invalid window handle“).
- OLE-Drag nur starten, wenn die Primärtaste **noch gedrückt** ist (`primary_button_down`) – sonst wartet OLE auf die nächste Eingabe und legt beim nächsten Mausbewegen ab.
- Windows-Dialogtitel (Eigenschaften) zeigen Namen **ohne** bekannte Endung („a Properties“) → Tests nicht auf „a.txt“ matchen.
- PowerShell `New-SmbShare`: Kommentar heißt `-Description` (nicht `-Remark`, das ist der NetAPI-Name); Runner hat nur Admin-Freigaben → für Netzwerk-Tests eigene Freigabe anlegen.
- SendKeys: `%` = Alt, `+` = Shift, `^` = Strg → wörtlich als `{%}`, `{+}`, `{^}` senden.
- CI läuft nicht bei reinen Doku-Commits (`paths-ignore`) → Doku-Abschluss braucht keinen eigenen Run.
- **Slint: Repeater über ein `int` (`for x in n`) baut bei JEDER Neubewertung alle Instanzen neu** (Modelle werden per Zeiger verglichen, `int` erzeugt jedes Mal ein neues) → Hover/Doppelklick gehen verloren. Für dynamische Listen immer ein stabiles `VecModel` aus Rust, nur am Ende wachsen/schrumpfen. Prüfen mit `init => { debug(…) }`.
- UI-Texte nur mit Zeichen, die Segoe UI sicher hat (Latin, Umlaute, Pfeile ←→↑↶↷›); Symbole wie ▾ ✓ als `Path`/Rectangle zeichnen – **gemessen**: ein „▾“ kostete auf frischer VM +1,2 s beim ersten Frame (CI #50 vs #51); Trace-Marken `init done`/`run`/`event loop running` grenzen den Start ein.
- Slint: `visible: false` spart **nicht** den Aufbau/das Layout beim ersten Frame → selten gebrauchte Overlays (Palette, Dialoge) unter `if …` legen, Fokus per `init` (CI #53).
- Slint TouchArea: `clicked` kommt **vor** `pointer-event(up)` → Zustand, den `clicked` prüft, erst im Pointer-Ereignis zurücksetzen.
- Slint: `row`/`col` sind auf jedem Element reserviert (GridLayout) → Property nicht `row` nennen.
- Windows-Server-Runner: PNG-Vorschaubilder ja, **BMP nein** (kein Anbieter) → Vorschau-Tests mit PNG; `shell::thumbnail` meldet `preview` ehrlich.
- X11/Xvfb: ein Mausrad-Klick kann doppelt ankommen → Rad-Aktionen drosseln; E2E-Trace nie in den beobachteten Ordner legen (Refresh-Schleife).
- Windows-E2E: `scripts/e2e-windows.ps1` (SendKeys + SetCursorPos/mouse_event, Fenster maximiert, Runner-Bildschirm 1024×768, DPI 1.0); Foreground-Lock mit Alt-Druck vor `SetForegroundWindow` umgehen.
- Papierkorb-Restore immer gegen Snapshot **vor** der Aktion (gleicher Originalpfad kann mehrfach im Papierkorb liegen).
- Slint std-widgets exportieren ein Global `Palette` → eigene Komponente nicht `Palette` nennen (Befehlspalette heißt `CommandPalette`). `Palette.color-scheme` ist in-out: setzen = Std-Widgets fest hell/dunkel; Slint färbt Std-Widgets selbst mit dem Windows-Akzent (`DwmGetColorizationColor`, nur Farbton).
- Palette-Befehle laufen per `Timer::single_shot(ZERO)` **nach** dem Schließen der Palette: sonst gewinnt die Fokus-Rückgabe an die Liste gegen den Dialog-Fokus, Enter im Dialog geht ins Leere (CI #60 W41, galt auch für „Endgültig löschen“/„Papierkorb leeren“ aus der Palette).
- Windows-E2E: Shell-Dialoge (Eigenschaften) kommen asynchron und manchmal spät → pollen statt fester Wartezeit und warten, bis sie zu sind (sonst klaut ein später Dialog den Fokus der Folgeschritte, CI #60 W11→W17).
- Windows-Kommandozeile: `"C:\"` (Laufwerk aus `%1`) kommt nach den CRT-Regeln als `C:"` an (`\"` = Anführungszeichen) → Start-Argumente immer durch `launch_arg`.
- Tests gegen die echte Registry nie mit `reset()` (fegt die echten Integrationsstellen = würde eine aktive Integration des Entwicklers entfernen) → `reset_with(.., &[])`.
- Snapshot-Tests nie über ganz `HKCU\Software\Classes` (MuiCache u. a. ändern sich beim Programmstart von selbst) → nur die Integrationsbereiche exportieren.
- Sandbox-Plattenkontingent: `target/debug` wächst auf >10 GB → bei „Bus error“ im Linker/„no space“ `rm -rf target/debug`.
- Registry-Theme direkt geschrieben (Test) sendet **kein** WM_SETTINGCHANGE → winit merkt nichts, unser Watcher schon; winit setzt die Titelleiste bei echten Systemwechseln selbst → wir setzen sie 400 ms danach erneut.

## Referenzen
- File Pilot: https://filepilot.tech (Panels/Tabs, Inspector, GoTo, Command Palette, pinbare Context-Menu-Einträge, ~2,5 MB Exe)
- Everything/voidtools: Name-Index per NTFS MFT + USN-Journal (~1 Mio Dateien in ~1 min, ~100 MB RAM)
- Sweetch: https://github.com/krllmrzv/Sweetch (Kreismenü per Drag / Ctrl+Alt+Space, Konvertierung/Tools/Aktionen)
