//! `HKCR\.<ext>\ShellNew` → [`explorer_core::newmenu::ShellNewRecord`].
//!
//! COM-only handlers are not read (no `NullFile` / `FileName` / `Data` /
//! `Command` means the record is dropped later). `Exclude` drops the entry.
//! A menu name of the form `@dll,-id` is a resource reference: we use the
//! friendly type name instead, when there is one.

use explorer_core::newmenu::ShellNewRecord;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CLASSES_ROOT, KEY_READ,
    REG_BINARY, REG_EXPAND_SZ, REG_SZ,
};

const ERROR_NO_MORE_ITEMS: u32 = 259;

struct Key(HKEY);

impl Key {
    fn open(parent: HKEY, sub: &str) -> Option<Key> {
        let name = wide(sub);
        let mut out: HKEY = std::ptr::null_mut();
        // SAFETY: NUL-terminated name, out pointer valid; closed in Drop.
        let rc = unsafe { RegOpenKeyExW(parent, name.as_ptr(), 0, KEY_READ, &mut out) };
        (rc == 0).then_some(Key(out))
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the key was opened by RegOpenKeyExW.
        unsafe { RegCloseKey(self.0) };
    }
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

fn subkeys(key: HKEY) -> Vec<String> {
    let mut names = Vec::new();
    let mut i = 0u32;
    loop {
        let mut buf = [0u16; 256];
        let mut len = buf.len() as u32;
        // SAFETY: buffer and length match; class and time are unused.
        let rc = unsafe {
            RegEnumKeyExW(
                key,
                i,
                buf.as_mut_ptr(),
                &mut len,
                std::ptr::null_mut::<u32>(),
                std::ptr::null_mut::<u16>(),
                std::ptr::null_mut::<u32>(),
                std::ptr::null_mut::<FILETIME>(),
            )
        };
        if rc == ERROR_NO_MORE_ITEMS {
            break;
        }
        // a too-long name must not stop the rest of the list
        if rc != 0 {
            i += 1;
            if i > 100_000 {
                break;
            }
            continue;
        }
        names.push(String::from_utf16_lossy(&buf[..len as usize]));
        i += 1;
    }
    names
}

fn query(key: HKEY, name: &str) -> Option<(u32, Vec<u8>)> {
    let wide_name = wide(name);
    let mut kind = 0u32;
    let mut size = 0u32;
    // SAFETY: size query with a null buffer is the documented pattern.
    let rc = unsafe {
        RegQueryValueExW(
            key,
            wide_name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if rc != 0 && rc != 234 {
        return None;
    }
    if size == 0 {
        return Some((kind, Vec::new()));
    }
    if size > 1024 * 1024 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    let mut size2 = size;
    // SAFETY: buffer length is `size2`.
    let rc = unsafe {
        RegQueryValueExW(
            key,
            wide_name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            data.as_mut_ptr(),
            &mut size2,
        )
    };
    if rc != 0 {
        return None;
    }
    data.truncate(size2 as usize);
    Some((kind, data))
}

fn decode_sz(bytes: &[u8]) -> String {
    let u: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&c| c != 0)
        .collect();
    let s = String::from_utf16_lossy(&u);
    expand_env(&s)
}

fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        if let Some(end) = rest.find('%') {
            let name = &rest[..end];
            if let Ok(val) = std::env::var(name) {
                out.push_str(&val);
            } else {
                out.push('%');
                out.push_str(name);
                out.push('%');
            }
            rest = &rest[end + 1..];
        } else {
            out.push('%');
            out.push_str(rest);
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn value_present(key: HKEY, name: &str) -> bool {
    query(key, name).is_some()
}

fn value_text(key: HKEY, name: &str) -> String {
    match query(key, name) {
        Some((k, b)) if k == REG_SZ || k == REG_EXPAND_SZ => decode_sz(&b),
        _ => String::new(),
    }
}

fn friendly_name(ext_key: &Key, ext: &str) -> String {
    let progid = value_text(ext_key.0, "");
    if progid.is_empty() || progid.starts_with('@') || progid.contains('\\') {
        return String::new();
    }
    let Some(prog) = Key::open(HKEY_CLASSES_ROOT, &progid) else {
        return String::new();
    };
    let name = value_text(prog.0, "");
    if name.is_empty() || name.starts_with('@') || name.eq_ignore_ascii_case(ext) {
        String::new()
    } else {
        name
    }
}

fn one(ext_name: &str) -> Option<ShellNewRecord> {
    let ext = ext_name.strip_prefix('.')?.to_ascii_lowercase();
    if ext.is_empty() || ext.contains('.') || ext.contains('\\') {
        return None;
    }
    let ext_key = Key::open(HKEY_CLASSES_ROOT, ext_name)?;
    let shell_new = Key::open(ext_key.0, "ShellNew")?;
    if value_present(shell_new.0, "Exclude") {
        return None;
    }
    let command = value_text(shell_new.0, "Command");
    let file_name = value_text(shell_new.0, "FileName");
    let null_file = value_present(shell_new.0, "NullFile");
    let data = match query(shell_new.0, "Data") {
        Some((k, b)) if k == REG_SZ || k == REG_EXPAND_SZ => decode_sz(&b).into_bytes(),
        Some((k, b)) if k == REG_BINARY => b,
        Some((_, b)) => b,
        None => Vec::new(),
    };
    let mut menu_text = value_text(shell_new.0, "MenuText");
    if menu_text.is_empty() || menu_text.starts_with('@') {
        menu_text = value_text(shell_new.0, "ItemName");
    }
    if menu_text.starts_with('@') {
        menu_text.clear();
    }
    if menu_text.is_empty() {
        menu_text = friendly_name(&ext_key, &ext);
    }
    if command.trim().is_empty() && data.is_empty() && file_name.trim().is_empty() && !null_file {
        return None;
    }
    Some(ShellNewRecord {
        extension: ext,
        menu_text,
        command,
        data,
        file_name,
        null_file,
    })
}

pub fn read() -> Vec<ShellNewRecord> {
    read_root(HKEY_CLASSES_ROOT)
}

fn read_root(root: HKEY) -> Vec<ShellNewRecord> {
    subkeys(root)
        .into_iter()
        .filter(|n| n.starts_with('.'))
        .filter_map(|n| one(&n))
        .collect()
}
