# Zwischenstand

**Stand:** 2026-11-05 · **Phase:** 1 · M0–M7 ✅ (M7.3 verschoben) · R-001 ✅ · **M8.1/M8.2 Installer ✅ (Run #69)** · M8 Härtung/Beta offen · **Branch:** `claude/relaxed-pasteur-ufhh1p`

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

### M8.1/M8.2 Installer ✅ (CI-Run #69)
- Boss-Entscheidung: **eigener Installer in Rust** (D-023), pro Benutzer, ohne Administratorrechte.
- `explorer2.exe --install [--quiet]` bzw. Einstellungen → „Installation“ → „Installieren“ (mit Nachfrage): kopiert App + Notfall-Werkzeug nach `%LOCALAPPDATA%\Programs\Explorer2`, legt im Startmenü „Explorer 2.0“ und „Explorer 2.0 – Windows-Standard wiederherstellen“ an (`.lnk` per `IShellLinkW`), trägt sich in **Apps & Features** ein (Version, Größe, Deinstallieren) und startet danach aus dem Installationsordner weiter. Startmenü + Eintrag laufen **über das Journal** – der Ein-Knopf-Reset und `explorer2-recover` entfernen sie mit.
- Integrationen, die aus einer heruntergeladenen Kopie eingeschaltet wurden, werden auf die installierte Exe umgebogen (nur eindeutig eigene; fremd übernommene bleiben).
- Update über eine laufende App: die laufende Exe wird beiseite umbenannt (`*.old-<pid>`), beim nächsten Start aufgeräumt.
- Deinstallation (Apps & Features / `explorer2-recover --uninstall [--quiet]`): kompletter R-001-Reset (alle Integrationen, Startmenü, Eintrag), dann löscht eine Temp-Kopie den Programmordner – **nur, wenn er ausschließlich unsere Dateien enthält** (Test mit fremder Datei) – und sich selbst; Einstellungen bleiben.
- Tests: integrate 24 Unit (Installieren → Deinstallieren = Snapshot, Update = ein Eintrag + erstes Original, Integrationen folgen der installierten Exe, fehlende Dateien → nichts journalisiert, fremder Ordner wird nie gelöscht, Eintrag im Manifest), Linux-Funktionstest + E2E [72] (Palette → Nachfrage → installierte Kopie läuft, alte schließt; Deinstallation räumt alles), Windows-CI `scripts/m8-install.ps1` [I1]–[U6] mit den echten Standardpfaden.
- CI #68: Windows komplett grün (I1–U6), Linux [72b–d] rot – der E2E-Job baute nur `explorer2`, ohne `explorer2-recover` daneben verweigert der Installer (gewollt). Fix: Job baut `-p explorer-integrate --bins` mit → #69 grün.

### M7 ✅ komplett (7.1 + 7.2); M7.3 IFEO verschoben (Boss, D-022) – nächster Schritt M8 Installer + Recovery

### M7.2 Win+E ✅ (CI-Run #67)
- Einstellungen/Palette: **„Win+E öffnet Explorer 2.0“** – `HKCU\Software\Classes\CLSID\{52205FD8-5DFB-447D-801A-D0B52F2E83E1}\shell\opennewwindow\command` = `"…\explorer2.exe"` (ohne Ordner → letzte Sitzung) + leerer `DelegateExecute` (sonst bleibt Explorers COM-Handler). Über Journal/Manifest wie M7.1, Aus = exakt vorher (`CLSID` selbst wird nur entfernt, wenn wir ihn angelegt haben und er leer ist).
- Windows-E2E W43–W43h: echtes Win+E per `keybd_event`; vorher Prüfung, ob die Runner-Shell Win+E überhaupt verarbeitet (sonst nur Registry-Prüfung + NOTE); an → Explorer 2.0 startet, kein Explorer-Fenster; aus → CLSID-Schlüssel identisch, Win+E öffnet wieder Windows' Explorer.
- CI #65/#66: Win+E startete Explorer 2.0 schon richtig (Prozess von der Shell gestartet, ohne Ordner, kein Explorer-Fenster); rot waren nur Testfehler – Trace-Datei erbt die Shell nicht, PS 5.1 zählt ein einzelnes `CimInstance` als `$null`.

### M7.1 Ordner öffnen + Kontextmenü ✅ (CI-Run #64; funktional schon #63)
- **CI #63**: Windows-E2E **81/81** inkl. W42–W42m: echtes ShellExecute auf einen Ordner und auf `C:\` startet Explorer 2.0 dort, Kontextmenü zeigt „Öffnen (Explorer 2.0)“ (Standard) + „In Explorer 2.0 öffnen“, `InvokeVerb` öffnet, `Folder` unberührt, beide aus → `reg export` identisch; R-001-Snapshot inkl. [R5]–[R5e]; Linux-E2E 188/188. Rot nur das Erststart-Gate (D-018): 3305 ms bei `probe_ms` **1189** (sonst 70–170), `window created` schon bei 897 ms vor jedem App-Code (sonst ~150) → VM; normaler Start best-of-3 **86,8 ms** (Budget 300). Boss: Gate an die VM koppeln (rot ab 2 s nur bei `probe_ms` < 300, sonst Warnung) → D-018 ergänzt.
- CI #62: W42 lief nicht (PS 5.1 + `reg export` auf fehlenden Schlüssel = Abbruch, Testskript-Fehler) → behoben; `[forced-gpu]` hing einmalig nach Skias GL-Rückfall (in #63 wieder 505 ms).
- Einstellungen → „Windows-Integration“: **„Ordner mit Explorer 2.0 öffnen (Doppelklick, „Ordner öffnen“)“** und **„„In Explorer 2.0 öffnen“ im Kontextmenü“**, einzeln an/aus (auch per Palette); nur unter Windows sichtbar. Aus = exakt der Zustand vorher (nur die Werte dieser Funktion, Rest bleibt im Journal).
- Ordner öffnen: eigenes Verb `explorer2open` als **Standard-Verb** von `Directory` und `Drive` (Windows liefert dort `none`, ohne eigenes Standard-Verb fiele alles auf `Folder` = Explorer zurück). `Folder` (Systemsteuerung, Papierkorb, …) bleibt bei Windows, Windows' `open`-Verb unangetastet.
- Sicherheit: Exe verschoben/gelöscht → beim nächsten Start werden die Einträge auf die laufende Exe umgebogen (sonst „Windows findet … nicht“ bei jedem Ordner); beim Einschalten Kopie von `explorer2-recover.exe` nach `%LOCALAPPDATA%\Explorer2` (überlebt das Löschen des App-Ordners); Hinweis „nicht verschieben“.
- Gefunden: Laufwerke kommen als `"C:\"` → nach Windows-Quoting `C:"` → `launch_arg` repariert (Test).
- Tests: integrate 18 Unit (u. a. Funktionen einzeln an/aus = Snapshot, verschobene Exe, fremd übernommenes Standard-Verb bleibt, Sweep ohne Journal räumt alle Funktionen, `Folder` nie berührt, jede Änderung im Manifest); R-001-CI-Test [R5]–[R5e] (Funktionen mit/ohne Journal zurück = Snapshot); Windows-E2E W42–W42m: per Palette an → **echtes ShellExecute auf einen Ordner startet Explorer 2.0 dort**, Laufwerk `C:\` kommt heil an, Kontextmenü-Eintrag sichtbar + per `InvokeVerb` aufrufbar, `Folder` unberührt, beide aus → `reg export` identisch zu vorher.

### R-001 Ein-Knopf-Reset („Notausgang“) ✅ komplett (CI-Run #59–#61)
- Neue Crate **`explorer-integrate`**: jede Windows-Integration (ab M7) läuft nur über `apply()`. Vor jeder Änderung steht der Zustand vorher im **Journal** (write-ahead, atomar mit Flush, Spiegelkopie, Prüfsumme; Registry-Werte roh als Typ + Bytes → bit-genauer Rückbau; Dateien mit Sicherung).
- `reset()`: Journal rückwärts – nur wo der Wert **noch uns gehört**; hat eine andere Software ihn inzwischen geändert, bleibt er und steht im Bericht („von anderer Software geändert – belassen“). Eigene Schlüssel nur, wenn leer. Danach **Sweep der bekannten Integrationsstellen** (Ordner-Verben Folder/Directory/Drive, eigene Kontextmenü-Einträge, Win+E, `HKCU\Software\Explorer2`; erkannt am Befehl mit `explorer2`) → funktioniert auch ohne oder mit kaputtem Journal. Fehler (z. B. HKLM ohne Admin) bleiben im Journal für den nächsten Versuch. Umgebung → `WM_SETTINGCHANGE`.
- **`explorer2-recover.exe`**: Konsole, keine UI/GPU (läuft auch im Abgesicherten Modus), `--status` (Exit 1 = aktiv), `--yes`, sonst Nachfrage; als zweite Datei im Windows-Artefakt.
- **Pflichttest in CI** (`scripts/r001-snapshot.ps1`, Snapshot mit Windows' eigenem `reg.exe export`, unabhängig von unserem Code): Integration an (Ordner-Verb, Umgebungsvariable, überschriebener Fremdwert, eigene Schlüssel, Datei) → Reset → **Snapshot identisch** – (1) aus dem Journal, (2) mit halb zerstörtem Journal über den Spiegel, (3) ganz ohne Journal über den Sweep; dazu „zweiter Reset = nichts zu tun“. Run #59: 15/15, Umgebung + Fremdwert bit-genau (gleicher Hash).
- Tests: 13 Unit-Tests gegen die Fake-Registry (u. a. Absturz zwischen Journal und Änderung, fremd geänderte Werte, HKLM verweigert → zweiter Versuch, Dateien, Formatschäden) + Windows-Unit-Test gegen die echte Registry (ohne Sweep, damit ein Entwickler-PC nie angefasst wird).
- Gefunden beim Testen: ein Schlüssel, unter dem ein Rückbau scheiterte, flog als „nicht leer“ aus dem Journal → zweiter Versuch hätte ihn liegen lassen; jetzt bleibt er drin.
- **CI #60 rot**, zwei Funde: (1) **echter Bug**: Palette-Befehl mit Dialog (Zurücksetzen, aber auch „Endgültig löschen“/„Papierkorb leeren“) – Enter im Dialog kam nicht an, weil die Fokus-Rückgabe der schließenden Palette mit dem Dialog-Fokus konkurrierte → Befehle laufen jetzt einen Event-Loop-Durchlauf später; Linux-E2E [71f2] deckt den Weg ab. (2) W11 wartete starr 2,5 s auf den Eigenschaften-Dialog der Shell; er kam später, blieb offen und nahm W12–W17 den Fokus → Test pollt jetzt bis 10 s und wartet aufs Schließen.
- **CI #61 grün**: Windows-E2E 68/68 (W41: echte Integration per App-Palette zurückgesetzt – 6 Werte/Dateien, 7 Schlüssel, 0 Fehler, Fremdwert exakt zurück; W11 Dialog nach 261 ms), R-001-Snapshot 15/15, Linux-E2E 188/188.
- R-001c: Einstellungen → **„Windows-Integration“** (Status beim Öffnen, Knopf „Alle Windows-Integrationen zurücksetzen“ nur wenn etwas aktiv ist, Nachfrage mit Stand), Palette-Befehl; Linux-E2E [71]–[71g] (beschädigtes Journal per Knopf zurückgesetzt, 187/187), Windows-E2E W41–W41h (echte Integration per App-Palette zurückgesetzt).

### M6.4 Farbschemata ✅ (CI-Run #58) → **M6 komplett**
- Einstellungen → **Farbschema: Wie Windows / Hell / Dunkel** (+ Befehlspalette „Farbschema: …“), Häkchen **„Windows-Akzentfarbe verwenden“**; gespeichert in settings.ini (`theme`, `system_accent`).
- **Alle Farben zentral** in `core::theme::tokens` (37 Tokens) → Slint-Global `Theme`; main.slint hat keine Hex-Farbe mehr. Hell = exakt der bisherige Look (Auswahl weiterhin #cce8ff).
- Akzent: Auswahl, Hover, aktueller Ort, Ablageziel übernehmen den **Farbton** des Windows-Akzents bei gleicher Helligkeit (Gelb wird nicht grell); Fokusrahmen/aktive Pane/Fortschritt nutzen Windows' eigene Schattierung (AccentPalette „Dark 1“ hell, „Light 2“ dunkel), bei Bedarf auf Kontrast ≥ 4,5 nachgedunkelt/-aufgehellt.
- „Wie Windows“ folgt **live**: Registry-Watcher (`RegNotifyChangeKeyValue` auf Themes + DWM + Accent) → neu lesen → nur bei Änderung anwenden. Std-Widgets (Buttons, Checkboxen, Scrollbalken) über `Palette.color-scheme`, Titelleiste über `DWMWA_USE_IMMERSIVE_DARK_MODE`.
- Kein Hell-Aufblitzen: Schema wird vor dem ersten Bild gesetzt (Registry-Lesen ~0,01 ms im Startpfad).
- Tests: core-Unit (WCAG-Kontrast aller Text-/Hintergrund-Paare für 12 Akzente × hell/dunkel inkl. Gelb/Weiß/Schwarz, Akzent-Rahmen ≥ 3:1, HSL-Rundlauf, AccentPalette/ABGR-Format, Settings-Rundlauf), Linux-E2E [70]–[70k] per **Pixelmessung** (Liste #fafafa → #191919, Auswahl #1c3f63, Seitenleiste #202020, Std-Button dunkel, Neustart startet dunkel, System dunkel + grüner Akzent → grüne Auswahl, Akzent-Häkchen aus → Blau), Windows-E2E W37–W40 mit **echter Registry** (live dunkel/hell, live Akzentwechsel, Titelleiste; Registry danach exakt wiederhergestellt).
- **CI #57:** funktional alles grün (Windows-E2E inkl. W37–W40, Linux-E2E 180/180), rot nur das Erststart-Gate mit 2709 ms. Ursache: **VM**, nicht M6.4. Schon *vor* dem ersten Theme-Code war `window created` bei 612 statt 144 ms (#56), die Hintergrund-Probe bei 521 statt 70 ms (7×) und run→Event-Loop bei 2011 statt 406 ms. Theme-Init kostet 0,49 ms, Linux First Paint 10 → 9 ms. Der Abbruch am Erststart hatte aber die echte Budget-Messung (normaler Start < 300 ms) verhindert → CI sammelt Zeit-Gates jetzt und wirft erst am Ende der Matrix, mit VM-Kennzahl `probe_ms` im Log.
- **CI #58 grün** (normale VM, `probe_ms` 84,5): Erststart 677 ms (#56 ohne M6.4: 633), normaler Start best-of-3 **102 ms** (#56: 94,5; Budget 300), Windows-E2E inkl. W37–W40 grün.

### M6.3 Befehlspalette ✅ (CI-Run #55)
- **Strg+Shift+P** öffnet ein Suchfeld oben mittig; ↑/↓/Bild↑/Bild↓ wählen, Enter führt aus, Esc/Klick daneben schließt.
- `core::commands` (rein, getestet): **ein** Verzeichnis aller 45 Befehle (ID, deutscher Name, Kürzel, Bedingung: Auswahl / echter Ordner / Einfügen möglich / Rückgängig / Wiederholen / Papierkorb), `fuzzy_score` (Teilfolge, Umlaute gefaltet, Wortanfänge + aufeinanderfolgende Treffer zählen), `rank` (zuletzt benutzte zuerst), `push_recent`.
- Ausführung über dieselben Wege wie die Tasten: `nav::run_action` (aus `list_key` herausgelöst) bzw. `fileops::command`; Unit-Test stellt sicher, dass jeder Befehl des Verzeichnisses ausgeführt wird.
- Nicht verfügbare Befehle grau (Enter → „Hier nicht möglich“); zuletzt benutzte (8) in settings.ini (`recent_commands`).
- „>“-Modus: Orte aus Seitenleiste, Laufwerken, virtuellen Orten und dem Verlauf aller Tabs.
- Tests: Unit, Linux-E2E [69]–[69g] (168/168), Windows-E2E W36.
- CI #53 rot am Erststart-Gate: run→erstes Bild 1376 ms (#51: 456, #52: 592; VM zusätzlich ~2× langsamer). Die Palette war unsichtbar (`visible: false`), aber **aufgebaut und gelayoutet** → jetzt eigene Komponente unter `if pal-open` (entsteht erst beim Öffnen, `init` fokussiert das Feld).
- #54 (gleicher App-Code wie #55) trotzdem rot: 1178 ms; #55: **731 ms** bei identischem Code → das Erststart-Gate misst vor allem die VM (Hintergrund-Probe 170 vs 89 ms). CI lässt den Windows-E2E jetzt auch nach einem roten Zeit-Gate laufen (Funktionssignal geht nicht mehr verloren). Gate jetzt Warnung ab 1 s, rot ab 2 s (Boss, D-018).

### M6.2 Inspector ✅ (CI-Run #52)
- Rechte Seitenleiste (280 px), **Alt+P** oder Knopf „Details“, Zustand in settings.ini (`inspector`).
- Inhalt: Vorschau (Windows-Vorschaubild 256 px bzw. großes Icon; Textdateien: erste 40 Zeilen, UTF-8/BOM, Binär erkannt), Titel + Typ, Größe (ab 1 KB mit Bytes), Geändert, Erstellt, Attribute, Abmessungen (aus dem Dateikopf, `core::inspect::image_size`), Pfad als markierbarer Text (kopieren).
- Mehrfachauswahl: Anzahl, Größe der Dateien, Gesamt inkl. Ordnerinhalt. Ordner: Inhalt + Gesamtgröße live im Hintergrund (`fs::dir_stats`: abbrechbar, Links/Junctions werden nicht verfolgt, unlesbare Ordner gezählt, Grenze 500.000 Einträge).
- Bewusst ohne Plattenlauf: der *offene* Ordner zeigt nur seinen direkten Inhalt (sonst würde `C:\` beim Öffnen die ganze Platte zählen); ganze Laufwerke werden nie automatisch gezählt.
- Entprellt (120 ms), Generationszähler verwirft veraltete Ergebnisse, laufende Zählung wird bei neuer Auswahl abgebrochen.
- Tests: Unit (`core::inspect`, `fs::stats` inkl. Symlink-Schleife + Grenze), Linux-E2E [68]–[68g] (161/161), Windows-E2E W35/W35b (echtes Vorschaubild + Maße des PNG).

### M6.1c Ansicht pro Ordner ✅ (CI-Run #51) → **M6.1 komplett**
- `core::views::FolderViews` (rein, getestet): Ordner → Ansicht, LRU bis 5000, Windows case-insensitiv, toleranter Parser; Datei `views.ini` (`modus<TAB>pfad`, atomar geschrieben).
- Ansicht hängt jetzt am **Tab** (beim Navigieren: gemerkte Ansicht des Ordners, sonst Standard); `PaneData` trägt Modus + Kachelmaße → linke und rechte Pane können verschieden aussehen. Umschalten merkt die Ansicht für den Ordner.
- Einstellungen: „Aktuelle Ansicht für alle Ordner“ → wird Standard (`view` in settings.ini), alle gemerkten Ordner vergessen.
- Tests: Unit, Linux-E2E [67]–[67e] (154/154).

### M6.1b Sortier-Menü + Rahmen-Auswahl ✅ (CI-Run #51)
- CI-Run #50 rot am **Erststart-Gate** (frische VM: 2140 ms > 1000 ms; Fenster→Event-Loop 1,6 s statt ~0,4 s in #40/#48). Verdacht: Zeichen „▾“ im neuen Sortieren-Knopf fehlt in Segoe UI → Schrift-Fallback-Suche beim ersten Rendern. Fix: Dreieck und Auswahlpunkt gezeichnet statt als Zeichen; neue Zeitmarken `init done`/`run` im Trace. **Bestätigt in #51**: Fenster→Event-Loop 1627 ms (#50) → 456 ms (#51, #48: 409 ms), Erststart 2140 → 702 ms; Init selbst nur 8 ms, die Zeit steckt im ersten Frame.
- „Sortieren ▾“ in der Statusleiste (Popup): Name/Änderungsdatum/Typ/Größe (Richtung bleibt), Auf-/Absteigend; aktuelle Wahl mit Punkt. Kachel-Ansicht ohne Spaltenköpfe (wie Explorer).
- Rahmen-Auswahl: Ziehen auf freier Fläche (unter den Zeilen, neben/zwischen Kacheln) zeichnet einen Rahmen; berührte Einträge werden live markiert, mit Strg zusätzlich zur Auswahl vom Start. `core::view::band_hits` + `Selection::set_band` (rein, getestet).
- Gefunden: Slint feuert `clicked` **vor** dem Pointer-Up-Ereignis → Rahmen wird im Pointer-Ereignis abgeschlossen, nicht in `clicked` (sonst kam das Ende nie an).
- Tests: Unit (Rahmen-Treffer Details/Raster, Auswahl mit/ohne Basis), Linux-E2E [65]–[66d] (149/149), Windows-E2E W34 (Rahmen über zwei Kacheln mit echter Maus).

### M6.1a Ordneroptionen ✅ (CI-Run #49)
- Einstellungs-Panel, Abschnitt „Ordneroptionen“ (in settings.ini: `show_extensions`, `show_protected`, `single_click`):
  - **Dateinamenerweiterungen anzeigen** (Standard an): aus → Endung nur bei *bekannten* Typen ausgeblendet (Windows: `HKCR\.ext` existiert, gecacht; Ordner, `.gitignore`, `ende.` nie). F2 bearbeitet dann nur den angezeigten Namen, die Endung wird beim Speichern wieder angehängt („x.txt“ tippen → „x.txt.txt“, wie Explorer); bei Fehler bleibt der getippte Text ohne Endung im Editor (keine doppelte Endung).
  - **Geschützte Systemdateien anzeigen** (Standard aus): versteckt+System bleibt auch bei „Ausgeblendete“ unsichtbar.
  - **Öffnen durch einfachen Klick**: Loslassen öffnet, Zeigen (600 ms) markiert, Strg/Shift-Klick wählt nur aus; Klicks bis 600 ms nach einem Öffnen werden ignoriert (sonst öffnet der 2. Klick eines Doppelklicks, was im nächsten Ordner unter dem Zeiger liegt); gilt für Zeilen und Kacheln.
- Tests: Unit (`core::display`, Settings-Roundtrip, Windows `registered_types`), Linux-E2E [63]–[64e] (141/141), Windows-E2E W31–W33b (zweiter Start mit settings.ini: `.txt` ausgeblendet, unbekannte Endung bleibt, `attrib +h +s`-Datei unsichtbar, F2 behält Endung).

### M5.7 Kachel-Ansicht + Vorschaubilder ✅ (CI-Run #48; W28c: echtes PNG-Vorschaubild, Kachel-Mitte exakt R220 G20 B20) → **M5 komplett**
- `core::view::ViewMode` (Details/Mittel/Groß/Extra groß, 48/96/256 px, Kachelmaße, `step` für Strg+Mausrad, Explorer-Kürzel Strg+Shift+1/2/3/6 inkl. US/DE-Shift-Zeichen), Einstellung `view` in settings.ini.
- `shell::thumbnail(path, px)`: `IShellItemImageFactory::GetImage` (Vorschau aus dem Windows-Thumbnail-Cache, sonst großes Shell-Icon), vormultipliziertes BGRA → RGBA; Windows-Test mit selbst geschriebener BMP.
- `app::thumbs`: 2 Worker, nur sichtbare Kacheln, neueste Anfrage zuerst, weggescrollte werden übersprungen, 96-MB-Grenze (LRU, kommen beim Zurückscrollen neu), Schlüssel inkl. Änderungszeit (bearbeitetes Bild → neue Vorschau).
- UI: Raster als zweite ListView über **stabile** Zeilen-/Spalten-Modelle aus Rust (ein `int`-Repeater hätte bei jeder Auswahländerung alle Kacheln neu gebaut → Doppelklick/Hover kaputt, gemessen per `init`-Debug), Kachel = gleiche Interaktion wie eine Zeile (Klick/Strg/Shift, Ziehen einer Mehrfachauswahl, Rechtsklick, Doppelklick, Slint-DnD/OLE-Drag), Pfeiltasten im Raster, F2-Feld über der Beschriftung, Kontextmenü per Tastatur an der Kachel, OLE-Drop-Trefferprüfung mit Zeilenhöhe/Spalten.
- Strg+Mausrad gedrosselt (max. 1 Stufe/250 ms, ab Start gemessen): X11 liefert einen Rad-Klick doppelt, Touchpads viele kleine Ereignisse.
- Tests: Unit (ViewMode, Tasten im Raster, Trefferprüfung im Raster, BGRA), Linux-E2E [62]–[62h] (132/132), Windows-E2E W28–W30 (echtes rotes PNG → roter Pixel in der Kachel).
- Runs #45–#47 rot: der Windows-Server-Runner hat **keinen BMP-Vorschau-Anbieter** (Shell gab das Icon) → `Thumb { image, preview }` (erst `THUMBNAILONLY`, dann `ICONONLY`), Tests unterscheiden beide Fälle; PNG-Vorschau gibt es dort.

### M5.6d2 Bibliotheken ✅ (CI-Run #44) → **M5.6 virtuelle Orte komplett**
- `shell::libraries`/`library` (`IShellLibrary` über die `.library-ms`-Dateien in `FOLDERID_Libraries`): lokalisierter Name, Mitgliedsordner (nur Dateisystem), Datei fürs Icon.
- Ansicht „Bibliotheken“ (`::bibliotheken`, Aliase „Bibliotheken“, `shell:Libraries`, CLSID): Zeilen mit echtem Bibliotheks-Icon und „N Ordner“; Bibliothek mit **einem** Ordner öffnet ihn direkt, mit mehreren → `::bibliothek:<Stamm>` listet ihre Ordner (Spalte „Ordner“ = übergeordneter Pfad).
- `shell:DocumentsLibrary`/`MusicLibrary`/`PicturesLibrary`/`VideosLibrary`/`CameraRoll`/`SavedPicturesLibrary` → Bibliothek; Hoch aus einer Bibliothek → „Bibliotheken“; Breadcrumb „Bibliotheken › Name“; Sitzung merkt sich Bibliotheks-Tabs.
- Schutz: alle Platzhalter (`::…`) laufen über `places::is_placeholder` → keine Dateioperation, kein Watcher.
- Bewusst noch nicht: zusammengeführte Ansicht (Explorer zeigt Inhalte aller Ordner gemischt) → eigener TODO.
- Tests: Unit (Platzhalter, Namen, `shell:`-Bibliotheken), Windows-Unit `libraries_read_without_crashing`, Linux-E2E [61]/[61b], Windows-E2E W23–W27 mit eigener Test-Bibliothek (2 Ordner).

### M5.6d1 Adresszeile + echte Shell-Icons ✅ (CI-Run #43)
- `core::address` (rein, getestet): `%VAR%` (unbekannte bleiben stehen, `100%` bleibt), `shell:<Name>` mit Unterpfad (`shell:Downloads\\Fotos`), Windows-Kanonnamen + Aliase (`Personal` = Dokumente), unbekannter Name → klare Fehlermeldung; `~` nur unter Unix. Existierende Pfade werden nie umgedeutet (Ordner namens `%x%` bleibt erreichbar).
- `fs::places::known_folder_path`: Windows `SHGetKnownFolderPath` (32 Ordner), Linux XDG-Gegenstücke wo vorhanden.
- Icons: `IconKey::Shell` (Parsing-Name → `SHParseDisplayName` + `SHGFI_PIDL`) für Papierkorb/Dieser PC; Seitenleiste zeigt echte Icons (Ordner mit desktop.ini-Icon, Laufwerke, Papierkorb), „Dieser PC“-Zeilen echte Laufwerks-Icons; ohne Shell bleibt das Ersatz-Quadrat.
- CI: reine Doku-Commits (`docs/**`, `*.md`) lösen keinen Lauf mehr aus.
- Tests: Unit (Adress-Parsing, Windows: Shell-Icons Papierkorb/PC/C:\\), Linux-E2E [60]–[60c], Windows-E2E W20–W22 (`%TEMP%` mit Langnamen, `shell:Downloads`, unbekannter Name).

### M5.6c Netzwerk ✅ (CI-Run #41, Windows-E2E W18–W19c mit echter SMB-Freigabe grün)
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
