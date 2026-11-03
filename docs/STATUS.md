# Zwischenstand

**Stand:** 2026-11-03 · **Phase:** 1 · M0 ✅ · **M1 lokal ✅, Windows-CI ausstehend** · **Branch:** `claude/relaxed-pasteur-ufhh1p`

## M1 – was gebaut wurde
- `explorer-fs`: Windows-Fastpath (`FindFirstFileExW` + Large Fetch), portabler std-Pfad, Verbatim-Langpfade, Hintergrund-Loader mit Batches/Abbruch, Watcher mit Debounce.
- `explorer-core`: Entry mit hidden/system/readonly/is_link, `ViewFilter` (Explorer-Regeln), schnellere natürliche Sortierung, `format_count`.
- `explorer2`: `nav.rs`-Controller – UI-Thread macht kein I/O, Generationszähler, Auto-Refresh mit adaptiver Drossel, Statusleiste, Fehleranzeige, Rückfall bei gelöschtem Ordner, Toggle „Ausgeblendete“.

## Messwerte (Linux-Sandbox, Release, 100 000 Einträge)
| Metrik | Ergebnis | Budget |
|---|---|---|
| Erster Batch (fs-Bench) | ~6 ms | < 100 ms |
| Voll laden + sortieren (fs-Bench) | ~250 ms | < 1 s |
| Sortierung allein | 47 ms (vorher 173 ms) | – |
| UI: erster Paint (E2E-Trace) | 10–13 ms | < 100 ms |
| UI: komplett angezeigt | 237–326 ms | < 1 s |

## Tests
- Unit/Integration: 31 Tests (core 7, fs 20, app 4) – grün; fs-Windows-Code per `clippy --target x86_64-pc-windows-msvc` sauber.
- E2E unter Xvfb (13 Checks, alle PASS): 100k-Ordner, Auto-Refresh, gelöschter Ordner → Elternordner, ungültiger Pfad → Fehler + Adresszeile zurückgesetzt, Toggle Ausgeblendete, Hoch, Adresszeile folgt nach Tippen, Refresh-Drossel, kein Absturz/Panic.
  - Skript: `scripts/e2e.sh`, läuft in CI als Job `e2e-linux` (Screenshots als Artefakt bei Fehler).

## Bugs gefunden & gefixt in M1
1. **Refresh-Endlosschleife**: Datei, die ständig geschrieben wird (hier: eigenes Trace-Log), löste alle ~100 ms einen kompletten Reload aus → adaptive Drossel (≥ 500 ms, 4× letzte Ladezeit, max 10 s). Gemessen: 30 → 6 Refreshes / 3 s.
2. **Adresszeile eingefroren**: Einweg-Bindung `text: current-path` wird von Slint gelöscht, sobald der User tippt → danach nie mehr aktualisiert (auch nicht bei Hoch/Doppelklick). Fix: Zwei-Wege-Bindung `<=>`.
3. Statusleiste „1 Elemente“ → „1 Element“.
4. Sortierung zu langsam (173 ms / 100k) → allokationsfrei, 47 ms.

## Offen
- CI-Lauf mit Windows-Benchmark + UI-Smoke-Test (echtes Fenster, 100k-Ordner) abwarten → erst dann M1 ✅.

## Nächste Schritte
1. CI prüfen, ggf. fixen → M1 abschließen.
2. M2: UI-Shell (Spalten-Header mit Sortierung, Icons, Details-Ansicht ausbauen, Tastaturnavigation Grundlagen).
