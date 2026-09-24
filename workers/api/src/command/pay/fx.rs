//! A PAYMENT IN ANOTHER CURRENCY (BLUEPRINT-OPERATIONAL-BLIND-SPOTS P3-2).
//!
//! MOVED to `dowiz_hub::room::pay::fx` (D7 phase 1), whose header states the
//! rate unit and the rounding; re-exported so `command::pay::fx` still names
//! it and its tests below run against the moved code.

pub use dowiz_hub::room::pay::fx::{convert, settle, Settled, RATE_SCALE};

#[cfg(test)]
use {super::super::Refused, dowiz_core::money::Currency};

#[cfg(test)]
mod tests;
