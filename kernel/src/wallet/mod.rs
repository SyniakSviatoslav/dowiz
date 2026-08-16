//! `wallet` — std host shim.
//!
//! The pure serde-free wallet core (`draft`, `outbox`, `record`, `transfer`, and the
//! card-data/break-glass grep-gates) lives in `dowiz_core::wallet`. `transfer`'s
//! crypto (X25519 + SHAKE256 + AES-256-GCM) is now hand-rolled in the no_std core,
//! so it is no longer `pq`-gated.

pub use dowiz_core::wallet::*;
