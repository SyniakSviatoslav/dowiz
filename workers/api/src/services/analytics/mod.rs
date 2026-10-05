//! The owner's numbers.

/// The daily sales cube: the history the hot log no longer holds (W-HIST P2b).
pub mod cube;
pub mod fold;
/// The kitchen's forecast and prep list (W-PREP P6/P7).
pub mod forecast;
pub mod handler;
/// The owner's numbers over any period, with comparisons (W-HIST P2c).
pub mod history;
/// The kitchen's numbers: ingredients, dishes, losses, by day (card I7).
pub mod kitchen;
/// "Most ordered this week: N": the storefront badge and the owner's same number (W-MR0).
pub mod week_top;
/// Its public route, `GET /api/public/locations/:slug/menu/week`.
pub mod week_route;

pub use handler::analytics;

#[cfg(test)]
mod tests;
