//! UI language (D-029): picks the language at start and hands it to Slint
//! (`@tr`, bundled `.po`) and to Rust texts (`explorer_i18n::tr!`).

/// Chooses and sets the language: `EXPLORER2_LANG` > setting > Windows language.
pub fn init(setting: &str) -> &'static str {
    let code = explorer_i18n::init(setting);
    crate::nav::trace_line(&format!(
        "language {code} (setting={setting:?} override={:?} system={:?})",
        std::env::var("EXPLORER2_LANG").ok(),
        explorer_i18n::system_languages()
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
