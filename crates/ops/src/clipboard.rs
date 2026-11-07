//! System clipboard for files, compatible with Windows Explorer:
//! `CF_HDROP` (file list) + "Preferred DropEffect" (copy = 1, move/cut = 2).
//! On other platforms there is no system file clipboard here (the app keeps
//! an internal one).

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipFiles {
    pub paths: Vec<PathBuf>,
    /// true = cut (move on paste), false = copy.
    pub cut: bool,
}

/// Puts files on the system clipboard. Returns false if unsupported/failed.
pub fn set_files(files: &ClipFiles) -> bool {
    imp::set_files(files)
}

/// Reads files from the system clipboard (e.g. copied in Explorer).
pub fn get_files() -> Option<ClipFiles> {
    imp::get_files()
}

/// Empties the system clipboard (Explorer does this after pasting a cut).
pub fn clear() -> bool {
    imp::clear()
}

/// Encodes the CF_HDROP payload: DROPFILES header (20 bytes, wide) + double-NUL list.
pub fn encode_hdrop(paths: &[PathBuf]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&20u32.to_le_bytes()); // pFiles: offset of the file list
    out.extend_from_slice(&0i32.to_le_bytes()); // pt.x
    out.extend_from_slice(&0i32.to_le_bytes()); // pt.y
    out.extend_from_slice(&0i32.to_le_bytes()); // fNC
    out.extend_from_slice(&1i32.to_le_bytes()); // fWide
    for p in paths {
        for u in p.to_string_lossy().encode_utf16() {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// Decodes a wide CF_HDROP payload (inverse of `encode_hdrop`).
pub fn decode_hdrop(data: &[u8]) -> Option<Vec<PathBuf>> {
    let off = u32::from_le_bytes(data.get(0..4)?.try_into().ok()?) as usize;
    let wide = i32::from_le_bytes(data.get(16..20)?.try_into().ok()?) != 0;
    if !wide {
        return None; // ANSI lists are not produced by any modern app
    }
    let units: Vec<u16> = data
        .get(off..)?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .collect();
    let paths: Vec<PathBuf> = units
        .split(|&u| u == 0)
        .take_while(|s| !s.is_empty())
        .map(|s| PathBuf::from(String::from_utf16_lossy(s)))
        .collect();
    Some(paths)
}

#[cfg(not(windows))]
mod imp {
    use super::ClipFiles;
    pub fn set_files(_: &ClipFiles) -> bool {
        false
    }
    pub fn get_files() -> Option<ClipFiles> {
        None
    }
    pub fn clear() -> bool {
        false
    }
}

#[cfg(windows)]
mod imp {
    use super::{decode_hdrop, encode_hdrop, ClipFiles};
    use windows_sys::Win32::Foundation::{GlobalFree, HWND};
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
        OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
    };
    use windows_sys::Win32::System::Ole::CF_HDROP;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, HWND_MESSAGE,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    fn drop_effect_format() -> u32 {
        // SAFETY: NUL-terminated string.
        unsafe { RegisterClipboardFormatW(wide("Preferred DropEffect").as_ptr()) }
    }

    /// Opens the clipboard with a hidden owner window (SetClipboardData fails without an owner).
    struct Open(HWND);

    impl Open {
        fn new() -> Option<Self> {
            let class = wide("STATIC");
            // SAFETY: message-only window of a system class; destroyed in Drop.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    std::ptr::null(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    std::ptr::null_mut(),
                    GetModuleHandleW(std::ptr::null()),
                    std::ptr::null(),
                )
            };
            if hwnd.is_null() {
                return None;
            }
            // The clipboard can be held by another process for a moment → retry briefly.
            for _ in 0..20 {
                // SAFETY: valid window handle.
                if unsafe { OpenClipboard(hwnd) } != 0 {
                    return Some(Open(hwnd));
                }
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            // SAFETY: created above.
            unsafe { DestroyWindow(hwnd) };
            None
        }
    }

    impl Drop for Open {
        fn drop(&mut self) {
            // SAFETY: clipboard was opened by us; window created by us.
            unsafe {
                CloseClipboard();
                DestroyWindow(self.0);
            }
        }
    }

    fn put(format: u32, bytes: &[u8]) -> bool {
        // SAFETY: allocate, fill and hand over a movable global block; on failure we free it.
        unsafe {
            let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
            if h.is_null() {
                return false;
            }
            let p = GlobalLock(h) as *mut u8;
            if p.is_null() {
                GlobalFree(h);
                return false;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            GlobalUnlock(h);
            if SetClipboardData(format, h).is_null() {
                GlobalFree(h);
                return false;
            }
            true // the clipboard owns the memory now
        }
    }

    fn get(format: u32) -> Option<Vec<u8>> {
        // SAFETY: data handle is owned by the clipboard; we only lock/copy/unlock it.
        unsafe {
            if IsClipboardFormatAvailable(format) == 0 {
                return None;
            }
            let h = GetClipboardData(format);
            if h.is_null() {
                return None;
            }
            let size = GlobalSize(h);
            let p = GlobalLock(h) as *const u8;
            if p.is_null() {
                return None;
            }
            let v = std::slice::from_raw_parts(p, size).to_vec();
            GlobalUnlock(h);
            Some(v)
        }
    }

    pub fn set_files(files: &ClipFiles) -> bool {
        let Some(_open) = Open::new() else {
            return false;
        };
        // SAFETY: clipboard is open and owned by our window.
        if unsafe { EmptyClipboard() } == 0 {
            return false;
        }
        let effect: u32 = if files.cut { 2 } else { 1 };
        put(CF_HDROP as u32, &encode_hdrop(&files.paths))
            && put(drop_effect_format(), &effect.to_le_bytes())
    }

    pub fn clear() -> bool {
        let Some(_open) = Open::new() else {
            return false;
        };
        // SAFETY: clipboard is open.
        unsafe { EmptyClipboard() != 0 }
    }

    pub fn get_files() -> Option<ClipFiles> {
        let _open = Open::new()?;
        let paths = decode_hdrop(&get(CF_HDROP as u32)?)?;
        let cut = get(drop_effect_format())
            .and_then(|b| {
                b.get(0..4)
                    .map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]))
            })
            .is_some_and(|e| e & 2 != 0);
        Some(ClipFiles { paths, cut })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdrop_roundtrip() {
        let paths = vec![
            PathBuf::from(r"C:\Users\Boss\Bericht.docx"),
            PathBuf::from(r"D:\Ärger\日本.txt"),
        ];
        let enc = encode_hdrop(&paths);
        assert_eq!(&enc[0..4], &20u32.to_le_bytes());
        assert_eq!(decode_hdrop(&enc), Some(paths));
        assert_eq!(decode_hdrop(&encode_hdrop(&[])), Some(vec![]));
        assert_eq!(decode_hdrop(&[1, 2, 3]), None);
    }

    /// Real system clipboard (Windows): our own roundtrip. Interop with Explorer's
    /// format is checked in CI via PowerShell Get-/Set-Clipboard -Format FileDropList.
    #[cfg(windows)]
    #[test]
    fn system_clipboard_roundtrip() {
        let files = ClipFiles {
            paths: vec![
                std::env::temp_dir().join("a.txt"),
                std::env::temp_dir().join("b c.txt"),
            ],
            cut: true,
        };
        assert!(set_files(&files));
        assert_eq!(get_files(), Some(files.clone()));
        let copy = ClipFiles {
            cut: false,
            ..files
        };
        assert!(set_files(&copy));
        assert_eq!(get_files().map(|f| f.cut), Some(false));
    }
}
