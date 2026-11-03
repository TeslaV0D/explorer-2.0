//! GPU capability probe, used to pick the renderer before the window exists.
//!
//! Windows: creates a hidden 1x1 window + legacy WGL context (the same
//! `opengl32.dll` path Skia's GL backend takes) and inspects
//! `GL_RENDERER`/`GL_VERSION`/`glCreateShader`. Takes a few ms; without a
//! driver it fails fast. Other platforms: not supported (toolkit default).

use explorer_core::renderer::ProbeResult;

/// Probes for a usable hardware OpenGL driver. Never panics.
pub fn probe_opengl() -> ProbeResult {
    #[cfg(windows)]
    {
        win::probe()
    }
    #[cfg(not(windows))]
    {
        ProbeResult::NotSupported
    }
}

#[cfg(windows)]
mod win {
    use explorer_core::renderer::{classify_gl, ProbeResult};
    use std::ffi::CStr;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC, HDC};
    use windows_sys::Win32::Graphics::OpenGL::{
        glGetString, wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent,
        ChoosePixelFormat, SetPixelFormat, GL_RENDERER, GL_VERSION, HGLRC, PFD_DOUBLEBUFFER,
        PFD_DRAW_TO_WINDOW, PFD_MAIN_PLANE, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA,
        PIXELFORMATDESCRIPTOR,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, UnregisterClassW,
        WNDCLASSW, WS_OVERLAPPED,
    };

    unsafe extern "system" fn wndproc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: forwarding the unchanged arguments to the default handler.
        unsafe { DefWindowProcW(h, m, w, l) }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// RAII cleanup in reverse order of creation, whatever step failed.
    struct Probe {
        class: Vec<u16>,
        hinst: *mut core::ffi::c_void,
        registered: bool,
        hwnd: HWND,
        hdc: HDC,
        ctx: HGLRC,
    }

    impl Drop for Probe {
        fn drop(&mut self) {
            // SAFETY: each handle is released only if it was successfully created.
            unsafe {
                if !self.ctx.is_null() {
                    wglMakeCurrent(std::ptr::null_mut(), std::ptr::null_mut());
                    wglDeleteContext(self.ctx);
                }
                if !self.hdc.is_null() {
                    ReleaseDC(self.hwnd, self.hdc);
                }
                if !self.hwnd.is_null() {
                    DestroyWindow(self.hwnd);
                }
                if self.registered {
                    UnregisterClassW(self.class.as_ptr(), self.hinst);
                }
            }
        }
    }

    fn gl_string(name: u32) -> String {
        // SAFETY: a context is current; glGetString returns a static NUL-terminated string or null.
        let p = unsafe { glGetString(name) };
        if p.is_null() {
            return String::new();
        }
        // SAFETY: non-null, NUL-terminated per GL spec.
        unsafe { CStr::from_ptr(p.cast()) }
            .to_string_lossy()
            .into_owned()
    }

    pub fn probe() -> ProbeResult {
        let unusable = |r: &str| ProbeResult::Unusable {
            reason: r.to_string(),
        };
        // SAFETY: plain Win32 calls; all handles are owned by `Probe` and cleaned up in Drop.
        unsafe {
            let mut p = Probe {
                class: wide(&format!("explorer2_gl_probe_{}", std::process::id())),
                hinst: GetModuleHandleW(std::ptr::null()),
                registered: false,
                hwnd: std::ptr::null_mut(),
                hdc: std::ptr::null_mut(),
                ctx: std::ptr::null_mut(),
            };
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: p.hinst,
                lpszClassName: p.class.as_ptr(),
                ..Default::default()
            };
            if RegisterClassW(&wc) == 0 {
                return unusable("RegisterClassW failed");
            }
            p.registered = true;
            let title = wide("gl probe");
            // Not WS_VISIBLE: never shown.
            p.hwnd = CreateWindowExW(
                0,
                p.class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPED,
                0,
                0,
                1,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                p.hinst,
                std::ptr::null(),
            );
            if p.hwnd.is_null() {
                return unusable("CreateWindowExW failed");
            }
            p.hdc = GetDC(p.hwnd);
            if p.hdc.is_null() {
                return unusable("GetDC failed");
            }
            let pfd = PIXELFORMATDESCRIPTOR {
                nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
                nVersion: 1,
                dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
                iPixelType: PFD_TYPE_RGBA,
                cColorBits: 32,
                cDepthBits: 24,
                cStencilBits: 8,
                iLayerType: PFD_MAIN_PLANE as u8,
                ..std::mem::zeroed()
            };
            let fmt = ChoosePixelFormat(p.hdc, &pfd);
            if fmt == 0 || SetPixelFormat(p.hdc, fmt, &pfd) == 0 {
                return unusable("no OpenGL pixel format");
            }
            p.ctx = wglCreateContext(p.hdc);
            if p.ctx.is_null() {
                return unusable("wglCreateContext failed");
            }
            if wglMakeCurrent(p.hdc, p.ctx) == 0 {
                return unusable("wglMakeCurrent failed");
            }
            let renderer = gl_string(GL_RENDERER);
            let version = gl_string(GL_VERSION);
            let has_create_shader = wglGetProcAddress(c"glCreateShader".as_ptr().cast()).is_some();
            classify_gl(&renderer, &version, has_create_shader)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_never_panics_and_is_fast() {
        let t = std::time::Instant::now();
        let r = probe_opengl();
        let took = t.elapsed();
        // Printed so CI logs show what each runner reports.
        println!("gpu probe: {r:?} in {took:?}");
        #[cfg(not(windows))]
        assert_eq!(r, ProbeResult::NotSupported);
        assert!(took < std::time::Duration::from_secs(5));
    }
}
