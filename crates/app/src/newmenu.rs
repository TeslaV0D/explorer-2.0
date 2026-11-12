//! Shell “New” entries, read once. Labels are translated when the menu opens,
//! so a language switch does not need another registry walk.

use explorer_core::newmenu::{NewSpec, ShellNewRecord};
use std::sync::OnceLock;

fn records() -> &'static [ShellNewRecord] {
    static CACHE: OnceLock<Vec<ShellNewRecord>> = OnceLock::new();
    CACHE.get_or_init(explorer_shell::shell_new_records)
}

pub fn specs() -> Vec<NewSpec> {
    explorer_core::newmenu::menu_specs(records(), &explorer_shell::shell_new_root())
}
