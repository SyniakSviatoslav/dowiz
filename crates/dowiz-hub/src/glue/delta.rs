//! WHAT A TAIL DID, per item and per hold, and the obstruction that two tails make together.
//!
//! A tail's effect is folded as DELTAS, mirroring `StockLedger::apply` line by line (on_hand,
//! reserved, the open holds, served-not-reversed). A delta is additive -- `base + dA + dB` does
//! not depend on the order -- for every event except the two that set a level absolutely
//! (`Stocktake`, `Removed`) and the first measurement of an uncounted shelf (`apply` clips a
//! negative uncounted level to 0 there). Those are the non-commuting cases; they are named,
//! not folded (`Conflict::Recount`, `Conflict::Diverged`).
//!
//! Only an item BOTH tails touched can be obstructed: anywhere else the glued section equals
//! one branch's own, which that branch's write door already decided.

use std::collections::BTreeMap;

use crate::stock::{moved_into, Qty, StockEvent, StockLedger, StockLevel};

/// One tail's effect on one item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ItemDelta {
    pub on_hand: Qty,
    pub reserved: Qty,
    /// A count or a deletion set the level outright: not a delta.
    pub absolute: bool,
    /// A delivery, a weighed batch or a prep output: the first of these on an uncounted shelf
    /// MEASURES it (`apply` clips its level to 0 first), so the tail alone cannot say whether
    /// the ancestor was counted.
    pub arms: bool,
}

/// One tail's effect, keyed for lookup in the other's.
#[derive(Debug, Clone, Default)]
pub(crate) struct TailDelta {
    pub items: BTreeMap<String, ItemDelta>,
    /// `(order, item)` -> change to the open hold.
    pub held: BTreeMap<(String, String), Qty>,
    /// `(order, item)` -> change to served-and-not-reversed.
    pub served: BTreeMap<(String, String), Qty>,
}

impl TailDelta {
    /// Fold a tail, OLDEST FIRST.
    pub(crate) fn of(events: &[StockEvent]) -> TailDelta {
        let mut t = TailDelta::default();
        for ev in events {
            t.step(ev);
        }
        t
    }

    fn item(&mut self, item: &str) -> &mut ItemDelta {
        self.items.entry(item.to_string()).or_default()
    }

    fn hold(&mut self, order: &str, item: &str, q: Qty) {
        *self.held.entry((order.to_string(), item.to_string())).or_insert(0) += q;
    }

    fn serve(&mut self, order: &str, item: &str, q: Qty) {
        *self.served.entry((order.to_string(), item.to_string())).or_insert(0) += q;
    }

    fn step(&mut self, ev: &StockEvent) {
        match ev {
            StockEvent::Received { item, qty } | StockEvent::Made { item, qty, .. } => {
                let d = self.item(item);
                d.on_hand += qty;
                d.arms = true;
            }
            StockEvent::Returned { item, qty, resell: true, .. } => self.item(item).on_hand += qty,
            StockEvent::Returned { item, .. } => {
                self.item(item);
            }
            StockEvent::Reserved { item, qty, order_id } => {
                self.item(item).reserved += qty;
                self.hold(order_id, item, *qty);
            }
            StockEvent::Consumed { item, qty, order_id } => {
                let d = self.item(item);
                d.on_hand -= qty;
                d.reserved -= qty;
                self.hold(order_id, item, -qty);
            }
            StockEvent::Released { item, qty, order_id } => {
                self.item(item).reserved -= qty;
                self.hold(order_id, item, -qty);
            }
            StockEvent::Wasted { item, qty, .. } | StockEvent::Cooked { item, qty, .. } => self.item(item).on_hand -= qty,
            StockEvent::Served { item, qty, order_id } => {
                self.item(item).on_hand -= qty;
                self.serve(order_id, item, *qty);
            }
            StockEvent::Unserved { item, qty, order_id } => {
                self.item(item).on_hand += qty;
                self.serve(order_id, item, -qty);
            }
            StockEvent::Produced { item, qty, out, into, .. } => match moved_into(item, into) {
                Some(to) => {
                    let to = to.to_string();
                    self.item(item).on_hand -= qty;
                    let d = self.item(&to);
                    d.on_hand += out;
                    d.arms = true;
                }
                None => {
                    self.item(item);
                }
            },
            StockEvent::Stocktake { item, .. } | StockEvent::Removed { item, .. } => self.item(item).absolute = true,
        }
    }
}

/// Which branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Hub,
    Kitchen,
}

/// An obstruction: an invariant the glued section breaks that neither branch broke alone.
/// DETECTED, NEVER RESOLVED: each one names what a resolver (escrow rights, the hub's rebase,
/// a person) has to decide (`docs/research/2026-10-05-bebop-db-gaps-strengths-tensor-graph.md`
/// G.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conflict {
    /// The last salmon sold twice: a counted shelf the glued section takes below zero
    /// (on hand, or available = on hand - reserved), while each branch alone kept it.
    Oversold { item: String, merged: StockLevel, hub: StockLevel, kitchen: StockLevel },
    /// A count (or a deletion) on one branch and a movement of the same item on the other:
    /// the order decides the level, so a person recounts. Also a first delivery on a shelf the
    /// other branch drew below zero (the fold clips that level to 0).
    Recount { item: String, absolute_in: Side },
    /// One reservation taken twice: released on one branch and consumed on the other, or
    /// consumed on both (`what` = "held"); or one sale voided on both (`what` = "served").
    DoubleTake { order_id: String, item: String, what: &'static str, merged: Qty },
    /// The two branches' restrictions to the common ancestor disagree for this item: the
    /// level each branch implies for the ancestor is not the same. Cannot happen to deltas
    /// over one shared prefix; it is a fold or an image that is not what it claims.
    Diverged { item: String, from_hub: StockLevel, from_kitchen: StockLevel },
    /// SHEAF S1: a sale its context recorded PAST ITS ESCROW RIGHTS (`stock::rights`), flagged at
    /// the write door and never refused (§H.3 default). Reported even on a fast-forward: after
    /// the glue a person keeps it, amends the order or refunds it.
    Provisional { side: Side, ctx: String, item: String, order_id: String, qty: Qty },
}

fn minus(l: StockLevel, d: &ItemDelta) -> StockLevel {
    StockLevel { on_hand: l.on_hand - d.on_hand, reserved: l.reserved - d.reserved }
}

fn plus(l: StockLevel, d: &ItemDelta) -> StockLevel {
    StockLevel { on_hand: l.on_hand + d.on_hand, reserved: l.reserved + d.reserved }
}

/// The floor a counted shelf keeps.
fn broken(l: StockLevel) -> bool {
    l.on_hand < 0 || l.available() < 0
}

/// Must the kitchen's own ledger be read? Only when an item both tails touched was measured on
/// the hub's tail while the hub counts it: then the ancestor may have been uncounted, and the
/// kitchen's level is not `ancestor + dKitchen` without knowing which.
pub(crate) fn needs_kitchen(hub: &StockLedger, dh: &TailDelta, dk: &TailDelta) -> bool {
    dh.items.iter().any(|(item, h)| h.arms && !h.absolute && hub.is_counted(item) && dk.items.contains_key(item))
}

/// The obstructions of two tails over one ancestor, given the hub's ledger at its tip and,
/// when `needs_kitchen` said so, the kitchen's. Without it the kitchen's level is DERIVED:
/// `ancestor + dKitchen`, the ancestor being `hub - dHub` exactly (every hub delta on a shared
/// item is additive there). Also answers the glued level of every item both tails touched.
pub(crate) fn obstructions(
    hub: &StockLedger,
    kit: Option<&StockLedger>,
    dh: &TailDelta,
    dk: &TailDelta,
) -> (Vec<Conflict>, Vec<(String, StockLevel)>) {
    let mut out = Vec::new();
    let mut shared = Vec::new();
    for (item, h) in &dh.items {
        let Some(k) = dk.items.get(item) else { continue };
        if h.absolute || k.absolute {
            let absolute_in = if h.absolute { Side::Hub } else { Side::Kitchen };
            out.push(Conflict::Recount { item: item.clone(), absolute_in });
            continue;
        }
        let (lh, ch) = (hub.level(item), hub.is_counted(item));
        let (lk, ck) = match kit {
            Some(kl) => (kl.level(item), kl.is_counted(item)),
            None => (plus(minus(lh, h), k), ch || k.arms),
        };
        if ch != ck {
            // Uncounted at the ancestor, first measured on one branch only. The other branch's
            // restriction is exact (it never measured); the clip at the measurement is a no-op
            // iff that ancestor level was not negative.
            let base = if ch { minus(lk, k) } else { minus(lh, h) };
            if base.on_hand < 0 {
                out.push(Conflict::Recount { item: item.clone(), absolute_in: if ch { Side::Hub } else { Side::Kitchen } });
                continue;
            }
        } else {
            // Both restrictions to the ancestor must name one level. Derived, they do by
            // construction; read from two ledgers, this is the cross-check.
            let (bh, bk) = (minus(lh, h), minus(lk, k));
            if bh != bk {
                out.push(Conflict::Diverged { item: item.clone(), from_hub: bh, from_kitchen: bk });
                continue;
            }
        }
        let merged = plus(lh, k);
        shared.push((item.clone(), merged));
        if (ch || ck) && broken(merged) && !(ch && broken(lh)) && !(ck && broken(lk)) {
            out.push(Conflict::Oversold { item: item.clone(), merged, hub: lh, kitchen: lk });
        }
    }
    takes(hub, kit, dh, dk, &mut out);
    (out, shared)
}

/// Holds and served lines both tails moved: the glued amount may not go below zero.
fn takes(hub: &StockLedger, kit: Option<&StockLedger>, dh: &TailDelta, dk: &TailDelta, out: &mut Vec<Conflict>) {
    let both: Vec<&(String, String)> = dh.held.keys().filter(|key| dk.held.contains_key(*key)).collect();
    if !both.is_empty() {
        let open = |l: &StockLedger| -> BTreeMap<(String, String), Qty> { l.stranded().into_iter().map(|(o, i, q)| ((o, i), q)).collect() };
        let open_h = open(hub);
        let open_k = kit.map(open);
        for key in both {
            let h = open_h.get(key).copied().unwrap_or(0);
            let k = match &open_k {
                Some(m) => m.get(key).copied().unwrap_or(0),
                None => h - dh.held[key] + dk.held[key],
            };
            let merged = h + dk.held[key];
            if merged < 0 && h >= 0 && k >= 0 {
                out.push(Conflict::DoubleTake { order_id: key.0.clone(), item: key.1.clone(), what: "held", merged });
            }
        }
    }
    for (key, dkq) in &dk.served {
        if !dh.served.contains_key(key) {
            continue;
        }
        let of = |l: &StockLedger| l.served_of(&key.0).into_iter().find(|(i, _)| *i == key.1).map(|x| x.1).unwrap_or(0);
        let merged = of(hub) + dkq;
        if merged < 0 {
            out.push(Conflict::DoubleTake { order_id: key.0.clone(), item: key.1.clone(), what: "served", merged });
        }
    }
}
