//! PURE. What an order's status MEANS, asked once instead of spelled out
//! fourteen times.
//!
//! THE WORKER'S OWN DESCRIPTION SAYS IT: "every order decision is made by
//! dowiz-kernel's json_api, never re-implemented here". The statuses were the
//! exception. `grep -n 'REJECTED" | "CANCELLED'` over this crate and the
//! native server found FOURTEEN hand-written `matches!` arms over three
//! different notions of the same idea, and they did not agree:
//!
//! * `DELIVERED | REJECTED | CANCELLED` — "the order is over", in `feedback`;
//! * the same list PLUS `PICKED_UP`, in `live_eta`;
//! * `REJECTED | CANCELLED` — "the venue took no money", in four folds.
//!
//! The first of those is a DEFECT and it is fixed by deleting the list: a
//! customer who collected their order could never leave a note, because
//! `PICKED_UP` was not in `feedback`'s copy, and the sentence they got back
//! was "this order is still running -- call the venue if something is wrong".
//!
//! `dowiz_kernel::OrderStatus` is the authority and every question below is
//! asked of it. The predicates themselves live on the FSM in
//! `dowiz-core::order_machine`, next to the statuses they classify, because
//! `tools/native-spa-server` asks the same nine questions and cannot link
//! this crate.

use dowiz_kernel::OrderStatus;

/// Is this order over?
///
/// DELIVERED, PICKED_UP, REJECTED, CANCELLED and COMPENSATED_REFUND — the
/// kernel's own list, so a status added to the FSM is terminal here on the day
/// it is added and not on the day somebody remembers this file.
///
/// AN UNREADABLE STATUS IS NOT OVER. A running order wrongly called finished
/// disappears from the kitchen's queue; one wrongly called running is visibly
/// stuck, and somebody fixes it.
pub fn is_terminal(status: &str) -> bool {
    OrderStatus::from_str(status).is_some_and(|s| s.is_terminal())
}

/// Did the venue take this order's money?
///
/// A REFUSED ORDER IS NOT MONEY TAKEN, and neither is a refunded one — the
/// venue gave it back. `REFUNDING` is still the venue's money until the refund
/// completes, which is the one honest answer while it is in flight.
///
/// AN UNKNOWN STATUS COUNTS AS MONEY, deliberately. The other way round, the
/// day the kernel gains a status this Worker has not been redeployed for,
/// every order in it silently leaves the takings and an owner's revenue falls
/// with nothing to explain it. Counting is visible; vanishing is not.
pub fn took_money(status: &str) -> bool {
    OrderStatus::from_str(status).is_none_or(|s| s.took_money())
}

/// Is the kitchen working on this one? Accepted, not finished, not waiting to
/// be accepted.
pub fn is_active(status: &str) -> bool {
    OrderStatus::from_str(status).is_some_and(|s| s.is_active())
}

/// What the VENUE took from one order, in minor units.
///
/// THE TIP IS THE COURIER'S, passing through. This subtraction lived in four
/// places -- the dashboard, the analytics, the customer list and the native
/// server's copies of all three -- and the dashboard once counted `DELIVERED`
/// only while the others counted everything not refused, so the same product
/// showed an owner two different numbers depending on which deployment they
/// opened, on the same screen.
pub fn venue_took(total: i64, tip: i64, status: &str) -> i64 {
    if took_money(status) {
        total - tip
    } else {
        0
    }
}
