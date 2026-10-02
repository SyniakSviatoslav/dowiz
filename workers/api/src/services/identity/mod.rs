//! Who may act, and the credentials that say so.
//!
//! The people and their memberships live in `crate::identity_store`; this is
//! the part a venue manages for itself.

pub mod keys;
pub mod staff;
pub mod staff_admin;
pub mod staff_rules;

/// Staff administration routes, through the route seam (W-COV C2).
#[cfg(test)]
#[path = "routes/tests.rs"]
mod route_tests;
