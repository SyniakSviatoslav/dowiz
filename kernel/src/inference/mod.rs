//! §H toy-pilot inference arc — a quantized, constant-time-gated, golden-checksum-
//! guarded toy neural-network classifier (BLUEPRINT-ITEM-34…44, 47).
//!
//! # Item 36 (eqc Indexed-Summation IR) — ALREADY LANDED via item 32
//! `kernel/src/laplacian_eqc_parity.rs` + `tools/eqc-rs/src/lib.rs` carry the
//! `Index`/`IndexSum` nodes, the array-`eval` reference, the `emit_int_checked`
//! i32-accumulator Q-format loop, the `emit_proof_program` self-assert (compiles
//! + exits 0 vs the tree-walking `eval`), and the overflow-ceiling refusal. All
//! 6 item-36 unit tests in `eqc-rs` are GREEN and the item-32 Laplacian consumer
//! stays green — one IR serves both, verified. No new code needed here; item 36's
//! acceptance is a property of `eqc-rs`, asserted today.
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
pub mod workspace;
