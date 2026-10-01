//! A set of records, and the keys that find them, in one bebop image.
//!
//! WHAT THIS REPLACES. Twenty-nine D1 tables and roughly thirty indexes. There
//! is no query planner here and no `WHERE`: **every access path is a key that
//! was written on purpose**, and the Kv layout keeps its keys SORTED, which is
//! what makes a prefix scan the whole of `ORDER BY` and `LIKE 'x%'` that this
//! product ever needed.
//!
//! WHY THAT IS AN IMPROVEMENT AND NOT A REGRESSION. Six of this platform's
//! defects were a venue chosen by a guess, and every one of them was a sentence
//! only a table can say -- `ORDER BY created_at_ms LIMIT 1` over rows belonging
//! to whoever. A key has no default row. `content_i18n` had no venue column at
//! all and nobody noticed for months; a key is `<venue image>/<kind>/<id>` by
//! construction, because the image IS the venue.
//!
//! THE ONE RULE THIS FILE ENFORCES MECHANICALLY: a record's index entries are
//! written, replaced and removed in the SAME operation as the record. A record
//! without its index, or an index pointing at a record that is gone, is not a
//! state a caller can reach by forgetting something -- which is exactly how
//! hand-maintained denormalisation usually fails.
//!
//! It knows that because each record stores the list of keys it owns. Replacing
//! a record removes the keys the OLD version owned before writing the new one's.
//!
//! LAYOUT inside the Kv, chosen so that the three kinds of entry sort apart:
//! ```text
//!   r/<kind>/<id>   the record, as JSON
//!   x/<index key>   an index entry: the id it points at
//!   i/<kind>/<id>   the index keys that record owns, newline separated
//! ```
//!
//! COST. The Kv layout is rewritten eagerly on every put -- O(n) in the number
//! of entries -- so a `Table` is for sets bounded by the venue's own size:
//! people, dishes, keys, memberships. Anything that grows with USAGE is an
//! `EvLog` and belongs in the hub's log, not here. Getting that backwards is
//! how the catalogue acquired the same 8x amplification the order log had.

use crate::HubError;
use bebop_store::kv::Kv;
use bebop_store::Store;

/// A record and the keys that find it.
pub struct Table {
    store: Store,
    kv: Kv,
    /// The doubling loop's limit, which is the only honest denominator for a
    /// compacted image's usage -- see `bebop-ceiling-not-capacity`.
    ceiling: usize,
}

/// Why a write was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// An index key declared unique already points at a different record.
    ///
    /// THIS IS THE UNIQUE CONSTRAINT, and it is checkable here for a reason a
    /// Worker holding a copy of an image could never manage: the object is
    /// single-threaded, so between this read and the write that follows it
    /// nothing else runs.
    Taken { key: String, held_by: String },
}

impl core::fmt::Display for TableError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TableError::Taken { key, held_by } => {
                write!(f, "{key} is already held by {held_by}")
            }
        }
    }
}

fn rec_key(kind: &str, id: &str) -> String {
    format!("r/{kind}/{id}")
}
fn own_key(kind: &str, id: &str) -> String {
    format!("i/{kind}/{id}")
}
fn idx_key(key: &str) -> String {
    format!("x/{key}")
}

impl Table {
    pub fn create(ceiling_bytes: usize) -> Result<Self, HubError> {
        // START SMALL AND DOUBLE. The ceiling is the refusal point, not the
        // allocation: a platform with four users should not carry a quarter of
        // a megabyte of empty arena on every read.
        let mut store = Store::create_bytes(ceiling_bytes.min(16 * 1024))?;
        Kv::init_bytes(&mut store)?;
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Table { store, kv, ceiling: ceiling_bytes })
    }

    pub fn load(bytes: &[u8], ceiling_bytes: usize) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Table { store, kv, ceiling: ceiling_bytes })
    }

    pub fn to_bytes(&mut self) -> Result<Vec<u8>, HubError> {
        Ok(self.kv.compacted_bytes_fit(self.ceiling)?)
    }

    pub fn usage(&self) -> crate::Usage {
        crate::usage_of(&self.store, crate::ceiling_cells(self.ceiling))
    }

    /// The fold of every entry, for the four-way check against bebop, python
    /// and `InMemoryStore`. Order-independent because the keys are sorted.
    pub fn root(&self) -> String {
        self.kv.snapshot_root()
    }

    // ── records ────────────────────────────────────────────────────────────

    pub fn get(&self, kind: &str, id: &str) -> Option<String> {
        self.kv.get(&rec_key(kind, id)).and_then(|v| String::from_utf8(v).ok())
    }

    pub fn has(&self, kind: &str, id: &str) -> bool {
        self.kv.get(&rec_key(kind, id)).is_some()
    }

    /// Every record of a kind, in key order. The `ORDER BY` this product has.
    pub fn all(&self, kind: &str) -> Vec<(String, String)> {
        let prefix = format!("r/{kind}/");
        self.kv
            .keys()
            .into_iter()
            .filter(|k| k.starts_with(&prefix))
            .filter_map(|k| {
                let id = k[prefix.len()..].to_string();
                self.kv.get(&k).and_then(|v| String::from_utf8(v).ok()).map(|j| (id, j))
            })
            .collect()
    }

    /// Everything an index prefix covers, in key order — the composite index.
    ///
    /// `scan("member/dubin-durres/")` is `WHERE location_id = ?`, and it is the
    /// venue filter that cannot be forgotten because it is in the key.
    pub fn scan(&self, prefix: &str) -> Vec<(String, String)> {
        let p = idx_key(prefix);
        self.kv
            .keys()
            .into_iter()
            .filter(|k| k.starts_with(&p))
            .filter_map(|k| {
                let rest = k[2..].to_string();
                self.kv.get(&k).and_then(|v| String::from_utf8(v).ok()).map(|id| (rest, id))
            })
            .collect()
    }

    /// One index entry. `lookup("user.email/a@b.c")` is the unique read.
    pub fn lookup(&self, key: &str) -> Option<String> {
        self.kv.get(&idx_key(key)).and_then(|v| String::from_utf8(v).ok())
    }

    /// Write a record and exactly the index keys it owns.
    ///
    /// `unique` names the subset of `index` that no other record may hold. The
    /// check is against the CURRENT holder, so re-saving a record under its own
    /// unique key is allowed and re-using someone else's is refused -- the two
    /// halves an `INSERT ... ON CONFLICT` had to be read carefully to tell
    /// apart.
    pub fn put(
        &mut self,
        kind: &str,
        id: &str,
        json: &str,
        index: &[(String, String)],
        unique: &[&str],
    ) -> Result<(), TableError> {
        for u in unique {
            if let Some((k, _)) = index.iter().find(|(k, _)| k == u) {
                if let Some(held) = self.lookup(k) {
                    if held != id {
                        return Err(TableError::Taken { key: k.clone(), held_by: held });
                    }
                }
            }
        }
        // THE OLD KEYS GO FIRST. A record that used to be findable by an email
        // it no longer has must stop being findable by it, and the only thing
        // that knows the old email is the old record's own list.
        self.drop_index(kind, id);
        self.kv.put(&rec_key(kind, id), json.as_bytes());
        let mut owned = Vec::with_capacity(index.len());
        for (k, v) in index {
            self.kv.put(&idx_key(k), v.as_bytes());
            owned.push(k.clone());
        }
        self.kv.put(&own_key(kind, id), owned.join("\n").as_bytes());
        Ok(())
    }

    /// The record and every key that found it.
    pub fn remove(&mut self, kind: &str, id: &str) -> bool {
        self.drop_index(kind, id);
        self.kv.remove(&own_key(kind, id));
        self.kv.remove(&rec_key(kind, id))
    }

    fn drop_index(&mut self, kind: &str, id: &str) {
        let owned = self
            .kv
            .get(&own_key(kind, id))
            .and_then(|v| String::from_utf8(v).ok())
            .unwrap_or_default();
        for k in owned.split('\n').filter(|k| !k.is_empty()) {
            self.kv.remove(&idx_key(k));
        }
    }

    // ── the conservation audit, applied to storage ─────────────────────────

    /// Recompute every index entry from the records alone.
    ///
    /// AN INDEX MUST BE DERIVABLE, and this is the proof. `f` is the same
    /// function the writers use, so a gate that rebuilds and asserts the root
    /// did not move is asserting that no write anywhere has left the index and
    /// the records disagreeing. That is `stranded()` for storage.
    pub fn rebuild_index<F>(&mut self, kinds: &[&str], f: F)
    where
        F: Fn(&str, &str, &str) -> Vec<(String, String)>,
    {
        for k in self.kv.keys() {
            if k.starts_with("x/") || k.starts_with("i/") {
                self.kv.remove(&k);
            }
        }
        for kind in kinds {
            for (id, json) in self.all(kind) {
                let index = f(kind, &id, &json);
                let mut owned = Vec::with_capacity(index.len());
                for (key, v) in &index {
                    self.kv.put(&idx_key(key), v.as_bytes());
                    owned.push(key.clone());
                }
                self.kv.put(&own_key(kind, &id), owned.join("\n").as_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests;
