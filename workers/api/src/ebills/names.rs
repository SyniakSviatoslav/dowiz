//! A TILL LINE MATCHED TO A DISH BY ITS NAME, for the write-off (operator
//! 2026-11-03: "списання можеш рахувати суто по назвам страв з ebills і
//! їхнім відповідникам у сервісі з техкартами").
//!
//! THE RULE. A till line whose code the owner has NOT mapped (`K_MAP`, which
//! still wins when set) is written off through the catalogue dish whose name
//! -- in ANY language the venue wrote (`hubstore::i18n_key`, `<locale>/product/
//! <id>/name`) or its own -- normalises to the same key as the line's. Exactly
//! one dish, or none: two dishes under one name is an ambiguity the owner
//! settles in the mapping sheet, never a guess about which salmon to draw.
//!
//! PURE AND DETERMINISTIC: the catalogue's `(id, json)` rows and the
//! translation table's `(key, value)` rows in, a sorted index out. `norm`
//! (`state.rs`) is the one spelling: case, whitespace, punctuation and the
//! Albanian diacritics (ë, ç) fold; nothing else does.
//!
//! NOT RETROACTIVE, BY DECISION (2026-11-03). The match is made at import,
//! once, like the owner's mapping: a sale imported while its name had no
//! dish keeps `ebills:<code>`, drew nothing, and is never re-drawn when a
//! dish of that name appears later -- `import::decide` keeps a sale once by
//! its uuid, and a ledger that rewrites last week's draws because the menu
//! changed today would make every stock count a function of the present.
//! The tab names those lines (`sales.rs`, `unmatched`) so the owner sees
//! what was counted as revenue and written off nowhere.

use super::state::norm;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Normalised name -> the product ids that carry it (sorted, deduplicated).
pub(crate) type Index = BTreeMap<String, BTreeSet<String>>;

/// What a name resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Match {
    One(String),
    None,
    /// `n` dishes share the name: not written off, shown to the owner.
    Ambiguous(usize),
}

/// The index over every name every dish has: its own `name`, then every
/// `<locale>/<type>/<id>/name` row of the translation table that names it.
pub(crate) fn index(products: &[(String, String)], i18n: &[(String, String)]) -> Index {
    let mut idx = Index::new();
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for (id, json) in products {
        ids.insert(id);
        let own = serde_json::from_str::<Value>(json).ok().and_then(|p| p.get("name").and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
        add(&mut idx, id, &own);
    }
    for (k, v) in i18n {
        let mut parts = k.splitn(4, '/');
        let (Some(_), Some(_), Some(eid), Some("name")) = (parts.next(), parts.next(), parts.next(), parts.next()) else { continue };
        if ids.contains(eid) {
            add(&mut idx, eid, v);
        }
    }
    idx
}

fn add(idx: &mut Index, id: &str, name: &str) {
    let key = norm(name);
    if !key.is_empty() {
        idx.entry(key).or_default().insert(id.to_string());
    }
}

pub(crate) fn how(idx: &Index, name: &str) -> Match {
    let key = norm(name);
    match idx.get(&key).filter(|_| !key.is_empty()) {
        None => Match::None,
        Some(ids) if ids.len() == 1 => Match::One(ids.iter().next().cloned().unwrap_or_default()),
        Some(ids) => Match::Ambiguous(ids.len()),
    }
}

/// The one dish this name is, or `None` (unknown or ambiguous alike).
pub(crate) fn resolve(idx: &Index, name: &str) -> Option<String> {
    match how(idx, name) {
        Match::One(id) => Some(id),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
