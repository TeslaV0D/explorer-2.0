//! Renderer choice ("Hardwarebeschleunigung"), pure logic so it is testable everywhere.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererOverride {
    Auto,
    Gpu,
    Software,
}

impl RendererOverride {
    /// Parses `EXPLORER2_RENDERER` (auto|gpu|software).
    pub fn parse(v: &str) -> Option<Self> {
        match v.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "gpu" | "skia" | "hardware" => Some(Self::Gpu),
            "software" | "sw" | "cpu" => Some(Self::Software),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    /// A real hardware OpenGL driver (>= 3.0 with shaders).
    Usable {
        renderer: String,
        version: String,
    },
    Unusable {
        reason: String,
    },
    /// No probe on this platform → keep the toolkit's own default.
    NotSupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererChoice {
    Gpu,
    Software,
    PlatformDefault,
}

/// Classifies what an OpenGL probe returned. Software GL implementations are
/// slower than our software renderer, so they count as "unusable".
pub fn classify_gl(renderer: &str, version: &str, has_create_shader: bool) -> ProbeResult {
    let r = renderer.to_ascii_lowercase();
    const SOFTWARE_GL: [&str; 5] = [
        "gdi generic",
        "llvmpipe",
        "softpipe",
        "swrast",
        "microsoft basic render",
    ];
    if let Some(sw) = SOFTWARE_GL.iter().find(|s| r.contains(*s)) {
        return ProbeResult::Unusable {
            reason: format!("software OpenGL ({sw})"),
        };
    }
    if !has_create_shader {
        return ProbeResult::Unusable {
            reason: "OpenGL without shader support".into(),
        };
    }
    let major: u32 = version
        .split(|c: char| !c.is_ascii_digit())
        .find(|p| !p.is_empty())
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);
    if major < 3 {
        return ProbeResult::Unusable {
            reason: format!("OpenGL {version} < 3.0"),
        };
    }
    ProbeResult::Usable {
        renderer: renderer.to_string(),
        version: version.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    Forced,
    DisabledInSettings,
    Gpu {
        renderer: String,
        version: String,
    },
    NoUsableGpu(String),
    NoProbe,
    /// `SLINT_BACKEND` set by the user/developer: toolkit decides.
    ToolkitEnv,
}

/// English, for traces/logs.
impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reason::Forced => f.write_str("forced by EXPLORER2_RENDERER"),
            Reason::DisabledInSettings => f.write_str("hardware acceleration disabled in settings"),
            Reason::Gpu { renderer, version } => write!(f, "GPU: {renderer} (OpenGL {version})"),
            Reason::NoUsableGpu(r) => write!(f, "no usable GPU: {r}"),
            Reason::NoProbe => f.write_str("no GPU probe on this platform"),
            Reason::ToolkitEnv => f.write_str("SLINT_BACKEND set"),
        }
    }
}

/// German status line for the settings panel.
pub fn status_de(choice: RendererChoice, reason: &Reason) -> String {
    let what = match choice {
        RendererChoice::Gpu => "GPU-Beschleunigung",
        RendererChoice::Software => "Software-Rendering",
        RendererChoice::PlatformDefault => "Standard-Renderer des Systems",
    };
    let why = match reason {
        Reason::Forced => "per EXPLORER2_RENDERER erzwungen".to_string(),
        Reason::DisabledInSettings => "in den Einstellungen deaktiviert".to_string(),
        Reason::Gpu { renderer, .. } if renderer != "cached" => renderer.clone(),
        Reason::Gpu { .. } => "geeignete GPU erkannt".to_string(),
        Reason::NoUsableGpu(_) => "keine geeignete GPU gefunden".to_string(),
        Reason::NoProbe => "keine GPU-Prüfung auf diesem System".to_string(),
        Reason::ToolkitEnv => "per SLINT_BACKEND festgelegt".to_string(),
    };
    format!("Aktuell: {what} ({why})")
}

/// Final decision + reason (shown in settings and traces).
pub fn decide(
    env_override: Option<RendererOverride>,
    hardware_acceleration: bool,
    probe: impl FnOnce() -> ProbeResult,
) -> (RendererChoice, Reason) {
    match env_override {
        Some(RendererOverride::Software) => return (RendererChoice::Software, Reason::Forced),
        Some(RendererOverride::Gpu) => return (RendererChoice::Gpu, Reason::Forced),
        Some(RendererOverride::Auto) | None => {}
    }
    if !hardware_acceleration {
        return (RendererChoice::Software, Reason::DisabledInSettings);
    }
    match probe() {
        ProbeResult::Usable { renderer, version } => {
            (RendererChoice::Gpu, Reason::Gpu { renderer, version })
        }
        ProbeResult::Unusable { reason } => (RendererChoice::Software, Reason::NoUsableGpu(reason)),
        ProbeResult::NotSupported => (RendererChoice::PlatformDefault, Reason::NoProbe),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_real_and_software_gl() {
        assert!(matches!(
            classify_gl(
                "NVIDIA GeForce RTX 4070/PCIe/SSE2",
                "4.6.0 NVIDIA 560.94",
                true
            ),
            ProbeResult::Usable { .. }
        ));
        assert!(matches!(
            classify_gl(
                "AMD Radeon(TM) Graphics",
                "4.6.0 Compatibility Profile",
                true
            ),
            ProbeResult::Usable { .. }
        ));
        assert!(matches!(
            classify_gl("Intel(R) UHD Graphics 620", "3.3.0 - Build 31.0", true),
            ProbeResult::Usable { .. }
        ));
        assert!(matches!(
            classify_gl("GDI Generic", "1.1.0", false),
            ProbeResult::Unusable { .. }
        ));
        assert!(matches!(
            classify_gl(
                "llvmpipe (LLVM 17.0.6, 256 bits)",
                "4.5 (Core Profile) Mesa 24.0",
                true
            ),
            ProbeResult::Unusable { .. }
        ));
        assert!(matches!(
            classify_gl("Some GPU", "2.1", true),
            ProbeResult::Unusable { .. }
        ));
        assert!(matches!(
            classify_gl("Some GPU", "4.6", false),
            ProbeResult::Unusable { .. }
        ));
        assert!(matches!(
            classify_gl("Some GPU", "", true),
            ProbeResult::Unusable { .. }
        ));
    }

    #[test]
    fn decision_matrix() {
        let usable = || ProbeResult::Usable {
            renderer: "GPU".into(),
            version: "4.6".into(),
        };
        let unusable = || ProbeResult::Unusable { reason: "x".into() };
        let panics = || -> ProbeResult { panic!("probe must not run") };

        assert_eq!(decide(None, true, usable).0, RendererChoice::Gpu);
        assert_eq!(decide(None, true, unusable).0, RendererChoice::Software);
        assert_eq!(
            decide(None, true, || ProbeResult::NotSupported).0,
            RendererChoice::PlatformDefault
        );
        // setting off / overrides never pay for a probe
        assert_eq!(decide(None, false, panics).0, RendererChoice::Software);
        assert_eq!(
            decide(Some(RendererOverride::Software), true, panics).0,
            RendererChoice::Software
        );
        assert_eq!(
            decide(Some(RendererOverride::Gpu), false, panics).0,
            RendererChoice::Gpu
        );
        assert_eq!(
            decide(Some(RendererOverride::Auto), true, unusable).0,
            RendererChoice::Software
        );
    }

    #[test]
    fn german_status() {
        assert_eq!(
            status_de(RendererChoice::Software, &Reason::DisabledInSettings),
            "Aktuell: Software-Rendering (in den Einstellungen deaktiviert)"
        );
        let gpu = Reason::Gpu {
            renderer: "NVIDIA RTX".into(),
            version: "4.6".into(),
        };
        assert_eq!(
            status_de(RendererChoice::Gpu, &gpu),
            "Aktuell: GPU-Beschleunigung (NVIDIA RTX)"
        );
        let cached = Reason::Gpu {
            renderer: "cached".into(),
            version: "cached".into(),
        };
        assert!(status_de(RendererChoice::Gpu, &cached).contains("geeignete GPU erkannt"));
        assert_eq!(
            Reason::NoUsableGpu("x".into()).to_string(),
            "no usable GPU: x"
        );
    }

    #[test]
    fn override_parsing() {
        assert_eq!(
            RendererOverride::parse(" GPU "),
            Some(RendererOverride::Gpu)
        );
        assert_eq!(
            RendererOverride::parse("software"),
            Some(RendererOverride::Software)
        );
        assert_eq!(
            RendererOverride::parse("auto"),
            Some(RendererOverride::Auto)
        );
        assert_eq!(RendererOverride::parse("nope"), None);
    }
}
