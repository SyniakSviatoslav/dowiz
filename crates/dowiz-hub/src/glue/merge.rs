//! SHEAF S2: THE HUB ADOPTS A GLUED TAIL (G.2's "rebase on the single writer"). The venue's
//! object is the one writer; a tablet's tail is replayed onto the hub's log after the hub's own
//! tail -- Replicache's server-authoritative rebase -- AS WRITTEN (`stock::adopt`), so each
//! adopted record keeps the bytes the tablet chained.
//!
//! NOTHING THE TABLET SENT IS LOST, and that is a law, not a hope: `recs.len() == merged + held`
//! (`Reply::law`). A record the hub's ledger refuses on rebase (a reservation past what is left,
//! a consume whose hold the hub already released) is not written as a movement; it is written
//! INSIDE a decision note, verbatim, for a person. Every obstruction the glue found
//! (`Conflict`) is a decision note too. Gluing still DETECTS and does not resolve: the decision
//! notes are the "decision needed" list (`desk.rs`) and a person closes each one.
//!
//! THEN THE RIGHTS (S1's escrow) are re-issued for the next offline stretch: every counted item
//! gets `SHARE_NUM/SHARE_DEN` of what is free of other contexts' live rights (R-S7 E5: the hub
//! keeps a quarter), written only where the share moved. Then one `glued` note, which makes a
//! retried request a replay: the same tail sent twice is adopted once.
//!
//! THE REPLY is everything the hub's log holds above the tablet's `base` -- the hub's own tail,
//! the adopted records, the notes, any checkpoint -- so the tablet can replay it verbatim and
//! land on the hub's tip (`tablet::rebase`). An adopted record goes back as its INDEX in the
//! tail the tablet sent (`Entry::Echo`), never as its bytes a second time.

use super::wire::{glue_tail, Tail};
use super::{Conflict, Refusal, Side, Why};
use crate::crypto::hex;
use crate::minijson::{esc, str_field};
use crate::stock::notes::is_note;
use crate::stock::rights::Rights;
use crate::stock::{Qty, StockLog};

/// The decision note's kind.
pub const DECIDE: &str = "decide";
/// The note that records one adopted tail.
pub const GLUED: &str = "glued";
/// A context's share of what is free: three quarters (the hub keeps a quarter, R-S7 E5).
pub const SHARE_NUM: Qty = 3;
pub const SHARE_DEN: Qty = 4;

/// One record of the reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A hub record, as written.
    Rec(String),
    /// The tablet's own record at this index of the tail it sent, adopted as written.
    Echo(usize),
}

/// What the hub answers a glue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// The hub's tip after the adoption: the tablet's next `base`.
    pub tip: [u8; 32],
    /// Everything above the tablet's `base`, OLDEST FIRST.
    pub entries: Vec<Entry>,
    /// The obstructions found, each now a decision note.
    pub conflicts: Vec<Conflict>,
    /// Records the hub's ledger took.
    pub merged: usize,
    /// Records it refused, `(position in the tail, reason)`, each in a decision note.
    pub held: Vec<(usize, String)>,
    /// How many grants were (re)issued to the context.
    pub grants: usize,
    /// True: this tail had been adopted already; nothing was written.
    pub replay: bool,
    /// What the tail cost on the wire (its records' bytes).
    pub wire_up: usize,
}

impl Reply {
    /// THE MERGE LAW: every record the tablet sent is a movement or a held decision.
    pub fn law(&self, sent: usize) -> bool {
        self.replay || sent == self.merged + self.held.len()
    }

    /// What the reply's records cost on the wire (an echo as its index's digits).
    pub fn wire_down(&self) -> usize {
        self.entries.iter().map(|e| match e { Entry::Rec(s) => s.len(), Entry::Echo(i) => i.to_string().len() }).sum()
    }
}

/// A refusal of the merge: the glue's, or the hub's ledger refusing its own history.
pub fn hub_refused(why: String) -> Refusal {
    Refusal { side: Side::Hub, why: Why::Ledger(why) }
}

/// GLUE AND ADOPT one tablet tail. `hub_bytes` is `log`'s image as loaded. On `Ok` the caller
/// persists `log` (nothing was written when `replay`); on a refusal nothing was written.
pub fn merge(hub_bytes: &[u8], log: &mut StockLog, t: &Tail) -> Result<Reply, Refusal> {
    let ktip = hex(&t.tip);
    let above = log.since(&t.base).ok_or(Refusal { side: Side::Kitchen, why: Why::NoCommonAncestor })?;
    if t.recs.is_empty() || above.iter().any(|r| r.contains(&ktip) && is_note(r, GLUED) && str_field(r, "ctx").as_deref() == Some(t.ctx.as_str()) && str_field(r, "ktip").as_deref() == Some(ktip.as_str())) {
        // A tablet with nothing to send asks only for the hub's tail (read-only, so a board that
        // polls writes nothing), except at its first sync, which is when it gets its rights; a
        // retried tail was adopted already.
        let first = t.base == [0u8; 32];
        let grants = if t.recs.is_empty() && first {
            let led = log.ledger().map_err(|e| hub_refused(e.to_string()))?;
            regrant(log, &t.ctx, &led)?
        } else {
            0
        };
        let all = if grants > 0 { log.since(&t.base).unwrap_or_default() } else { above };
        return Ok(reply(log, t, all, Vec::new(), 0, Vec::new(), grants, !t.recs.is_empty()));
    }
    // ONE FOLD FOR THE TURN (S2 profile: the glue, the adoption and the grants each folded the
    // shelf, 0.64 ms apiece): the glue reads it, the adoption carries it forward, the grants read
    // what the adoption left.
    let start = log.ledger_since().map_err(|e| hub_refused(e.to_string()))?;
    let (g, _) = glue_tail(hub_bytes, log, &start.0, t)?;
    let old_tip = log.tip_id();
    let (adopted, led) = log.adopt_from(&t.recs, start).map_err(|e| hub_refused(e.to_string()))?;
    let mut n = 0usize;
    let mut note = |log: &mut StockLog, body: String| -> Result<(), Refusal> {
        n += 1;
        let id = format!("{}-{n}", &ktip[..16]);
        log.append_note(DECIDE, &format!(r#"{{"did":"{id}","ctx":"{}",{body}}}"#, esc(&t.ctx))).map_err(|e| hub_refused(e.to_string()))
    };
    for c in &g.conflicts {
        note(log, conflict_body(c))?;
    }
    for (pos, why) in &adopted.held {
        note(log, format!(r#""kind":"held","pos":{pos},"why":"{}","rec":"{}""#, esc(why), esc(&t.recs[*pos])))?;
    }
    let grants = regrant(log, &t.ctx, &led)?;
    let body = format!(
        r#"{{"ctx":"{}","base":"{}","ktip":"{ktip}","n":{},"merged":{},"held":{}}}"#,
        esc(&t.ctx),
        hex(&t.base),
        t.recs.len(),
        adopted.merged,
        adopted.held.len()
    );
    log.append_note(GLUED, &body).map_err(|e| hub_refused(e.to_string()))?;
    // The reply is what was above `base` before plus what this turn wrote: one walk over the
    // new records only, not a second walk down to `base`.
    let mut all = above;
    all.extend(log.since(&old_tip).unwrap_or_default());
    Ok(reply(log, t, all, g.conflicts, adopted.merged, adopted.held, grants, false))
}

/// The reply over `all`, the hub's records above the tablet's base, OLDEST FIRST.
#[allow(clippy::too_many_arguments)]
fn reply(log: &StockLog, t: &Tail, all: Vec<String>, conflicts: Vec<Conflict>, merged: usize, held: Vec<(usize, String)>, grants: usize, replay: bool) -> Reply {
    let mut j = 0;
    let entries = all
        .into_iter()
        .map(|r| {
            if j < t.recs.len() && r == t.recs[j] {
                j += 1;
                Entry::Echo(j - 1)
            } else {
                Entry::Rec(r)
            }
        })
        .collect();
    Reply { tip: log.tip_id(), entries, conflicts, merged, held, grants, replay, wire_up: t.recs.iter().map(String::len).sum() }
}

/// A conflict as a decision note's keys (the note adds `did` and `ctx`).
fn conflict_body(c: &Conflict) -> String {
    let side = |s: &Side| if *s == Side::Hub { "hub" } else { "kitchen" };
    match c {
        Conflict::Oversold { item, merged, .. } => format!(r#""kind":"oversold","item":"{}","qty":{}"#, esc(item), merged.on_hand.min(merged.available())),
        Conflict::Recount { item, absolute_in } => format!(r#""kind":"recount","item":"{}","side":"{}""#, esc(item), side(absolute_in)),
        Conflict::DoubleTake { order_id, item, what, merged } => {
            format!(r#""kind":"double","item":"{}","order":"{}","what":"{what}","qty":{merged}"#, esc(item), esc(order_id))
        }
        Conflict::Diverged { item, .. } => format!(r#""kind":"diverged","item":"{}""#, esc(item)),
        Conflict::Provisional { side: s, item, order_id, qty, .. } => {
            format!(r#""kind":"provisional","item":"{}","order":"{}","qty":{qty},"side":"{}""#, esc(item), esc(order_id), side(s))
        }
    }
}

/// RE-ISSUE `ctx`'s rights: for every counted item, `SHARE_NUM/SHARE_DEN` of what is available
/// and promised to no other context, written only where it differs from what `ctx` has left.
/// One ledger (the caller's) and one rights fold for all of them. Answers how many grants were written.
fn regrant(log: &mut StockLog, ctx: &str, led: &crate::stock::StockLedger) -> Result<usize, Refusal> {
    let rights: Rights = log.rights();
    let mut n = 0;
    for (item, _) in led.items() {
        if !led.is_counted(&item) {
            continue;
        }
        let free = (led.available(&item) - rights.outstanding(&item, ctx, None)).max(0);
        let share = free * SHARE_NUM / SHARE_DEN;
        if rights.budget(ctx, &item).map(|b| b.left()) == Some(share) || (share == 0 && rights.budget(ctx, &item).is_none()) {
            continue;
        }
        log.grant_unchecked(ctx, &item, share).map_err(|e| hub_refused(e.to_string()))?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
#[path = "merge/tests.rs"]
pub(crate) mod tests;
