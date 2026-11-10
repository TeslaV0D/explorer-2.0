# Plan – Phase 1: basic features

Goal: a fast, reliable file manager that can replace Explorer in everyday use – **before** search and shortcuts arrive.

## 1. Guiding principles
- **The UI thread never does I/O.** Everything asynchronous, results stream in (first rows immediately, the rest loads afterwards).
- **Virtualised lists** (render/load icons only for visible rows).
- **Shell compatibility through real Windows APIs** (`IShellItem`, `IShellFolder`, `IFileOperation`, `IContextMenu`) instead of building our own – this way the Recycle Bin, Libraries, network, OneDrive placeholders and third-party extensions work “for free”.
- **Fast path for normal folders:** directly `NtQueryDirectoryFile`/`FindFirstFileExW` (`FIND_FIRST_EX_LARGE_FETCH`) instead of shell enumeration → speed; shell path only for virtual folders.
- **Long paths (`\\?\`, >260 characters)**, Unicode, per-monitor DPI v2 from day 1 (manifest).
- Core as its own crate (testable platform-neutrally), Windows-specific parts behind traits.

## 2. Architecture (proposal)
```
crates/
  core/        Model: paths, entries, sorting/filters, command system, settings (platform-neutral, unit tests on Linux)
  fs/          Directory enumeration, watcher (ReadDirectoryChangesW), file-ops queue
  winshell/    Windows adapter: IShellItem, icons/thumbnails, context menu, properties, Recycle Bin, drag & drop, clipboard
  ui/          User interface (framework: D-001 open)
  app/         Binary, single instance, argument parsing (path, /select,, --new-window)
  installer/   Registry integration + recovery (see WINDOWS-INTEGRATION.md)
```
Data flow: `app` → command (e.g. `Navigate(path)`) → `core` state → `fs` task (async, cancellable) → result stream → `ui` diff update.

## 3. UI framework – open decision D-001
| Option | Pro | Contra |
|---|---|---|
| **Rust + Slint** (recommendation) | Small native binary, fast start, GPU/software renderer, declarative UI | Own controls for fancy lists; accessibility still younger |
| Rust + egui | Very quick to build, immediate mode fits the File Pilot style | A11y/IME/native look & feel weaker, continuous-redraw discipline needed |
| C# + WinUI 3 | Native Fluent look, a11y, Win11 context menu easier | Bigger, slower start, virtualisation with 100k+ items tricky, runtime dependency |
| Own renderer (wgpu/Direct2D) | Maximum control like File Pilot | By far the highest effort (text, IME, a11y all on our own) |

## 4. Feature scope Phase 1 (priority P0 = must, P1 = should, P2 = could)

### 4.1 Navigation & layout
- P0 Address bar (breadcrumb **and** editable path with autocompletion), back/forward/up, history
- P0 Tabs (drag-out, restore on start), **split view** (2 panels)
- P0 Sidebar: Quick access/favourites, This PC, drives, network, Recycle Bin
- P1 GoTo (path jump à la File Pilot), command palette (Ctrl+Shift+P), path alias
- P1 Inspector panel (preview of text/image/folder size/properties)

### 4.2 File list
- P0 Views: details, list, icons (S/M/L/XL), tiles; columns selectable/sortable/remembering their width
- P0 Sort, group, filter (quick filter in the folder), natural sorting (“file2” < “file10”)
- P0 Multi-selection (mouse + keyboard, lasso, Ctrl/Shift, type-ahead “typing jumps”)
- P0 Real icons + overlay icons (shortcut, cloud status, shared), thumbnails async + cache
- P1 Flatten view (all subfolders flat), folder sizes on demand

### 4.3 File operations
- P0 Copy/move/delete (Recycle Bin + permanently)/rename/new (folder, file, shortcut)
- P0 Via `IFileOperation`: Recycle Bin, UAC elevation, conflict dialog, shell-conformant
- P0 Own **operations queue** with progress, pause/cancel, speed, error list, **undo/redo**
- P0 Clipboard (Ctrl+C/X/V compatible with Explorer, `CF_HDROP` + Preferred DropEffect)
- P0 Drag & drop internally + from/to other apps (move/copy/link, modifier keys)
- P1 Batch rename (patterns, numbering, date, regex), copy path (various formats), archives (read/write zip)
- P1 Hashes, create shortcuts/symlinks/junctions, change attributes

### 4.4 Context menu & shell
- P0 Context menu with **third-party shell extensions** (host the classic `IContextMenu`; Win11 entries via `IExplorerCommand`/registry verbs)
- P0 Properties dialog (`SHObjectProperties`), “Open with”, standard verbs (`ShellExecuteEx`)
- P1 Own searchable & pinnable context menu (File Pilot idea)

### 4.5 Settings (parity with “Folder Options”)
- P0 Hidden/system files, show extensions, protected system files, full path in title
- P0 Single/double click, checkboxes, details pane/preview, start folder, new-window behaviour
- P0 Read/write the same registry values (`HKCU\...\Explorer\Advanced`) so settings stay consistent
- P1 Own settings (theme light/dark/system, font/density, animations off, hotkey remapping, save layouts)

### 4.6 Robustness
- P0 File watcher with debounce, auto-refresh, recovery when the current folder is deleted
- P0 Network/disconnected drives: timeouts, never a UI freeze
- P0 Crash safety: session restore, local crash log (no telemetry)
- P0 Single instance + fast forwarding of new invocations (start < 150 ms “warm”)

## 5. Performance budgets (measurable, checked in CI by benchmark)
| Metric | Target |
|---|---|
| Cold start to first window | < 300 ms; warm (background process) < 80 ms |
| Folder with 100 000 files: first rows | < 100 ms, complete < 1 s |
| Scrolling | stable 60 fps, no icon flicker |
| RAM idle | < 80 MB |
| Binary | < 20 MB (ideal: < 10 MB) |

## 6. Milestones
| M | Content | Result |
|---|---|---|
| M0 | Repo setup, ADR D-001, Cargo workspace, CI (Windows + Linux), manifest (DPI, longPath) | Empty window builds in CI |
| M1 | `fs` core: enumeration, sorting, watcher, benchmarks | CLI dump of a 100k folder < budget |
| M2 | UI shell: window, virtualised list, details view, address bar | Browse folders |
| M3 | Tabs, split, sidebar, history, keyboard navigation, session restore | Everyday navigation |
| M4 | File operations + queue + undo + clipboard + drag & drop | Move data safely |
| M5 | Shell integration: icons/thumbs, context menu, properties, Recycle Bin, virtual folders, network | Everyday parity |
| M6 | Settings/folder options, themes, command palette, Inspector | Feature-complete (P0/P1) |
| M7 | Windows integration stages 1–3 (default handler, Win+E, `/select,`) | Replaces Explorer in everyday use |
| M8 | Installer, recovery mode, hardening, dogfooding beta | Release 0.1 |

## 7. Risks
- **No Windows in the sandbox:** everything Windows-specific is only testable via CI → set up a VM/Windows-runner pipeline early, Boss tests builds manually.
- **Explorer is deeply embedded in the system** (shell, Win+E, file dialogs): a complete replacement is only possible in stages – details/limits in `WINDOWS-INTEGRATION.md`.
- **Third-party shell extensions can crash** → host them in a separate process/with thread isolation (timeout, watchdog).
- Virtual folders (Libraries, Control Panel) are only cleanly reachable through the shell namespace.
