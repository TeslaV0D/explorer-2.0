# Phase 2 – System-wide instant search (Everything style)

As of: 2026-11-05 · Boss decisions: **index in stages** (D-024), **search field per tab + global quick search** (D-025).

## Goals (measurable)
| Goal | Budget |
|---|---|
| Search-as-you-type over ~1 M entries | **< 10 ms** per query (best-of-N, CI benchmark with a synthetic index) |
| Memory for ~1 M entries | **< 100 MB** (names + tree + size + date) |
| Initial index without admin (directory walk, NTFS SSD) | 1 M entries **< 2 min**, in the background, app usable immediately |
| Start with a saved index | index loaded **< 1 s**, then reconciliation in the background |
| Live updates (file created/renamed/deleted) | visible **< 1 s** |

## Stage S1 – without admin (now)
- **S1.1 Index core** (new crate `explorer-index`, platform-neutral, fully testable on Linux):
  compact structure (names in one byte-arena buffer, node = parent index + name range + size + modification date + flags), path via the parent chain, lower-case copy of the names for fast matching, insert/delete/rename for live updates.
  **Query language** (Everything subset): words = AND, `|` = OR, `!` = NOT, `"…"` phrase, `*`/`?` wildcards, `ext:jpg;png`, `size:>10mb`, `dm:today|yesterday|2026-11-05|>2026-01`, `file:`/`folder:`, `path:` (match against the whole path), `regex:`. Parallel search over blocks (std threads), results sortable (name, path, size, date).
  Tests + random-data tests (parser) + benchmark with 1 M synthetic entries (CI gate).
- **S1.2 Persistence**: one binary file per drive in `%LOCALAPPDATA%\Explorer2\index\` (header with version + volume ID + checksum, written atomically, damaged = rebuild, never a crash), exclusion lists in the settings.
- **S1.3 Build + live**: parallel walk per drive (FindFirstFileExW Large Fetch, D-005; work queue over N threads, low priority), progress; live changes via recursive `ReadDirectoryChangesW` on the drive roots (buffer overflow → partial rescan of the folder); on start with a saved index, reconciliation via folder modification times (re-read only changed folders). Network/USB optional + can be switched off.
  - Implementation (`explorer-search`): service with one maintenance thread per drive. Writes only in slices ≤ 15 ms; reorder and save under the read lock. Ids are valid per generation. Only an exact state is saved (truncated change → the old file stays). Saved index → searchable first, then watcher, then reconciliation. Overflow → reconcile the drive with `since` = last exact moment.
- **S1.4 UI**: search field in every tab (Ctrl+F / F3) – searches from the current folder, hits as a virtual place in the tab with **all** file functions (open, copy, delete, rename, context menu, Inspector) and a “Folder” column; global quick search (Ctrl+Shift+F in the app, overlay like the palette) across all drives. Status “Index: n entries, up to date / building x %”.
- **S1.5 Polish**: hit highlighting, sorting via column header, search history, Linux E2E + Windows CI (real tree, live update, persistence, restart).

## Stage S2 – optional, with admin (later, separate decision)
NTFS MFT scan + USN journal via a small service (admin once, opt-in). Mandatory beforehand: R-001 extension for services (registration in the journal, recovery removes the service, also in Safe Mode). Only after the field test of S1.

## Not in Phase 2
Content search (full text), system-wide hotkey outside the app (would be an integration → R-001, later).
