//! THE WRITE DOORS OF `storages.rs`: the fold now, the venue's storages, a
//! storage's card (name, rename, archive) and a transfer -- each refused with
//! nothing written when it would break the storage rules.

use super::exact::{decimal, round_half_up, valid_move_id};
use super::{valid_id, Moved, Storage, DEFAULT, DEFAULTS, MOVED, NAME_MAX, STORAGE};
use crate::minijson::{esc, int_field, str_field};
use crate::stock::journal::Journal;
use crate::stock::{StockError, StockLog};

impl StockLog {
    /// The fold NOW, from the newest checkpoint and the records after it:
    /// shelf, cost, lots, carry and storages. Its `entries` are the tail only.
    pub fn journal_now(&self) -> Result<Journal, StockError> {
        let t = self.tail(|_, _| true);
        let mut j = t.base.unwrap_or_default();
        for rec in &t.recs {
            j.step(rec)?;
        }
        Ok(j)
    }

    /// The venue's storages: the three defaults, then every card in the log
    /// (the newest card of an id wins), in first-seen order.
    pub fn storages(&self) -> Vec<Storage> {
        let mut out: Vec<Storage> = DEFAULTS.iter().map(|d| Storage { id: d.to_string(), name: String::new(), archived: false }).collect();
        for rec in self.notes(STORAGE) {
            let Some(id) = str_field(&rec, "id").filter(|i| valid_id(i)) else { continue };
            let card = Storage {
                id: id.clone(),
                name: str_field(&rec, "name").unwrap_or_default(),
                archived: int_field(&rec, "archived") == Some(1),
            };
            match out.iter_mut().find(|s| s.id == id) {
                Some(s) => *s = card,
                None => out.push(card),
            }
        }
        out
    }

    /// Name, rename or archive a storage. REFUSED, nothing written: a bad id
    /// or name, archiving [`DEFAULT`] (where every old record lives), and
    /// archiving a storage that still holds stock -- move it out, or count it
    /// to zero, first. A storage is never deleted: its history stays.
    pub fn put_storage(&mut self, s: &Storage) -> Result<(), StockError> {
        let refuse = |m: &str| Err(StockError::Linkage(m.to_string()));
        let name = s.name.trim();
        if !valid_id(&s.id) {
            return refuse("a storage id is 1 to 32 of a-z, 0-9, _ and -");
        }
        if name.chars().count() > NAME_MAX || (name.is_empty() && !DEFAULTS.contains(&s.id.as_str())) {
            return refuse("a storage has a name of 1 to 40 characters");
        }
        if s.archived {
            if s.id == DEFAULT {
                return refuse("the default storage cannot be archived");
            }
            let j = self.journal_now()?;
            // EXACT: 0.25 kg left in it is stock in it (2026-10-17).
            let held = j.stores.in_store_exact(&s.id, &j.ledger, &j.carry);
            if let Some((item, q)) = held.first() {
                return Err(StockError::Linkage(format!("{}: still holds {} of {item}; move it out or count it first", s.id, decimal(*q))));
            }
            // W-STORE2: a station's storage stays open while it is bound.
            if let Some((station, _)) = j.stores.bound().iter().find(|(_, st)| **st == s.id) {
                return Err(StockError::Linkage(format!("{}: the {station} station draws from it; unbind the station first", s.id)));
            }
        }
        let body = format!(r#"{{"id":"{}","name":"{}","archived":{}}}"#, esc(&s.id), esc(name), i64::from(s.archived));
        self.append_note(STORAGE, &body)
    }

    /// A TRANSFER of `m.uq` millionths, EXACT (2026-10-17). Refused, nothing
    /// written: unsigned, not a quantity, the same storage twice, a storage
    /// the venue does not have (or archived, as the destination), more than
    /// `from` holds EXACTLY ("salmon: kitchen holds only 0.5"), an id reused
    /// for another transfer -- and, checked on the write, storages that would
    /// not sum to the item's exact shelf. Answers `false`, nothing written,
    /// for a transfer whose `id` is already in the log (a retry moves once).
    pub fn move_stock(&mut self, m: &Moved) -> Result<bool, StockError> {
        if m.by.trim().is_empty() {
            return Err(StockError::Unsigned);
        }
        if m.uq <= 0 {
            return Err(StockError::NotPositive { qty: m.uq });
        }
        if let Some(id) = m.id.as_deref() {
            if !valid_move_id(id) {
                return Err(StockError::Linkage(format!("{id:?}: a move id is 1 to 64 of A-Z, a-z, 0-9, _ and -")));
            }
            if let Some(was) = self.notes(MOVED).iter().filter_map(|r| Moved::of(r)).find(|w| w.id.as_deref() == Some(id)) {
                let same = (&was.item, was.uq, &was.from, &was.to) == (&m.item, m.uq, &m.from, &m.to);
                return if same { Ok(false) } else { Err(StockError::Linkage(format!("move {id} was already used for another transfer"))) };
            }
        }
        let known = self.storages();
        let has = |id: &str, open: bool| known.iter().any(|s| s.id == id && !(open && s.archived));
        if m.from == m.to || !has(&m.from, false) || !has(&m.to, true) {
            return Err(StockError::Linkage(format!("a transfer is between two storages of this venue: {} -> {}", m.from, m.to)));
        }
        let j = self.journal_now()?;
        let (st, led, carry) = (&j.stores, &j.ledger, &j.carry);
        let have = st.exact(&m.item, &m.from, led, carry);
        if have < m.uq {
            return Err(StockError::Linkage(format!("{}: {} holds only {}", m.item, m.from, decimal(have))));
        }
        // THE WHOLE UNITS the booked levels move: what makes `to`'s booked
        // level its exact one rounded, never more than `from` books. A whole
        // transfer with no fraction anywhere is its own quantity (old bytes).
        let to_have = st.exact(&m.item, &m.to, led, carry);
        let to_level = st.level(&m.item, &m.to, led);
        let want = round_half_up(to_have.saturating_add(m.uq)) - to_level;
        let mv = Moved { qty: want.clamp(0, st.level(&m.item, &m.from, led).max(0)), ..m.clone() };
        // THE INVARIANT, on the write: each storage moves by exactly `uq` and
        // the storages still sum to the item's exact shelf.
        let mut after = st.clone();
        after.apply_move(&mv, led, carry);
        let sum: i64 = after.exact_of(&m.item, led, carry).iter().map(|x| x.1).sum();
        let ok = sum == carry.exact(&m.item, led.level(&m.item).on_hand)
            && after.exact(&m.item, &m.from, led, carry) == have - m.uq
            && after.exact(&m.item, &m.to, led, carry) == to_have + m.uq;
        if !ok {
            return Err(StockError::Linkage(format!("{}: the storages would not sum to the shelf; nothing moved", m.item)));
        }
        self.append_note(MOVED, &mv.body()).map(|()| true)
    }
}
