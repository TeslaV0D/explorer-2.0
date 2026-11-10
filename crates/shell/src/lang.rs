//! The user's preferred UI languages (D-029).

/// Windows: `GetUserPreferredUILanguages` (e.g. `["de-DE", "en-US"]`),
/// most preferred first. Elsewhere empty (the app reads the POSIX variables).
pub fn ui_languages() -> Vec<String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
        let (mut count, mut len) = (0u32, 0u32);
        // SAFETY: size query with a null buffer, then a buffer of that size.
        unsafe {
            if GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                std::ptr::null_mut(),
                &mut len,
            ) == 0
                || len == 0
            {
                return Vec::new();
            }
            let mut buf = vec![0u16; len as usize];
            if GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                buf.as_mut_ptr(),
                &mut len,
            ) == 0
            {
                return Vec::new();
            }
            // multi-string: "de-DE\0en-US\0\0"
            buf.split(|&c| c == 0)
                .filter(|s| !s.is_empty())
                .map(String::from_utf16_lossy)
                .collect()
        }
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}
