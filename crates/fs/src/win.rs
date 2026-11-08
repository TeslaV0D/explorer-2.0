//! Windows fast path: FindFirstFileExW with basic info level and large fetch.

use crate::path::{filetime_to_unix, to_verbatim};
use crate::FsError;
use explorer_core::{Entry, EntryKind};
use std::io;
use std::ops::ControlFlow;
use std::path::Path;
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
    FIND_FIRST_EX_LARGE_FETCH, WIN32_FIND_DATAW,
};

const ATTR_READONLY: u32 = 0x1;
const ATTR_HIDDEN: u32 = 0x2;
const ATTR_SYSTEM: u32 = 0x4;
const ATTR_DIRECTORY: u32 = 0x10;
const ATTR_REPARSE_POINT: u32 = 0x400;
const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;

struct FindHandle(HANDLE);

impl Drop for FindHandle {
    fn drop(&mut self) {
        // SAFETY: handle came from a successful FindFirstFileExW and is closed once.
        unsafe { FindClose(self.0) };
    }
}

pub fn enumerate(dir: &Path, f: &mut dyn FnMut(Entry) -> ControlFlow<()>) -> Result<(), FsError> {
    let abs = std::path::absolute(dir)?;
    let mut pattern = to_verbatim(&abs.to_string_lossy());
    if !pattern.ends_with('\\') {
        pattern.push('\\');
    }
    pattern.push('*');
    let wide: Vec<u16> = pattern.encode_utf16().chain(Some(0)).collect();

    let mut data = WIN32_FIND_DATAW::default();
    // SAFETY: `wide` is NUL-terminated, `data` is a valid WIN32_FIND_DATAW.
    let h = unsafe {
        FindFirstFileExW(
            wide.as_ptr(),
            FindExInfoBasic,
            (&mut data as *mut WIN32_FIND_DATAW).cast(),
            FindExSearchNameMatch,
            std::ptr::null(),
            FIND_FIRST_EX_LARGE_FETCH,
        )
    };
    if h == INVALID_HANDLE_VALUE {
        // SAFETY: trivial FFI call.
        let err = unsafe { GetLastError() };
        // An empty drive root has no "."/".." and reports FILE_NOT_FOUND.
        if err == ERROR_FILE_NOT_FOUND && dir.is_dir() {
            return Ok(());
        }
        return Err(io::Error::from_raw_os_error(err as i32).into());
    }
    let handle = FindHandle(h);
    loop {
        if let Some(e) = to_entry(&data) {
            if f(e).is_break() {
                return Ok(());
            }
        }
        // SAFETY: valid handle and buffer.
        if unsafe { FindNextFileW(handle.0, &mut data) } == 0 {
            // SAFETY: trivial FFI call.
            let err = unsafe { GetLastError() };
            if err == ERROR_NO_MORE_FILES {
                return Ok(());
            }
            return Err(io::Error::from_raw_os_error(err as i32).into());
        }
    }
}

fn to_entry(d: &WIN32_FIND_DATAW) -> Option<Entry> {
    let len = d
        .cFileName
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(d.cFileName.len());
    let name = String::from_utf16_lossy(&d.cFileName[..len]);
    if name == "." || name == ".." || name.is_empty() {
        return None;
    }
    let a = d.dwFileAttributes;
    let is_dir = a & ATTR_DIRECTORY != 0;
    // Only real links count; OneDrive/cloud placeholders are reparse points too.
    let is_link = a & ATTR_REPARSE_POINT != 0
        && matches!(
            d.dwReserved0,
            IO_REPARSE_TAG_SYMLINK | IO_REPARSE_TAG_MOUNT_POINT
        );
    let mtime = (u64::from(d.ftLastWriteTime.dwHighDateTime) << 32)
        | u64::from(d.ftLastWriteTime.dwLowDateTime);
    Some(Entry {
        name,
        kind: if is_dir {
            EntryKind::Dir
        } else {
            EntryKind::File
        },
        size: if is_dir {
            0
        } else {
            (u64::from(d.nFileSizeHigh) << 32) | u64::from(d.nFileSizeLow)
        },
        modified: filetime_to_unix(mtime),
        hidden: a & ATTR_HIDDEN != 0,
        system: a & ATTR_SYSTEM != 0,
        readonly: a & ATTR_READONLY != 0,
        is_link,
    })
}
