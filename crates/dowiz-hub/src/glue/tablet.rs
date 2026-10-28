//! SHEAF S2: THE TABLET'S SIDE (G.1 "the tablet's local stalk"). A kitchen tablet holds the
//! venue's stock log as it was at its last sync -- its `base`, the hub's tip then -- with its
//! own records appended while it was offline. Three things happen to that image, and this file
//! is all three, so the wasm module (`bebop-wasm --features tablet`) is a thin C surface over it
//! and every rule is the hub crate's own:
//!
//!   `record` -- one movement, decided against the tablet's own ledger by the same write door the
//!              hub uses; a sale is `sell(ctx, ..)`, flagged provisional past the tablet's
//!              rights (S1), never refused for them (§H.3 default, unanswered).
//!   `tail`   -- the records above `base`, as written, and the tip: the wire form (`wire::Tail`).
//!   `items`  -- the tablet's shelf, for the screen that records a movement.
//!   `rebase` -- the hub's reply replayed onto the image cut at `base`: the hub's records and the
//!              tablet's own echoed back, verbatim, landing on the hub's tip or refused. Records
//!              the tablet wrote AFTER it sent the tail are carried on top, verbatim.
//!
//! A tablet never writes a checkpoint (it would hold the branch's state, not the hub's); the
//! hub's checkpoints arrive in its tail and are replayed like any record.

use super::merge::Entry;
use super::wire::Tail;
use crate::stock::meta::Meta;
use crate::stock::{decode, StockError, StockEvent, StockLog};

/// What a fresh tablet image starts at (it grows, as the hub's does).
pub const FRESH_BYTES: usize = 64 * 1024;

/// Why the tablet could not do it.
#[derive(Debug)]
pub enum TabErr {
    /// The image is not a stock log, or it lost records (a quarantined crc).
    Image,
    /// The movement is not one this codec reads.
    Movement,
    /// The shelf refused it, as the hub's write door would.
    Stock(StockError),
    /// `base` is not in the tablet's chain.
    Base,
    /// The reply does not replay onto the image: an echo past the tail sent, or a tip that
    /// is not the hub's.
    Reply(&'static str),
}

fn open(image: &[u8]) -> Result<StockLog, TabErr> {
    let mut log = if image.is_empty() { StockLog::create_sized(FRESH_BYTES).map_err(|_| TabErr::Image)? } else { StockLog::load(image).map_err(|_| TabErr::Image)? };
    if log.quarantined() > 0 {
        return Err(TabErr::Image);
    }
    log.set_checkpoint_every(usize::MAX);
    Ok(log)
}

/// RECORD ONE MOVEMENT on the tablet. `rec` is one record in the codec's form
/// (`{"k":"wasted","item":..,"qty":..,"reason":..,"by":..}`); `now` the tablet's clock (ms).
/// Answers the new image and whether the movement was a sale past the tablet's rights.
pub fn record(image: &[u8], ctx: &str, rec: &str, now: i64) -> Result<(Vec<u8>, bool), TabErr> {
    let mut log = open(image)?;
    log.set_clock(now);
    let ev = decode(rec).ok_or(TabErr::Movement)?;
    let prov = match &ev {
        StockEvent::Served { item, qty, order_id } => log.sell(ctx, item, *qty, order_id).map_err(TabErr::Stock)?,
        _ => {
            log.append_with(&ev, &Meta { ctx: Some(ctx.to_string()), ..Meta::default() }).map_err(TabErr::Stock)?;
            false
        }
    };
    Ok((log.to_bytes_trimmed(), prov))
}

/// THE SHELF AS THE TABLET SEES IT: every item its log knows, sorted, with what is on hand --
/// the list the offline stock sheet steps through (no typed name that is not a supply).
pub fn items(image: &[u8]) -> Result<Vec<(String, crate::stock::Qty)>, TabErr> {
    let led = open(image)?.ledger().map_err(TabErr::Stock)?;
    Ok(led.items().into_iter().map(|(i, l)| (i, l.on_hand)).collect())
}

/// THE TAIL TO SEND: the records above `base`, as written, and the tablet's tip.
pub fn tail(image: &[u8], ctx: &str, base: &[u8; 32]) -> Result<Tail, TabErr> {
    let log = open(image)?;
    let recs = log.since(base).ok_or(TabErr::Base)?;
    Ok(Tail { ctx: ctx.to_string(), base: *base, tip: log.tip_id(), recs })
}

/// REPLAY THE HUB'S REPLY. `sent` is how many records of the tail went with the request (the
/// tablet may have written more since); `entries` and `tip` are the reply's. `from_zero`: the
/// reply starts at the hub's first record, not at `base` -- a tablet whose base the hub no
/// longer holds (the owner reset the shelf) starts over, its own records carried on top.
/// Answers the new image, whose records up to `tip` are the hub's, byte for byte.
pub fn rebase(image: &[u8], base: &[u8; 32], from_zero: bool, sent: usize, entries: &[Entry], tip: &[u8; 32]) -> Result<Vec<u8>, TabErr> {
    let old = open(image)?;
    let mine = old.since(base).ok_or(TabErr::Base)?;
    if sent > mine.len() {
        return Err(TabErr::Reply("more records sent than the tablet holds"));
    }
    let mut prefix: Vec<String> = Vec::new();
    if *base != [0u8; 32] && !from_zero {
        // The image cut at `base`: its own records from genesis, as written (their ids depend
        // only on the bytes and the record below, so a fresh image holding them holds the
        // same chain).
        let all = old.since(&[0u8; 32]).ok_or(TabErr::Base)?;
        prefix = all[..all.len() - mine.len()].to_vec();
    }
    let mut replay: Vec<&str> = prefix.iter().map(String::as_str).collect();
    for e in entries {
        replay.push(match e {
            Entry::Rec(s) => s.as_str(),
            Entry::Echo(i) if *i < sent => mine[*i].as_str(),
            Entry::Echo(_) => return Err(TabErr::Reply("an echo past the tail that was sent")),
        });
    }
    let mut fresh = open(&[])?;
    fresh.replay_raw(&replay, tip).map_err(|_| TabErr::Reply("the replay does not land on the hub's tip"))?;
    let unsent: Vec<&str> = mine[sent..].iter().map(String::as_str).collect();
    fresh.carry_raw(&unsent).map_err(TabErr::Stock)?;
    Ok(fresh.to_bytes_trimmed())
}

#[cfg(test)]
#[path = "tablet/tests.rs"]
mod tests;
