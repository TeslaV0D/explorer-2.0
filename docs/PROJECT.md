# Project description – Explorer 2.0

## Vision
A file manager that **fully** replaces Windows Explorer: wherever Windows opens an Explorer window today, Explorer 2.0 appears instead. Noticeably faster, more stable (no hangs on large folders/network drives) and more modern (tabs, split panels, Inspector, command palette, keyboard-first).

## Goals
1. **Speed:** start, folder changes, scrolling and sorting feel instant (budgets in `PLAN-BASICS.md`).
2. **Feature parity** with Windows Explorer: folder/file operations, context menu (including third-party shell extensions), properties, Recycle Bin, network, all folder options.
3. **Robustness:** the UI thread never blocks on I/O; errors (permissions, locked files, disconnected network drives) are handled cleanly; undo for operations.
4. **System-wide replacement:** opening folders from any app, “Show in folder”, Win+E, file dialogs (as far as technically possible) – see `WINDOWS-INTEGRATION.md`.
5. **Later:** system-wide instant search (like voidtools Everything) and file action shortcuts (like Sweetch).

## Non-goals (for now)
- macOS/Linux UI (the core logic stays portable for tests, though).
- Windows < 10.
- Content indexing of files (Phase 2 starts with names, like Everything).

## Phases
| Phase | Content |
|---|---|
| **1 – Basics** | Fast, reliable file manager + shell integration (currently in planning) |
| 2 – Search | Own index (NTFS MFT/USN), instant search, filters, Everything syntax compatibility |
| 3 – Shortcuts | Radial/drag menu for file actions (convert, tools, actions) à la Sweetch |
| 4 – Polish | Plugins, settings sync, telemetry-free, auto-update |

## Target platform
Windows 10 (1809+) & 11, x64 (ARM64 later). Single-binary philosophy, portable mode + installer.
