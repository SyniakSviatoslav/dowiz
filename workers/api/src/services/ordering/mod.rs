//! Placing an order, and the preview of one.
//!
//! The pricer is here because BOTH the checkout and the promo preview need it
//! and used to have one each.

pub mod preview;
pub mod pricing;

#[cfg(test)]
mod tests;
