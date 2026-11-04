//! Type names ("Textdokument", "JPEG-Bild") like the Explorer "Typ" column.
//! The shell answer depends only on the extension (and folder/file), so it
//! is cached per extension; a cache miss is one shell call.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Thread-safe cache: lower-case extension → shell type name.
#[derive(Clone, Default)]
pub struct TypeNames(Arc<Mutex<HashMap<String, Option<String>>>>);

impl TypeNames {
    /// Process-wide cache (loader threads of all tabs share it).
    pub fn global() -> &'static TypeNames {
        static G: OnceLock<TypeNames> = OnceLock::new();
        G.get_or_init(TypeNames::default)
    }

    /// Shell type name for a file with extension `ext` (any case, no dot),
    /// `None` if the shell has none (or on other platforms).
    pub fn file(&self, ext: &str) -> Option<String> {
        let ext = ext.to_ascii_lowercase();
        if let Some(v) = self.0.lock().unwrap_or_else(|e| e.into_inner()).get(&ext) {
            return v.clone();
        }
        let v = lookup(&ext, false);
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(ext, v.clone());
        v
    }
}

/// Shell type name of a folder (`is_dir`) or of a file with extension `ext`.
pub fn type_name(ext: &str, is_dir: bool) -> Option<String> {
    if is_dir {
        static DIR: OnceLock<Option<String>> = OnceLock::new();
        return DIR.get_or_init(|| lookup("", true)).clone();
    }
    TypeNames::global().file(ext)
}

fn lookup(ext: &str, is_dir: bool) -> Option<String> {
    #[cfg(windows)]
    {
        crate::win::type_name(ext, is_dir)
    }
    #[cfg(not(windows))]
    {
        let _ = (ext, is_dir);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_is_case_insensitive_and_consistent() {
        let t = TypeNames::default();
        assert_eq!(t.file("TXT"), t.file("txt"));
        assert_eq!(t.0.lock().unwrap().len(), 1);
    }

    #[cfg(not(windows))]
    #[test]
    fn none_without_shell() {
        assert_eq!(type_name("txt", false), None);
        assert_eq!(type_name("", true), None);
    }

    /// Real shell names (German or English CI image) – just plausibility.
    #[cfg(windows)]
    #[test]
    fn windows_names_are_plausible() {
        let txt = type_name("txt", false).expect("txt has a type name");
        let dir = type_name("", true).expect("folders have a type name");
        assert!(!txt.is_empty() && txt != dir, "{txt} / {dir}");
        let unknown = type_name("zzqqxx", false).expect("unknown ext still gets 'ZZQQXX File'");
        assert!(unknown.to_lowercase().contains("zzqqxx"), "{unknown}");
    }
}
