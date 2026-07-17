//! T8 / A6 — digest-pinned determinism test for the integer-CORDIC sin/cos primitive.
//!
//! These tests prove (and keep proving) that the Q30 CORDIC kernel in `crate::cordic`
//! is *bit-identical across runs* and *sensitive to adversarial mutation*:
//!
//!  * `cordic_digest_pinned` recomputes the FNV-1a digest from this module's own output over the
//!    canonical 51,471-sample sweep, and asserts it equals the pinned `CORDIC_SINCOS_DIGEST`
//!    (doc 20 item 4: measured bit-identical twice). The digest is COMPUTED, never hardcoded-
//!    claimed.
//!  * `cordic_digest_has_teeth` runs the same sweep with `ITERS - 1` rotations and asserts the
//!    digest DIFFERS — proving the pin is a real falsifier, not a dead constant.
//!  * `cordic_cross_arch_identical` is `#[ignore]`d: it states the activation condition for the
//!    W1-L1 cross-arch leg (x86_64 + aarch64 both emit the digest) per the Batch-1 §5.6 deferred-
//!    seam convention. It is NOT claimed to have run.

use eqc_rs::cordic::{compute_digest, compute_digest_with_iters, CORDIC_SINCOS_DIGEST};

#[test]
fn cordic_digest_pinned() {
    // Recompute the digest live from the module's output — do NOT trust the constant blindly.
    let measured = compute_digest();
    assert_eq!(
        measured, CORDIC_SINCOS_DIGEST,
        "CORDIC digest drifted from the pinned value — the kernel is no longer bit-identical\n\
         measured = 0x{measured:016x}, pinned = 0x{CORDIC_SINCOS_DIGEST:016x}"
    );
}

#[test]
fn cordic_digest_has_teeth() {
    // Adversarial mutation: one fewer rotation iteration MUST change the digest.
    // If this fails the digest is a dead constant and proves nothing.
    use eqc_rs::cordic::ITERS;
    let mutated = compute_digest_with_iters(ITERS - 1);
    assert_ne!(
        mutated, CORDIC_SINCOS_DIGEST,
        "ITERS-1 did NOT change the digest — the pin has no teeth (0x{mutated:016x})"
    );
}

/// # Ignored — W1-L1 cross-arch leg (deferred seam, Batch-1 §5.6 convention).
///
/// ACTIVATION CONDITION: a non-x86_64 aarch64 runner (real hardware, qemu-aarch64, or a
/// cross-compiled+run target such as `aarch64-unknown-linux-gnu` under qemu) must be available,
/// AND this test must be executed on BOTH the native x86_64 host and that aarch64 target with
/// the resulting two digests byte-compared equal. Until an aarch64 runner exists in CI, this
/// test is `#[ignore]`d and the cross-arch proof is an explicitly-OPEN checklist line — it is
/// NOT silently claimed to have passed. The kernel's only ops are `i64` +,-,<,==,>> (arithmetic
/// shift), whose Rust semantics are identical on every target, so the single-host `cordic_digest_pinned`
/// above is the standing cross-target proof on any host that runs it.
///
/// To activate (example):
///   cargo test --release --target aarch64-unknown-linux-gnu -- --ignored cordic_cross_arch_identical
/// then compare `measured` against the x86_64 `CORDIC_SINCOS_DIGEST`.
#[test]
#[ignore = "cross-arch aarch64 runner not available; activation condition in doc comment"]
fn cordic_cross_arch_identical() {
    let measured = compute_digest();
    assert_eq!(
        measured, CORDIC_SINCOS_DIGEST,
        "cross-arch digest mismatch: 0x{measured:016x} != 0x{CORDIC_SINCOS_DIGEST:016x}"
    );
}
