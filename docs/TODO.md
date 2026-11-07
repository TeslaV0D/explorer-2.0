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
- [x] M5 Shell-Integration (Boss-Freigabe 2026-11-04) – **komplett: M5.0–M5.5 (Run #37), M5.6 (Run #39–#44), M5.7 (Run #48)** – in Teilschritten, jeder einzeln CI-geprüft:
  - [x] M5.0 Exe-Größe: `opt-level="s"` (**Windows 19,7 → 16,5 MB**, Linux 22,9 → 19,6 MB; Sortieren 100k +6–10 ms, Rest im Rauschen), CI-Gate < 20 MB
  - [x] M5.1 Echte Icons: Shell-Icons (`SHGetFileInfoW`/`IShellItemImageFactory`), Cache pro Endung + individuell für .exe/.lnk/.ico/.url, im Hintergrund, Overlays (Verknüpfung) – Linux: Fallback wie bisher
  - [x] M5.2 Shell-Typnamen (Typ-Sortierung nach angezeigtem Namen) („Textdokument“, „JPEG-Bild“) per `SHGetFileInfoW(SHGFI_TYPENAME|USEFILEATTRIBUTES)`, gecacht pro Endung
  - [x] M5.3 Kontextmenü (Linux: eigenes Fallback-Menü später): `IContextMenu` (inkl. Erweiterungen Dritter, „Senden an“, „Öffnen mit“ via IContextMenu2/3), Rechtsklick auf Auswahl / leere Fläche (Ordner-Hintergrund), eigene Einträge davor (Kopieren/Einfügen/… über unsere Engine)
  - [x] M5.4 Eigenschaften (`SHObjectProperties`/`SHMultiFileProperties`, Alt+Enter; „Öffnen mit“ kommt über das Shell-Menü)
  - [x] M5.5 OLE-Drag&Drop mit anderen Apps (Windows-E2E zieht per echter Maus): Ziehen aus der App (`DoDragDrop` + `SHCreateDataObject`), Ablegen in die App (`RegisterDragDrop`, Winit-Handler ersetzen), Standard-Aktion je Laufwerk wie Explorer
  - [x] M5.6 Virtuelle Orte (schneller Pfad bleibt für normale Ordner) – CI-Run #39–#44:
    - [x] M5.6a Papierkorb-Ansicht: Liste (Gelöscht am, Ursprünglicher Ort, Größe), Wiederherstellen, endgültig löschen, leeren, Ablegen auf Papierkorb = löschen, Seitenleiste, Adress-Aliase (CI-Run #39)
    - [x] M5.6b „Dieser PC“: Laufwerke mit Speicherplatz, Klick auf Überschrift in der Seitenleiste, „Hoch“ vom Laufwerk → Dieser PC (CI-Run #39)
    - [x] M5.6c Netzwerk: `\\server` → Freigaben (`NetShareEnum`, ohne Admin-Freigaben `C$`/`IPC$`), Kommentar in „Typ“, Anmeldedialog (`WNetAddConnection3W`) bei Zugriff verweigert – einmal pro Ordner, „Hoch“ von Freigabe → Server, Breadcrumb `\\nas › film` (CI-Run #41)
    - [x] M5.6d1 Adresszeile wie Explorer: `%VAR%`, `shell:<Name>` (32 Known Folders, z. B. `shell:Downloads\\x`, `shell:Common Startup`), `~` (Unix); echte Shell-Icons in Seitenleiste (Ordner, Laufwerke, Papierkorb) und „Dieser PC“ (CI-Run #43)
    - [x] M5.6d2 Bibliotheken (`IShellLibrary`): Ansicht „Bibliotheken“, Bibliothek mit einem Ordner öffnet ihn, mit mehreren zeigt ihre Ordner; `shell:Libraries`/`shell:DocumentsLibrary` … (CI-Run #44)
    - [ ] Später: zusammengeführte Bibliotheks-Ansicht (Inhalte aller Ordner in einer Liste, wie Explorer) – braucht Dateioperationen mit Pfad pro Zeile
    - [ ] Später: Undo für „Wiederherstellen“, Shell-Kontextmenü im Papierkorb
  - [x] M5.7 Kachel-Ansicht + Vorschaubilder (Boss: Weg 1) – CI-Run #48
    - [x] M5.7a Vorschaubilder: `IShellItemImageFactory` (Windows-Thumbnail-Cache, sonst großes Icon), 2 Worker, nur sichtbare Kacheln, neueste zuerst, 96-MB-Speichergrenze (LRU), Schlüssel inkl. Änderungszeit
    - [x] M5.7b Kachel-Ansicht: Mittel/Groß/Extra groß (48/96/256 px), Strg+Shift+1/2/3/6, Strg+Mausrad (gedrosselt), Knöpfe „Details/Symbole“ in der Statusleiste, Raster-Navigation (←→↑↓, Bild↑↓), Klick/Strg/Shift, Doppelklick, Kontextmenü, F2 über der Beschriftung, Ziehen & Ablegen (auch OLE, Trefferprüfung im Raster), Einstellung `view` gespeichert
    - [ ] Später: Spaltenköpfe in der Kachel-Ansicht durch Sortier-Menü ersetzen; Rahmen-Auswahl (Lasso) für beide Ansichten; Ansicht pro Ordner merken
- [x] M6 Ordneroptionen-Parität, Themes, Command Palette, Inspector (Boss: Reihenfolge M6.1 → M6.4) – komplett (CI-Run #58)
  - [x] M6.1 Ordneroptionen wie im Explorer (CI-Run #49–#51)
    - [x] M6.1a Einstellungs-Panel „Ordneroptionen“: Dateinamenerweiterungen anzeigen (aus = Endung bekannter Typen ausblenden, Windows: in HKCR registriert; F2 bearbeitet dann nur den Stamm, Endung bleibt), geschützte Systemdateien anzeigen, Öffnen durch einfachen Klick (Markieren durch Zeigen, Doppelklick-Schutz) – CI-Run #49
    - [x] M6.1b Sortier-Menü „Sortieren ▾“ in der Statusleiste (Spalte behält Richtung, Richtung getrennt), Kachel-Ansicht ohne Spaltenköpfe, Rahmen-Auswahl auf freier Fläche in beiden Ansichten (Strg = hinzufügen) – CI-Run #51
    - [ ] Später: Rahmen-Auswahl mit Auto-Scroll am Rand; Rahmen auch aus dem freien Teil einer Detail-Zeile starten (Explorer)
    - [x] M6.1c Ansicht pro Ordner merken (`views.ini`, max. 5000 Ordner, älteste fallen raus; Windows ohne Groß/Klein) + „Aktuelle Ansicht für alle Ordner“; Ansicht pro Tab → zwei Panes können verschieden aussehen – CI-Run #51
  - [x] M6.2 Inspector (Alt+P / „Details“): Vorschau (Shell-Bild 256 px, Text erste 40 Zeilen), Name/Typ/Größe/Geändert/Erstellt/Attribute/Pfad (markier- und kopierbar), Bildmaße aus dem Dateikopf (PNG/JPEG/GIF/BMP/WebP), Mehrfachauswahl mit Summen, Ordnergröße im Hintergrund (abbrechbar, keine Links, max. 500.000 Einträge, nie ganze Laufwerke; offener Ordner nur direkter Inhalt) – CI-Run #52
    - [ ] Später: Inspector-Breite verstellbar; Metadaten aus Shell-Eigenschaften (Autor, Länge von Videos/Audio, EXIF)
  - [x] M6.3 Befehlspalette (Strg+Shift+P): zentrales Befehlsverzeichnis (`core::commands`, 45 Befehle mit Kürzel + Bedingung), unscharfe Suche (Umlaute/Groß-Klein egal, Wortanfänge zählen), nicht verfügbare Befehle grau, zuletzt benutzte oben (settings.ini), „>“ = Orte (Schnellzugriff, Laufwerke, virtuelle Orte, zuletzt besuchte Ordner) – CI-Run #55
    - [ ] Später: frei belegbare Tastenkürzel auf Basis des Befehlsverzeichnisses
    - [x] Erststart-Gate: Warnung ab 1 s, rot ab 2 s (Boss: Variante 1, D-018)
  - [x] M6.4 Farbschemata: Wie Windows (live) / Hell / Dunkel, Windows-Akzentfarbe (Auswahl-Familie im Farbton des Akzents, Rahmen in Windows-Schattierung), alle Farben zentral + kontrastgetestet, Std-Widgets + Titelleiste folgen – CI-Run #58
    - [ ] Später: Windows-Kontrastdesigns (High Contrast) respektieren; Akzent-Häkchen auch für Std-Widgets (Slint färbt sie selbst nach `DwmGetColorizationColor`)
- [x] **R-001 Ein-Knopf-Reset** für alle Integrationen (App-Knopf + eigenständiges Recovery-Tool + Journal + CI-Snapshot-Test) – Voraussetzung, bevor irgendeine Integration ausgeliefert wird
  - [x] R-001a Crate `explorer-integrate`: Write-Ahead-Journal (atomar, Spiegel, Prüfsumme), `apply`/`reset`/`sweep`/`status`, rohe Registry-Werte, Dateien mit Sicherung, Umgebung + WM_SETTINGCHANGE, fremd geänderte Werte bleiben, Fehler bleiben im Journal; 13 Unit-Tests (Fake-Registry) + Test gegen echte Registry – CI-Run #59
  - [x] R-001b `explorer2-recover.exe` (Konsole, `--status`/`--yes`) + CI-Pflichttest `scripts/r001-snapshot.ps1` (reg.exe-Export vorher = nachher: aus Journal, über Spiegel, ganz ohne Journal) – CI-Run #59
  - [x] R-001c Einstellungen → „Windows-Integration“ (Status + „Alle Windows-Integrationen zurücksetzen“ mit Nachfrage) + Palette; Linux-E2E [71]–[71g] (188/188), Windows-E2E W41–W41h – CI-Run #61
  - [x] Startmenü-Eintrag „Windows-Standard wiederherstellen“ + Deinstallation ruft Reset (M8.1, Run #69)
  - [ ] Später: `.reg`/PowerShell-Fallback-Skript, Selbstheilung beim explorer.exe-Proxy, HKLM-Journal-Kopie in `%ProgramData%`
- [x] M7 Windows-Integration Stufe 1–2 (Stufe 3 verschoben, D-022) (siehe `WINDOWS-INTEGRATION.md`) + Verifikationspunkte auf echtem Windows
  - [x] M7.1 Ordner öffnen (eigenes Standard-Verb `explorer2open` für Directory/Drive, `Folder` bleibt bei Windows) + „In Explorer 2.0 öffnen“ im Kontextmenü (Ordner, Laufwerk, Ordner-Hintergrund); einzeln schaltbar (Einstellungen + Palette), Aus = exakt vorheriger Zustand; Exe verschoben → beim Start neu verknüpft; Notfall-Werkzeug-Kopie in `%LOCALAPPDATA%\Explorer2`; `"C:\"`-Quoting repariert; Windows-E2E W42 (echtes ShellExecute), R-001-Test [R5] – CI-Run #64
  - [x] M7.2 Win+E: `HKCU\…\CLSID\{52205FD8-…}\shell\opennewwindow\command` = Exe (ohne Ordner = letzte Sitzung) + leerer `DelegateExecute`; Schalter + Palette; Windows-E2E W43 (echtes Win+E per `keybd_event`, vorher Prüfung, ob die Runner-Shell Win+E überhaupt verarbeitet) – CI-Run #67
  - [ ] M7.3 `explorer.exe`-Aufrufe (IFEO-Proxy, HKLM/Admin, hohes Risiko) – **verschoben** (Boss, D-022), erst nach Praxistest von M7.1/7.2
  - [ ] Auf echtem Windows 11 prüfen (Boss): Doppelklick auf Ordner/Laufwerk auf dem Desktop, „Ordner öffnen“ aus Browser-Downloads, Kontextmenü (Win11: unter „Weitere Optionen“)
- [~] M8 Installer, Recovery-Tool, Härtung, Beta (Boss: eigener Installer in Rust, D-023)
  - [x] M8.1 Installer-Kern: `explorer2.exe --install` (pro Benutzer, ohne Admin), Startmenü (App + „Windows-Standard wiederherstellen“), Apps & Features, Integrationen auf installierte Exe umbiegen, Update über laufende App, `explorer2-recover --uninstall [--quiet]` (Reset + Programmordner per Temp-Kopie, Schutz vor fremden Ordnern); CI `scripts/m8-install.ps1` (echte Standardpfade, Start über Startmenü, Deinstallation über `QuietUninstallString` → alles wie vorher) – CI-Run #69
  - [x] M8.2 Einstellungen → „Installation“ (portabel/installiert, Installieren/Aktualisieren mit Nachfrage, danach Neustart aus dem Installationsordner) + Palette; Linux-E2E [72] – CI-Run #69
  - [x] M8.3 Härtung (Boss, 2026-11-05) – CI-Run #70–#73
    - [x] M8.3a Absturzbericht (Panic jedes Threads + Windows-Ausnahmen → `crash/crash-<utc>-<pid>.txt` mit Ursache, Ort, letzten 200 Protokollzeilen) + Hinweis beim nächsten Start („Bericht anzeigen“); Start-Wächter: 2 abgebrochene Starts in Folge → sicherer Start (Software-Darstellung, keine Sitzung, keine GPU-Prüfung), nach 5 s stabil = zurück auf normal; GPU-Prüfung, die den Prozess mitreißt → GPU dauerhaft „ungeeignet“; SIGTERM zählt nicht als Absturz; Linux-E2E [73]–[73j], Windows `scripts/m8-crash.ps1` [C1]–[C8] – CI-Run #70
    - [x] M8.3b Eine Instanz: zweiter Start (Ordner-Doppelklick, Win+E, Startmenü) öffnet einen Tab im laufenden Fenster, minimiertes Fenster kommt nach vorn; Named Pipe (Windows, nur lokal, pro Sitzung) / Unix-Socket 0600; Antwort erst, wenn der Tab offen ist – hängende Instanz → nach 4 s eigenes Fenster; `--new-instance` + Schalter „Ordner im offenen Fenster öffnen“ (Einstellungen/Palette); Installer-Neustart mit `--new-instance`; Linux-E2E [74]–[74i], Windows `scripts/m8-instance.ps1` [N1]–[N9] – CI-Run #71
    - [x] M8.3c Zufallsdaten-Tests für alle Parser/Text-Helfer (`core/tests/fuzz.rs`, `integrate/tests/fuzz.rs`: kein Panic, Speichern+Laden = gleich, jedes abgeschnittene/veränderte Journal wird erkannt; `FUZZ_ROUNDS`/`FUZZ_SEED`) – 3 Funde gefixt; Dauertest Linux `scripts/e2e-stress.sh` [75]–[75e] (RSS/Dateien/Threads) + Windows `scripts/m8-stress.ps1` [S1]–[S6] (Private Bytes/Handles/Threads/GDI/USER, zwei Messphasen, Bild-Cache herausgerechnet) – CI-Run #72/#73
    - [ ] Optimierung (später): Kachel-Bild-Cache hält für Dateien **ohne** eigene Vorschau (nur Shell-Icon) je Datei ein Bild → pro Endung teilen (Windows-Dauertest: 50 MiB Cache bei 2400 leeren Dateien; im Budget, aber unnötig)
  - [ ] Später: Code-Signatur (SmartScreen), automatische Updates, `.reg`/PowerShell-Fallback, Verknüpfung auf dem Desktop (optional)

## Technische Schulden / Follow-ups
- [x] Exe-Größe: 19,7 → **16,5 MB** (Run #29, `opt-level="s"`, CI-Gate < 20 MB). Weitere Ideen bei Bedarf: – vor M5 prüfen: `opt-level="z"`/`lto`/`strip`, unnötige Slint-Features (Wayland?), Skia vs. femtovg; vorher 17,2 MB (Budget < 20 MB ok, Ideal < 10 MB verfehlt) → in M2 prüfen: Skia vs. femtovg/Software-Renderer, `opt-level="z"`-Vergleich
- [x] Renderer-Entscheidung: Option 2 (GPU-Probe) + Einstellung (D-009)
- [ ] `NtQueryDirectoryFile` als Alternative zum FindFirstFileExW messen (nur falls Windows-Bench knapp wird)
- [ ] GitHub-Actions auf Node-24-Versionen heben (checkout@v4 → v5 etc., Deprecation-Warnung in CI)

- [ ] Echte Hardware-GPU-Probe nur über Boss-Test verifizierbar (CI hat keine GPU) → Artefakt auf eigenem PC testen: Einstellungen-Panel zeigt „GPU-Beschleunigung (<GPU-Name>)“
- [ ] Linux: keine GPU-Probe (Plattform-Default) – nur Dev-Plattform, ok

## Phase 2 – Systemweite Suche (Everything-Stil) – **läuft** (Boss 2026-11-05, Plan: `docs/PLAN-SEARCH.md`)
- [x] S1 ohne Admin (D-024) – komplett (CI-Run #93), nächster Schritt: Praxistest (`docs/PRAXISTEST-SUCHE.md`)
  - [x] S1.1 Index-Kern `explorer-index` (82 MB / 1 Mio, Namensabfragen ≤ 5,1 ms Windows-CI, ≤ 3,2 ms Linux-CI; CI-Run #78): kompakte Struktur (Namens-Arena, Elternkette), Einfügen/Löschen/Umbenennen, Abfragesprache (UND/ODER/NICHT, Phrase, `*?`, `ext:` `size:` `dm:` `file:` `folder:` `path:` `regex:`), parallele Suche, Sortierung; Benchmark 1 Mio Einträge < 10 ms, < 100 MB
  - [x] S1.2 Persistenz pro Laufwerk (Version, Volume-Kennung, Prüfsumme, atomar, jede Beschädigung erkannt = neu bauen, nie Absturz; 1 Mio: Datei 69 MB, Laden 116 ms, Speichern 179 ms; CI-Gate Laden < 1 s) + Ausschlusslisten (`Exclusions`: Pfade + Namensmuster, Standard-Ausschlüsse, Einstellung `index_exclude`) – CI-Run #80 (Laden Linux 88 ms, Windows 69 ms)
  - [x] S1.3 `explorer-search`: paralleler Aufbau, Abgleich beim Start über Ordner-Änderungszeiten (in Scheiben), Live-Updates (rekursiver Watcher, Stapel, Überlauf → Abgleich), Dienst pro Laufwerk (Generationen, Neu-Ordnen unter Lese-Sperre, nur exakte Stände speichern), Laufwerks-Kennung; CI-Beispiel `drive` (Messläufe `C:\` kalt/warm/Priorität, Gate `D:\`) – CI-Run #83: kalte VM-Platte 5k/s, warm 207k/s, Live 204 ms
    - [x] Netzlaufwerke/USB: optional + abschaltbar (S1.5c, Laufwerksauswahl)
    - [x] Linux: inotify-Grenze (`max_user_watches`) im Index-Status anzeigen (S1.5a: verständlicher Text)
    - [x] Watcher-Fehler/Überläufe zählen + letzten Grund im Status zeigen (S1.5a)
  - [x] S1.4 UI (CI-Run #90)
    - [x] S1.4a Dienst in der App: Start 3 s nach dem Fenster im Hintergrund, Laufwerke = feste lokale (Linux `$HOME`), Sperrdatei `index/lock` (zweites Fenster indexiert nicht), Speichern beim Beenden, Status + Ein/Aus + „Neu abgleichen“ in den Einstellungen (Panel scrollt auf kleinen Bildschirmen), Palette `search-rescan`/`toggle-index`, Overrides `EXPLORER2_INDEX=off`/`EXPLORER2_INDEX_ROOTS`/`EXPLORER2_INDEX_DELAY_MS`, E2E [76] – CI #90
    - [x] S1.4b Suchfeld pro Tab (Strg+F/Strg+E/F3, ab aktuellem Ordner, Platzhalter `::suche:`, Treffer mit vollen Pfaden = alle Dateifunktionen, Spalte „Ordner“ relativ, max. 20.000 angezeigt, ohne Index Ordner durchlaufen, Treffer folgen dem Index, Esc/leeres Feld = zurück ohne Spur im Verlauf), E2E [76i], [77]–[77k], Windows [W44] – CI #89/#90
    - [x] Lokale Zeitzone für Datumsanzeige **und** `dm:`-Abfragen (S1.5a, Sommerzeit pro Datum)
    - [x] S1.4c globale Schnellsuche (Strg+Umschalt+F + Palette „Alle Laufwerke durchsuchen“): Overlay im Stil der Palette, beste 50 (exakter Name → Namensanfang → kürzer), Enter öffnet, Strg/Umschalt+Enter zeigt markiert im Ordner, Fußzeile mit Index-Umfang, E2E [78], Windows [W45] – CI #89/#90
  - [x] S1.5 Feinschliff (CI-Run #91/#93)
    - [x] S1.5a Ortszeit (Anzeige + `dm:`), verpasste Watcher-Ereignisse im Status – CI #91
    - [x] S1.5b Treffer fett markiert, Sortieren per Spaltenkopf in Treffern (getestet), Suchverlauf (↑/↓ im Suchfeld, Liste in der Schnellsuche), E2E [79] – CI #91
    - [~] S1.5c Laufwerksauswahl in den Einstellungen (automatisch = feste lokale; Netz/USB/Linux-`/` zuschaltbar; DVD nie), Neustart des Dienstes, E2E [80] – CI #93
- [x] **Praxistest S1 auf echtem Windows (Boss, 2026-11-07): „es funktioniert“** – S2 vorerst nicht nötig
- [ ] S2 optional mit Admin: MFT/USN-Dienst – erst nach Praxistest S1 + R-001-Erweiterung für Dienste
- [ ] Später: Inhaltssuche, systemweiter Hotkey (Integration → R-001)

## Redesign (Boss 2026-11-07) – Plan `docs/PLAN-REDESIGN.md`, nur Design/UX, keine Logik
- [x] Entscheidungen 1–5 aus dem Plan (Boss 2026-11-07: „passt so“ = alle Empfehlungen)
- [x] R0 Absicherung (CI-Run #101): UI-Dump `EXPLORER2_UI_DUMP` (Item-Baum über `i-slint-core`), Test-Hilfen `scripts/ui-lib.sh` + `UiBox`/`UiClick`/`ListTop` in `e2e-windows.ps1`, alle Panel-/Leisten-Klicks per Name, Zeilen relativ zum Spaltenkopf, sprechende Namen für Symbol-Knöpfe, Galerie `scripts/gallery.sh` (22 Screens Linux) + Windows-Galerie (10 Screens) als CI-Artefakte (`ui-gallery-linux` 22, `ui-gallery-windows` 10)
- [x] R1 Design-System (CI-Run #104): Farbrollen `core::theme::Design` + Kontrast-Test (12 Akzente × hell/dunkel), Global `Ui`, 50 Lucide-Icons (`scripts/make-ui-icons.py`), Bausteine `ui/kit.slint`, UI-Kit `EXPLORER2_UI_KIT=1` (Galerie + E2E [81])
- [x] R2 Hauptlayout (CI-Run #110): globale Leiste aufgelöst (Aktionen in der Navigationszeile der aktiven Ansicht, ⋮-Menü, Statusleiste), Tooltips, Breakpoints, Dunkel als Standard
- [x] R3 Seitenleiste (CI-Run #115): Linien-Icons, Akzentmarke, dezente Laufwerksbalken
- [x] R4 Navigation (CI-Run #120): Icon-Knöpfe, Brotkrumen-Segmente, Suchfeld mit Lupe, 34 px
- [ ] R5 Tabs: IDE-Stil, Akzentstrich am aktuellen Tab der aktiven Ansicht, × bei Hover – lokal fertig, CI ausstehend
- [ ] E2E-Wackler beobachten: [49] erster Drag nach Gesamtlauf, [72c] installierte Kopie ohne X-Verbindung (CI #107)
- [ ] R2 Hauptlayout (globale Leiste auflösen, Statusleiste, Breakpoints)
- [ ] R3 Seitenleiste · R4 Navigation · R5 Tabs
- [ ] R6 Dateiliste · R7 Inspector · R8 Menüs + Dialoge
- [ ] R9 Zustände + Bewegung · R10 Konsistenz + QA (Galerie vorher/nachher, Gates, Abnahme)

## Phase 3 – Datei-Aktions-Shortcuts (Sweetch-Stil)  *(später, nur beschrieben)*
- [ ] Tray-Helper mit Kreismenü: Drag von Dateien auf Sektor oder Hotkey (Sweetch: Ctrl+Alt+Space) 
- [ ] Modifier-Gruppen: Shift = Konvertieren, Ctrl = Tools, Alt = Aktionen
- [ ] Aktionen: Konvertieren (Bild/Audio/Video/PDF/Doc via ffmpeg/pandoc – optional nachladbar), Archivieren, Kopieren nach…, Umbenennen, eigene Skripte
- [ ] Konfigurierbare Sektoren/Profile, lokale Verarbeitung (keine Cloud)
- [ ] Lizenz klären: Sweetch ist MIT → Ideen/Code-Nutzung mit Attribution möglich

## Phase 4 – Später
- [ ] Plugins/Skripting, Settings-Sync, Auto-Update, ARM64, Code-Signing
