//! Shell adapter: file icons and type names like Windows Explorer shows them.
//!
//! The platform-neutral part (icon keys, id registry, caches) is tested on
//! every platform; the Windows part asks the shell (`SHGetFileInfoW`). Other
//! platforms return `None`, and the UI keeps its built-in fallback look.

mod icons;
mod types;
#[cfg(windows)]
mod win;

pub use icons::{
    bgra_to_rgba, icon_key, init_thread, load_icon, Icon, IconKey, IconRegistry, IconSize,
};
pub use types::{type_name, TypeNames};

/// True where real shell icons/type names are available.
pub const SHELL_SUPPORTED: bool = cfg!(windows);
