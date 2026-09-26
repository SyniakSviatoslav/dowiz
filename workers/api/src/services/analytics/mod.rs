//! The owner's numbers.

pub mod fold;
pub mod handler;
/// The kitchen's numbers: ingredients, dishes, losses, by day (card I7).
pub mod kitchen;

pub use handler::analytics;

#[cfg(test)]
mod tests;
