//! Renderer selection ("Hardwarebeschleunigung") before the window is created.

use crate::config;
use crate::nav::{since_start_ms, trace_line};
use explorer_core::renderer::{
    decide, status_de, ProbeResult, Reason, RendererChoice, RendererOverride,
};
use explorer_core::settings::GpuProbeCache;
use std::time::{Duration, Instant};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Selection {
    pub choice: RendererChoice,
    pub reason: Reason,
    /// Probe result came from the cache → refresh it in the background.
    pub used_cache: bool,
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
            used_cache: false,
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
    let mut used_cache = false;
    let mut probe_ms = 0.0;
    let (choice, reason) = decide(
        env_override,
        settings.hardware_acceleration,
        || match settings.cached_probe(VERSION) {
            Some(GpuProbeCache::Usable) => {
                used_cache = true;
                ProbeResult::Usable {
                    renderer: "cached".into(),
                    version: "cached".into(),
                }
            }
            Some(GpuProbeCache::Unusable) => {
                used_cache = true;
                ProbeResult::Unusable {
                    reason: "cached probe".into(),
                }
            }
            None => {
                let p0 = Instant::now();
                let r = explorer_gpu::probe_opengl();
                probe_ms = p0.elapsed().as_secs_f64() * 1000.0;
                store_probe(&r);
                r
            }
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
        "renderer choice={choice:?} reason=\"{reason}\" cached={used_cache} probe_ms={probe_ms:.1} select_ms={:.1} at={:.1}ms",
        t0.elapsed().as_secs_f64() * 1000.0,
        since_start_ms()
    ));
    Selection {
        choice,
        reason,
        used_cache,
    }
}

fn store_probe(r: &ProbeResult) {
    let cache = match r {
        ProbeResult::Usable { .. } => GpuProbeCache::Usable,
        ProbeResult::Unusable { .. } => GpuProbeCache::Unusable,
        ProbeResult::NotSupported => return,
    };
    if let Err(e) = config::update(|s| {
        s.gpu_probe = Some(cache);
        s.gpu_probe_version = VERSION.into();
    }) {
        trace_line(&format!("probe cache not saved: {e}"));
    }
}

/// Keeps the cached probe fresh (driver installed/removed) without paying for
/// it on the startup path: re-probes a few seconds after start, applies next start.
pub fn refresh_probe_in_background(sel: &Selection) {
    if !sel.used_cache {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("gpu-reprobe".into())
        .spawn(|| {
            std::thread::sleep(Duration::from_secs(5));
            let before = config::load().gpu_probe;
            let r = explorer_gpu::probe_opengl();
            store_probe(&r);
            let after = config::load().gpu_probe;
            trace_line(&format!(
                "background re-probe: {r:?} changed={}",
                before != after
            ));
        });
}
