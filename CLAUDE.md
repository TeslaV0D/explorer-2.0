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
| D-009 | Renderer: **GPU-Probe vor Fensterstart** (WGL: GL ≥ 3.0 + `glCreateShader`, Software-GL wie GDI Generic/llvmpipe = unbrauchbar) → Skia (GPU) oder Software. Ergebnis gecacht in settings.ini (pro App-Version), Hintergrund-Re-Probe 5 s nach Start. Einstellung „Hardwarebeschleunigung verwenden, wenn verfügbar“ (wie Browser), Override `EXPLORER2_RENDERER=auto|gpu|software`; `SLINT_BACKEND` hat Vorrang | fix (Boss, 2026-11-03) |
| D-010 | App-Icon: gelber Ordner + blaues Band (Windows-11-Explorer-Anmutung) auf heller abgerundeter Kachel (Apple-Files-Anmutung), Vorwärts-Chevrons = „2.0“; 16–32 px vereinfachte Variante ohne Kachel | fix (M1.5) |
| D-008 | Perf-Gates in CI auf **best-of-N** (≥ 9 Läufe), Median/Max/OS-Stalls nur ausweisen – Shared-Runner haben sporadische OS-Stalls (gemessen 168 ms im rohen FindFirstFileExW); Budgets selbst unverändert | fix (M1) |

## Arbeitsumgebung / Constraints
- Dev-Sandbox ist **Linux**, Zielplattform **Windows**. Windows-Code kann hier nicht ausgeführt werden → Build/Test via GitHub Actions (`windows-latest`); Kernlogik plattformneutral halten, damit sie auch unter Linux testbar ist.
- Branch: `claude/relaxed-pasteur-ufhh1p`. Kein PR ohne explizite Ansage.
- User-Präferenz: Anrede „Boss“, Bro-Ton, auf Deutsch antworten, kurz & präzise.

## Meilenstein-Stand
- M0 ✅ · M1 ✅ (2026-11-03, Run #10) · M1.5 Renderer-Wahl + Einstellungen + Icon (lokal ✅, CI ausstehend) · danach M2
- Arbeitsregel (Boss): nächste Stufe erst, wenn die aktuelle vollständig abgeschlossen, getestet und CI-grün ist.

## Code-Layout (seit M0)
- `crates/core` (`explorer-core`): plattformneutrales Modell, Sortierung (natürlich, Ordner zuerst), Unit-Tests
- `crates/fs` (`explorer-fs`): `enumerate`/`read_dir` (Windows: `win.rs` FindFirstFileExW; sonst `portable.rs`), `path.rs` (Verbatim/FILETIME, plattformneutral getestet), `loader.rs` (Hintergrund-Load, Batches, Cancel), `watch.rs` (notify + `Debounce`), Benchmark `examples/bench.rs`
- `crates/gpu` (`explorer-gpu`): `probe_opengl()` – Windows-WGL-Probe (verstecktes Fenster, Legacy-Kontext), sonst `NotSupported`; ohne Skia → lokal per `--target x86_64-pc-windows-msvc` prüfbar
- `crates/core`: zusätzlich `settings.rs` (key=value, tolerant) und `renderer.rs` (`classify_gl`, `decide`, `Reason`, `status_de`)
- `crates/app` (`explorer2`): Slint-UI (`ui/main.slint`, Einstellungs-Panel), `src/nav.rs` = Navigations-Controller (State im `thread_local` auf UI-Thread, Generationszähler, Refresh-Drossel), `src/gfx.rs` = Renderer-Wahl vor Fenster, `src/config.rs` = settings.ini (`%APPDATA%\Explorer2` / XDG, Override `EXPLORER2_CONFIG_DIR`), atomar + Mutex
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
- Sicherheitscheck blockt `rm -rf $VAR` in `bash -c`-Strings → eindeutige Temp-Ordner statt Löschen.
- Lange CI-Logs: `get_job_logs` mit `return_content=false` → URL per `curl` laden und greppen (tail wird von Cache-Upload-Zeilen geflutet).

## Referenzen
- File Pilot: https://filepilot.tech (Panels/Tabs, Inspector, GoTo, Command Palette, pinbare Context-Menu-Einträge, ~2,5 MB Exe)
- Everything/voidtools: Name-Index per NTFS MFT + USN-Journal (~1 Mio Dateien in ~1 min, ~100 MB RAM)
- Sweetch: https://github.com/krllmrzv/Sweetch (Kreismenü per Drag / Ctrl+Alt+Space, Konvertierung/Tools/Aktionen)
