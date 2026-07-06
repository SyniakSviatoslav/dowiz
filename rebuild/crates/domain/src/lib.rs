//! `domain` — the invariant core of the DeliveryOS Rust rebuild (Phase A).
//!
//! Pure, no IO: this crate must never depend on sqlx/tokio/axum. It ports the semantics that
//! currently live in `packages/domain/src` (Node/TypeScript) — money arithmetic, the order-status
//! transition machine, tenant identity, and the ADR-0010 error envelope shape — as a compiled,
//! exhaustively-tested contract that the `api` crate (and later surfaces) build on.
#![forbid(unsafe_code)]
// `unwrap`/`expect` in test assertions is idiomatic (a panicking test IS the failing test) — these
// two lints exist to police production code paths, not `#[cfg(test)]` modules, so they're relaxed
// there only. The workspace-level `deny` still applies to every non-test line in this crate.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod codec;
pub mod error;
pub mod kernel;
pub mod money;
pub mod order_status;
pub mod tenant;

pub use codec::{CodecError, canonical_bytes, decode_log, encode_log, from_bytes};
pub use error::{DomainError, ErrorCode, ErrorEnvelope};
pub use kernel::{
    Command, CommandHash, Envelope, Event, OrderState, OrderTotals, Ts, decide, fold, replay,
    replay_envelopes,
};
// Sovereign-core money composition (GRAND-PLAN 0b-1) — re-exported so the shell adapter
// (`api::routes::orders::pricing`) and pg.rs can `use domain::kernel::pricing::{…}` (or the flat
// `domain::…`) for the pure integer money surface. The core `DeliveryTier`/`FeeLocation` are
// integer-meter; the shell keeps its own f64 same-named shapes (see kernel::pricing doc-comments).
pub use kernel::pricing::{
    DeliveryTier, FeeLocation, GroupInfo, ModifierInfo, PricedModifierRow, PricedOrderItemRow,
    PricingError, PricingItem, PricingSnapshot, ProductInfo, apply_tax, charged_tax, compose_total,
    compute_line_total, compute_order_pricing, delivery_fee_for_order, resolve_delivery_fee,
};
pub use money::{Lek, MoneyError};
pub use order_status::{ALL_STATUSES, OrderStatus, assert_transition, can_transition, is_terminal};
pub use tenant::TenantId;
