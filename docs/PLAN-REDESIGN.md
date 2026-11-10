# Plan: UI/UX redesign (inspired by File Pilot)

As of 2026-11-08 · **R0–R10 implemented** (CI #101–#131, sign-off: field test `docs/FIELD-TEST-REDESIGN.md`) · Boss's brief: **design + feel only**. Functions, logic,
data models, file operations, navigation, search, integration (R-001) stay
unchanged. Afterwards every button triggers exactly the same action as before.

## 1. Guard rails (apply to every stage)
- **The work rule stays:** next stage only when the current one is done, tested and CI-green.
- **UI layer only:** `crates/app/ui/main.slint`, `core::theme` (colours), new pure
  design building blocks. Rust logic is only touched where the new layout strictly
  needs it (e.g. filling a menu model) – never change behaviour.
- **Budgets stay gates:** start < 300 ms (best-of-3), first-start gate (D-018),
  exe < 20 MB, 100k-folder bench. New font glyphs/fonts are the known
  trap (one “▾” cost 1.2 s) → symbols as vector `Path`, no new fonts.
- **All E2E stay green** (Linux 254 checks, Windows E2E including W44/W45, smoke matrix).
- **Accessibility:** contrast ≥ 4.5 for text stays a mandatory test (D-019 tests are extended, not deleted).

## 2. Current-state analysis (steps 1+2)

The labels in the “Today” column are the German UI texts of that time.

| Area | Today | Component / place |
|---|---|---|
| Window chrome | global bar 40 px (Zwei Ansichten, Neuer Ordner, Ausschneiden, Kopieren, Einfügen, Umbenennen, Löschen, ↶ ↷, Ausgeblendete, Details, Einstellungen) | `MainWindow` “global bar” |
| Per pane | tabs 34 px, navigation 40 px (← → ↑, breadcrumbs, search field), column header 26 px, rows 24 px | `FilePane`, `TabItem`, `NavButton`, `HeaderCell`, `ResizeHandle` |
| Vertical loss up to the 1st row | **140 px** | |
| Sidebar | Quick access, “Dieser PC” (This PC) + drives with fill bars | `Sidebar`, `SidebarItem` |
| File list | details + tiles (48/96/256), selection/focus/hover/cut states, rubber-band selection, DnD, inline rename, search highlighting | `FilePane`, `Tile` |
| Inspector | right side, preview 200 px + key/value rows with separator lines | MainWindow “inspector”, `InfoRow` |
| Status bar | count/selection/errors + sort menu + details/icons | MainWindow “status bar”, `SortItem` |
| Overlays | command palette, quick search, settings (scrolls), conflict/confirmation dialog, jobs panel, crash notice | `CommandPalette`, MainWindow |
| Context menu | Windows: **native shell menu** (`TrackPopupMenuEx`, third-party extensions); Linux: none | `shell::win_menu`, `fileops::context_menu` |
| Colours | 38 tokens, light/dark, 12 accents, “Like Windows” live, WCAG-tested | `core::theme::tokens` → Slint `Theme.c.*` (D-019) |
| Font | system font (Windows: Segoe UI), 11–15 px | Slint default |
| Symbols | file icons from the shell (good); UI symbols mostly text characters (← → ↑ ↶ ↷ +), a few `Path`s | scattered |
| Tests | ~100 fixed click coordinates (e2e.sh, e2e-ops.sh, e2e-search.sh, e2e-windows.ps1) | |

## 3. Design system (step 3)

### Colours – dark (reference look), layers instead of black
| Token | Value | Use |
|---|---|---|
| `app` | `#141414` | window background, sidebar |
| `panel` | `#1A1A1A` | pane background, tab bar |
| `surface` | `#202020` | file list, Inspector, menus |
| `raised` | `#262626` | input fields, active tab, dialogs |
| `hover` | `#2A2A2A` | hover |
| `selected` | accent at ~18 % opacity on `surface` | selection (+ 2 px accent bar on the left) |
| `selected-idle` | `#2C2C2C` | selection in the inactive pane |
| `border` | `#2E2E2E` / `border-strong` `#3A3A3A` | lines, frames |
| `text` / `text2` / `muted` | `#EDEDED` / `#A3A3A3` / `#6B6B6B` | hierarchy |
| `accent` | calm blue `#4F8CFF` (option “Windows accent colour” stays) | focus, active elements |

Light is derived from the same roles (same hierarchy, light layers) – it stays
fully supported. Everything stays in `core::theme` with contrast tests (new roles are
added, the tests check every text/background pairing).

### Typography
System font (Windows 11: Segoe UI Variable, otherwise Segoe UI) – costs nothing, looks native.
Scale: **11** (group headers, capitals, letter-spaced), **12** (secondary text, columns),
**13** (default, file names), **14** (palette input). No headings > 14 px.
Weights: 400 / 600; numbers in columns right-aligned.

### Grid, radius, lines, shadows
- 4 px grid; dense sizes: row **24 px** (compact 22 as an option later), tabs **30**,
  navigation **34**, column header **24**, status bar **24**, sidebar **200** (min. 160).
- Radius: 4 (rows, small buttons), 6 (fields, tabs), 8 (menus, dialogs, palette).
- 1 px lines in `border`; shadows only for floating layers (menu, dialog, palette): soft, short.
- “Glass”: only menus/palette with a slightly transparent `surface` (Slint cannot do
  background blur → no glassmorphism effects).

### Symbols
Own, uniform line set as Slint `Path` (16 px, 1.5 px stroke, round caps),
based on Lucide (ISC license, notice in `THIRD-PARTY.md`): back, forward, up, search, more
(⋮), new folder, cut, copy, paste, rename, delete, undo,
redo, two views, details, tiles, Inspector, settings, close,
check mark, arrow (sorting), star (Quick access), drive, network drive, USB,
Recycle Bin, folder. **File icons stay the real Windows icons** (recognisability).

### States (the same everywhere)
Hover = `hover` · pressed = `raised` · focus (keyboard) = 1 px accent frame ·
selected = `selected` + accent bar · inactive pane = `selected-idle` · disabled = 40 % opacity.

### Motion
120 ms (hover, selection), 160 ms (menu/palette appear: opacity + 4 px offset),
180 ms (Inspector/sidebar in/out). Nothing waits for an animation; the keyboard
acts immediately. Software renderer: animations only where they do not redraw the whole list.

## 4. Target layout

```
┌ Tabs ─────────────────────────────────────────────────────────────┐ 30
│ Projects ×   Downloads ×   Documents ×   +                         │
├ Navigation ───────────────────────────────────────────────────────┤ 34
│ ←  →  ↑   C: › Projects › MyApp                 [🔍 Search in …]  ⋮ │
├──────────────┬─────────────────────────────────────┬──────────────┤
│ QUICK ACCESS │ Name              Modified   Size    │ PREVIEW      │
│ ★ Home       │ ▌📁 Assets        …                  │              │
│ ★ Desktop    │  📄 README.md     …                  │ NAME         │
│              │                                      │ TYPE …       │
│ DRIVES       │                                      │              │
│ ▣ System (C:)│                                      │              │
├──────────────┴─────────────────────────────────────┴──────────────┤ 24
│ 142 items · 3 selected                       14.2 GB free   ≡  ▦  ⓘ │
└────────────────────────────────────────────────────────────────────┘
```
(The real UI shows these labels in German.)
- **The global bar goes away.** Its actions move without any behaviour change
  (same `command(...)` call):
  - File actions (New folder, Cut/Copy/Paste, Rename, Delete,
    ↶/↷) as compact icon buttons with tooltips on the right of the navigation row. In a
    narrow window they end up in the ⋮ menu.
  - Two views, hidden items, settings and Inspector/details/tiles into the
    ⋮ menu or the status bar.
  - Everything stays in the command palette and on the keyboard shortcuts.
- Saves **~50 px** of height for files.
- **Two views:** each pane with its own tab and navigation row. The active pane is
  marked by an accent stroke on the tab.
- **Narrow windows:** < 900 px sidebar at 160 px; < 760 px the Inspector closes
  (stays toggleable), file actions into the ⋮ menu; the file list keeps priority.

## 5. Stages

| Stage | Content | Done when |
|---|---|---|
| **R0 Safeguards** | (a) **Robust test targets**: click targets by name instead of coordinate. First a spike: Slint testing API (`ElementHandle` + `accessible-label`) in a test mode that writes the positions of named elements to the trace; otherwise a central coordinate table (`scripts/ui-targets.sh`/`.ps1`) with a measuring script. (b) **Screenshot gallery** “before” of all screens (light/dark, split, search, palette, settings, Inspector, dialogs, tiles, narrow window) as a CI artifact. Side effect: `accessible-label` on all controls (screen readers) | E2E unchanged green, gallery in the artifact |
| **R1 Design system** | new colour roles in `core::theme` (dark + light, accent) including contrast tests; Slint global `Ui` (sizes, radii, durations); symbol component + set; building blocks `IconButton`, `ToolbarButton`, `MenuItem`, `SectionLabel`, `Field` | app still looks the same, building blocks tested (gallery page) |
| **R2 Main layout** | global bar gone, actions moved, new status bar, breakpoints | all actions reachable by click/palette/key (E2E), gallery |
| **R3 Sidebar** | group headers small/letter-spaced, 26 px entries, symbols, accent mark for active, drive fill level subtle | E2E + gallery |
| **R4 Navigation** | icon buttons, breadcrumbs as calm segments, search field with magnifier, ⋮ menu | E2E + gallery |
| **R5 Tabs** | IDE tabs 30 px, active tab in `raised` + accent stroke, close × on hover/active | E2E + gallery |
| **R6 File list** | header 24 px with vector sort arrow, rows 24 px, hover/selection/multi-selection with accent bar, focus frame, tiles in the same style, rubber-band selection, drop target | E2E (selection/DnD/rename) + pixel checks + gallery |
| **R7 Inspector** | “Property Inspector”: preview at the top, groups (general, date, path) with small capital headers, without lines per row, path copyable, in/out animated | E2E [68] + gallery |
| **R8 Menus + dialogs** | context menu (see decision 2), conflict/confirmation dialog, palette, quick search, settings, jobs panel, notice in the new style, animated appearance | E2E + Windows E2E (menu, dialogs) |
| **R9 States + motion** | pass: all hover/active/selected/focus/disabled the same, animations 120–180 ms | pixel checks of the states |
| **R10 Consistency + QA** | gallery “after” next to “before”, start/first-start gates, exe size, Windows E2E, DPI 125/150 % (Mesa CI), checklist for your field test | everything green + Boss sign-off |

## 6. Decisions for you (with recommendation)
1. **Default colour scheme:** *Recommendation:* **dark** as default for new installations (reference look); “Like Windows” and light stay selectable, existing settings are kept.
2. **Context menu (Windows):** the native shell menu cannot be styled. *Recommendation:* own compact dark menu with our commands (the same actions as today) + an entry “Weitere Optionen” (more options) that opens the native menu with all extensions (like Windows 11); additionally switch the native menu to dark. Alternative: only darken the native menu.
3. **Global bar:** *Recommendation:* dissolve it as in section 4 (actions into navigation/⋮/status bar, everything still via palette + keys).
4. **Font:** *Recommendation:* system font (Segoe UI Variable) instead of an embedded font (costs start time + size, D-015/first-start gate).
5. **Accent colour:** *Recommendation:* calm blue as default, option “use Windows accent colour” stays.

## 7. Risks
- **Test coordinates:** R0 makes click targets nameable; otherwise every stage breaks dozens of checks.
- **First frame:** avoid new glyphs/fonts; many `Path` symbols are cheap but are measured in the first-start gate.
- **Slint limits:** no background blur, no partial styling of text (highlighting stays three-part), animations sparing in the software renderer.
- **Darkening the native menu** uses an undocumented Windows function (used by many apps). If it fails it stays light, functionally unchanged.
- **Two window-size worlds:** CI Windows 1024×768, Linux 1280×800 → test breakpoints in both.
