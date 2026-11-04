//! Thumbnails for the icon view: the shell's own image for a file at a given
//! size – a real preview for pictures, videos, PDFs … (from the Windows
//! thumbnail cache) and the large icon for everything else.

use crate::icons::Icon;
use std::path::Path;

/// An image for the icon view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumb {
    pub image: Icon,
    /// A real preview of the content (not just the file type's icon).
    pub preview: bool,
}

/// Image of `path` that fits into `px`×`px` (blocking; worker thread).
/// `None` without shell support or if the shell has nothing.
pub fn thumbnail(path: &Path, px: u32) -> Option<Thumb> {
    #[cfg(windows)]
    {
        crate::win::thumbnail(path, px)
    }
    #[cfg(not(windows))]
    {
        let _ = (path, px);
        None
    }
}

/// Premultiplied BGRA (what the shell's image factory returns) → straight
/// RGBA; bitmaps without any alpha are opaque (JPEG thumbnails) (pure → tested).
pub fn premultiplied_bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    let has_alpha = bgra.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
    let mut out = Vec::with_capacity(bgra.len());
    for p in bgra.as_chunks::<4>().0 {
        let a = if has_alpha { p[3] } else { 255 };
        let un = |c: u8| -> u8 {
            if a == 0 || a == 255 {
                c
            } else {
                ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8
            }
        };
        out.extend_from_slice(&[un(p[2]), un(p[1]), un(p[0]), a]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premultiplied_conversion() {
        // opaque red, half-transparent premultiplied white, fully transparent
        let px = [0, 0, 255, 255, 128, 128, 128, 128, 0, 0, 0, 0];
        assert_eq!(
            premultiplied_bgra_to_rgba(&px),
            vec![255, 0, 0, 255, 255, 255, 255, 128, 0, 0, 0, 0]
        );
        // no alpha at all → opaque
        assert_eq!(
            premultiplied_bgra_to_rgba(&[10, 20, 30, 0]),
            vec![30, 20, 10, 255]
        );
    }

    #[test]
    fn no_thumbnails_without_shell() {
        if !cfg!(windows) {
            assert_eq!(thumbnail(Path::new("/x.png"), 96), None);
        }
    }
}
