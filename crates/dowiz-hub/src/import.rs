//! Turning a file the owner already has into a menu.
//!
//! THE PROBLEM THIS SOLVES. Until now a hub got its catalogue from a
//! hand-written JSON bundle applied over a shell. That is fine for the first
//! venue, where the operator does it, and it is the whole onboarding story for
//! every venue after — which is to say there isn't one. A restaurant has its
//! menu in a spreadsheet. This reads that.
//!
//! PARSING IS PURE AND LIVES HERE. No file I/O, no HTTP, no catalogue writes:
//! this takes text and returns a draft plus a list of everything it could not
//! understand. That separation is what lets the dangerous part — price parsing —
//! be tested exhaustively without a server.
//!
//! AMBIGUITY IS REFUSED, NOT GUESSED. A price this cannot read with certainty
//! becomes a warning naming the row and the text, and that row is left out. The
//! alternative is a menu that silently sells a dish at a hundredth of its price,
//! which nobody notices until the day's takings are counted.

use crate::minijson::esc;

pub mod recipes;

/// One dish, as read out of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftProduct {
    pub id: String,
    pub category_id: String,
    pub name: String,
    /// `None` WHEN THE FILE HAS NO DESCRIPTION COLUMN, which is not the same
    /// as an empty cell: a two-column `name,price` sheet says nothing about
    /// descriptions, and reading that as `""` blanked every dish's text on a
    /// price update (audit D5). The writer keeps the stored value on `None`.
    pub description: Option<String>,
    /// Integer minor units. See [`parse_price`].
    pub price: i64,
    /// `None` when the file has no Available column: a price sheet does not
    /// put a dish stopped for "keg empty" back on sale (audit D5).
    pub available: Option<bool>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftCategory {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
}

/// What a file turned into, and what it could not.
#[derive(Debug, Default, Clone)]
pub struct MenuDraft {
    pub categories: Vec<DraftCategory>,
    pub products: Vec<DraftProduct>,
    /// Rows that were NOT imported, each saying which row and why. Surfaced to
    /// the owner rather than logged: a row silently missing from their menu is
    /// a dish they cannot sell and will not find out about until a customer asks.
    pub warnings: Vec<String>,
    /// Did the file HAVE a category column? Without one every row lands in
    /// "Menu", and a writer that believed it would move every existing dish
    /// out of its own category on a price update (audit D5).
    pub category_column: bool,
}

impl MenuDraft {
    pub fn as_json(&self) -> String {
        let cats: Vec<String> = self
            .categories
            .iter()
            .map(|c| {
                format!(
                    r#"{{"id":"{}","name":"{}","sortOrder":{}}}"#,
                    esc(&c.id),
                    esc(&c.name),
                    c.sort_order
                )
            })
            .collect();
        let prods: Vec<String> = self
            .products
            .iter()
            .map(|p| {
                format!(
                    r#"{{"id":"{}","categoryId":"{}","name":"{}","description":{},"price":{},"available":{},"sortOrder":{}}}"#,
                    esc(&p.id),
                    esc(&p.category_id),
                    esc(&p.name),
                    p.description.as_deref().map_or("null".to_string(), |d| format!("\"{}\"", esc(d))),
                    p.price,
                    p.available.map_or("null".to_string(), |a| a.to_string()),
                    p.sort_order
                )
            })
            .collect();
        let warns: Vec<String> = self.warnings.iter().map(|w| format!("\"{}\"", esc(w))).collect();
        format!(
            r#"{{"categories":[{}],"products":[{}],"warnings":[{}]}}"#,
            cats.join(","),
            prods.join(","),
            warns.join(",")
        )
    }
}

/// A stable id from a human name.
///
/// DETERMINISTIC, because it is what makes importing the same file twice an
/// update rather than a duplication. Non-alphanumerics collapse to a single
/// dash so "Sake Futomaki" and "sake  futomaki!" are the same dish, which is
/// what a person editing a spreadsheet expects.
///
/// Non-ASCII is KEPT, lowercased. Dropping it would map every Albanian or
/// Ukrainian dish name onto the same empty slug and silently merge the menu into
/// one item — a failure mode worth more than the convenience of ASCII ids.
pub fn slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pending_dash = false;
    for c in s.trim().to_lowercase().chars() {
        if c.is_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() {
        // A name with nothing alphanumeric in it still needs an id, and an
        // empty one would collide with every other such name.
        return "item".into();
    }
    out
}

mod price;
pub use price::parse_price;

mod csv;
use csv::{column_of, detect_separator, split_csv_line, truthy};

/// Parse a CSV export into a menu draft.
pub fn from_csv(text: &str) -> MenuDraft {
    let mut draft = MenuDraft::default();
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let Some(header_line) = lines.next() else {
        draft.warnings.push("the file is empty".into());
        return draft;
    };
    let sep = detect_separator(header_line);
    let header: Vec<Option<&'static str>> =
        split_csv_line(header_line, sep).iter().map(|h| column_of(h)).collect();

    let idx = |key: &str| header.iter().position(|h| *h == Some(key));
    let (Some(i_name), Some(i_price)) = (idx("name"), idx("price")) else {
        draft.warnings.push(format!(
            "the header must contain a name column and a price column; found: {}",
            split_csv_line(header_line, sep).join(", ")
        ));
        return draft;
    };
    let (i_cat, i_desc, i_avail, i_id) =
        (idx("category"), idx("description"), idx("available"), idx("id"));
    draft.category_column = i_cat.is_some();

    let mut seen_categories: Vec<String> = Vec::new();
    // Row 1 is the header, so data starts at 2 -- the number the owner sees in
    // their spreadsheet, which is the only number a warning can usefully cite.
    for (n, line) in lines.enumerate() {
        let row = n + 2;
        let cells = split_csv_line(line, sep);
        let cell = |i: Option<usize>| i.and_then(|i| cells.get(i)).map(String::as_str).unwrap_or("");

        let name = cells.get(i_name).map(String::as_str).unwrap_or("").trim();
        if name.is_empty() {
            draft.warnings.push(format!("row {row}: no name, skipped"));
            continue;
        }
        let price = match parse_price(cells.get(i_price).map(String::as_str).unwrap_or("")) {
            Ok(p) => p,
            Err(why) => {
                draft.warnings.push(format!("row {row} ({name}): {why}"));
                continue;
            }
        };

        let cat_name = cell(i_cat);
        let cat_name = if cat_name.trim().is_empty() { "Menu" } else { cat_name.trim() };
        let cat_id = slug(cat_name);
        if !seen_categories.iter().any(|c| c == &cat_id) {
            seen_categories.push(cat_id.clone());
            draft.categories.push(DraftCategory {
                id: cat_id.clone(),
                name: cat_name.to_string(),
                // Categories keep the order they appear in the file. A
                // restaurant's menu is already ordered the way they want it
                // read; re-sorting it alphabetically would be us overruling them.
                sort_order: (seen_categories.len() - 1) as i64,
            });
        }

        let explicit_id = cell(i_id).trim();
        let id = if explicit_id.is_empty() {
            // Scoped by category so two categories may both have a "Special".
            format!("{cat_id}-{}", slug(name))
        } else {
            slug(explicit_id)
        };

        let n_in_cat = draft.products.iter().filter(|p| p.category_id == cat_id).count() as i64;
        draft.products.push(DraftProduct {
            id,
            category_id: cat_id,
            name: name.to_string(),
            description: i_desc.map(|_| cell(i_desc).trim().to_string()),
            price,
            available: i_avail.map(|_| truthy(cell(i_avail))),
            sort_order: n_in_cat,
        });
    }

    // Two rows naming the same dish in the same category would collapse into
    // one, with the last price winning. Silently. Say so instead.
    let mut ids: Vec<&str> = draft.products.iter().map(|p| p.id.as_str()).collect();
    ids.sort_unstable();
    let mut dups: Vec<String> = Vec::new();
    for w in ids.windows(2) {
        if w[0] == w[1] && !dups.iter().any(|d| d == w[0]) {
            dups.push(w[0].to_string());
        }
    }
    for d in dups {
        draft
            .warnings
            .push(format!("{d:?} appears more than once; the last row wins"));
    }

    if draft.products.is_empty() && draft.warnings.is_empty() {
        draft.warnings.push("no rows found under the header".into());
    }
    draft
}

#[cfg(test)]
mod tests;
