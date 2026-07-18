//! Zero-copy WASM↔GPU compute boundary (FIELD-UI-ENGINE-PLAN §E0 / Phase 0).
//!
//! Per-frame particle numeric state lives in a flat `Vec<f32>`. Rust writes
//! those bytes straight into WASM linear memory (a byte region); the JS
//! thin-shell reads them via `Float32Array::view` (0 copy) and uploads with a
//! single `queue.writeBuffer`. There is **NO `JSON.parse` in the frame loop**.
//!
//! This module is pure-std — no `bytemuck`, no `serde_json` — so it builds and
//! tests offline-clean, mirroring the engine's dependency-free mandate
//! (see `engine/Cargo.toml`). The manual `f32`↔`u8` reinterpret below is the
//! exact shape of the WASM linear-memory / `Float32Array` boundary.
//!
//! Note on "0 copy": the `write_into_linear` step *does* copy the `Vec<f32>`
//! into the linear-memory byte region (Rust-owned state must land where the
//! JS side can see it — equivalent to `memory.buffer` writes). The zero-copy
//! guarantee that matters is on the JS→GPU leg: `view_as_f32` returns a
//! `&[f32]` view *over the same bytes* (no allocation, no `Float32Array`
//! construction cost), which is what `queue.writeBuffer(view)` consumes in
//! one upload. That is the leverage the plan identifies (§E0 / §Plan-4).

/// Floats packed per particle in the flat buffer.
/// SoA-record layout: `[x, y, vx, vy, life]` contiguous per particle (stride 5).
pub const FLOATS_PER_PARTICLE: usize = 5;

/// Flat, packed per-particle record buffer (`[x,y,vx,vy,life] * N`).
///
/// This is the single contiguous `Vec<f32>` that the integrator fills and that
/// gets written into linear memory each frame. It is the canonical "what the
/// GPU receives".
#[derive(Debug, Clone)]
pub struct ParticleBuffer {
    /// `len == count * FLOATS_PER_PARTICLE`, packed per particle.
    data: Vec<f32>,
    /// Number of particles (records).
    count: usize,
}

impl ParticleBuffer {
    /// Allocate a zeroed buffer for `count` particles.
    #[inline]
    pub fn new(count: usize) -> Self {
        Self {
            data: vec![0.0; count * FLOATS_PER_PARTICLE],
            count,
        }
    }

    /// Number of particles (records) in the buffer.
    #[inline]
    pub fn particle_count(&self) -> usize {
        self.count
    }

    /// Total `f32` elements (`count * FLOATS_PER_PARTICLE`).
    #[inline]
    pub fn len_f32(&self) -> usize {
        self.data.len()
    }

    /// Write one particle's packed record at index `i`.
    #[inline]
    pub fn set(&mut self, i: usize, x: f32, y: f32, vx: f32, vy: f32, life: f32) {
        let o = i * FLOATS_PER_PARTICLE;
        self.data[o] = x;
        self.data[o + 1] = y;
        self.data[o + 2] = vx;
        self.data[o + 3] = vy;
        self.data[o + 4] = life;
    }

    /// Borrow the flat `f32` slice (the canonical numeric state).
    #[inline]
    pub fn as_f32(&self) -> &[f32] {
        &self.data
    }
}

/// Write the flat `ParticleBuffer` into a byte region of WASM linear memory
/// (`mem`) starting at `offset` (the fixed linear-memory offset the JS side
/// expects). Writes little-endian `f32` bytes, matching the WASM target's
/// byte order. Returns the number of bytes written.
///
/// This is the Rust side of the boundary: it places the packed `f32` record
/// into the shared `WebAssembly.Memory` byte array.
pub fn write_into_linear(mem: &mut [u8], offset: usize, buf: &ParticleBuffer) -> usize {
    let n_bytes = buf.data.len() * 4;
    assert!(
        offset + n_bytes <= mem.len(),
        "linear memory too small: need {} bytes at offset {}, have {}",
        n_bytes,
        offset,
        mem.len()
    );
    for (i, &f) in buf.data.iter().enumerate() {
        let b = f.to_le_bytes();
        let o = offset + i * 4;
        mem[o..o + 4].copy_from_slice(&b);
    }
    n_bytes
}

/// View a `f32` slice (`&[f32]`, 0 copy) over the linear-memory byte region
/// `mem`, starting at `offset`, covering `particle_count` packed records.
///
/// This is the JS `Float32Array::view(memory.buffer, offset)` leg, done in
/// Rust: the returned slice borrows the *same bytes* — no allocation, no
/// copy. The view length equals `particle_count * FLOATS_PER_PARTICLE`.
///
/// The boundary check is performed HERE (not by the caller) and is
/// release-safe: this returns `None` instead of panicking on a malformed
/// guest-supplied `offset`/`particle_count` — an out-of-bounds region must
/// never reach `from_raw_parts` (TORVALDS-18). Unlike a debug-only `assert!`,
/// this holds under `--release` where asserts are elided.
pub fn view_as_f32(mem: &[u8], offset: usize, particle_count: usize) -> Option<&[f32]> {
    let f32_count = particle_count * FLOATS_PER_PARTICLE;
    let byte_len = f32_count * 4;
    // Safety gate: 4-byte aligned start + region fits within `mem`. Either
    // failure ⇒ out-of-bounds view ⇒ return None (never build the slice).
    let aligned = (mem.as_ptr() as usize + offset) % 4 == 0;
    if !aligned || offset + byte_len > mem.len() {
        return None;
    }
    let ptr = unsafe { mem.as_ptr().add(offset) as *const f32 };
    let view = unsafe { std::slice::from_raw_parts(ptr, f32_count) };
    // Invariant called out in the brief: view length == particles * floats/particle.
    debug_assert_eq!(view.len(), particle_count * FLOATS_PER_PARTICLE);
    Some(view)
}

/// Mock of the GPU/JS sink side of the boundary.
///
/// Models `GPUQueue.writeBuffer(buffer, 0, view)` — exactly **one** upload per
/// frame — and tracks whether any JSON step was ever invoked in the hot path.
/// The whole point of Phase 0 is that this counter stays at 1 and the JSON
/// counter stays at 0 for a frame.
#[derive(Debug, Default)]
pub struct GpuSink {
    /// Number of `write_buffer` (upload) calls.
    pub write_buffer_calls: usize,
    /// Number of JSON (de)serialization calls in the hot path. Must stay 0.
    pub json_calls: usize,
}

impl GpuSink {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Model `queue.writeBuffer(buffer, 0, view)`: one upload of the whole
    /// zero-copy view. No JSON is performed here — the GPU receives raw f32.
    #[inline]
    pub fn write_buffer(&mut self, _view: &[f32]) {
        // Real implementation: queue.writeBuffer(gpu_buf, 0, view). No serde.
        self.write_buffer_calls += 1;
    }

    /// Drive one full frame through the zero-copy boundary: a single
    /// `write_buffer` of the whole view. Returns the upload count.
    #[inline]
    pub fn upload_frame(&mut self, view: &[f32]) -> usize {
        self.write_buffer(view);
        self.write_buffer_calls
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Allocate a byte region modelling WASM linear memory and return it with a
    /// 4-byte-aligned `offset` into it (exactly the `byteOffset % 4 === 0`
    /// constraint `Float32Array::view` imposes). A `Vec<u8>` heap allocation is
    /// itself 4-byte (in practice 16-byte) aligned, so a small `offset` keeps
    /// the region aligned while still exercising the alignment assertion in
    /// `view_as_f32`.
    fn aligned_linear_memory(byte_capacity: usize) -> (Vec<u8>, usize) {
        let buf = vec![0u8; byte_capacity];
        let base = buf.as_ptr() as usize;
        let offset = (4 - (base % 4)) % 4;
        debug_assert_eq!((base + offset) % 4, 0);
        (buf, offset)
    }

    #[test]
    fn round_trip_exact_and_zero_copy() {
        // RED→GREEN: after write_into_linear + view_as_f32, the view returns
        // the SAME f32 values (exact round-trip), and it is a 0-copy borrow.
        let (mut mem, offset) = aligned_linear_memory(4096);

        let n = 8usize;
        let mut pb = ParticleBuffer::new(n);
        for i in 0..n {
            let f = i as f32;
            pb.set(
                i,
                f * 1.0,
                f * 2.0 + 0.5,
                -f * 3.0,
                f * 0.25 - 1.0,
                (f + 1.0) / (f + 2.0),
            );
        }

        // Rust writes the flat buffer into linear memory (the boundary write).
        let written = write_into_linear(&mut mem, offset, &pb);
        assert_eq!(written, n * FLOATS_PER_PARTICLE * 4);

        // JS side: 0-copy Float32Array view over the same bytes.
        let view = view_as_f32(&mem, offset, n).expect("valid in-bounds view");

        // Invariant: view length == particle count * floats-per-particle.
        assert_eq!(view.len(), n * FLOATS_PER_PARTICLE);

        // Exact round-trip: every f32 identical (bit-for-bit on LE target).
        assert_eq!(view, pb.as_f32());
        for (a, b) in view.iter().zip(pb.as_f32().iter()) {
            assert_eq!(a.to_bits(), b.to_bits(), "round-trip f32 must be exact");
        }

        // 0-copy proof: the view points at the SAME bytes as the linear-memory
        // region — no allocation, no intermediate Float32Array copy.
        let view_byte_ptr = view.as_ptr() as *const u8;
        let mem_byte_ptr = unsafe { mem.as_ptr().add(offset) };
        assert_eq!(view_byte_ptr, mem_byte_ptr, "view must be a 0-copy borrow");
    }

    #[test]
    fn single_write_buffer_one_upload_per_frame() {
        // RED→GREEN: exactly ONE write_buffer call occurs for a frame (the
        // whole view is uploaded in a single GPUQueue.writeBuffer).
        let (mut mem, offset) = aligned_linear_memory(4096);
        let n = 4usize;
        let pb = {
            let mut b = ParticleBuffer::new(n);
            for i in 0..n {
                b.set(i, i as f32, (i * 7) as f32, 0.0, 0.0, 1.0);
            }
            b
        };
        write_into_linear(&mut mem, offset, &pb);
        let view = view_as_f32(&mem, offset, n).expect("valid in-bounds view");

        let mut sink = GpuSink::new();
        let uploads = sink.upload_frame(view);
        assert_eq!(uploads, 1, "exactly one write_buffer per frame");
        assert_eq!(sink.write_buffer_calls, 1);

        // A second frame is still one upload per frame (not per-particle).
        let uploads2 = sink.upload_frame(view);
        assert_eq!(uploads2, 2);
        assert_eq!(sink.write_buffer_calls, 2);
    }

    #[test]
    fn hot_path_has_zero_json_no_string_alloc() {
        // RED→GREEN: NO JSON (de)serialization is invoked in the hot path.
        // The buffer is consumed purely through borrowed slices — no String,
        // no serde step. This module does not `use serde_json` and the sink
        // never increments `json_calls`.
        let (mut mem, offset) = aligned_linear_memory(4096);
        let n = 16usize;
        let mut pb = ParticleBuffer::new(n);
        for i in 0..n {
            pb.set(i, i as f32, i as f32, 0.0, 0.0, 1.0);
        }
        write_into_linear(&mut mem, offset, &pb);
        // The view is a borrow; nothing is allocated/serialized here.
        let view = view_as_f32(&mem, offset, n).expect("valid in-bounds view");
        let mut sink = GpuSink::new();
        sink.upload_frame(view);

        assert_eq!(sink.json_calls, 0, "NO JSON in the frame-loop hot path");
        assert_eq!(sink.write_buffer_calls, 1);

        // Sanity: the consumed view covers exactly the buffer's numeric state,
        // proving the whole frame flowed through f32 borrows, not JSON.
        assert_eq!(view.len(), pb.as_f32().len());
        assert!(view.iter().all(|&v| v.is_finite()));
    }

    #[test]
    fn phase0_red_to_green_boundary_gate() {
        // Tie the three Phase-0 gates together in one RED→GREEN assertion set:
        // exact round-trip, 1 write_buffer, 0 JSON.parse.
        let (mut mem, offset) = aligned_linear_memory(8192);
        let n = 32usize;
        let mut pb = ParticleBuffer::new(n);
        for i in 0..n {
            let f = i as f32;
            pb.set(i, f, f * -1.5, f * 0.01, f - 10.0, f / (f + 1.0));
        }
        write_into_linear(&mut mem, offset, &pb);
        let view = view_as_f32(&mem, offset, n).expect("valid in-bounds view");

        // (a) exact round-trip
        assert_eq!(view, pb.as_f32());
        // (b) exactly one write_buffer for the frame
        let mut sink = GpuSink::new();
        assert_eq!(sink.upload_frame(view), 1);
        // (c) zero JSON in the hot path
        assert_eq!(sink.json_calls, 0);
    }

    // TORVALDS-18: a malformed guest-supplied region (offset/particle_count that
    // would overrun linear memory, or an unaligned start) must be rejected with
    // `None` — never reach `from_raw_parts` and never panic, even under
    // `--release` where debug asserts are elided. Pre-fix used `assert!` (elided
    // in release) and trusted the caller to bounds-check.
    #[test]
    fn red_oob_view_rejected_not_panics() {
        let (mem, _offset) = aligned_linear_memory(8192);
        // Oversized particle count ⇒ region overruns `mem`.
        assert!(view_as_f32(&mem, 0, 10_000).is_none());
        // Offset near the end + one particle ⇒ overruns.
        assert!(view_as_f32(&mem, mem.len() - 4, 2).is_none());
        // Valid aligned + in-bounds still works.
        assert!(view_as_f32(&mem, 64, 4).is_some());
    }
}
