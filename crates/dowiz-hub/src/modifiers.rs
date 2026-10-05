//! What a customer may change about a dish, and what it costs.
//!
//! THE HUB PRICES, THE CLIENT ASKS. A basket arrives carrying option ids and
//! nothing else -- no prices, no totals. Everything is looked up in the
//! catalogue and summed here, for the same reason the base price is: a client
//! that can name its own price will eventually be asked to.
//!
//! THE RULES ARE THE VENUE'S, ENFORCED HERE. "Choose a size" is required and
//! takes exactly one; "extras" takes up to three; "no onion" takes any number.
//! A basket that breaks one of those is refused with the group named, because
//! a kitchen that receives a roll with two sizes selected has to phone the
//! customer, and a kitchen that receives one with no size has to guess.

use crate::minijson::{int_field, str_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Option_ {
    pub id: String,
    pub name: String,
    /// Added to the line's unit price, in minor units. May be NEGATIVE -- "no
    /// avocado, minus fifty" is a real thing a venue offers.
    pub price_delta: i64,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub name: String,
    /// Fewest that must be chosen. `min >= 1` is what "required" means; there
    /// is no separate flag, because two ways to say the same thing drift.
    pub min: i64,
    /// Most that may be chosen. Zero means no ceiling.
    pub max: i64,
    pub options: Vec<Option_>,
}

impl Group {
    pub fn required(&self) -> bool {
        self.min >= 1
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ModError {
    /// An option id that is not on this dish. Either a stale menu in the
    /// customer's browser or somebody trying it on; the answer is the same.
    Unknown { option: String },
    /// A group that must be chosen from, and was not.
    Missing { group: String, need: i64 },
    /// Too many from one group.
    TooMany { group: String, max: i64, got: i64 },
    /// Too few, where the group is chosen from at all.
    TooFew { group: String, min: i64, got: i64 },
    /// The option exists and the kitchen has run out of it.
    Unavailable { option: String },
}

impl std::fmt::Display for ModError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModError::Unknown { option } => write!(f, "{option} is not an option on this dish"),
            ModError::Missing { group, need } => {
                write!(f, "choose {need} from \"{group}\"")
            }
            ModError::TooMany { group, max, got } => {
                write!(f, "\"{group}\" takes at most {max}, {got} chosen")
            }
            ModError::TooFew { group, min, got } => {
                write!(f, "\"{group}\" takes at least {min}, {got} chosen")
            }
            ModError::Unavailable { option } => write!(f, "{option} has run out"),
        }
    }
}

/// Read a dish's modifier groups out of its catalogue record.
///
/// Hand-parsed like everything else in this crate, and tolerant: a dish with no
/// groups is the normal case, and a malformed group is skipped rather than
/// taking the menu down. A venue that cannot sell anything because one option
/// has a typo in it is worse off than one selling a dish without its extras.
pub fn groups_of(product_json: &str) -> Vec<Group> {
    let mut out = Vec::new();
    let Some(start) = product_json.find("\"modifierGroups\"") else { return out };
    let rest = &product_json[start..];
    let Some(open) = rest.find('[') else { return out };

    // Walk to the matching close bracket rather than the first one: the groups
    // array contains options arrays, and stopping at the first `]` would read
    // only the first group's options.
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut end = open;
    let mut in_str = false;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b'[' if !in_str => depth += 1,
            b']' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if end <= open {
        return out;
    }
    let body = &rest[open + 1..end];

    // Each group is an object with its own nested options array.
    for gchunk in split_objects(body) {
        // The group's OWN fields are the ones before its options array. Reading
        // the whole chunk let a group with no id silently adopt its first
        // option's -- so two groups could end up sharing one id, and the rule
        // checks would then apply to the wrong one.
        let head = match gchunk.find("\"options\"") {
            Some(i) => &gchunk[..i],
            None => gchunk.as_str(),
        };
        let Some(id) = str_field(head, "id") else { continue };
        let name = str_field(head, "name").unwrap_or_else(|| id.clone());
        let min = int_field(head, "min").unwrap_or(0).max(0);
        let max = int_field(head, "max").unwrap_or(0).max(0);
        let mut options = Vec::new();
        if let Some(opos) = gchunk.find("\"options\"") {
            let tail = &gchunk[opos..];
            if let (Some(o), Some(c)) = (tail.find('['), tail.rfind(']')) {
                if c > o {
                    for ochunk in split_objects(&tail[o + 1..c]) {
                        let Some(oid) = str_field(&ochunk, "id") else { continue };
                        options.push(Option_ {
                            name: str_field(&ochunk, "name").unwrap_or_else(|| oid.clone()),
                            price_delta: int_field(&ochunk, "priceDelta").unwrap_or(0),
                            available: int_field(&ochunk, "available").unwrap_or(1) != 0
                                && !ochunk.contains("\"available\":false"),
                            id: oid,
                        });
                    }
                }
            }
        }
        if !options.is_empty() {
            out.push(Group { id, name, min, max, options });
        }
    }
    out
}

/// The body of the JSON ARRAY that `"key"` holds -- the text between its
/// brackets -- or `None` when the key is absent or holds anything else.
///
/// STRING-AWARE, and that is the whole point (W-AUDIT S7, 2026-09-27).
/// `bom_of` found `"bom"`, took the first `[` after it and the first `]`
/// after that: `"bom":null,"ingredients":[...]` read the NEXT array in the
/// record, and a `]` inside any supply name cut the recipe short, so the
/// lines after it were never reserved. This walks brackets outside strings
/// only, as `groups_of` always has.
pub(crate) fn array_of<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\"");
    let mut from = 0usize;
    let open = loop {
        let at = json[from..].find(&pat)? + from;
        let after = json[at + pat.len()..].trim_start();
        // `"bom" :` with anything but `[` behind the colon is not an array.
        let Some(v) = after.strip_prefix(':') else { from = at + pat.len(); continue };
        let v = v.trim_start();
        if !v.starts_with('[') {
            from = at + pat.len();
            continue;
        }
        break json.len() - v.len();
    };
    let bytes = json.as_bytes();
    let (mut depth, mut in_str, mut i) = (0i32, false, open);
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b'[' if !in_str => depth += 1,
            b']' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return Some(&json[open + 1..i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Split a JSON array body into its top-level object chunks.
pub(crate) fn split_objects(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let (mut depth, mut start, mut in_str) = (0i32, 0usize, false);
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b'{' if !in_str => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            b'}' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    out.push(body[start..=i].to_string());
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// What one line's chosen options cost, after checking they are allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Priced {
    /// Sum of the chosen deltas. Added to the base price.
    pub delta: i64,
    /// The chosen options, named, so the kitchen ticket reads in words.
    pub chosen: Vec<(String, String, i64)>,
}

/// Validate a selection against a dish's groups and price it.
///
/// ORDER OF CHECKS MATTERS. Unknown ids are caught first: an id that is not on
/// the dish cannot be counted toward a group's minimum, and reporting "choose
/// one size" for a basket that named a size from a different dish would send
/// the customer looking in the wrong place.
pub fn price(groups: &[Group], chosen_ids: &[String]) -> Result<Priced, ModError> {
    // Every chosen id must exist on this dish.
    for id in chosen_ids {
        if !groups.iter().any(|g| g.options.iter().any(|o| &o.id == id)) {
            return Err(ModError::Unknown { option: id.clone() });
        }
    }

    let mut delta = 0i64;
    let mut chosen = Vec::new();

    for g in groups {
        let picked: Vec<&Option_> = g
            .options
            .iter()
            .filter(|o| chosen_ids.iter().any(|c| c == &o.id))
            .collect();
        let n = picked.len() as i64;

        if n == 0 {
            if g.required() {
                return Err(ModError::Missing { group: g.name.clone(), need: g.min });
            }
            continue;
        }
        if g.max > 0 && n > g.max {
            return Err(ModError::TooMany { group: g.name.clone(), max: g.max, got: n });
        }
        // A minimum applies once the group is touched at all: "pick two sauces
        // or none" is a real rule, and a group with min 2 chosen once is wrong
        // whether or not it was required.
        if n < g.min {
            return Err(ModError::TooFew { group: g.name.clone(), min: g.min, got: n });
        }
        for o in picked {
            if !o.available {
                return Err(ModError::Unavailable { option: o.name.clone() });
            }
            delta = delta.saturating_add(o.price_delta);
            chosen.push((o.id.clone(), o.name.clone(), o.price_delta));
        }
    }
    Ok(Priced { delta, chosen })
}

#[cfg(test)]
mod tests;

/// Options that draw stock: `optionBom` on the dish (R13, W-LOST).
pub mod bom;
