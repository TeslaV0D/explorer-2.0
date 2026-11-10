# Field test: redesign (R10) – checklist for real Windows

As of 2026-11-08 · Goal: sign off the new look (File Pilot style, D-026) on your
machine. **Functions are unchanged** – the only things checked are whether everything
looks good, is readable and works the same (or better).

The app UI is German; UI labels are quoted as they appear, with a translation.

Exe: CI artifact `explorer2-windows-x64` of the latest green run (or the
installed version via Settings → “Installieren / aktualisieren” (install / update)).
Emergency exit as always: Start menu “Explorer 2.0 – Windows-Standard
wiederherstellen” (restore Windows default) or `explorer2-recover.exe --yes`.

## 1. First impression (2 min)
- [ ] Start is fast (feels like before, window there immediately).
- [ ] A new installation starts **dark**; a previously chosen colour scheme stays.
- [ ] Text is sharp (Segoe UI), no boxes/replacement glyphs, no clipped texts.
- [ ] Title bar matches the scheme (dark/light).

## 2. Main window
- [ ] **No bar at the top** any more: file actions (New folder, Cut, Copy, Paste, Rename, Delete, ↶/↷) as icons on the right of the navigation row; tooltip shows name + shortcut.
- [ ] **⋮ “Mehr” (More)**: two views, hidden items, Inspector, command palette, settings.
- [ ] **Status bar** bottom right: sorting, details/icons, Inspector, settings.
- [ ] Make the window narrow (< 900 px): the sidebar gets narrower; with two views the file actions move into the ⋮ menu – nothing overlaps.

## 3. Sidebar, tabs, navigation
- [ ] Sidebar: line icons, active place with a blue mark on the left, drives with a thin fill bar (red from 90 %).
- [ ] Tabs: the current tab “merges” with the navigation row, blue stroke only in the active view; × appears on hover; middle-click closes.
- [ ] Address bar: segments, current folder bright; click into the free area = edit the path; search field with magnifier.

## 4. File list
- [ ] Rows as rounded tiles; selection blue with a bar on the left; grey without focus; focus frame when navigating with the arrow keys.
- [ ] Column header: sort arrow (drawn) at the sorted column; drag the column width.
- [ ] Tile view (Ctrl+Shift+2): same style, thumbnails.
- [ ] Dragging onto a folder: the target lights up; rubber-band selection with the mouse.

## 5. Inspector (Alt+P)
- [ ] Preview at the top, groups ALLGEMEIN / DATUM / ORT (general / date / location), no line per row.
- [ ] “Pfad kopieren” (copy path) puts the path on the clipboard (paste into Notepad).

## 6. Menus and dialogs
- [ ] **Right-click** = our compact menu (Open, Cut, Copy, Paste, Rename, Delete, Properties); on empty space: New folder, Paste, Refresh, Properties.
- [ ] **“Weitere Optionen” (More options)** opens the real Windows menu (7-Zip, Git, “Send to” …) – **dark** when the app is dark.
- [ ] Shift+F10 opens the menu; ↑/↓, Enter, Esc work.
- [ ] Confirm delete (Shift+Del), conflict when copying: dialogs in the new style, Enter/Esc as usual.
- [ ] Command palette (Ctrl+Shift+P) and quick search (Ctrl+Shift+F): magnifier, blue selection, fades in softly.
- [ ] Settings: everything readable, colour scheme Like Windows / Light / Dark switches immediately.

## 7. Display scaling
- [ ] Windows Settings → Display → Scale **125 %** and **150 %**: icons sharp, nothing clipped, click targets hit.

## 8. Colours
- [ ] **Light** and **Dark** both comfortably readable (including dimmed texts like date/size).
- [ ] “Windows-Akzentfarbe verwenden” (use Windows accent colour) with a strong accent colour (e.g. yellow, green): selection stays subtle and readable.

## Feedback
Per item, “fine” or a short sentence or screenshot (Win+Shift+S) is enough.
I'll turn anything noticeable into polish tasks.
