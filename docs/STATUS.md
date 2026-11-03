# Zwischenstand

**Stand:** 2026-11-03 · **Phase:** 1 – Planung · **Branch:** `claude/relaxed-pasteur-ufhh1p`

## Erledigt
- Repo-Analyse: nur LICENSE vorhanden, Greenfield.
- Referenzen ausgewertet: File Pilot (Panels/Tabs, Inspector, GoTo, Command Palette, ~2,5 MB), Sweetch (Kreismenü, .NET, MIT), Everything (MFT+USN, ~100 MB / 1 Mio Dateien).
- Doku angelegt: `CLAUDE.md` (Memory), `PROJECT`, `PLAN-BASICS`, `WINDOWS-INTEGRATION`, `TODO`, `STATUS`.
- Windows-Integration in 5 Stufen durchdacht (Registry-Verben → Win+E → explorer.exe-Proxy → Dialoge (Grenze) → Shell bleibt).

## Offen / blockiert
- **Entscheidung D-001 (Stack/UI-Framework)** – ohne die kein M0-Start. Empfehlung: Rust + Slint.
- Windows-Verifikation der ⚠️-Punkte nicht möglich in der Linux-Sandbox → CI + manuelle Tests durch Boss.

## Nächste Schritte
1. Boss: Plan lesen, D-001 entscheiden, Prioritäten anpassen.
2. M0: Workspace + CI + Fenster-Skelett.
3. M1: fs-Kern + Benchmarks.
