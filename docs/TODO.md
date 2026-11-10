# TO-DO

Legend: `[ ]` open · `[~]` in progress · `[x]` done · priority P0/P1/P2

The app UI is German; UI labels are quoted as they appear (with a translation where it helps).

## Now (planning)
- [x] Create project structure + doc files (CLAUDE.md, PROJECT, PLAN-BASICS, WINDOWS-INTEGRATION, TODO, STATUS)
- [x] References researched (File Pilot, Sweetch, Everything)
- [x] D-001 decided: Rust + Slint
- [ ] Have Boss sign off / adjust the Phase 1 plan (continues implicitly, adjustments any time)

## Phase 1 – Basics (details: `PLAN-BASICS.md`)
- [x] M0 Repo setup: Cargo workspace, CI (Windows+Linux, fmt/clippy/test, release build + exe smoke test), skeleton window, Windows manifest (PerMonitorV2, longPathAware, UTF-8) – **completed 2026-11-03, CI run #3 green**
- [x] M1 fs core – **completed 2026-11-03, CI run #10 fully green**
  - [x] Windows fast path `FindFirstFileExW` (`FindExInfoBasic` + `LARGE_FETCH`), attributes from the find data (hidden/system/readonly/link), cloud placeholder ≠ link
  - [x] Long paths via `\?\` verbatim (including UNC, `.`/`..` normalisation), FILETIME→Unix
  - [x] Portable path (std) for Linux/tests, symlinks/dangling links
  - [x] Background loader: batches (256 first, then 8192), cancellation, error classes (NotFound/access denied/not a folder)
  - [x] Watcher (`notify`: ReadDirectoryChangesW/inotify) + debounce (100 ms quiet / max 500 ms) + adaptive refresh throttle (≥500 ms, 4× load time, max 10 s)
  - [x] Explorer visibility rules (hidden items / protected system files) + toggle “Ausgeblendete” (hidden items)
  - [x] Natural sorting allocation-free (100k: 173 → 47 ms), deterministic for “equal” names
  - [x] UI: never loads on the UI thread, generation counter against stale results, status bar, error display, fallback to the parent folder when the current folder disappears
  - [x] Benchmark tool (`cargo run --release -p explorer-fs --example bench -- --assert`) + CI budgets
  - [x] E2E test under Xvfb (13 checks): `scripts/e2e.sh`, runs as CI job `e2e-linux`
  - [x] CI green including Windows benchmark + Windows UI smoke test (run #6)
  - [x] CI green with separate start/data measurement (run #9: data 0.9 ms / 140 ms on Windows)
  - [x] CI green with a stable bench gate (best-of-9, run #10: median 1.17 ms / 115 ms, OS stalls 2/9 absorbed)
- [x] M1.5 Renderer choice + settings + icon – **completed 2026-11-03, CI run #13 green**
  - [x] GPU probe (WGL) + classification (software GL = unusable), cache in settings.ini, background re-probe
  - [x] Setting “Hardwarebeschleunigung verwenden, wenn verfügbar” (use hardware acceleration when available) (panel “Einstellungen”), takes effect on the next start
  - [x] settings.ini (atomic, tolerant), “Ausgeblendete” is saved
  - [x] App icon (SVG → ICO/PNG), exe icon + window icon
  - [x] Linux locally: 6 environments (no GL, Mesa GL, forced GPU/software, setting off) + E2E 20/20
  - [x] CI: Windows environment matrix (7 scenarios) + Linux GL check + exe icon check green (run #13)
- [x] M2 UI shell – **completed 2026-11-03, CI run #16 green**
  - [x] Details view: columns name / modified date / type / size, click sorts (▲▼, 2nd click reverses), column widths draggable, sorting saved
  - [x] Type column (“Dateiordner”, “PNG-Datei”, “Datei”) + sorting by type (real shell type names in M5)
  - [x] Selection like Explorer: click, Ctrl+click, Shift+click, click into empty space clears; selection survives auto-refresh/re-sorting (by name)
  - [x] Keyboard: arrows, PgUp/PgDn, Home/End (+Shift/Ctrl), Ctrl+A, Ctrl+Space, Enter, Backspace/Alt+↑, F5, Ctrl+L/Alt+D, type-ahead (including letter cycling)
  - [x] Open: double-click/Enter → navigate folders, files with the default app (ShellExecuteW), multiple files (max. 20)
  - [x] Breadcrumb address bar (clickable segments, “…” truncation, click into empty space/Ctrl+L = edit, Esc = back)
  - [x] After “Up” the folder we came from is selected; the list has keyboard focus at start
  - [x] CI green (including Windows ShellExecute test, run #16)
- [ ] M2 rest/polish (later): save column widths, show/hide columns, views list/icons/tiles (plan 4.2), grouping
- [x] M3 Tabs, split view, sidebar, history, session restore – **completed 2026-11-03, CI run #19 green**
  - [x] Tabs per pane: new (+ / Ctrl+T, same folder), close (× / middle-click / Ctrl+W, last tab stays), switch (click / Ctrl+Tab / Ctrl+Shift+Tab)
  - [x] Split view (button “Zwei Ansichten” (two views) / Ctrl+\), F6 switches pane, click activates, keyboard focus follows the active pane; closing the last tab on the right = split off
  - [x] History per tab: ← → buttons, Alt+←/→, mouse side buttons; “back to the parent folder” selects the folder we came from
  - [x] Sidebar: Quick access (Known Folders/XDG), drives with name, fill bar (red > 90 %), “x GB frei von y GB” (x GB free of y GB), refresh every 15 s in the background
  - [x] Session: panes, tabs, active tabs/pane, split in session.ini; restore on start without a path argument; only visible tabs load immediately (lazy); missing folders → nearest existing parent folder
  - [x] CI green (run #19)
- [ ] M3 rest/polish (later): reorder/tear off tabs by drag, draggable pane divider, sidebar: pin network/Recycle Bin/favourites, middle-click in sidebar = new tab, drive changes via WM_DEVICECHANGE instead of polling
- [x] M4 File operations – **completed 2026-11-03, CI run #26 green**
  - [x] Crate `explorer-ops`: jobs copy/move/Recycle Bin/delete permanently in the background (progress, pause, cancel), conflicts (replace/merge, skip, keep both, cancel, “for all”)
  - [x] Data safety: temp file + rename, when moving across drives delete the source only after a complete + flushed copy, “Replace” puts the old file into the Recycle Bin, cancelling leaves no partial files, errors per item (job keeps running)
  - [x] Undo/redo (Ctrl+Z/Ctrl+Y, max. 50): copy → Recycle Bin, move/rename → back, Recycle Bin → restore (snapshot before the undo), permanently deleted = not undoable
  - [x] Clipboard Ctrl+C/X/V (+ Ctrl/Shift+Ins), Windows-compatible (CF_HDROP + Preferred DropEffect, exchange with Explorer), cut greys out entries, Esc clears, a cut is pasted only once
  - [x] Del → Recycle Bin, Shift+Del → confirmation dialog → permanent
  - [x] F2 inline rename (name without extension selected), Esc cancels, invalid/existing → error message + editor stays open; Ctrl+Shift+N “Neuer Ordner” (new folder) + immediate rename
  - [x] Command bar (New folder, Cut, Copy, Paste, Rename, Delete, ↶, ↷)
  - [x] Operations panel (from 300 ms runtime): title, bar, “x of y · MB/s · n s left”, pause/resume, cancel
  - [x] Drag & drop in the app: onto a folder row = into it, onto the list = into the displayed folder, default move, Ctrl = copy; never into itself/a subfolder; multi-selection survives dragging
  - [x] Tests: ops 18 integration tests (+ disk full via tmpfs in CI, locked file on Windows), E2E 38–52 (40 checks), CI clipboard interop with Windows PowerShell
  - [x] CI green (run #26: E2E 100/100, Windows ops 19/19 including Recycle Bin + locked file, clipboard interop with Windows, disk full)
- [ ] M4 rest/polish (later): DnD with other apps (OLE, M5), default action per drive (same drive = move, other = copy, like Explorer), job queue (sequential instead of parallel, per drive), conflict dialog with thumbnails, “Copy to…”/“Move to…”, progress in the taskbar (ITaskbarList3), Recycle Bin view, handle junctions/hardlinks cleanly when copying, alternate data streams/ACLs (CopyFileExW already copies ADS)
- [x] M5 Shell integration (Boss approval 2026-11-04) – **complete: M5.0–M5.5 (run #37), M5.6 (runs #39–#44), M5.7 (run #48)** – in sub-steps, each CI-checked individually:
  - [x] M5.0 Exe size: `opt-level="s"` (**Windows 19.7 → 16.5 MB**, Linux 22.9 → 19.6 MB; sorting 100k +6–10 ms, the rest within noise), CI gate < 20 MB
  - [x] M5.1 Real icons: shell icons (`SHGetFileInfoW`/`IShellItemImageFactory`), cache per extension + individual for .exe/.lnk/.ico/.url, in the background, overlays (shortcut) – Linux: fallback as before
  - [x] M5.2 Shell type names (type sorting by displayed name) (“Textdokument”, “JPEG-Bild”) via `SHGetFileInfoW(SHGFI_TYPENAME|USEFILEATTRIBUTES)`, cached per extension
  - [x] M5.3 Context menu (Linux: own fallback menu later): `IContextMenu` (including third-party extensions, “Send to”, “Open with” via IContextMenu2/3), right-click on selection / empty space (folder background), own entries before them (copy/paste/… through our engine)
  - [x] M5.4 Properties (`SHObjectProperties`/`SHMultiFileProperties`, Alt+Enter; “Open with” comes through the shell menu)
  - [x] M5.5 OLE drag & drop with other apps (Windows E2E drags with a real mouse): drag out of the app (`DoDragDrop` + `SHCreateDataObject`), drop into the app (`RegisterDragDrop`, replace the winit handler), default action per drive like Explorer
  - [x] M5.6 Virtual places (fast path stays for normal folders) – CI runs #39–#44:
    - [x] M5.6a Recycle Bin view: list (deleted on, original location, size), restore, delete permanently, empty, dropping onto the Recycle Bin = delete, sidebar, address aliases (CI run #39)
    - [x] M5.6b “Dieser PC” (This PC): drives with storage space, click on the heading in the sidebar, “Up” from a drive → This PC (CI run #39)
    - [x] M5.6c Network: `\\server` → shares (`NetShareEnum`, without admin shares `C$`/`IPC$`), comment in “Type”, login dialog (`WNetAddConnection3W`) on access denied – once per folder, “Up” from a share → server, breadcrumb `\\nas › film` (CI run #41)
    - [x] M5.6d1 Address bar like Explorer: `%VAR%`, `shell:<name>` (32 Known Folders, e.g. `shell:Downloads\\x`, `shell:Common Startup`), `~` (Unix); real shell icons in the sidebar (folders, drives, Recycle Bin) and “This PC” (CI run #43)
    - [x] M5.6d2 Libraries (`IShellLibrary`): view “Bibliotheken” (Libraries), a library with one folder opens it, with several shows its folders; `shell:Libraries`/`shell:DocumentsLibrary` … (CI run #44)
    - [ ] Later: merged library view (contents of all folders in one list, like Explorer) – needs file operations with a path per row
    - [ ] Later: undo for “Restore”, shell context menu in the Recycle Bin
  - [x] M5.7 Tile view + thumbnails (Boss: route 1) – CI run #48
    - [x] M5.7a Thumbnails: `IShellItemImageFactory` (Windows thumbnail cache, otherwise large icon), 2 workers, only visible tiles, newest first, 96 MB memory limit (LRU), key including modification time
    - [x] M5.7b Tile view: medium/large/extra large (48/96/256 px), Ctrl+Shift+1/2/3/6, Ctrl+mouse wheel (throttled), buttons “Details/Symbole” in the status bar, grid navigation (←→↑↓, PgUp/PgDn), click/Ctrl/Shift, double-click, context menu, F2 over the label, drag & drop (also OLE, hit test in the grid), setting `view` saved
    - [ ] Later: replace column headers in the tile view with a sort menu; rubber-band selection (lasso) for both views; remember the view per folder
- [x] M6 Folder-options parity, themes, command palette, Inspector (Boss: order M6.1 → M6.4) – complete (CI run #58)
  - [x] M6.1 Folder options like in Explorer (CI runs #49–#51)
    - [x] M6.1a Settings panel “Ordneroptionen” (folder options): show file name extensions (off = hide the extension of known types, Windows: registered in HKCR; F2 then edits only the stem, the extension stays), show protected system files, open with a single click (select by hovering, double-click protection) – CI run #49
    - [x] M6.1b Sort menu “Sortieren ▾” in the status bar (column keeps its direction, direction separate), tile view without column headers, rubber-band selection on free space in both views (Ctrl = add) – CI run #51
    - [ ] Later: rubber-band selection with auto-scroll at the edge; also start the rubber band from the free part of a details row (Explorer)
    - [x] M6.1c Remember the view per folder (`views.ini`, max. 5000 folders, oldest drop out; Windows case-insensitive) + “Aktuelle Ansicht für alle Ordner” (current view for all folders); view per tab → two panes can look different – CI run #51
  - [x] M6.2 Inspector (Alt+P / “Details”): preview (shell image 256 px, text first 40 lines), name/type/size/modified/created/attributes/path (selectable and copyable), image dimensions from the file header (PNG/JPEG/GIF/BMP/WebP), multi-selection with totals, folder size in the background (cancellable, no links, max. 500,000 entries, never whole drives; open folder only its direct contents) – CI run #52
    - [ ] Later: adjustable Inspector width; metadata from shell properties (author, length of videos/audio, EXIF)
  - [x] M6.3 Command palette (Ctrl+Shift+P): central command directory (`core::commands`, 45 commands with shortcut + condition), fuzzy search (umlauts/case don't matter, word starts count), unavailable commands greyed out, recently used on top (settings.ini), “>” = places (Quick access, drives, virtual places, recently visited folders) – CI run #55
    - [ ] Later: freely assignable keyboard shortcuts based on the command directory
    - [x] First-start gate: warning from 1 s, red from 2 s (Boss: variant 1, D-018)
  - [x] M6.4 Colour schemes: Like Windows (live) / Light / Dark, Windows accent colour (selection family in the hue of the accent, frames in the Windows shade), all colours central + contrast-tested, std widgets + title bar follow – CI run #58
    - [ ] Later: respect Windows contrast themes (High Contrast); accent check marks for std widgets too (Slint colours them itself from `DwmGetColorizationColor`)
- [x] **R-001 One-button reset** for all integrations (app button + standalone recovery tool + journal + CI snapshot test) – prerequisite before any integration ships
  - [x] R-001a Crate `explorer-integrate`: write-ahead journal (atomic, mirror, checksum), `apply`/`reset`/`sweep`/`status`, raw registry values, files with backup, environment + WM_SETTINGCHANGE, values changed by others stay, errors stay in the journal; 13 unit tests (fake registry) + test against the real registry – CI run #59
  - [x] R-001b `explorer2-recover.exe` (console, `--status`/`--yes`) + mandatory CI test `scripts/r001-snapshot.ps1` (reg.exe export before = after: from the journal, via the mirror, entirely without a journal) – CI run #59
  - [x] R-001c Settings → “Windows-Integration” (status + “Alle Windows-Integrationen zurücksetzen” (reset all Windows integrations) with confirmation) + palette; Linux E2E [71]–[71g] (188/188), Windows E2E W41–W41h – CI run #61
  - [x] Start menu entry “Windows-Standard wiederherstellen” (restore Windows default) + uninstall calls reset (M8.1, run #69)
  - [ ] Later: `.reg`/PowerShell fallback script, self-healing for the explorer.exe proxy, HKLM journal copy in `%ProgramData%`
- [x] M7 Windows integration stages 1–2 (stage 3 postponed, D-022) (see `WINDOWS-INTEGRATION.md`) + verification points on real Windows
  - [x] M7.1 Open folders (own default verb `explorer2open` for Directory/Drive, `Folder` stays with Windows) + “In Explorer 2.0 öffnen” (open in Explorer 2.0) in the context menu (folder, drive, folder background); individually switchable (settings + palette), off = exactly the previous state; exe moved → relinked on start; emergency tool copy in `%LOCALAPPDATA%\Explorer2`; `"C:\"` quoting fixed; Windows E2E W42 (real ShellExecute), R-001 test [R5] – CI run #64
  - [x] M7.2 Win+E: `HKCU\…\CLSID\{52205FD8-…}\shell\opennewwindow\command` = exe (without folder = last session) + empty `DelegateExecute`; switch + palette; Windows E2E W43 (real Win+E via `keybd_event`, checking beforehand whether the runner shell processes Win+E at all) – CI run #67
  - [ ] M7.3 `explorer.exe` invocations (IFEO proxy, HKLM/admin, high risk) – **postponed** (Boss, D-022), only after the field test of M7.1/7.2
  - [ ] Check on real Windows 11 (Boss): double-click on a folder/drive on the desktop, “Open folder” from browser downloads, context menu (Win11: under “Show more options”)
- [~] M8 Installer, recovery tool, hardening, beta (Boss: own installer in Rust, D-023)
  - [x] M8.1 Installer core: `explorer2.exe --install` (per user, without admin), Start menu (app + “Windows-Standard wiederherstellen”), Apps & Features, redirect integrations to the installed exe, update over the running app, `explorer2-recover --uninstall [--quiet]` (reset + program folder via a temp copy, protection against foreign folders); CI `scripts/m8-install.ps1` (real default paths, start via the Start menu, uninstall via `QuietUninstallString` → everything as before) – CI run #69
  - [x] M8.2 Settings → “Installation” (portable/installed, install/update with confirmation, then restart from the installation folder) + palette; Linux E2E [72] – CI run #69
  - [x] M8.3 Hardening (Boss, 2026-11-05) – CI runs #70–#73
    - [x] M8.3a Crash report (panic of any thread + Windows exceptions → `crash/crash-<utc>-<pid>.txt` with cause, location, last 200 log lines) + notice on the next start (“Bericht anzeigen” – show report); start guard: 2 aborted starts in a row → safe start (software rendering, no session, no GPU check), stable after 5 s = back to normal; a GPU check that takes the process down → GPU permanently “unsuitable”; SIGTERM does not count as a crash; Linux E2E [73]–[73j], Windows `scripts/m8-crash.ps1` [C1]–[C8] – CI run #70
    - [x] M8.3b Single instance: a second start (folder double-click, Win+E, Start menu) opens a tab in the running window, a minimised window comes to the front; named pipe (Windows, local only, per session) / Unix socket 0600; reply only once the tab is open – a hanging instance → own window after 4 s; `--new-instance` + switch “Ordner im offenen Fenster öffnen” (open folders in the open window) (settings/palette); installer restart with `--new-instance`; Linux E2E [74]–[74i], Windows `scripts/m8-instance.ps1` [N1]–[N9] – CI run #71
    - [x] M8.3c Random-data tests for all parsers/text helpers (`core/tests/fuzz.rs`, `integrate/tests/fuzz.rs`: no panic, save+load = equal, every truncated/modified journal is detected; `FUZZ_ROUNDS`/`FUZZ_SEED`) – 3 findings fixed; endurance test Linux `scripts/e2e-stress.sh` [75]–[75e] (RSS/files/threads) + Windows `scripts/m8-stress.ps1` [S1]–[S6] (private bytes/handles/threads/GDI/USER, two measuring phases, image cache factored out) – CI runs #72/#73
    - [ ] Optimisation (later): the tile image cache holds one image per file for files **without** their own preview (shell icon only) → share per extension (Windows endurance test: 50 MiB cache with 2400 empty files; within budget, but unnecessary)
  - [~] M8.4 Release package (Boss, 2026-11-08): `Explorer2-Setup-<v>-x64.exe` (one file, contains app + recovery, calls `--install`; confirmation → install → “Jetzt starten?” (start now?), `--quiet` for scripts), portable ZIP, `SHA256SUMS.txt`; `scripts/release-package.ps1` (build the setup + smoke test [S1]–[S9]: silent install into temp paths, files = build, version, Apps & Features, Start menu, again = update, uninstall cleans everything up) runs in every CI run; `.github/workflows/release.yml`: new version (without release) pushed or tag `v*` → tag + GitHub release; version 0.1.0
    - [x] First release v0.1.0 published (release run #1 after CI #134 green, setup smoke test 9/9): https://github.com/TeslaV0D/explorer-2.0/releases/tag/v0.1.0
    - [ ] Installed by Boss on real Windows via the setup (SmartScreen, Start menu, uninstall)
  - [ ] Later: code signing (SmartScreen), automatic updates, `.reg`/PowerShell fallback, desktop shortcut (optional)

## Technical debt / follow-ups
- [x] Exe size: 19.7 → **16.5 MB** (run #29, `opt-level="s"`, CI gate < 20 MB). Further ideas if needed: – check before M5: `opt-level="z"`/`lto`/`strip`, unnecessary Slint features (Wayland?), Skia vs. femtovg; previously 17.2 MB (budget < 20 MB ok, ideal < 10 MB missed) → check in M2: Skia vs. femtovg/software renderer, `opt-level="z"` comparison
- [x] Renderer decision: option 2 (GPU probe) + setting (D-009)
- [ ] Measure `NtQueryDirectoryFile` as an alternative to FindFirstFileExW (only if the Windows bench gets tight)
- [ ] Raise GitHub Actions to Node 24 versions (checkout@v4 → v5 etc., deprecation warning in CI)

- [ ] Real hardware GPU probe only verifiable by a Boss test (CI has no GPU) → test the artifact on your own PC: settings panel shows “GPU-Beschleunigung (<GPU name>)”
- [ ] Linux: no GPU probe (platform default) – dev platform only, ok

## Phase 2 – System-wide search (Everything style) – **in progress** (Boss 2026-11-05, plan: `docs/PLAN-SEARCH.md`)
- [x] S1 without admin (D-024) – complete (CI run #93), next step: field test (`docs/FIELD-TEST-SEARCH.md`)
  - [x] S1.1 Index core `explorer-index` (82 MB / 1 M, name queries ≤ 5.1 ms Windows CI, ≤ 3.2 ms Linux CI; CI run #78): compact structure (name arena, parent chain), insert/delete/rename, query language (AND/OR/NOT, phrase, `*?`, `ext:` `size:` `dm:` `file:` `folder:` `path:` `regex:`), parallel search, sorting; benchmark 1 M entries < 10 ms, < 100 MB
  - [x] S1.2 Persistence per drive (version, volume ID, checksum, atomic, every corruption detected = rebuild, never a crash; 1 M: file 69 MB, load 116 ms, save 179 ms; CI gate load < 1 s) + exclusion lists (`Exclusions`: paths + name patterns, default exclusions, setting `index_exclude`) – CI run #80 (load Linux 88 ms, Windows 69 ms)
  - [x] S1.3 `explorer-search`: parallel build, reconciliation on start via folder modification times (in slices), live updates (recursive watcher, batches, overflow → reconciliation), service per drive (generations, reordering under the read lock, only save exact states), volume ID; CI example `drive` (measuring runs `C:\` cold/warm/priority, gate `D:\`) – CI run #83: cold VM disk 5k/s, warm 207k/s, live 204 ms
    - [x] Network drives/USB: optional + switchable (S1.5c, drive selection)
    - [x] Linux: show the inotify limit (`max_user_watches`) in the index status (S1.5a: understandable text)
    - [x] Count watcher errors/overflows + show the last reason in the status (S1.5a)
  - [x] S1.4 UI (CI run #90)
    - [x] S1.4a Service in the app: starts 3 s after the window in the background, drives = fixed local ones (Linux `$HOME`), lock file `index/lock` (a second window does not index), save on exit, status + on/off + “Neu abgleichen” (reconcile again) in the settings (panel scrolls on small screens), palette `search-rescan`/`toggle-index`, overrides `EXPLORER2_INDEX=off`/`EXPLORER2_INDEX_ROOTS`/`EXPLORER2_INDEX_DELAY_MS`, E2E [76] – CI #90
    - [x] S1.4b Search field per tab (Ctrl+F/Ctrl+E/F3, from the current folder, placeholder `::suche:`, hits with full paths = all file functions, column “Ordner” (folder) relative, max. 20,000 shown, without an index walk the folder, hits follow the index, Esc/empty field = back without a trace in the history), E2E [76i], [77]–[77k], Windows [W44] – CI #89/#90
    - [x] Local time zone for date display **and** `dm:` queries (S1.5a, daylight saving per date)
    - [x] S1.4c Global quick search (Ctrl+Shift+F + palette “Alle Laufwerke durchsuchen” (search all drives)): overlay in the style of the palette, best 50 (exact name → name prefix → shorter), Enter opens, Ctrl/Shift+Enter shows it selected in its folder, footer with index scope, E2E [78], Windows [W45] – CI #89/#90
  - [x] S1.5 Polish (CI runs #91/#93)
    - [x] S1.5a Local time (display + `dm:`), missed watcher events in the status – CI #91
    - [x] S1.5b Hits marked bold, sorting by column header in hits (tested), search history (↑/↓ in the search field, list in the quick search), E2E [79] – CI #91
    - [~] S1.5c Drive selection in the settings (automatic = fixed local drives; network/USB/Linux `/` can be added; DVD never), restart of the service, E2E [80] – CI #93
- [x] **Field test S1 on real Windows (Boss, 2026-11-07): “it works”** – S2 not needed for now
- [ ] S2 optional with admin: MFT/USN service – only after the field test of S1 + R-001 extension for services
- [ ] Later: content search, system-wide hotkey (integration → R-001)

## Redesign (Boss 2026-11-07) – plan `docs/PLAN-REDESIGN.md`, design/UX only, no logic
- [x] Decisions 1–5 from the plan (Boss 2026-11-07: “passt so” = all recommendations)
- [x] R0 Safeguards (CI run #101): UI dump `EXPLORER2_UI_DUMP` (item tree via `i-slint-core`), test helpers `scripts/ui-lib.sh` + `UiBox`/`UiClick`/`ListTop` in `e2e-windows.ps1`, all panel/bar clicks by name, rows relative to the column header, meaningful names for icon buttons, gallery `scripts/gallery.sh` (22 screens Linux) + Windows gallery (10 screens) as CI artifacts (`ui-gallery-linux` 22, `ui-gallery-windows` 10)
- [x] R1 Design system (CI run #104): colour roles `core::theme::Design` + contrast test (12 accents × light/dark), global `Ui`, 50 Lucide icons (`scripts/make-ui-icons.py`), building blocks `ui/kit.slint`, UI kit `EXPLORER2_UI_KIT=1` (gallery + E2E [81])
- [x] R2 Main layout (CI run #110): global bar dissolved (actions in the navigation row of the active view, ⋮ menu, status bar), tooltips, breakpoints, dark as default
- [x] R3 Sidebar (CI run #115): line icons, accent mark, subtle drive bars
- [x] R4 Navigation (CI run #120): icon buttons, breadcrumb segments, search field with magnifier, 34 px
- [x] R5 Tabs (CI run #123): IDE style, accent stroke on the current tab of the active view, × on hover
- [x] R6 File list (CI run #124): header 24 px with chevron, row tiles with accent bar, focus/drop, tiles, rubber-band selection
- [x] R7 Inspector (CI run #125): groups, copy path, fade-in
- [x] R8 Menus + dialogs (CI run #128): own context menu + “Weitere Optionen” (more options; native menu dark), dialogs/palette/quick search/settings/jobs/notice/status bar; installer closes only after the installed copy started successfully
- [x] R9 States + motion (CI run #130): pressed/hover/appear uniform, pixel checks of the states
- [x] R10 Consistency + QA (CI run #131): old tokens gone, scaling 125/150 % (E2E [83]), gallery before/after, budgets, checklist → **redesign R0–R10 complete**
- [ ] Field test redesign on real Windows (Boss, `docs/FIELD-TEST-REDESIGN.md`)
- [ ] Watch the exe size (19.31 MB after the redesign, gate 20 MB): when it gets close, measure the causes (Slint codegen per component), make nothing worse (D-015)
- [ ] Watch E2E flakes: [49] first drag after the full run ([72c] fixed: installer waits for the running copy, CI #128)

## Boss wishes (2026-11-08) – for later, not planned yet
- [~] **Localisation (i18n)**: translate the UI into further languages (supersedes D-007, e.g. Slint `@tr` + Rust texts) – plan `docs/PLAN-I18N.md` (2026-11-10), waiting for Boss decisions
  - [ ] Hit meanings exactly, not confusing: terms as in Windows Explorer of the respective language (Recycle Bin, This PC, Quick access, Properties …), glossary per language
  - [ ] Layout robustness when switching: check **all** texts for whether they fit into buttons/menus/columns/dialogs/settings (German/French/Russian long, CJK different font) – shorten, wrap or make widths dynamic; gallery per language + pixel/overflow check in E2E
  - [ ] Language switchable in the settings (default: Windows language), live or with a restart notice
  - [ ] Numbers/dates/sizes according to the locale; mind font coverage (gotcha: missing glyphs cost start time)
- [ ] **“New” submenu like Windows**: not only folders but all creation options (text document, shortcut, zipped folder, Word/Excel/… from `HKCR\.<ext>\ShellNew` – `NullFile`/`FileName`/`Data`/`Command`), name directly in rename mode, undo like for New folder
- [ ] **Autostart with Windows** (opt-in, switch in settings + palette)
  - [ ] Entry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (without admin), **only via `integrate::apply`** (journal) + entry in `manifest::points()` → reset/recovery/uninstall remove it (R-001, snapshot test)
  - [ ] Start in the background (`--background`: no window, instance server + search index warm) → Win+E/open folder shows a window immediately via the single-instance hand-over; closing the window = **keep running in the background + tray icon** (Boss 2026-11-08)
  - [ ] Tray icon (`Shell_NotifyIconW`): click = open window, menu = new window / settings / exit; keep idle memory small (free the folder data of closed windows), endurance test for leaks
  - [ ] Mind the start guard (a background start must not count as a crash), measure start costs at Windows login (must not slow down the login), redirect the exe path after update/installation like in M7
- [ ] **“Open in Explorer” system-wide → Explorer 2.0** (Boss 2026-11-08, example: Steam “Browse local files” still opens Windows Explorer)
  - [ ] First measure **how** the affected apps open it (Process Monitor / API trace: Steam, browser downloads “Show in folder”, VS Code, Discord, installers …) and keep a list of the routes
  - [ ] Route 1 – `ShellExecute` with an explicit verb `open`/`explore` (bypasses our default verb `explorer2open`, D-021): HKCU override of `Directory`/`Drive` `shell\open` + `shell\explore` (without admin), only via `integrate::apply` + `manifest`, individually switchable
  - [ ] Route 2 – direct call `explorer.exe <path>` or `explorer.exe /select,<file>`: only via IFEO redirection (= M7.3, postponed per D-022) – understand `/select,` (open folder + select file), `/e,`, `/root,`, `shell:` paths and virtual folders or pass them on to the real Explorer
  - [ ] Route 3 – `SHOpenFolderAndSelectItems` (goes through the running Explorer shell): check feasibility (ShellWindows/DDE), otherwise document as a known gap
  - [ ] Never break anything: `Folder` (virtual folders, Control Panel, Recycle Bin …) stays with Windows Explorer, fallback to the real Explorer when Explorer 2.0 is missing/crashes; R-001 snapshot test for all new values; Windows E2E per route
- [ ] **Emergency: Shift + right-click opens the native Windows context menu directly** (instead of our own; including extended verbs like “Copy as path”, Windows behaviour with Shift = extended verbs `CMF_EXTENDEDVERBS`)

## Documentation in English (Boss 2026-11-09) – planned start Thu 12.11. 16:00, brought forward by Boss, done Tue 10.11.
- [x] Rewrite all docs on GitHub in English (done 2026-11-10): `docs/*.md` (PROJECT, PLAN-*, WINDOWS-INTEGRATION, FIELD-TEST-* (formerly PRAXISTEST-*), STATUS, TODO), `THIRD-PARTY.md`, `scripts/release-notes.md` **and `CLAUDE.md`** (Boss 2026-11-09) – content 1:1, no logic change; D-004 changed accordingly
- [x] Complete the README (done 2026-11-10, screenshots in `docs/screenshots/` from the Linux gallery): what Explorer 2.0 is, features (panes/tabs, search, Inspector, palette, colour schemes), screenshots from the gallery, download/installation (setup/portable, SmartScreen notice), emergency exit/reset (R-001), building from source, tests/CI, project structure, license

## Phase 3 – File action shortcuts (Sweetch style)  *(later, described only)*
- [ ] Tray helper with radial menu: drag files onto a sector or hotkey (Sweetch: Ctrl+Alt+Space)
- [ ] Modifier groups: Shift = convert, Ctrl = tools, Alt = actions
- [ ] Actions: convert (image/audio/video/PDF/doc via ffmpeg/pandoc – optionally downloadable), archive, copy to…, rename, own scripts
- [ ] Configurable sectors/profiles, local processing (no cloud)
- [ ] Clarify the license: Sweetch is MIT → use of ideas/code with attribution possible

## Phase 4 – Later
- [ ] Plugins/scripting, settings sync, auto-update, ARM64, code signing
