//! Shell adapter: file icons and type names like Windows Explorer shows them.
//!
//! The platform-neutral part (icon keys, id registry, caches) is tested on
//! every platform; the Windows part asks the shell (`SHGetFileInfoW`). Other
//! platforms return `None`, and the UI keeps its built-in fallback look.

mod icons;
mod menu;
mod types;
#[cfg(windows)]
mod win;
#[cfg(windows)]
mod win_menu;

pub use icons::{
    bgra_to_rgba, icon_key, init_thread, load_icon, Icon, IconKey, IconRegistry, IconSize,
};
pub use menu::{
    client_to_screen, menu_entries, own_verb, show_context_menu, show_properties, MenuEntry,
    MenuOutcome, OwnVerb,
};
pub use types::{type_name, TypeNames};

/// True where real shell icons/type names are available.
pub const SHELL_SUPPORTED: bool = cfg!(windows);
