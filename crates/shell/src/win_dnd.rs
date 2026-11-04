//! Windows OLE drag & drop: our `IDropTarget` on the main window and
//! `SHDoDragDrop` (shell data object + default drag image) for drags out.

use crate::dnd::{DropEffect, DropEvent, DropKeys, DropPhase};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use windows::core::{implement, Ref};
use windows::Win32::Foundation::{HWND, POINT, POINTL};
use windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::Ole::{
    IDropTarget, IDropTarget_Impl, OleInitialize, RegisterDragDrop, ReleaseStgMedium,
    RevokeDragDrop, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
    DROPEFFECT_NONE,
};
use windows::Win32::System::SystemServices::{MK_CONTROL, MK_SHIFT, MODIFIERKEYS_FLAGS};
use windows::Win32::UI::Shell::{DragQueryFileW, SHDoDragDrop, HDROP};

pub type Handler = Box<dyn Fn(&DropEvent) -> DropEffect>;

fn to_ole(e: DropEffect) -> DROPEFFECT {
    match e {
        DropEffect::None => DROPEFFECT_NONE,
        DropEffect::Copy => DROPEFFECT_COPY,
        DropEffect::Move => DROPEFFECT_MOVE,
    }
}

fn from_ole(e: DROPEFFECT) -> DropEffect {
    if e.0 & DROPEFFECT_MOVE.0 != 0 {
        DropEffect::Move
    } else if e.0 & DROPEFFECT_COPY.0 != 0 {
        DropEffect::Copy
    } else {
        DropEffect::None
    }
}

/// File paths of a data object (CF_HDROP), empty if it carries none.
pub fn hdrop_paths(data: &IDataObject) -> Vec<PathBuf> {
    let fmt = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    // SAFETY: standard GetData/ReleaseStgMedium pairing; HDROP read via DragQueryFileW.
    unsafe {
        let Ok(mut medium) = data.GetData(&fmt) else {
            return Vec::new();
        };
        let hdrop = HDROP(medium.u.hGlobal.0);
        let n = DragQueryFileW(hdrop, u32::MAX, None);
        let mut out = Vec::with_capacity(n as usize);
        for i in 0..n {
            let len = DragQueryFileW(hdrop, i, None) as usize;
            let mut buf = vec![0u16; len + 1];
            let got = DragQueryFileW(hdrop, i, Some(&mut buf)) as usize;
            out.push(PathBuf::from(String::from_utf16_lossy(&buf[..got])));
        }
        ReleaseStgMedium(&mut medium);
        out
    }
}

#[implement(IDropTarget)]
pub struct Target {
    handler: Handler,
    paths: RefCell<Vec<PathBuf>>,
    allowed: Cell<u32>,
}

impl Target {
    pub fn new(handler: Handler) -> Self {
        Target {
            handler,
            paths: RefCell::new(Vec::new()),
            allowed: Cell::new(0),
        }
    }

    fn ask(&self, phase: DropPhase, keys: MODIFIERKEYS_FLAGS, pt: &POINTL) -> DropEffect {
        let allowed = self.allowed.get();
        let paths = self.paths.borrow();
        let ev = DropEvent {
            phase,
            paths: &paths,
            screen: (pt.x, pt.y),
            keys: DropKeys {
                ctrl: keys.0 & MK_CONTROL.0 != 0,
                shift: keys.0 & MK_SHIFT.0 != 0,
            },
            allow_copy: allowed & (DROPEFFECT_COPY.0 | DROPEFFECT_LINK.0) != 0,
            allow_move: allowed & DROPEFFECT_MOVE.0 != 0,
        };
        if paths.is_empty() {
            return DropEffect::None; // text, images, … are not files
        }
        (self.handler)(&ev)
    }
}

impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(
        &self,
        data: Ref<IDataObject>,
        keys: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        *self.paths.borrow_mut() = data.as_ref().map(hdrop_paths).unwrap_or_default();
        // SAFETY: OLE passes a valid in/out pointer.
        unsafe {
            self.allowed.set((*effect).0);
            *effect = to_ole(self.ask(DropPhase::Over, keys, pt));
        }
        Ok(())
    }

    fn DragOver(
        &self,
        keys: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        // SAFETY: as above.
        unsafe {
            self.allowed.set((*effect).0);
            *effect = to_ole(self.ask(DropPhase::Over, keys, pt));
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        let none = POINTL { x: 0, y: 0 };
        let _ = self.ask(DropPhase::Leave, MODIFIERKEYS_FLAGS(0), &none);
        self.paths.borrow_mut().clear();
        Ok(())
    }

    fn Drop(
        &self,
        data: Ref<IDataObject>,
        keys: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        *self.paths.borrow_mut() = data.as_ref().map(hdrop_paths).unwrap_or_default();
        // SAFETY: as above.
        unsafe {
            self.allowed.set((*effect).0);
            *effect = to_ole(self.ask(DropPhase::Drop, keys, pt));
        }
        self.paths.borrow_mut().clear();
        Ok(())
    }
}

pub fn register(hwnd: isize, handler: Handler) -> Result<(), String> {
    let hwnd = HWND(hwnd as *mut _);
    // SAFETY: OLE calls on our own window from the UI thread.
    unsafe {
        let _ = OleInitialize(None); // winit initialized OLE already; this just balances
        let _ = RevokeDragDrop(hwnd); // winit's own handler (we take over)
        let target: IDropTarget = Target::new(handler).into();
        RegisterDragDrop(hwnd, &target).map_err(|e| e.message())
    }
}

pub fn start_drag(hwnd: isize, items: &[PathBuf]) -> Result<DropEffect, String> {
    let Some(dir) = items.first().and_then(|p| p.parent()) else {
        return Ok(DropEffect::None);
    };
    let hwnd = HWND(hwnd as *mut _);
    let data: IDataObject = crate::win_menu::ui_object(hwnd, dir, items)?;
    // SAFETY: modal OLE drag on the UI thread; the shell supplies the drop source + drag image.
    unsafe {
        let _ = OleInitialize(None);
        SHDoDragDrop(
            Some(hwnd),
            &data,
            None,
            DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK,
        )
        .map(from_ole)
        .map_err(|e| e.message())
    }
}

pub fn screen_to_client(hwnd: isize, x: i32, y: i32) -> (i32, i32) {
    let mut p = POINT { x, y };
    // SAFETY: valid window handle (or harmless failure).
    unsafe {
        let _ = windows::Win32::Graphics::Gdi::ScreenToClient(HWND(hwnd as *mut _), &mut p);
    }
    (p.x, p.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    /// The full data path of a drop: shell data object → our IDropTarget →
    /// handler sees the paths, modifier keys and allowed effects.
    #[test]
    fn drop_target_receives_files() {
        unsafe {
            let _ = OleInitialize(None);
        }
        let d = std::env::temp_dir().join(format!("e2-dnd-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let (a, b) = (d.join("a ä.txt"), d.join("b.txt"));
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        let data: IDataObject =
            crate::win_menu::ui_object(HWND::default(), &d, &[a.clone(), b.clone()]).unwrap();
        assert_eq!(hdrop_paths(&data), vec![a.clone(), b.clone()]);

        type Log = Rc<RefCell<Vec<(DropPhase, usize, bool, bool)>>>;
        let seen: Log = Rc::default();
        let log = seen.clone();
        let target: IDropTarget = Target::new(Box::new(move |ev| {
            log.borrow_mut()
                .push((ev.phase, ev.paths.len(), ev.keys.ctrl, ev.allow_move));
            if ev.keys.ctrl {
                DropEffect::Copy
            } else {
                DropEffect::Move
            }
        }))
        .into();
        let pt = POINTL { x: 10, y: 10 };
        let mut eff = DROPEFFECT_COPY | DROPEFFECT_MOVE;
        unsafe {
            target
                .DragEnter(&data, MODIFIERKEYS_FLAGS(0), pt, &mut eff)
                .unwrap();
            assert_eq!(eff, DROPEFFECT_MOVE);
            let mut eff2 = DROPEFFECT_COPY | DROPEFFECT_MOVE;
            target.DragOver(MK_CONTROL, pt, &mut eff2).unwrap();
            assert_eq!(eff2, DROPEFFECT_COPY);
            let mut eff3 = DROPEFFECT_COPY | DROPEFFECT_MOVE;
            target.Drop(&data, MK_CONTROL, pt, &mut eff3).unwrap();
            assert_eq!(eff3, DROPEFFECT_COPY);
        }
        let log = seen.borrow();
        assert_eq!(log[0], (DropPhase::Over, 2, false, true));
        assert_eq!(log[1], (DropPhase::Over, 2, true, true));
        assert_eq!(log[2], (DropPhase::Drop, 2, true, true));
        let _ = std::fs::remove_dir_all(&d);
    }
}
