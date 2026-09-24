//! LINK, NEVER MERGE (§3.4 of BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22).
//!
//! PURE. The `alias` kind in the venue's `people` image: id = a customer key
//! that is NOT canonical, value `{canonical, by: "rule"|"owner", at_ms,
//! reason}`. Nothing is ever merged -- the orders, the card, the consent acts
//! and the wallet stay filed under the key they were filed under -- so
//! UNLINKING IS `remove("alias", id)` and the two rows come back.
//!
//! `canonical` is the key the link was made TO, which may itself be an alias:
//! `resolve` follows the chain, so undoing one link undoes exactly that one.
//! A chain longer than `MAX_HOPS`, or a cycle, is refused at write time, and
//! `resolve` stops at `MAX_HOPS` so a corrupted image cannot spin a request.

use std::collections::BTreeMap;

use dowiz_hub::table::Table;
use serde_json::{json, Value};

/// The record kind in the venue's `people` image.
pub const KIND: &str = "alias";

/// The longest chain `resolve` follows and `link` will create.
pub const MAX_HOPS: usize = 8;

/// Who wrote the link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    /// `identity::canonical_digits`, at placement.
    Rule,
    /// The owner, from the console, audited as `Linked`.
    Owner,
}

impl By {
    pub fn as_str(self) -> &'static str {
        match self {
            By::Rule => "rule",
            By::Owner => "owner",
        }
    }
}

/// Every alias in the image, `from -> canonical`, read once per request.
#[derive(Debug, Default, Clone)]
pub struct Aliases(BTreeMap<String, String>);

impl Aliases {
    pub fn of(t: &Table) -> Self {
        Aliases(
            t.all(KIND)
                .into_iter()
                .filter_map(|(id, j)| Some((id, target_of(&j)?)))
                .collect(),
        )
    }

    /// The key `key` is shown under: itself when it is linked to nothing.
    pub fn resolve(&self, key: &str) -> String {
        let mut at = key;
        for _ in 0..MAX_HOPS {
            match self.0.get(at) {
                Some(next) if next != key => at = next,
                _ => break,
            }
        }
        at.to_string()
    }

    /// The keys linked INTO `canonical` (not `canonical` itself), sorted.
    pub fn members(&self, canonical: &str) -> Vec<String> {
        self.0.keys().filter(|k| *k != canonical && self.resolve(k) == canonical).cloned().collect()
    }

    /// Every key whose CARD speaks for the person placing under `key`: the
    /// row it is shown under and every key linked into that row. `pending` is
    /// the rule's link this very placement is about to write (a spelling's
    /// first order), so the E.164 card's allergies bind it already.
    pub fn circle(&self, key: &str, pending: Option<&str>) -> Vec<String> {
        let canonical = match pending {
            Some(to) if !self.0.contains_key(key) => self.resolve(to),
            _ => self.resolve(key),
        };
        let mut out = vec![canonical.clone()];
        out.extend(self.members(&canonical));
        if !out.iter().any(|k| k == key) {
            out.push(key.to_string());
        }
        out
    }

    fn hops(&self, key: &str) -> usize {
        let (mut at, mut n) = (key, 0);
        while let Some(next) = self.0.get(at) {
            n += 1;
            if n > MAX_HOPS {
                break;
            }
            at = next;
        }
        n
    }
}

fn target_of(json_text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(json_text).ok()?;
    v.get("canonical")?.as_str().filter(|s| !s.is_empty()).map(str::to_string)
}

/// ONLY AN OWNER LINKS PEOPLE. Staff, couriers and customers are refused
/// before the venue is even asked; `owner_and_venue` then proves the owner
/// owns THIS venue.
pub fn may_link(c: &crate::auth::Claims) -> bool {
    matches!(c, crate::auth::Claims::Owner { .. })
}

/// Whether the owner may link `from` to `to`. `Err` is the refusal, answered
/// 409 and written nowhere.
pub fn check_link(t: &Table, from: &str, to: &str) -> Result<(), String> {
    if from == to {
        return Err("a customer cannot be linked to themselves".into());
    }
    if t.has(KIND, from) {
        return Err("this customer is already linked; unlink them first".into());
    }
    let all = Aliases::of(t);
    if all.resolve(to) == from {
        return Err("these two are already one customer".into());
    }
    if all.hops(to) >= MAX_HOPS {
        return Err(format!("a chain of links is at most {MAX_HOPS} long"));
    }
    Ok(())
}

/// Write the link `from -> to`, after `check_link`.
pub fn link(t: &mut Table, from: &str, to: &str, by: By, reason: &str, at_ms: i64) -> Result<(), String> {
    check_link(t, from, to)?;
    let v = json!({ "canonical": to, "by": by.as_str(), "at_ms": at_ms, "reason": reason });
    t.put(KIND, from, &v.to_string(), &[], &[]).map_err(|e| format!("alias: {e:?}"))
}

/// THE RULE, AT PLACEMENT: link a second spelling at the moment it FIRST
/// appears -- when no card has been filed under `from` yet. A spelling that
/// has ordered before is not a first appearance, so an owner's unlink is not
/// undone by that customer's next order. Never overwrites a link. True when a
/// link was written.
pub fn rule_link(t: &mut Table, from: &str, to: &str, at_ms: i64) -> bool {
    if t.has(super::record::KIND, from) {
        return false;
    }
    link(t, from, to, By::Rule, "same number, another spelling", at_ms).is_ok()
}

/// THE ALLERGIES OF A LINKED PERSON ARE THE UNION of their cards'. Link,
/// never merge, applies to what is FILED; what the kitchen must not serve is
/// read across every spelling, or an allergy typed on one card is lost the
/// day the customer types their number the other way.
pub fn allergens_of(t: &Table, key: &str, pending: Option<&str>) -> Vec<String> {
    let mut all: Vec<String> = Vec::new();
    for k in Aliases::of(t).circle(key, pending) {
        for a in super::allergy::of_record(t.get(super::record::KIND, &k).as_deref()) {
            if !all.contains(&a) {
                all.push(a);
            }
        }
    }
    all
}

/// Remove the link `from` was made with. True when there was one.
pub fn unlink(t: &mut Table, from: &str) -> bool {
    t.remove(KIND, from)
}

/// The audit entries of an owner's link or unlink: one per person, subject
/// `cust:<key>`, shaped as a `Revealed` payload (`by`, `at`, `reason`) plus
/// what was done and to whom. LINKING TWO PEOPLE IS A REVEAL OF BOTH.
pub fn audit(act: &str, who: &str, at: i64, reason: &str, from: &str, to: &str) -> [(String, String); 2] {
    let one = |me: &str, other: &str| {
        let body = json!({ "by": who, "at": at, "reason": reason, "act": act, "with": other });
        (format!("cust:{me}"), body.to_string())
    };
    [one(from, to), one(to, from)]
}

#[cfg(test)]
mod tests;
