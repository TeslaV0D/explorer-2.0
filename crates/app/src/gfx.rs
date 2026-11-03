//! Renderer selection ("Hardwarebeschleunigung") before the window is created.
//!
//! The GPU probe never runs on the startup path: without a cached result the app
//! starts with the software renderer right away and probes in the background; the
//! result applies from the next start. With a cached result, a delayed background
//! re-probe keeps the cache fresh (driver installed/removed).

use crate::config;
use crate::nav::{since_start_ms, trace_line};
use explorer_core::renderer::{
    decide, status_de, ProbeResult, Reason, RendererChoice, RendererOverride,
};
use explorer_core::settings::GpuProbeCache;
use std::time::{Duration, Instant};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// When to run the background probe after start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundProbe {
    None,
    /// No cache yet: probe immediately (result for the next start).
    Now,
    /// Cache used: re-probe later to keep it fresh.
    Later,
}

pub struct Selection {
    pub choice: RendererChoice,
    pub reason: Reason,
    pub background: BackgroundProbe,
}

/// Human-readable status for the settings panel.
pub fn status_text(sel: &Selection) -> String {
    status_de(sel.choice, &sel.reason)
}

pub fn select() -> Selection {
    let t0 = Instant::now();
    if std::env::var_os("SLINT_BACKEND").is_some() {
        let sel = Selection {
            choice: RendererChoice::PlatformDefault,
            reason: Reason::ToolkitEnv,
            background: BackgroundProbe::None,
        };
        trace_line(&format!(
            "renderer choice={:?} reason=\"{}\" at={:.1}ms",
            sel.choice,
            sel.reason,
            since_start_ms()
        ));
        return sel;
    }
    let settings = config::load();
    let env_override = std::env::var("EXPLORER2_RENDERER")
        .ok()
        .and_then(|v| RendererOverride::parse(&v));
    let mut background = BackgroundProbe::None;
    let (choice, reason) = decide(
        env_override,
        settings.hardware_acceleration,
        || match settings.cached_probe(VERSION) {
            Some(GpuProbeCache::Usable) => {
                background = BackgroundProbe::Later;
                ProbeResult::Usable {
                    renderer: "cached".into(),
                    version: "cached".into(),
                }
            }
            Some(GpuProbeCache::Unusable) => {
                background = BackgroundProbe::Later;
                ProbeResult::Unusable {
                    reason: "cached probe".into(),
                }
            }
            None if explorer_gpu::PROBE_SUPPORTED => {
                background = BackgroundProbe::Now;
                ProbeResult::Pending
            }
            None => ProbeResult::NotSupported,
        },
    );
    let renderer = match choice {
        RendererChoice::Gpu => Some("skia"), // "skia" still falls back by itself if the probe was wrong
        RendererChoice::Software => Some("software"),
        RendererChoice::PlatformDefault => None,
    };
    if let Some(name) = renderer {
        let res = slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name(name.into())
            .select();
        if let Err(e) = res {
            trace_line(&format!("renderer {name} failed: {e} -> trying software"));
            let _ = slint::BackendSelector::new()
                .backend_name("winit".into())
                .renderer_name("software".into())
                .select();
        }
    }
    trace_line(&format!(
        "renderer choice={choice:?} reason=\"{reason}\" background={background:?} select_ms={:.1} at={:.1}ms",
        t0.elapsed().as_secs_f64() * 1000.0,
        since_start_ms()
    ));
    Selection {
        choice,
        reason,
        background,
    }
}

fn store_probe(r: &ProbeResult) {
    let cache = match r {
        ProbeResult::Usable { .. } => GpuProbeCache::Usable,
        ProbeResult::Unusable { .. } => GpuProbeCache::Unusable,
        ProbeResult::NotSupported | ProbeResult::Pending => return,
    };
    if let Err(e) = config::update(|s| {
        s.gpu_probe = Some(cache);
        s.gpu_probe_version = VERSION.into();
    }) {
        trace_line(&format!("probe cache not saved: {e}"));
    }
}

/// Runs the GPU probe off the UI thread (see module docs).
pub fn probe_in_background(sel: &Selection) {
    let delay = match sel.background {
        BackgroundProbe::None => return,
        BackgroundProbe::Now => Duration::ZERO,
        BackgroundProbe::Later => Duration::from_secs(5),
    };
    let _ = std::thread::Builder::new()
        .name("gpu-probe".into())
        .spawn(move || {
            std::thread::sleep(delay);
            let before = config::load().gpu_probe;
            let p0 = Instant::now();
            let r = explorer_gpu::probe_opengl();
            let probe_ms = p0.elapsed().as_secs_f64() * 1000.0;
            store_probe(&r);
            let after = config::load().gpu_probe;
            trace_line(&format!(
                "background probe: {r:?} probe_ms={probe_ms:.1} changed={} applies_next_start=true",
                before != after
            ));
        });
}
