//! PURE. The rules every room command shares: who may touch a round in which
//! status, why a line may leave it, what a line set costs, and how the shelf
//! follows the lines.
//!
//! MOVED to `dowiz_hub::room::rules` (D7 phase 1) with `amend` and `pay`, which
//! use it there; `transfer`, `refund`, `waste` and the exception fold name it
//! here, through this re-export.

pub use dowiz_hub::room::rules::*;
