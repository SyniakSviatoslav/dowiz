//! What an order's status MEANS, asked of the FSM instead of spelled out.
//!
//! THE SAME NINE QUESTIONS AS THE WORKER'S `services::orders::status`, and for
//! the same reason: this file and `workers/api` each wrote the lists by hand
//! and neither list agreed with the other or with `allowed_next`. The
//! predicates themselves are on `OrderStatus` in `dowiz-core::order_machine`,
//! beside the statuses they classify; this is only the `&str` doorway, because
//! an order arrives here as JSON.

use dowiz_core::order_machine::OrderStatus;

/// Is this order over?
///
/// AN UNREADABLE STATUS IS NOT OVER. A running order wrongly called finished
/// disappears from the kitchen's queue; one wrongly called running is visibly
/// stuck, and somebody fixes it.
pub fn is_terminal(status: &str) -> bool {
    OrderStatus::from_str(status).is_some_and(|s| s.is_terminal())
}

/// Did the venue take this order's money?
///
/// AN UNKNOWN STATUS COUNTS AS MONEY, deliberately: the other way round, a
/// status this binary has not been rebuilt for silently removes every order in
/// it from the takings, and an owner's revenue falls with nothing to explain
/// it. Counting is visible; vanishing is not.
pub fn took_money(status: &str) -> bool {
    OrderStatus::from_str(status).is_none_or(|s| s.took_money())
}

/// What the VENUE took from one order, in minor units. The tip is the
/// courier's, passing through.
pub fn venue_took(total: i64, tip: i64, status: &str) -> i64 {
    if took_money(status) {
        total - tip
    } else {
        0
    }
}
