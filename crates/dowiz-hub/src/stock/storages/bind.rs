//! A STATION BOUND TO A STORAGE (W-STORE2, operator 2026-10-05: "один склад
//! на всіх і прив'язаний"). By default ONE storage serves every station: a
//! sale draws from the storage that last received the item (`Stores::home`),
//! exactly as before this file. The owner MAY bind a kitchen station (the
//! dish's `"station"`: kitchen, sushi, bar -- `bell_route.rs`) to a storage;
//! a dish of a bound station then draws from that storage.
//!
//! THE BINDING IS A NOTE (`{"station","store","by"}`, kind [`BOUND`]); an
//! empty `store` unbinds. The newest note of a station wins. It is folded into
//! `Stores::bound` and carried through checkpoints (section `K`, written only
//! when a station is bound) -- a venue that never binds has no note, no key on
//! any record and byte-equal checkpoints.
//!
//! THE DRAW. `draws_for` knows each dish's station, so every `Draw` carries
//! its share per station (in millionths). The write door resolves the shares
//! against the bindings: if ANY share lands in a bound storage, the record
//! gets `"drawn":"<store>=<uq>;..."` ("" = the unbound home). A `served` is
//! spread over its shares at once; a `reserved` is REMEMBERED (`Stores::held`,
//! checkpoint section `B`) and its later `consumed` -- which `settle` writes
//! with no meta -- is spread over the same shares; `released` forgets it.
//!
//! LACK OF STOCK IS DECIDED AS IT ALWAYS WAS: the reservation is checked
//! against the WHOLE shelf (`ledger::decide`), never one storage. A bound
//! storage that does not hold enough goes negative in that storage, as the
//! unbound home already could -- no silent fall-through to another storage.

use std::collections::BTreeMap;

use super::{valid_id, Stores};
use crate::minijson::{esc, str_field};
use crate::stock::notes::is_note;
use crate::stock::{Qty, StockError, StockEvent, StockLog};

/// The note kind of a binding.
pub const BOUND: &str = "bound";
/// The kitchen stations a dish can belong to (`bell_route::Station`).
pub const STATIONS: [&str; 3] = ["kitchen", "sushi", "bar"];

/// station -> storage.
pub type Bound = BTreeMap<String, String>;

/// A dish's station from its catalogue record: `"sushi"`/`"bar"`, anything
/// else (absent included) the kitchen -- the rule `bell_route::of_line` uses.
pub fn station_of(product_json: &str) -> &'static str {
    match str_field(product_json, "station").as_deref() {
        Some("sushi") => "sushi",
        Some("bar") => "bar",
        _ => "kitchen",
    }
}

/// The binding a raw record is, `(station, store)` ("" = unbound), or `None`.
pub fn bound_of(rec: &str) -> Option<(String, String)> {
    if !is_note(rec, BOUND) {
        return None;
    }
    let station = str_field(rec, "station").filter(|s| STATIONS.contains(&s.as_str()))?;
    Some((station, str_field(rec, "store").unwrap_or_default()))
}

/// Fold one binding into `bound`.
pub fn set(bound: &mut Bound, station: &str, store: &str) {
    if store.is_empty() {
        bound.remove(station);
    } else {
        bound.insert(station.to_string(), store.to_string());
    }
}

/// `store=uq;store=uq`, in the given order.
pub fn encode_drawn(shares: &[(String, i64)]) -> String {
    shares.iter().map(|(s, u)| format!("{s}={u}")).collect::<Vec<_>>().join(";")
}

/// [`encode_drawn`]'s inverse; a malformed share makes the whole key `None`
/// (the record then draws as an unbound one).
pub fn parse_drawn(s: &str) -> Option<Vec<(String, i64)>> {
    let mut out = Vec::new();
    for part in s.split(';') {
        let (st, u) = part.split_once('=')?;
        let u: i64 = u.parse().ok().filter(|u| *u > 0)?;
        if !st.is_empty() && !valid_id(st) {
            return None;
        }
        out.push((st.to_string(), u));
    }
    (!out.is_empty()).then_some(out)
}

/// The `drawn` key for a draw whose share per station is `stations`, or
/// `None` when no share lands in a bound storage (the record is written as
/// it always was). Shares of one storage are merged, sorted by storage.
pub fn resolve(bound: &Bound, stations: &[(String, i64)]) -> Option<String> {
    if bound.is_empty() {
        return None;
    }
    let mut per: BTreeMap<String, i64> = BTreeMap::new();
    for (st, uq) in stations {
        let to = bound.get(st).cloned().unwrap_or_default();
        *per.entry(to).or_insert(0) += uq;
    }
    if per.keys().all(String::is_empty) {
        return None;
    }
    let shares: Vec<(String, i64)> = per.into_iter().filter(|(_, u)| *u > 0).collect();
    Some(encode_drawn(&shares))
}

/// Each supply's share per station over an order's `(product JSON, portions)`
/// lines, in whole base units -- `reservations_for`'s arithmetic, kept per
/// station (a room round re-reserved after an amendment, `room::rules`).
pub fn station_shares(lines: &[(String, i64)]) -> BTreeMap<String, Vec<(String, i64)>> {
    let mut per: BTreeMap<String, BTreeMap<&'static str, i64>> = BTreeMap::new();
    for (product_json, ordered) in lines {
        if *ordered <= 0 {
            continue;
        }
        let st = station_of(product_json);
        for l in crate::stock::bom_of(product_json) {
            let e = per.entry(l.supply).or_default().entry(st).or_insert(0);
            *e = e.saturating_add(l.qty.saturating_mul(*ordered));
        }
    }
    per.into_iter().map(|(i, m)| (i, m.into_iter().map(|(s, q)| (s.to_string(), q)).collect())).collect()
}

impl Stores {
    /// The stations bound now.
    pub fn bound(&self) -> &Bound {
        &self.bound
    }

    pub(in crate::stock) fn bind(&mut self, station: &str, store: &str) {
        set(&mut self.bound, station, store);
    }

    /// `d` of `item` spread over `shares` by their weight: each share its
    /// floor, the last the remainder, so the parts always sum to `d`.
    fn spread(&mut self, item: &str, shares: &[(String, i64)], d: Qty) {
        let total: i128 = shares.iter().map(|(_, u)| i128::from(*u)).sum();
        let home = self.home(item).to_string();
        if total <= 0 {
            self.add(item, &home, d);
            return;
        }
        let mut left = d;
        for (i, (s, u)) in shares.iter().enumerate() {
            let part = if i + 1 == shares.len() { left } else { (i128::from(d) * i128::from(*u) / total) as Qty };
            left -= part;
            let to = if s.is_empty() { home.as_str() } else { s.as_str() };
            self.add(item, to, part);
        }
    }

    /// A draw of a bound station. True: this record is folded here.
    pub(super) fn drawn_step(&mut self, ev: &StockEvent, drawn: Option<&str>, d: Qty) -> bool {
        let shares = drawn.and_then(parse_drawn);
        let key = |o: &str, i: &str| (o.to_string(), i.to_string());
        match ev {
            StockEvent::Reserved { item, order_id, .. } => {
                let Some(sh) = shares else { return false };
                let e = self.held.entry(key(order_id, item)).or_default();
                for (s, u) in sh {
                    match e.iter_mut().find(|(x, _)| *x == s) {
                        Some(x) => x.1 = x.1.saturating_add(u),
                        None => e.push((s, u)),
                    }
                }
                true
            }
            StockEvent::Consumed { item, order_id, .. } => match self.held.remove(&key(order_id, item)) {
                Some(sh) => {
                    self.spread(item, &sh, d);
                    true
                }
                None => false,
            },
            StockEvent::Released { item, order_id, .. } => {
                self.held.remove(&key(order_id, item));
                false
            }
            StockEvent::Served { item, .. } => match shares {
                Some(sh) => {
                    self.spread(item, &sh, d);
                    true
                }
                None => false,
            },
            _ => false,
        }
    }
}

impl StockLog {
    /// The stations bound now (the fold from the newest checkpoint).
    pub fn bindings(&self) -> Result<Bound, StockError> {
        Ok(self.journal_now()?.stores.bound)
    }

    /// Bind `station` to `store`, or unbind it (`store` empty). REFUSED,
    /// nothing written: unsigned; a station that is not kitchen, sushi or
    /// bar; a storage this venue does not have, or one archived -- the words
    /// name it.
    pub fn bind_station(&mut self, station: &str, store: &str, by: &str) -> Result<(), StockError> {
        if by.trim().is_empty() {
            return Err(StockError::Unsigned);
        }
        if !STATIONS.contains(&station) {
            return Err(StockError::Linkage(format!("{station:?}: a station is kitchen, sushi or bar")));
        }
        if !store.is_empty() && !self.storages().iter().any(|s| s.id == store && !s.archived) {
            return Err(StockError::Linkage(format!("{store}: no such storage here, or it is archived")));
        }
        let body = format!(r#"{{"station":"{}","store":"{}","by":"{}"}}"#, esc(station), esc(store), esc(by));
        self.append_note(BOUND, &body)
    }
}

#[cfg(test)]
#[path = "bind_tests.rs"]
mod tests;
