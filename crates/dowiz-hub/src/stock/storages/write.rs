//! THE WRITE DOORS OF `storages.rs`: the fold now, the venue's storages, a
//! storage's card (name, rename, archive) and a transfer -- each refused with
//! nothing written when it would break the storage rules.

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
            let held = j.stores.in_store(&s.id, &j.ledger);
            if let Some((item, q)) = held.first() {
                return Err(StockError::Linkage(format!("{}: still holds {q} of {item}; move it out or count it first", s.id)));
            }
        }
        let body = format!(r#"{{"id":"{}","name":"{}","archived":{}}}"#, esc(&s.id), esc(name), i64::from(s.archived));
        self.append_note(STORAGE, &body)
    }

    /// A TRANSFER. Refused, nothing written: unsigned, not a quantity, the
    /// same storage twice, a storage the venue does not have (or archived,
    /// as the destination), and more than `from` holds -- a transfer never
    /// makes stock, so it cannot take what is not there.
    pub fn move_stock(&mut self, m: &Moved) -> Result<(), StockError> {
        if m.by.trim().is_empty() {
            return Err(StockError::Unsigned);
        }
        if m.qty <= 0 {
            return Err(StockError::NotPositive { qty: m.qty });
        }
        let known = self.storages();
        let has = |id: &str, open: bool| known.iter().any(|s| s.id == id && !(open && s.archived));
        if m.from == m.to || !has(&m.from, false) || !has(&m.to, true) {
            return Err(StockError::Linkage(format!("a transfer is between two storages of this venue: {} -> {}", m.from, m.to)));
        }
        let j = self.journal_now()?;
        let available = j.stores.level(&m.item, &m.from, &j.ledger);
        if available < m.qty {
            return Err(StockError::OutOfStock { item: m.item.clone(), wanted: m.qty, available });
        }
        self.append_note(MOVED, &m.body())
    }
}

