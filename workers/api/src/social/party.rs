//! D18 (G6): A THREAD IS BOUND TO ITS PRINCIPAL, not only to the venue.
//!
//! THE DEFECT. `messages` and `send` asked `principal_at` -- "does this token
//! belong to this venue" -- and nothing else. Every customer token of the venue
//! does, and since `2cfab80b` anyone mints one by booking a table with no
//! account. With order X's id (couriers and staff all see it) that token read
//! and posted as CUSTOMER in X's thread; a courier read every conversation.
//!
//! THE RULE, the one `live.rs` already applies to the order's socket:
//!   * a customer token speaks and reads only in the thread of ITS order
//!     (`order_id == thread_id`); any other is a 404, never a 403 that would
//!     confirm the thread exists. A booking's token names `rsv_..`, which is
//!     no order, so it reaches no thread at all.
//!   * the owner is the venue;
//!   * staff read with a capability that works orders (`caps.allows`), as the
//!     console's socket does, and do not speak for the venue;
//!   * a courier neither reads nor speaks here.

use crate::auth::{Cap, Principal};
use dowiz_kernel::thread::Party;

/// Which side `p` is on in thread `thread_id`, or the refusal.
/// `speaking` is a post; a read is `false`.
pub fn thread_party(p: &Principal, thread_id: &str, speaking: bool) -> Result<Party, (u16, &'static str)> {
    match p {
        Principal::Owner { .. } => Ok(Party::Venue),
        Principal::Customer { order_id, .. } if order_id == thread_id => Ok(Party::Customer),
        Principal::Customer { .. } => Err((404, "not found")),
        Principal::Staff { .. } if speaking => Err((403, "staff do not speak in this thread")),
        Principal::Staff { caps, .. } if caps.allows(Cap::TakeOrders) || caps.allows(Cap::Advance) => Ok(Party::Venue),
        Principal::Staff { .. } => Err((403, "no capability for the venue's conversations")),
        Principal::Courier { .. } => Err((403, "a courier does not read or speak in this thread")),
    }
}

#[cfg(test)]
mod tests;
