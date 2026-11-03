# Zwischenstand

**Stand:** 2026-11-03 · **Phase:** 1 · M0 ✅ · **M1: Run #9 komplett grün; Run #10 (stabiler Bench-Gate) ausstehend** · **Branch:** `claude/relaxed-pasteur-ufhh1p`

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
- Run #10: Bench-Gate jetzt **best-of-9** (Fremdlast verlängert nur, verkürzt nie) + Median/Max/OS-Stalls ausgewiesen (D-008). Budgets unverändert (100 ms / 1 s). Grün abwarten → M1 ✅.
- Fix im Test selbst: E2E-Regex an neues `at=`-Feld angepasst (lokal 13/13 PASS).

## Nächste Schritte
1. CI prüfen, ggf. fixen → M1 abschließen.
2. M2: UI-Shell (Spalten-Header mit Sortierung, Icons, Details-Ansicht ausbauen, Tastaturnavigation Grundlagen).
