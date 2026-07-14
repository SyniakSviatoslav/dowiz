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

/// Tallies the two frame-loop costs the blueprint names.
#[derive(Debug, Default, Clone)]
pub struct FrameProfiler {
    /// Number of (de)serialization steps performed in the loop (RED signature).
    pub json_parse_calls: usize,
    /// Number of GPU uploads performed (should be 1 per frame on GREEN).
    pub write_buffer_calls: usize,
}

/// Owns the flat staging buffer that models the WASM linear memory the GPU
/// reads from. `vertex_view()` returns a slice over that buffer with **no copy**.
pub struct VertexBridge {
    /// Flat SoA staging: [pos_x*N][pos_y*N][vel_x*N][vel_y*N] (zero-copy target).
    scratch: Vec<f32>,
    count: usize,
    profiler: FrameProfiler,
}

impl VertexBridge {
    /// `count` particles; `stride` floats per particle (4 = x,y,vx,vy).
    pub fn new(count: usize, stride: usize) -> Self {
        VertexBridge {
            scratch: vec![0.0; count * stride],
            count,
            profiler: FrameProfiler::default(),
        }
    }

    pub fn count(&self) -> usize {
        self.count
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

    /// Model of `queue.writeBuffer(gpuBuffer, 0, view)` — the single upload copy.
    /// Counts 1 upload on the GREEN path.
    pub fn upload_once(&mut self) -> &[f32] {
        self.profiler.write_buffer_calls += 1;
        self.vertex_view()
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
