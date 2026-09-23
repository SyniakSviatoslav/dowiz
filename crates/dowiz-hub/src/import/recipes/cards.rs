//! The RECIPES file: tech cards, dated versions, semi-finished products.
//!
//! A SEMI-FINISHED PRODUCT IS FLATTENED HERE, to the leaf supplies, scaled by
//! its own batch size — iiko's `getPrepared`, done in exact fractions. The
//! hub's `bom` is flat by design (`stock::bom_of`); a sauce imported as a
//! supply named "sauce" would reserve a thing nobody ever restocks. Unlike
//! `getPrepared`, a leaf that does not come to a whole base unit is REFUSED,
//! not rounded to the gram.

use super::num::{self, Base, Rat};
use super::flatten::flatten;
use super::{DraftRecipe, Opts, RecipeDraft, Sheet, QTY_MAX};
use crate::import::{slug, truthy};

const DAY_MS: i64 = 86_400_000;

fn truthy_cell(s: &str) -> bool {
    !s.trim().is_empty() && truthy(s)
}

pub(super) enum Item {
    Supply { id: String, base: Base, qty: Rat, net: Option<i64>, yld: Option<i64> },
    Pre { key: String, qty: Rat },
}

pub(super) struct Card {
    pub(super) key: String,
    pub(super) dish: String,
    pub(super) first_row: usize,
    pub(super) from: Option<i64>,
    pub(super) to: Option<i64>,
    pub(super) in_force: bool,
    pub(super) prepack: bool,
    /// The batch a prepack's lines make, in its base unit.
    pub(super) batch: Option<(Base, Rat)>,
    pub(super) items: Vec<Item>,
    pub(super) errors: Vec<String>,
}

/// A quantity cell as an exact amount of `base`, the unit taken from the cell
/// itself, then the unit column, then the caller's hint.
fn amount(cell: &str, unit_col: &str, hint: Option<&str>) -> Result<(Base, Rat), String> {
    let (n, in_cell) = num::split_unit(cell);
    let word = in_cell.or(if unit_col.is_empty() { None } else { Some(unit_col) }).or(hint);
    let Some(word) = word else {
        return Err(format!("{cell:?} has no unit; say which unit the quantities are in"));
    };
    let Some((base, per)) = num::unit_of(word) else {
        return Err(format!("unit {word:?} is not g, kg, ml, l or pieces"));
    };
    Ok((base, Rat::of(num::parse_decimal(n)?, per)))
}

pub(super) fn read(text: &str, opts: &Opts, draft: &mut RecipeDraft) {
    let Some(sheet) = Sheet::parse(text) else { return };
    if !sheet.has("dish") || !sheet.has("ingredient") || !(sheet.has("qty") || sheet.has("gross")) {
        draft.warnings.push("the recipes file needs dish, ingredient and qty (or gross) columns".into());
        return;
    }
    // ── rows into cards, one per (dish, from, to) ──
    let mut cards: Vec<Card> = Vec::new();
    let mut owner: Vec<Option<usize>> = Vec::with_capacity(sheet.rows.len());
    for (row, cells) in &sheet.rows {
        let dish = sheet.get(cells, "dish");
        if dish.is_empty() {
            draft.warnings.push(format!("recipes row {row}: no dish, skipped"));
            owner.push(None);
            continue;
        }
        let mut errors = Vec::new();
        let mut day = |k: &str| match sheet.get(cells, k) {
            "" => None,
            raw => num::parse_day(raw).map_err(|e| errors.push(format!("row {row}: {e}"))).ok(),
        };
        let (from, to) = (day("from"), day("to"));
        let key = slug(dish);
        let at = match cards.iter().position(|c| c.key == key && c.from == from && c.to == to) {
            Some(i) => i,
            None => {
                cards.push(Card { key, dish: dish.into(), first_row: *row, from, to, in_force: true, prepack: false, batch: None, items: vec![], errors: vec![] });
                cards.len() - 1
            }
        };
        owner.push(Some(at));
        let card = &mut cards[at];
        card.errors.extend(errors);
        card.prepack |= truthy_cell(sheet.get(cells, "prepack"));
        let batch = sheet.get(cells, "batch");
        if !batch.is_empty() && card.batch.is_none() {
            card.prepack = true;
            match amount(batch, "", opts.unit_hint) {
                Ok((b, r)) if r.num > 0 => card.batch = Some((b, r)),
                Ok(_) => card.errors.push(format!("row {row}: batch is zero")),
                Err(e) => card.errors.push(format!("row {row}: batch {e}")),
            }
        }
    }

    // ── the card in force today, and only that one ──
    let today = opts.local_now_ms.div_euclid(DAY_MS);
    for c in cards.iter_mut() {
        c.in_force = c.from.is_none_or(|f| f <= today) && c.to.is_none_or(|t| today < t);
    }
    let stale = cards.iter().filter(|c| !c.in_force).count();
    if stale > 0 {
        draft.warnings.push(format!(
            "{stale} dated card(s) not in force today were not imported; only today's card is"
        ));
    }
    let mut twice: Vec<String> = Vec::new();
    for c in cards.iter().filter(|c| c.in_force) {
        if cards.iter().filter(|o| o.in_force && o.key == c.key).count() > 1 && !twice.contains(&c.key) {
            twice.push(c.key.clone());
        }
    }

    // ── each row's line, now that every card's name is known ──
    let prepacks: Vec<(String, Option<Base>)> = cards
        .iter()
        .filter(|c| c.in_force && c.prepack)
        .map(|c| (c.key.clone(), c.batch.map(|b| b.0)))
        .collect();
    let dishes: Vec<String> = cards.iter().filter(|c| c.in_force).map(|c| c.key.clone()).collect();
    for ((row, cells), at) in sheet.rows.iter().zip(owner) {
        let Some(at) = at.filter(|i| cards[*i].in_force) else { continue };
        match line(&sheet, cells, *row, opts, draft, &prepacks, &dishes) {
            Ok(item) => cards[at].items.push(item),
            Err(e) => cards[at].errors.push(format!("row {row}: {e}")),
        }
    }
    cards.retain(|c| c.in_force);

    // ── flatten, check, and hand each dish its bom ──
    for c in cards.iter() {
        let products: Vec<&(String, String)> = opts
            .products
            .iter()
            .filter(|(id, name)| *id == c.dish || *id == c.key || slug(name) == c.key)
            .collect();
        let pid = match products.as_slice() {
            [one] => one.0.clone(),
            [] => {
                if !c.prepack {
                    draft.warnings.push(format!("recipes row {}: dish {:?} is not on the menu; import the menu first", c.first_row, c.dish));
                }
                continue;
            }
            many => {
                let ids: Vec<&str> = many.iter().map(|p| p.0.as_str()).collect();
                draft.warnings.push(format!("recipes row {}: {:?} names {} dishes ({}); give the card an id", c.first_row, c.dish, ids.len(), ids.join(", ")));
                continue;
            }
        };
        let refuse = |draft: &mut RecipeDraft, why: &str| {
            draft.warnings.push(format!("recipes, {}: {why}; the dish gets no recipe from this file", c.dish));
            if !draft.without_recipe.contains(&pid) {
                draft.without_recipe.push(pid.clone());
            }
        };
        if twice.contains(&c.key) {
            refuse(draft, "more than one card is in force today");
            continue;
        }
        let mut report = Vec::new();
        match flatten(c, &cards, &mut report) {
            Ok(lines) => {
                for r in report {
                    draft.flattened.push(format!("{}: {r}", c.dish));
                }
                draft.recipes.push(DraftRecipe { product_id: pid, dish: c.dish.clone(), lines });
            }
            Err(why) => refuse(draft, &why),
        }
    }
}

/// One row as a line: a supply in its base unit, or a use of a prepack.
fn line(
    sheet: &Sheet,
    cells: &[String],
    row: usize,
    opts: &Opts,
    draft: &mut RecipeDraft,
    prepacks: &[(String, Option<Base>)],
    dishes: &[String],
) -> Result<Item, String> {
    let name = sheet.get(cells, "ingredient");
    if name.is_empty() {
        return Err("no ingredient".into());
    }
    // GROSS is the quantity kept: it is what the incumbents write off.
    let cell = match (sheet.get(cells, "gross"), sheet.get(cells, "qty")) {
        ("", "") if !sheet.get(cells, "net").is_empty() => {
            return Err("only a net quantity; the reserved quantity is gross".into())
        }
        ("", "") => return Err("no quantity".into()),
        ("", q) => q,
        (g, _) => g,
    };
    let unit_col = sheet.get(cells, "unit");
    let (base, qty) = amount(cell, unit_col, opts.unit_hint)?;
    let key = slug(name);
    let supply = draft.supplies.iter().find(|s| s.id == key || slug(&s.name) == key);
    if let Some((_, pbase)) = prepacks.iter().find(|p| p.0 == key) {
        if supply.is_some() {
            return Err(format!("{name:?} is both a supply and a semi-finished product"));
        }
        let Some(pbase) = pbase else { return Err(format!("semi-finished {name:?} has no readable batch size")) };
        if *pbase != base {
            return Err(format!("{name:?} is counted in {}, not {}", pbase.as_str(), base.as_str()));
        }
        return Ok(Item::Pre { key, qty });
    }
    let Some(s) = supply else {
        if dishes.contains(&key) {
            return Err(format!("{name:?} is a dish in this file with no batch size; a semi-finished product needs one"));
        }
        return Err(format!("{name:?} is not in the ingredients file"));
    };
    if s.unit != base.as_str() {
        return Err(format!("{name:?} is counted in {}, this line is in {}", s.unit, base.as_str()));
    }
    // Net and yield are kept, not used: a value that does not read is dropped
    // with a warning, and the line (whose gross read) stands.
    let mut side = |k: &str| match sheet.get(cells, k) {
        "" => None,
        raw => match amount(raw, unit_col, opts.unit_hint).and_then(|(_, r)| r.whole(base, QTY_MAX)) {
            Ok(v) => Some(v),
            Err(e) => {
                draft.warnings.push(format!("recipes row {row}: {k} {raw:?} {e}; not kept"));
                None
            }
        },
    };
    let (net, yld) = (side("net"), side("yield"));
    Ok(Item::Supply { id: s.id.clone(), base, qty, net, yld })
}
