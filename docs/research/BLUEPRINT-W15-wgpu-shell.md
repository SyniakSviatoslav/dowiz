# BLUEPRINT W15 — wgpu GPU adapter shell (dowiz-engine)

Status: FEATURE-GATED SHELL. Offline: feature OFF, default build wgpu-free (26/0).
Online trigger documented. Decart: see NEXT-PHASES-research-decisions.md.

## New file: `dowiz/engine/src/gpu_field.rs`
A `GpuField` trait mirroring the CPU field API so the W10 demo could switch
backends without touching call sites:
```
#[cfg(feature = "wgpu")]
pub trait GpuField {
    fn new(circles: &[f64], w: u32, h: u32) -> Self;
    fn step(&mut self);
    fn frame(&self) -> Vec<u8>; // RGBA8, identical layout to CPU FieldSim
}
```
Under `#[cfg(feature = "wgpu")]`: a `WgpuField` impl using `wgpu` +
`pollster` for surface/adapter + compute pass that rasterizes the same
M∇²U+ΓU̇+c²LU=S field on the GPU. The OUTPUT contract is byte-identical to
`FieldSim::frame()` so `index.html` (W10) is backend-agnostic.

## Cargo.toml (engine)
```
[features]
default = []
wgpu = ["dep:wgpu", "dep:pollster", "dep:winit"]   # OFF by default
[dependencies]
wgpu = { version = "0.19", optional = true }
pollster = { version = "0.3", optional = true }
winit = { version = "0.29", optional = true }
```

## build.rs (set cfg only when dep present)
```rust
// Sets `cfg(wgpu_available)` iff the `wgpu` feature resolved (dep reachable).
fn main() {
    println!("cargo:rustc-check-cfg=cfg(wgpu_available)");
    if std::env::var("CARGO_FEATURE_WGPU").is_ok() {
        println!("cargo:rustc-cfg=wgpu_available");
    }
}
```
Default build: `wgpu_available` unset → `GpuField` module compiles to a
`compile_error!`/stub only if the feature is forced without the dep (guards the
offline trap).

## Tests (RED→GREEN)
- DEFAULT `cargo test -p dowiz-engine` → 26/0 (wgpu OFF, no GPU code compiled).
- ONLINE CI (`--features wgpu`, wgpu reachable): a `gpu_raster_smoke` that
  creates a headless `wgpu` adapter, runs a few steps, asserts `frame().len() ==
  w*h*4` and no NaN bytes. Documented trigger, NOT run offline.

## Honest ceiling
`wgpu` crate is NOT in the offline cache → the `wgpu` feature cannot compile
here today. Shipped: the trait boundary + default-off gating + online CI trigger
so the moment the dep is reachable the GPU backend drops in. This is a documented
ceiling, not a gap. The CPU field-frame (W5) + static demo (W10) already deliver
the browser-observable physics render that wgpu would only accelerate.
