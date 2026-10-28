//! SHEAF S2: A TABLET'S TAIL AS IT TRAVELS, glued against the hub's log (R-BEBOPDB G.3 "on the
//! wire: only the tail"). S0 glued two whole images; a tablet never sends its image. It sends
//! the records it wrote since its last sync, AS WRITTEN, with two ids:
//!
//!   `base` -- the hub record its branch starts from (the hub's tip at the last sync), and
//!   `tip`  -- its own newest record's id.
//!
//! THE GLUING CONDITION, ON THE WIRE. A record's id is `sha256(prev ‖ payload)` (`chain.rs`), so
//! the hub recomputes the tablet's chain from `base` over the bytes it received and must land on
//! `tip`: one 32-byte compare proves the tail is the tablet's whole section above the overlap,
//! untouched and complete. `base` must be in the hub's chain -- the overlap's digest is that id.
//! Nothing else about the tablet's image is needed or sent.
//!
//! NO-TRACKING, AS A RESTRICTION MAP (G.1 item 3). A kitchen tail is stock movements and nothing
//! else: every record must decode as a `StockEvent` and carry only keys a stock record has
//! (`KEYS`: the codec's and `meta.rs`'s). A record with any other key -- a phone, a guest, a
//! taste vector -- is refused by name before anything is read from it (`Why::Tracking`), so a
//! guest profile cannot ride into the hub on a kitchen tail. (An `order` id is opaque here, as
//! it is in every stock record.)
//!
//! What the obstruction is, and how it is found, is S0's (`delta`): the same fold of the two
//! tails over one ancestor, with the hub's ledger at its tip.

use super::delta::{self, TailDelta};
use super::walk::{to_cells, Chain};
use super::{open, refuse, tail_events, Conflict, Glued, Refusal, Side, Why};
use crate::stock::rights::{provisional_of, PROV_KEY};
use crate::stock::{decode, StockEvent, StockLedger, StockLog};

/// The longest record a tail may carry: a movement with every meta key is ~400 bytes.
pub const REC_MAX: usize = 1024;
/// The most records one tail may carry (~10 days of a busy kitchen; a tablet syncs per tail).
pub const TAIL_MAX: usize = 4096;

/// Every key a stock record may carry: `codec.rs`'s, then `meta.rs`'s.
pub const KEYS: [&str; 33] = [
    "k", "item", "qty", "order", "reason", "by", "observed", "id", "resell", "chosen_by", "out", "stage", "into", "act",
    "planned", "gross", "at", "expiry", "unit_cost", "per", "expected", "value", "uq", "prov", "lot", "supplier", "doc",
    "session", "by_", "store", "treated", "drawn", "ctx",
];

/// A tablet's tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tail {
    /// The context that wrote it (`kitchen`, a waiter's tablet).
    pub ctx: String,
    /// The hub record the branch starts from; all zeros for a tablet that never synced.
    pub base: [u8; 32],
    /// The tablet's newest record's id (`base` when the tail is empty).
    pub tip: [u8; 32],
    /// The records above `base`, OLDEST FIRST, as written.
    pub recs: Vec<String>,
}

/// The top-level keys of one flat JSON object, or `None` when it is not one (nested values,
/// a broken string). Kitchen records are flat by construction (`encode` + `with_meta`).
pub fn keys_of(rec: &str) -> Option<Vec<String>> {
    let b = rec.trim().as_bytes();
    if b.first() != Some(&b'{') || b.last() != Some(&b'}') {
        return None;
    }
    let mut keys = Vec::new();
    let mut i = 1;
    let ws = |i: &mut usize| {
        while *i < b.len() && b[*i].is_ascii_whitespace() {
            *i += 1;
        }
    };
    loop {
        ws(&mut i);
        if b.get(i) == Some(&b'}') && i + 1 == b.len() && keys.is_empty() {
            return Some(keys);
        }
        let (key, after) = string_at(b, i)?;
        keys.push(key);
        i = after;
        ws(&mut i);
        if b.get(i) != Some(&b':') {
            return None;
        }
        i += 1;
        ws(&mut i);
        i = match b.get(i)? {
            b'"' => string_at(b, i)?.1,
            b'{' | b'[' => return None,
            _ => {
                let s = i;
                while i < b.len() && !matches!(b[i], b',' | b'}') {
                    i += 1;
                }
                if i == s {
                    return None;
                }
                i
            }
        };
        ws(&mut i);
        match b.get(i)? {
            b',' => i += 1,
            b'}' if i + 1 == b.len() => return Some(keys),
            _ => return None,
        }
    }
}

/// The string starting at `b[i] == '"'`: its raw text and the index after its closing quote.
fn string_at(b: &[u8], i: usize) -> Option<(String, usize)> {
    if b.get(i) != Some(&b'"') {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'"' => return Some((String::from_utf8_lossy(&b[i + 1..j]).into_owned(), j + 1)),
            _ => j += 1,
        }
    }
    None
}

/// The record at `pos` as a movement a tablet may send, or the refusal that names it.
fn admit(pos: usize, rec: &str) -> Result<StockEvent, Refusal> {
    if rec.len() > REC_MAX {
        return refuse(Side::Kitchen, Why::TooLong { pos });
    }
    let Some(keys) = keys_of(rec) else { return refuse(Side::Kitchen, Why::NotAMovement { pos }) };
    if let Some(k) = keys.into_iter().find(|k| !KEYS.contains(&k.as_str())) {
        return refuse(Side::Kitchen, Why::Tracking { pos, key: k });
    }
    match decode(rec) {
        Some(ev) => Ok(ev),
        None => refuse(Side::Kitchen, Why::NotAMovement { pos }),
    }
}

/// THE GLUING CHECK for a wire tail against the hub's log (`hub_bytes` is `hub`'s image, as the
/// hub's object holds it; `led_h` its ledger now, folded once by the caller for the whole turn).
/// Answers what S0's `glue` answers, plus the tail decoded.
pub fn glue_tail(hub_bytes: &[u8], hub: &StockLog, led_h: &StockLedger, t: &Tail) -> Result<(Glued, Vec<StockEvent>), Refusal> {
    if hub.quarantined() > 0 {
        return refuse(Side::Hub, Why::Quarantined(hub.quarantined()));
    }
    if t.recs.len() > TAIL_MAX {
        return refuse(Side::Kitchen, Why::TooLong { pos: TAIL_MAX });
    }
    let ch = open(Side::Hub, hub_bytes)?;
    let th = hub_tail(&ch, &t.base)?;
    // The tablet's section: every record admitted, every link recomputed, ending at its tip.
    let mut ek = Vec::with_capacity(t.recs.len());
    let mut prov = Vec::new();
    let mut prev = t.base;
    for (pos, rec) in t.recs.iter().enumerate() {
        ek.push(admit(pos, rec)?);
        prev = crate::content_id_chained(&prev, rec.as_bytes());
        if rec.contains(PROV_KEY) {
            if let Some(p) = provisional_of(rec) {
                prov.push(Conflict::Provisional { side: Side::Kitchen, ctx: p.ctx, item: p.item, order_id: p.order_id, qty: p.qty });
            }
        }
    }
    if prev != t.tip {
        return refuse(Side::Kitchen, Why::TipMismatch);
    }
    let mut hub_prov = Vec::new();
    let eh = tail_events(Side::Hub, &ch, &th, &t.base, &mut hub_prov)?;
    hub_prov.extend(prov);
    let mut g = Glued {
        ancestor: to_cells(&t.base),
        hub_tail: th.len(),
        kitchen_tail: t.recs.len(),
        wire_bytes: t.recs.iter().map(String::len).sum(),
        conflicts: hub_prov,
        shared: Vec::new(),
    };
    if th.is_empty() || ek.is_empty() {
        return Ok((g, ek));
    }
    let (dh, dk) = (TailDelta::of(&eh), TailDelta::of(&ek));
    let led_k = if delta::needs_kitchen(led_h, &dh, &dk) { Some(kitchen_ledger(&ch, th.len(), &ek)?) } else { None };
    let (conflicts, shared) = delta::obstructions(led_h, led_k.as_ref(), &dh, &dk);
    g.conflicts.extend(conflicts);
    g.shared = shared;
    Ok((g, ek))
}

/// The hub's records above `base`, newest first; refused when `base` is not in the hub's chain.
fn hub_tail(ch: &Chain<'_>, base: &[u8; 32]) -> Result<Vec<usize>, Refusal> {
    let zero = *base == [0u8; 32];
    let mut th = Vec::new();
    for o in ch.objs() {
        if !zero && ch.id(o) == *base {
            return Ok(th);
        }
        th.push(o);
    }
    if zero {
        return Ok(th);
    }
    refuse(Side::Kitchen, Why::NoCommonAncestor)
}

/// The tablet's ledger, when S0's derivation cannot give it: the hub's records up to `base`
/// folded from genesis, then the tablet's tail. Rare (`delta::needs_kitchen`) and O(log).
fn kitchen_ledger(ch: &Chain<'_>, above: usize, ek: &[StockEvent]) -> Result<StockLedger, Refusal> {
    let mut evs: Vec<StockEvent> = ch.objs().skip(above).filter_map(|o| decode(&String::from_utf8_lossy(&ch.payload(o)))).collect();
    evs.reverse();
    evs.extend_from_slice(ek);
    StockLedger::fold(&evs).map_err(|e| Refusal { side: Side::Kitchen, why: Why::Ledger(e.to_string()) })
}

#[cfg(test)]
#[path = "wire/tests.rs"]
mod tests;
