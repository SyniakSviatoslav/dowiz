//! GRANT (§5, operator decision): roles as a record outside the core. `holds` says a file is whole,
//! not that a kitchen tablet's key may move money: that is this fold's. The core does not read this
//! record and `verify` does not count it; it is an ordinary body under the implementor's framing.
//!
//! ```text
//! GRANT  "ein-grant/1" ‖ actor_id 32 ‖ n LE8 ‖ (resource u8 ‖ action u8) × n
//! ```
//!
//! THE FOLD, and why the file's order plays no part (it is signed by nobody, `settle.rs`):
//!
//!   * Per `(author, target)` the author's newest GRANT by its own seq stands, so a narrower one
//!     takes back the rest and `n = 0` takes back all of it. A key is not revoked by this.
//!   * A GRANT is in force when its author is a root, or holds every pair in it: narrow only. It is
//!     the least fixpoint from nothing, so two keys granting each other start nothing.
//!   * A GRANT past its author's `revoked_at` is void (`Key::valid_at`). One below it stands. An
//!     unknown author grants nothing, and neither does a tombstone.
//!   * A GRANT over more than its author holds is not cut down: it gives nothing, fail closed. That is
//!     true of an author's newest one even when an older, smaller one was fine.
//!   * Anything that carries the mark and does not parse is counted in `bad`, never read as no GRANT.
//!
//! DENY BY DEFAULT: no record, no pair. The red-line table is from bebop2's `redline.rs`: it does not
//! decide anything by itself and only names what a gate must never let through on a default. Numbers
//! are that crate's pinned discriminants, so a scope written there means the same here. A number
//! this table does not know is carried, and it matches only itself.
//!
//! A TIME THIS CANNOT SEE. There is no order across devices, so a GRANT and a record it would
//! authorise are not compared: `allows` answers from the file as it stands. Call it on a file that
//! holds, and only then: nothing here checks a signature. No adoption gate calls this yet (P4).

#[allow(unused_imports)]
use alloc::{vec, vec::Vec};
use alloc::collections::{BTreeMap, BTreeSet};
use einfile_log::doc::keys::Key;
use einfile_log::doc::Ids;
use einfile_log::Record;

pub const GRANT_MARK: &[u8] = b"ein-grant/1";

type Actor = [u8; 32];
/// `(resource, action)`, by number.
pub type Pair = (u8, u8);

const R_LEDGER: u8 = 0x02;
const R_ORDER: u8 = 0x06;
const R_CLAIM: u8 = 0x0C;
const R_AUTH: u8 = 0x0F;
const R_SECRET: u8 = 0x10;
const R_MIGRATION: u8 = 0x11;
const A_APPEND: u8 = 0x03;
const A_CREATE_ORDER: u8 = 0x05;
const A_SETTLEMENT: u8 = 0x13;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedLine {
    Money,
    Auth,
    Secrets,
    Migrations,
}

/// What a pair is, when it is a red line.
pub fn is_red_line(p: Pair) -> Option<RedLine> {
    match p {
        (R_LEDGER, A_SETTLEMENT | A_APPEND) | (R_ORDER, A_CREATE_ORDER) | (R_CLAIM, _) => Some(RedLine::Money),
        (R_AUTH, _) => Some(RedLine::Auth),
        (R_SECRET, _) => Some(RedLine::Secrets),
        (R_MIGRATION, _) => Some(RedLine::Migrations),
        _ => None,
    }
}

pub fn grant_body(actor: &Actor, pairs: &[Pair]) -> Vec<u8> {
    let mut b = Vec::with_capacity(GRANT_MARK.len() + 40 + pairs.len() * 2);
    b.extend_from_slice(GRANT_MARK);
    b.extend_from_slice(actor);
    b.extend_from_slice(&(pairs.len() as u64).to_le_bytes());
    for (r, a) in pairs {
        b.push(*r);
        b.push(*a);
    }
    b
}

/// `None` when `body` is not exactly a GRANT.
pub fn parse_grant(body: &[u8]) -> Option<(Actor, Vec<Pair>)> {
    let r = body.strip_prefix(GRANT_MARK)?;
    let actor: Actor = r.get(..32)?.try_into().ok()?;
    let n = u64::from_le_bytes(r.get(32..40)?.try_into().ok()?);
    let list = r.get(40..)?;
    if list.len() % 2 != 0 || (list.len() / 2) as u64 != n {
        return None;
    }
    Some((actor, list.chunks_exact(2).filter_map(|p| Some((*p.first()?, *p.get(1)?))).collect()))
}

/// Who holds what, folded from a file's records.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Grants {
    roots: BTreeSet<Actor>,
    held: BTreeMap<Actor, BTreeSet<Pair>>,
    /// Bodies with the mark that are no GRANT.
    pub bad: usize,
}

impl Grants {
    /// `recs` and `keys` of a file that holds, under the roots the caller brings (header).
    pub fn fold(recs: &[Record], keys: &[Key], roots: &[Actor], ids: &dyn Ids) -> Grants {
        let mut bad = 0usize;
        let mut newest: BTreeMap<(Actor, Actor), (u64, BTreeSet<Pair>)> = BTreeMap::new();
        for r in recs {
            if ids.redacted(r.flags, &r.payload) {
                continue;
            }
            let body = ids.body(r.flags, &r.payload);
            if !body.starts_with(GRANT_MARK) {
                continue;
            }
            let Some((target, pairs)) = parse_grant(body) else {
                bad += 1;
                continue;
            };
            if !keys.iter().find(|k| k.actor_id == r.actor_pubkey).is_some_and(|k| k.valid_at(r.actor_seq)) {
                continue;
            }
            let e = newest.entry((r.actor_pubkey, target)).or_insert((0, BTreeSet::new()));
            if r.actor_seq >= e.0 {
                *e = (r.actor_seq, pairs.into_iter().collect());
            }
        }
        let roots: BTreeSet<Actor> = roots.iter().copied().collect();
        let mut held: BTreeMap<Actor, BTreeSet<Pair>> = BTreeMap::new();
        loop {
            let mut grew = false;
            for ((author, target), (_, pairs)) in &newest {
                let may = roots.contains(author) || held.get(author).is_some_and(|h| pairs.is_subset(h));
                if may {
                    let h = held.entry(*target).or_default();
                    let before = h.len();
                    h.extend(pairs.iter().copied());
                    grew |= h.len() != before;
                }
            }
            if !grew {
                return Grants { roots, held, bad };
            }
        }
    }

    /// May `actor` do `p`? A root may; anyone else only by a pair in force. Nothing else says yes.
    pub fn allows(&self, actor: &Actor, p: Pair) -> bool {
        self.roots.contains(actor) || self.held.get(actor).is_some_and(|h| h.contains(&p))
    }
}

#[cfg(test)]
mod tests;
