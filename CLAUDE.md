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

## Arbeitsumgebung / Constraints
- Dev-Sandbox ist **Linux**, Zielplattform **Windows**. Windows-Code kann hier nicht ausgeführt werden → Build/Test via GitHub Actions (`windows-latest`); Kernlogik plattformneutral halten, damit sie auch unter Linux testbar ist.
- Branch: `claude/relaxed-pasteur-ufhh1p`. Kein PR ohne explizite Ansage.
- User-Präferenz: Anrede „Boss“, Bro-Ton, auf Deutsch antworten, kurz & präzise.

## Code-Layout (seit M0)
- `crates/core` (`explorer-core`): plattformneutrales Modell, Sortierung (natürlich, Ordner zuerst), Unit-Tests
- `crates/fs` (`explorer-fs`): Verzeichnis-Enumeration (portabel via std; Windows-Fastpath folgt in M1)
- `crates/app` (`explorer2`): Slint-UI (`ui/main.slint`), Binary
- CI: `.github/workflows/ci.yml` (fmt, clippy -D warnings, test auf Windows+Linux; Release-Artefakt `explorer2.exe`)
- Lokal prüfen: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- Slint-Gotcha: Widgets (`Button`, `ListView`) brauchen `import { … } from "std-widgets.slint";`

## Referenzen
- File Pilot: https://filepilot.tech (Panels/Tabs, Inspector, GoTo, Command Palette, pinbare Context-Menu-Einträge, ~2,5 MB Exe)
- Everything/voidtools: Name-Index per NTFS MFT + USN-Journal (~1 Mio Dateien in ~1 min, ~100 MB RAM)
- Sweetch: https://github.com/krllmrzv/Sweetch (Kreismenü per Drag / Ctrl+Alt+Space, Konvertierung/Tools/Aktionen)
