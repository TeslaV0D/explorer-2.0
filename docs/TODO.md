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
- [x] M1.5 Renderer-Wahl + Einstellungen + Icon – **abgeschlossen 2026-11-03, CI-Run #13 grün**
  - [x] GPU-Probe (WGL) + Klassifizierung (Software-GL = unbrauchbar), Cache in settings.ini, Hintergrund-Re-Probe
  - [x] Einstellung „Hardwarebeschleunigung verwenden, wenn verfügbar“ (Panel „Einstellungen“), wirkt beim nächsten Start
  - [x] settings.ini (atomar, tolerant), „Ausgeblendete“ wird gespeichert
  - [x] App-Icon (SVG → ICO/PNG), Exe-Icon + Fenster-Icon
  - [x] Linux lokal: 6 Umgebungen (ohne GL, Mesa-GL, forciert GPU/Software, Setting aus) + E2E 20/20
  - [x] CI: Windows-Umgebungsmatrix (7 Szenarien) + Linux-GL-Check + Exe-Icon-Check grün (Run #13)
- [x] M2 UI-Shell – **abgeschlossen 2026-11-03, CI-Run #16 grün**
  - [x] Details-Ansicht: Spalten Name / Änderungsdatum / Typ / Größe, Klick sortiert (▲▼, 2. Klick dreht), Spaltenbreiten ziehbar, Sortierung gespeichert
  - [x] Typ-Spalte („Dateiordner“, „PNG-Datei“, „Datei“) + Sortierung nach Typ (echte Shell-Typnamen in M5)
  - [x] Auswahl wie Explorer: Klick, Strg+Klick, Umschalt+Klick, Klick ins Leere hebt auf; Auswahl bleibt bei Auto-Refresh/Umsortieren erhalten (per Name)
  - [x] Tastatur: Pfeile, Bild↑↓, Pos1/Ende (+Umschalt/Strg), Strg+A, Strg+Leertaste, Enter, Rücktaste/Alt+↑, F5, Strg+L/Alt+D, Typeahead (inkl. Buchstaben-Zyklus)
  - [x] Öffnen: Doppelklick/Enter → Ordner navigieren, Dateien mit Standard-App (ShellExecuteW), mehrere Dateien (max. 20)
  - [x] Breadcrumb-Adressleiste (klickbare Segmente, „…“-Kürzung, Klick ins Leere/Strg+L = Bearbeiten, Esc = zurück)
  - [x] Nach „Hoch“ ist der Herkunftsordner markiert; Liste hat beim Start Tastaturfokus
  - [x] CI grün (inkl. Windows-ShellExecute-Test, Run #16)
- [ ] M2-Rest/Politur (später): Spaltenbreiten speichern, Spalten ein/ausblenden, Ansichten Liste/Symbole/Kacheln (Plan 4.2), Gruppieren
- [x] M3 Tabs, Split-View, Sidebar, Verlauf, Session-Restore – **abgeschlossen 2026-11-03, CI-Run #19 grün**
  - [x] Tabs pro Pane: neu (+ / Strg+T, gleicher Ordner), schließen (× / Mittelklick / Strg+W, letzter Tab bleibt), wechseln (Klick / Strg+Tab / Strg+Umschalt+Tab)
  - [x] Split-View (Button „Zwei Ansichten“ / Strg+\), F6 wechselt Pane, Klick aktiviert, Tastaturfokus folgt aktiver Pane; letzter Tab rechts schließen = Split aus
  - [x] Verlauf je Tab: ← → Buttons, Alt+←/→, Maus-Seitentasten; „Zurück zum Elternordner“ markiert Herkunftsordner
  - [x] Seitenleiste: Schnellzugriff (Known Folders/XDG), Laufwerke mit Name, Füllbalken (rot > 90 %), „x GB frei von y GB“, Refresh alle 15 s im Hintergrund
  - [x] Sitzung: Panes, Tabs, aktive Tabs/Pane, Split in session.ini; Restore beim Start ohne Pfad-Argument; nur sichtbare Tabs laden sofort (lazy); fehlende Ordner → nächster existierender Elternordner
  - [x] CI grün (Run #19)
- [ ] M3-Rest/Politur (später): Tabs per Drag umsortieren/abreißen, Pane-Trenner ziehbar, Sidebar: Netzwerk/Papierkorb/Favoriten anheften, Mittelklick in Sidebar = neuer Tab, Laufwerks-Änderungen per WM_DEVICECHANGE statt Polling
- [x] M4 Dateioperationen – **abgeschlossen 2026-11-03, CI-Run #26 grün**
  - [x] Crate `explorer-ops`: Jobs Kopieren/Verschieben/Papierkorb/Endgültig löschen im Hintergrund (Fortschritt, Pause, Abbruch), Konflikte (Ersetzen/Zusammenführen, Überspringen, Beide behalten, Abbrechen, „für alle“)
  - [x] Datensicherheit: Temp-Datei + Rename, Quelle bei Verschieben über Laufwerke erst nach vollständiger + geflushter Kopie löschen, „Ersetzen“ legt Alt-Datei in den Papierkorb, Abbruch hinterlässt keine Teil-Dateien, Fehler pro Element (Job läuft weiter)
  - [x] Undo/Redo (Strg+Z/Strg+Y, max. 50): Kopie → Papierkorb, Verschieben/Umbenennen → zurück, Papierkorb → wiederherstellen (Snapshot vor dem Undo), endgültig gelöscht = nicht rückgängig
  - [x] Zwischenablage Strg+C/X/V (+ Strg/Umschalt+Einfg), Windows-kompatibel (CF_HDROP + Preferred DropEffect, Austausch mit Explorer), Ausschneiden blendet Einträge halb aus, Esc hebt auf, Cut wird nur einmal eingefügt
  - [x] Entf → Papierkorb, Umschalt+Entf → Bestätigungsdialog → endgültig
  - [x] F2 Inline-Umbenennen (Name ohne Endung markiert), Esc bricht ab, ungültig/vorhanden → Fehlermeldung + Editor bleibt offen; Strg+Umschalt+N „Neuer Ordner“ + sofort Umbenennen
  - [x] Befehlsleiste (Neuer Ordner, Ausschneiden, Kopieren, Einfügen, Umbenennen, Löschen, ↶, ↷)
  - [x] Operations-Panel (ab 300 ms Laufzeit): Titel, Balken, „x von y · MB/s · noch n s“, Pause/Fortsetzen, Abbrechen
  - [x] Drag & Drop in der App: auf Ordnerzeile = hinein, auf Liste = in angezeigten Ordner, Standard Verschieben, Strg = Kopieren; nie in sich selbst/Unterordner; Mehrfachauswahl bleibt beim Ziehen erhalten
  - [x] Tests: ops 18 Integrationstests (+ Disk-Full per tmpfs in CI, gesperrte Datei auf Windows), E2E 38–52 (40 Checks), CI-Clipboard-Interop mit Windows PowerShell
  - [x] CI grün (Run #26: E2E 100/100, Windows-ops 19/19 inkl. Papierkorb + gesperrte Datei, Clipboard-Interop mit Windows, Disk-Full)
- [ ] M4-Rest/Politur (später): DnD mit anderen Apps (OLE, M5), Standard-Aktion je Laufwerk (gleiches Laufwerk = Verschieben, anderes = Kopieren, wie Explorer), Job-Warteschlange (nacheinander statt parallel, pro Laufwerk), Konfliktdialog mit Vorschaubildern, „Kopieren nach…“/„Verschieben nach…“, Fortschritt in der Taskleiste (ITaskbarList3), Papierkorb-Ansicht, Junctions/Hardlinks beim Kopieren sauber behandeln, Alternate Data Streams/ACLs (CopyFileExW kopiert ADS bereits)
- [~] M5 Shell-Integration (Boss-Freigabe 2026-11-04) – in Teilschritten, jeder einzeln CI-geprüft:
  - [x] M5.0 Exe-Größe: `opt-level="s"` (Linux 22,9 → 19,6 MB; Sortieren 100k +6–10 ms, Rest im Rauschen), CI-Gate < 20 MB
  - [~] M5.1 Echte Icons (lokal grün, CI ausstehend): Shell-Icons (`SHGetFileInfoW`/`IShellItemImageFactory`), Cache pro Endung + individuell für .exe/.lnk/.ico/.url, im Hintergrund, Overlays (Verknüpfung) – Linux: Fallback wie bisher
  - [~] M5.2 Shell-Typnamen (lokal grün, CI ausstehend; Typ-Sortierung nach angezeigtem Namen) („Textdokument“, „JPEG-Bild“) per `SHGetFileInfoW(SHGFI_TYPENAME|USEFILEATTRIBUTES)`, gecacht pro Endung
  - [~] M5.3 Kontextmenü (lokal grün, CI ausstehend; Linux: nur UI-Pfad, eigenes Fallback-Menü später): `IContextMenu` (inkl. Erweiterungen Dritter, „Senden an“, „Öffnen mit“ via IContextMenu2/3), Rechtsklick auf Auswahl / leere Fläche (Ordner-Hintergrund), eigene Einträge davor (Kopieren/Einfügen/… über unsere Engine)
  - [~] M5.4 Eigenschaften (`SHObjectProperties`/`SHMultiFileProperties`, Alt+Enter; „Öffnen mit“ kommt über das Shell-Menü) – CI ausstehend
  - [ ] M5.5 OLE-Drag&Drop mit anderen Apps: Ziehen aus der App (`DoDragDrop` + `SHCreateDataObject`), Ablegen in die App (`RegisterDragDrop`, Winit-Handler ersetzen), Standard-Aktion je Laufwerk wie Explorer
  - [ ] M5.6 Virtuelle Orte: Papierkorb-Ansicht (Wiederherstellen/Leeren), „Dieser PC“, Netzwerk/UNC, Bibliotheken – über Shell-Namespace (`IShellFolder`/`IEnumIDList`), schneller Pfad bleibt für normale Ordner
  - [ ] M5.7 Thumbnails (für spätere Symbol-Ansicht vorbereitet, Cache)
- [ ] M6 Ordneroptionen-Parität, Themes, Command Palette, Inspector
- [ ] **R-001 Ein-Knopf-Reset** für alle Integrationen (App-Knopf + eigenständiges Recovery-Tool + Journal + CI-Snapshot-Test) – Voraussetzung, bevor irgendeine Integration ausgeliefert wird
- [ ] M7 Windows-Integration Stufe 1–3 (siehe `WINDOWS-INTEGRATION.md`) + Verifikationspunkte auf echtem Windows
- [ ] M8 Installer, Recovery-Tool, Härtung, Beta

## Technische Schulden / Follow-ups
- [ ] **Exe-Größe 19,7 MB (Run #26, Budget < 20 MB fast erreicht!)** – vor M5 prüfen: `opt-level="z"`/`lto`/`strip`, unnötige Slint-Features (Wayland?), Skia vs. femtovg; vorher 17,2 MB (Budget < 20 MB ok, Ideal < 10 MB verfehlt) → in M2 prüfen: Skia vs. femtovg/Software-Renderer, `opt-level="z"`-Vergleich
- [x] Renderer-Entscheidung: Option 2 (GPU-Probe) + Einstellung (D-009)
- [ ] `NtQueryDirectoryFile` als Alternative zum FindFirstFileExW messen (nur falls Windows-Bench knapp wird)
- [ ] GitHub-Actions auf Node-24-Versionen heben (checkout@v4 → v5 etc., Deprecation-Warnung in CI)

- [ ] Echte Hardware-GPU-Probe nur über Boss-Test verifizierbar (CI hat keine GPU) → Artefakt auf eigenem PC testen: Einstellungen-Panel zeigt „GPU-Beschleunigung (<GPU-Name>)“
- [ ] Linux: keine GPU-Probe (Plattform-Default) – nur Dev-Plattform, ok

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
