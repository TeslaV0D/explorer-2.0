# Zwischenstand

**Stand:** 2026-11-03 · **Phase:** 1 · **M0 ✅ abgeschlossen** · nächster Meilenstein: **M1 (fs-Kern)** · **Branch:** `claude/relaxed-pasteur-ufhh1p`

## M0 – Ergebnis
- Cargo-Workspace `core` / `fs` / `app`, Slint-Fenster mit Pfadleiste, Hoch-Button, Ordnerliste (Doppelklick öffnet).
- Windows-Manifest eingebettet (PerMonitorV2, longPathAware, UTF-8-Codepage, asInvoker).
- CI-Run #3 (`6cab9c1`) komplett grün: fmt, clippy `-D warnings`, Tests auf Windows + Linux, Release-Build, Smoke-Test (`--version` → `explorer2 0.0.1`, Manifest-Strings in Exe geprüft), Artefakt `explorer2-windows-x64` hochgeladen.
- Release-Exe: **17,2 MB** (18 081 280 B), Release-Build ~2,5 min mit Cache.
- Lokal: UI unter Xvfb gestartet und per Screenshot geprüft.

## Bugs gefunden & gefixt während M0
- UI: Hoch-Button gestreckt, Datum als Epoch-Zahl, Ordner-Emoji kaputt → gefixt.
- Release-Exe crashte bei `--version` (GUI-Subsystem + `panic=abort` + `println!` auf geschlossenes stdout) → panikfreie Ausgabe + `AttachConsole`.
- Smoke-Test-Bug: PowerShell wartet nicht auf GUI-Exes → `Start-Process -Wait`.

## Nicht verifiziert (bewusst)
- GUI-Fenster auf echtem Windows nicht visuell gesehen (CI-Runner ohne Desktop-Test) → Boss kann Artefakt aus CI-Run #3 herunterladen und starten.

## Nächste Schritte (M1)
1. Windows-Fastpath Enumeration (`FindFirstFileExW` + `FIND_FIRST_EX_LARGE_FETCH`), Attribute (hidden/system/reparse).
2. Async-Laden (UI-Thread blockiert nie), Watcher (`ReadDirectoryChangesW` / notify), Debounce.
3. Benchmark: 100k-Dateien-Ordner, Budget erste Zeilen < 100 ms.
