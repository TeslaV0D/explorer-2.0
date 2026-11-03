# Plan – Phase 1: Basis-Features

Ziel: ein schneller, zuverlässiger Dateimanager, der im Alltag den Explorer ersetzen kann – **bevor** Suche und Shortcuts kommen.

## 1. Leitprinzipien
- **UI-Thread macht nie I/O.** Alles asynchron, Ergebnisse streamen (erste Zeilen sofort, Rest nachladen).
- **Virtualisierte Listen** (nur sichtbare Zeilen rendern/Icons laden).
- **Shell-Kompatibilität über echte Windows-APIs** (`IShellItem`, `IShellFolder`, `IFileOperation`, `IContextMenu`) statt Eigenbau – so funktionieren Papierkorb, Libraries, Netzwerk, OneDrive-Platzhalter und Drittanbieter-Erweiterungen „geschenkt“.
- **Schneller Pfad für normale Ordner:** direkt `NtQueryDirectoryFile`/`FindFirstFileExW` (`FIND_FIRST_EX_LARGE_FETCH`) statt Shell-Enumeration → Geschwindigkeit; Shell-Pfad nur für virtuelle Ordner.
- **Langpfade (`\\?\`, >260 Zeichen)**, Unicode, Per-Monitor-DPI-v2 von Tag 1 (Manifest).
- Kern als eigene Crate (plattformneutral testbar), Windows-Spezifisches hinter Traits.

## 2. Architektur (Vorschlag)
```
crates/
  core/        Modell: Pfade, Einträge, Sortierung/Filter, Befehlssystem, Settings (plattformneutral, Unit-Tests unter Linux)
  fs/          Verzeichnis-Enumeration, Watcher (ReadDirectoryChangesW), Datei-Ops-Queue
  winshell/    Windows-Adapter: IShellItem, Icons/Thumbnails, Kontextmenü, Properties, Papierkorb, Drag&Drop, Clipboard
  ui/          Oberfläche (Framework: D-001 offen)
  app/         Binary, Single-Instance, Argument-Parsing (Pfad, /select,, --new-window)
  installer/   Registry-Integration + Recovery (siehe WINDOWS-INTEGRATION.md)
```
Datenfluss: `app` → Command (z. B. `Navigate(path)`) → `core` State → `fs` Task (async, cancelbar) → Ergebnis-Stream → `ui` Diff-Update.

## 3. UI-Framework – offene Entscheidung D-001
| Option | Pro | Contra |
|---|---|---|
| **Rust + Slint** (Empfehlung) | Native Binary klein, schneller Start, GPU/Software-Renderer, deklarative UI | Eigene Controls für Fancy-Listen; Accessibility noch jünger |
| Rust + egui | Sehr schnell zu bauen, Immediate Mode passt zu File-Pilot-Stil | A11y/IME/natives Look&Feel schwächer, Dauer-Redraw-Disziplin nötig |
| C# + WinUI 3 | Natives Fluent-Look, A11y, Win11-Kontextmenü leichter | Größer, Start langsamer, Virtualisierung bei 100k+ Items knifflig, Runtime-Abhängigkeit |
| Eigener Renderer (wgpu/Direct2D) | Maximale Kontrolle wie File Pilot | Mit Abstand höchster Aufwand (Text, IME, A11y selbst) |

## 4. Feature-Umfang Phase 1 (Priorität P0 = muss, P1 = soll, P2 = kann)

### 4.1 Navigation & Layout
- P0 Adressleiste (Breadcrumb **und** editierbarer Pfad mit Autovervollständigung), Zurück/Vor/Hoch, Verlauf
- P0 Tabs (Drag-out, Wiederherstellung beim Start), **Split-View** (2 Panels)
- P0 Seitenleiste: Schnellzugriff/Favoriten, Dieser PC, Laufwerke, Netzwerk, Papierkorb
- P1 GoTo (Pfad-Sprung à la File Pilot), Command Palette (Ctrl+Shift+P), Pfad-Alias
- P1 Inspector-Panel (Vorschau Text/Bild/Ordnergröße/Eigenschaften)

### 4.2 Dateiliste
- P0 Ansichten: Details, Liste, Symbole (S/M/L/XL), Kacheln; Spalten wählbar/sortierbar/breitenmerkend
- P0 Sortieren, Gruppieren, Filtern (Schnellfilter im Ordner), natürliche Sortierung („file2“ < „file10“)
- P0 Multi-Selektion (Maus + Tastatur, Lasso, Ctrl/Shift, Typeahead „Tippen springt“)
- P0 Echte Icons + Overlay-Icons (Verknüpfung, Cloud-Status, Shared), Thumbnails async + Cache
- P1 Flatten-View (alle Unterordner flach), Ordnergrößen on demand

### 4.3 Dateioperationen
- P0 Kopieren/Verschieben/Löschen (Papierkorb + endgültig)/Umbenennen/Neu (Ordner, Datei, Verknüpfung)
- P0 Via `IFileOperation`: Papierkorb, UAC-Elevation, Konflikt-Dialog, Shell-konform
- P0 Eigene **Operations-Queue** mit Fortschritt, Pause/Abbruch, Geschwindigkeit, Fehler-Liste, **Undo/Redo**
- P0 Zwischenablage (Ctrl+C/X/V kompatibel mit Explorer, `CF_HDROP` + Preferred DropEffect)
- P0 Drag&Drop intern + von/zu anderen Apps (Move/Copy/Link, Modifier-Tasten)
- P1 Batch-Rename (Muster, Nummerierung, Datum, Regex), Pfad kopieren (diverse Formate), Archive (zip lesen/schreiben)
- P1 Hashes, Verknüpfungen/Symlinks/Junctions anlegen, Attribute ändern

### 4.4 Kontextmenü & Shell
- P0 Kontextmenü mit **Shell-Erweiterungen Dritter** (Klassisches `IContextMenu` hosten; Win11-Einträge via `IExplorerCommand`/Registry-Verben)
- P0 Eigenschaften-Dialog (`SHObjectProperties`), „Öffnen mit“, Standardverben (`ShellExecuteEx`)
- P1 Eigenes durchsuchbares & pinbares Kontextmenü (File-Pilot-Idee)

### 4.5 Einstellungen (Parität „Ordneroptionen“)
- P0 Versteckte/System-Dateien, Erweiterungen anzeigen, geschützte Systemdateien, Vollpfad in Titel
- P0 Einfach-/Doppelklick, Checkboxen, Detailbereich/Vorschau, Start-Ordner, Neues-Fenster-Verhalten
- P0 Lesen/Schreiben derselben Registry-Werte (`HKCU\...\Explorer\Advanced`), damit Einstellungen konsistent bleiben
- P1 Eigene Settings (Theme hell/dunkel/System, Schrift/Dichte, Animationen aus, Hotkey-Remapping, Layouts speichern)

### 4.6 Robustheit
- P0 Datei-Watcher mit Debounce, Auto-Refresh, Wiederherstellung bei Löschung des aktuellen Ordners
- P0 Netzwerk-/getrennte Laufwerke: Timeouts, nie UI-Freeze
- P0 Absturzsicherheit: Session-Restore, Crash-Log lokal (keine Telemetrie)
- P0 Single-Instance + schnelle Weiterleitung neuer Aufrufe (Start < 150 ms „warm“)

## 5. Performance-Budgets (messbar, in CI per Benchmark prüfen)
| Metrik | Ziel |
|---|---|
| Kalter Start bis erstes Fenster | < 300 ms; warm (Hintergrundprozess) < 80 ms |
| Ordner mit 100 000 Dateien: erste Zeilen | < 100 ms, vollständig < 1 s |
| Scrollen | 60 fps stabil, kein Icon-Flackern |
| RAM idle | < 80 MB |
| Binary | < 20 MB (Ideal: < 10 MB) |

## 6. Meilensteine
| M | Inhalt | Ergebnis |
|---|---|---|
| M0 | Repo-Setup, ADR D-001, Cargo-Workspace, CI (Windows + Linux), Manifest (DPI, longPath) | Leeres Fenster baut in CI |
| M1 | `fs`-Kern: Enumeration, Sortierung, Watcher, Benchmarks | CLI-Dump von 100k-Ordner < Budget |
| M2 | UI-Shell: Fenster, virtualisierte Liste, Details-Ansicht, Adressleiste | Ordner browsen |
| M3 | Tabs, Split, Sidebar, Verlauf, Tastaturnavigation, Session-Restore | Alltagsnavigation |
| M4 | Dateioperationen + Queue + Undo + Clipboard + Drag&Drop | Daten sicher bewegen |
| M5 | Shell-Integration: Icons/Thumbs, Kontextmenü, Properties, Papierkorb, virtuelle Ordner, Netzwerk | Parität im Alltag |
| M6 | Einstellungen/Ordneroptionen, Themes, Command Palette, Inspector | Feature-komplett (P0/P1) |
| M7 | Windows-Integration Stufe 1–3 (Default-Handler, Win+E, `/select,`) | Ersetzt Explorer im Alltag |
| M8 | Installer, Recovery-Modus, Härtung, Dogfooding-Beta | Release 0.1 |

## 7. Risiken
- **Kein Windows in der Sandbox:** alles Windows-spezifische nur über CI testbar → früh VM/Windows-Runner-Pipeline, Boss testet Builds manuell.
- **Explorer ist tief im System** (Shell, Win+E, Datei-Dialoge): kompletter Ersatz ist nur in Stufen möglich – Details/Grenzen in `WINDOWS-INTEGRATION.md`.
- **Drittanbieter-Shell-Erweiterungen können crashen** → in separatem Prozess/Thread-Isolation hosten (Timeout, Watchdog).
- Virtuelle Ordner (Libraries, Control Panel) nur über Shell-Namespace sauber erreichbar.
