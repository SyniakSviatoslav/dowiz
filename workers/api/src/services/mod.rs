//! The services the Worker is built from.
//!
//! `extra.rs` WAS 3,853 LINES AND IS GONE. What follows is the shape it was
//! dismantled into, and the rule that decided where each piece landed.
//!
//! THE SHAPE, from blueprint 2 §5: one directory per area, at most ONE file in
//! it that touches a port, and everything else a function of its arguments.
//! The 300-line ceiling is a means, not an end — thirty files of a hundred
//! lines that all reach into each other are worse than the one file they came
//! from — so the split follows the TESTABILITY SEAM. What can be decided from
//! arguments alone moves out and gets tests; what needs a Durable Object stays
//! in a `mod.rs` that only sequences calls.
//!
//! WHY IT WAS DONE IN SLICES. Each slice landed its tests in the commit BEFORE
//! the handlers moved, because a test written after a move tests the move.
//!
//! WHAT A DIRECTORY HERE OWES. One `mod.rs` or handler file that touches a
//! port, everything else a function of its arguments, and a `tests.rs` for
//! whatever is a function of its arguments. A directory with no `tests.rs` is
//! saying it holds no rules -- which is true of pure orchestration and is a
//! claim worth re-reading whenever one of its files grows.

pub mod analytics;
/// The door before the body (W-FIX O9): the order, locked by a test.
mod authority_first;
pub mod campaigns;
pub mod catalogue;
pub mod courier;
pub mod customers;
pub mod engagement;
pub mod identity;
pub mod loyalty;
pub mod ordering;
pub mod operations;
pub mod orders;
pub mod venue;

/// The console's settings and operations routes, through the route seam (W-COV C2).
#[cfg(test)]
#[path = "console_routes/tests.rs"]
mod console_routes;

/// Money-touching routes, through the route seam (W-COV C2).
#[cfg(test)]
#[path = "money_routes/tests.rs"]
mod money_routes;

/// Public and platform routes, through the route seam (W-COV C2).
#[cfg(test)]
#[path = "public_routes/tests.rs"]
mod public_routes;

/// The fifteen catalogue readers of BN1, pinned before and after their move
/// into the object (`/fold/*`, lane W-BN1A).
#[cfg(test)]
#[path = "catalogue_routes/tests.rs"]
mod catalogue_routes;

/// The twenty-three remaining catalogue (and order-log) readers of BN1,
/// pinned before and after their move into the object (lane W-BN1B).
#[cfg(test)]
#[path = "catalogue_reads/tests.rs"]
mod catalogue_reads;
