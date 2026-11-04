# Zwischenstand

**Stand:** 2026-11-03 · **Phase:** 1 · M0 ✅ · M1 ✅ · M1.5 ✅ · M2 ✅ · M3 ✅ (CI-Run #19) · M4 ✅ (CI-Run #26) · **M5.0–M5.5 ✅ (CI-Run #37)** · **Branch:** `claude/relaxed-pasteur-ufhh1p`

## M5 – Shell-Integration (läuft, Freigabe Boss 2026-11-04)
### CI-Run #37 – alles grün ✅ (M5.0–M5.5)
- Windows-Tests inkl. echte Icons, Shell-Typnamen, Kontextmenü-Verben, Drop-Target-Datenpfad; Clipboard-Interop; Benchmark.
- UI-Matrix: Drop-Target registriert (1. Versuch), Icons + Typnamen geladen, Drag ohne gedrückte Taste wird übersprungen.
- **Windows-E2E mit echter Tastatur/Maus: 16/16** (Auswahl, F2 + Undo, Neuer Ordner, Strg+C → Windows-Clipboard, Strg+V, Entf → Papierkorb + Undo, echtes Shell-Menü, Eigenschaften-Dialog, echtes OLE-Drag per Maus).
- Linux-E2E 104/104. Exe Windows 16,5 MB.

### M5.0 Exe-Größe
- Analyse (Linux-Build als Näherung): 22,9 MB gesamt, davon Skia ≈ 8 MB (vorkompiliert, durch `opt-level` nicht beeinflussbar), Rest Rust (std, Slint-Core/Renderer, Winit, Text-Shaping harfrust/skrifa/parley, SVG usvg/resvg, Bilddecoder). Wayland/X11/zbus/sctk-adwaita (inkl. eingebetteter Cantarell-Schrift) sind **nur Linux**. Bildformate/SVG kommen aus Slint-Interna → nicht sauber abschaltbar.
- Maßnahme: `opt-level = "s"` → Linux **22,9 → 19,6 MB (−15 %)**. Bench 100k (2×9 Läufe): erste Zeilen und Gesamtladen im Rauschen, Sortieren 43 → ~50 ms. `"z"` spart kaum mehr, ist langsamer.
- E2E mit `"s"`: 100/100, First Paint 19 ms.
- CI-Gate: Windows-Exe ≥ 20 MB → Build rot. **Windows-Exe jetzt 16,47 MB** (Run #29, vorher 19,7 MB).

### M5.1 Icons / M5.2 Typnamen
- Neue Crate `explorer-shell`: Icon-Schlüssel (pro Endung bzw. pro Datei für exe/lnk/ico/url/…, Ordner mit R/S-Attribut), threadsichere `IconRegistry`, `SHGetFileInfoW` → HICON → RGBA (Alpha bzw. Maske), Typnamen per `SHGFI_TYPENAME|USEFILEATTRIBUTES` mit Cache.
- App: Zeile trägt nur eine `icon`-id, UI hält `[image]`-Modell; Worker-Thread lädt gebündelt nach, Einzel-Icons nur für sichtbare Zeilen (Viewport-Callback ±20 Zeilen). Typ-Spalte zeigt Shell-Namen, **Sortieren nach Typ nach angezeigtem Namen** (wie Explorer: .jpg/.jpeg zusammen).
- Linux: keine Shell → Fallback-Quadrate und eigene Typnamen wie bisher.
- Tests: shell 8 Unit (Schlüsselregeln, Registry unter Konkurrenz, Pixel-Konvertierung) + Windows-Test mit echten Icons (16/32 px, Ordner ≠ txt, eigene Exe), core +1 (Typ-Sortierung), app +3; CI-UI-Matrix prüft „icons loaded“ + Shell-Typnamen.

### M5.3 Kontextmenü / M5.4 Eigenschaften
- `explorer-shell::menu`: Shell-Kontextmenü über `IShellFolder::GetUIObjectOf` (Auswahl) bzw. `CreateViewObject` (Ordner-Hintergrund) → `IContextMenu`, Untermenüs (Senden an, Öffnen mit, Neu) über Window-Subclassing + `IContextMenu2/3::HandleMenuMsg(2)`. Ausführung per Position (`MAKEINTRESOURCE`).
- Eigene Verben statt Shell: Umbenennen, Löschen, Ausschneiden/Kopieren/Einfügen, Neuer Ordner → laufen über unsere Engine (Undo, Fortschritt, Konfliktdialog). Alles andere (Erweiterungen Dritter) führt die Shell aus, danach Refresh.
- Rechtsklick wie Explorer: nicht markierte Zeile wird markiert, Mehrfachauswahl bleibt; leere Fläche = Ordner-Hintergrund; Shift+F10/Menü-Taste an der fokussierten Zeile; Alt+Enter = Eigenschaften (eine Datei: `SHObjectProperties`, mehrere: `SHMultiFileProperties`).
- Windows-Test: Menü wird gebaut (ohne Anzeige) und muss die Verben open/delete/rename/properties enthalten; Hintergrund-Menü nicht leer. E2E Linux 53–53d (UI-Pfad).
- CI-Run #29: Tests + Kontextmenü-Test + Release-Build auf Windows grün (App kompiliert mit HWND-Code); Icons laden in der UI-Matrix (2 Icons nach 23 ms). Rot nur der `kinds`-Trace: erste 200 Zeilen im 100k-Ordner sind alles Ordner → Trace sucht jetzt alle Zeilen ab (max. 10 Namen).
- CI-Run #28: Shell-Icons und Typnamen laufen auf Windows (Runner englisch: „File folder“); nur ein App-Test erwartete fest den Linux-Fallback → plattformbewusst gemacht.

### M5.5 OLE-Drag&Drop
- Ablegen: eigenes `IDropTarget` (`#[implement]`) ersetzt per `RevokeDragDrop`/`RegisterDragDrop` den Winit-Handler; Pfade aus `CF_HDROP` (`DragQueryFileW`). Ziel per Hit-Test (Slint meldet Listen-Geometrie + Scroll je Pane) → Ordnerzeile = hinein, Datei/leer = angezeigter Ordner.
- Standard wie Explorer: gleiches Laufwerk = Verschieben, anderes = Kopieren, Strg = Kopieren, Umschalt = Verschieben, begrenzt auf das, was die Quelle erlaubt; nie in sich selbst. Ausführung über unsere Engine (Fortschritt, Konflikte, Undo).
- Ziehen: `SHDoDragDrop` mit Shell-Datenobjekt (`GetUIObjectOf`) = Explorer-kompatibel inkl. Drag-Bild; Start per Timer nach dem Slint-Callback (modale Schleife), danach Pointer-Release nachgereicht. Windows: auch In-App-DnD über OLE; Linux: Slint-DragArea wie bisher.
- Tests: Windows-Unit-Test schiebt ein echtes Shell-Datenobjekt durch unser `IDropTarget` (Pfade inkl. Umlaut, Strg → Kopieren); CI-UI-Matrix: Drop-Target registriert + Test-Hook `EXPLORER2_TEST_OLE_DRAG` (modale Drag-Schleife im laufenden Event-Loop, kein Absturz/Hänger); App-Unit-Test Hit-Test (Panes, Scroll, Sidebar/Header).

- **CI-Run #31 (rot)**: Drop-Test verglich `%TEMP%`-Kurznamen (`RUNNER~1`) roh mit den Langnamen der Shell → Test vergleicht kanonisch. Dabei echtes Risiko gefunden: Start über einen Kurznamen-Pfad hätte Drop-Vergleiche (gleicher Ordner/in sich selbst) ausgehebelt → **Navigation expandiert jetzt Kurznamen** (`explorer_fs::path::long_path`, `GetLongPathNameW`, Junctions bleiben) und Drop-Vergleiche sind unter Windows case-insensitiv.

- **Bug (CI #32/#33, UI-Matrix)**: OLE-Drop-Target wurde nie registriert („Invalid window handle“) – der Timer lief, bevor Slint das native Fenster erzeugt hatte → Ablegen aus anderen Apps wäre tot gewesen. Fix: Registrierung wiederholt bis HWND existiert (50 ms, max. 5 s), sonst Rückfall auf In-App-DnD.
- Neu: **Windows-E2E mit echter Tastatur/Maus** (`scripts/e2e-windows.ps1`, CI-Schritt im Release-Job, Screenshots bei Fehler).

### M5.6c Netzwerk (lokal grün, CI ausstehend)
- `\\server` in der Adresszeile → Freigaben des Servers als Ordnerliste (`NetShareEnum` Level 1); Admin-Freigaben (`C$`, `ADMIN$`, `IPC$`), Drucker und `…$` ausgeblendet wie im Explorer; Freigabe-Kommentar steht in „Typ“.
- Server will Anmeldung → Windows-Anmeldedialog (`WNetAddConnection3W`, Fenster als Owner, im Hintergrund-Thread), danach neu laden; gilt auch für Ordner in Freigaben (Zugriff verweigert / Anmeldung fehlgeschlagen) – höchstens einmal pro Pfad, keine Dialog-Schleifen.
- „Hoch“ von `\\nas\film` → `\\nas` (Freigabe vorausgewählt), vom Server → „Dieser PC“; Breadcrumb `\\nas › film › …`; im Server-Ordner kann nichts angelegt werden.
- Tests: Unit (Pfad-Parsing, Filter, Fehlercodes), Windows-Unit gegen `localhost`, Windows-E2E W18–W19c mit echter SMB-Freigabe (`New-SmbShare`), Linux-E2E [59] (klare Fehlermeldung).

### M5.6a/b Papierkorb-Ansicht + „Dieser PC“ ✅ (CI-Run #39, alles grün inkl. Windows-E2E W15–W17b)
- `core::places::VirtualPlace` (Platzhalter-Pfade `::papierkorb` / `::dieser-pc`, nie mit echten Pfaden verwechselbar) → Tabs, Verlauf, Sitzung, Breadcrumb funktionieren unverändert; Adress-Aliase: „Papierkorb“, `shell:RecycleBinFolder`, „Dieser PC“, `shell:MyComputerFolder`, Explorer-CLSIDs.
- `ops::trashview`: Liste mit echtem Dateinamen (Windows-Anzeigenamen ohne Endung → Endung aus `$R…`), gleiche Namen eindeutig („a.txt (2)“), Wiederherstellen verweigert Überschreiben, endgültig löschen, leeren.
- Sicherheit: in virtuellen Orten liefern `pane_dir`/`selected_paths`/`entry_at` nichts → Einfügen, Neuer Ordner, Drops, Umbenennen können nie auf einem Platzhalter-Pfad landen; nicht erlaubte Befehle melden „Hier nicht möglich“.
- Befehlsleiste im Papierkorb: Wiederherstellen / Endgültig löschen / Papierkorb leeren; Spalten „Gelöscht am“, „Ursprünglicher Ort“. Dateien auf den Papierkorb ziehen = löschen. Jedes Löschen/Undo aktualisiert offene Papierkorb-Tabs.
- „Dieser PC“: Laufwerke mit „x GB frei von y GB“, Öffnen → Laufwerkswurzel, „Hoch“ von einer Wurzel → Dieser PC.
- Tests: core +2, ops +3 (inkl. echter Papierkorb-Durchlauf), E2E Linux **116/116** (neu 55–58c), Windows-E2E erweitert (W15–W17b).

### Renderer-Vergleich Skia vs. FemtoVG (CI #35, Windows, D-015)
| | Skia (heute) | FemtoVG |
|---|---|---|
| Exe-Größe | 16,5 MB | **9,0 MB** |
| Bild (Mesa-GL) | Referenz | praktisch gleich; Schrift minimal kräftiger, Icons/Layout identisch |
| Kaltstart mit OpenGL (Mesa llvmpipe, best of 3) | **982 ms** | 1245 ms (+27 %) |
| Ohne nutzbares OpenGL, GPU erzwungen | Fallback, 408 ms | Fallback, **225 ms** |
| Abstürze | keine | keine |
- Bewertung: Bild gleichwertig, aber auf dem einzigen messbaren GL-Pfad (Software-GL) startet FemtoVG langsamer → **Boss: bleibt bei Skia** (D-016). Vergleichsjob aus der CI entfernt, Skript bleibt. Mesa ist kein echter GPU-Treiber; Entscheidung erst nach Test auf echter Grafikkarte (CI-Artefakt `explorer2-femtovg-test`).
- Linux-Vergleich (Xvfb/Mesa) bestätigt: Inhalt identisch, nur Text-Glättung unterscheidet sich.
- Fund: OLE-Drag ohne gedrückte Taste (Loslassen zwischen Ziehschwelle und Drag-Start) hätte beim nächsten Mausbewegen „irgendwo“ abgelegt → Drag startet nur noch bei gedrückter Primärtaste (berücksichtigt vertauschte Tasten).

## M4 – Dateioperationen
- **Neue Crate `explorer-ops`** (plattformneutral testbar): `exec` (Job-Engine, synchrones `run` + `spawn` im Hintergrund), `undo`, `simple` (Umbenennen, Neuer Ordner), `names` (Windows-Namensregeln, „ - Kopie“, „ (2)“), `clipboard` (CF_HDROP), `win_copy` (CopyFileExW mit Fortschritt/Abbruch).
- **Sicherheitsregeln**: Kopie in `.<name>.e2part` + Rename; Verschieben über Laufwerke löscht die Quelle erst nach vollständiger, geflushter Kopie; „Ersetzen“ = alte Datei in den Papierkorb (rückgängig machbar); Abbruch räumt Teil-Dateien weg; Fehler pro Element, Job läuft weiter.
- **App** (`src/fileops.rs`): Zwischenablage (intern + Windows-System-Clipboard), Jobs + Operations-Panel, Konflikt-/Bestätigungsdialog (modal, Tasten R/S/B/A/Esc bzw. Enter/Esc), Inline-Umbenennen, Neuer Ordner, Undo/Redo im Hintergrund, Drag & Drop (Slint 1.18 `DragArea`/`DropArea`), Befehlsleiste. Nach Einfügen/Umbenennen sind die neuen Elemente markiert.
- **Tests lokal**: ops 18 Integrationstests (+ Disk-Full mit 1-MiB-tmpfs), app 17 Unit (Tasten, Titel, Fortschrittstext, DnD-Regeln), E2E **100/100** (neu 38–52: Umbenennen inkl. Undo/Redo/Esc/Fehler, Neuer Ordner, Kopieren/Einfügen mit Konflikt „Beide behalten“ und „Ersetzen“ + Undo, Ausschneiden, Entf + Wiederherstellen, Umschalt+Entf mit Dialog, DnD verschieben/Strg-kopieren, 60 000-Dateien-Job mit Panel/Pause/Abbruch/Undo).

### CI-Run #26 – alles grün ✅
- E2E Linux **100/100** (M4-Block 38–52 inkl. DnD, Konflikte, Undo, 60 000-Dateien-Job mit Pause/Abbruch).
- Windows: ops-Integrationstests 19/19 (inkl. Papierkorb + Wiederherstellen, gesperrte Datei), Clipboard-Interop: Windows (.NET `GetFileDropList`) sieht unsere Dateien inkl. Umlaut, wir lesen Windows-Kopien, Ausschneiden-Flag überlebt.
- Linux: Disk-Full (1-MiB-tmpfs) → Fehlermeldung, keine Teil-Datei.
- Windows-UI-Matrix: normale Starts 44–45 ms bis Inhalt, Erststart 636 ms (frische VM), Daten: erste Zeilen 1,3 ms, alle 100k 133–143 ms.
- Exe 19,7 MB (vorher 17,2) – knapp unter Budget 20 MB → als Schuld vermerkt.
- Weg dorthin: Run #21 Clippy-1.99-Lint, #22 8.3-Kurznamen, #23 Papierkorb-Anzeigenamen, #24/#25 Clipboard-Test-Cmdlet (s. Funde).

### Funde in M4
- **Bug (Integrationstest, flaky 1/15)**: Undo von „Ersetzen“ legte erst die neue Datei in den Papierkorb und stellte dann den *neuesten* Papierkorb-Eintrag mit gleichem Pfad wieder her – also die gerade gelöschte neue Datei statt der alten. Fix: Papierkorb-Snapshot **vor** dem Undo, Wiederherstellen nur daraus.
- **Perf**: `fsync` pro kopierter Datei hätte Massenkopien vervielfacht → nur noch beim Verschieben (Quelle wird danach gelöscht).
- **DnD**: TouchArea als Geschwister über der DragArea schluckt den Mausdruck → TouchArea muss *Kind* der DragArea sein.
- Mehrfachauswahl wäre beim Ziehen verloren gegangen (Klick bei „down“) → bei bereits markierter Zeile wird der Klick erst beim Loslassen angewendet (Explorer-Verhalten).
- **CI-Run #21**: neuer Clippy-Lint der CI-Toolchain 1.99 (`chunks_exact_to_as_chunks`) → `as_chunks`, MSRV 1.88; lokal jetzt ebenfalls 1.99.
- **Bug (CI-Run #22, nur Windows)**: Wiederherstellen aus dem Papierkorb fand nichts, weil unser Pfad den 8.3-Kurznamen `RUNNER~1` enthielt, der Papierkorb aber den Langnamen speichert. Hätte echte Nutzer genauso getroffen (Kurznamen aus `%TEMP%`, Groß-/Kleinschreibung). Fix: Elternordner kanonisieren + auf Windows case-insensitiv vergleichen.
- **Bug (CI-Run #23, nur Windows)**: Ordner wurden jetzt gefunden, Dateien nicht: Der Papierkorb meldet den *Anzeigenamen*, und der blendet bekannte Endungen aus („weg“ statt „weg.txt“). Der Restore hätte die Datei sogar **ohne Endung** zurückgelegt. Fix: Abgleich über Namensstamm + Endung der `$R…`-Datei, Restore mit echtem Originalnamen.
- Testdesign: Pause *vor* der Konfliktfrage blockiert den Job schon vor der Frage → Test pausiert jetzt nach der Frage.

## M3 – Tabs, Split-View, Seitenleiste, Verlauf, Sitzung
- **Architektur**: `App` → Panes (1–2) → Tabs; jeder Tab eigener Pfad/Auswahl/Loader/Watcher/Verlauf. Hintergrund-Ergebnisse werden per stabiler Tab-ID zugeordnet (Tabs können zwischendurch geschlossen werden) + Generationszähler.
- **Tabs**: +/Strg+T (gleicher Ordner), ×/Mittelklick/Strg+W, Strg+(Umschalt+)Tab; inaktive Tabs laden erst bei Auswahl; Sortier-/Filterwechsel markiert alle als „stale“, sichtbare laden sofort neu.
- **Split-View**: „Zwei Ansichten“/Strg+\, F6 wechselt, Klick aktiviert, blaue Akzentlinie = aktive Pane, Tastaturfokus folgt der aktiven Pane.
- **Verlauf**: ←/→-Buttons, Alt+←/→, Maus-Seitentasten.
- **Seitenleiste**: Schnellzugriff (Windows Known Folders → auch umgeleitete/OneDrive-Ordner), „Dieser PC“ mit Laufwerken, Füllbalken, „x GB frei von y GB“; Abfrage im Hintergrund, Refresh alle 15 s, keine „Datenträger einlegen“-Dialoge.
- **Sitzung**: session.ini (entprellt 400 ms + beim Schließen); Start ohne Pfad → Restore inkl. aktiver Pane; Pfad-Argument gewinnt (wie `explorer.exe C:\x`).

### CI-Run #19 – alles grün ✅
- E2E Linux 60/60 (Drossel: 6 Refreshes / 3 s).
- Windows: Places-Test mit echten Known Folders + Laufwerken (C:\ mit Füllstand) grün; Laufwerksabfrage 0,17–0,34 ms (2 Laufwerke).
- Windows-UI-Matrix: normale Starts 66–69 ms (best 65,5 ms, Budget 300), Erststart 255 ms, Daten: erste Zeilen 0,8–2,2 ms, alle 100k 181–246 ms. (Mesa forciert GPU: 992 ms Kaltstart – reiner Testpfad auf Software-GL, Auto wählt dort korrekt Software.)
- fs-Bench Windows: Median 1,23 ms / 137 ms.

### Tests M3
- Unit: core 32 (+History, Session, format_usage), fs 24 (+Places: Anzeigenamen, Füllgrad, XDG-Parser, reale Laufwerke), app 13 (+Tasten für Tabs/Pane/Verlauf, Tab-Titel, Crumb-Budget).
- E2E 60/60 (neu 27–37): Sidebar-Klick, Alt+←/→, Maus-Zurück-Taste, Strg+T + unabhängige Navigation, Strg+Tab, Split an/aus, **Fokus wandert in neue Pane**, F6, Klick aktiviert Pane, Session gespeichert/restauriert (inkl. aktiver Pane), **lazy Laden** restaurierter Tabs, Strg+W, letzter Tab bleibt, Pfad-Argument schlägt Session.

### Funde in M3
- **Bug**: Nach Split blieb der Tastaturfokus in der linken Pane, während die rechte als aktiv galt → F6 „sprang zurück“. Ursache: neue Repeater-Instanz verpasst den Fokus-Auftrag (`changed` feuert nicht bei Init). Fix: Fokus folgt `active` (+ `init`). Per Tasten-Trace (`EXPLORER2_TRACE_KEYS`) nachgewiesen.
- **Bug**: Restore ignorierte die gespeicherte aktive Pane → behoben, E2E prüft `active_pane=1`.
- **Bug (eigener Code-Review)**: neue Tabs haben ihren Pfad schon → Breadcrumbs/Watcher wären nie eingerichtet worden → Setup jetzt auch, wenn noch kein Watcher existiert.
- Shortcuts kamen nicht an, solange keine Liste fokussiert war → Root-FocusScope als Fallback.
- E2E-Diagnose verbessert: je Block eigenes Log.
- **Bug (CI-Run #18)**: Refresh-Drossel garantierte den Abstand nur ab *Ende* eines Refreshs – Events während eines laufenden Refreshs starteten sofort einen zweiten (CI: 8 statt ≤ 7 Refreshes/3 s; seit M3 mehr Schreibvorgänge pro Refresh). Fix: Mindestabstand 500 ms ab *Start*, nach Done nur verlängern. Gemessen: kleinster Abstand 501 ms.

## M2 – Details-Ansicht, Auswahl, Tastatur, Öffnen, Breadcrumb
- **Details-Ansicht**: Spaltenköpfe Name / Änderungsdatum / Typ / Größe, Klick sortiert (▲▼), Breiten ziehbar; Sortierung in settings.ini.
- **Auswahl** (`core::selection`): Klick / Strg / Umschalt wie Windows-ListView (Strg+Klick setzt Anker auch beim Abwählen), Klick ins Leere hebt auf; bei Auto-Refresh/Umsortieren per Name übernommen.
- **Tastatur** (`keys::map_key`): Pfeile, Bild↑↓, Pos1/Ende, Strg+A, Strg+Leertaste, Enter, Rücktaste/Alt+↑, F5, Strg+L/Alt+D, Typeahead.
- **Öffnen**: Ordner navigieren, Dateien per `ShellExecuteW` (Windows) mit Standard-App; Windows-Test führt echte `.cmd` über Standard-Verb aus.
- **Breadcrumb** (`core::breadcrumb`): Windows-, UNC- und Unix-Pfade, „…“-Kürzung links; Strg+L/Klick ins Leere = Bearbeiten, Esc = Abbruch.
- Nach „Hoch“ ist der Herkunftsordner markiert (wie Explorer).

### CI-Run #16 – alles grün ✅ (Windows)
| Szenario | Fenster erstellt | Kaltstart bis Inhalt | Hinweis |
|---|---|---|---|
| Erststart (kein Cache) | 134 ms | **221 ms** | Software sofort, Probe im Hintergrund (34 ms, „gdi generic“) |
| Normaler Start 1/2/3 (Cache) | ~6,5 ms | **56 / 55 / 56 ms** | best 54,7 ms (Budget 300) |
| forciert GPU (ohne GPU) | 6,4 ms | 370 ms | Skia-Fallback |
| forciert Software / Setting aus | ~6,5 ms | 57 / 54 ms | |
| Mesa auto | 7,4 ms | 151 ms | Hintergrund-Probe erkennt llvmpipe (260 ms, nicht im Startpfad) |
| Mesa forciert GPU | 7,6 ms | 458 ms | Skia-OpenGL ok |

Daten in allen Szenarien: erste Zeilen 0,8–1,6 ms, alle 100k 162–285 ms. ShellExecute-Test (echte `.cmd` über Standard-Verb) grün. fs-Bench: Median 1,22 ms / 139 ms.

### Tests M2
- Unit: core 26 (Auswahl, Typeahead, Breadcrumb, Typ-Sortierung, Settings), app 10 (Tasten, Status, Auswahl-Remap, Öffnen).
- E2E 37/37 (neu 12–26): Sortierreihenfolgen per Spaltenkopf, Richtungswechsel, Persistenz, Pfeile/Umschalt+Ende/Strg+A, **Auswahlfarbe per Pixel**, Typeahead, Enter öffnet Datei (Stub-Öffner), Enter/Doppelklick öffnet Ordner, Rücktaste markiert Herkunftsordner, Strg+L+Esc, Breadcrumb-Klick.

### Funde in M2
- **CI-Run #15 rot (2 Ursachen):**
  1. E2E [26]: Testfehler – Breadcrumb navigierte korrekt zur Wurzel `/` (kurzer CI-Pfad → kein „…“), Prüfmuster `//*` passte nie → Vorfahren-Prüfung korrigiert (Gegenfälle getestet).
  2. Windows-Erststart 1666 ms: GPU-Probe **581 ms** (vorher 3–67 ms) + Fenster-Erstellung 755 ms (frische VM). → **Design-Fix**: Probe läuft nie mehr im Startpfad (ohne Cache: sofort Software, Probe im Hintergrund, gilt ab nächstem Start). Neuer Trace „window created“. Budget-Prüfung: normale Starts best-of-3 < 300 ms (hart), Erststart < 1 s (Plausibilität, ausgewiesen).
- Testerwartung falsch (nicht Code): Strg+Klick setzt den Auswahl-Anker auch beim Abwählen – wie Windows-ListView; Test korrigiert.
- Vermeintlicher Render-Bug (fokussierte Zeile „ohne“ Auswahlfarbe) war ein Lesefehler auf verkleinertem Screenshot – Pixelmessung zeigte korrekte Farbe; „Fix“ zurückgenommen, Auswahlfarbe jetzt per Pixel im E2E geprüft.
- Klick auf die Adressleiste traf nach Breadcrumb-Umbau ein Segment → Explorer-Shortcuts Strg+L/Alt+D ergänzt, E2E nutzt sie.
- Eigenes Gotcha ignoriert (`viewport-y` deprecated) → `content-y`.

## M1.5 – Renderer-Wahl, Einstellungen, Icon (Boss: Option 2 + Schalter)
- **GPU-Probe** (`explorer-gpu`, Windows/WGL): verstecktes 1×1-Fenster, Legacy-GL-Kontext, prüft `GL_RENDERER`/`GL_VERSION`/`glCreateShader`. Software-GL (GDI Generic, llvmpipe, Microsoft Basic Render) = unbrauchbar, weil langsamer als unser Software-Renderer.
- **Entscheidung** (`core::renderer::decide`): Override `EXPLORER2_RENDERER` > Einstellung aus → Software > Probe (gecacht) → Skia-GPU oder Software. `SLINT_BACKEND` gesetzt → Toolkit entscheidet.
- **Cache**: Ergebnis in `settings.ini` (pro App-Version), Hintergrund-Re-Probe 5 s nach Start (Treiber neu/entfernt → wirkt beim nächsten Start). Erster Start zahlt die Probe einmal.
- **Einstellungen-Panel**: „Hardwarebeschleunigung verwenden, wenn verfügbar“ + Statuszeile („Aktuell: Software-Rendering (keine geeignete GPU gefunden)“). `settings.ini` atomar geschrieben, tolerant gelesen; „Ausgeblendete“ wird ebenfalls gespeichert.
- **Icon**: gelber Ordner + blaues Band auf heller Kachel; Exe-Icon (ICO 16–256) + Fenster-Icon.

### Getestete Umgebungen (lokal, Linux/Xvfb)
| Umgebung | Ergebnis |
|---|---|
| ohne GL, auto | Plattform-Default, läuft |
| ohne GL, forciert GPU | Skia (Fallback), läuft |
| forciert Software | läuft |
| Mesa-GL (llvmpipe), auto / forciert GPU | läuft; GPU-Modus lädt nachweislich EGL/Mesa (58–62 Treiberzeilen), Software-Modus 0 |
| Einstellung aus | Software, Grund „disabled in settings“ |
| E2E | 20/20 PASS (neu: Panel speichert Setting, Ausgeblendete persistiert, Neustart respektiert Setting, Fenster-Icon gesetzt) |

### CI-Run #12 (Windows-Umgebungsmatrix)
| Szenario | Kaltstart bis Inhalt | Ergebnis |
|---|---|---|
| Auto, erster Start (Probe 3,4 ms → „gdi generic“) | ~50 ms | ✅ Software (vorher 488–679 ms) |
| Auto, aus Cache + Hintergrund-Re-Probe | 129 ms | ✅ |
| GPU erzwungen (ohne GPU) | 355 ms | ✅ läuft (Skia-Fallback) |
| Software erzwungen | 45 ms | ✅ |
| Einstellung aus | 45 ms | ✅ |
| Mesa-GL | – | ❌ **Testaufbau**: Mesa 26.2.6 MSVC-Paket enthält kein opengl32.dll (bekanntes Upstream-Problem) → auf MinGW-Paket umgestellt |

### CI-Run #13 – alles grün ✅
| Szenario (Windows) | Wahl | Kaltstart bis Inhalt |
|---|---|---|
| ohne GPU, 1. Start (Probe 67 ms) | Software („gdi generic“) | 270 ms |
| ohne GPU, Cache | Software | **55 ms** |
| forciert GPU ohne GPU | Skia-Fallback, läuft | 502 ms |
| forciert Software | Software | 56 ms |
| Einstellung aus | Software | 59 ms |
| Mesa, auto | Software (llvmpipe erkannt) | 299 ms |
| Mesa, forciert GPU | **Skia-OpenGL-Surface ok** (GL-Pfad bewiesen) | 485 ms |

Daten-Budgets in allen Szenarien eingehalten (erste Zeilen 0,8–1,4 ms, alle 100k 151–206 ms). Exe-Icon geprüft (Mittelpixel 255/203/69). Hinweis: Probe kostet beim **ersten** Start je nach Treiber 3–239 ms (danach Cache) – echte GPU-Werte nur auf Boss-PC messbar.

### Bugs/Funde in M1.5
- E2E-Klickkoordinate „Ausgeblendete“ hätte nach neuem Button den falschen Knopf getroffen → angepasst (x=880).
- UI-Statuszeile war englisch → `Reason`-Enum: Englisch für Traces, Deutsch (`status_de`) fürs Panel.
- Vermeintlich fehlendes Fenster-Icon war ein `xprop`-Anzeigeproblem (Rohdaten: 256×256, gelb) → als E2E-Check [11] abgesichert.

## M1 – was gebaut wurde
- `explorer-fs`: Windows-Fastpath (`FindFirstFileExW` + Large Fetch), portabler std-Pfad, Verbatim-Langpfade, Hintergrund-Loader mit Batches/Abbruch, Watcher mit Debounce.
- `explorer-core`: Entry mit hidden/system/readonly/is_link, `ViewFilter` (Explorer-Regeln), schnellere natürliche Sortierung, `format_count`.
- `explorer2`: `nav.rs`-Controller – UI-Thread macht kein I/O, Generationszähler, Auto-Refresh mit adaptiver Drossel, Statusleiste, Fehleranzeige, Rückfall bei gelöschtem Ordner, Toggle „Ausgeblendete“.

## Messwerte (Release, 100 000 Einträge)
| Metrik | Windows (CI-Runner) | Linux (Sandbox) | Budget |
|---|---|---|---|
| Erster Batch (fs-Bench) | **1,3 ms** | ~6 ms | < 100 ms |
| Voll laden + sortieren (fs-Bench) | **130 ms** | ~250 ms | < 1 s |
| Sortierung allein | 46 ms | 47 ms (vorher 173 ms) | – |
| UI: Loader fertig (100k inkl. Format) | 187 ms | 233 ms | < 1 s |
| UI: erste Zeilen bereit (Loader-Thread) | **0,9 ms** (Default) / 1,1 ms (Software) | 1–1,3 ms | < 100 ms |
| UI: alle 100k bereit (Loader-Thread) | **140 ms** / 146 ms | 277–296 ms | < 1 s |
| UI: Kaltstart bis erster Inhalt – Default-Renderer (Skia, GL schlägt fehl → Fallback) | **488 ms** | 26 ms | M8: < 300 ms ❌ |
| UI: Kaltstart bis erster Inhalt – Software-Renderer | **43 ms** | 37 ms | M8: < 300 ms ✅ |

**Renderer-Befund (Run #9):** Auf Systemen ohne GPU-OpenGL (CI-Runner, VMs, RDP) kostet der fehlgeschlagene Skia-GL-Versuch ~450 ms Startzeit. → Entscheidung für M2/M8 nötig (Software-Renderer als Default? GL-Fähigkeit vorab prüfen? D3D zuerst?).

**OS-Stalls auf Windows-CI (Run #8/#9):** Einzelne Läufe brauchen 168–170 ms schon im **rohen** `FindFirstFileExW`-Aufruf (unser Code-Anteil ~0,2 ms) – sporadisch (Run #9: 2 von 6 Läufen), Ursache Fremdlast auf dem Runner (Defender/Indexer nach Massen-Anlage). In Run #8 traf es die Mehrheit → Median kippte. Für Nutzer abgefedert: UI blockiert nie, zeigt „Lädt …“.

**Befund Windows-CI:** Runner hat kein GPU-OpenGL → Slint/Skia meldet „Failed to initialize Skia opengl surface … Falling back“, App läuft trotzdem (Fallback funktioniert = gut für VMs/RDP). Der GL-Versuch kostet Startzeit → TODO: auf Windows D3D zuerst / GL-Versuch vermeiden (M2/M8).

## Tests
- Unit/Integration: 31 Tests (core 7, fs 20, app 4) – grün; fs-Windows-Code per `clippy --target x86_64-pc-windows-msvc` sauber.
- E2E unter Xvfb (13 Checks, alle PASS): 100k-Ordner, Auto-Refresh, gelöschter Ordner → Elternordner, ungültiger Pfad → Fehler + Adresszeile zurückgesetzt, Toggle Ausgeblendete, Hoch, Adresszeile folgt nach Tippen, Refresh-Drossel, kein Absturz/Panic.
  - Skript: `scripts/e2e.sh`, läuft in CI als Job `e2e-linux` (Screenshots als Artefakt bei Fehler).

## Bugs gefunden & gefixt in M1
1. **Refresh-Endlosschleife**: Datei, die ständig geschrieben wird (hier: eigenes Trace-Log), löste alle ~100 ms einen kompletten Reload aus → adaptive Drossel (≥ 500 ms, 4× letzte Ladezeit, max 10 s). Gemessen: 30 → 6 Refreshes / 3 s.
2. **Adresszeile eingefroren**: Einweg-Bindung `text: current-path` wird von Slint gelöscht, sobald der User tippt → danach nie mehr aktualisiert (auch nicht bei Hoch/Doppelklick). Fix: Zwei-Wege-Bindung `<=>`.
3. Statusleiste „1 Elemente“ → „1 Element“.
4. Sortierung zu langsam (173 ms / 100k) → allokationsfrei, 47 ms.
6. **Flaky Perf-Gate**: Median über 5 Läufe kippte bei sporadischen OS-Stalls auf dem Runner (Beweis: rohe OS-Abfrage selbst 168 ms) → best-of-N-Gate, 9 Läufe, Stalls werden gezählt und ausgegeben.
5. Messfehler: „first paint“ auf Windows (369 ms) enthielt Fenster-/Renderer-Start. 1. Versuch (Marker „event loop running“) war falsch: Run #7 zeigte, dass Slint den Renderer erst **nach** Loop-Start beim ersten Frame initialisiert (GL-Fehlversuch + Fallback blockiert UI-Thread ~490 ms). Rendering-Notifier als Marker ungeeignet (Software-Renderer/Skia-Software-Fallback unterstützen ihn nicht). **Lösung:** Daten-Latenz wird im Loader-Thread gemessen (`data_ready=`), Kaltstart bis Inhalt separat (`at=`); CI vergleicht Default- vs. Software-Renderer.

## Offen
- ✅ Run #10 grün. Windows-Bench (9 Läufe): erster Batch best 1,1 ms / **Median 1,17 ms** / Max 1,52 s (OS-Stall); voll+sortiert best 101 ms / **Median 115 ms**; OS-Stalls 2/9 (roh 176–314 ms) – Gate hält stabil.
- UI-Smoke Run #10: Default-Renderer erste Zeilen 1,3 ms, alle 100k 230 ms, **Kaltstart 679 ms**; Software-Renderer 1,3 ms / 190 ms / **Kaltstart 54 ms**.
- Offen für M2: **Renderer-Entscheidung** (Boss).
- Fix im Test selbst: E2E-Regex an neues `at=`-Feld angepasst (lokal 13/13 PASS).

## Nächste Schritte
1. ✅ M4 abgeschlossen (Run #26) → **Freigabe M5 (Boss)**; davor Exe-Größe prüfen (19,7 MB).
2. M5: Shell-Integration (Icons/Thumbnails, Kontextmenü, Eigenschaften, Papierkorb, OLE-Drag&Drop mit anderen Apps).
3. Boss: Artefakt auf echtem PC testen (GPU, Kopieren/Einfügen mit dem echten Explorer).
