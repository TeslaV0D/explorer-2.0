# Explorer 2.0

Schneller, moderner, robuster Ersatz für den Windows Explorer. Siehe `docs/PROJECT.md`, Plan in `docs/PLAN-BASICS.md`, Stand in `docs/STATUS.md`.

## Installieren

Neueste Version: **[Releases](https://github.com/TeslaV0D/explorer-2.0/releases/latest)** → `Explorer2-Setup-<Version>-x64.exe` herunterladen und doppelklicken (pro Benutzer, ohne Administratorrechte). Deinstallieren über „Einstellungen → Apps“. Details in den Release-Notes.

## Release erstellen

Version in `Cargo.toml` heben, committen, Tag `v<Version>` pushen → `.github/workflows/release.yml` baut, testet das Setup und veröffentlicht das Release.
