//! §H toy-pilot inference arc — a quantized, constant-time-gated, golden-checksum-
//! guarded toy neural-network classifier (BLUEPRINT-ITEM-34…44, 47).
//!
//! Module layout (each item is its own submodule with a named `#[test]` proving
//! its falsifiable acceptance criterion):
//! - `fixed`    — item 35: fixed-point number-format + rounding-law spec (executable).
//! - `oracle`   — item 37: scalar, obviously-correct integer-domain reference (the "schoolbook").
//! - `workspace`— item 38: static tensor workspace on the arena (const offsets, zero mid-inference alloc).
//! - `spec`     — item 34: the toy-pilot architecture, D, weights, plane classification (the `f(x)=y` contract).
//! - `simd_i8`  — item 39: AVX2 quantized dot/matmul kernels, oracle fallback + differential target.
//! - `checksum` — item 40: per-layer golden CRC32 oracle + hard-fail to safe state.
//! - `weights`  — item 41: embedded (committed Rust) weight pipeline, SHA3 self-check, ML-DSA codesign.
//! - `scheduler`— item 42: fixed-sequence (cyclomatic-1) inference spine.
//! - `plane`    — item 43: constant-time inference gate (plane-classified; dudect design owed).
//!
//! The whole arc is integer-domain ⇒ bit-identical across native/wasm32 (item 42
//! cross-target proof). No new dependencies. Build model: `cargo test --offline --lib`.

pub mod fixed;
