//! Item 7 (space-grade roadmap §C): planted-fault self-test for the `kani-gate`.
//!
//! A DELIBERATELY-BROKEN overflow, annotated `#[kani::should_panic]`. `cargo kani`
//! reports this harness SUCCESSFUL **only because the fault is actually caught** — if
//! the model checker ever stopped detecting the overflow (a misconfigured invocation, a
//! stripped assertion pass, a broken toolchain, a filter matching nothing), this harness
//! flips to FAILED and the gate goes RED. It runs on EVERY `kani-gate` invocation,
//! making synthesis §9.7's "at least one seeded fault demonstrably caught" a STANDING
//! property rather than a one-off demo — the exact analog of `ct_gate.rs`'s planted-leak
//! self-test. The RED-path demonstration (remove `should_panic` → gate RED) is recorded
//! in the PR per BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md §6.5.
//!
//! The whole file is behind `#[cfg(kani)]` (see lib.rs), so it is compiled out of every
//! `cargo build`/`cargo test` and adds nothing to `Cargo.toml`/`Cargo.lock`.

/// Planted i32 overflow. For symbolic `a, b`, the addition `a + b` overflows for some
/// pair, which Kani MUST report as a panic. `#[kani::should_panic]` inverts the verdict,
/// so this harness is SUCCESSFUL iff the overflow IS detected. Removing the attribute is
/// the §6.5 RED-path demonstration (the gate then reports this harness FAILED).
#[kani::proof]
#[kani::should_panic]
fn proof_selftest_planted_overflow() {
    let a: i32 = kani::any();
    let b: i32 = kani::any();
    // Unchecked `+` overflows for some (a, b); Kani reports a panic ⇒ should_panic ⇒ PASS.
    let _c = a + b;
    core::hint::black_box(_c);
}
