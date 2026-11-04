//! Windows shell calls. Everything here is a thin wrapper; the logic lives in
//! the platform-neutral modules.

use crate::icons::{bgra_to_rgba, Icon, IconKey, IconSize};
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
};
use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL};
use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows_sys::Win32::UI::Shell::{
    SHGetFileInfoW, SHFILEINFOW, SHGFI_ADDOVERLAYS, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SMALLICON,
    SHGFI_TYPENAME, SHGFI_USEFILEATTRIBUTES,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

fn wide(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn wide_str(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn init_thread() {
    // SAFETY: plain call; S_FALSE / RPC_E_CHANGED_MODE (already initialized) are fine.
    unsafe {
        let _ = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
    }
}

/// `SHGetFileInfoW` into a zeroed struct; returns false on failure.
fn file_info(path: &[u16], attrs: u32, flags: u32, out: &mut SHFILEINFOW) -> bool {
    // SAFETY: path is NUL-terminated, out is a valid struct of the given size.
    unsafe {
        SHGetFileInfoW(
            path.as_ptr(),
            attrs,
            out,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        ) != 0
    }
}

pub fn type_name(ext: &str, is_dir: bool) -> Option<String> {
    // USEFILEATTRIBUTES: the file need not exist, the answer depends on the extension only.
    let (name, attrs) = if is_dir {
        (wide_str("folder"), FILE_ATTRIBUTE_DIRECTORY)
    } else if ext.is_empty() {
        (wide_str("file"), FILE_ATTRIBUTE_NORMAL)
    } else {
        (wide_str(&format!("file.{ext}")), FILE_ATTRIBUTE_NORMAL)
    };
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    if !file_info(
        &name,
        attrs,
        SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES,
        &mut info,
    ) {
        return None;
    }
    let len = info
        .szTypeName
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(info.szTypeName.len());
    let s = String::from_utf16_lossy(&info.szTypeName[..len]);
    (!s.is_empty()).then_some(s)
}

/// Icon of a shell object given by parsing name (recycle bin, This PC).
fn shell_object_icon(name: &str, size_flag: u32) -> Option<Icon> {
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows_sys::Win32::UI::Shell::{SHParseDisplayName, SHGFI_PIDL};
    let w = wide_str(name);
    let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
    // SAFETY: NUL-terminated name, out pointer; the PIDL is freed below.
    let hr = unsafe {
        SHParseDisplayName(
            w.as_ptr(),
            std::ptr::null_mut(),
            &mut pidl,
            0,
            std::ptr::null_mut(),
        )
    };
    if hr < 0 || pidl.is_null() {
        return None;
    }
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    // SAFETY: with SHGFI_PIDL the first argument is the PIDL.
    let ok = unsafe {
        SHGetFileInfoW(
            pidl as *const u16,
            0,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_PIDL | SHGFI_ICON | size_flag,
        )
    } != 0;
    unsafe { CoTaskMemFree(pidl as *const c_void) };
    if !ok || info.hIcon.is_null() {
        return None;
    }
    let icon = icon_pixels(info.hIcon);
    // SAFETY: we own the icon returned by SHGetFileInfoW.
    unsafe { DestroyIcon(info.hIcon) };
    icon
}

pub fn load_icon(key: &IconKey, size: IconSize) -> Option<Icon> {
    let size_flag = match size {
        IconSize::Small => SHGFI_SMALLICON,
        IconSize::Large => SHGFI_LARGEICON,
    };
    if let IconKey::Shell(name) = key {
        return shell_object_icon(name, size_flag);
    }
    let (path, attrs, flags) = match key {
        IconKey::Folder => (
            wide_str("folder"),
            FILE_ATTRIBUTE_DIRECTORY,
            SHGFI_USEFILEATTRIBUTES,
        ),
        IconKey::Ext(e) if e.is_empty() => (
            wide_str("file"),
            FILE_ATTRIBUTE_NORMAL,
            SHGFI_USEFILEATTRIBUTES,
        ),
        IconKey::Ext(e) => (
            wide_str(&format!("file.{e}")),
            FILE_ATTRIBUTE_NORMAL,
            SHGFI_USEFILEATTRIBUTES,
        ),
        // real file: its own icon + overlays (shortcut arrow, …)
        IconKey::File(p) => (wide(p), 0, SHGFI_ADDOVERLAYS),
        IconKey::Shell(_) => return None, // handled above
    };
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    if !file_info(&path, attrs, SHGFI_ICON | size_flag | flags, &mut info) || info.hIcon.is_null() {
        return None;
    }
    let icon = icon_pixels(info.hIcon);
    // SAFETY: we own the icon returned by SHGetFileInfoW.
    unsafe { DestroyIcon(info.hIcon) };
    icon
}

/// Reads a 32-bit top-down copy of a bitmap.
fn bitmap_bits(hbm: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    if hbm.is_null() {
        return None;
    }
    // SAFETY: GDI calls with valid handles and buffers sized from GetObjectW.
    unsafe {
        let mut bm: BITMAP = std::mem::zeroed();
        if GetObjectW(
            hbm,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bm as *mut _ as *mut c_void,
        ) == 0
        {
            return None;
        }
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        if w <= 0 || h <= 0 || w > 1024 || h > 1024 {
            return None;
        }
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        let lines = GetDIBits(
            dc,
            hbm,
            0,
            h as u32,
            buf.as_mut_ptr() as *mut c_void,
            &mut bi,
            DIB_RGB_COLORS,
        );
        DeleteDC(dc);
        (lines == h).then_some((w as u32, h as u32, buf))
    }
}

/// The shell's image for a file fitting `px`×`px`: a real preview if a
/// thumbnail provider has one (`preview = true`), else the file's icon.
/// Machines without providers (e.g. Windows Server) only get icons.
pub fn thumbnail(path: &Path, px: u32) -> Option<crate::thumbs::Thumb> {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF, SIIGBF_ICONONLY,
        SIIGBF_RESIZETOFIT, SIIGBF_THUMBNAILONLY,
    };
    init_thread();
    // SAFETY: COM calls with valid arguments.
    let f: IShellItemImageFactory =
        unsafe { SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None) }.ok()?;
    let size = SIZE {
        cx: px as i32,
        cy: px as i32,
    };
    let get = |flags: SIIGBF| -> Option<Icon> {
        // SAFETY: valid factory; the returned bitmap is deleted right after copying.
        let hbm = unsafe { f.GetImage(size, flags) }.ok()?;
        let raw = hbm.0 as HBITMAP;
        let bits = bitmap_bits(raw);
        unsafe { DeleteObject(raw) };
        let (w, h, bgra) = bits?;
        Some(Icon {
            width: w,
            height: h,
            rgba: crate::thumbs::premultiplied_bgra_to_rgba(&bgra),
        })
    };
    if let Some(image) = get(SIIGBF(SIIGBF_RESIZETOFIT.0 | SIIGBF_THUMBNAILONLY.0)) {
        return Some(crate::thumbs::Thumb {
            image,
            preview: true,
        });
    }
    get(SIIGBF(SIIGBF_RESIZETOFIT.0 | SIIGBF_ICONONLY.0)).map(|image| crate::thumbs::Thumb {
        image,
        preview: false,
    })
}

fn icon_pixels(hicon: HICON) -> Option<Icon> {
    // SAFETY: GetIconInfo hands us two bitmaps that we must delete.
    let mut ii: ICONINFO = unsafe { std::mem::zeroed() };
    if unsafe { GetIconInfo(hicon, &mut ii) } == 0 {
        return None;
    }
    let color = bitmap_bits(ii.hbmColor);
    let mask = bitmap_bits(ii.hbmMask);
    unsafe {
        if !ii.hbmColor.is_null() {
            DeleteObject(ii.hbmColor);
        }
        if !ii.hbmMask.is_null() {
            DeleteObject(ii.hbmMask);
        }
    }
    let (w, h, bgra) = color?; // monochrome icons (no color bitmap) are not worth supporting
    let mask = mask
        .filter(|(mw, mh, _)| *mw == w && *mh == h)
        .map(|(_, _, m)| m);
    Some(Icon {
        width: w,
        height: h,
        rgba: bgra_to_rgba(&bgra, mask.as_deref()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnails_of_a_picture_and_a_text_file() {
        // 64×32 solid red 24-bit BMP, written by hand (no image crate needed)
        let dir = std::env::temp_dir().join(format!("e2-thumb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (w, h) = (64u32, 32u32);
        let row = (w * 3).div_ceil(4) * 4;
        let size = 54 + row * h;
        let mut b = Vec::new();
        b.extend_from_slice(b"BM");
        for v in [size, 0, 54, 40, w, h] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        for v in [0u32, row * h, 2835, 2835, 0, 0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for _ in 0..h {
            for _ in 0..w {
                b.extend_from_slice(&[0, 0, 255]); // BGR red
            }
            b.resize(b.len() + (row - w * 3) as usize, 0);
        }
        let bmp = dir.join("rot.bmp");
        std::fs::write(&bmp, &b).unwrap();
        let thumb = thumbnail(&bmp, 96).expect("thumbnail or icon of a bmp");
        let t = thumb.image;
        if !thumb.preview {
            // no thumbnail provider on this machine (GitHub's Windows Server
            // runners): the icon fallback is the right answer here
            eprintln!(
                "no picture thumbnail provider: got the icon ({}x{})",
                t.width, t.height
            );
            assert!(t.width > 16 && t.width <= 96, "large icon: {}", t.width);
            return;
        }
        assert!(
            t.width <= 96 && t.height <= 96 && t.width > 0,
            "{}x{}",
            t.width,
            t.height
        );
        assert_eq!(t.rgba.len(), (t.width * t.height * 4) as usize);
        let c = ((t.height / 2 * t.width + t.width / 2) * 4) as usize;
        let px = &t.rgba[c..c + 4];
        assert!(
            px[0] > 200 && px[1] < 60 && px[2] < 60 && px[3] > 200,
            "center pixel {px:?}"
        );
        let txt = dir.join("a.txt");
        std::fs::write(&txt, "x").unwrap();
        let i = thumbnail(&txt, 48).expect("files without preview get their icon");
        assert!(!i.preview, "a text file has no picture preview");
        let i = i.image;
        assert!(
            i.width > 16 && i.width <= 48,
            "large icon, not the 16 px one: {}",
            i.width
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn real_icons_load() {
        init_thread();
        for key in [
            IconKey::Folder,
            IconKey::Ext("txt".into()),
            IconKey::Ext(String::new()),
            IconKey::Shell(crate::icons::RECYCLE_BIN.into()),
            IconKey::Shell(crate::icons::THIS_PC.into()),
            IconKey::File(r"C:\".into()),
        ] {
            for size in [IconSize::Small, IconSize::Large] {
                let i =
                    load_icon(&key, size).unwrap_or_else(|| panic!("no icon for {key:?} {size:?}"));
                let px = if size == IconSize::Small { 16 } else { 32 };
                assert_eq!((i.width, i.height), (px, px), "{key:?}");
                assert_eq!(i.rgba.len(), (px * px * 4) as usize);
                assert!(
                    i.rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 0),
                    "{key:?} has visible pixels"
                );
            }
        }
        let exe = std::env::current_exe().unwrap();
        assert!(
            load_icon(&IconKey::File(exe), IconSize::Small).is_some(),
            "own exe icon"
        );
        // folder and txt icons differ
        let f = load_icon(&IconKey::Folder, IconSize::Small).unwrap();
        let t = load_icon(&IconKey::Ext("txt".into()), IconSize::Small).unwrap();
        assert_ne!(f.rgba, t.rgba);
    }
}
