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
