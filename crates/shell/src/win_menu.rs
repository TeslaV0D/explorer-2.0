//! Windows: shell context menu via `IShellFolder::GetUIObjectOf` /
//! `CreateViewObject` → `IContextMenu`, properties via `SHObjectProperties` /
//! `SHMultiFileProperties`.

use crate::menu::{own_verb, MenuEntry, MenuOutcome};
use std::cell::RefCell;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::core::{Interface, PCSTR, PCWSTR, PSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, IDataObject, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    DefSubclassProc, IContextMenu, IContextMenu2, IContextMenu3, ILFindLastID, ILFree,
    IShellFolder, RemoveWindowSubclass, SHGetDesktopFolder, SHMultiFileProperties,
    SHObjectProperties, SHParseDisplayName, SetWindowSubclass, CMF_CANRENAME, CMF_EXPLORE,
    CMF_NORMAL, CMIC_MASK_PTINVOKE, CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX, GCS_VERBW,
    SHOP_FILEPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetMenuItemCount, GetMenuItemID, GetMenuStringW,
    SetForegroundWindow, TrackPopupMenuEx, HMENU, MF_BYPOSITION, SW_SHOWNORMAL, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM, WM_MENUCHAR,
};

/// `CMIC_MASK_UNICODE` (= SEE_MASK_UNICODE): use the wide fields.
const CMIC_MASK_UNICODE: u32 = 0x0000_4000;
const FIRST_CMD: u32 = 1;
const LAST_CMD: u32 = 0x7FFF;
const SUBCLASS_ID: usize = 0xE2C7;

fn wide(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn com_init() {
    // SAFETY: plain call; already-initialized results are fine.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

/// Owned absolute PIDL.
struct Pidl(*mut ITEMIDLIST);

impl Pidl {
    fn parse(p: &Path) -> Result<Pidl, String> {
        let w = wide(p);
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        // SAFETY: NUL-terminated path, out pointer valid.
        unsafe { SHParseDisplayName(PCWSTR(w.as_ptr()), None, &mut pidl, 0, None) }
            .map_err(|e| format!("{}: {e}", p.display()))?;
        Ok(Pidl(pidl))
    }
}

impl Drop for Pidl {
    fn drop(&mut self) {
        // SAFETY: allocated by SHParseDisplayName.
        unsafe { ILFree(Some(self.0)) }
    }
}

/// The context menu for `items` in `dir` (or of the folder background).
fn context_menu(hwnd: HWND, dir: &Path, items: &[PathBuf]) -> Result<IContextMenu, String> {
    com_init();
    let e = |e: windows::core::Error| e.message();
    // SAFETY: COM calls on valid interfaces; PIDLs outlive the calls.
    unsafe {
        let desktop = SHGetDesktopFolder().map_err(e)?;
        let dir_pidl = Pidl::parse(dir)?;
        let folder: IShellFolder = desktop.BindToObject(dir_pidl.0, None).map_err(e)?;
        if items.is_empty() {
            return folder.CreateViewObject::<IContextMenu>(hwnd).map_err(e);
        }
        let full: Vec<Pidl> = items
            .iter()
            .map(|p| Pidl::parse(p))
            .collect::<Result<_, _>>()?;
        let children: Vec<*const ITEMIDLIST> = full
            .iter()
            .map(|p| ILFindLastID(p.0) as *const ITEMIDLIST)
            .collect();
        folder
            .GetUIObjectOf::<IContextMenu>(hwnd, &children, None)
            .map_err(e)
    }
}

/// Any UI object (context menu, data object, …) of `items` (all in `dir`).
pub(crate) fn ui_object<T: Interface>(
    hwnd: HWND,
    dir: &Path,
    items: &[PathBuf],
) -> Result<T, String> {
    com_init();
    let e = |e: windows::core::Error| e.message();
    // SAFETY: COM calls on valid interfaces; PIDLs outlive the calls.
    unsafe {
        let desktop = SHGetDesktopFolder().map_err(e)?;
        let dir_pidl = Pidl::parse(dir)?;
        let folder: IShellFolder = desktop.BindToObject(dir_pidl.0, None).map_err(e)?;
        let full: Vec<Pidl> = items
            .iter()
            .map(|p| Pidl::parse(p))
            .collect::<Result<_, _>>()?;
        let children: Vec<*const ITEMIDLIST> = full
            .iter()
            .map(|p| ILFindLastID(p.0) as *const ITEMIDLIST)
            .collect();
        folder.GetUIObjectOf::<T>(hwnd, &children, None).map_err(e)
    }
}

fn verb_of(cm: &IContextMenu, cmd: u32) -> String {
    let mut buf = [0u16; 128];
    // SAFETY: GCS_VERBW writes a wide string of at most `cchmax` chars.
    let ok = unsafe {
        cm.GetCommandString(
            (cmd - FIRST_CMD) as usize,
            GCS_VERBW,
            None,
            PSTR(buf.as_mut_ptr() as *mut u8),
            buf.len() as u32,
        )
    };
    if ok.is_err() {
        return String::new();
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

fn build(cm: &IContextMenu) -> Result<HMENU, String> {
    // SAFETY: fresh menu handed to the shell to fill.
    unsafe {
        let menu = CreatePopupMenu().map_err(|e| e.message())?;
        let hr = cm.QueryContextMenu(
            menu,
            0,
            FIRST_CMD,
            LAST_CMD,
            CMF_NORMAL | CMF_EXPLORE | CMF_CANRENAME,
        );
        if hr.is_err() {
            let _ = DestroyMenu(menu);
            return Err(hr.message());
        }
        Ok(menu)
    }
}

thread_local! {
    /// Menu shown right now (owner-drawn submenus need our window messages).
    static ACTIVE: RefCell<Option<IContextMenu>> = const { RefCell::new(None) };
}

unsafe extern "system" fn subclass(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if matches!(
        msg,
        WM_INITMENUPOPUP | WM_DRAWITEM | WM_MEASUREITEM | WM_MENUCHAR
    ) {
        let handled = ACTIVE.with_borrow(|cm| {
            let cm = cm.as_ref()?;
            if let Ok(cm3) = cm.cast::<IContextMenu3>() {
                let mut res = LRESULT(0);
                // SAFETY: forwarding the message as documented for IContextMenu3.
                if unsafe { cm3.HandleMenuMsg2(msg, wp, lp, Some(&mut res)) }.is_ok() {
                    return Some(res);
                }
            } else if let Ok(cm2) = cm.cast::<IContextMenu2>() {
                // SAFETY: as above.
                if unsafe { cm2.HandleMenuMsg(msg, wp, lp) }.is_ok() {
                    return Some(LRESULT(0));
                }
            }
            None
        });
        if let Some(r) = handled {
            return r;
        }
    }
    // SAFETY: default chain.
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}

pub fn show(hwnd: isize, dir: &Path, items: &[PathBuf], at: (i32, i32)) -> MenuOutcome {
    let hwnd = HWND(hwnd as *mut _);
    let cm = match context_menu(hwnd, dir, items) {
        Ok(cm) => cm,
        Err(e) => return MenuOutcome::Failed(e),
    };
    let menu = match build(&cm) {
        Ok(m) => m,
        Err(e) => return MenuOutcome::Failed(e),
    };
    ACTIVE.with_borrow_mut(|a| *a = Some(cm.clone()));
    // SAFETY: modal menu on our own window; subclass removed right after.
    let cmd = unsafe {
        let _ = SetWindowSubclass(hwnd, Some(subclass), SUBCLASS_ID, 0);
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenuEx(
            menu,
            (TPM_RETURNCMD | TPM_RIGHTBUTTON).0,
            at.0,
            at.1,
            hwnd,
            None,
        )
        .0 as u32;
        let _ = RemoveWindowSubclass(hwnd, Some(subclass), SUBCLASS_ID);
        cmd
    };
    ACTIVE.with_borrow_mut(|a| *a = None);
    let outcome = if cmd < FIRST_CMD {
        MenuOutcome::Dismissed
    } else {
        let verb = verb_of(&cm, cmd);
        match own_verb(&verb) {
            Some(v) => MenuOutcome::Own(v),
            None => invoke(&cm, hwnd, cmd, at, verb),
        }
    };
    // SAFETY: menu created in build().
    unsafe {
        let _ = DestroyMenu(menu);
    }
    outcome
}

fn invoke(cm: &IContextMenu, hwnd: HWND, cmd: u32, at: (i32, i32), verb: String) -> MenuOutcome {
    let offset = (cmd - FIRST_CMD) as usize;
    let info = CMINVOKECOMMANDINFOEX {
        cbSize: std::mem::size_of::<CMINVOKECOMMANDINFOEX>() as u32,
        fMask: CMIC_MASK_UNICODE | CMIC_MASK_PTINVOKE,
        hwnd,
        // MAKEINTRESOURCE(offset): command by position, not by verb string
        lpVerb: PCSTR(offset as *const u8),
        lpVerbW: PCWSTR(offset as *const u16),
        nShow: SW_SHOWNORMAL.0,
        ptInvoke: POINT { x: at.0, y: at.1 },
        ..Default::default()
    };
    // SAFETY: the EX struct starts with the base struct (documented cast).
    match unsafe { cm.InvokeCommand(&info as *const _ as *const CMINVOKECOMMANDINFO) } {
        Ok(()) => MenuOutcome::Shell(verb),
        Err(e) => MenuOutcome::Failed(e.message()),
    }
}

pub fn client_to_screen(hwnd: isize, x: i32, y: i32) -> (i32, i32) {
    let mut p = POINT { x, y };
    // SAFETY: valid window handle (or harmless failure leaves the point unchanged).
    unsafe {
        let _ = windows::Win32::Graphics::Gdi::ClientToScreen(HWND(hwnd as *mut _), &mut p);
    }
    (p.x, p.y)
}

pub fn entries(dir: &Path, items: &[PathBuf]) -> Result<Vec<MenuEntry>, String> {
    let cm = context_menu(HWND::default(), dir, items)?;
    let menu = build(&cm)?;
    let mut out = Vec::new();
    // SAFETY: reading our own menu.
    unsafe {
        let n = GetMenuItemCount(Some(menu));
        for i in 0..n.max(0) {
            let mut buf = [0u16; 256];
            let len = GetMenuStringW(menu, i as u32, Some(&mut buf), MF_BYPOSITION);
            let label = String::from_utf16_lossy(&buf[..len.max(0) as usize]);
            let id = GetMenuItemID(menu, i);
            let verb = if id != u32::MAX && id >= FIRST_CMD {
                verb_of(&cm, id)
            } else {
                String::new()
            };
            if !label.is_empty() || !verb.is_empty() {
                out.push(MenuEntry { label, verb });
            }
        }
        let _ = DestroyMenu(menu);
    }
    Ok(out)
}

pub fn properties(hwnd: isize, items: &[PathBuf]) -> Result<(), String> {
    com_init();
    let hwnd = HWND(hwnd as *mut _);
    match items {
        [] => Ok(()),
        [one] => {
            let w = wide(one);
            // SAFETY: NUL-terminated path; the dialog runs modeless in the shell.
            let ok = unsafe {
                SHObjectProperties(
                    Some(hwnd),
                    SHOP_FILEPATH,
                    PCWSTR(w.as_ptr()),
                    PCWSTR::null(),
                )
            };
            if ok.as_bool() {
                Ok(())
            } else {
                Err(format!("Eigenschaften nicht verfügbar: {}", one.display()))
            }
        }
        many => {
            let dir = many[0].parent().ok_or("kein Ordner")?;
            let e = |e: windows::core::Error| e.message();
            // SAFETY: as in context_menu.
            unsafe {
                let data: IDataObject = ui_object(hwnd, dir, many)?;
                SHMultiFileProperties(&data, 0).map_err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_menu_for_a_file_and_the_background() {
        let d = std::env::temp_dir().join(format!("e2-menu-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("a.txt");
        std::fs::write(&f, b"x").unwrap();
        let items = entries(&d, std::slice::from_ref(&f)).expect("file menu");
        let verbs: Vec<&str> = items.iter().map(|e| e.verb.as_str()).collect();
        eprintln!("file menu: {items:?}");
        assert!(items.len() >= 5, "a real menu: {items:?}");
        for v in ["open", "delete", "rename", "properties"] {
            assert!(
                verbs.iter().any(|x| x.eq_ignore_ascii_case(v)),
                "verb {v} missing: {verbs:?}"
            );
        }
        let bg = entries(&d, &[]).expect("background menu");
        eprintln!("background menu: {bg:?}");
        assert!(!bg.is_empty(), "background menu has entries");
        let _ = std::fs::remove_dir_all(&d);
    }
}
