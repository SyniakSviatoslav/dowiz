//! Who orders here, without saying who they are.
//!
//! THERE IS NO CUSTOMER REGISTRY BEHIND THIS. The list is a fold over the
//! orders, computed per request and stored nowhere, so the venue holds exactly
//! what it held before. What a registry would add is not the data but the
//! CONVENIENCE — a ready-made list, sorted by value, one click from export —
//! so the protection lives where it can still do work: redacted by default,
//! un-redacting is deliberate, and the act is written into the append-only log.

// `redact` is NOT here: it lives in `dowiz_hub::redact` because the native
// server had a character-for-character copy of it, with the same defect and
// the same absence of tests.
pub mod alias;
pub mod alias_routes;
pub mod allergy;
pub mod at_placement;
pub mod consent_log;
pub mod consent_routes;
pub mod forget;
pub mod handlers;
pub mod identity;
pub mod record;
pub mod record_routes;
pub mod roll;
/// The guest's taste, scored with their consent (W-MR0 MR8; the one server scorer, no-scoring.sh).
pub mod taste;
pub mod taste_routes;
/// W-SENSE row 7: the owner's segment builder route (the rule is `taste/builder.rs`).
pub mod taste_builder;
pub mod view;

#[cfg(test)]
mod tests;

/// The customer routes, through the route seam (W-COV C2).
#[cfg(test)]
#[path = "routes/tests.rs"]
mod route_tests;
