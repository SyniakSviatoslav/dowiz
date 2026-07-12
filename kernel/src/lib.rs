//! dowiz-kernel — deterministic core (Rust→WASM).
//! Canonical kernel. The TS app (`/root/dowiz` apps/*) is the legacy oracle; this replaces it.
//! No float on money, no I/O. Verified-by-Math: RED+GREEN tests per module.

pub mod analytics;
pub mod domain;
pub mod money;
pub mod order_machine;
pub mod pq;
pub mod wasm;

// Re-export the headline types so wasm-bindgen consumers and tests share one surface.
pub use analytics::{reduce_anomalies, ChannelEvent, ChannelLedger};
pub use domain::{apply_event, compute_order_total, place_order, Order, OrderItem};
pub use money::{
    apply_tax, assert_non_negative, compute_line_total, convert_all_to_eur_cents, round_half_up,
    to_minor_unit,
};
pub use order_machine::{assert_transition, fold_transitions, OrderStatus, TransitionError};
pub use wasm::{apply_event_js, channel_ledger_js, place_order_js, reduce_anomalies_js};
