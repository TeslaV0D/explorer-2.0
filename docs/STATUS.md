# Zwischenstand

**Stand:** 2026-11-03 · **Phase:** 1 – M0 fast fertig · **Branch:** `claude/relaxed-pasteur-ufhh1p`

## Erledigt
- Planung + Doku (siehe `CLAUDE.md`), D-001 = **Rust + Slint**.
- M0-Skelett: Cargo-Workspace (`core`, `fs`, `app`), CI-Workflow, Slint-Fenster mit Pfadleiste, „Hoch“-Button, Ordnerliste (Doppelklick öffnet Ordner).
- Lokal grün (Linux): `cargo fmt --check`, `clippy -D warnings`, `cargo test` (Sortierung/natürliche Reihenfolge, Verzeichnis-Lesen inkl. Fehlerfall). `cargo check` für die UI läuft durch.

## Bugs gefunden & gefixt (M0-Tests)
- UI (per Xvfb-Screenshot): Hoch-Button über halbe Breite gestreckt; Datum als roher Epoch-Wert; Ordner-Emoji in Software-Renderer kaputt → alle gefixt.
- CI-Run #2: Release-Exe (GUI-Subsystem, `panic=abort`) crashte bei `--version`, weil `println!` auf geschlossenes stdout paniert → sichere Ausgabe (`writeln!` ohne Panic) + `AttachConsole(ATTACH_PARENT_PROCESS)`. Smoke-Test nutzte `& exe`, PowerShell wartet bei GUI-Exes nicht → `Start-Process -Wait`.

## Nicht verifiziert
- UI wurde nur *kompiliert*, nicht visuell gestartet (kein Display in der Sandbox).
- Windows-Build/CI-Lauf noch nicht gesehen – erster Push triggert ihn.

## Offen
- Windows-Manifest (DPI/Langpfade) + Exe-Icon.
- Skia-Renderer auf Windows-CI bestätigen; falls Build zu schwer/langsam → Fallback `renderer-femtovg` oder nur Software.

## Nächste Schritte
1. CI-Ergebnis prüfen, Manifest ergänzen → M0 abschließen.
2. M1: Windows-Fastpath für Enumeration (`FindFirstFileExW`), Watcher, Benchmark mit 100k Dateien.
