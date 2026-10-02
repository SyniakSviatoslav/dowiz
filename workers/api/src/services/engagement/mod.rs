//! What the venue says to people who are not ordering right now: its posts,
//! and the assistant that drafts them.

pub mod assist;
pub mod posts;
pub mod verdict;
pub mod voice;

/// The engagement routes, through the route seam (W-COV C2).
#[cfg(test)]
#[path = "routes/tests.rs"]
mod route_tests;
