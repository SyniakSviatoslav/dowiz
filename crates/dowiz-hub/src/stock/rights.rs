//! SHEAF S1: ESCROW RIGHTS IN THE STOCK LOG (R-BEBOPDB Part G, rows G.2 and G.5 S1; Balegas et
//! al. 2015, bounded counters). The hub hands a tablet, at each sync, a share of what it has:
//! `rights{ctx, item, qty, until}`. Offline, the tablet sells against that share; the shares of
//! all contexts never add up to more than the hub's `available`, so when the tails are glued the
//! shelf cannot go below zero through sales that stayed inside their rights -- the escrow
//! theorem, proved over the same fold in `bebop-lang/formal/Bebop/Stock.lean` (`escrow`).
//!
//! A SALE PAST ITS RIGHTS IS RECORDED, NEVER REFUSED. Operator question §H.3 is unanswered; the
//! default under "працюй сам" is the report's recommendation: the tablet keeps selling, the sale
//! past its share is written PROVISIONAL (`"prov":1`), and after the glue it is a person's to
//! decide (`glue::Conflict::Provisional`) -- keep it, amend the order, refund it.
//!
//! THE FORMAT, AND WHY NO READER HAD TO CHANGE FIRST. A grant is a NOTE (`notes.rs`):
//! `{"k":"note","note":"rights","ctx":..,"item":..,"qty":..,"until":..,"at":..}`. `decode`
//! answers `None` for every note, so the shelf, the journal, the cost book, the checkpoints, the
//! station bindings, glue's tail and the wasm decider (which loads through `StockLog`) all walk
//! past it as they always have; `StockLog::load` quarantines only a failed crc. A tablet's sale is
//! an ordinary `served` record with two META keys, `ctx` and `prov` (`meta.rs`): `decode` reads
//! the same `item`/`qty`/`order`, so it folds exactly as the plain sale. A log with no grants and
//! no tablet sales is byte-for-byte the log it was (`oldimage_tests.rs`, unchanged digests).
//!
//! THE BUDGET IS A FOLD, like the shelf: the newest grant to `(ctx, item)` REPLACES the one before
//! it (a sync re-issues the share), and every sale that context recorded inside its rights after
//! it uses the budget up. A provisional sale uses nothing: it is already past.
//!
//! NOT HERE (S2): the hub's own order door (`Reserved` through `room`) does not yet subtract the
//! rights outstanding to tablets; only `sell(HUB, ..)` does. Until S2 wires it, an online order
//! can take stock a tablet was promised, and `unflagged_oversold` is the check that would show it.

use std::collections::BTreeMap;

use super::meta::{meta_of, Meta};
use super::notes::is_note;
use super::{decode, Qty, StockError, StockEvent, StockLedger, StockLog};
use crate::minijson::{esc, int_field, str_field};

/// The note kind of a grant.
pub const NOTE: &str = "rights";
/// The quoted key every provisional sale carries (`meta.rs` writes `"prov":1`): a cheap
/// prefilter for readers. It matches the key, not the value, so a record spelled with
/// whitespace still reaches the full parse; a false hit only costs that parse.
pub const PROV_KEY: &str = r#""prov""#;

/// The quoted key a grant and a context's sale carry (`meta.rs` writes `"ctx":"..."`): the fold's
/// prefilter, as `PROV_KEY` is glue's.
pub const CTX_KEY: &str = r#""ctx""#;

/// The hub's own context: its share is what no tablet holds.
pub const HUB: &str = "hub";

/// One context's share of one item since its newest grant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Budget {
    pub granted: Qty,
    /// Sold inside the rights since the grant.
    pub used: Qty,
    /// The grant's last instant (ms, the hub's clock); `None` = until the next sync.
    pub until: Option<i64>,
}

impl Budget {
    /// What the context may still sell inside its rights.
    pub fn left(&self) -> Qty {
        (self.granted - self.used).max(0)
    }

    /// Still held at `now`: an expired grant is the hub's again.
    pub fn live(&self, now: Option<i64>) -> bool {
        match (self.until, now) {
            (Some(u), Some(n)) => n <= u,
            _ => true,
        }
    }
}

/// Every context's budget, folded from the log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rights {
    budgets: BTreeMap<(String, String), Budget>,
}

/// A sale recorded past its rights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provisional {
    pub ctx: String,
    pub item: String,
    pub order_id: String,
    pub qty: Qty,
}

/// A draw that took a counted shelf below zero WITHOUT being flagged provisional: what the
/// escrow theorem says cannot happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oversale {
    pub item: String,
    pub on_hand: Qty,
    /// The record's position in the log, oldest = 0.
    pub at: usize,
}

impl Rights {
    /// Fold the budgets from raw records, OLDEST FIRST.
    pub fn fold(raw: &[String]) -> Rights {
        let mut r = Rights::default();
        for rec in raw {
            r.step(rec);
        }
        r
    }

    fn step(&mut self, rec: &str) {
        // Only a grant and a context's sale move a budget, and both carry the `"ctx"` key: every
        // other record (a checkpoint, an order's reservation, a delivery) leaves here before the
        // meta parse, which on a 3000-record log was ~7 ms of a sync's ~8 (S2 profile).
        if !rec.contains(CTX_KEY) {
            return;
        }
        if is_note(rec, NOTE) {
            if let Some((ctx, item, qty)) = grant_of(rec) {
                self.budgets.insert((ctx, item), Budget { granted: qty, used: 0, until: int_field(rec, "until") });
            }
            return;
        }
        // The two keys this fold reads, read alone (the full `meta_of` parses twenty), with
        // `meta_of`'s own filters: a blank context is none, and only `prov:1` is provisional.
        let ctx = str_field(rec, "ctx").filter(|c| !c.trim().is_empty());
        let prov = int_field(rec, "prov").filter(|p| *p == 1);
        let (Some(ctx), None) = (ctx, prov) else { return };
        if let Some(StockEvent::Served { item, qty, .. }) = decode(rec) {
            if let Some(b) = self.budgets.get_mut(&(ctx, item)) {
                b.used += qty;
            }
        }
    }

    pub fn budget(&self, ctx: &str, item: &str) -> Option<Budget> {
        self.budgets.get(&(ctx.to_string(), item.to_string())).copied()
    }

    /// The rights still held at `now` on `item` by every context except `except`.
    pub fn outstanding(&self, item: &str, except: &str, now: Option<i64>) -> Qty {
        self.budgets.iter().filter(|((c, i), b)| i == item && c != except && b.live(now)).map(|(_, b)| b.left()).sum()
    }

    /// What `ctx` may sell of `item` inside its rights now: its own budget, or -- for the hub --
    /// what is available and promised to no tablet.
    pub fn may_sell(&self, ctx: &str, item: &str, led: &StockLedger, now: Option<i64>) -> Qty {
        if ctx == HUB {
            return (led.available(item) - self.outstanding(item, HUB, now)).max(0);
        }
        self.budget(ctx, item).filter(|b| b.live(now)).map_or(0, |b| b.left())
    }
}

fn grant_of(rec: &str) -> Option<(String, String, Qty)> {
    let ctx = str_field(rec, "ctx").filter(|c| !c.trim().is_empty())?;
    Some((ctx, str_field(rec, "item")?, int_field(rec, "qty").filter(|q| *q >= 0)?))
}

/// A raw record that is a sale flagged provisional.
pub fn provisional_of(rec: &str) -> Option<Provisional> {
    let m = meta_of(rec);
    m.prov?;
    match decode(rec)? {
        StockEvent::Served { item, qty, order_id } => Some(Provisional { ctx: m.ctx.unwrap_or_default(), item, order_id, qty }),
        _ => None,
    }
}

impl StockLog {
    /// Every context's budget now.
    pub fn rights(&self) -> Rights {
        Rights::fold(&self.raw())
    }

    /// THE HUB GRANTS `ctx` A SHARE of `item` (0 revokes), replacing its previous grant.
    /// Refused, nothing written: no context or the hub's own, a negative quantity, or a share
    /// that with every other context's live rights would exceed what is available -- the
    /// escrow theorem's premise, kept at the only door that creates rights.
    pub fn grant(&mut self, ctx: &str, item: &str, qty: Qty, until: Option<i64>) -> Result<(), StockError> {
        let ctx = ctx.trim();
        if ctx.is_empty() || ctx == HUB || item.trim().is_empty() {
            return Err(StockError::Malformed);
        }
        if qty < 0 {
            return Err(StockError::NotPositive { qty });
        }
        let led = self.ledger()?;
        let others = self.rights().outstanding(item, ctx, self.clock);
        let free = led.available(item) - others;
        if qty > free {
            return Err(StockError::OutOfStock { item: item.to_string(), wanted: qty, available: free.max(0) });
        }
        let mut body = format!(r#"{{"ctx":"{}","item":"{}","qty":{qty}"#, esc(ctx), esc(item));
        if let Some(u) = until {
            body.push_str(&format!(r#","until":{u}"#));
        }
        body.push('}');
        self.append_note(NOTE, &body)
    }

    /// S2's re-issue at a sync (`glue::merge`): the same note as `grant`, for a caller that has
    /// already checked the premise against ONE ledger and ONE rights fold for every item
    /// (`grant` folds both per call, which at a sync's ~80 items is the whole log 160 times).
    pub(crate) fn grant_unchecked(&mut self, ctx: &str, item: &str, qty: Qty) -> Result<(), StockError> {
        if ctx.trim().is_empty() || ctx.trim() == HUB || qty < 0 {
            return Err(StockError::Malformed);
        }
        self.append_note(NOTE, &format!(r#"{{"ctx":"{}","item":"{}","qty":{qty}}}"#, esc(ctx.trim()), esc(item)))
    }

    /// `ctx` SELLS `qty` of `item` for `order_id`: always recorded (a `Served` stamped with the
    /// context), and flagged provisional when the shelf is counted and the sale is past what
    /// `ctx` may sell inside its rights. Answers whether it was provisional.
    pub fn sell(&mut self, ctx: &str, item: &str, qty: Qty, order_id: &str) -> Result<bool, StockError> {
        let led = self.ledger()?;
        let inside = qty <= self.rights().may_sell(ctx, item, &led, self.clock);
        let prov = led.is_counted(item) && !inside;
        let ev = StockEvent::Served { item: item.to_string(), qty, order_id: order_id.to_string() };
        let meta = Meta { ctx: Some(ctx.to_string()), prov: prov.then_some(1), ..Meta::default() };
        self.append_with(&ev, &meta)?;
        Ok(prov)
    }

    /// Every sale flagged provisional, OLDEST FIRST.
    pub fn provisional_sales(&self) -> Vec<Provisional> {
        self.raw().iter().filter_map(|r| provisional_of(r)).collect()
    }

    /// THE ESCROW CHECK over a whole log: replay it with every provisional sale set aside, and
    /// name each draw that still takes a counted shelf below zero. Empty = every sale beyond what
    /// was on hand was flagged.
    pub fn unflagged_oversold(&self) -> Result<Vec<Oversale>, StockError> {
        let mut led = StockLedger::default();
        let mut out = Vec::new();
        for (at, rec) in self.raw().iter().enumerate() {
            let Some(ev) = decode(rec) else { continue };
            if provisional_of(rec).is_some() {
                continue;
            }
            led.apply(&ev)?;
            let draws = matches!(ev, StockEvent::Served { .. } | StockEvent::Consumed { .. } | StockEvent::Wasted { .. } | StockEvent::Cooked { .. });
            let level = led.level(ev.item()).on_hand;
            if draws && led.is_counted(ev.item()) && level < 0 {
                out.push(Oversale { item: ev.item().to_string(), on_hand: level, at });
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "rights/tests.rs"]
mod tests;
