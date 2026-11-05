//! The real registry (64-bit view), raw values via `RegQueryValueExW`.

use crate::model::{Hive, KeyPath, RegData};
use crate::registry::Registry;
use std::io;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, LPARAM, WPARAM};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyExW, RegDeleteTreeW, RegDeleteValueW, RegOpenKeyExW,
    RegQueryInfoKeyW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE, KEY_WOW64_64KEY,
    REG_OPTION_NON_VOLATILE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};

pub struct WinRegistry;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn root(h: Hive) -> HKEY {
    match h {
        Hive::CurrentUser => HKEY_CURRENT_USER,
        Hive::LocalMachine => HKEY_LOCAL_MACHINE,
    }
}

fn missing(rc: u32) -> bool {
    rc == ERROR_FILE_NOT_FOUND || rc == ERROR_PATH_NOT_FOUND
}

fn err(rc: u32) -> io::Error {
    io::Error::from_raw_os_error(rc as i32)
}

/// Open key (closed on drop).
struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: an open key from RegOpenKeyExW/RegCreateKeyExW.
        unsafe { RegCloseKey(self.0) };
    }
}

fn open(key: &KeyPath, access: u32) -> io::Result<Option<Key>> {
    let p = wide(&key.path);
    let mut h: HKEY = std::ptr::null_mut();
    // SAFETY: valid NUL-terminated path, out pointer.
    let rc = unsafe {
        RegOpenKeyExW(
            root(key.hive),
            p.as_ptr(),
            0,
            access | KEY_WOW64_64KEY,
            &mut h,
        )
    };
    match rc {
        0 => Ok(Some(Key(h))),
        rc if missing(rc) => Ok(None),
        rc => Err(err(rc)),
    }
}

impl Registry for WinRegistry {
    fn get(&self, key: &KeyPath, name: &str) -> io::Result<Option<RegData>> {
        let Some(k) = open(key, KEY_QUERY_VALUE)? else {
            return Ok(None);
        };
        let n = wide(name);
        loop {
            let (mut kind, mut len) = (0u32, 0u32);
            // SAFETY: size query (null data pointer).
            let rc = unsafe {
                RegQueryValueExW(
                    k.0,
                    n.as_ptr(),
                    std::ptr::null(),
                    &mut kind,
                    std::ptr::null_mut(),
                    &mut len,
                )
            };
            if missing(rc) {
                return Ok(None);
            }
            if rc != 0 {
                return Err(err(rc));
            }
            let mut buf = vec![0u8; len as usize];
            let mut got = len;
            // SAFETY: buffer of `got` bytes.
            let rc = unsafe {
                RegQueryValueExW(
                    k.0,
                    n.as_ptr(),
                    std::ptr::null(),
                    &mut kind,
                    buf.as_mut_ptr(),
                    &mut got,
                )
            };
            match rc {
                0 => {
                    buf.truncate(got as usize);
                    return Ok(Some(RegData { kind, bytes: buf }));
                }
                234 => continue, // ERROR_MORE_DATA: grew in between
                rc => return Err(err(rc)),
            }
        }
    }

    fn set(&mut self, key: &KeyPath, name: &str, data: &RegData) -> io::Result<()> {
        let p = wide(&key.path);
        let mut h: HKEY = std::ptr::null_mut();
        // SAFETY: valid path/out pointers; creates missing keys.
        let rc = unsafe {
            RegCreateKeyExW(
                root(key.hive),
                p.as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE | KEY_WOW64_64KEY,
                std::ptr::null(),
                &mut h,
                std::ptr::null_mut(),
            )
        };
        if rc != 0 {
            return Err(err(rc));
        }
        let k = Key(h);
        let n = wide(name);
        // SAFETY: data pointer/length of our buffer.
        let rc = unsafe {
            RegSetValueExW(
                k.0,
                n.as_ptr(),
                0,
                data.kind,
                data.bytes.as_ptr(),
                data.bytes.len() as u32,
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(err(rc))
        }
    }

    fn delete_value(&mut self, key: &KeyPath, name: &str) -> io::Result<()> {
        let Some(k) = open(key, KEY_SET_VALUE)? else {
            return Ok(());
        };
        let n = wide(name);
        // SAFETY: open key, NUL-terminated name.
        match unsafe { RegDeleteValueW(k.0, n.as_ptr()) } {
            0 => Ok(()),
            rc if missing(rc) => Ok(()),
            rc => Err(err(rc)),
        }
    }

    fn key_exists(&self, key: &KeyPath) -> io::Result<bool> {
        Ok(open(key, KEY_READ)?.is_some())
    }

    fn key_is_empty(&self, key: &KeyPath) -> io::Result<bool> {
        let Some(k) = open(key, KEY_READ)? else {
            return Ok(true);
        };
        let (mut subkeys, mut values) = (0u32, 0u32);
        use std::ptr::null_mut;
        // SAFETY: only the two counters are requested.
        let rc = unsafe {
            RegQueryInfoKeyW(
                k.0,
                null_mut(),
                null_mut(),
                null_mut(),
                &mut subkeys,
                null_mut(),
                null_mut(),
                &mut values,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        if rc != 0 {
            return Err(err(rc));
        }
        Ok(subkeys == 0 && values == 0)
    }

    fn delete_empty_key(&mut self, key: &KeyPath) -> io::Result<()> {
        if !self.key_is_empty(key)? {
            return Err(io::Error::other("key not empty"));
        }
        let p = wide(&key.path);
        // SAFETY: valid path; fails for keys with subkeys anyway.
        match unsafe { RegDeleteKeyExW(root(key.hive), p.as_ptr(), KEY_WOW64_64KEY, 0) } {
            0 => Ok(()),
            rc if missing(rc) => Ok(()),
            rc => Err(err(rc)),
        }
    }

    fn delete_tree(&mut self, key: &KeyPath) -> io::Result<()> {
        if !self.key_exists(key)? {
            return Ok(());
        }
        let p = wide(&key.path);
        // SAFETY: valid path below the hive root (never the root itself).
        let rc = unsafe { RegDeleteTreeW(root(key.hive), p.as_ptr()) };
        if rc != 0 && !missing(rc) {
            return Err(err(rc));
        }
        // RegDeleteTreeW may leave the (now empty) key itself
        self.delete_empty_key(key)
    }

    fn notify_environment(&mut self) {
        let what = wide("Environment");
        let mut result = 0usize;
        // SAFETY: broadcast with a string lParam, bounded by the timeout.
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0 as WPARAM,
                what.as_ptr() as LPARAM,
                SMTO_ABORTIFHUNG,
                5000,
                &mut result,
            )
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real registry: apply → reset gives back the exact original, incl. a
    /// pre-existing value we overwrote and keys we created.
    #[test]
    fn real_registry_round_trip() {
        let mut reg = WinRegistry;
        let base = KeyPath::hkcu(&format!(
            r"Software\Explorer2UnitTest-{}",
            std::process::id()
        ));
        let pre = base.child("pre");
        reg.set(&pre, "keep", &RegData::sz("original")).unwrap();
        reg.set(&pre, "num", &RegData::dword(7)).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let paths = crate::JournalPaths::in_dir(dir.path());
        let changes = vec![
            crate::Change::SetValue {
                key: pre.clone(),
                name: "keep".into(),
                data: RegData::sz(r#""C:\x\explorer2.exe" "%1""#),
            },
            crate::Change::SetValue {
                key: base.child(r"new\deep\command"),
                name: "".into(),
                data: RegData::sz("ours"),
            },
        ];
        crate::apply(&mut reg, &paths, &changes).unwrap();
        assert_eq!(
            reg.get(&base.child(r"new\deep\command"), "").unwrap(),
            Some(RegData::sz("ours"))
        );
        // no sweep of the known points: never touch a real integration
        let r = crate::reset_with(&mut reg, &paths, &[]);
        assert!(r.ok(), "{r:?}");
        assert_eq!(
            reg.get(&pre, "keep").unwrap(),
            Some(RegData::sz("original"))
        );
        assert_eq!(reg.get(&pre, "num").unwrap(), Some(RegData::dword(7)));
        assert!(
            !reg.key_exists(&base.child("new")).unwrap(),
            "created keys removed"
        );
        reg.delete_tree(&base).unwrap();
        assert!(!reg.key_exists(&base).unwrap());
    }
}
