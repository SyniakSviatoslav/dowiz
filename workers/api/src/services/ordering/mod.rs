//! Placing an order, and the preview of one.
//!
//! The pricer is here because BOTH the checkout and the promo preview need it
//! and used to have one each.

/// The basket's catalogue nodes, answered by the venue's object (BN1, `/fold/basket`).
pub mod basket;
pub mod channel;
pub mod fulfilment;
/// "Order for later": the chosen time, checked in the venue's zone (N4.4).
pub mod later;
pub mod preview;
pub mod promo_fields;
pub mod promotions;
pub mod pricing;
pub mod rates;
pub mod tax_block;
pub mod tax_cfg;

#[cfg(test)]
mod tests;
