//! EXACT LEVELS PER STORAGE (operator 2026-10-17: "зроби дробове переміщення
//! між складами"): 0.25 kg, half a gram, half a piece moved from one storage
//! to another changes both EXACTLY, and the item's total does not move by one
//! millionth. Integers only: millionths (`uq`), the unit the carry keeps.
//!
//! THE MODEL. The whole levels (`Stores::levels`) stay what they were: a
//! fold of booked deltas that sums to the ledger's `on_hand`. Beside them,
//! `Stores::frac` keeps per `(item, storage)` the millionths that storage's
//! exact level differs from its whole one, and the item's carried remainder
//! that no storage has taken yet -- the RESIDUAL -- sits with the item's home
//! (where an unnamed sale draws from, where an unnamed delivery goes):
//!
//! ```text
//! exact(item, s) = level(item, s) x 10^6 + frac(item, s) + [s == home] x residual(item)
//! residual(item) = -carry(item) - Σ_s frac(item, s)
//! ```
//!
//! so `Σ_s exact(item, s) = on_hand x 10^6 - carry`, the item's exact shelf
//! (`Carry::exact`), BY CONSTRUCTION, whatever the record.
//!
//! WHAT MOVES `frac`: a record that changes the carry and lands in a storage
//! that is NOT the home (a delivery, write-off or count naming it) takes the
//! change there; a transfer moves its own fraction (`uq - qty x 10^6`) from
//! `from` to `to`; and before an item's home changes (a delivery or transfer
//! to another storage) the residual is PINNED to the old home, so a delivery
//! elsewhere never shifts a fraction between two storages it did not touch.
//!
//! THE OLD-IMAGE RULE. A log with no fraction anywhere keeps `frac` empty,
//! the checkpoint section `F` is written only when it is not, and every
//! checkpoint already on a venue reads as it did: old whole levels are exact
//! levels with zero fraction, and the old item carry stays the item's (with
//! its home), so no total changes. Known limit: a station-bound draw's
//! sub-unit fraction stays with the home, its whole units go to the station.

use std::collections::BTreeSet;

use super::{Moved, Stores, DEFAULT, MOVED};
use crate::minijson::{esc, int_field, str_field};
use crate::stock::carry::{Carry, MICRO};
use crate::stock::notes::is_note;
use crate::stock::StockLedger;

/// A move id (the console's `moveId`): 1..=64 of `A-Z a-z 0-9 _ -`.
pub fn valid_move_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Millionths as the shortest exact decimal: 750000 -> "0.75", -50000 ->
/// "-0.05", 7000000 -> "7". Never through a float.
pub fn decimal(uq: i64) -> String {
    let (neg, a) = (uq < 0, uq.unsigned_abs());
    let (w, f) = (a / MICRO as u64, a % MICRO as u64);
    let mut s = if neg { format!("-{w}") } else { w.to_string() };
    if f != 0 {
        s.push('.');
        s.push_str(format!("{f:06}").trim_end_matches('0'));
    }
    s
}

/// Whole units nearest `uq` millionths, half up.
pub(super) fn round_half_up(uq: i64) -> i64 {
    (i128::from(uq) + i128::from(MICRO) / 2).div_euclid(i128::from(MICRO)) as i64
}

impl Moved {
    /// `uq` millionths of `item` from `from` to `to`, signed by `by`.
    pub fn new(item: &str, uq: i64, from: &str, to: &str, by: &str) -> Moved {
        Moved { item: item.into(), qty: 0, uq, from: from.into(), to: to.into(), by: by.into(), id: None }
    }

    /// The note's body (`StockLog::append_note`). A whole transfer with no
    /// id is the very bytes written before fractions moved.
    pub fn body(&self) -> String {
        let uq = if self.uq == self.qty.saturating_mul(MICRO) { String::new() } else { format!(r#","uq":{}"#, self.uq) };
        let id = self.id.as_deref().map(|i| format!(r#","id":"{}""#, esc(i))).unwrap_or_default();
        format!(
            r#"{{"item":"{}","qty":{}{uq},"from":"{}","to":"{}","by":"{}"{id}}}"#,
            esc(&self.item),
            self.qty,
            esc(&self.from),
            esc(&self.to),
            esc(&self.by)
        )
    }

    /// The transfer a raw record is, or `None` for any other record.
    pub fn of(rec: &str) -> Option<Moved> {
        if !is_note(rec, MOVED) {
            return None;
        }
        let qty = int_field(rec, "qty").filter(|q| *q >= 0)?;
        Some(Moved {
            item: str_field(rec, "item")?,
            qty,
            uq: int_field(rec, "uq").unwrap_or(qty.saturating_mul(MICRO)).max(0),
            from: str_field(rec, "from")?,
            to: str_field(rec, "to")?,
            by: str_field(rec, "by").unwrap_or_default(),
            id: str_field(rec, "id"),
        })
        .filter(|m| m.uq > 0)
    }
}

impl Stores {
    pub(super) fn add_frac(&mut self, item: &str, store: &str, d: i64) {
        if d == 0 {
            return;
        }
        let key = (item.to_string(), store.to_string());
        let v = self.frac.get(&key).copied().unwrap_or(0).saturating_add(d);
        if v == 0 {
            self.frac.remove(&key);
        } else {
            self.frac.insert(key, v);
        }
    }

    /// The item's carried remainder no storage has taken: it is the home's.
    fn residual(&self, item: &str, carry_of: i64) -> i64 {
        let held: i64 = self.frac.iter().filter(|((i, _), _)| i == item).map(|(_, v)| *v).sum();
        carry_of.saturating_neg().saturating_sub(held)
    }

    /// Before `item`'s home becomes `new_home`: the residual stays with the
    /// storage it is in (`carry_of`: the item's carry at that point).
    pub(super) fn pin_home(&mut self, item: &str, new_home: &str, carry_of: i64) {
        let old = self.home(item).to_string();
        if old != new_home {
            let r = self.residual(item, carry_of);
            self.add_frac(item, &old, r);
        }
    }

    /// A record's carry change `dc` that landed in `here`: the home's
    /// residual takes it by construction; any other storage takes it here.
    pub(super) fn land_frac(&mut self, item: &str, here: &str, dc: i64) {
        if dc != 0 && self.home(item) != here {
            self.add_frac(item, here, dc.saturating_neg());
        }
    }

    /// `item`'s EXACT level in `store`, in millionths.
    pub fn exact(&self, item: &str, store: &str, led: &StockLedger, carry: &Carry) -> i64 {
        if !self.used {
            return if store == DEFAULT { carry.exact(item, led.level(item).on_hand) } else { 0 };
        }
        let key = (item.to_string(), store.to_string());
        let mut v = self.level(item, store, led).saturating_mul(MICRO).saturating_add(self.frac.get(&key).copied().unwrap_or(0));
        if self.home(item) == store {
            v = v.saturating_add(self.residual(item, carry.of(item)));
        }
        v
    }

    /// `item`'s non-zero EXACT levels, by storage id. They sum to
    /// `carry.exact(item, on_hand)`.
    pub fn exact_of(&self, item: &str, led: &StockLedger, carry: &Carry) -> Vec<(String, i64)> {
        let mut ids: BTreeSet<String> = BTreeSet::new();
        ids.insert(if self.used { self.home(item).to_string() } else { DEFAULT.to_string() });
        ids.extend(self.levels.keys().chain(self.frac.keys()).filter(|(i, _)| i == item).map(|(_, s)| s.clone()));
        ids.into_iter().map(|s| (s.clone(), self.exact(item, &s, led, carry))).filter(|(_, q)| *q != 0).collect()
    }

    /// Every item with a non-zero EXACT level in `store` (an archive refusal).
    pub fn in_store_exact(&self, store: &str, led: &StockLedger, carry: &Carry) -> Vec<(String, i64)> {
        let mut items: BTreeSet<String> = led.items().into_iter().map(|(i, _)| i).collect();
        items.extend(self.levels.keys().chain(self.frac.keys()).map(|(i, _)| i.clone()));
        items.into_iter().map(|i| (i.clone(), self.exact(&i, store, led, carry))).filter(|(_, q)| *q != 0).collect()
    }

    /// The checkpoint's `F` section (` F <n> <item> <store> <uq>...`), or ""
    /// when no storage holds a fraction -- an old venue's checkpoint keeps its bytes.
    pub(in crate::stock) fn frac_section(&self) -> String {
        if self.frac.is_empty() {
            return String::new();
        }
        let mut s = format!(" F {}", self.frac.len());
        for ((i, st), v) in &self.frac {
            s.push_str(&format!(" {}:{i} {}:{st} {v}", i.len(), st.len()));
        }
        s
    }

    /// Read a `F` section at the start of `rest`; answers what follows it.
    pub(in crate::stock) fn read_frac<'a>(&mut self, rest: &'a str) -> Option<&'a str> {
        let tok = |r: &'a str| -> Option<(&'a str, &'a str)> {
            let t = r.strip_prefix(' ')?;
            let end = t.find(' ').unwrap_or(t.len());
            Some((&t[..end], &t[end..]))
        };
        let text = |r: &'a str| -> Option<(String, &'a str)> {
            let (len, t) = r.strip_prefix(' ')?.split_once(':')?;
            let len: usize = len.parse().ok()?;
            Some((t.get(..len)?.to_string(), &t[len..]))
        };
        let (tag, mut r) = tok(rest)?;
        let (n, r2) = tok(r).filter(|_| tag == "F")?;
        r = r2;
        for _ in 0..n.parse::<usize>().ok()? {
            let (i, r3) = text(r)?;
            let (st, r4) = text(r3)?;
            let (v, r5) = tok(r4)?;
            self.frac.insert((i, st), v.parse().ok()?);
            r = r5;
        }
        Some(r)
    }
}

#[cfg(test)]
#[path = "exact/tests.rs"]
mod tests;
