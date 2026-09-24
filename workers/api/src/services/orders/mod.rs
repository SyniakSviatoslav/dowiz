//! One order, and what its status means.

pub mod feedback;
pub mod aggregator;
pub mod kitchen_ack;
pub mod legs;
pub mod mine;
pub mod print;
pub mod refund;
pub mod room;
pub mod status;

#[cfg(test)]
mod tests;
