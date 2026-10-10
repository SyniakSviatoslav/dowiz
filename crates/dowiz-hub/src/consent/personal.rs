//! PERSONALISATION: the guest's taste, scored by the venue (W-MR0 row MR8; DECISIONS.md D0 amendment
//! 2026-10-04). OPERATOR RULING of the same day: ON BY DEFAULT under legitimate interest (GDPR 6(1)(f)),
//! not consent. So this purpose's log holds OBJECTIONS (a `Withdrawn` act, `Method::Objection`,
//! Art. 21) and, should a guest turn it back on, a `Given` act naming the sentence they read.
//! Never a reason to change a price. Kept out of `consent.rs` so that file stays under 300 lines.

use super::{Act, State, CHANNELS, KIND_ACT};
use crate::logimage::Entry;

pub const PURPOSE_PERSONALISATION: &str = "personalisation";
/// Personalisation is not a message channel: it happens on the venue's own storefront, and that is
/// the only channel it may be filed under (and the only purpose that may use it).
pub const CHANNEL_STOREFRONT: &str = "storefront";

/// The channels a purpose may be filed under: ONE list, read by `check` and by the erasure that
/// withdraws every one of them (a forget that walked only the message channels left a
/// personalisation grant standing, or wrote a withdrawal `check` refuses).
pub fn channels_of(purpose: &str) -> &'static [&'static str] {
    if purpose == PURPOSE_PERSONALISATION {
        &[CHANNEL_STOREFRONT]
    } else {
        &CHANNELS
    }
}

/// PERSONALISATION IS ON UNLESS THE GUEST OBJECTED: true when the newest valid personalisation act
/// for `key` is a withdrawal (an objection, or an erasure). No act at all is NOT an objection.
pub fn objected(entries: &[Entry], key: &str) -> bool {
    newest(entries, key, PURPOSE_PERSONALISATION, CHANNEL_STOREFRONT).is_some_and(|a| a.state == State::Withdrawn)
}

/// The newest valid act for one key, purpose and channel, by the act's own clock; a tie goes to
/// the withdrawal (`consent::state` folds this too).
///
/// THE SUBJECT IS TESTED BEFORE THE ACT IS PARSED (W-LOOPB, R-LOOPS row 4): the one writer
/// (`log::write`) files every act under `subject_of(act.key)`, so another subject's acts are
/// not parsed only to be dropped by the key test. An act filed under a subject that is not its
/// key's was not written by this code and is not counted (`tests::a_misfiled_act_is_not_counted`).
pub(super) fn newest(entries: &[Entry], key: &str, purpose: &str, channel: &str) -> Option<Act> {
    let mut best: Option<Act> = None;
    let subject = super::subject_of(key);
    for e in entries.iter().filter(|e| e.kind == KIND_ACT && e.subject == subject) {
        let Some(act) = Act::parse(&e.json) else { continue };
        if act.key != key || act.purpose != purpose || act.channel != channel || super::check(&act).is_err() {
            continue;
        }
        let takes = match &best {
            None => true,
            Some(b) => act.at_ms > b.at_ms || (act.at_ms == b.at_ms && act.state == State::Withdrawn),
        };
        if takes {
            best = Some(act);
        }
    }
    best
}
