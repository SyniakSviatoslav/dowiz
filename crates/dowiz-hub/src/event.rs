//! WHAT HAPPENED: the kinds of event the order log holds, an event as it is
//! read back, and the one decoder between the stored bytes and the two.
//!
//! The payload layout is `[kind][id_len][id bytes][order json]`, written by
//! `Hub::append` (`log.rs`). A record this build cannot read is not dropped:
//! it comes back as a `Quarantined` with the promise it broke.

use bebop_store::evlog::Record;

use crate::forget;

/// What happened. The kind is the first payload byte so a record can be routed
/// without parsing the JSON behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A new order, payload = the kernel's serialized order.
    Placed = 1,
    /// A status transition the KERNEL allowed, payload = the updated order.
    Advanced = 2,
    /// Money arrived. NOT a status transition: an order can be paid while still
    /// PENDING, and paying is not something the order FSM has an edge for. It is
    /// its own fact, recorded as its own event rather than smuggled through a
    /// transition the kernel would rightly refuse.
    Paid = 3,
    /// Somebody LOOKED at a customer's contact details.
    ///
    /// A read, in a log of writes, and deliberately so. The order log is the
    /// only append-only, tamper-evident thing this hub has, and an audit trail
    /// kept anywhere softer is an audit trail that can be tidied up. The
    /// payload names who looked, at whom, and when; it carries no contact
    /// details itself, because a log of who read a phone number that also
    /// contains the phone number has doubled the exposure it exists to record.
    Revealed = 4,
    /// A fact was ADDED to an order without its status moving: the customer's
    /// note, a courier's proof of delivery.
    ///
    /// Not `Advanced`, which means specifically a transition the KERNEL
    /// allowed. Writing an annotation as a transition would put events in the
    /// log that the order machine never decided, and the first person to audit
    /// the lifecycle would find a status change with no edge behind it.
    Noted = 5,
    /// A MARK IN THE LOG WHERE HISTORY WAS MOVED OUT OF IT.
    ///
    /// Written by `rotate` as the first record of a fresh hot image, naming the
    /// archived image's tip and how many events it holds. It is not an order
    /// and never folds into one; it exists so that a log can be read and the
    /// reader can tell the difference between "this venue has never taken an
    /// order" and "the older ones are somewhere else, and here is where".
    Checkpoint = 6,
    /// THE MONEY ON A ROUND CHANGED, and the kitchen had not taken it yet — or
    /// a person holding `void` took a line off after it had.
    ///
    /// NOT `Noted`, which promises that no money moved: every reader that asks
    /// "did this order's total change?" would otherwise have to open every
    /// note and look. A delta that may change `items`, `subtotal`, `discount`,
    /// `total`, `fulfilment.table`, `adjustments` and `amended`, and nothing
    /// else; written only by the object's `amend` and `transfer` commands, and
    /// signed by the person who made it. Not a status: the order FSM is never
    /// asked, and its golden signature does not move.
    /// (docs/design/BLUEPRINT-POS-THE-ROOM-2026-09-22.md §3.)
    Amended = 7,
    /// A PERSON WAS FORGOTTEN: the declaration that names how many records
    /// were redacted in place (`forget.rs`). Not an order; no contact details.
    Forgotten = 8,
}

impl EventKind {
    /// Does this event describe an ORDER? Everything that folds the log into
    /// orders asks this first.
    pub fn is_order(self) -> bool {
        matches!(
            self,
            EventKind::Placed
                | EventKind::Advanced
                | EventKind::Paid
                | EventKind::Noted
                | EventKind::Amended
        )
    }

    /// The kind a stored byte names, for a caller that carries events across
    /// a boundary the enum cannot cross -- a Durable Object's JSON body, say.
    pub fn from_u8(b: u8) -> Option<Self> {
        Self::from_byte(b)
    }

    fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(EventKind::Placed),
            2 => Some(EventKind::Advanced),
            3 => Some(EventKind::Paid),
            4 => Some(EventKind::Revealed),
            5 => Some(EventKind::Noted),
            6 => Some(EventKind::Checkpoint),
            7 => Some(EventKind::Amended),
            8 => Some(EventKind::Forgotten),
            _ => None,
        }
    }
}

/// One event, as read back out of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub order_id: String,
    /// The kernel's order JSON at the moment of the event.
    pub order_json: String,
    pub seq: u64,
}

/// A record the log holds and this build cannot read.
///
/// IT IS EVIDENCE, NOT AN ERROR MESSAGE, which is why it carries the id: the
/// record stays in the image verbatim, and a human can find it there. Nothing
/// here repairs anything — an automatic repair of a record nobody has looked at
/// is how a corrupted order becomes a plausible one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quarantined {
    /// The record's chain id in hex, as `chain_check` and the archives name it.
    pub id: String,
    /// Its position in the chain, newest first — the same order `events()` uses.
    pub at: usize,
    /// Which of the payload's promises it broke.
    pub reason: &'static str,
}

/// WHY THE FAILURE HAS A NAME. `decode` returned `None` for four different
/// things and every caller turned that into "not an event", so a record with an
/// unknown kind was indistinguishable from one whose payload was shredded. The
/// quarantine list is evidence for a human, and "it did not decode" is not
/// evidence. One decoder, four named refusals, and `decode` is this with the
/// name thrown away.
pub(crate) fn decode_or_reason(r: &Record) -> Result<Event, &'static str> {
    if r.payload.len() < 2 {
        return Err("short");
    }
    // The high bit marks a record redacted in place (`forget.rs`); the kind
    // is the low seven.
    let kind = EventKind::from_byte(r.payload[0] & !forget::REDACTED_BIT).ok_or("kind")?;
    let id_len = r.payload[1] as usize;
    if r.payload.len() < 2 + id_len {
        return Err("framing");
    }
    let order_id = String::from_utf8(r.payload[2..2 + id_len].to_vec()).map_err(|_| "id-utf8")?;
    let order_json =
        String::from_utf8(r.payload[2 + id_len..].to_vec()).map_err(|_| "json-utf8")?;
    Ok(Event { kind, order_id, order_json, seq: r.actor_seq })
}

pub(crate) fn decode(r: &Record) -> Option<Event> {
    decode_or_reason(r).ok()
}
