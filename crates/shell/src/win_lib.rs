//! `IShellLibrary` reading (Windows).

use crate::libraries::Library;
use std::path::{Path, PathBuf};
use windows::core::{HSTRING, PWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER, STGM_READ,
};
use windows::Win32::UI::Shell::{
    FOLDERID_Libraries, IShellItem, IShellItemArray, IShellLibrary, SHCreateItemFromParsingName,
    SHGetKnownFolderPath, ShellLibrary, KF_FLAG_DEFAULT, LFF_FORCEFILESYSTEM, SIGDN,
    SIGDN_FILESYSPATH, SIGDN_NORMALDISPLAY,
};

fn take(p: PWSTR) -> Option<String> {
    if p.is_null() {
        return None;
    }
    // SAFETY: shell-allocated NUL-terminated string, freed right after copying.
    let s = unsafe { p.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(p.0 as *const _)) };
    s
}

fn name(item: &IShellItem, kind: SIGDN) -> Option<String> {
    // SAFETY: plain COM call on a valid item.
    take(unsafe { item.GetDisplayName(kind) }.ok()?)
}

fn libraries_dir() -> Option<PathBuf> {
    // SAFETY: valid GUID; the returned string is freed by `take`.
    let p = unsafe { SHGetKnownFolderPath(&FOLDERID_Libraries, KF_FLAG_DEFAULT, None) }.ok()?;
    take(p).map(PathBuf::from)
}

fn read(file: &Path) -> Option<Library> {
    crate::init_thread(); // COM on this (loader) thread
    let stem = file.file_stem()?.to_string_lossy().into_owned();
    // SAFETY: COM calls with valid arguments; interfaces are released on drop.
    unsafe {
        let item: IShellItem =
            SHCreateItemFromParsingName(&HSTRING::from(file.as_os_str()), None).ok()?;
        let lib: IShellLibrary =
            CoCreateInstance(&ShellLibrary, None, CLSCTX_INPROC_SERVER).ok()?;
        lib.LoadLibraryFromItem(&item, STGM_READ.0).ok()?;
        let display = name(&item, SIGDN_NORMALDISPLAY).unwrap_or_else(|| stem.clone());
        let mut folders = Vec::new();
        if let Ok(arr) = lib.GetFolders::<IShellItemArray>(LFF_FORCEFILESYSTEM) {
            for i in 0..arr.GetCount().unwrap_or(0) {
                if let Some(p) = arr
                    .GetItemAt(i)
                    .ok()
                    .and_then(|it| name(&it, SIGDN_FILESYSPATH))
                {
                    folders.push(PathBuf::from(p));
                }
            }
        }
        Some(Library {
            stem,
            display,
            folders,
            file: file.to_path_buf(),
        })
    }
}

pub fn libraries() -> Vec<Library> {
    let Some(dir) = libraries_dir() else {
        return Vec::new();
    };
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<Library> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("library-ms"))
        })
        .filter_map(|p| read(&p))
        .collect();
    out.sort_by_key(|l| l.display.to_lowercase());
    out
}

pub fn library(stem: &str) -> Option<Library> {
    read(&libraries_dir()?.join(format!("{stem}.library-ms")))
}

#[cfg(test)]
mod tests {
    #[test]
    fn libraries_read_without_crashing() {
        // runners may or may not have the default libraries; when they do,
        // every listed folder is an absolute path
        for l in super::libraries() {
            eprintln!("library {} ({}): {:?}", l.display, l.stem, l.folders);
            assert!(l.folders.iter().all(|f| f.is_absolute()), "{l:?}");
            assert_eq!(
                super::library(&l.stem).map(|x| x.folders),
                Some(l.folders.clone())
            );
        }
    }
}
