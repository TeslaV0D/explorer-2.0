# TO-DO

Legende: `[ ]` offen · `[~]` in Arbeit · `[x]` erledigt · Prio P0/P1/P2

## Jetzt (Planung)
- [x] Projektstruktur + Doku-Dateien anlegen (CLAUDE.md, PROJECT, PLAN-BASICS, WINDOWS-INTEGRATION, TODO, STATUS)
- [x] Referenzen recherchiert (File Pilot, Sweetch, Everything)
- [x] D-001 entschieden: Rust + Slint
- [ ] Phase-1-Plan von Boss abnehmen lassen / anpassen (läuft implizit weiter, Anpassungen jederzeit)

## Phase 1 – Basis (Details: `PLAN-BASICS.md`)
- [x] M0 Repo-Setup: Cargo-Workspace, CI (Windows+Linux, fmt/clippy/test, Release-Build + Exe-Smoke-Test), Skelett-Fenster, Windows-Manifest (PerMonitorV2, longPathAware, UTF-8) – **abgeschlossen 2026-11-03, CI-Run #3 grün**
- [x] M1 fs-Kern – **abgeschlossen 2026-11-03, CI-Run #10 komplett grün**
  - [x] Windows-Fastpath `FindFirstFileExW` (`FindExInfoBasic` + `LARGE_FETCH`), Attribute aus Find-Daten (hidden/system/readonly/Link), Cloud-Platzhalter ≠ Link
  - [x] Langpfade via `\?\`-Verbatim (inkl. UNC, `.`/`..`-Normalisierung), FILETIME→Unix
  - [x] Portabler Pfad (std) für Linux/Tests, Symlinks/Dangling-Links
  - [x] Hintergrund-Loader: Batches (256 zuerst, dann 8192), Abbruch, Fehlerklassen (NotFound/Zugriff verweigert/Kein Ordner)
  - [x] Watcher (`notify`: ReadDirectoryChangesW/inotify) + Debounce (100 ms ruhig / max 500 ms) + adaptive Refresh-Drossel (≥500 ms, 4× Ladezeit, max 10 s)
  - [x] Explorer-Sichtbarkeitsregeln (Ausgeblendete / geschützte Systemdateien) + Toggle „Ausgeblendete“
  - [x] Natürliche Sortierung allokationsfrei (100k: 173 → 47 ms), deterministisch bei „gleichen“ Namen
  - [x] UI: Laden nie im UI-Thread, Generationszähler gegen veraltete Ergebnisse, Statusleiste, Fehleranzeige, Rückfall auf Elternordner wenn aktueller Ordner verschwindet
  - [x] Benchmark-Tool (`cargo run --release -p explorer-fs --example bench -- --assert`) + CI-Budgets
  - [x] E2E-Test unter Xvfb (13 Checks): `scripts/e2e.sh`, läuft als CI-Job `e2e-linux`
  - [x] CI grün inkl. Windows-Benchmark + Windows-UI-Smoke-Test (Run #6)
  - [x] CI grün mit getrennter Start-/Daten-Messung (Run #9: Daten 0,9 ms / 140 ms auf Windows)
  - [x] CI grün mit stabilem Bench-Gate (best-of-9, Run #10: Median 1,17 ms / 115 ms, OS-Stalls 2/9 abgefangen)
- [ ] M2 UI-Shell: Fenster, virtualisierte Liste, Details-Ansicht, Adressleiste
- [ ] M3 Tabs, Split-View, Sidebar, Verlauf, Tastatur, Session-Restore
- [ ] M4 Dateioperationen: Queue, Undo/Redo, Clipboard, Drag&Drop, Konfliktdialog
- [ ] M5 Shell-Integration: Icons/Thumbnails, Kontextmenü (Dritte), Properties, Papierkorb, Netzwerk, virtuelle Ordner
- [ ] M6 Ordneroptionen-Parität, Themes, Command Palette, Inspector
- [ ] M7 Windows-Integration Stufe 1–3 (siehe `WINDOWS-INTEGRATION.md`) + Verifikationspunkte auf echtem Windows
- [ ] M8 Installer, Recovery-Tool, Härtung, Beta

## Technische Schulden / Follow-ups
- [ ] Exe-Größe 17,2 MB (Budget < 20 MB ok, Ideal < 10 MB verfehlt) → in M2 prüfen: Skia vs. femtovg/Software-Renderer, `opt-level="z"`-Vergleich
- [ ] **Renderer-Entscheidung** (gemessen Run #9, Windows ohne GPU-GL): Default/Skia 488 ms vs. Software 43 ms Kaltstart → Optionen: Software als Default, GL-Fähigkeit vorab prüfen, D3D zuerst. Entscheiden in M2 (Boss)
- [ ] Exe-Icon (`.ico` in `explorer2.rc`) – braucht Grafik/Logo
- [ ] `NtQueryDirectoryFile` als Alternative zum FindFirstFileExW messen (nur falls Windows-Bench knapp wird)
- [ ] GitHub-Actions auf Node-24-Versionen heben (checkout@v4 → v5 etc., Deprecation-Warnung in CI)

## Phase 2 – Systemweite Suche (voidtools/Everything-Stil)  *(später, nur beschrieben)*
- [ ] NTFS-Index über MFT-Scan + USN-Journal-Live-Updates (Dienst/Admin-Helper nötig, Alternative: Verzeichnis-Walk für FAT/Netz)
- [ ] Kompakter In-Memory-Index (Ziel: ~1 Mio Dateien ≈ 100 MB RAM, Initial-Index < 1 min)
- [ ] Instant-Suche beim Tippen (<10 ms/Query), Sortierung nach Name/Pfad/Größe/Datum, Regex, Wildcards, Filter (`ext:`, `size:`, `dm:` – Everything-Syntax)
- [ ] Persistenter Index (schneller Start), Ausschlusslisten, Netz/Wechseldatenträger optional
- [ ] Integration: Suchfeld in jedem Tab, globale Hotkey-Suche, später optional Inhaltssuche

## Phase 3 – Datei-Aktions-Shortcuts (Sweetch-Stil)  *(später, nur beschrieben)*
- [ ] Tray-Helper mit Kreismenü: Drag von Dateien auf Sektor oder Hotkey (Sweetch: Ctrl+Alt+Space) 
- [ ] Modifier-Gruppen: Shift = Konvertieren, Ctrl = Tools, Alt = Aktionen
- [ ] Aktionen: Konvertieren (Bild/Audio/Video/PDF/Doc via ffmpeg/pandoc – optional nachladbar), Archivieren, Kopieren nach…, Umbenennen, eigene Skripte
- [ ] Konfigurierbare Sektoren/Profile, lokale Verarbeitung (keine Cloud)
- [ ] Lizenz klären: Sweetch ist MIT → Ideen/Code-Nutzung mit Attribution möglich

## Phase 4 – Später
- [ ] Plugins/Skripting, Settings-Sync, Auto-Update, ARM64, Code-Signing
