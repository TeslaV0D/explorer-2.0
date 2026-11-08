//! Clipboard probe for CI interop tests with the Windows shell:
//!   clip set [--cut] <path>...   puts files on the clipboard (CF_HDROP + Preferred DropEffect)
//!   clip get                      prints "cut"/"copy" and one path per line
use explorer_ops::clipboard::{get_files, set_files, ClipFiles};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = std::io::stdout();
    match args.first().map(String::as_str) {
        Some("set") => {
            let cut = args.iter().any(|a| a == "--cut");
            let paths: Vec<PathBuf> = args[1..]
                .iter()
                .filter(|a| *a != "--cut")
                .map(PathBuf::from)
                .collect();
            if set_files(&ClipFiles { paths, cut }) {
                ExitCode::SUCCESS
            } else {
                let _ = writeln!(out, "set failed");
                ExitCode::FAILURE
            }
        }
        Some("get") => match get_files() {
            Some(c) => {
                let _ = writeln!(out, "{}", if c.cut { "cut" } else { "copy" });
                for p in c.paths {
                    let _ = writeln!(out, "{}", p.display());
                }
                ExitCode::SUCCESS
            }
            None => {
                let _ = writeln!(out, "empty");
                ExitCode::FAILURE
            }
        },
        _ => {
            let _ = writeln!(out, "usage: clip set [--cut] <path>... | clip get");
            ExitCode::from(2)
        }
    }
}
