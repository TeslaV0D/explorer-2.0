# Praxistest Suche (S1) auf echtem Windows

Ziel: Bevor wir über S2 (MFT/USN mit Admin) entscheiden, zeigt S1 auf deinem
PC, wie schnell und zuverlässig die Suche **ohne** Admin ist. Dauer: 15–20 min.

## 1. Holen und starten
- GitHub → Actions → letzter grüner Lauf auf `claude/relaxed-pasteur-ufhh1p`
  (CI-Run #93 oder neuer) → Artefakt **explorer2-windows-x64** laden, entpacken.
- Am besten mit Mitschrift starten (PowerShell im entpackten Ordner):

  ```powershell
  $env:EXPLORER2_TRACE_FILE = "$env:TEMP\explorer2-trace.txt"; .\explorer2.exe
  ```

  Ohne Mitschrift geht auch (Doppelklick); dann fehlen nur die Zahlen.
- Nichts wird installiert oder in Windows eingetragen, solange du es nicht
  ausdrücklich in den Einstellungen einschaltest.

## 2. Was du prüfst

| # | Schritt | Worauf achten |
|---|---|---|
| 1 | App starten, ~1 min warten, **Einstellungen → Suche** öffnen | Status pro Laufwerk: „wird aufgebaut … n Einträge“ → „bereit, n Einträge (aufgebaut in x s)“. **Wie lange für C:?** |
| 2 | Während des Aufbaus normal weiterarbeiten | Bleibt die App flüssig? Wird der PC spürbar langsamer? |
| 3 | **Strg+Umschalt+F**, z. B. `rechnung` tippen | Treffer < 1 s nach dem Tippen? Beste Treffer oben? Enter öffnet, Strg+Enter zeigt im Ordner |
| 4 | In einem Ordner **Strg+F**, etwas tippen | Nur Treffer darunter, Spalte „Ordner“, Treffer fett markiert, Rechtsklick/Kopieren/Löschen funktionieren |
| 5 | Abfragesprache: `ext:pdf dm:thisweek`, `size:>100mb`, `*.jpg urlaub` | Ergebnisse plausibel? Datum in Ortszeit? |
| 6 | Im Windows Explorer eine Datei anlegen, umbenennen, löschen | Taucht sie in einer offenen Suche nach ~1 s auf / verschwindet sie? |
| 7 | App schließen, wieder öffnen | Suche sofort wieder da („geladen“ statt „aufgebaut“)? |
| 8 | Optional: Netzlaufwerk/USB-Stick in den Einstellungen zuschalten | Aufbau, Suche, Abziehen des Sticks ohne Absturz |
| 9 | Speicher: Task-Manager → explorer2.exe | Arbeitsspeicher nach dem Aufbau (Ziel: < 100 MB pro 1 Mio Einträge) |

## 3. Was du mir schickst
- Kurz: Was war gut, was hat genervt, was fehlte?
- Die Zahlen aus den Einstellungen (Einträge, Aufbauzeit pro Laufwerk) und
  den Arbeitsspeicher.
- Falls mitgeschrieben: `%TEMP%\explorer2-trace.txt` (enthält Pfade deiner
  Dateien – vorher durchsehen oder nur die Zeilen mit `search:` schicken).
- Bei einem Absturz: den Ordner `%APPDATA%\Explorer2\crash`.

## 4. Danach entscheiden wir
- **S1 reicht** (Aufbau erträglich, Live-Updates zuverlässig): S2 bleibt
  liegen, weiter mit Phase 3 oder Beta-Paket (M8.4).
- **Aufbau zu langsam / Änderungen gehen verloren**: S2 (MFT lesen + USN-Journal,
  Dienst mit Admin, mit R-001-Rückbau) planen.

Notausgang wie immer: Einstellungen → „Alle Windows-Integrationen zurücksetzen“
bzw. `explorer2-recover.exe --yes`. Der Suchindex selbst liegt nur in
`%APPDATA%\Explorer2\index` (löschen = beim nächsten Start neu aufbauen).
