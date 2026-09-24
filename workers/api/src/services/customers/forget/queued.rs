//! FORGETTING WHAT IS STILL WAITING TO BE SENT (G8, D15).
//!
//! The outbox holds messages RENDERED AT ENQUEUE TIME: a kitchen ticket
//! carries the name, the phone and the address; a campaign entry is addressed
//! `to` the person's number. An entry still waiting when the person is
//! forgotten would carry them out of the venue after the erasure, and the
//! drain re-checks only consent -- which a forgotten person no longer has on
//! the key the entry names, but an order ticket never asked.
//!
//! So the forget turn DROPS them. Nothing is rewritten: the live-order refusal
//! (`forget::LIVE`) means every order of this person has ended, and a ticket
//! about an ended order has no purpose left; a campaign message to somebody
//! who asked to be forgotten has none either.
//!
//! WHAT IS KEPT: every entry that is not about this person, and the campaign
//! marks (`send::MARK`): a mark holds only `camp:<campaign>:<key>`, the
//! pseudonym, and it is what stops a pressed "send" from queueing twice.
//! Fiscal documents are keyed by their own uuid and never match: a legal
//! obligation is not the person's to erase.

use std::collections::BTreeSet;

use dowiz_hub::table::Table;

use crate::outbox::{Entry, KIND};
use crate::services::campaigns::send::{parse_id, OUTBOX_KIND};

/// PURE. Whether a waiting entry is about this person: a notification whose
/// id is `<order>/...` for one of their orders, or a campaign entry filed
/// under one of their keys.
pub fn is_theirs(e: &Entry, orders: &BTreeSet<String>, keys: &BTreeSet<String>) -> bool {
    if e.kind == OUTBOX_KIND {
        return parse_id(&e.id).is_some_and(|(_, k)| keys.contains(k));
    }
    e.id.split_once('/').is_some_and(|(order, _)| orders.contains(order))
}

/// PURE. Drop every waiting entry that is theirs. Returns how many went.
pub fn drop_queued(t: &mut Table, orders: &BTreeSet<String>, keys: &BTreeSet<String>) -> usize {
    let ids: Vec<String> = t
        .all(KIND)
        .into_iter()
        .filter_map(|(id, j)| serde_json::from_str::<Entry>(&j).ok().filter(|e| is_theirs(e, orders, keys)).map(|_| id))
        .collect();
    ids.iter().filter(|id| t.remove(KIND, id)).count()
}

#[cfg(test)]
mod tests;
