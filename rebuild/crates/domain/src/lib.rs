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
pub use kernel::{Command, Event, OrderState, Ts, decide, fold, replay};
pub use money::{Lek, MoneyError};
pub use order_status::{ALL_STATUSES, OrderStatus, assert_transition, can_transition, is_terminal};
pub use tenant::TenantId;
