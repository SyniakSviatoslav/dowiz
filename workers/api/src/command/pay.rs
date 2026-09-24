//! TAKE A PAYMENT: one `Paid` event with an amount, a method, a currency, a
//! signer, and — for cash — the till it went into.
//!
//! THE DECIDER MOVED to `dowiz_hub::room::pay` (D7 phase 1), with the currency
//! rule (`fx`); its header holds the rules (the split, the currency, cash and
//! the till, the confirmed frame). What stays here needs the Worker: the
//! wallet's debit (`wallet`, the kernel's ledger), its audit (`legs`) and the
//! tender fold (`tender`, the sitting projection).
//!
//! `decide` below is an adapter for `hubdo.rs`'s own `OrderView`, exactly as
//! in `command::amend`; it goes when that file re-exports the hub's.

pub use dowiz_hub::room::pay::{settles, PayIn, PayOut, Room};

use super::amend::hub_view;
use super::Refused;
use crate::hubdo::OrderView;
use serde_json::Value;
#[cfg(test)]
use serde_json::json;

pub mod fx;
pub mod legs;
pub mod tender;
pub mod wallet;

/// `dowiz_hub::room::pay::decide`: THE WHOLE PAYMENT, over the order image
/// already in memory; nothing is written unless every rule passed.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    current: Option<&OrderView>,
    input: &PayIn,
    room: &Room,
) -> Result<(Value, String, u64), Refused> {
    dowiz_hub::room::pay::decide(hub, current.map(hub_view).as_ref(), input, room)
}

#[cfg(test)]
mod tests;
