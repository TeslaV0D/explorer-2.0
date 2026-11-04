//! Shell adapter: file icons and type names like Windows Explorer shows them.
//!
//! The platform-neutral part (icon keys, id registry, caches) is tested on
//! every platform; the Windows part asks the shell (`SHGetFileInfoW`). Other
//! platforms return `None`, and the UI keeps its built-in fallback look.

mod dnd;
mod icons;
mod libraries;
mod menu;
mod thumbs;
mod types;
#[cfg(windows)]
mod win;
#[cfg(windows)]
mod win_dnd;
#[cfg(windows)]
mod win_lib;
#[cfg(windows)]
mod win_menu;

pub use dnd::{
    default_effect, primary_button_down, register_drop_target, same_volume, start_drag, DropEffect,
    DropEvent, DropKeys, DropPhase,
};
pub use icons::{
    bgra_to_rgba, icon_key, init_thread, load_icon, Icon, IconKey, IconRegistry, IconSize,
    RECYCLE_BIN, THIS_PC,
};
pub use libraries::{libraries, library, stem_for_shell_name, Library};
pub use menu::{
    client_to_screen, menu_entries, own_verb, show_context_menu, show_properties, MenuEntry,
    MenuOutcome, OwnVerb,
};
pub use thumbs::{premultiplied_bgra_to_rgba, thumbnail, Thumb};
pub use types::{type_name, TypeNames};

/// Screen pixels → window-client pixels (Windows); identity elsewhere.
pub fn screen_to_client(hwnd: isize, x: i32, y: i32) -> (i32, i32) {
    #[cfg(windows)]
    {
        win_dnd::screen_to_client(hwnd, x, y)
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        (x, y)
    }
}

/// True where real shell icons/type names are available.
pub const SHELL_SUPPORTED: bool = cfg!(windows);
