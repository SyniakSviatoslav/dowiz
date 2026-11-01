//! Kani proof harnesses over the small pure functions of bebop-log and bebop-image (W-MG-KANI).
//!
//! WHAT THIS IS. Kani is a BOUNDED MODEL CHECKER: it turns a harness into a formula and asks a
//! solver for a counterexample among every input the harness allows. It is a development tool. It
//! is NOT a fuzzer (it runs nothing, generates no corpus, has no seed; this repo has no fuzzers, on
//! purpose) and it is NOT in any shipped build: this whole crate is behind `cfg(kani)`, which only
//! `cargo kani` sets, and no crate depends on it.
//!
//! WHAT A VERDICT WOULD MEAN. "All inputs" below always means all inputs WITHIN THE HARNESS'S BOUND,
//! and each module says its bound. A body one byte past a bound was not looked at. A loop past
//! `#[kani::unwind(n)]` is not silently dropped: the run reports a failed unwinding assertion.
//!
//! STATUS: NOT RUN. `.github/workflows/kani.yml` is the first thing that runs these, and until it
//! has, every property here is a STATEMENT TO CHECK, not a result. Kani is not installed on the box.
//!
//! Two habits in every harness:
//!   - "no panic" has no assert of its own: reaching any panic, slice overrun or arithmetic overflow
//!     is itself a failed check, so the parser call IS the property.
//!   - a `kani::cover!` beside each conditional assert: an assert under `if let Some(..)` passes
//!     vacuously if `Some` is unreachable. A cover that reports UNSATISFIABLE means the harness
//!     checked nothing, and that counts as red whatever the verdict line says.

#![cfg(kani)]

mod common;
mod forgotten;
mod idx;
mod id_message;
mod keys;
mod marks;
mod merge;
mod truncate;
