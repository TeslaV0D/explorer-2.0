# Phase 2 – Systemweite Sofortsuche (Everything-Stil)

Stand: 2026-11-05 · Boss-Entscheidungen: **Index stufenweise** (D-024), **Suchfeld pro Tab + globale Schnellsuche** (D-025).

## Ziele (messbar)
| Ziel | Budget |
|---|---|
| Suche beim Tippen über ~1 Mio Einträge | **< 10 ms** pro Anfrage (best-of-N, CI-Benchmark mit synthetischem Index) |
| Speicher für ~1 Mio Einträge | **< 100 MB** (Namen + Baum + Größe + Datum) |
| Erstindex ohne Admin (Verzeichnis-Durchlauf, NTFS-SSD) | 1 Mio Einträge **< 2 min**, im Hintergrund, App sofort benutzbar |
| Start mit gespeichertem Index | Index geladen **< 1 s**, danach Abgleich im Hintergrund |
| Live-Updates (Datei angelegt/umbenannt/gelöscht) | sichtbar **< 1 s** |

## Stufe S1 – ohne Admin (jetzt)
- **S1.1 Index-Kern** (neue Crate `explorer-index`, plattformneutral, voll unter Linux testbar):
  kompakte Struktur (Namen in einem Byte-Arena-Puffer, Knoten = Elternindex + Namensbereich + Größe + Änderungsdatum + Flags), Pfad über die Elternkette, Kleinschreib-Kopie der Namen für schnelles Matching, Einfügen/Löschen/Umbenennen für Live-Updates.
  **Abfragesprache** (Everything-Teilmenge): Wörter = UND, `|` = ODER, `!` = NICHT, `"…"` Phrase, `*`/`?` Platzhalter, `ext:jpg;png`, `size:>10mb`, `dm:today|yesterday|2026-11-05|>2026-01`, `file:`/`folder:`, `path:` (Treffer im ganzen Pfad), `regex:`. Parallele Suche über Blöcke (std-Threads), Ergebnisse sortierbar (Name, Pfad, Größe, Datum).
  Tests + Zufallsdaten-Tests (Parser) + Benchmark 1 Mio synthetische Einträge (CI-Gate).
- **S1.2 Persistenz**: Binärdatei pro Laufwerk in `%LOCALAPPDATA%\Explorer2\index\` (Kopf mit Version + Volume-ID + Prüfsumme, atomar geschrieben, beschädigt = neu bauen, nie Absturz), Ausschlusslisten in den Einstellungen.
- **S1.3 Aufbau + Live**: paralleler Durchlauf je Laufwerk (FindFirstFileExW Large Fetch, D-005; Arbeits-Queue über N Threads, niedrige Priorität), Fortschritt; Live-Änderungen über rekursives `ReadDirectoryChangesW` auf den Laufwerkswurzeln (Puffer-Überlauf → Teil-Rescan des Ordners); beim Start mit gespeichertem Index Abgleich über Ordner-Änderungszeiten (nur geänderte Ordner neu lesen). Netz/USB optional + abschaltbar.
- **S1.4 UI**: Suchfeld in jedem Tab (Strg+F / F3) – sucht ab dem aktuellen Ordner, Treffer als virtueller Ort im Tab mit **allen** Dateifunktionen (Öffnen, Kopieren, Löschen, Umbenennen, Kontextmenü, Inspector) und Spalte „Ordner“; globale Schnellsuche (Strg+Umschalt+F in der App, Overlay wie die Palette) über alle Laufwerke. Status „Index: n Einträge, aktuell / wird aufgebaut x %“.
- **S1.5 Feinschliff**: Treffer-Hervorhebung, Sortierung per Spaltenkopf, Verlauf der Suchen, Linux-E2E + Windows-CI (echter Baum, Live-Update, Persistenz, Neustart).

## Stufe S2 – optional, mit Admin (später, eigene Entscheidung)
NTFS-MFT-Scan + USN-Journal über einen kleinen Dienst (einmal Admin, opt-in). Pflicht vorher: R-001-Erweiterung für Dienste (Registrierung im Journal, Recovery entfernt den Dienst, auch im abgesicherten Modus). Erst nach Praxistest von S1.

## Nicht in Phase 2
Inhaltssuche (Volltext), systemweiter Hotkey außerhalb der App (wäre eine Integration → R-001, später).
