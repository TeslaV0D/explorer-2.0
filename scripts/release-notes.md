## Install

1. Download **`Explorer2-Setup-@VERSION@-x64.exe`** and double-click it.
2. Windows may show “Windows protected your PC” (the file is not signed yet) → **More info → Run anyway**.
3. Confirm – done. No administrator rights needed.

It installs to `%LOCALAPPDATA%\Programs\Explorer2`, with a Start menu entry “Explorer 2.0” and an entry under “Settings → Apps”. Running a newer setup over an existing installation = update (even while Explorer 2.0 is open).

**Windows integration** (open folders in Explorer 2.0 by double-click, Win+E) is **off** after installation – switch it on item by item in the settings. Everything can be reset with one button: in the app, via the Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” (restore Windows default) or, as a last resort, with `explorer2-recover.exe --yes`.

The app's user interface is currently German.

## Uninstall

“Settings → Apps → Explorer 2.0 → Uninstall”. This resets all Windows integrations and removes the program, the Start menu entries and the Apps entry. Your settings (`%APPDATA%\Explorer2`) are kept.

## Portable

Unpack `Explorer2-@VERSION@-x64-portable.zip` and start `explorer2.exe` – no installation. You can install later from within the app (Einstellungen → Installation).

## Checksums

`SHA256SUMS.txt` – verify with `Get-FileHash .\Explorer2-Setup-@VERSION@-x64.exe`.

Requirement: Windows 10/11 x64.
