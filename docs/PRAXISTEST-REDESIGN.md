# Praxistest Redesign (R10) – Checkliste für echtes Windows

Stand 2026-11-08 · Ziel: das neue Aussehen (File-Pilot-Stil, D-026) auf deinem
Rechner abnehmen. **Funktionen sind unverändert** – geprüft wird nur, ob alles
gut aussieht, lesbar ist und sich gleich (oder besser) bedienen lässt.

Exe: CI-Artefakt `explorer2-windows-x64` des letzten grünen Laufs (oder
installierte Version per Einstellungen → „Installieren / aktualisieren“).
Notausgang wie immer: Startmenü „Explorer 2.0 – Windows-Standard
wiederherstellen“ bzw. `explorer2-recover.exe --yes`.

## 1. Erster Eindruck (2 Min.)
- [ ] Start ist schnell (gefühlt wie vorher, Fenster sofort da).
- [ ] Neue Installation startet **dunkel**; eine früher gewählte Farbe bleibt.
- [ ] Schrift scharf (Segoe UI), keine Kästchen/Ersatzzeichen, keine abgeschnittenen Texte.
- [ ] Titelleiste passt zum Schema (dunkel/hell).

## 2. Hauptfenster
- [ ] **Keine Leiste oben** mehr: Datei-Aktionen (Neuer Ordner, Ausschneiden, Kopieren, Einfügen, Umbenennen, Löschen, ↶/↷) als Symbole rechts in der Navigationszeile; Tooltip zeigt Name + Kürzel.
- [ ] **⋮ „Mehr“**: Zwei Ansichten, Ausgeblendete Elemente, Inspector, Befehlspalette, Einstellungen.
- [ ] **Statusleiste** unten rechts: Sortieren, Details/Symbole, Inspector, Einstellungen.
- [ ] Fenster schmal ziehen (< 900 px): Seitenleiste wird schmaler; bei zwei Ansichten wandern die Datei-Aktionen ins ⋮-Menü – nichts überlappt.

## 3. Seitenleiste, Tabs, Navigation
- [ ] Seitenleiste: Linien-Symbole, aktiver Ort mit blauer Marke links, Laufwerke mit dünnem Füllbalken (rot ab 90 %).
- [ ] Tabs: aktueller Tab „verschmilzt“ mit der Navigationszeile, blauer Strich nur in der aktiven Ansicht; × erscheint beim Drüberfahren; Mittelklick schließt.
- [ ] Adresszeile: Segmente, aktueller Ordner hell; Klick in die freie Fläche = Pfad bearbeiten; Suchfeld mit Lupe.

## 4. Dateiliste
- [ ] Zeilen als abgerundete Plättchen; Auswahl blau mit Balken links; ohne Fokus grau; Fokusrahmen beim Navigieren mit Pfeiltasten.
- [ ] Spaltenkopf: Sortierpfeil (gezeichnet) an der sortierten Spalte; Spaltenbreite ziehen.
- [ ] Kachelansicht (Strg+Umschalt+2): gleicher Stil, Vorschaubilder.
- [ ] Ziehen auf einen Ordner: Ziel leuchtet; Rahmenauswahl mit der Maus.

## 5. Inspector (Alt+P)
- [ ] Vorschau oben, Gruppen ALLGEMEIN / DATUM / ORT, keine Linien je Zeile.
- [ ] „Pfad kopieren“ legt den Pfad in die Zwischenablage (in Notepad einfügen).

## 6. Menüs und Dialoge
- [ ] **Rechtsklick** = unser kompaktes Menü (Öffnen, Ausschneiden, Kopieren, Einfügen, Umbenennen, Löschen, Eigenschaften); auf freier Fläche: Neuer Ordner, Einfügen, Aktualisieren, Eigenschaften.
- [ ] **„Weitere Optionen“** öffnet das echte Windows-Menü (7-Zip, Git, „Senden an“ …) – **dunkel**, wenn die App dunkel ist.
- [ ] Shift+F10 öffnet das Menü; ↑/↓, Enter, Esc funktionieren.
- [ ] Löschen bestätigen (Umschalt+Entf), Konflikt beim Kopieren: Dialoge im neuen Stil, Enter/Esc wie gewohnt.
- [ ] Befehlspalette (Strg+Umschalt+P) und Schnellsuche (Strg+Umschalt+F): Lupe, blaue Auswahl, blendet weich ein.
- [ ] Einstellungen: alles lesbar, Farbschema Wie Windows / Hell / Dunkel wechselt sofort.

## 7. Anzeige-Skalierung
- [ ] Windows-Einstellungen → Anzeige → Skalierung **125 %** und **150 %**: Symbole scharf, nichts abgeschnitten, Klickflächen treffen.

## 8. Farben
- [ ] **Hell** und **Dunkel** beide angenehm lesbar (auch gedämpfte Texte wie Datum/Größe).
- [ ] „Windows-Akzentfarbe verwenden“ mit einer kräftigen Akzentfarbe (z. B. Gelb, Grün): Auswahl bleibt dezent und lesbar.

## Rückmeldung
Pro Punkt reicht „passt“ oder ein kurzer Satz bzw. Screenshot (Win+Umschalt+S).
Auffälligkeiten nehme ich als Feinschliff-Aufgaben auf.
