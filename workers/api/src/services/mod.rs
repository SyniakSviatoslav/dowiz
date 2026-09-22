//! The services `extra.rs` is being dismantled into.
//!
//! THE SHAPE, from blueprint 2 §5: one directory per area, at most ONE file in
//! it that touches a port, and everything else a function of its arguments.
//! The 300-line ceiling is a means, not an end — thirty files of a hundred
//! lines that all reach into each other are worse than the one file they came
//! from — so the split follows the TESTABILITY SEAM. What can be decided from
//! arguments alone moves out and gets tests; what needs a Durable Object stays
//! in a `mod.rs` that only sequences calls.
//!
//! WHY NOT ALL AT ONCE. Each slice lands its tests in the commit BEFORE the
//! handlers move, because a test written after a move tests the move.

pub mod courier;
pub mod customers;
pub mod ordering;
