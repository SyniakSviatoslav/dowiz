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

/// Compute a deterministic SHA3-256 model version identifier from `f64` weights.
///
/// Flattens each weight to its little-endian byte representation, concatenates,
/// and hashes the result with `sha3_256`. Two models with identical `&[f64]`
/// weights produce the same hash; any weight change flips the digest.
pub fn model_version_hash(weights: &[f64]) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(weights.len() * 8);
    for w in weights {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    crate::event_log::sha3_256(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_version_hash_deterministic() {
        let w = vec![1.0, 2.0, 3.0, -4.5, std::f64::consts::PI];
        let h1 = model_version_hash(&w);
        let h2 = model_version_hash(&w);
        assert_eq!(h1, h2, "same weights must produce identical hash");
    }

    #[test]
    fn model_version_hash_different_weights_differ() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0001];
        assert_ne!(model_version_hash(&a), model_version_hash(&b));
    }

    #[test]
    fn model_version_hash_empty_is_stable() {
        let h = model_version_hash(&[]);
        assert_eq!(h.len(), 32);
        assert_eq!(model_version_hash(&[]), h);
    }
}

pub mod fixed;
pub mod golden;
pub mod oracle;
pub mod simd_i8;
pub mod spec;
pub mod workspace;
