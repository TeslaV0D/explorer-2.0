//! `.lnk` shortcuts via the shell (`IShellLinkW` + `IPersistFile`), returned
//! as bytes so the caller can write them through the integration journal.

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use windows::core::{Interface, HSTRING};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

fn err(e: windows::core::Error) -> io::Error {
    io::Error::other(e.message())
}

pub fn make(target: &Path, args: &str, description: &str) -> io::Result<Vec<u8>> {
    static N: AtomicU32 = AtomicU32::new(0);
    let tmp = std::env::temp_dir().join(format!(
        "explorer2-{}-{}.lnk",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    // SAFETY: COM calls on this thread with owned HSTRING arguments.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW =
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(err)?;
        link.SetPath(&HSTRING::from(target.as_os_str()))
            .map_err(err)?;
        link.SetArguments(&HSTRING::from(args)).map_err(err)?;
        link.SetDescription(&HSTRING::from(description))
            .map_err(err)?;
        link.SetIconLocation(&HSTRING::from(target.as_os_str()), 0)
            .map_err(err)?;
        if let Some(dir) = target.parent() {
            link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))
                .map_err(err)?;
        }
        let file: IPersistFile = link.cast().map_err(err)?;
        file.Save(&HSTRING::from(tmp.as_os_str()), true)
            .map_err(err)?;
    }
    let bytes = std::fs::read(&tmp);
    let _ = std::fs::remove_file(&tmp);
    bytes
}
