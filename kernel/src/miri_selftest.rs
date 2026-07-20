//! Item 52 (space-grade roadmap §J): planted-fault self-test for the `miri-gate`.
//!
//! A DELIBERATELY out-of-bounds read, gated so it is harmless on the native
//! backend but UB under the Miri interpreter. `cargo miri test miri_selftest::`
//! MUST report Undefined Behavior and halt — the gate (`scripts/miri-gate.sh`,
//! the `mode=miri` row with `min_tests = UB`) asserts that Miri DID report the
//! UB, so the gate is proven to detect real UB rather than passing vacuously.
//!
//! If a misconfigured Miri ever stopped detecting the UB (stripped
//! instrumentation, broken toolchain, a filter matching nothing), the run would
//! exit ZERO (UB uncaught) and the gate goes RED — the `ct_gate`/`kani_selftest`
//! planted-fault idiom, making "the gate detects real UB" a standing property,
//! not a one-off demo.
//!
//! The whole module is compiled under `cfg(test)` (so a plain `cargo test`
//! proves it compiles and the no-op native branch runs harmlessly — zero
//! footprint in any shipping/non-test build) AND under `cfg(miri)` (where the
//! planted-UB branch executes). Declared in `lib.rs` as
//! `#[cfg(any(test, miri))] mod miri_selftest;`.

/// Planted out-of-bounds read. Under native `cargo test` the body is the no-op
/// branch (proves compilation, executes NO UB). Under `cargo miri test` the
/// `cfg(miri)` branch runs and reads one past a length-1 slice — Miri MUST
/// report `Undefined Behavior`.
#[test]
fn miri_selftest_planted_oob() {
    // Native backend: no-op. (Runs under `cargo test` to prove this module
    // compiles; the UB branch below is compiled out here.)
    #[cfg(not(miri))]
    {
        // Intentionally empty — the real check only exists under Miri.
        let _sanity: u8 = 0;
        std::hint::black_box(_sanity);
    }
    // Miri interpreter: execute the planted UB. `slice` has length 1; index 99
    // is an out-of-bounds read that Miri is obligated to flag.
    #[cfg(miri)]
    {
        let slice: &[u8] = &[1u8];
        // SAFETY: intentionally ABSENT — this is the planted fault.
        let _val = unsafe { *slice.as_ptr().add(99) };
        std::hint::black_box(_val);
    }
}
