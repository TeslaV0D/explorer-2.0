//! UI language (D-029): picks the language at start and hands it to Slint
//! (`@tr`, bundled `.po`) and to Rust texts (`explorer_i18n::tr!`).

/// The user's preferred UI languages: Windows' list, else the POSIX variables.
pub fn system() -> Vec<String> {
    let win = explorer_shell::ui_languages();
    if !win.is_empty() {
        return win;
    }
    explorer_i18n::posix_languages(|k| std::env::var(k).ok())
}

/// Chooses and sets the language: `EXPLORER2_LANG` > setting > Windows language.
pub fn init(setting: &str) -> &'static str {
    let over = std::env::var("EXPLORER2_LANG").ok();
    let sys = system();
    let code = explorer_i18n::resolve(setting, over.as_deref(), &sys);
    explorer_i18n::set_language(code);
    crate::nav::trace_line(&format!(
        "language {code} (setting={setting:?} override={over:?} system={sys:?})"
    ));
    code
}

/// Switches the Slint texts; needs an existing window (Slint's rule).
pub fn apply_ui(code: &str) {
    match slint::select_bundled_translation(code) {
        Ok(()) => crate::nav::trace_line(&format!("language ui={code}")),
        Err(e) => crate::nav::trace_line(&format!("language ui={code} not bundled: {e}")),
    }
}
