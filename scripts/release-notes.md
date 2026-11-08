## Installieren

1. **`Explorer2-Setup-@VERSION@-x64.exe`** herunterladen und doppelklicken.
2. Windows zeigt evtl. „Der Computer wurde durch Windows geschützt“ (die Datei ist noch nicht signiert) → **Weitere Informationen → Trotzdem ausführen**.
3. Bestätigen – fertig. Keine Administratorrechte nötig.

Installiert wird nach `%LOCALAPPDATA%\Programs\Explorer2`, mit Startmenü-Eintrag „Explorer 2.0“ und Eintrag unter „Einstellungen → Apps“. Ein neueres Setup über eine bestehende Installation laufen lassen = Update (auch wenn Explorer 2.0 gerade offen ist).

**Windows-Integration** (Ordner per Doppelklick in Explorer 2.0 öffnen, Win+E) ist nach der Installation **aus** – in den Einstellungen einzeln einschalten. Alles lässt sich mit einem Knopf zurücksetzen: in der App, über den Startmenü-Eintrag „Explorer 2.0 – Windows-Standard wiederherstellen“ oder notfalls mit `explorer2-recover.exe --yes`.

## Deinstallieren

„Einstellungen → Apps → Explorer 2.0 → Deinstallieren“. Das setzt alle Windows-Integrationen zurück und entfernt Programm, Startmenü und Eintrag. Deine Einstellungen (`%APPDATA%\Explorer2`) bleiben.

## Portable

`Explorer2-@VERSION@-x64-portable.zip` entpacken und `explorer2.exe` starten – ohne Installation. Installieren geht später in der App (Einstellungen → Installation).

## Prüfsummen

`SHA256SUMS.txt` – prüfen mit `Get-FileHash .\Explorer2-Setup-@VERSION@-x64.exe`.

Voraussetzung: Windows 10/11 x64.
