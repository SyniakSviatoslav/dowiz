//! The owner's numbers.

/// The daily sales cube: the history the hot log no longer holds (W-HIST P2b).
pub mod cube;
pub mod fold;
pub mod handler;
/// The owner's numbers over any period, with comparisons (W-HIST P2c).
pub mod history;
/// The kitchen's numbers: ingredients, dishes, losses, by day (card I7).
pub mod kitchen;

pub use handler::analytics;

#[cfg(test)]
mod tests;
