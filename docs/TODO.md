# TO-DO

Legende: `[ ]` offen · `[~]` in Arbeit · `[x]` erledigt · Prio P0/P1/P2

## Jetzt (Planung)
- [x] Projektstruktur + Doku-Dateien anlegen (CLAUDE.md, PROJECT, PLAN-BASICS, WINDOWS-INTEGRATION, TODO, STATUS)
- [x] Referenzen recherchiert (File Pilot, Sweetch, Everything)
- [x] D-001 entschieden: Rust + Slint
- [ ] Phase-1-Plan von Boss abnehmen lassen / anpassen (läuft implizit weiter, Anpassungen jederzeit)

## Phase 1 – Basis (Details: `PLAN-BASICS.md`)
- [~] M0 Repo-Setup: [x] Cargo-Workspace, [x] CI (Windows+Linux), [x] rustfmt/clippy, [x] Skelett-Fenster mit Ordnerliste · [ ] Windows-Manifest (PerMonitorV2, longPathAware) + Icon (`build.rs`/winresource) · [ ] CI-Lauf auf GitHub grün bestätigen
- [ ] M1 fs-Kern: Enumeration (FindFirstFileEx/NtQueryDirectoryFile), Sortierung, Watcher, Benchmarks (100k Dateien)
- [ ] M2 UI-Shell: Fenster, virtualisierte Liste, Details-Ansicht, Adressleiste
- [ ] M3 Tabs, Split-View, Sidebar, Verlauf, Tastatur, Session-Restore
- [ ] M4 Dateioperationen: Queue, Undo/Redo, Clipboard, Drag&Drop, Konfliktdialog
- [ ] M5 Shell-Integration: Icons/Thumbnails, Kontextmenü (Dritte), Properties, Papierkorb, Netzwerk, virtuelle Ordner
- [ ] M6 Ordneroptionen-Parität, Themes, Command Palette, Inspector
- [ ] M7 Windows-Integration Stufe 1–3 (siehe `WINDOWS-INTEGRATION.md`) + Verifikationspunkte auf echtem Windows
- [ ] M8 Installer, Recovery-Tool, Härtung, Beta

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
