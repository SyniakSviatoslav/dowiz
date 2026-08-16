//! `wallet` — std host shim.
//!
//! The pure serde-free wallet core (`draft`, `outbox`, `record`, the
//! card-data/break-glass grep-gates) lives in `dowiz_core::wallet`. `transfer`
//! stays here: it reuses the `pq`-gated crypto primitives (`pq::x25519`,
//! `aes-gcm` volume) that are OPT-IN in the kernel and absent from the no_std
//! core.

pub use dowiz_core::wallet::*;

#[cfg(feature = "pq")]
pub mod transfer;
