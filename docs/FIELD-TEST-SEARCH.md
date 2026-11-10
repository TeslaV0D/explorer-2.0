# Field test: search (S1) on real Windows

Goal: before we decide on S2 (MFT/USN with admin), S1 shows on your PC how
fast and reliable the search is **without** admin. Duration: 15–20 min.

## 1. Get it and start it
- GitHub → Actions → latest green run on `claude/relaxed-pasteur-ufhh1p`
  (CI run #93 or newer) → download the artifact **explorer2-windows-x64**, unpack it.
- Ideally start with a trace (PowerShell in the unpacked folder):

  ```powershell
  $env:EXPLORER2_TRACE_FILE = "$env:TEMP\explorer2-trace.txt"; .\explorer2.exe
  ```

  Without a trace works too (double-click); only the numbers are missing then.
- Nothing is installed or registered in Windows unless you explicitly
  switch it on in the settings.

## 2. What you check

The app UI is German; UI labels are quoted as they appear, with a translation.

| # | Step | What to look for |
|---|---|---|
| 1 | Start the app, wait ~1 min, open **Einstellungen → Suche** (Settings → Search) | Status per drive: “wird aufgebaut … n Einträge” (building … n entries) → “bereit, n Einträge (aufgebaut in x s)” (ready, n entries (built in x s)). **How long for C:?** |
| 2 | Keep working normally while it builds | Does the app stay smooth? Does the PC get noticeably slower? |
| 3 | **Ctrl+Shift+F**, type e.g. `rechnung` | Hits < 1 s after typing? Best hits at the top? Enter opens, Ctrl+Enter shows in folder |
| 4 | In a folder press **Ctrl+F**, type something | Only hits below it, “Ordner” (Folder) column, hits marked bold, right-click/copy/delete work |
| 5 | Query language: `ext:pdf dm:thisweek`, `size:>100mb`, `*.jpg urlaub` | Results plausible? Dates in local time? |
| 6 | In Windows Explorer create, rename, delete a file | Does it appear in / disappear from an open search after ~1 s? |
| 7 | Close the app, open it again | Search available immediately (“geladen” (loaded) instead of “aufgebaut” (built))? |
| 8 | Optional: enable a network drive/USB stick in the settings | Build, search, unplugging the stick without a crash |
| 9 | Memory: Task Manager → explorer2.exe | Working set after the build (goal: < 100 MB per 1 M entries) |

## 3. What you send me
- Briefly: what was good, what was annoying, what was missing?
- The numbers from the settings (entries, build time per drive) and
  the memory.
- If traced: `%TEMP%\explorer2-trace.txt` (contains paths of your
  files – review it first or only send the lines with `search:`).
- After a crash: the folder `%APPDATA%\Explorer2\crash`.

## 4. Then we decide
- **S1 is enough** (build bearable, live updates reliable): S2 stays
  shelved, continue with Phase 3 or the beta package (M8.4).
- **Build too slow / changes get lost**: plan S2 (read the MFT + USN journal,
  service with admin, with R-001 rollback).

Emergency exit as always: Settings → “Alle Windows-Integrationen zurücksetzen”
(reset all Windows integrations) or `explorer2-recover.exe --yes`. The search
index itself lives only in `%APPDATA%\Explorer2\index` (delete = rebuilt on the
next start).
