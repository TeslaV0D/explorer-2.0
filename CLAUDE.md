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

## Arbeitsumgebung / Constraints
- Dev-Sandbox ist **Linux**, Zielplattform **Windows**. Windows-Code kann hier nicht ausgeführt werden → Build/Test via GitHub Actions (`windows-latest`); Kernlogik plattformneutral halten, damit sie auch unter Linux testbar ist.
- Branch: `claude/relaxed-pasteur-ufhh1p`. Kein PR ohne explizite Ansage.
- User-Präferenz: Anrede „Boss“, Bro-Ton, auf Deutsch antworten, kurz & präzise.

## Code-Layout (seit M0)
- `crates/core` (`explorer-core`): plattformneutrales Modell, Sortierung (natürlich, Ordner zuerst), Unit-Tests
- `crates/fs` (`explorer-fs`): `enumerate`/`read_dir` (Windows: `win.rs` FindFirstFileExW; sonst `portable.rs`), `path.rs` (Verbatim/FILETIME, plattformneutral getestet), `loader.rs` (Hintergrund-Load, Batches, Cancel), `watch.rs` (notify + `Debounce`), Benchmark `examples/bench.rs`
- `crates/app` (`explorer2`): Slint-UI (`ui/main.slint`), `src/nav.rs` = Navigations-Controller (State im `thread_local` auf UI-Thread, Generationszähler, Refresh-Drossel)
- CI: `.github/workflows/ci.yml` (fmt, clippy -D warnings, test auf Windows+Linux; Release-Artefakt `explorer2.exe`)
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

## Referenzen
- File Pilot: https://filepilot.tech (Panels/Tabs, Inspector, GoTo, Command Palette, pinbare Context-Menu-Einträge, ~2,5 MB Exe)
- Everything/voidtools: Name-Index per NTFS MFT + USN-Journal (~1 Mio Dateien in ~1 min, ~100 MB RAM)
- Sweetch: https://github.com/krllmrzv/Sweetch (Kreismenü per Drag / Ctrl+Alt+Space, Konvertierung/Tools/Aktionen)
