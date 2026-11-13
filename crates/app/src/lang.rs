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

// ---------------------------------------------------------------- L4: live switch

use crate::MainWindow;
use slint::{Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

type Relabel = Rc<dyn Fn(&MainWindow)>;

thread_local! {
    /// Rust-built texts set once (renderer status, notices …): set again
    /// after a language switch.
    static KEEP: RefCell<Vec<(&'static str, Relabel)>> = const { RefCell::new(Vec::new()) };
    /// Setting values behind the entries of the language list.
    static CHOICES: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static NAMES: Rc<VecModel<SharedString>> = Rc::new(VecModel::default());
}

/// Sets a text now and again after every language switch (`key` replaces
/// an earlier registration of the same text).
pub fn keep(win: &MainWindow, key: &'static str, f: impl Fn(&MainWindow) + 'static) {
    f(win);
    KEEP.with_borrow_mut(|k| {
        k.retain(|(n, _)| *n != key);
        k.push((key, Rc::new(f)));
    });
}

/// Fills the language list of the settings (after `apply_ui`).
pub fn offer(win: &MainWindow) {
    let setting = crate::config::load().language;
    let choices = explorer_i18n::choices(&setting);
    let auto = explorer_i18n::resolve("", None, &explorer_i18n::system_languages());
    let names: Vec<SharedString> = choices
        .iter()
        .map(|c| match explorer_i18n::lang(c) {
            Some(l) => l.native.into(),
            None => explorer_i18n::tr!(
                "System default ({0})",
                explorer_i18n::lang(auto).map_or(auto, |l| l.native)
            )
            .into(),
        })
        .collect();
    let index = explorer_i18n::normalize(&setting)
        .and_then(|c| choices.iter().position(|x| *x == c))
        .unwrap_or(0);
    NAMES.with(|m| {
        if m.row_count() == names.len() {
            for (i, n) in names.into_iter().enumerate() {
                if m.row_data(i).as_ref() != Some(&n) {
                    m.set_row_data(i, n);
                }
            }
        } else {
            m.set_vec(names);
        }
        win.set_language_names(ModelRc::from(m.clone()));
    });
    win.set_language_index(index as i32);
    CHOICES.with_borrow_mut(|c| *c = choices);
}

/// An entry of the language list was picked: save it and switch live.
pub fn choose(win: &MainWindow, index: usize) {
    let Some(setting) = CHOICES.with_borrow(|c| c.get(index).copied()) else {
        return;
    };
    if let Err(e) = crate::config::update(|s| s.language = setting.to_string()) {
        win.set_error(e.into());
    }
    // a choice made here wins over EXPLORER2_LANG (that only picks the start)
    let code = explorer_i18n::resolve(setting, None, &explorer_i18n::system_languages());
    crate::nav::trace_line(&format!("language switch setting={setting:?} -> {code}"));
    switch(win, code);
}

/// Date column fits the widest stamp of the current language.
pub fn date_column(win: &MainWindow) {
    win.set_date_sample(explorer_i18n::widest_stamp().into());
}

/// Shows the whole window in `code` without a restart.
pub fn switch(win: &MainWindow, code: &'static str) {
    explorer_i18n::set_language(code);
    apply_ui(code);
    date_column(win);
    offer(win);
    let keep: Vec<Relabel> = KEEP.with_borrow(|k| k.iter().map(|(_, f)| f.clone()).collect());
    for f in keep {
        f(win);
    }
    crate::nav::relabel(win);
    crate::search::relabel(win);
    crate::inspector::relabel(win);
    if win.get_settings_open() {
        crate::integration::refresh(win);
    }
    crate::nav::trace_line(&format!("language switched to {code}"));
}

/// Palette commands `language-<code>` / `language-system`.
pub fn choose_code(win: &MainWindow, code: &str) {
    let setting = if code == "system" { "" } else { code };
    let index = CHOICES.with_borrow(|c| c.iter().position(|x| *x == setting));
    match index {
        Some(i) => {
            win.set_language_index(i as i32);
            choose(win, i);
        }
        None => crate::nav::trace_line(&format!("language {code}: not offered")),
    }
}
