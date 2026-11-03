# Windows-Integration – Explorer systemweit ersetzen

> Status: **Recherche/Plan.** Aussagen mit ⚠️ sind aus Erfahrungswissen/Praxis anderer Explorer-Alternativen (Explorer++, Directory Opus, Files, Total Commander) abgeleitet und **müssen auf echtem Windows 10/11 verifiziert werden** (in der Dev-Sandbox nicht testbar).

Es gibt **keine** offizielle „Default File Manager“-Einstellung in Windows. Ersatz = mehrere Einstiegspunkte einzeln übernehmen. Stufen von sicher → riskant:

## Stufe 1 – Ordner-Verben in der Registry (sicher, reversibel, HKCU)
Wenn irgendeine App einen Ordner „öffnet“ (`ShellExecute`, „Ordner öffnen“ in Browsern/IDEs, Doppelklick auf Ordner-Verknüpfung), löst Windows das Verb `open` auf `Folder`/`Directory`/`Drive` auf.
- `HKCU\Software\Classes\Directory\shell\open\command` → `"explorer2.exe" "%1"`
- `HKCU\Software\Classes\Folder\shell\open\command` → dito, plus leerer Wert `DelegateExecute` (ohne ihn greift der Standard-Handler weiter) ⚠️
- Analog `Drive\shell\open\command`; Shell-Verb `explore` ebenfalls belegen.
- Per-User (HKCU) → keine Admin-Rechte; Deinstallation = Schlüssel löschen.
- Zusätzlich eigene Einträge „In Explorer 2.0 öffnen“ im Kontextmenü (`Directory\Background\shell`) – harmlos als Fallback.

## Stufe 2 – Win+E und „Dieser PC“ ⚠️
- Win+E startet über den Shell-Hotkey ein „Computer“-Fenster. Bekannter Registry-Weg: `HKCU\Software\Classes\CLSID\{52205FD8-5DFB-447D-801A-D0B52F2E83E1}\shell\opennewwindow\command` mit eigenem Befehl (+ leerer `DelegateExecute`).
- Alternative/Fallback: Low-Level-Keyboard-Hook (`WH_KEYBOARD_LL`) in unserem Hintergrundprozess, der Win+E abfängt und verschluckt. Nachteil: Hook muss laufen (Autostart/Tray), Antivirus-Sensibilität.
- Entscheidung später per Praxistest, welcher Weg auf Win10/Win11 stabil ist.

## Stufe 3 – `explorer.exe`-Aufrufe (`/select,`, `/e,`, `explorer.exe C:\x`) ⚠️
Viele Apps rufen direkt `explorer.exe /select,"C:\file"` auf („Im Ordner anzeigen“, Browser-Downloads, Teams, VS Code …). Das umgeht Stufe 1.
- Optionen: (a) **Image File Execution Options** (`HKLM\...\Image File Execution Options\explorer.exe` → `Debugger` = unser Proxy). Proxy leitet Aufrufe mit Pfad-Argumenten an uns um und startet bei **Shell-Start / ohne Argumente** den echten Explorer (ein Debugger, der dasselbe Image selbst startet, wird nicht erneut umgeleitet). (b) Dateisystem-/Dienst-Hooks → abzulehnen.
- **Hohes Risiko** (explorer.exe ist zugleich Desktop/Taskleiste/Startmenü; Bug = kaputte Shell). Daher: opt-in, Admin nötig (HKLM), **Safe-Mode-/Recovery-Tool** (`explorer2-recover.exe`, Wiederherstellung per Registry-Skript, Boot-Fallback-Ordner), strenge Allowlist der Argumentmuster, Tests in VM mit Snapshot.

## Stufe 4 – Datei-Dialoge (Öffnen/Speichern)
Common Item Dialog (`IFileOpenDialog`) wird **in-process** in der jeweiligen App gehostet; nicht ersetzbar ohne Injection. Realistisch: Dialoge bleiben Windows-nativ (**bewusste Grenze**), evtl. später DLL-Extension/Hook als freiwilliges Add-on („Pfad im Dialog nach Explorer 2.0 springen“, Quick-Switch à la Listary/Dopus). Nicht in Phase 1.

## Stufe 5 – Taskleiste, Startmenü, Desktop-Icons
Gehören zur Shell (`explorer.exe`). Kein Ersatz geplant; wir bleiben Dateimanager, nicht Shell-Replacement. (Optional: Desktop-Ordner im Dateimanager voll nutzbar.)

## Weitere Integrationspunkte
| Thema | Ansatz |
|---|---|
| **Kontextmenü hosten** | `IShellFolder::GetUIObjectOf` → `IContextMenu` (+ `IContextMenu2/3`), Einträge Dritter laufen so mit; Win11-Einträge (`IExplorerCommand`) über Verb-Registry bzw. Sparse-Package ⚠️ |
| **Eigene Kontextmenü-Einträge in Win11** | `IExplorerCommand` + Sparse-MSIX (Package-Identity) für Top-Level-Menü; sonst „Weitere Optionen“ |
| **Dateioperationen** | `IFileOperation` (Papierkorb, UAC, Konflikte, Undo-konform) |
| **Icons / Thumbnails** | `SHGetFileInfo`/`IShellItemImageFactory`, Overlay-Handler (`IShellIconOverlayIdentifier`) |
| **Eigenschaften** | `SHObjectProperties`, `IPropertyStore`, Spalten via `PKEY_*` |
| **Vorschau** | `IPreviewHandler` hosten (Office/PDF-Vorschauhandler) |
| **Drag&Drop / Clipboard** | `IDropTarget`, `IDataObject`, `CF_HDROP`, `Preferred DropEffect`, `CFSTR_SHELLIDLIST` |
| **Virtuelle Ordner** | Shell-Namespace/PIDLs (This PC, Papierkorb, Netzwerk, Libraries, Control Panel) |
| **Cloud-Platzhalter** | OneDrive/Cloud-Files-API Attribute (`RECALL_ON_DATA_ACCESS`) respektieren, nie unbeabsichtigt hydratisieren |
| **Langpfade/DPI** | Manifest: `longPathAware`, `PerMonitorV2`, `supportedOS` |
| **Single-Instance** | Named Mutex + Named Pipe, Autostart-Daemon → warmer Start |
| **Jump List / Taskleiste** | `ICustomDestinationList`, AppUserModelID |
| **„Öffnen mit“/Dateizuordnung** | Wir sind Ordner-Handler, *nicht* Standard-App für Dateitypen. `UserChoice` ist hash-geschützt → nicht umgehen. |

## Installation & Sicherheit
- Installer (Inno Setup/WiX) **und** Portable-Modus; HKCU-first, HKLM nur für Stufe 3 (separat bestätigt).
- **Jede Änderung protokollieren + Rückbau-Funktion** („Alles auf Windows-Standard zurücksetzen“ in Settings und als Standalone-Recovery).
- Kein Kernel-Treiber, keine DLL-Injection in Fremdprozesse in Phase 1.
- Code-Signing später nötig (SmartScreen/AV-Akzeptanz).

## Offene Verifikationspunkte (auf Windows testen)
1. Greift `Folder\shell\open\command` + `DelegateExecute` auf Win11 24H2/25H2 wie erwartet?
2. Win+E-Umleitung per CLSID-Verb vs. Keyboard-Hook – welcher ist stabil?
3. IFEO-Proxy: Verhalten bei Shell-Neustart, `explorer.exe /factory`, Taskleisten-Pins.
4. Win11-Top-Level-Kontextmenü: Sparse-Package-Aufwand vs. Nutzen.
