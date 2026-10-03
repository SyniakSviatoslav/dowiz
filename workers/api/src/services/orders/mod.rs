//! One order, and what its status means.

pub mod feedback;
pub mod aggregator;
/// The customer <-> courier chat of one order (operator 2026-10-02).
pub mod chat;
pub mod kitchen_ack;
pub mod legs;
pub mod mine;
/// The offline cash sale and its 48 h fiscal queue (W-OFFSALE, OF3).
pub mod offline_sale;
pub mod print;
/// `GET /api/order/:id` (moved out of `lib.rs`, W-COV C2).
pub mod read;
pub mod refund;
pub mod room;
pub mod status;

#[cfg(test)]
mod tests;

/// The printer routes, through the route seam (W-COV C2).
#[cfg(test)]
#[path = "print_routes/tests.rs"]
mod print_routes;
