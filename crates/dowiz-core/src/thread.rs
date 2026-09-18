//! Message threads — an append-only log between a customer, a venue and a courier.
//!
//! The product's chat screens had nothing behind them: no thread, no message, no
//! ordering. This is that domain, and it is deliberately the smallest thing that
//! is actually correct.
//!
//! # What this is
//!
//! An append-only log per thread, ordered by a **sender-assigned sequence** and
//! broken ties by sender id — not by a clock. Two devices that were offline and
//! then sync must agree on the order of what they each wrote, and a wall clock
//! cannot give them that: phones disagree, and a message can arrive with a
//! timestamp before one it answers. So `seq` is per-sender and monotonic, and
//! the merge is deterministic (MANIFESTO C2: no clock in the decision path).
//!
//! A message carries a `sent_at_ms` for DISPLAY only. It never orders anything,
//! and [`merge`] would give the same answer if every timestamp were zero — which
//! is what [`tests::order_ignores_the_clock`] asserts.
//!
//! # What this is NOT
//!
//! * **No presence.** "Online" is a fact about a transport, not about a thread,
//!   and a thread that stores it would be wrong the moment a phone loses signal.
//! * **No delivery receipts beyond what a reader states.** A read mark is a
//!   message the reader wrote, replayed like any other.
//! * **No ranking of participants.** `DECISIONS.md` D0.
//!
//! Sending is out of scope by construction: this module produces and merges a
//! log, and a transport carries it. That is the same split [`crate::messenger`]
//! already makes — it builds deep links and never sends either.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Who wrote a message. A role, not a person: a thread is between the parties to
/// an order, and the courier on it may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Party {
    Customer,
    Venue,
    Courier,
    /// The system itself — "your order is on the way". Never a person.
    System,
}

impl Party {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Customer => "CUSTOMER",
            Self::Venue => "VENUE",
            Self::Courier => "COURIER",
            Self::System => "SYSTEM",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "CUSTOMER" => Some(Self::Customer),
            "VENUE" => Some(Self::Venue),
            "COURIER" => Some(Self::Courier),
            "SYSTEM" => Some(Self::System),
            _ => None,
        }
    }

    /// A stable rank used ONLY to break a tie between two messages that carry the
    /// same sequence number from different parties. It is not a priority and
    /// nothing else may read it.
    fn tiebreak(&self) -> u8 {
        match self {
            Self::System => 0,
            Self::Venue => 1,
            Self::Courier => 2,
            Self::Customer => 3,
        }
    }
}

/// What a message says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// Text the sender typed. Carried verbatim; never interpreted.
    Text(String),
    /// A marker that everything up to `through_seq` from every party has been
    /// read by `by`. A read mark IS a message, so it replays like one.
    Read { through_seq: u64 },
}

/// The largest text a single message may carry.
///
/// A bound rather than none: an unbounded message is an unbounded write to
/// whatever stores it, and the append log is replayed on a phone.
pub const MAX_TEXT_BYTES: usize = 4096;

/// One entry in the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Unique within the thread. The idempotency key: a resend is a no-op.
    pub id: u64,
    pub from: Party,
    /// Per-sender, strictly increasing. Two messages from one party never share
    /// a sequence; two parties may.
    pub seq: u64,
    /// For display only. Never orders anything.
    pub sent_at_ms: i64,
    pub body: Body,
}

/// A thread is its messages, in merge order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Thread {
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadError {
    /// The text is empty or longer than [`MAX_TEXT_BYTES`].
    BadText(usize),
    /// A second, DIFFERENT message arrived under an id already in the log.
    IdConflict(u64),
    /// A sender reused a sequence number for a different message.
    SeqConflict { from: Party, seq: u64 },
    /// A read mark points past anything that exists.
    ReadAhead { through_seq: u64, highest: u64 },
}

impl ThreadError {
    pub fn message(&self) -> String {
        match self {
            Self::BadText(n) => format!("message text of {n} bytes is outside 1..={MAX_TEXT_BYTES}"),
            Self::IdConflict(id) => format!("message id {id} already used by a different message"),
            Self::SeqConflict { from, seq } => {
                format!("{} reused sequence {seq}", from.as_str())
            }
            Self::ReadAhead {
                through_seq,
                highest,
            } => format!("read mark through {through_seq} is past the highest sequence {highest}"),
        }
    }
}

/// Check a message on its own, before it meets a thread.
pub fn validate(msg: &Message) -> Result<(), ThreadError> {
    if let Body::Text(t) = &msg.body {
        let n = t.len();
        if n == 0 || n > MAX_TEXT_BYTES {
            return Err(ThreadError::BadText(n));
        }
    }
    Ok(())
}

/// Append one message.
///
/// **Idempotent by id.** Appending a message whose id is already present is a
/// no-op when it is byte-for-byte the same message, and an error when it is not
/// — a resend must be free, and a forgery under a used id must not be.
pub fn append(mut thread: Thread, msg: Message) -> Result<Thread, ThreadError> {
    validate(&msg)?;

    if let Some(existing) = thread.messages.iter().find(|m| m.id == msg.id) {
        return if *existing == msg {
            Ok(thread) // the same message twice: a resend, not a second message
        } else {
            Err(ThreadError::IdConflict(msg.id))
        };
    }

    if thread
        .messages
        .iter()
        .any(|m| m.from == msg.from && m.seq == msg.seq)
    {
        return Err(ThreadError::SeqConflict {
            from: msg.from,
            seq: msg.seq,
        });
    }

    if let Body::Read { through_seq } = msg.body {
        let highest = thread.messages.iter().map(|m| m.seq).max().unwrap_or(0);
        if through_seq > highest {
            return Err(ThreadError::ReadAhead {
                through_seq,
                highest,
            });
        }
    }

    thread.messages.push(msg);
    order(&mut thread.messages);
    Ok(thread)
}

/// Merge two views of the same thread — two phones that were both offline.
///
/// Deterministic and commutative: `merge(a, b) == merge(b, a)`. A conflicting id
/// stops the merge rather than silently keeping one side.
pub fn merge(a: &Thread, b: &Thread) -> Result<Thread, ThreadError> {
    let mut out = a.clone();
    for m in &b.messages {
        out = append(out, m.clone())?;
    }
    Ok(out)
}

/// The total order: by sequence, then by the sender's stable tiebreak, then by
/// id. Every component is data already in the message, so two nodes sort the
/// same log identically without talking to each other.
fn order(messages: &mut [Message]) {
    messages.sort_by(|x, y| {
        x.seq
            .cmp(&y.seq)
            .then(x.from.tiebreak().cmp(&y.from.tiebreak()))
            .then(x.id.cmp(&y.id))
    });
}

/// The next sequence a party should use.
pub fn next_seq(thread: &Thread, from: Party) -> u64 {
    thread
        .messages
        .iter()
        .filter(|m| m.from == from)
        .map(|m| m.seq)
        .max()
        .map_or(1, |s| s + 1)
}

/// How much `who` has not read: every text message from somebody else with a
/// sequence above the highest they have marked read.
pub fn unread_for(thread: &Thread, who: Party) -> usize {
    let read_through = thread
        .messages
        .iter()
        .filter(|m| m.from == who)
        .filter_map(|m| match m.body {
            Body::Read { through_seq } => Some(through_seq),
            _ => None,
        })
        .max()
        .unwrap_or(0);

    thread
        .messages
        .iter()
        .filter(|m| m.from != who)
        .filter(|m| matches!(m.body, Body::Text(_)))
        .filter(|m| m.seq > read_through)
        .count()
}

/// The last text in the thread — what a list of conversations shows.
pub fn last_text(thread: &Thread) -> Option<&Message> {
    thread
        .messages
        .iter()
        .rev()
        .find(|m| matches!(m.body, Body::Text(_)))
}

/// Build a text message from a party, taking the next sequence itself so a
/// caller cannot accidentally reuse one.
pub fn compose(
    thread: &Thread,
    from: Party,
    id: u64,
    text: &str,
    sent_at_ms: i64,
) -> Result<Message, ThreadError> {
    let msg = Message {
        id,
        from,
        seq: next_seq(thread, from),
        sent_at_ms,
        body: Body::Text(text.to_string()),
    };
    validate(&msg)?;
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(id: u64, from: Party, seq: u64, at: i64, t: &str) -> Message {
        Message {
            id,
            from,
            seq,
            sent_at_ms: at,
            body: Body::Text(t.to_string()),
        }
    }

    #[test]
    fn messages_append_and_order() {
        let mut t = Thread::default();
        t = append(t, text(2, Party::Venue, 2, 200, "on its way")).unwrap();
        t = append(t, text(1, Party::Customer, 1, 100, "where is it?")).unwrap();
        assert_eq!(
            t.messages.iter().map(|m| m.id).collect::<Vec<_>>(),
            alloc::vec![1, 2]
        );
    }

    #[test]
    fn order_ignores_the_clock() {
        // The same messages with every timestamp set to zero must sort the same
        // way. If they did not, two phones with skewed clocks would disagree
        // about what answered what.
        let with_clock = {
            let mut t = Thread::default();
            t = append(t, text(1, Party::Customer, 1, 9_999, "a")).unwrap();
            t = append(t, text(2, Party::Venue, 2, 1, "b")).unwrap();
            t
        };
        let without = {
            let mut t = Thread::default();
            t = append(t, text(1, Party::Customer, 1, 0, "a")).unwrap();
            t = append(t, text(2, Party::Venue, 2, 0, "b")).unwrap();
            t
        };
        assert_eq!(
            with_clock.messages.iter().map(|m| m.id).collect::<Vec<_>>(),
            without.messages.iter().map(|m| m.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_resend_is_a_no_op() {
        let m = text(1, Party::Customer, 1, 100, "hello");
        let t = append(Thread::default(), m.clone()).unwrap();
        let again = append(t.clone(), m).unwrap();
        assert_eq!(t, again);
        assert_eq!(again.messages.len(), 1);
    }

    #[test]
    fn a_different_message_under_a_used_id_is_refused() {
        let t = append(Thread::default(), text(1, Party::Customer, 1, 100, "hello")).unwrap();
        let forged = text(1, Party::Customer, 1, 100, "send money");
        assert_eq!(
            append(t, forged),
            Err(ThreadError::IdConflict(1))
        );
    }

    #[test]
    fn a_sender_cannot_reuse_a_sequence() {
        let t = append(Thread::default(), text(1, Party::Customer, 1, 100, "a")).unwrap();
        assert_eq!(
            append(t, text(2, Party::Customer, 1, 200, "b")),
            Err(ThreadError::SeqConflict {
                from: Party::Customer,
                seq: 1
            })
        );
    }

    #[test]
    fn two_parties_may_share_a_sequence() {
        let mut t = append(Thread::default(), text(1, Party::Customer, 1, 100, "a")).unwrap();
        t = append(t, text(2, Party::Venue, 1, 100, "b")).unwrap();
        assert_eq!(t.messages.len(), 2);
        // The tie is broken by the sender's stable rank: Venue before Customer.
        assert_eq!(t.messages[0].from, Party::Venue);
    }

    #[test]
    fn merge_is_commutative() {
        let a = {
            let mut t = Thread::default();
            t = append(t, text(1, Party::Customer, 1, 1, "a")).unwrap();
            append(t, text(3, Party::Customer, 2, 3, "c")).unwrap()
        };
        let b = append(Thread::default(), text(2, Party::Venue, 1, 2, "b")).unwrap();
        assert_eq!(merge(&a, &b).unwrap(), merge(&b, &a).unwrap());
    }

    #[test]
    fn merge_stops_on_a_conflict_rather_than_picking_a_side() {
        let a = append(Thread::default(), text(1, Party::Customer, 1, 1, "a")).unwrap();
        let b = append(Thread::default(), text(1, Party::Customer, 1, 1, "different")).unwrap();
        assert_eq!(merge(&a, &b), Err(ThreadError::IdConflict(1)));
    }

    #[test]
    fn empty_and_oversized_text_are_refused() {
        assert_eq!(
            append(Thread::default(), text(1, Party::Customer, 1, 0, "")),
            Err(ThreadError::BadText(0))
        );
        let huge = "x".repeat(MAX_TEXT_BYTES + 1);
        assert_eq!(
            append(Thread::default(), text(1, Party::Customer, 1, 0, &huge)),
            Err(ThreadError::BadText(MAX_TEXT_BYTES + 1))
        );
    }

    #[test]
    fn the_largest_allowed_text_is_accepted() {
        let big = "x".repeat(MAX_TEXT_BYTES);
        assert!(append(Thread::default(), text(1, Party::Customer, 1, 0, &big)).is_ok());
    }

    #[test]
    fn a_read_mark_cannot_point_at_the_future() {
        let t = append(Thread::default(), text(1, Party::Venue, 1, 1, "a")).unwrap();
        let ahead = Message {
            id: 2,
            from: Party::Customer,
            seq: 1,
            sent_at_ms: 2,
            body: Body::Read { through_seq: 99 },
        };
        assert_eq!(
            append(t, ahead),
            Err(ThreadError::ReadAhead {
                through_seq: 99,
                highest: 1
            })
        );
    }

    #[test]
    fn unread_counts_only_what_the_other_side_wrote() {
        let mut t = Thread::default();
        t = append(t, text(1, Party::Venue, 1, 1, "a")).unwrap();
        t = append(t, text(2, Party::Venue, 2, 2, "b")).unwrap();
        t = append(t, text(3, Party::Customer, 1, 3, "mine")).unwrap();
        assert_eq!(unread_for(&t, Party::Customer), 2);
        assert_eq!(unread_for(&t, Party::Venue), 1);

        t = append(
            t,
            Message {
                id: 4,
                from: Party::Customer,
                seq: 2,
                sent_at_ms: 4,
                body: Body::Read { through_seq: 2 },
            },
        )
        .unwrap();
        assert_eq!(unread_for(&t, Party::Customer), 0);
    }

    #[test]
    fn compose_takes_the_next_sequence_for_you() {
        let mut t = Thread::default();
        let a = compose(&t, Party::Customer, 1, "first", 1).unwrap();
        assert_eq!(a.seq, 1);
        t = append(t, a).unwrap();
        let b = compose(&t, Party::Customer, 2, "second", 2).unwrap();
        assert_eq!(b.seq, 2);
        // The other party starts its own run at 1.
        assert_eq!(next_seq(&t, Party::Venue), 1);
    }

    #[test]
    fn last_text_skips_read_marks() {
        let mut t = Thread::default();
        t = append(t, text(1, Party::Venue, 1, 1, "on its way")).unwrap();
        t = append(
            t,
            Message {
                id: 2,
                from: Party::Customer,
                seq: 1,
                sent_at_ms: 2,
                body: Body::Read { through_seq: 1 },
            },
        )
        .unwrap();
        assert_eq!(
            last_text(&t).map(|m| m.id),
            Some(1),
            "a read mark is not the last thing anybody said"
        );
    }

    #[test]
    fn party_wire_form_round_trips() {
        for p in [Party::Customer, Party::Venue, Party::Courier, Party::System] {
            assert_eq!(Party::from_str(p.as_str()), Some(p));
        }
        assert_eq!(Party::from_str("ADMIN"), None);
    }

    #[test]
    fn this_module_stores_no_presence_and_no_score() {
        // Presence belongs to a transport; a score belongs nowhere (D0). The
        // tokens are split so this gate cannot match itself.
        let src = include_str!("thread.rs");
        for token in [
            concat!("is_", "online"),
            concat!("last_", "seen"),
            concat!("typing"),
            concat!("sender_", "score"),
            concat!("reputation"),
        ] {
            let hits = src.matches(token).count();
            assert!(
                hits <= 1,
                "{token} appears {hits} times — it does not belong in a thread"
            );
        }
    }
}
