//! Windows part of `theme`: registry reads, `RegNotifyChangeKeyValue`
//! watcher, `DWMWA_USE_IMMERSIVE_DARK_MODE`.

use explorer_core::theme::{accent_from_palette, rgb_from_abgr, Accent, SystemTheme};
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_FAILED, WAIT_OBJECT_0};
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegGetValueW, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
    KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME, RRF_RT_REG_BINARY,
    RRF_RT_REG_DWORD,
};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForMultipleObjects, INFINITE};

const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const ACCENT: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent";
const DWM: &str = r"Software\Microsoft\Windows\DWM";
/// Watched (with subtree): covers Personalize; DWM changes with the accent.
const WATCH: [(&str, bool); 3] = [
    (r"Software\Microsoft\Windows\CurrentVersion\Themes", true),
    (DWM, false),
    (ACCENT, false),
];

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn dword(key: &str, value: &str) -> Option<u32> {
    let (k, v) = (wide(key), wide(value));
    let mut out = 0u32;
    let mut len = 4u32;
    // SAFETY: NUL-terminated names, out buffer of `len` bytes.
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            k.as_ptr(),
            v.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut out as *mut u32).cast(),
            &mut len,
        )
    };
    (rc == 0).then_some(out)
}

fn binary(key: &str, value: &str) -> Option<Vec<u8>> {
    let (k, v) = (wide(key), wide(value));
    let mut buf = vec![0u8; 64];
    let mut len = buf.len() as u32;
    // SAFETY: as above, `len` = buffer size in bytes.
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            k.as_ptr(),
            v.as_ptr(),
            RRF_RT_REG_BINARY,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut len,
        )
    };
    (rc == 0).then(|| {
        buf.truncate(len as usize);
        buf
    })
}

pub fn read() -> SystemTheme {
    // missing value = light (Windows' default)
    let dark = dword(PERSONALIZE, "AppsUseLightTheme") == Some(0);
    let accent = binary(ACCENT, "AccentPalette")
        .and_then(|b| accent_from_palette(&b))
        .or_else(|| dword(ACCENT, "AccentColorMenu").map(|v| Accent::plain(rgb_from_abgr(v))))
        .or_else(|| dword(DWM, "AccentColor").map(|v| Accent::plain(rgb_from_abgr(v))));
    SystemTheme { dark, accent }
}

pub fn watch(changed: impl Fn() + Send + 'static) {
    let spawn = std::thread::Builder::new()
        .name("theme-watch".into())
        .spawn(move || {
            let mut keys: Vec<(HKEY, HANDLE, bool)> = Vec::new();
            for (path, subtree) in WATCH {
                let p = wide(path);
                let mut key: HKEY = std::ptr::null_mut();
                // SAFETY: valid name/out pointer; keys live as long as the thread.
                if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, p.as_ptr(), 0, KEY_NOTIFY, &mut key) }
                    != 0
                {
                    continue; // e.g. no Accent key on Windows Server
                }
                // SAFETY: plain auto-reset event.
                let ev = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
                if ev.is_null() {
                    unsafe { RegCloseKey(key) };
                    continue;
                }
                keys.push((key, ev, subtree));
            }
            if keys.is_empty() {
                return;
            }
            let arm = |k: &(HKEY, HANDLE, bool)| {
                // SAFETY: open key + event; asynchronous → signals `ev` once.
                unsafe {
                    RegNotifyChangeKeyValue(
                        k.0,
                        k.2 as i32,
                        REG_NOTIFY_CHANGE_LAST_SET | REG_NOTIFY_CHANGE_NAME,
                        k.1,
                        1,
                    )
                }
            };
            for k in &keys {
                arm(k);
            }
            let events: Vec<HANDLE> = keys.iter().map(|k| k.1).collect();
            loop {
                // SAFETY: array of valid event handles.
                let r = unsafe {
                    WaitForMultipleObjects(events.len() as u32, events.as_ptr(), 0, INFINITE)
                };
                if r == WAIT_FAILED || r >= WAIT_OBJECT_0 + events.len() as u32 {
                    break;
                }
                // Settings writes several values in a row → settle, then re-arm
                // every key (a notification is one-shot) and report once.
                std::thread::sleep(Duration::from_millis(150));
                for k in &keys {
                    arm(k);
                }
                changed();
            }
            for (key, ev, _) in keys {
                unsafe {
                    RegCloseKey(key);
                    CloseHandle(ev);
                }
            }
        });
    let _ = spawn;
}

pub fn title_bar(hwnd: isize, dark: bool) -> Result<(), String> {
    let v: i32 = dark as i32;
    // SAFETY: attribute 20 takes a BOOL; a stale HWND only yields an error.
    let hr = unsafe {
        DwmSetWindowAttribute(
            hwnd as _,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            (&v as *const i32).cast(),
            4,
        )
    };
    if hr == 0 {
        Ok(())
    } else {
        Err(format!("DwmSetWindowAttribute failed: 0x{hr:08x}"))
    }
}
