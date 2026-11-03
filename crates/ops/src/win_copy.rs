//! Windows file copy via CopyFileExW: keeps attributes, timestamps and
//! alternate data streams like Explorer, with progress + cancellation.

use std::ffi::c_void;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::{
    CopyFileExW, COPY_FILE_FAIL_IF_EXISTS, PROGRESS_CANCEL, PROGRESS_CONTINUE,
};

struct State<'a> {
    on_progress: &'a mut dyn FnMut(u64) -> bool,
    last: u64,
}

unsafe extern "system" fn progress(
    _total: i64,
    transferred: i64,
    _stream_size: i64,
    _stream_transferred: i64,
    _stream: u32,
    _reason: u32,
    _src: HANDLE,
    _dst: HANDLE,
    data: *const c_void,
) -> u32 {
    // SAFETY: `data` is the &mut State passed to CopyFileExW below, alive for the whole call.
    let st = unsafe { &mut *(data as *mut State) };
    let now = transferred.max(0) as u64;
    let delta = now.saturating_sub(st.last);
    st.last = now;
    if (st.on_progress)(delta) {
        PROGRESS_CONTINUE
    } else {
        PROGRESS_CANCEL
    }
}

/// Copies `src` to `dst` (must not exist). `on_progress(bytes_since_last)` returns
/// false to cancel; a cancelled/failed copy leaves no `dst` behind (CopyFileExW deletes it).
pub fn copy_file(
    src: &Path,
    dst: &Path,
    on_progress: &mut dyn FnMut(u64) -> bool,
) -> io::Result<()> {
    let w = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>()
    };
    let (s, d) = (w(src), w(dst));
    let mut st = State {
        on_progress,
        last: 0,
    };
    // SAFETY: NUL-terminated paths; callback data points to `st`, which outlives the call.
    let ok = unsafe {
        CopyFileExW(
            s.as_ptr(),
            d.as_ptr(),
            Some(progress),
            &mut st as *mut State as *const c_void,
            std::ptr::null_mut(),
            COPY_FILE_FAIL_IF_EXISTS,
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
