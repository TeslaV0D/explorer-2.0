# Windows integration – replacing Explorer system-wide

> Status: **research/plan.** Statements marked ⚠️ are derived from practical experience of other Explorer alternatives (Explorer++, Directory Opus, Files, Total Commander) and **must be verified on real Windows 10/11** (not testable in the dev sandbox).

Windows has **no** official “default file manager” setting. Replacement = taking over several entry points one by one. Stages from safe → risky:

## Stage 1 – folder verbs in the registry (safe, reversible, HKCU)
> **Implementation M7.1 (D-021):** do not overwrite `open`, but add our own verb `explorer2open` under `HKCU\Software\Classes\{Directory,Drive}\shell` + set the default verb (`@`) there to `explorer2open` (Windows ships `none` → otherwise falls through to `Folder` = Explorer). **`Folder` stays untouched** (it also opens Control Panel, Recycle Bin …). Context menu: own verb `explorer2` (Directory `%1`, Directory\Background `%V`, Drive `%1`). Everything via `explorer-integrate` (journal, manifest, reset).

When any app “opens” a folder (`ShellExecute`, “Open folder” in browsers/IDEs, double-click on a folder shortcut), Windows resolves the verb `open` on `Folder`/`Directory`/`Drive`.
- `HKCU\Software\Classes\Directory\shell\open\command` → `"explorer2.exe" "%1"`
- `HKCU\Software\Classes\Folder\shell\open\command` → the same, plus an empty `DelegateExecute` value (without it the default handler keeps taking effect) ⚠️
- Likewise `Drive\shell\open\command`; also take over the shell verb `explore`.
- Per user (HKCU) → no admin rights; uninstall = delete the keys.
- Additionally our own entries “Open in Explorer 2.0” in the context menu (`Directory\Background\shell`) – harmless as a fallback.

## Stage 2 – Win+E and “This PC” ⚠️
> **Implementation M7.2:** registry route (CLSID verb `opennewwindow` + empty `DelegateExecute`), no keyboard hook. Feature `win-e` in `explorer-integrate`.

- Win+E starts a “Computer” window through the shell hotkey. Known registry route: `HKCU\Software\Classes\CLSID\{52205FD8-5DFB-447D-801A-D0B52F2E83E1}\shell\opennewwindow\command` with our own command (+ empty `DelegateExecute`).
- Alternative/fallback: low-level keyboard hook (`WH_KEYBOARD_LL`) in our background process that intercepts and swallows Win+E. Drawback: the hook has to run (autostart/tray), antivirus sensitivity.
- Decision later by field test which route is stable on Win10/Win11.

## Stage 3 – `explorer.exe` invocations (`/select,`, `/e,`, `explorer.exe C:\x`) ⚠️
Many apps call `explorer.exe /select,"C:\file"` directly (“Show in folder”, browser downloads, Teams, VS Code …). This bypasses stage 1.
- Options: (a) **Image File Execution Options** (`HKLM\...\Image File Execution Options\explorer.exe` → `Debugger` = our proxy). The proxy redirects invocations with path arguments to us and starts the real Explorer on **shell start / without arguments** (a debugger that starts the same image itself is not redirected again). (b) File-system/service hooks → to be rejected.
- **High risk** (explorer.exe is also desktop/taskbar/Start menu; bug = broken shell). Therefore: opt-in, admin needed (HKLM), **Safe Mode/recovery tool** (`explorer2-recover.exe`, restore via registry script, boot fallback folder), strict allowlist of argument patterns, tests in a VM with snapshots.

## Stage 4 – file dialogs (Open/Save)
The Common Item Dialog (`IFileOpenDialog`) is hosted **in-process** in each app; not replaceable without injection. Realistically: dialogs stay Windows-native (**deliberate limit**), maybe later a DLL extension/hook as a voluntary add-on (“jump to the Explorer 2.0 path in the dialog”, quick switch à la Listary/Dopus). Not in Phase 1.

## Stage 5 – taskbar, Start menu, desktop icons
These belong to the shell (`explorer.exe`). No replacement planned; we remain a file manager, not a shell replacement. (Optional: desktop folder fully usable in the file manager.)

## Further integration points
| Topic | Approach |
|---|---|
| **Hosting the context menu** | `IShellFolder::GetUIObjectOf` → `IContextMenu` (+ `IContextMenu2/3`), third-party entries come along this way; Win11 entries (`IExplorerCommand`) via verb registry or sparse package ⚠️ |
| **Own context menu entries in Win11** | `IExplorerCommand` + sparse MSIX (package identity) for the top-level menu; otherwise “Show more options” |
| **File operations** | `IFileOperation` (Recycle Bin, UAC, conflicts, undo-compatible) |
| **Icons / thumbnails** | `SHGetFileInfo`/`IShellItemImageFactory`, overlay handlers (`IShellIconOverlayIdentifier`) |
| **Properties** | `SHObjectProperties`, `IPropertyStore`, columns via `PKEY_*` |
| **Preview** | Host `IPreviewHandler` (Office/PDF preview handlers) |
| **Drag & drop / clipboard** | `IDropTarget`, `IDataObject`, `CF_HDROP`, `Preferred DropEffect`, `CFSTR_SHELLIDLIST` |
| **Virtual folders** | Shell namespace/PIDLs (This PC, Recycle Bin, Network, Libraries, Control Panel) |
| **Cloud placeholders** | Respect OneDrive/Cloud Files API attributes (`RECALL_ON_DATA_ACCESS`), never hydrate unintentionally |
| **Long paths/DPI** | Manifest: `longPathAware`, `PerMonitorV2`, `supportedOS` |
| **Single instance** | Named mutex + named pipe, autostart daemon → warm start |
| **Jump list / taskbar** | `ICustomDestinationList`, AppUserModelID |
| **“Open with”/file association** | We are the folder handler, *not* the default app for file types. `UserChoice` is hash-protected → do not bypass. |

## R-001: one-button reset (hard requirement, Boss)
> **Implemented** (2026-11-05, crate `explorer-integrate`, D-020): journal, app button, `explorer2-recover.exe`, CI snapshot test including “journal missing/damaged”. Open until M7/M8: `.reg`/PowerShell fallback script, Start menu entry, uninstall, self-healing (stage 3), HKLM copy in `%ProgramData%` (the mirror currently lives in `%LOCALAPPDATA%`). **Rule for M7:** every new integration only via `apply()` and register its locations in `manifest::points()`.

Lesson from File Pilot: after a failure, only a Windows reinstall helped there. That must **never** happen with us.
- **Journal**: before every change (registry keys/values, IFEO, Win+E verb, context menu, autostart, environment variables/PATH, shortcuts, file associations, hotkey hooks) the **original state** (value, type, “did not exist”) is written to a journal (`%APPDATA%\Explorer2\integration-journal.json` + a copy in `%ProgramData%` for HKLM parts).
- **Reset button in the app** (“Alle Windows-Integrationen zurücksetzen” – reset all Windows integrations): replays the journal backwards, removes all our own keys, restarts `explorer.exe` if needed.
- **Standalone recovery tool** `explorer2-recover.exe` (no UI/GPU dependency, console): works when the app does not start, also in Safe Mode; additionally a bundled `.reg`/PowerShell fallback script and a Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” (restore Windows default).
- **Self-healing**: if the `explorer.exe` proxy (stage 3) fails to start cleanly several times → automatically fall back to the Windows default.
- **Uninstall** calls the same reset.
- **Mandatory test in CI (Windows runner/VM)**: snapshot (registry areas, environment) → all integrations on → reset → snapshot must be **identical**; plus the test “journal missing/damaged” → reset uses known default values.

## Installation & security
> **Implementation M8 (D-023):** own installer in Rust instead of Inno Setup/MSIX – per user (`%LOCALAPPDATA%\Programs\Explorer2`), Start menu + Apps & Features via the integration journal, uninstall = R-001 reset + program folder. HKLM/admin only if M7.3 comes later.

- Installer (Inno Setup/WiX) **and** portable mode; HKCU first, HKLM only for stage 3 (confirmed separately).
- **Log every change + rollback function** (“reset everything to the Windows default” in the settings and as standalone recovery).
- No kernel driver, no DLL injection into foreign processes in Phase 1.
- Code signing needed later (SmartScreen/AV acceptance).

## Open verification points (test on Windows)
1. Does `Folder\shell\open\command` + `DelegateExecute` work as expected on Win11 24H2/25H2?
2. Win+E redirection via CLSID verb vs. keyboard hook – which one is stable?
3. IFEO proxy: behaviour on shell restart, `explorer.exe /factory`, taskbar pins.
4. Win11 top-level context menu: sparse package effort vs. benefit.
