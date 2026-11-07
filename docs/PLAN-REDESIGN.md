# Plan: UI/UX-Redesign (File-Pilot-inspiriert)

Stand 2026-11-07 · Auftrag Boss: **nur Design + Bediengefühl**. Funktionen, Logik,
Datenmodelle, Dateioperationen, Navigation, Suche, Integration (R-001) bleiben
unverändert. Jeder Knopf löst danach exakt dieselbe Aktion aus wie vorher.

## 1. Leitplanken (gelten für jede Stufe)
- **Arbeitsregel bleibt:** nächste Stufe erst, wenn die aktuelle fertig, getestet und CI-grün ist.
- **Nur UI-Schicht:** `crates/app/ui/main.slint`, `core::theme` (Farben), neue reine
  Design-Bausteine. Rust-Logik wird nur angefasst, wo das neue Layout es zwingend
  braucht (z. B. ein Menü-Modell füllen) – nie Verhalten ändern.
- **Budgets bleiben Gates:** Start < 300 ms (best-of-3), Erststart-Gate (D-018),
  Exe < 20 MB, 100k-Ordner-Bench. Neue Schrift-Glyphen/Fonts sind die bekannte
  Falle (ein „▾“ kostete 1,2 s) → Symbole als Vektor-`Path`, keine neuen Fonts.
- **Alle E2E bleiben grün** (Linux 254 Checks, Windows-E2E inkl. W44/W45, Smoke-Matrix).
- **Barrierefreiheit:** Kontrast ≥ 4,5 für Text bleibt Test-Pflicht (D-019-Tests werden erweitert, nicht gelöscht).

## 2. Ist-Analyse (Schritt 1+2)

| Bereich | Heute | Komponente / Ort |
|---|---|---|
| Fenster-Chrome | globale Leiste 40 px (Zwei Ansichten, Neuer Ordner, Ausschneiden, Kopieren, Einfügen, Umbenennen, Löschen, ↶ ↷, Ausgeblendete, Details, Einstellungen) | `MainWindow` „global bar“ |
| Pro Pane | Tabs 34 px, Navigation 40 px (← → ↑, Brotkrumen, Suchfeld), Spaltenkopf 26 px, Zeilen 24 px | `FilePane`, `TabItem`, `NavButton`, `HeaderCell`, `ResizeHandle` |
| Vertikaler Verlust bis zur 1. Zeile | **140 px** | |
| Seitenleiste | Schnellzugriff, „Dieser PC“ + Laufwerke mit Füllbalken | `Sidebar`, `SidebarItem` |
| Dateiliste | Details + Kacheln (48/96/256), Auswahl-/Fokus-/Hover-/Cut-Zustände, Rahmenauswahl, DnD, Inline-Umbenennen, Such-Hervorhebung | `FilePane`, `Tile` |
| Inspector | rechts, Vorschau 200 px + Key/Value-Zeilen mit Trennlinien | MainWindow „inspector“, `InfoRow` |
| Statusleiste | Anzahl/Auswahl/Fehler + Sortieren-Menü + Details/Symbole | MainWindow „status bar“, `SortItem` |
| Overlays | Befehlspalette, Schnellsuche, Einstellungen (scrollt), Konflikt-/Bestätigungsdialog, Jobs-Panel, Absturz-Hinweis | `CommandPalette`, MainWindow |
| Kontextmenü | Windows: **natives Shell-Menü** (`TrackPopupMenuEx`, Erweiterungen Dritter); Linux: keins | `shell::win_menu`, `fileops::context_menu` |
| Farben | 38 Tokens, hell/dunkel, 12 Akzente, „Wie Windows“ live, WCAG-getestet | `core::theme::tokens` → Slint `Theme.c.*` (D-019) |
| Schrift | Systemschrift (Windows: Segoe UI), 11–15 px | Slint-Standard |
| Symbole | Datei-Icons aus der Shell (gut); UI-Symbole meist Textzeichen (← → ↑ ↶ ↷ +), wenige `Path` | verstreut |
| Tests | ~100 feste Klick-Koordinaten (e2e.sh, e2e-ops.sh, e2e-search.sh, e2e-windows.ps1) | |

## 3. Design-System (Schritt 3)

### Farben – dunkel (Referenz-Look), Ebenen statt Schwarz
| Token | Wert | Verwendung |
|---|---|---|
| `app` | `#141414` | Fensterhintergrund, Seitenleiste |
| `panel` | `#1A1A1A` | Pane-Hintergrund, Tab-Leiste |
| `surface` | `#202020` | Dateiliste, Inspector, Menüs |
| `raised` | `#262626` | Eingabefelder, aktiver Tab, Dialoge |
| `hover` | `#2A2A2A` | Hover |
| `selected` | Akzent bei ~18 % Deckkraft auf `surface` | Auswahl (+ 2-px-Akzentbalken links) |
| `selected-idle` | `#2C2C2C` | Auswahl in inaktiver Pane |
| `border` | `#2E2E2E` / `border-strong` `#3A3A3A` | Linien, Rahmen |
| `text` / `text2` / `muted` | `#EDEDED` / `#A3A3A3` / `#6B6B6B` | Hierarchie |
| `accent` | ruhiges Blau `#4F8CFF` (Option „Windows-Akzentfarbe“ bleibt) | Fokus, aktive Elemente |

Hell wird aus denselben Rollen abgeleitet (gleiche Hierarchie, helle Ebenen) – bleibt
voll unterstützt. Alles bleibt in `core::theme` mit Kontrast-Tests (neue Rollen kommen
dazu, die Tests prüfen jede Text/Hintergrund-Paarung).

### Typografie
Systemschrift (Windows 11: Segoe UI Variable, sonst Segoe UI) – kostet nichts, wirkt nativ.
Skala: **11** (Gruppenköpfe, Großbuchstaben, gesperrt), **12** (Sekundärtext, Spalten),
**13** (Standard, Dateinamen), **14** (Paletteneingabe). Keine Überschriften > 14 px.
Gewichte: 400 / 600; Zahlen in Spalten rechtsbündig.

### Raster, Radius, Linien, Schatten
- 4-px-Raster; dichte Maße: Zeile **24 px** (kompakt 22 als Option später), Tabs **30**,
  Navigation **34**, Spaltenkopf **24**, Statusleiste **24**, Seitenleiste **200** (min. 160).
- Radius: 4 (Zeilen, kleine Knöpfe), 6 (Felder, Tabs), 8 (Menüs, Dialoge, Palette).
- 1-px-Linien in `border`; Schatten nur für schwebende Ebenen (Menü, Dialog, Palette): weich, kurz.
- „Glas“: nur Menüs/Palette mit leicht transparentem `surface` (Slint kann keinen
  Hintergrund-Blur → keine Glassmorphism-Effekte).

### Symbole
Eigenes, einheitliches Linien-Set als Slint-`Path` (16 px, 1,5 px Strich, runde Enden),
Basis Lucide (ISC-Lizenz, Hinweis in `THIRD-PARTY.md`): zurück, vor, hoch, suchen, mehr
(⋮), neuer Ordner, ausschneiden, kopieren, einfügen, umbenennen, löschen, rückgängig,
wiederholen, zwei Ansichten, Details, Kacheln, Inspector, Einstellungen, Schließen,
Haken, Pfeil (Sortierung), Stern (Schnellzugriff), Laufwerk, Netzlaufwerk, USB,
Papierkorb, Ordner. **Datei-Icons bleiben die echten Windows-Icons** (Erkennbarkeit).

### Zustände (überall gleich)
Hover = `hover` · Gedrückt = `raised` · Fokus (Tastatur) = 1-px-Akzentrahmen ·
Ausgewählt = `selected` + Akzentbalken · inaktive Pane = `selected-idle` · deaktiviert = 40 % Deckkraft.

### Bewegung
120 ms (Hover, Auswahl), 160 ms (Menü/Palette erscheinen: Deckkraft + 4 px Versatz),
180 ms (Inspector/Seitenleiste ein/aus). Nichts wartet auf eine Animation; Tastatur
wirkt sofort. Software-Renderer: Animationen nur, wo sie nicht die ganze Liste neu zeichnen.

## 4. Ziel-Layout

```
┌ Tabs ─────────────────────────────────────────────────────────────┐ 30
│ Projects ×   Downloads ×   Documents ×   +                         │
├ Navigation ───────────────────────────────────────────────────────┤ 34
│ ←  →  ↑   C: › Projects › MyApp                 [🔍 Suchen in …]  ⋮ │
├──────────────┬─────────────────────────────────────┬──────────────┤
│ SCHNELLZUGR. │ Name              Geändert   Größe   │ VORSCHAU     │
│ ★ Home       │ ▌📁 Assets        …                  │              │
│ ★ Desktop    │  📄 README.md     …                  │ NAME         │
│              │                                      │ TYP …        │
│ LAUFWERKE    │                                      │              │
│ ▣ System (C:)│                                      │              │
├──────────────┴─────────────────────────────────────┴──────────────┤ 24
│ 142 Elemente · 3 ausgewählt                 14,2 GB frei   ≡  ▦  ⓘ │
└────────────────────────────────────────────────────────────────────┘
```
- **Globale Leiste entfällt.** Ihre Aktionen wandern ohne Verhaltensänderung
  (gleicher `command(...)`-Aufruf):
  - Datei-Aktionen (Neuer Ordner, Ausschneiden/Kopieren/Einfügen, Umbenennen, Löschen,
    ↶/↷) als kompakte Symbol-Knöpfe mit Tooltip rechts in der Navigationszeile. Bei
    schmalem Fenster landen sie im ⋮-Menü.
  - Zwei Ansichten, Ausgeblendete, Einstellungen sowie Inspector/Details/Kacheln in das
    ⋮-Menü bzw. in die Statusleiste.
  - Alles bleibt in der Befehlspalette und auf den Tastenkürzeln.
- Spart **~50 px** Höhe für Dateien.
- **Zwei Ansichten:** jede Pane mit eigener Tab- und Navigationszeile. Die aktive Pane ist
  durch einen Akzentstrich am Tab markiert.
- **Schmale Fenster:** < 900 px Seitenleiste auf 160 px; < 760 px Inspector schließt sich
  (bleibt umschaltbar), Datei-Aktionen ins ⋮-Menü; die Dateiliste behält Vorrang.

## 5. Stufen

| Stufe | Inhalt | Fertig, wenn |
|---|---|---|
| **R0 Absicherung** | (a) **Robuste Testziele**: Klickziele per Name statt Koordinate. Erst Spike: Slint-Testing-API (`ElementHandle` + `accessible-label`) in einem Test-Modus, der die Positionen benannter Elemente ins Trace schreibt; sonst zentrale Koordinaten-Tabelle (`scripts/ui-targets.sh`/`.ps1`) mit Mess-Skript. (b) **Screenshot-Galerie** „vorher“ aller Screens (hell/dunkel, Split, Suche, Palette, Einstellungen, Inspector, Dialoge, Kacheln, schmales Fenster) als CI-Artefakt. Nebeneffekt: `accessible-label` an allen Bedienelementen (Screenreader) | E2E unverändert grün, Galerie im Artefakt |
| **R1 Design-System** | neue Farbrollen in `core::theme` (dunkel + hell, Akzent) inkl. Kontrast-Tests; Slint-Global `Ui` (Maße, Radien, Dauer); Symbol-Komponente + Set; Bausteine `IconButton`, `ToolbarButton`, `MenuItem`, `SectionLabel`, `Field` | App sieht noch gleich aus, Bausteine getestet (Galerie-Seite) |
| **R2 Hauptlayout** | globale Leiste weg, Aktionen umgezogen, neue Statusleiste, Breakpoints | alle Aktionen per Klick/Palette/Taste erreichbar (E2E), Galerie |
| **R3 Seitenleiste** | Gruppenköpfe klein/gesperrt, 26-px-Einträge, Symbole, Akzentmarke für aktiv, Laufwerks-Füllstand dezent | E2E + Galerie |
| **R4 Navigation** | Symbol-Knöpfe, Brotkrumen als ruhige Segmente, Suchfeld mit Lupe, ⋮-Menü | E2E + Galerie |
| **R5 Tabs** | IDE-Tabs 30 px, aktiver Tab in `raised` + Akzentstrich, Schließen-× bei Hover/aktiv | E2E + Galerie |
| **R6 Dateiliste** | Kopf 24 px mit Vektor-Sortierpfeil, Zeilen 24 px, Hover/Auswahl/Multi-Auswahl mit Akzentbalken, Fokusrahmen, Kacheln im selben Stil, Rahmenauswahl, Drop-Ziel | E2E (Auswahl/DnD/Rename) + Pixel-Checks + Galerie |
| **R7 Inspector** | „Property Inspector“: Vorschau oben, Gruppen (Allgemein, Datum, Pfad) mit kleinen Großbuchstaben-Köpfen, ohne Linien pro Zeile, Pfad kopierbar, Ein/Aus animiert | E2E [68] + Galerie |
| **R8 Menüs + Dialoge** | Kontextmenü (siehe Entscheidung 2), Konflikt-/Bestätigungsdialog, Palette, Schnellsuche, Einstellungen, Jobs-Panel, Hinweis im neuen Stil, Erscheinen animiert | E2E + Windows-E2E (Menü, Dialoge) |
| **R9 Zustände + Bewegung** | Durchgang: alle Hover/Active/Selected/Fokus/Disabled gleich, Animationen 120–180 ms | Pixel-Checks der Zustände |
| **R10 Konsistenz + QA** | Galerie „nachher“ neben „vorher“, Start-/Erststart-Gates, Exe-Größe, Windows-E2E, DPI 125/150 % (Mesa-CI), Checkliste für deinen Praxistest | alles grün + Abnahme Boss |

## 6. Entscheidungen für dich (mit Empfehlung)
1. **Standard-Farbschema:** *Empfehlung:* **Dunkel** als Standard für neue Installationen (Referenz-Look); „Wie Windows“ und Hell bleiben wählbar, bestehende Einstellungen bleiben.
2. **Kontextmenü (Windows):** Das native Shell-Menü lässt sich nicht stylen. *Empfehlung:* eigenes kompaktes dunkles Menü mit unseren Befehlen (dieselben Aktionen wie heute) + Eintrag „Weitere Optionen“, der das native Menü mit allen Erweiterungen öffnet (wie Windows 11); das native Menü zusätzlich auf dunkel schalten. Alternative: nur das native Menü abdunkeln.
3. **Globale Leiste:** *Empfehlung:* auflösen wie in Abschnitt 4 (Aktionen in Navigation/⋮/Statusleiste, alles weiter per Palette + Tasten).
4. **Schrift:** *Empfehlung:* Systemschrift (Segoe UI Variable) statt eingebetteter Schrift (kostet Start + Größe, D-015/Erststart-Gate).
5. **Akzentfarbe:** *Empfehlung:* ruhiges Blau als Standard, Option „Windows-Akzentfarbe verwenden“ bleibt.

## 7. Risiken
- **Testkoordinaten:** R0 macht Klickziele benennbar; sonst bricht jede Stufe dutzende Checks.
- **Erster Frame:** neue Glyphen/Fonts vermeiden; viele `Path`-Symbole sind billig, werden aber im Erststart-Gate gemessen.
- **Slint-Grenzen:** kein Hintergrund-Blur, kein Teil-Styling von Text (Hervorhebung bleibt dreiteilig), Animationen im Software-Renderer sparsam.
- **Natives Menü abdunkeln** nutzt eine nicht dokumentierte Windows-Funktion (von vielen Apps genutzt). Bei Fehlschlag bleibt es hell, funktional unverändert.
- **Zwei Fenster-Größen-Welten:** CI-Windows 1024×768, Linux 1280×800 → Breakpoints in beiden testen.
