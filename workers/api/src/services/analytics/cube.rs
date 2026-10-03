//! PURE. THE DAILY SALES CUBE: the venue's history, one row per local day
//! (W-HIST P2b; operator 2026-10-03: "аналітику варто суттєво покращити і
//! точно добавити історію").
//!
//! A TENSION, STATED. `fold.rs` opens with "NO ANALYTICS STORE": a second
//! table of numbers is a second thing that can disagree with the orders. This
//! file IS such a store, and the operator approved it on 2026-10-03 (report
//! `docs/research/2026-10-03-depth-stock-analytics-models.md`, Q2). It keeps
//! the law in spirit, because it is a CACHE WITH A VERIFIER and never a source:
//!   * DERIVED: a row is the fold (`fold_orders`) of the orders that left the
//!     hot log at a rotation. Nothing else writes one.
//!   * VERIFIED: `hubdo/cube.rs` `trace` refolds a day from the archives its row
//!     names and compares the two rows BYTE FOR BYTE (`line`).
//!   * REBUILDABLE: drop the image and the next catch-up refolds every archive.
//! It exists because the hot log keeps thirty days (`hubstore::HOT_KEEP_MS`),
//! the kitchen window allows sixty-two and the owner's a year, and a request
//! cannot read a year of archives.
//!
//! DISJOINT BY CONSTRUCTION. A stored row holds only orders that LEFT the hot
//! log (in archive N, absent from the image after it, `moved`). The hot log
//! holds the rest. A day's number is the stored row plus the hot fold of that
//! day, and no order is in both.
//!
//! NOTHING ABOUT A PERSON IS KEPT: no phone, no name, no customer key. Money
//! is integer minor units.

use dowiz_hub::stock::meta::day_of_local_ms;
use dowiz_hub::tz::Zone;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

pub const DAY_MS: i64 = 86_400_000;
/// The image's name in the venue's object. Not `log@`: `is_archive_id`
/// refuses it, so no history route can read it as orders.
pub const IMAGE: &str = "cube";
/// The written format. A reader refuses any other.
pub const FORMAT: i64 = 1;

/// One venue-day. Short keys: a year is 365 of these in one image.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayCube {
    /// The venue's local day, `yyyymmdd`.
    pub d: i64,
    /// Orders placed, refused ones included (a customer who tried).
    pub o: i64,
    /// Of those, refused: the venue took no money.
    pub x: i64,
    /// What the venue took: total minus tip, of the accepted orders.
    pub t: i64,
    /// Food sales: unit price x quantity of the accepted orders' lines.
    pub f: i64,
    /// Delivery, pickup, at a table.
    pub k: [i64; 3],
    /// Orders by the venue's hour, and what the venue took by that hour.
    pub h: [i64; 24],
    pub ht: [i64; 24],
    /// channel -> [orders, taken].
    pub c: BTreeMap<String, [i64; 2]>,
    /// dish -> [portions, food sales, stamped portions, their stamped cost].
    pub m: BTreeMap<String, [i64; 4]>,
    /// The archives this row was folded from, in rotation order. Empty on a
    /// hot partial, which no archive holds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub src: Vec<String>,
}

impl DayCube {
    pub fn new(d: i64) -> Self {
        DayCube { d, ..DayCube::default() }
    }

    /// Add `other` into this row: every count summed, the sources joined.
    pub fn add(&mut self, other: &DayCube) {
        self.o += other.o;
        self.x += other.x;
        self.t += other.t;
        self.f += other.f;
        for i in 0..3 {
            self.k[i] += other.k[i];
        }
        for i in 0..24 {
            self.h[i] += other.h[i];
            self.ht[i] += other.ht[i];
        }
        for (ch, v) in &other.c {
            let e = self.c.entry(ch.clone()).or_default();
            e[0] += v[0];
            e[1] += v[1];
        }
        for (id, v) in &other.m {
            let e = self.m.entry(id.clone()).or_default();
            for i in 0..4 {
                e[i] += v[i];
            }
        }
        for s in &other.src {
            if !self.src.contains(s) {
                self.src.push(s.clone());
            }
        }
    }
}

/// The venue's day of an instant, `yyyymmdd`.
pub fn day_of(zone: Zone, at: i64) -> i64 {
    day_of_local_ms(dowiz_hub::tz::local_ms(zone, at))
}

/// The venue's hour of an instant, 0..=23, in the offset in force THEN.
pub fn hour_of(zone: Zone, at: i64) -> usize {
    ((dowiz_hub::tz::local_ms(zone, at).rem_euclid(DAY_MS)) / 3_600_000).clamp(0, 23) as usize
}

/// Fold orders into one row per local day. The same rules as `fold::fold`
/// (whose tests pin them): a refused order counts as a visit and by hour,
/// channel and kind, never as money or as a dish; a line with no product is
/// not a dish; an order dated after `now` is nobody's day yet.
pub fn fold_orders(orders: &[Value], zone: Zone, now: i64) -> BTreeMap<i64, DayCube> {
    use crate::services::orders::status;
    let mut out: BTreeMap<i64, DayCube> = BTreeMap::new();
    for o in orders {
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        if at > now {
            continue;
        }
        let d = day_of(zone, at);
        let row = out.entry(d).or_insert_with(|| DayCube::new(d));
        let st = o.get("status").and_then(Value::as_str).unwrap_or("");
        let refused = !status::took_money(st);
        let took = status::venue_took(
            o.get("total").and_then(Value::as_i64).unwrap_or(0),
            o.get("tip").and_then(Value::as_i64).unwrap_or(0),
            st,
        );
        row.o += 1;
        row.x += i64::from(refused);
        row.t += took;
        row.k[match crate::services::ordering::fulfilment::of(o) {
            "delivery" => 0,
            "dine_in" => 2,
            _ => 1,
        }] += 1;
        let hr = hour_of(zone, at);
        row.h[hr] += 1;
        row.ht[hr] += took;
        let ch = row.c.entry(crate::services::ordering::channel::of(o).unwrap_or("unknown").to_string()).or_default();
        ch[0] += 1;
        ch[1] += took;
        if refused {
            continue;
        }
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let Some(id) = it.get("product_id").and_then(Value::as_str).filter(|s| !s.is_empty()) else { continue };
            let q = it.get("quantity").and_then(Value::as_i64).unwrap_or(0);
            let money = it.get("unit_price").and_then(Value::as_i64).unwrap_or(0) * q;
            row.f += money;
            let m = row.m.entry(id.to_string()).or_default();
            m[0] += q;
            m[1] += money;
            if let Some(c) = crate::command::place::cost::line_cost(it, q) {
                m[2] += q;
                m[3] += c;
            }
        }
    }
    out
}

/// The orders that LEFT the hot log at the rotation that wrote `archive`:
/// in it, and not in the image after it (`next_ids`, the next archive or the
/// hot log). An order kept at that rotation is in the next image and is
/// counted when it leaves, never twice.
pub fn moved(archive: Vec<(String, String)>, next_ids: &HashSet<String>) -> Vec<(String, Value)> {
    archive
        .into_iter()
        .filter(|(id, _)| !next_ids.contains(id))
        .filter_map(|(id, json)| serde_json::from_str::<Value>(&json).ok().map(|o| (id, o)))
        .collect()
}

/// A row's canonical bytes: field order fixed by the struct, maps sorted.
/// THE VERIFIER COMPARES THESE, so a row and its refold agree byte for byte
/// or they do not agree.
pub fn line(r: &DayCube) -> String {
    serde_json::to_string(r).unwrap_or_default()
}

/// The image: a head line, then one line per day, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cube {
    /// The archives already folded in, in rotation order.
    pub folded: Vec<String>,
    pub rows: BTreeMap<i64, DayCube>,
}

#[derive(Serialize, Deserialize)]
struct Head {
    v: i64,
    folded: Vec<String>,
}

impl Cube {
    pub fn encode(&self) -> Vec<u8> {
        let head = serde_json::to_string(&Head { v: FORMAT, folded: self.folded.clone() }).unwrap_or_default();
        let mut s = head;
        s.push('\n');
        for r in self.rows.values() {
            s.push_str(&line(r));
            s.push('\n');
        }
        s.into_bytes()
    }

    /// The whole image.
    pub fn decode(bytes: &[u8]) -> Result<Cube, String> {
        Self::decode_range(bytes, i64::MIN, i64::MAX)
    }

    /// The head, and only the rows from `from` to `to` (`yyyymmdd`,
    /// inclusive) parsed: a row's day is read off its first key without
    /// parsing it, so a seven-day read of a year's image parses seven rows.
    /// AN UNREADABLE IMAGE IS AN ERROR, never an empty history.
    pub fn decode_range(bytes: &[u8], from: i64, to: i64) -> Result<Cube, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "the cube is not text".to_string())?;
        let mut lines = text.lines();
        let head: Head = serde_json::from_str(lines.next().unwrap_or("")).map_err(|e| format!("the cube's head is unreadable: {e}"))?;
        if head.v != FORMAT {
            return Err(format!("the cube is format {}, this reader knows {FORMAT}", head.v));
        }
        let mut rows = BTreeMap::new();
        for (n, l) in lines.enumerate().filter(|(_, l)| !l.is_empty()) {
            let d = l
                .strip_prefix("{\"d\":")
                .map(|r| r.chars().take_while(char::is_ascii_digit).collect::<String>())
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or(format!("cube row {} has no day", n + 1))?;
            if d < from || d > to {
                continue;
            }
            let r: DayCube = serde_json::from_str(l).map_err(|e| format!("cube row {d} is unreadable: {e}"))?;
            rows.insert(d, r);
        }
        Ok(Cube { folded: head.folded, rows })
    }

    /// Fold one archive's moved orders in: each day's partial is added to its
    /// row and names the archive. An archive already folded is refused, so a
    /// retried catch-up cannot count a day twice.
    pub fn absorb(&mut self, archive: &str, partials: BTreeMap<i64, DayCube>) -> bool {
        if self.folded.iter().any(|a| a == archive) {
            return false;
        }
        for (d, mut p) in partials {
            p.src = vec![archive.to_string()];
            self.rows.entry(d).or_insert_with(|| DayCube::new(d)).add(&p);
        }
        self.folded.push(archive.to_string());
        true
    }

    /// The listed archives not folded yet, in rotation order (tests only:
    /// the DO answers `pending` from its own list).
    #[cfg(test)]
    pub fn pending(&self, listed: &[String]) -> Vec<String> {
        listed.iter().filter(|a| !self.folded.contains(a)).cloned().collect()
    }
}

#[cfg(test)]
#[path = "cube/tests.rs"]
mod tests;
