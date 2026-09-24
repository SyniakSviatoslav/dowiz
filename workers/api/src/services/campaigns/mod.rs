//! CAMPAIGNS (§2.5, §3.6 of BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22):
//! segment, preview the count and the cost, send through the outbox, measure
//! by promo code.
//!
//! A CAMPAIGN SYSTEM WITHOUT A CONSENT FOLD IS A SPAM SYSTEM WITH A QUEUE, so
//! the consent is not a filter applied on the way; it is a TYPE on the way
//! in. `send::entry` takes a `&Consented`, which only `consent::state` can
//! make (G1), and the drain asks the fold again before delivering (G4).
//!
//!   segment.rs   the closed list of predicates, pure
//!   campaign.rs  the `campaign` log (def / sent / gone) and its folds, pure
//!   audience.rs  orders ⋈ consent ⋈ cards → recipients holding the witness
//!   send.rs      recipients → outbox entries, bounded; the drain's gate
//!   template.rs  the approved WhatsApp template a campaign is sent as
//!   stop.rs      an inbound STOP → a consent withdrawal
//!   rail.rs      the drain's I/O hooks
//!   handlers.rs  the owner's routes

pub mod audience;
pub mod campaign;
pub mod handlers;
pub mod rail;
pub mod segment;
pub mod send;
pub mod stop;
pub mod template;
