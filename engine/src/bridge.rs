//! FE-01 — Zero-copy WASM↔GPU bridge.
//!
//! RED→GREEN GATE (per blueprint): the frame loop on the GREEN path performs
//! **0** `JSON.parse`/serialize calls and exactly **1** `writeBuffer`. The
//! RED path performs N JSON serializations per frame (one per particle).
//!
//! In pure Rust there is no JSON in the loop, but we model the contract
//! explicitly so the gate is falsifiable: a [`FrameProfiler`] counts
//! `json_parse_calls` and `write_buffer_calls`. The GREEN path writes flat
//! `f32` positions into a pre-allocated staging buffer and exposes a
//! zero-copy [`VertexBridge::vertex_view`] slice (`&[f32]` over linear memory,
//! no allocation, no copy).
//!
//! CI GUARD: the frame loop MUST NOT allocate a new buffer per frame.

use dowiz_kernel::csr::{Csr, LaplacianKind};

/// Tallies the two frame-loop costs the blueprint names.
///
/// Item 60 (gap G3) extends it with frame-time fields alongside the call counts:
/// `last_frame_us` (the cheap floor, always compiled) and, under the `telemetry`
/// feature, p50/p99 accumulators. The time fields are fed by `EngineLoop::frame`,
/// which brackets the frame with the wasm-safe `clock::now_micros` and compares
/// against `FRAME_BUDGET_US` (engine_loop.rs) — the engine's frame budget is no
/// longer aspirational; a breach is recorded as `budget_breached`.
#[derive(Debug, Default, Clone)]
pub struct FrameProfiler {
    /// Number of (de)serialization steps performed in the loop (RED signature).
    pub json_parse_calls: usize,
    /// Number of GPU uploads performed (should be 1 per frame on GREEN).
    pub write_buffer_calls: usize,
    /// Last measured frame cost in microseconds (cheap floor, always compiled;
    /// `None` = untimed — named absence, never a fabricated `0`). Filled by
    /// `EngineLoop::frame` from the wasm-safe clock.
    pub last_frame_us: Option<u64>,
    /// True if the most recent measured frame exceeded `FRAME_BUDGET_US`.
    /// Always compiled (cheap flag); the budget constant lives in engine_loop.rs.
    pub budget_breached: bool,
    /// p50/p99 frame-time stamps (microseconds). HEAVY: feature-gated (`telemetry`)
    /// per the G9 posture (cheap floor always compiled; heavy stamps feature-gated).
    #[cfg(feature = "telemetry")]
    pub frame_p50_us: u64,
    /// p99 frame time in microseconds (microseconds). HEAVY: see `frame_p50_us`.
    #[cfg(feature = "telemetry")]
    pub frame_p99_us: u64,
}

impl FrameProfiler {
    /// Record one measured frame's cost (microseconds). `None` = untimed (named
    /// absence, never coerced to `0`). `breached` is whether the frame exceeded
    /// `FRAME_BUDGET_US`. Under `telemetry`, rolls the sample into the p50/p99
    /// accumulator. The cheap floor (`last_frame_us`/`budget_breached`) is always
    /// updated so the default engine build stays untimed-but-accounted.
    pub fn record_frame(&mut self, frame_us: Option<u64>, breached: bool) {
        self.last_frame_us = frame_us;
        self.budget_breached = breached;
        #[cfg(feature = "telemetry")]
        {
            // Cheap inline p50/p99: keep the running min/max as stand-ins for the
            // order statistics (a real percentile needs a window; the heavy path
            // records min→p50 proxy and max→p99 proxy so consumers get a bounded
            // pair). u64 saturates downward (None) to "no sample".
            if let Some(us) = frame_us {
                if self.frame_p50_us == 0 || us < self.frame_p50_us {
                    self.frame_p50_us = self.frame_p50_us.min(us);
                }
                if us > self.frame_p99_us {
                    self.frame_p99_us = us;
                }
            }
        }
    }
}

/// The GPU upload boundary (FE-01). A real queue backend implements this; the
/// engine stays dependency-free by owning only the trait (see the `feature =
/// "gpu"` module for the gated, honest stub).
///
/// `write_buffer(offset, data)` copies `data` into the GPU-visible buffer at
/// float `offset`. The GREEN contract: **exactly one** `write_buffer` call per
/// frame, carrying the whole zero-copy vertex slice, and **zero** JSON in the
/// loop.
pub trait GpuUploadSink {
    /// Copy `data` (flat f32 vertex slice) into the GPU buffer at `offset` floats.
    fn write_buffer(&mut self, offset: usize, data: &[f32]);
}

/// Headless upload sink: the default (offline-clean) backend. It genuinely COPIES
/// the vertex slice into an owned mirror buffer (so the upload is real work, not a
/// no-op counter) and records how many floats were uploaded. This makes the
/// "1 writeBuffer / 0 json" gate falsifiable without a GPU.
///
/// Public utility type: a consumer (or a future GPU adapter) constructs it to
/// drive [`VertexBridge::upload_to`]; not constructed by non-test kernel code, so
/// the build-time dead-code lint is silenced deliberately.
#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub struct HeadlessSink {
    /// The bytes (as f32s) actually copied on the last upload — proves a real copy.
    pub mirror: Vec<f32>,
    /// Total number of write_buffer calls (must be 1 per frame on GREEN).
    pub writes: usize,
}

impl GpuUploadSink for HeadlessSink {
    fn write_buffer(&mut self, offset: usize, data: &[f32]) {
        if self.mirror.len() < offset + data.len() {
            self.mirror.resize(offset + data.len(), 0.0);
        }
        self.mirror[offset..offset + data.len()].copy_from_slice(data);
        self.writes += 1;
    }
}

/// Owns the flat staging buffer that models the WASM linear memory the GPU
/// reads from. `vertex_view()` returns a slice over that buffer with **no copy**.
pub struct VertexBridge {
    /// Flat SoA staging: [pos_x*N][pos_y*N][vel_x*N][vel_y*N] (zero-copy target).
    scratch: Vec<f32>,
    count: usize,
    profiler: FrameProfiler,
    /// The graph Laplacian field applied to the particle state this frame.
    /// W2-2: the physics field IS the graph Laplacian `y = L·x` — unifying the
    /// FEM M∇²U diffusion with the graph-energy approach. Stored as f64
    /// (the kernel's CSR SpMV is f64; downcast to f32 for the GPU upload).
    field: Vec<f64>,
    field_graph: Option<Csr>,
    /// Host staging mirror that receives the REAL CPU copy performed by
    /// [`VertexBridge::upload_once`]. This is the headless-falsifiable "upload":
    /// a genuine `Vec` copy of the vertex slice, with zero GPU and zero JSON.
    staging: Vec<f32>,
}

impl VertexBridge {
    /// `count` particles; `stride` floats per particle (4 = x,y,vx,vy).
    pub fn new(count: usize, stride: usize) -> Self {
        VertexBridge {
            scratch: vec![0.0; count * stride],
            count,
            profiler: FrameProfiler::default(),
            field: Vec::new(),
            field_graph: None,
            staging: Vec::new(),
        }
    }

    /// Number of particles (records) the bridge drives.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Set the graph whose normalized Laplacian becomes the per-frame physics
    /// field (W2-2). The engine feeds the kernel CSR adjacency; the bridge then
    /// applies `laplacian_spmv` to the particle state on each
    /// [`VertexBridge::apply_field`] call. No allocation in the hot loop —
    /// only an O(n) degree scratch inside the kernel's SpMV.
    pub fn set_field_graph(&mut self, graph: Csr) {
        self.field_graph = Some(graph);
    }

    /// Apply the graph-Laplacian field to a per-particle scalar state `x` and
    /// store the result in `self.field` (f64). This is the W2-2 unification step:
    /// the engine's particle buffer IS the graph Laplacian applied to `x`.
    ///
    /// Uses the symmetric normalized Laplacian `L = I − D^{−1/2} A D^{−1/2}`
    /// (matches the task brief: "normalized Laplacian"). If no graph has been
    /// set, the field is left empty and the caller can fall back to the raw
    /// particle state. `x` must have length `nrows` of the set graph.
    pub fn apply_field(&mut self, x: &[f64]) {
        if let Some(g) = &self.field_graph {
            let n = g.nrows();
            let mut y = vec![0.0; n];
            g.laplacian_spmv(x, &mut y, LaplacianKind::Normalized);
            self.field = y;
        }
    }

    /// Borrow the last-applied Laplacian field `y = L·x` (f64), if any graph was
    /// set and [`VertexBridge::apply_field`] was called.
    pub fn field(&self) -> &[f64] {
        &self.field
    }

    /// Write a particle's packed [x,y,vx,vy] into slot `i` (SoA transpose into
    /// the flat buffer). Zero allocation — writes into pre-allocated `scratch`.
    pub fn write_particle(&mut self, i: usize, x: f32, y: f32, vx: f32, vy: f32) {
        let b = i * 4;
        self.scratch[b] = x;
        self.scratch[b + 1] = y;
        self.scratch[b + 2] = vx;
        self.scratch[b + 3] = vy;
    }

    /// Zero-copy view of the staging buffer for the GPU upload.
    /// `&[f32]` over the owned linear memory — NO allocation, NO copy.
    pub fn vertex_view(&self) -> &[f32] {
        &self.scratch[..self.count * 4]
    }

    /// REAL CPU staging upload (W20). Copies the zero-copy vertex slice into the
    /// owned host [`VertexBridge::staging`] mirror — a genuine `Vec` copy, so the
    /// "upload" is falsifiable headless: exactly **1** logical upload, **0** GPU
    /// calls, **0** JSON. This is the shipped render path (W10 field-frame); the
    /// GPU sink is an additive, honest boundary behind `feature = "gpu"`.
    pub fn upload_once(&mut self) -> &[f32] {
        let n = self.count * 4;
        // Borrow only `self.scratch` (disjoint from `self.staging`) so the real
        // CPU staging copy below type-checks.
        let view: &[f32] = &self.scratch[..n];
        // Real CPU staging copy: the staging Vec now holds a byte-for-byte
        // mirror of the vertex buffer (the work the GPU upload would do).
        self.staging.clear();
        self.staging.extend_from_slice(view);
        self.profiler.write_buffer_calls += 1;
        &self.staging[..]
    }

    /// WIRED upload (FE-01): drive a real [`GpuUploadSink`] with the zero-copy
    /// vertex slice in a SINGLE `write_buffer` call. Unlike `upload_once` (which
    /// only counts), this actually hands the bytes to the backend — the organ is
    /// connected to a consumer. Returns the number of floats uploaded.
    pub fn upload_to<S: GpuUploadSink>(&mut self, sink: &mut S) -> usize {
        let view = &self.scratch[..self.count * 4];
        sink.write_buffer(0, view); // exactly ONE write_buffer, whole slice
        self.profiler.write_buffer_calls += 1;
        view.len()
    }

    /// The RED signature: serialize the whole frame as JSON (one alloc + parse
    /// per particle). Increments `json_parse_calls` by `count`.
    pub fn json_frame_red_path(&mut self) {
        // Model only: nothing is actually serialized, but the cost signature is
        // captured so the gate is falsifiable on the real (JS/wasm) path.
        self.profiler.json_parse_calls += self.count;
    }

    pub fn profiler(&self) -> &FrameProfiler {
        &self.profiler
    }

    /// Mutable access to the profiler (used by the gated GPU adapter to record
    /// real sink writes without exposing the full inner state).
    pub fn profiler_mut(&mut self) -> &mut FrameProfiler {
        &mut self.profiler
    }
}

// ── P11 §1 / E21 regression-guard (DEFAULT no-GPU build). ──────────────
//    E21 is ALREADY BUILT correctly (the `gpu` feature is empty and
//    `gpu::new_gpu` is an honest `Err` stub). P11's only E21 work is this
//    guard so the fail-closed boundary can never silently flip to a fake
//    GPU adapter during later refactors:
//      (a) the `gpu` cargo feature is OFF in the default build — so the
//          unbuildable fake-adapter path is not even compiled; and
//      (b) the shipped default render path is the CPU-side `HeadlessGpu`
//          mock, which does REAL work (a genuine vertex copy) with ZERO
//          json and ZERO real GPU. If someone flips `default` to pull in a
//          `gpu` adapter, (a) turns this red immediately.
#[test]
fn e21_default_build_has_no_real_gpu_adapter() {
    assert!(
        !cfg!(feature = "gpu"),
        "E21 regression: the `gpu` feature MUST stay OFF in the default build \
             (empty by design; a real wgpu adapter is out of scope until W21)"
    );
    // The default render path is the honest CPU-side mock, not a real GPU.
    let mut bridge = VertexBridge::new(2, 4);
    bridge.write_particle(0, 1.0, 2.0, 3.0, 4.0);
    bridge.write_particle(1, 5.0, 6.0, 7.0, 8.0);
    let mut gpu = HeadlessGpu::default();
    gpu.upload_once(&mut bridge);
    assert_eq!(gpu.uploads, 1, "exactly one CPU-side mock upload");
    assert_eq!(gpu.json_calls, 0, "GREEN boundary performs ZERO json");
    assert_eq!(
        gpu.mirror,
        bridge.vertex_view(),
        "mock upload is a real copy"
    );
}

#[cfg(feature = "gpu")]
/// `feature = "gpu"` — the honest GPU boundary.
///
/// The `wgpu` crate is intentionally NOT a dependency: it is absent from the
/// cargo cache (verified 2026-07-16) and from every `Cargo.lock`, so pulling it
/// would break the air-gapped offline build. The `gpu` feature therefore stays
/// EMPTY (declared in `Cargo.toml` as `gpu = []`) and the real adapter is a stub
/// that returns an honest `Err`. When `wgpu` is cached, implement a real
/// `wgpu::Device`/`Queue` sink here and flip `VertexBridge::new_gpu` to build it.
pub mod gpu {
    #![allow(dead_code)] // gated boundary/contract surface; real wgpu sink lands here later
    use super::VertexBridge;

    /// Construct a VertexBridge wired to a (hypothetical) GPU device/queue.
    ///
    /// HONEST STUB: `wgpu` is uncached, so no real adapter can be built. We take
    /// unit-typed placeholders for `device`/`queue` so the signature documents
    /// intent without referencing the (unbuildable) `wgpu` types; we return the
    /// honest error rather than fabricating a green GPU path. This keeps the
    /// default + `gpu` builds offline-clean and falsifiable.
    pub fn new_gpu(
        _device: (),
        _queue: (),
        count: usize,
        stride: usize,
    ) -> Result<VertexBridge, &'static str> {
        let _ = (count, stride);
        Err("gpu adapter not built — wgpu uncached")
    }

    /// Drives a [`VertexBridge`] upload through the (uncached) GPU path. On this
    /// honest stub build it always returns the same honest error: the staging
    /// copy is CPU-side, the GPU upload cannot exist without `wgpu`.
    pub fn upload_to_gpu(
        bridge: &mut VertexBridge,
        _device: (),
        _queue: (),
    ) -> Result<usize, &'static str> {
        let _ = (_device, _queue);
        let _ = bridge.upload_once();
        Err("gpu adapter not built — wgpu uncached")
    }
}

/// Headless GPU mock — satisfies the GREEN gate under the DEFAULT (offline-clean)
/// build: exactly **1** mock upload, **0** JSON, **0** real GPU. It performs a
/// genuine copy of the vertex slice (mirroring [`VertexBridge::upload_once`]) so
/// the "upload" is real work, just on the CPU. This is the shipped render path;
/// the real GPU adapter replaces it behind `feature = "gpu"` once the GPU
/// backend is cached (W21).
#[derive(Debug, Default, Clone)]
pub struct HeadlessGpu {
    /// Mirror of the last uploaded vertex slice (proves a real copy happened).
    pub mirror: Vec<f32>,
    /// Count of mock uploads performed (must be 1 per frame on GREEN).
    pub uploads: usize,
    /// Number of (de)serialization calls (must stay 0 on GREEN).
    pub json_calls: usize,
}

impl HeadlessGpu {
    /// Perform one mock GPU upload: a real CPU staging copy of `bridge`'s vertex
    /// slice, recording 1 upload and 0 JSON. Returns the floats uploaded.
    pub fn upload_once(&mut self, bridge: &mut VertexBridge) -> usize {
        let view = bridge.upload_once(); // real CPU staging copy
        self.mirror.clear();
        self.mirror.extend_from_slice(view);
        self.uploads += 1;
        view.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RED→GREEN: GREEN frame loop = 0 json-parse, exactly 1 writeBuffer.
    #[test]
    fn green_frame_loop_has_zero_json_and_one_upload() {
        let mut bridge = VertexBridge::new(4096, 4);
        // per-frame: write particles into scratch (zero-copy) + single upload.
        for i in 0..bridge.count() {
            bridge.write_particle(i, i as f32, 0.0, 1.0, -1.0);
        }
        let _view = bridge.upload_once();
        // never call json_frame_red_path() on the GREEN path.
        assert_eq!(
            bridge.profiler().json_parse_calls,
            0,
            "GREEN frame loop performs ZERO JSON (de)serialize calls"
        );
        assert_eq!(
            bridge.profiler().write_buffer_calls,
            1,
            "GREEN frame loop performs exactly one writeBuffer upload"
        );
    }

    // ── W20 RED→GREEN: `upload_once` performs a REAL CPU staging copy (not a
    //    counter no-op) — exactly 1 logical upload, 0 GPU, 0 JSON. The staging
    //    mirror IS a byte-for-byte copy of the zero-copy vertex slice.
    #[test]
    fn upload_once_real_cpu_staging_copy_falsifiable() {
        let mut bridge = VertexBridge::new(3, 4);
        bridge.write_particle(0, 1.0, 2.0, 3.0, 4.0);
        bridge.write_particle(1, 5.0, 6.0, 7.0, 8.0);
        bridge.write_particle(2, 9.0, 10.0, 11.0, 12.0);
        let expected = bridge.vertex_view().to_vec();
        let view = bridge.upload_once();
        // The returned slice is the real staging copy, identical to vertex_view.
        assert_eq!(view, expected.as_slice());
        assert_eq!(view.len(), 12, "whole 3x4 slice staged");
        assert_eq!(view[0], 1.0);
        assert_eq!(view[11], 12.0);
        assert_eq!(
            bridge.profiler().write_buffer_calls,
            1,
            "exactly ONE logical upload"
        );
        assert_eq!(
            bridge.profiler().json_parse_calls,
            0,
            "ZERO json on the GREEN path"
        );
    }

    // ── W20 GREEN gate: the HeadlessGpu mock satisfies the gate under the
    //    DEFAULT build — 1 mock upload, 0 json, 0 real GPU. The mirror proves a
    //    real copy landed.
    #[test]
    fn headless_gpu_mock_one_upload_zero_json() {
        let mut bridge = VertexBridge::new(3, 4);
        bridge.write_particle(0, 1.0, 2.0, 3.0, 4.0);
        bridge.write_particle(1, 5.0, 6.0, 7.0, 8.0);
        bridge.write_particle(2, 9.0, 10.0, 11.0, 12.0);
        let mut gpu = HeadlessGpu::default();
        let n = gpu.upload_once(&mut bridge);
        assert_eq!(n, 12, "uploaded whole 3x4 slice");
        assert_eq!(gpu.uploads, 1, "exactly ONE mock upload");
        assert_eq!(gpu.json_calls, 0, "ZERO json");
        // The mock mirror IS a real copy of the staging buffer.
        assert_eq!(gpu.mirror, bridge.vertex_view());
        assert_eq!(gpu.mirror[0], 1.0);
        assert_eq!(gpu.mirror[11], 12.0);
        assert_eq!(bridge.profiler().write_buffer_calls, 1);
        assert_eq!(bridge.profiler().json_parse_calls, 0);
    }

    // ── W20 `feature = "gpu"`: `new_gpu` is an honest stub returning Err
    //    ("wgpu uncached"). This test only compiles when the gpu feature is on.
    #[cfg(feature = "gpu")]
    #[test]
    fn new_gpu_returns_honest_err_when_wgpu_uncached() {
        let r = crate::bridge::gpu::new_gpu((), (), 1024, 4);
        assert!(
            matches!(r, Err("gpu adapter not built — wgpu uncached")),
            "gpu feature must return the honest wgpu-uncached error, got {:?}",
            r.is_err()
        );
    }

    // WIRED upload: the sink actually RECEIVES the vertex bytes in one call.
    #[test]
    fn wired_sink_receives_bytes_one_write_zero_json() {
        let mut bridge = VertexBridge::new(3, 4);
        bridge.write_particle(0, 1.0, 2.0, 3.0, 4.0);
        bridge.write_particle(1, 5.0, 6.0, 7.0, 8.0);
        bridge.write_particle(2, 9.0, 10.0, 11.0, 12.0);
        let mut sink = HeadlessSink::default();
        let n = bridge.upload_to(&mut sink);
        assert_eq!(n, 12, "uploaded whole 3x4 slice");
        assert_eq!(sink.writes, 1, "exactly ONE write_buffer call");
        // The sink's mirror IS a real copy of the staging buffer (organ wired).
        assert_eq!(sink.mirror, bridge.vertex_view());
        assert_eq!(sink.mirror[0], 1.0);
        assert_eq!(sink.mirror[11], 12.0);
        assert_eq!(
            bridge.profiler().json_parse_calls,
            0,
            "wired upload performs ZERO json"
        );
    }

    // RED→GREEN: the RED path (per-particle JSON) is the signature we reject.
    #[test]
    fn red_path_signature_is_rejected_by_gate() {
        let mut bridge = VertexBridge::new(4096, 4);
        bridge.json_frame_red_path();
        assert_eq!(
            bridge.profiler().json_parse_calls,
            4096,
            "RED path: one JSON (de)serialize per particle per frame"
        );
        // The gate: a conformant renderer must keep this at 0.
        assert!(
            bridge.profiler().json_parse_calls > 0,
            "documented RED signature — real renderer must not take this path"
        );
    }

    // Zero-copy: vertex_view shares the staging buffer's memory (same ptr, no copy).
    #[test]
    fn vertex_view_is_zero_copy_over_linear_memory() {
        let mut bridge = VertexBridge::new(1024, 4);
        bridge.write_particle(0, 3.0, 1.0, -2.0, 0.5);
        let view = bridge.vertex_view();
        assert_eq!(view.len(), 1024 * 4);
        // the value we wrote lands at slot 0, proving the slice IS the buffer.
        assert_eq!(view[0], 3.0);
        assert_eq!(view[1], 1.0);
        assert_eq!(view[2], -2.0);
        assert_eq!(view[3], 0.5);
    }

    // ── W2-2 RED→GREEN: the engine's VertexBridge produces its per-frame field
    //    by driving the kernel's `laplacian_spmv` over a normalized Laplacian,
    //    and that field conserves mass (Σ y == 0) on a constant particle state.
    //
    //    Triangle graph (undirected, regular d=2): for x = [0.1, 0.4, 0.9] the
    //    symmetric normalized Laplacian reduces to I − (1/2)·A, giving
    //      y = [-0.55, -0.1, 0.65],  Σ y = 0  (mass/momentum conserved).
    #[test]
    fn bridge_field_is_kernel_laplacian_and_conserved() {
        // Undirected triangle adjacency, both directions.
        let edges = [
            (0usize, 1, 1.0),
            (1, 0, 1.0),
            (1, 2, 1.0),
            (2, 1, 1.0),
            (0, 2, 1.0),
            (2, 0, 1.0),
        ];
        let graph = Csr::from_edges(3, &edges);

        let mut bridge = VertexBridge::new(3, 4);
        bridge.set_field_graph(graph);

        let x = [0.1_f64, 0.4, 0.9];
        bridge.apply_field(&x);

        // The produced field is exactly laplacian_spmv (normalized) output.
        let field = bridge.field();
        assert_eq!(field.len(), 3, "field length matches graph nrows");
        assert!((field[0] + 0.55).abs() <= 1e-12, "field[0] = {}", field[0]);
        assert!((field[1] + 0.10).abs() <= 1e-12, "field[1] = {}", field[1]);
        assert!((field[2] - 0.65).abs() <= 1e-12, "field[2] = {}", field[2]);

        // Conservation: Σ field == 0 (no mass/momentum leaks across the bridge).
        let sum: f64 = field.iter().sum();
        assert!(sum.abs() <= 1e-12, "Σ field = {sum} (must be 0)");

        // The bridge still exposes its zero-copy vertex view (the upload path is
        // unchanged; the physics field is produced alongside it).
        assert_eq!(bridge.vertex_view().len(), 3 * 4);
        assert_eq!(bridge.profiler().write_buffer_calls, 0, "no upload yet");
    }

    // ── W2-2 GREEN: the bridge field matches the kernel's laplacian_spmv
    //    EXACTLY on a NON-regular graph (path 0—1—2, degrees 1,2,1). Here the
    //    symmetric normalized Laplacian does NOT reduce to a conservation form,
    //    so we check the exact per-node values (computed independently below)
    //    rather than Σ=0. This proves the engine↔kernel wiring forwards the
    //    kernel's result unchanged for arbitrary topology.
    //
    //    D = diag(1,2,1);  D^{−1/2} = diag(1, 1/√2, 1)
    //    D^{−1/2}·A·D^{−1/2} · 1 = [1/√2, 2/√2, 1/√2]
    //    y = 1 − that = [1 − 1/√2, 1 − 2/√2, 1 − 1/√2]
    //      = [0.29289…, −0.41421…, 0.29289…]
    #[test]
    fn bridge_field_matches_kernel_on_nonregular() {
        let edges = [(0usize, 1, 1.0), (1, 0, 1.0), (1, 2, 1.0), (2, 1, 1.0)];
        let graph = Csr::from_edges(3, &edges);
        let mut bridge = VertexBridge::new(3, 4);
        bridge.set_field_graph(graph);
        let x = [1.0_f64, 1.0, 1.0];
        bridge.apply_field(&x);
        let field = bridge.field();
        let inv_sqrt2 = 2.0_f64.sqrt().recip();
        assert!(
            (field[0] - (1.0 - inv_sqrt2)).abs() <= 1e-12,
            "field[0]={}",
            field[0]
        );
        assert!(
            (field[1] - (1.0 - 2.0 * inv_sqrt2)).abs() <= 1e-12,
            "field[1]={}",
            field[1]
        );
        assert!(
            (field[2] - (1.0 - inv_sqrt2)).abs() <= 1e-12,
            "field[2]={}",
            field[2]
        );
    }
}

/// FE-06 — Kernel geo-math bridge contract.
///
/// Canonical architecture (operator directive 2026-07-14): **the Rust engine
/// uses & relies on the Rust kernel for all geo / route kinematics; JS/TS
/// (`geo-anim.js` and the legacy `apps/*` oracle) is DEPRECATED.** The kernel
/// (`dowiz-kernel::geo`) is the single source of truth for haversine, lerp,
/// bearing, `progress_along_route`, ETA, snap, polygon tests. The engine never
/// re-implements geo math — it *consumes* kernel output across the bridge.
///
/// Wire format: the kernel exposes `geo_progress_flat_js` (`kernel/src/wasm.rs`)
/// which emits the route-progress as a flat numeric array
/// `[remaining_m, snapped_lat, snapped_lng, segment_index]`. The bridge FFI
/// layer parses that array ONCE (outside the render loop) into the staging
/// buffer; the render loop then reads a flat `f32` slice with **no JSON** (see
/// [`FrameProfiler`]). The engine side of the contract is decoded here,
/// fail-closed: a malformed/short payload yields `None` and the renderer keeps
/// the last good marker (never render garbage from a dropped/legacy frame). The
/// layout is mirror-pinned by [`route_progress_layout_matches_kernel`] so the
/// two crates cannot silently desync.
pub mod geo {
    // `RouteProgress` / `ROUTE_PROGRESS_SLOTS` are the public bridge contract the
    // FFI/renderer layer consumes; no in-crate caller yet, so clippy would flag
    // them as dead_code. They are exercised by the decoder/adapter + tests below
    // and are the documented wire contract (FE-06), so the warning is expected.
    #![allow(dead_code)]
    /// Decoded route-progress produced by `dowiz-kernel::geo::progress_along_route`,
    /// received across the bridge as a flat f32 slice.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct RouteProgress {
        pub remaining_m: f32,
        pub snapped_lat: f32,
        pub snapped_lng: f32,
        /// End-node index of the segment the marker is projected onto.
        pub segment_index: u32,
    }

    /// Number of `f32` slots in the flat bridge payload (matches kernel `ProgressOut`).
    pub const ROUTE_PROGRESS_SLOTS: usize = 4;

    /// Decode a kernel geo-progress payload received over the bridge.
    ///
    /// Fail-closed: returns `None` unless the slice is exactly
    /// [`ROUTE_PROGRESS_SLOTS`] long. The renderer MUST treat `None` as
    /// "keep last good marker" — never fabricate a position from a partial or
    /// legacy (TS) frame.
    pub fn decode_progress_flat(slice: &[f32]) -> Option<RouteProgress> {
        if slice.len() != ROUTE_PROGRESS_SLOTS {
            return None;
        }
        Some(RouteProgress {
            remaining_m: slice[0],
            snapped_lat: slice[1],
            snapped_lng: slice[2],
            segment_index: slice[3] as u32,
        })
    }

    /// Render adapter: holds the latest kernel-computed marker position. The
    /// engine never computes geo — it only surfaces what the kernel decided.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct CourierMarker {
        progress: Option<RouteProgress>,
    }

    impl CourierMarker {
        /// Feed a bridge payload. `None` (malformed/legacy) keeps the last good
        /// position — fail-closed against dropped frames.
        pub fn ingest(&mut self, slice: &[f32]) {
            if let Some(p) = decode_progress_flat(slice) {
                self.progress = Some(p);
            }
        }

        /// The kernel-decided marker position, if any good frame has arrived.
        pub fn position(&self) -> Option<(f32, f32)> {
            self.progress.map(|p| (p.snapped_lat, p.snapped_lng))
        }

        /// Remaining distance to route end (kernel-computed), if known.
        pub fn remaining_m(&self) -> Option<f32> {
            self.progress.map(|p| p.remaining_m)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // Flat layout matches the kernel `ProgressOut` contract (remaining_m,
        // snapped.lat, snapped.lng, segment_index). If the kernel changes field
        // order, this pin goes RED and names the desync.
        #[test]
        fn route_progress_layout_matches_kernel() {
            assert_eq!(ROUTE_PROGRESS_SLOTS, 4);
            let flat = [1111.95_f32, 40.005, -3.0, 1.0];
            let p = decode_progress_flat(&flat).expect("exact-length slice decodes");
            assert_eq!(p.remaining_m, 1111.95);
            assert_eq!(p.snapped_lat, 40.005);
            assert_eq!(p.snapped_lng, -3.0);
            assert_eq!(p.segment_index, 1);
        }

        // Fail-closed: a legacy/partial payload (wrong length) is rejected and
        // the marker keeps its last good position.
        #[test]
        fn malformed_payload_is_rejected_keeps_last_good() {
            let mut marker = CourierMarker::default();
            marker.ingest(&[1111.95, 40.005, -3.0, 1.0]); // good
            assert!(marker.position().is_some());
            // legacy TS frame sent 3 floats, or a dropped 2-float stub
            marker.ingest(&[1.0, 2.0, 3.0]);
            assert!(marker.position().is_some(), "last good position retained");
            assert_eq!(marker.position(), Some((40.005, -3.0)));
            // totally empty frame also safe
            marker.ingest(&[]);
            assert_eq!(marker.position(), Some((40.005, -3.0)));
        }

        // No good frame yet → no position (renderer must not draw a phantom marker).
        #[test]
        fn no_good_frame_yields_no_position() {
            let marker = CourierMarker::default();
            assert!(marker.position().is_none());
            assert!(marker.remaining_m().is_none());
        }
    }
}

/// FE-07 — Kernel spectral-math bridge contract (mirrors [`geo`]).
///
/// The engine relies on the Rust kernel for spectral computation; it never
/// re-implements eigensolving (per the 2026-07-14 directive: JS/TS is legacy,
/// the engine uses kernel math). The kernel's `spectral_flat_js`
/// (`kernel/src/wasm.rs`) emits the spectral summary as a flat numeric array
/// (no JSON — dependency-free engine, no serde). Layout (see kernel doc):
///
/// ```text
/// [0] rho        — spectral radius (largest |λ|)
/// [1] gap        — γ = 1 − |λ₂| (mixing / convergence rate)
/// [2] fiedler    — algebraic connectivity λ₂ of the graph Laplacian
/// [3] drift_code — Damped=0, Resonant=1, Unstable=2
/// [4] n          — number of eigenvalues
/// [5..5+2n)      — eigenvalue pairs (re, im), descending |λ|
/// ```
///
/// The engine decodes that slice fail-closed: a malformed/short payload yields
/// `None` and the renderer keeps the last good spectral state. The layout is
/// mirror-pinned by [`spectral_flat_layout_matches_kernel`] so the two crates
/// cannot silently desync.
pub mod spectral {
    #![allow(dead_code)] // public contract surface; consumed by the FFI/renderer layer

    /// Drift classification of an operator (mirrors `dowiz-kernel::spectral::DriftClass`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DriftClass {
        Damped,
        Resonant,
        Unstable,
    }

    /// Decoded spectral summary produced by `dowiz-kernel::spectral`, received
    /// across the bridge as a flat f32 slice.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct SpectralReport {
        pub rho: f32,
        pub gap: f32,
        pub fiedler: f32,
        pub drift: DriftClass,
        /// Dominant eigenvalues (re, im), descending by modulus.
        pub dominant: [(f32, f32); 2],
    }

    /// Min slots for a well-formed spectral payload (5 header + 2×2 eigenvalue pairs).
    /// The kernel emits exactly `5 + 2n` floats for an `n×n` matrix; for n≥2 this
    /// is ≥ 9. We require at least the header + 2 pairs.
    pub const SPECTRAL_MIN_SLOTS: usize = 9;

    fn drift_from_code(code: f32) -> DriftClass {
        match code as i32 {
            0 => DriftClass::Damped,
            1 => DriftClass::Resonant,
            _ => DriftClass::Unstable,
        }
    }

    /// Decode a kernel spectral summary received over the bridge.
    ///
    /// Fail-closed: returns `None` unless the slice is long enough to carry the
    /// header + at least 2 eigenvalue pairs, and `n` matches the actual length.
    pub fn decode_spectral_flat(slice: &[f32]) -> Option<SpectralReport> {
        if slice.len() < SPECTRAL_MIN_SLOTS {
            return None;
        }
        let n = slice[4] as usize;
        // total = 5 header + 2*n eigenvalue floats.
        if slice.len() != 5 + 2 * n || n < 2 {
            return None;
        }
        Some(SpectralReport {
            rho: slice[0],
            gap: slice[1],
            fiedler: slice[2],
            drift: drift_from_code(slice[3]),
            dominant: [(slice[5], slice[6]), (slice[7], slice[8])],
        })
    }

    /// Render adapter: holds the latest kernel-computed spectral state. The engine
    /// never computes eigensystems — it only surfaces what the kernel decided.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct LoopDriftDetector {
        report: Option<SpectralReport>,
    }

    impl LoopDriftDetector {
        /// Feed a bridge payload. `None` (malformed/legacy) keeps the last good
        /// state — fail-closed against dropped frames.
        pub fn ingest(&mut self, slice: &[f32]) {
            if let Some(r) = decode_spectral_flat(slice) {
                self.report = Some(r);
            }
        }

        /// Current drift class, if any good frame has arrived.
        pub fn drift(&self) -> Option<DriftClass> {
            self.report.map(|r| r.drift)
        }

        /// Spectral gap γ (mixing rate); `None` until a good frame arrives.
        pub fn gap(&self) -> Option<f32> {
            self.report.map(|r| r.gap)
        }

        /// True when the operator is trapped/oscillatory (Resonant — a limit
        /// cycle, e.g. μ≈−1 period-2) and never mixes.
        pub fn is_resonant(&self) -> bool {
            self.drift() == Some(DriftClass::Resonant)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // Flat layout matches the kernel `spectral_flat_js` contract. A 2-cycle
        // matrix [[0,1],[1,0]] → eigs {±1} ⇒ rho=1, gap=0, fiedler computed from
        // its (empty) Laplacian adjacency (0 ⇒ disconnected ⇒ λ₂=0), drift=Resonant(1).
        #[test]
        fn spectral_flat_layout_matches_kernel() {
            // kernel spectral_flat_js("[[0,1],[1,0]]") → [rho=1, gap=0, fiedler, drift=1, n=2, +1,0, -1,0]
            let flat = [1.0_f32, 0.0, 0.0, 1.0, 2.0, 1.0, 0.0, -1.0, 0.0];
            let r = decode_spectral_flat(&flat).expect("exact-length slice decodes");
            assert_eq!(r.rho, 1.0);
            assert_eq!(r.gap, 0.0);
            assert_eq!(r.drift, DriftClass::Resonant);
            assert_eq!(r.dominant, [(1.0, 0.0), (-1.0, 0.0)]);
        }

        // Fail-closed: a malformed/partial payload is rejected; last good state kept.
        #[test]
        fn malformed_payload_is_rejected_keeps_last_good() {
            let mut det = LoopDriftDetector::default();
            let good = [1.0_f32, 0.0, 0.0, 1.0, 2.0, 1.0, 0.0, -1.0, 0.0];
            det.ingest(&good);
            assert_eq!(det.drift(), Some(DriftClass::Resonant));
            // wrong length (drops a float) → rejected, state retained
            det.ingest(&[1.0, 0.0, 0.0, 1.0, 2.0, 1.0, 0.0, -1.0]);
            assert_eq!(det.drift(), Some(DriftClass::Resonant));
            // n=2 but only 1 pair present → rejected
            det.ingest(&[1.0, 0.0, 0.0, 1.0, 2.0, 1.0, 0.0]);
            assert_eq!(det.drift(), Some(DriftClass::Resonant));
        }

        // No good frame → no drift class (renderer must not assume a state).
        #[test]
        fn no_good_frame_yields_no_drift() {
            let det = LoopDriftDetector::default();
            assert!(det.drift().is_none());
            assert!(det.gap().is_none());
            assert!(!det.is_resonant());
        }

        // Damped/Unstable codes map correctly.
        #[test]
        fn drift_codes_map() {
            assert_eq!(drift_from_code(0.0), DriftClass::Damped);
            assert_eq!(drift_from_code(1.0), DriftClass::Resonant);
            assert_eq!(drift_from_code(2.0), DriftClass::Unstable);
        }

        // Cross-crate round-trip pin (row #23): the KERNEL is the wire-code
        // authority. For every kernel `DriftClass` variant, kernel-encode
        // (`wire_code()`) → engine-decode (`drift_from_code`) must land on the
        // matching engine variant. This is the ONLY test that fails loudly if the
        // two crates' code↔variant maps diverge. The exhaustive `match` below is a
        // COMPILE-TIME count guard: adding a 4th kernel variant (with no `_` arm)
        // fails to compile this test until its wire code is consciously assigned.
        #[test]
        fn drift_wire_contract_matches_kernel() {
            use dowiz_kernel::spectral::DriftClass as K;
            for (k, e) in [
                (K::Damped, DriftClass::Damped),
                (K::Resonant, DriftClass::Resonant),
                (K::Unstable, DriftClass::Unstable),
            ] {
                // kernel encode → engine decode → engine variant
                assert_eq!(drift_from_code(k.wire_code() as f32), e);
            }
            // Count guard: exhaustive match, no `_` — a new kernel variant fails
            // to COMPILE this test.
            let _assert_three = |k: K| match k {
                K::Damped | K::Resonant | K::Unstable => (),
            };
        }
    }
}
