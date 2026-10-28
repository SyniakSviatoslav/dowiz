//! SHEAF S0: THE GLUING CHECK for one pair of contexts, `hub ∩ kitchen`, and one object kind,
//! the stock log (R-BEBOPDB Part G, row S0; operator 2026-10-05 "bebop має стати sheaf based
//! engine"). Ported from the X5 prototype (`bebopdb-bench` `glue`, 590 us at 2000+20+20), onto
//! the real `StockEvent` fold instead of the prototype's two-verb JSON.
//!
//! THE MODEL, on what exists. A context's stalk is its stock-log image; a kitchen branch is the
//! hub's image at some tip with the kitchen's records appended. The RESTRICTION to the overlap
//! is "cut the chain at the common ancestor", and the overlap's DIGEST is the ancestor's id
//! itself: a chained id commits to every record before it (`chain.rs`), so two chains that
//! hold the same id agree on the whole prefix -- one 32-byte compare, no Merkle tree. The
//! digest is handed back in the cover manifest's convention (`cover::Kind::Log`: the tip as
//! four little-endian cells), so S2 can write it as the context's `last_glued_tip` as is.
//!
//! THE GLUING CONDITION. Over the shared prefix the two sections agree by construction; what
//! can fail is the glued TAILS: `fold(ancestor) + dHub + dKitchen` may break an invariant that
//! neither branch broke alone (the last salmon sold on two tablets). That obstruction is what
//! this returns, by name and per item (`delta::Conflict`). GLUING DETECTS, IT DOES NOT RESOLVE
//! (memory `bebop-sheaf-engine-decision`): nothing here writes, merges or picks a winner. The
//! resolvers are S1 (escrow rights), the hub's rebase and a person.
//!
//! WHAT IS CHECKED BEFORE ONE RECORD IS BELIEVED, and refused by name (`Why`): the hub's image
//! loads as a stock log with nothing quarantined (`StockLog::load`, every record's crc); each
//! root's crc holds and its tip is the newest record's id; every KITCHEN-OR-HUB TAIL record's
//! crc holds, its id is `sha256(prev ‖ payload)`, and its `prev` is the record below it, down
//! to the ancestor; the cover manifest's `last_glued` tip, when given, is in both chains.
//!
//! THE KITCHEN IMAGE IS NOT LOADED WHOLE. Nothing below its ancestor is used: that prefix is
//! the hub's, which was loaded and checked, and the kitchen's copy is compared by one id. Its
//! tail is checked link by link above. Its ledger is DERIVED -- `ancestor + dKitchen`, the
//! ancestor being `hub - dHub` -- except when the hub's tail measured a shared item the hub
//! counts (`delta::needs_kitchen`); then the kitchen is loaded and folded like the hub.
//!
//! COST. The ancestor is found by walking both chains in step from the same chain position
//! (the root's count), reading four id cells per record over the bytes (`walk.rs`, no copy):
//! O(dHub + dKitchen), never the whole log. The hub's ledger is `StockLog::ledger` (newest
//! checkpoint + its tail). Measured in `tests.rs` `measure_glue_2000_20_20`.
//!
//! NOT IN S0: a kitchen image that holds ONLY its tail (G.1's "fresh image whose first record
//! has prev = tip_hub", S2's wire form); a v1 root (`Why::OldFormat`); any write path.

use crate::stock::rights::{provisional_of, PROV_KEY};
use crate::stock::{decode, StockLevel, StockLog};

mod delta;
pub mod walk;
/// S2: a tablet's tail as it travels, glued against the hub's log.
pub mod wire;
/// S2: the hub adopting a glued tail, and the reply the tablet replays.
pub mod merge;
/// S2: the tablet's side -- record offline, send the tail, replay the reply.
pub mod tablet;
/// S2: the decisions a glue left for a person, and their resolution.
pub mod desk;
/// S2b: the kitchen's order rail offline -- the tablet's taps, rebased through the order machine.
pub mod orders;
pub use delta::{Conflict, Side};
use walk::{Chain, OpenErr};

/// Why a pair of images was not glued. Every refusal names the branch it found the fault in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// The image is not a stock log this reads (`StockLog::load` refused, or no root).
    Unreadable(&'static str),
    /// A v1 log: S0 walks v2 only (every log that has grown once is v2).
    OldFormat,
    /// The load quarantined records for a failed crc: a section with holes is not glued.
    Quarantined(usize),
    /// The root's tip is not the newest record's id.
    TipMismatch,
    /// A tail record (0 = newest) whose crc fails.
    BadCrc { pos: usize },
    /// A tail record whose id is not `sha256(prev ‖ payload)`, or whose `prev` is not the
    /// record below it.
    BrokenLink { pos: usize },
    /// The cover manifest's last glued tip is not in this chain.
    LastGluedNotInLog,
    /// The two chains share no record.
    NoCommonAncestor,
    /// The ledger at the tip refused its own history.
    Ledger(String),
    /// S2, a wire tail (`wire.rs`; `pos` counts from its OLDEST record): a record that is not a
    /// stock movement a tablet may write (a note, a checkpoint, a grant, unreadable JSON).
    NotAMovement { pos: usize },
    /// S2 NO-TRACKING: a tail record carrying a key no stock record has (`wire::KEYS`).
    Tracking { pos: usize, key: String },
    /// S2: a tail record longer than `wire::REC_MAX`, or a tail longer than `wire::TAIL_MAX`.
    TooLong { pos: usize },
}

/// A refusal: the branch and the fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub side: Side,
    pub why: Why,
}

/// How the two sections relate once glued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The same tip: nothing to glue.
    Same,
    /// Only `ahead` wrote since the ancestor: the glued section is that branch's.
    FastForward { ahead: Side },
    /// Both wrote; the tails commute and every invariant holds.
    Commutes,
    /// Both wrote, and the glued section breaks an invariant (`Glued::conflicts`).
    Obstructed,
}

/// The gluing check's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glued {
    /// The overlap's digest: the common ancestor's id as the cover manifest's four cells
    /// (all zeros when both chains start empty).
    pub ancestor: [i64; 4],
    /// Records after the ancestor on each branch.
    pub hub_tail: usize,
    pub kitchen_tail: usize,
    /// What the kitchen's tail costs on the wire (object cells x 8).
    pub wire_bytes: usize,
    pub conflicts: Vec<Conflict>,
    /// The glued level of every item both tails touched (not those a conflict refused to
    /// fold), sorted by item.
    pub shared: Vec<(String, StockLevel)>,
}

impl Glued {
    pub fn state(&self) -> State {
        match (self.hub_tail, self.kitchen_tail, self.conflicts.is_empty()) {
            (0, 0, _) => State::Same,
            (_, 0, _) => State::FastForward { ahead: Side::Hub },
            (0, _, _) => State::FastForward { ahead: Side::Kitchen },
            (_, _, true) => State::Commutes,
            (_, _, false) => State::Obstructed,
        }
    }
}

fn refuse<T>(side: Side, why: Why) -> Result<T, Refusal> {
    Err(Refusal { side, why })
}

/// THE GLUING CHECK. `hub` and `kitchen` are the two stock-log images; `last_glued` is the
/// cover manifest's tip for this pair, when there is one.
pub fn glue(hub: &[u8], kitchen: &[u8], last_glued: Option<&[i64; 4]>) -> Result<Glued, Refusal> {
    let lh = load(Side::Hub, hub)?;
    let ch = open(Side::Hub, hub)?;
    let ck = open(Side::Kitchen, kitchen)?;
    let (anc, th, tk) = ancestor(&ch, &ck)?;
    if let Some(cells) = last_glued {
        let want = walk::from_cells(cells);
        if want != anc && want != [0u8; 32] {
            // Not the ancestor: each chain must still hold it, above or below.
            for (side, c) in [(Side::Hub, &ch), (Side::Kitchen, &ck)] {
                if !c.objs().any(|o| c.id(o) == want) {
                    return refuse(side, Why::LastGluedNotInLog);
                }
            }
        }
    }
    let mut provisional = Vec::new();
    let eh = tail_events(Side::Hub, &ch, &th, &anc, &mut provisional)?;
    let ek = tail_events(Side::Kitchen, &ck, &tk, &anc, &mut provisional)?;
    let wire_bytes = tk.iter().map(|o| ck.wire_cells(*o) * 8).sum();
    let mut g = Glued {
        ancestor: walk::to_cells(&anc),
        hub_tail: th.len(),
        kitchen_tail: tk.len(),
        wire_bytes,
        conflicts: provisional,
        shared: Vec::new(),
    };
    if th.is_empty() || tk.is_empty() {
        return Ok(g);
    }
    let led_h = ledger(Side::Hub, &lh)?;
    let (dh, dk) = (delta::TailDelta::of(&eh), delta::TailDelta::of(&ek));
    let led_k = if delta::needs_kitchen(&led_h, &dh, &dk) { Some(ledger(Side::Kitchen, &load(Side::Kitchen, kitchen)?)?) } else { None };
    let (conflicts, shared) = delta::obstructions(&led_h, led_k.as_ref(), &dh, &dk);
    g.conflicts.extend(conflicts);
    g.shared = shared;
    Ok(g)
}

fn load(side: Side, bytes: &[u8]) -> Result<StockLog, Refusal> {
    let log = StockLog::load(bytes).map_err(|_| Refusal { side, why: Why::Unreadable("not a stock log this loader reads") })?;
    if log.quarantined() > 0 {
        return refuse(side, Why::Quarantined(log.quarantined()));
    }
    Ok(log)
}

fn ledger(side: Side, log: &StockLog) -> Result<crate::stock::StockLedger, Refusal> {
    log.ledger().map_err(|e| Refusal { side, why: Why::Ledger(e.to_string()) })
}

fn open(side: Side, bytes: &[u8]) -> Result<Chain<'_>, Refusal> {
    let c = Chain::open(bytes).map_err(|e| Refusal {
        side,
        why: match e {
            OpenErr::NoRoot => Why::Unreadable("no log root"),
            OpenErr::RootCrc => Why::Unreadable("the log root's crc fails"),
            OpenErr::OldFormat => Why::OldFormat,
        },
    })?;
    let newest = c.objs().next().map(|o| c.id(o)).unwrap_or([0u8; 32]);
    if newest != c.tip {
        return refuse(side, Why::TipMismatch);
    }
    Ok(c)
}

/// The newest record both chains hold, and each chain's records above it (newest first).
/// Both chains share their prefix position for position, so the walk starts the longer one
/// `|nH - nK|` records down and then steps both together.
fn ancestor(h: &Chain<'_>, k: &Chain<'_>) -> Result<([u8; 32], Vec<usize>, Vec<usize>), Refusal> {
    let (mut ih, mut ik) = (h.objs(), k.objs());
    let (mut th, mut tk) = (Vec::new(), Vec::new());
    for _ in k.n..h.n {
        th.extend(ih.next());
    }
    for _ in h.n..k.n {
        tk.extend(ik.next());
    }
    loop {
        match (ih.next(), ik.next()) {
            (Some(a), Some(b)) => {
                let id = h.id(a);
                if id == k.id(b) {
                    return Ok((id, th, tk));
                }
                th.push(a);
                tk.push(b);
            }
            (None, None) if th.len() + tk.len() == h.n.abs_diff(k.n) => return Ok(([0u8; 32], th, tk)),
            _ => return refuse(Side::Kitchen, Why::NoCommonAncestor),
        }
    }
}

/// A tail's records, checked link by link down to the ancestor, decoded OLDEST FIRST.
/// Records that are not stock events (checkpoints, notes, station bindings) move nothing and
/// are skipped, as every fold skips them. A sale flagged provisional (S1) is pushed to `prov`.
fn tail_events(
    side: Side,
    c: &Chain<'_>,
    tail: &[usize],
    anc: &[u8; 32],
    prov: &mut Vec<Conflict>,
) -> Result<Vec<crate::stock::StockEvent>, Refusal> {
    let mut out = Vec::with_capacity(tail.len());
    for (pos, &o) in tail.iter().enumerate() {
        if !c.crc_ok(o) {
            return refuse(side, Why::BadCrc { pos });
        }
        let below = tail.get(pos + 1).map(|b| c.id(*b)).unwrap_or(*anc);
        let payload = c.payload(o);
        let prev = c.prev(o);
        if prev != below || crate::content_id_chained(&prev, &payload) != c.id(o) {
            return refuse(side, Why::BrokenLink { pos });
        }
        let rec = String::from_utf8_lossy(&payload);
        // Only a record carrying the `"prov"` key can be provisional; the substring test keeps
        // the full meta parse off every other record, which measured +170 us at 2000+20+20
        // when it ran on all forty.
        if rec.contains(PROV_KEY) {
            if let Some(p) = provisional_of(&rec) {
                prov.push(Conflict::Provisional { side, ctx: p.ctx, item: p.item, order_id: p.order_id, qty: p.qty });
            }
        }
        if let Some(ev) = decode(&rec) {
            out.push(ev);
        }
    }
    out.reverse();
    Ok(out)
}

#[cfg(test)]
mod tests;
