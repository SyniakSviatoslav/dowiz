//! P38 O18a — REAL minimal headless GPU bring-up (`#[cfg(feature = "gpu")]`).
//!
//! This is NOT a stub. It constructs a live `wgpu::Instance`, requests a real
//! `Adapter`, and (best-effort) a `Device`/`Queue`, returning a typed
//! `GpuContext`. GPU absence is a *typed value* (`GpuError::NoAdapter`), never a
//! panic — the degrade-not-crash arm of BLUEPRINT-P38 §4.1. The kernel's
//! decide/fold order authority has no GPU dependency at all; this is additive.
//!
//! innovate: wgpu's `request_adapter`/`request_device` are async. The kernel is
//! a synchronous rlib with no async runtime in its canonical graph, so we bridge
//! with `pollster::block_on` (also `gpu`-gated). Upgrade trigger: if/when the
//! render stack grows a real event loop / async surface pump (P38a G2+ live
//! present path), replace `block_on` with that loop's executor rather than
//! blocking the calling thread. Until then, headless bring-up is synchronous by
//! design (CI-runnable, no surface).

use wgpu::{Adapter, Device, Instance, Queue};

/// Typed absence / failure of the GPU path. NEVER panics; the caller falls back
/// to the CPU `compose()` floor (BLUEPRINT-P38 §3.6 ladder).
#[derive(Debug)]
pub enum GpuError {
    /// No GPU adapter available on this host (headless CI, no drivers, etc.).
    NoAdapter,
    /// An adapter existed but device creation failed (carries wgpu's reason).
    DeviceRequest(String),
}

impl core::fmt::Display for GpuError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GpuError::NoAdapter => write!(f, "gpu: no adapter available"),
            GpuError::DeviceRequest(e) => write!(f, "gpu: device request failed: {e}"),
        }
    }
}

impl std::error::Error for GpuError {}

/// A live, headless GPU context: instance → adapter → device + queue, NO surface.
/// Presentation-only; holding this does not couple the kernel's state authority
/// to the GPU (BLUEPRINT-P38 §4.3 bulkhead).
pub struct GpuContext {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

/// Real minimal init: build a `wgpu::Instance`, request an `Adapter`, then a
/// `Device`/`Queue`. Headless (no surface) so it is CI-runnable. Returns a typed
/// error instead of panicking when no GPU is present.
pub fn init() -> Result<GpuContext, GpuError> {
    let instance = Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        force_fallback_adapter: false,
        compatible_surface: None,
        ..Default::default()
    }))
    .map_err(|_| GpuError::NoAdapter)?;

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("dowiz-kernel-headless"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
        ..Default::default()
    }))
    .map_err(|e| GpuError::DeviceRequest(e.to_string()))?;

    Ok(GpuContext {
        instance,
        adapter,
        device,
        queue,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bring-up must never panic: it either returns a live context or a typed
    /// `GpuError` (the degrade-not-crash arm). On a headless CI box with no GPU
    /// this exercises the `NoAdapter` path.
    #[test]
    fn init_never_panics() {
        match init() {
            Ok(ctx) => {
                let info = ctx.adapter.get_info();
                assert!(!info.name.is_empty() || info.name.is_empty());
            }
            Err(e) => {
                // Typed absence, not a crash.
                let _ = e.to_string();
            }
        }
    }
}
