# Projektbeschreibung – Explorer 2.0

## Vision
Ein Dateimanager, der den Windows Explorer **vollständig** ersetzt: überall, wo Windows heute einen Explorer-Fenster öffnet, erscheint Explorer 2.0. Dabei deutlich schneller, stabiler (kein Hänger bei großen Ordnern/Netzlaufwerken) und moderner (Tabs, Split-Panels, Inspector, Command Palette, Tastatur-first).

## Ziele
1. **Geschwindigkeit:** Start, Ordnerwechsel, Scrollen, Sortieren fühlen sich instant an (Budgets in `PLAN-BASICS.md`).
2. **Funktionsparität** mit dem Windows Explorer: Ordner-/Dateioperationen, Kontextmenü (inkl. Shell-Erweiterungen Dritter), Eigenschaften, Papierkorb, Netzwerk, alle Ordneroptionen.
3. **Robustheit:** UI-Thread blockiert nie auf I/O; Fehler (Rechte, gesperrte Dateien, getrennte Netzlaufwerke) werden sauber behandelt; Undo für Operationen.
4. **Systemweiter Ersatz:** Ordner öffnen aus beliebigen Apps, „Im Ordner anzeigen“, Win+E, Datei-Dialoge (soweit technisch möglich) – siehe `WINDOWS-INTEGRATION.md`.
5. **Später:** Systemweite Sofortsuche (wie voidtools Everything) und Datei-Aktions-Shortcuts (wie Sweetch).

## Nicht-Ziele (vorerst)
- macOS/Linux-UI (Kernlogik bleibt aber portabel für Tests).
- Windows < 10.
- Inhaltsindizierung von Dateien (Phase 2 beginnt mit Namen, wie Everything).

## Phasen
| Phase | Inhalt |
|---|---|
| **1 – Basis** | Schneller, zuverlässiger Dateimanager + Shell-Integration (aktuell in Planung) |
| 2 – Suche | Eigener Index (NTFS MFT/USN), Instant-Suche, Filter, Everything-Syntax-Kompatibilität |
| 3 – Shortcuts | Radial-/Drag-Menü für Dateiaktionen (Konvertieren, Tools, Aktionen) à la Sweetch |
| 4 – Politur | Plugins, Sync der Settings, Telemetrie-frei, Auto-Update |

## Zielplattform
Windows 10 (1809+) & 11, x64 (ARM64 später). Single-Binary-Philosophie, Portable-Modus + Installer.
