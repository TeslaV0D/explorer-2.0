//! `explorer2-recover` – puts Windows back to its own defaults (R-001).
//!
//! Console only, no UI/GPU: works when Explorer 2.0 itself does not start,
//! also in Safe Mode. Same reset as the app's settings button.
//!
//! ```text
//! explorer2-recover            show what is active, ask, reset
//! explorer2-recover --yes      reset without asking
//! explorer2-recover --status   only show (exit 1 = something is active)
//! ```
//! Exit codes: 0 ok / nothing active, 1 active (`--status`) or reset errors,
//! 2 usage.

use explorer_integrate::{reset, status, JournalPaths, MemRegistry, Registry};
use std::io::{self, BufRead, IsTerminal, Write};

const USAGE: &str = "Explorer 2.0 – Windows-Standard wiederherstellen

  explorer2-recover            zeigt den Stand, fragt nach und setzt zurück
  explorer2-recover --yes      setzt ohne Nachfrage zurück
  explorer2-recover --status   zeigt nur den Stand (Exit-Code 1 = aktiv)

Entfernt alle Windows-Integrationen von Explorer 2.0 (Ordner-Verben,
Win+E, Kontextmenü, Umgebungsvariablen, Verknüpfungen …) und stellt die
vorherigen Werte exakt wieder her. Werte, die inzwischen eine andere
Software geändert hat, bleiben unangetastet.";

fn out(s: &str) {
    // never panic on a closed console (GUI double click, redirected output)
    let _ = writeln!(io::stdout(), "{s}");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (yes, only_status) = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => (false, false),
        ["--yes"] | ["-y"] => (true, false),
        ["--status"] => (false, true),
        ["--help"] | ["-h"] | ["/?"] => {
            out(USAGE);
            return;
        }
        _ => {
            out(USAGE);
            std::process::exit(2);
        }
    };
    let Some(paths) = JournalPaths::default_paths() else {
        out("Fehler: Konfigurationsordner nicht gefunden (APPDATA fehlt).");
        std::process::exit(1);
    };
    let mut reg: Box<dyn Registry> = match explorer_integrate::system_registry() {
        Some(r) => r,
        // other platforms: no registry, files only
        None => Box::new(MemRegistry::new()),
    };
    out("Explorer 2.0 – Windows-Standard wiederherstellen");
    out(&format!("Journal: {}", paths.primary.display()));
    let st = status(reg.as_mut(), &paths);
    out(&format!("Stand: {}", st.summary_de()));
    if only_status {
        std::process::exit(if st.any() { 1 } else { 0 });
    }
    if !yes && io::stdin().is_terminal() {
        let _ = write!(
            io::stdout(),
            "Jetzt alles auf Windows-Standard zurücksetzen? [j/N] "
        );
        let _ = io::stdout().flush();
        let mut line = String::new();
        let _ = io::stdin().lock().read_line(&mut line);
        if !matches!(
            line.trim().to_lowercase().as_str(),
            "j" | "ja" | "y" | "yes"
        ) {
            out("Abgebrochen – nichts geändert.");
            return;
        }
    } else if !yes {
        out("Keine Konsole für die Nachfrage – bitte mit --yes starten.");
        std::process::exit(2);
    }
    let r = reset(reg.as_mut(), &paths);
    out(&r.summary_de());
    for k in &r.kept {
        out(&format!("  belassen: {k}"));
    }
    for e in &r.errors {
        out(&format!("  Fehler: {e}"));
    }
    std::process::exit(if r.ok() { 0 } else { 1 });
}
