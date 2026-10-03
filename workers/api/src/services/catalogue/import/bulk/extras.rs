//! THE SUPPLIES FILE'S EXTRA COLUMNS (W-STOCK P1): what the console's form
//! holds and the hub's ingredients parser does not read -- the losses on
//! cleaning and cooking, and the pack a supply is bought in.
//!
//!   clean_pm   per mille of the gross left after cleaning (1000 = none lost)
//!   cook_pm    per mille of the net after cooking (above 1000 when it grows: rice)
//!   pack       the pack's name ("box", "thes 5 kg")
//!   pack_qty   what the pack holds, in the supply's base unit (g, ml or pieces)
//!
//! The hub's parser ignores a column it does not know, so a file with these
//! reads exactly as before there; this reads them beside it, keyed by the row's
//! supply id (its `id` cell, else its name as the hub slugs it). A value that
//! is not one is a warning naming the row and is left out -- never a guess and
//! never a refusal of the file. PURE.

use std::collections::BTreeMap;

use crate::recipe::weights::{CLEAN_MAX, COOK_MAX};
use crate::services::operations::supplies::nomenclature::{Pack, PACK_NAME_MAX, PACK_QTY_MAX};

/// One row's extras.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extra {
    pub clean_pm: Option<i64>,
    pub cook_pm: Option<i64>,
    pub pack: Option<Pack>,
}

pub type Extras = BTreeMap<String, Extra>;

/// The separator a header line uses: the most frequent of `;`, tab and `,`.
fn separator(header: &str) -> char {
    [';', '\t', ','].into_iter().max_by_key(|c| (header.matches(*c).count(), *c == ',')).unwrap_or(',')
}

/// One CSV line into cells, double quotes honoured ("a, b" is one cell, "" a quote).
pub fn cells(line: &str, sep: char) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if quoted && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' => quoted = !quoted,
            c if c == sep && !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn whole(raw: &str) -> Option<i64> {
    raw.trim().parse::<i64>().ok()
}

/// Every row's extras, and a warning for each value left out.
pub fn read(text: &str) -> (Extras, Vec<String>) {
    let (mut out, mut warn) = (Extras::new(), Vec::new());
    let mut lines = text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty());
    let Some((_, header)) = lines.next() else { return (out, warn) };
    let sep = separator(header);
    let head: Vec<String> = cells(header.trim_start_matches('\u{feff}'), sep).into_iter().map(|h| h.to_lowercase()).collect();
    let col = |names: &[&str]| head.iter().position(|h| names.contains(&h.as_str()));
    let (id_c, name_c) = (col(&["id", "code", "kod", "kodi", "код", "артикул"]), col(&["name", "emri", "назва", "название"]));
    let (clean_c, cook_c, pack_c, qty_c) = (col(&["clean_pm"]), col(&["cook_pm"]), col(&["pack"]), col(&["pack_qty"]));
    if clean_c.is_none() && cook_c.is_none() && qty_c.is_none() {
        return (out, warn);
    }
    for (i, line) in lines {
        let row = i + 1;
        let c = cells(line, sep);
        let at = |k: Option<usize>| k.and_then(|k| c.get(k)).map(String::as_str).unwrap_or("");
        let key = match at(id_c) {
            "" => dowiz_hub::import::slug(at(name_c)),
            id => dowiz_hub::import::slug(id),
        };
        if key.is_empty() {
            continue;
        }
        let mut x = Extra::default();
        let mut pm = |col: Option<usize>, max: i64, word: &str| match at(col) {
            "" => None,
            raw => match whole(raw).filter(|v| (1..=max).contains(v)) {
                Some(v) => Some(v),
                None => {
                    warn.push(format!("ingredients row {row}: {word} {raw:?} is not 1 to {max} per mille; left out"));
                    None
                }
            },
        };
        x.clean_pm = pm(clean_c, CLEAN_MAX, "clean_pm");
        x.cook_pm = pm(cook_c, COOK_MAX, "cook_pm");
        if let Some(raw) = Some(at(qty_c)).filter(|s| !s.is_empty()) {
            let name = match at(pack_c) {
                "" => "pack",
                n => n,
            };
            match whole(raw).filter(|q| (1..=PACK_QTY_MAX).contains(q)) {
                Some(qty) if name.chars().count() <= PACK_NAME_MAX => x.pack = Some(Pack { name: name.to_string(), qty }),
                _ => warn.push(format!("ingredients row {row}: pack {name:?} of {raw:?} is not 1 to {PACK_QTY_MAX} of the base unit; left out")),
            }
        }
        if x != Extra::default() {
            out.insert(key, x);
        }
    }
    (out, warn)
}

#[cfg(test)]
#[path = "extras/tests.rs"]
mod tests;
