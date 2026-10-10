//! THE OBJECT'S EDGE TABLE (R-GRAPH D4, DW2): every `/fold/*` route this
//! object answers, the images it is derived from, the projection it serves,
//! how that projection is kept, and what its output is keyed by. ONE static
//! table, so three things that used to be scattered are read off one place:
//!
//!   * LAW 8. `rebuild` walks every `Step::Memo` row (`rebuild_rows`): a memo
//!     added to the table without a check makes the rebuild an ERROR, never a
//!     silent pass. Every other step folds from the bytes per read, writes, or
//!     answers about the object itself -- there is no kept answer to go stale.
//!   * THE ROUTES. `edges/tests.rs` reads the route table out of `hubdo.rs`
//!     (and the two read dispatchers, `reads.rs` and `facts.rs`) and refuses a
//!     route with no row, a row with no route, and a duplicate: rows == routes.
//!   * EARLY CUTOFF (AX3, D5). `OutKey::K64` rows compare their output before
//!     moving anything downstream (`fold/menu/out.rs`); `OutKey::Generation`
//!     rows are keyed by their input's generation and are stepped or dropped
//!     by the write itself (`fold::projection::Written`: a whole-image write --
//!     rotation, import, forget -- drops every dependent).
//!
//! Inputs are IMAGE names as stored (`m:<id>`); a row whose answer is a fold
//! of the orders projection names `log`, which that projection is derived from.

use super::HubImages;
use worker::*;

/// How a row's answer is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Kept in the object between requests, keyed by input generations.
    Memo,
    /// The `recent` ring: written by the writes, cleared by a gap.
    Window,
    /// Folded from the images on every request; nothing is kept.
    PerRead,
    /// Held in memory and derived from no image (sockets, counters).
    Memory,
    /// A write: a command, an import, the timer.
    Command,
    /// Writes a memo's output out of the object (R2).
    Sink,
    /// The check itself (law 8).
    Check,
    /// Needs the platform (`fetch`), not an image.
    Platform,
}

/// What a row's output is keyed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutKey {
    /// No kept output.
    None,
    /// Its input image's generation (the orders projection: `x-generation`).
    Generation,
    /// crc K64 of the output bytes, confirmed byte for byte (`fold/menu/out.rs`).
    K64,
}

/// One edge: `route` (the segment after `/fold/`) serves `projection`,
/// derived from `inputs` by `step`, its output keyed by `out_key`.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(test), allow(dead_code))] // `inputs` and `out_key` are read by `edges/tests.rs` and by people
pub(crate) struct Edge {
    pub route: &'static str,
    pub inputs: &'static [&'static str],
    pub projection: &'static str,
    pub step: Step,
    pub out_key: OutKey,
}

/// The orders projection (`fold::projection`, `HubImages::folded`).
pub(crate) const ORDERS: &str = "orders";
/// The storefront's menu (`fold::menu::Memo`, `HubImages::menu`).
pub(crate) const MENU: &str = "menu";

const LOG: &[&str] = &["log"];
const CAT: &[&str] = &["catalog"];
const MENU_IN: &[&str] = &["catalog", "i18n", "settings"];
const NONE: &[&str] = &[];

const fn e(route: &'static str, inputs: &'static [&'static str], projection: &'static str, step: Step, out_key: OutKey) -> Edge {
    Edge { route, inputs, projection, step, out_key }
}
const fn read(route: &'static str, inputs: &'static [&'static str]) -> Edge {
    e(route, inputs, route, Step::PerRead, OutKey::None)
}
const fn cmd(route: &'static str, inputs: &'static [&'static str]) -> Edge {
    e(route, inputs, route, Step::Command, OutKey::None)
}

/// EVERY `/fold/*` ROUTE, one row each. `edges/tests.rs` measures this
/// against the route table; add the row in the same change as the route.
pub(crate) const EDGES: &[Edge] = &[
    // ── the kept projections (law 8 checks these) ──
    e("orders", LOG, ORDERS, Step::Memo, OutKey::Generation),
    e("order", LOG, ORDERS, Step::Memo, OutKey::Generation),
    e("menu", MENU_IN, MENU, Step::Memo, OutKey::K64),
    e("products", MENU_IN, MENU, Step::Memo, OutKey::K64),
    e("publish", MENU_IN, MENU, Step::Sink, OutKey::K64),
    e("changes", LOG, "changes", Step::Window, OutKey::Generation),
    // ── folded per read ──
    read("venue", CAT),
    read("preps", CAT),
    read("catalogue", &["catalog", "i18n"]),
    read("basket", CAT),
    read("analytics", &["log", "catalog", "cube", "settings"]),
    read("kitchen", &["log", "catalog", "stock", "cube"]),
    read("stock", &["catalog", "stock"]),
    read("exceptions", &["log", "till", "settings", "catalog", "ledger"]),
    read("week_top", &["log", "catalog"]),
    read("prep", &["log", "catalog", "stock", "cube", "bookings"]),
    read("haccp", &["catalog", "stock"]),
    read("reveals", LOG),
    read("assist", &["log", "catalog", "stock"]),
    read("graph", &["log", "catalog", "stock"]),
    read("kitchen_facts", &["log", "catalog", "stock"]),
    read("waste", &["stock", "log"]),
    e("generation", LOG, "generation", Step::PerRead, OutKey::Generation),
    // ── memory, the check, the platform ──
    e("positions", NONE, "positions", Step::Memory, OutKey::None),
    e("counters", NONE, "counters", Step::Memory, OutKey::None),
    e("rebuild", &["log", "stock", "catalog", "i18n", "settings"], "rebuild", Step::Check, OutKey::None),
    e("cron", NONE, "cron", Step::Platform, OutKey::None),
    e("socket", NONE, "socket", Step::Platform, OutKey::None),
    // ── the writes ──
    cmd("timer", &["outbox", "fiscal", "ebills", "floor", "settings"]),
    cmd("compact", NONE),
    cmd("bulk", CAT),
    cmd("print", &["outbox"]),
    cmd("room", &["log", "stock", "till", "ledger"]),
    cmd("chat", NONE),
    cmd("forget", &["people", "taste", "log", "catalog"]),
    cmd("ebills", &["ebills", "floor", "log", "stock"]),
    cmd("refund", &["log", "stock", "outbox"]),
    cmd("aggregator", &["log", "stock"]),
    cmd("returned", &["stock"]),
    cmd("stock_move", &["stock"]),
    cmd("stock_reset", &["stock"]),
    cmd("kitchen_ack", LOG),
    cmd("place", &["log", "stock", "settings", "outbox", "fiscal"]),
    cmd("advance", &["log", "stock"]),
    cmd("assign", &["log", "ops"]),
    cmd("append", LOG),
];

/// The row a route names, if any.
#[cfg(test)]
pub(crate) fn row(route: &str) -> Option<&'static Edge> {
    EDGES.iter().find(|r| r.route == route)
}

/// The kept projections, once each, in table order.
pub(crate) fn memos() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for r in EDGES.iter().filter(|r| r.step == Step::Memo) {
        if !out.contains(&r.projection) {
            out.push(r.projection);
        }
    }
    out
}

impl HubImages {
    /// LAW 8 OVER THE TABLE: every memo row is checked, or the rebuild fails.
    /// `report` already holds the orders check (`rebuild::of_log`, or nothing
    /// to check when there is no log).
    pub(super) async fn rebuild_rows(&self, mut report: crate::rebuild::Report) -> Result<crate::rebuild::Report> {
        for projection in memos() {
            let intact = match projection {
                ORDERS => true, // `rebuild::of_log`: its findings are `report.stale`
                MENU => self.menu_intact().await?,
                other => return Err(Error::RustError(format!("edges: the memo `{other}` has no rebuild check"))),
            };
            report.checked.push(projection.to_string());
            if !intact {
                report.stale_memos.push(projection.to_string());
            }
        }
        Ok(report)
    }

    /// Is the menu being served the fold of its inputs' bytes? A memo that is
    /// not current is not served (the next read refolds): intact. A current one
    /// is refolded from the bytes, ignoring it, and compared by its out bytes.
    async fn menu_intact(&self) -> Result<bool> {
        let gens = self.menu_gens_in_memory();
        let served = {
            let memo = self.menu.borrow();
            if !crate::fold::menu::is_current(&memo, gens) {
                return Ok(true);
            }
            memo.as_ref().map(crate::fold::menu::Memo::out_bytes)
        };
        let [c, i, s] = super::menu::MENU_INPUTS;
        let (c, i, s) = (self.image(c).await?, self.image(i).await?, self.image(s).await?);
        let bytes = |x: &Option<(super::Meta, Vec<u8>)>| x.as_ref().map(|(_, b)| b.clone());
        let rails = crate::fold::menu::Rails {
            stripe_key: self.state.secret("STRIPE_PUBLISHABLE_KEY"),
            telegram_bot: self.state.secret("TELEGRAM_BOT_USERNAME"),
        };
        let fresh = crate::fold::menu::Memo::from_images(gens, bytes(&c).as_deref(), bytes(&i).as_deref(), bytes(&s).as_deref(), rails);
        Ok(matches!((fresh, served), (Ok(f), Some(b)) if f.out_bytes() == b))
    }
}

#[cfg(test)]
mod tests;
