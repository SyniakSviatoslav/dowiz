//! THE TWO SIDES OF A MERGE (W-MG-MERGE2): which records a MERGE's rows put on which side, in what
//! order they fold, and the conflicts between them. One reading for the writer and for `recheck`:
//! the same function over the same rows, so the answer may depend on the rows and on nothing else.
//!
//! ONE ORDER FOR EVERY COPY. The order across devices is signed by nobody, and two honest copies of
//! one history hold it in two orders. A count is not additive, so a side folded in file order gave
//! two levels: A into B named a conflict B into A did not, and a converged file failed its audit
//! (`atk5_recheck`). `canonical` is a function of signed bytes alone: a record's RANK is one past
//! what it is known to follow (its device's record before it; a device's first record, the `prev`
//! it chained on; a MERGE, every row it names), then `(rank, actor, seq)`. Each device's own order
//! stands. Plain `(actor, seq)` is not enough: a device sorted ahead of the delivery it sold from
//! would fold a sale before the shelf existed, and the first delivery clips.
//!
//! THE LIMIT. An ADOPTION signs nothing: a device that took a tail in with no MERGE and then wrote
//! has no signed edge to that tail, so its record may rank before a count it saw. Every copy then
//! folds that same wrong order: a level off by what the count reset, in every file alike.
//!
//! THE FOLD IS A HISTORY'S (`StockLedger::history`), not a write door's: a side holding both halves
//! of a named conflict is what a merge is for.

use std::collections::{BTreeMap, BTreeSet};

use einstore::evlog::doc::keys::read_keys;
use einstore::evlog::doc::{parse_merge, Doc, Ids, Row};
use einstore::evlog::Record;

use super::stock::{stock_event, KIND_STOCK};
use super::{hex32, HubIds, SealedHub};
use crate::glue::{obstructed, Conflict};
use crate::stock::{StockLedger, StockLevel};

type Actor = [u8; 32];

/// Every actor with a KEY row. An actor neither list names is the shared past only if it has none.
pub(super) fn keyed(sh: &SealedHub) -> BTreeSet<Actor> {
    let view = Doc::open(&sh.store).ok();
    read_keys(&sh.store, view.and_then(|v| v.keys)).iter().map(|k| k.actor_id).collect()
}

/// `recs` oldest first, in the one order every copy agrees on (header).
pub(super) fn canonical(recs: Vec<Record>) -> Vec<Record> {
    let (mut by_id, mut at, mut last) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    let mut ranked: Vec<(u64, Record)> = Vec::with_capacity(recs.len());
    for r in recs {
        let mut rank: u64 = last.get(&r.actor_pubkey).or_else(|| by_id.get(&r.prev)).copied().unwrap_or(0);
        if let Some(v) = parse_merge(HubIds.body(r.flags, &r.payload)) {
            for row in v.ours.iter().chain(&v.theirs) {
                rank = rank.max(at.get(&(row.actor_id, row.seq)).copied().unwrap_or(0));
            }
        }
        rank += 1;
        by_id.insert(r.id, rank);
        at.insert((r.actor_pubkey, r.actor_seq), rank);
        last.insert(r.actor_pubkey, rank);
        ranked.push((rank, r));
    }
    ranked.sort_by(|(x, a), (y, b)| (x, a.actor_pubkey, a.actor_seq).cmp(&(y, b.actor_pubkey, b.actor_seq)));
    ranked.into_iter().map(|(_, r)| r).collect()
}

/// A MERGE's rows, indexed: a body may repeat one row a million times.
struct Sides<'a> {
    ours: BTreeMap<Actor, u64>,
    theirs: BTreeMap<Actor, u64>,
    keyed: &'a BTreeSet<Actor>,
}

fn index(rows: &[Row]) -> BTreeMap<Actor, u64> {
    let mut m = BTreeMap::new();
    for r in rows {
        m.entry(r.actor_id).or_insert(r.seq);
    }
    m
}

impl<'a> Sides<'a> {
    fn of(keyed: &'a BTreeSet<Actor>, ours: &[Row], theirs: &[Row]) -> Self {
        Sides { ours: index(ours), theirs: index(theirs), keyed }
    }

    /// On the side whose rows are `mine`? An actor neither list names and that has no key wrote
    /// before the seal: the shared past. A keyed actor neither list names wrote AFTER this MERGE:
    /// it is on no side, or a later delivery would move the answer to an older question.
    fn on(&self, r: &Record, mine: &BTreeMap<Actor, u64>, other: &BTreeMap<Actor, u64>) -> bool {
        match mine.get(&r.actor_pubkey) {
            Some(seq) => r.actor_seq <= *seq,
            None => !other.contains_key(&r.actor_pubkey) && !self.keyed.contains(&r.actor_pubkey),
        }
    }

    /// `(on ours, on theirs)`.
    fn place(&self, r: &Record) -> (bool, bool) {
        (self.on(r, &self.ours, &self.theirs), self.on(r, &self.theirs, &self.ours))
    }
}

/// Does each list hold a record the other lacks? Every honest MERGE's does: one is written only
/// when both sides wrote (`both_wrote`). This is all a file can ask of the rows (`merge.rs`).
pub(super) fn two_tails(recs: &[Record], keyed: &BTreeSet<Actor>, ours: &[Row], theirs: &[Row]) -> bool {
    let s = Sides::of(keyed, ours, theirs);
    let (mut o, mut t) = (false, false);
    for r in recs {
        let (a, b) = s.place(r);
        o |= a && !b;
        t |= b && !a;
    }
    o && t
}

pub(super) fn is_stock(r: &Record) -> bool {
    crate::body_of(r).first() == Some(&KIND_STOCK)
}

/// A movement nobody here can read is still a movement: a person looks (`Diverged`'s words).
fn unreadable(r: &Record) -> Conflict {
    let item = format!("нечитаний запис складу {} №{}", &hex32(&r.actor_pubkey)[..16], r.actor_seq);
    Conflict::Diverged { item, from_hub: StockLevel::default(), from_kitchen: StockLevel::default() }
}

/// The obstructions between the two sides `ours` and `theirs` name, over `recs` (`canonical`). `Err`:
/// the quantities do not fit the arithmetic, the only thing that can refuse.
pub(super) fn conflicts_at(recs: &[Record], keyed: &BTreeSet<Actor>, ours: &[Row], theirs: &[Row]) -> Result<Vec<Conflict>, String> {
    let s = Sides::of(keyed, ours, theirs);
    let (mut side_o, mut side_t, mut tail_o, mut tail_t, mut out) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for r in recs {
        let (o, t) = s.place(r);
        let Some(ev) = stock_event(r) else {
            if is_stock(r) && o != t {
                out.push(unreadable(r));
            }
            continue;
        };
        if o {
            side_o.push(ev.clone());
        }
        if t {
            side_t.push(ev.clone());
        }
        if o && !t {
            tail_o.push(ev);
        } else if t && !o {
            tail_t.push(ev);
        }
    }
    if tail_o.is_empty() || tail_t.is_empty() {
        return Ok(out);
    }
    let fold = |e: &[crate::stock::StockEvent]| StockLedger::history(e).map_err(|e| e.to_string());
    let (led_o, led_t) = (fold(&side_o)?, fold(&side_t)?);
    // `glue::delta` adds levels and deltas with plain `+`: bound the sum before it runs, in any build.
    let moved: i128 = tail_o.iter().chain(&tail_t).map(|e| e.moved()).sum();
    if 2 * (led_o.peak().max(led_t.peak()) + moved) > i64::MAX as i128 {
        return Err("a level of the two sides would pass i64".to_string());
    }
    out.extend(obstructed(&led_o, &led_t, &tail_o, &tail_t));
    Ok(out)
}
