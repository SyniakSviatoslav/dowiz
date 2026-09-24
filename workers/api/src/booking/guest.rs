//! Who may do what to a booking, and what a guest must give to make one.
//!
//! PURE: no Worker runtime, no clock, no storage. The handlers in `create`,
//! `reservations` and `venue` ask these questions; `guest/tests.rs` asks them
//! natively, refusal and positive twin side by side.
//!
//! A GUEST HAS NO ACCOUNT, the same as a guest who orders: they give a name
//! and a phone, and the booking comes back with a token scoped to that ONE
//! booking (`Claims::Customer` with the reservation id as its `order_id`).
//! That token reads and cancels the booking and nothing else.

use dowiz_kernel::reservation::{allowed_next, ReservationStatus};

use crate::auth::Principal;

/// A name longer than this is not a name somebody will say at the door.
pub const NAME_MAX: usize = 80;
/// The same floor the storefront's placement puts on a phone the courier dials.
pub const PHONE_MIN_DIGITS: usize = 8;
/// LIVE BOOKINGS ONE PHONE MAY HOLD AT ONCE. The form is open to the world, so
/// without a ceiling one script fills the room with tables nobody sits at
/// under names nobody chose. Three is a family's evening, a lunch and a
/// birthday; a fourth is a phone call, which the venue sheet carries.
pub const MAX_OPEN_PER_PHONE: usize = 3;

/// Which side of the counter a caller is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The guest who holds this booking's own token.
    Guest,
    /// The venue: its owner, or a member of its staff.
    Venue,
}

impl Side {
    /// The word written into the event log's `actor`. FROM THE PRINCIPAL,
    /// never from the body: the old route wrote whatever `actor` the caller
    /// sent, and `create` wrote `CUSTOMER` even when the owner booked.
    pub fn actor(&self) -> &'static str {
        match self {
            Side::Guest => "CUSTOMER",
            Side::Venue => "VENUE",
        }
    }
}

/// The side a principal is on FOR THIS BOOKING. A customer token for another
/// booking (or for an order) is answered 404, the way a principal of another
/// venue is: a 403 would confirm the booking exists.
pub fn side_of(p: &Principal, reservation_id: &str) -> Result<Side, (u16, &'static str)> {
    match p {
        Principal::Customer { order_id, .. } if order_id == reservation_id => Ok(Side::Guest),
        Principal::Customer { .. } => Err((404, "not found")),
        Principal::Owner { .. } | Principal::Staff { .. } => Ok(Side::Venue),
        Principal::Courier { .. } => Err((403, "forbidden role")),
    }
}

/// May this side move a booking INTO `to`? The kernel still decides whether
/// the move is legal from where the booking is; this decides whose move it
/// is. A guest can only withdraw; everything else is the venue's word -- and
/// the venue cannot sign a guest's cancellation.
pub fn may_move(side: Side, to: ReservationStatus) -> Result<(), &'static str> {
    use ReservationStatus::*;
    match (side, to) {
        (Side::Guest, CancelledByGuest) => Ok(()),
        (Side::Guest, _) => Err("a guest may only cancel their own booking"),
        (Side::Venue, CancelledByGuest) => {
            Err("the venue cannot cancel in the guest's name; use CANCELLED_BY_VENUE")
        }
        (Side::Venue, Requested) => Err("a booking cannot be moved back to REQUESTED"),
        (Side::Venue, _) => Ok(()),
    }
}

/// The moves this side may make from `from`: the kernel's list, narrowed to
/// the side's own. The console draws its buttons from THIS, so no status
/// chain is restated in JavaScript.
pub fn moves(side: Side, from: ReservationStatus) -> Vec<ReservationStatus> {
    allowed_next(from).iter().copied().filter(|to| may_move(side, *to).is_ok()).collect()
}

/// The guest's phone as the venue's customer key.
///
/// ONE PHONE, ONE KEY: `+355 69 123 4567` and `069 123 4567` are one guest, so
/// the key is taken from `identity::canonical_digits` when the number is one
/// of the venue's country's, and from the digits as typed otherwise (a
/// visitor's `+39…` is still a phone). HMAC'd with the venue's secret
/// (`customer_key`), so no index key carries the digits themselves -- and it
/// is the SAME key the customer list uses, so a booking and an order by one
/// person name one customer.
pub fn phone_key(secret: &[u8], phone: &str) -> Option<String> {
    use crate::services::customers::{handlers::customer_key, identity};
    let digits = phone.chars().filter(char::is_ascii_digit).count();
    if digits < PHONE_MIN_DIGITS {
        return None;
    }
    let canonical = identity::canonical_digits(phone, identity::VENUE_DIAL);
    Some(customer_key(secret, canonical.as_deref().unwrap_or(phone)))
}

/// What a guest must give: a name, and a phone the venue can call. Answers the
/// trimmed name and phone, or the sentence the form shows.
pub fn contact(name: &str, phone: &str) -> Result<(String, String), &'static str> {
    let (name, phone) = (name.trim(), phone.trim());
    if name.is_empty() {
        return Err("a booking needs the name it is under");
    }
    if name.chars().count() > NAME_MAX {
        return Err("that name is too long for a booking");
    }
    if phone.chars().filter(char::is_ascii_digit).count() < PHONE_MIN_DIGITS {
        return Err("a booking needs a phone number the venue can call");
    }
    Ok((name.to_string(), phone.to_string()))
}

#[cfg(test)]
mod tests;
