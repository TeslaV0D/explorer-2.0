# Zwischenstand

**Stand:** 2026-11-03 · **Phase:** 1 · M0 ✅ · M1 ✅ · **M1.5 (Renderer-Wahl, Einstellungen, Icon) lokal ✅, CI ausstehend** · danach M2 · **Branch:** `claude/relaxed-pasteur-ufhh1p`

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
1. CI-Umgebungsmatrix (Windows ohne GPU, Mesa-GL, forciert, Setting aus; Linux-GL; Exe-Icon) grün → M1.5 ✅.
2. M2: UI-Shell (Spalten-Header mit Sortierung, Datei-Icons, Details-Ansicht, Tastaturnavigation).
3. Boss: Artefakt auf echtem PC mit GPU testen (Einstellungen → Statuszeile zeigt GPU-Namen).
