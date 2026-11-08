# Explorer 2.0

Schneller, moderner, robuster Ersatz für den Windows Explorer. Siehe `docs/PROJECT.md`, Plan in `docs/PLAN-BASICS.md`, Stand in `docs/STATUS.md`.

## Installieren

Neueste Version: **[Releases](https://github.com/TeslaV0D/explorer-2.0/releases/latest)** → `Explorer2-Setup-<Version>-x64.exe` herunterladen und doppelklicken (pro Benutzer, ohne Administratorrechte). Deinstallieren über „Einstellungen → Apps“. Details in den Release-Notes.

## Release erstellen

Version in `Cargo.toml` heben und pushen (`main` oder `claude/**`) → `.github/workflows/release.yml` sieht eine Version ohne Release, baut, testet das Setup, legt den Tag `v<Version>` an und veröffentlicht das Release. Alternativ Tag `v<Version>` pushen oder den Workflow von Hand starten.
