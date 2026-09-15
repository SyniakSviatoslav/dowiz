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

/// One dish, as read out of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftProduct {
    pub id: String,
    pub category_id: String,
    pub name: String,
    pub description: String,
    /// Integer minor units. See [`parse_price`].
    pub price: i64,
    pub available: bool,
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
                    r#"{{"id":"{}","categoryId":"{}","name":"{}","description":"{}","price":{},"available":{},"sortOrder":{}}}"#,
                    esc(&p.id),
                    esc(&p.category_id),
                    esc(&p.name),
                    esc(&p.description),
                    p.price,
                    p.available,
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

/// Read a price out of whatever the owner typed.
///
/// Returns `Err` with a human reason rather than a guess. Accepted: digits,
/// with spaces, thin spaces or commas as thousands separators, and an optional
/// trailing or leading currency word ("900", "1 200", "1,200", "900 lek",
/// "ALL 900", "900L").
///
/// REFUSED: anything with a decimal point or comma-as-decimal. The whole system
/// prints these integers WHOLE — the storefront, the admin pane and the courier
/// app all use `maximumFractionDigits: 0`, because the lek's minor unit is the
/// lek. So "9.50" could mean 9, 10, 950 or 9.5, and every one of those is a
/// different price. Refusing costs the owner one correction; guessing costs
/// them the difference on every order until someone notices.
pub fn parse_price(raw: &str) -> Result<i64, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("no price".into());
    }
    let digits: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Err(format!("no digits in {t:?}"));
    }

    // A separator with one or two digits after it and nothing further is a
    // DECIMAL, not a thousands group: "1,50" and "9.50" are prices with
    // fractions, "1,500" and "1.500" are thousands. Distinguishing them by
    // guesswork is exactly what this refuses to do.
    if let Some(pos) = t.rfind(['.', ',']) {
        let after: String = t[pos + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
        let tail_is_only_digits = t[pos + 1..]
            .chars()
            .skip(after.len())
            .all(|c| !c.is_ascii_digit());
        if (after.len() == 1 || after.len() == 2) && tail_is_only_digits {
            return Err(format!(
                "{t:?} looks like a fractional price; this menu's currency has no subunit, \
                 so write it as a whole number"
            ));
        }
    }

    digits.parse::<i64>().map_err(|_| format!("price {t:?} is too large"))
}

/// Split one CSV line, honouring double quotes.
///
/// Hand-written because a CSV crate is outside the allowlist, and because the
/// dialect needed here is small and fully specified: comma or semicolon
/// separated, `"` quoting, `""` for a literal quote. A spreadsheet export from
/// Excel or Google Sheets is exactly this.
fn split_csv_line(line: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            c if c == sep && !in_quotes => {
                out.push(cur.trim().to_string());
                cur = String::new();
            }
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// The separator a spreadsheet actually used.
///
/// Excel in a locale that uses a decimal comma writes SEMICOLON-separated CSV.
/// Assuming a comma would parse such a file as one giant column per row and
/// import nothing, with no error — the file was valid, it just meant something
/// else. Chosen by counting on the header line, where both characters are
/// separators and neither is data.
fn detect_separator(header: &str) -> char {
    if header.matches(';').count() > header.matches(',').count() {
        ';'
    } else {
        ','
    }
}

/// Match a header cell to a known column, tolerantly.
fn column_of(name: &str) -> Option<&'static str> {
    let n = name.trim().to_lowercase();
    let n = n.trim_start_matches('\u{feff}'); // Excel writes a BOM
    match n {
        "category" | "categoria" | "kategoria" | "категорія" | "категория" | "розділ" => {
            Some("category")
        }
        "name" | "product" | "item" | "dish" | "emri" | "назва" | "страва" => Some("name"),
        "description" | "desc" | "pershkrimi" | "përshkrimi" | "опис" | "склад" => {
            Some("description")
        }
        "price" | "cmimi" | "çmimi" | "ціна" => Some("price"),
        "available" | "in stock" | "stock" | "наявність" | "є" => Some("available"),
        "id" | "sku" | "code" | "kod" | "артикул" => Some("id"),
        _ => None,
    }
}

fn truthy(s: &str) -> bool {
    !matches!(
        s.trim().to_lowercase().as_str(),
        "0" | "no" | "false" | "n" | "jo" | "ні" | "нет" | "немає" | "off"
    )
}

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
            description: cell(i_desc).trim().to_string(),
            price,
            available: i_avail.map_or(true, |_| truthy(cell(i_avail))),
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
mod tests {
    use super::*;

    #[test]
    fn slugs_are_stable_and_keep_non_ascii() {
        assert_eq!(slug("Sake Futomaki"), "sake-futomaki");
        assert_eq!(slug("  sake   futomaki! "), "sake-futomaki");
        assert_eq!(slug("Sake Futomaki"), slug("SAKE FUTOMAKI"));
        // The important one: a Ukrainian or Albanian name must NOT slug to
        // nothing, or the whole menu merges into a single item.
        assert_eq!(slug("Суші сет"), "суші-сет");
        assert_eq!(slug("Byrek me spinaq"), "byrek-me-spinaq");
        assert_ne!(slug("Суші сет"), slug("Піца"));
        assert_eq!(slug("???"), "item", "a name with no letters still needs an id");
    }

    #[test]
    fn whole_prices_parse() {
        assert_eq!(parse_price("900"), Ok(900));
        assert_eq!(parse_price(" 900 "), Ok(900));
        assert_eq!(parse_price("900 lek"), Ok(900));
        assert_eq!(parse_price("ALL 900"), Ok(900));
        assert_eq!(parse_price("900L"), Ok(900));
        assert_eq!(parse_price("1 200"), Ok(1200));
        assert_eq!(parse_price("1\u{202f}200"), Ok(1200), "a narrow no-break space is a separator");
        assert_eq!(parse_price("1,200"), Ok(1200));
        assert_eq!(parse_price("1.500"), Ok(1500));
        assert_eq!(parse_price("0"), Ok(0));
    }

    /// The central refusal. Every one of these could be read four different
    /// ways, and three of them would sell the dish at the wrong price.
    #[test]
    fn fractional_prices_are_refused_not_guessed() {
        for raw in ["9.50", "9,50", "1200.00", "0.99", "9.5", "1 200,50"] {
            let got = parse_price(raw);
            assert!(got.is_err(), "{raw:?} must be refused, got {got:?}");
            assert!(
                got.unwrap_err().contains("whole number"),
                "the refusal must tell the owner what to do instead"
            );
        }
    }

    #[test]
    fn junk_prices_are_refused() {
        for raw in ["", "   ", "free", "-", "n/a"] {
            assert!(parse_price(raw).is_err(), "{raw:?} must be refused");
        }
    }

    #[test]
    fn a_plain_export_imports() {
        let csv = "Category,Name,Description,Price,Available\n\
                   Rolls,Sake Futomaki,salmon and rice,900,yes\n\
                   Rolls,Ebi Maki,prawn,750,yes\n\
                   Drinks,Water,,100,no\n";
        let d = from_csv(csv);
        assert!(d.warnings.is_empty(), "{:?}", d.warnings);
        assert_eq!(d.categories.len(), 2);
        assert_eq!(d.categories[0].name, "Rolls");
        assert_eq!(d.categories[0].sort_order, 0);
        assert_eq!(d.products.len(), 3);
        assert_eq!(d.products[0].id, "rolls-sake-futomaki");
        assert_eq!(d.products[0].price, 900);
        assert_eq!(d.products[0].description, "salmon and rice");
        assert!(d.products[0].available);
        assert!(!d.products[2].available, "\"no\" must mean unavailable");
        assert_eq!(d.products[2].category_id, "drinks");
        // Sort order restarts per category, so each category reads top to bottom.
        assert_eq!(d.products[0].sort_order, 0);
        assert_eq!(d.products[1].sort_order, 1);
        assert_eq!(d.products[2].sort_order, 0);
    }

    /// Excel in a comma-decimal locale writes semicolons. Assuming commas would
    /// parse the whole file as one column and import NOTHING, with no error.
    #[test]
    fn a_semicolon_export_imports_too() {
        let csv = "Kategoria;Emri;Çmimi\nRolls;Sake Futomaki;900\n";
        let d = from_csv(csv);
        assert!(d.warnings.is_empty(), "{:?}", d.warnings);
        assert_eq!(d.products.len(), 1);
        assert_eq!(d.products[0].price, 900);
    }

    /// The owner's own language must work, and so must Excel's BOM.
    #[test]
    fn ukrainian_headers_and_a_bom_are_understood() {
        let csv = "\u{feff}Розділ,Назва,Ціна\nСети,Суші сет,2500\n";
        let d = from_csv(csv);
        assert!(d.warnings.is_empty(), "{:?}", d.warnings);
        assert_eq!(d.products.len(), 1);
        assert_eq!(d.products[0].name, "Суші сет");
        assert_eq!(d.products[0].price, 2500);
        assert_eq!(d.categories[0].name, "Сети");
    }

    #[test]
    fn quoted_cells_survive_their_commas() {
        let csv = "Category,Name,Description,Price\n\
                   Rolls,\"Futomaki, large\",\"rice, nori, salmon\",900\n";
        let d = from_csv(csv);
        assert_eq!(d.products.len(), 1, "{:?}", d.warnings);
        assert_eq!(d.products[0].name, "Futomaki, large");
        assert_eq!(d.products[0].description, "rice, nori, salmon");
    }

    #[test]
    fn a_doubled_quote_is_a_literal_quote() {
        let csv = "Name,Price\n\"The \"\"Big\"\" One\",900\n";
        let d = from_csv(csv);
        assert_eq!(d.products[0].name, r#"The "Big" One"#);
    }

    /// A bad row must be reported BY ROW NUMBER and must not stop the import.
    /// An import that aborts on the first typo is one the owner gives up on.
    #[test]
    fn a_bad_row_is_named_and_the_rest_still_import() {
        let csv = "Category,Name,Price\n\
                   Rolls,Good One,900\n\
                   Rolls,Bad Price,9.50\n\
                   Rolls,No Name,700\n\
                   Rolls,Another Good,800\n";
        let mut csv = csv.to_string();
        csv = csv.replace("Rolls,No Name,700", "Rolls,,700");
        let d = from_csv(&csv);
        assert_eq!(d.products.len(), 2, "the good rows must survive: {:?}", d.products);
        assert_eq!(d.warnings.len(), 2, "{:?}", d.warnings);
        assert!(d.warnings[0].starts_with("row 3 (Bad Price)"), "{:?}", d.warnings[0]);
        assert!(d.warnings[1].starts_with("row 4"), "{:?}", d.warnings[1]);
    }

    #[test]
    fn a_missing_price_column_is_an_error_not_an_empty_menu() {
        let d = from_csv("Category,Name\nRolls,Sake\n");
        assert!(d.products.is_empty());
        assert_eq!(d.warnings.len(), 1);
        assert!(d.warnings[0].contains("price column"), "{:?}", d.warnings);
    }

    #[test]
    fn an_empty_file_says_so() {
        assert!(from_csv("").warnings[0].contains("empty"));
        assert!(from_csv("Name,Price\n").warnings[0].contains("no rows"));
    }

    /// Importing the same file twice must update, not duplicate. That is what
    /// the deterministic slug buys.
    #[test]
    fn the_same_file_twice_produces_the_same_ids() {
        let csv = "Category,Name,Price\nRolls,Sake Futomaki,900\n";
        let a = from_csv(csv);
        let b = from_csv(csv);
        assert_eq!(a.products[0].id, b.products[0].id);
        // And a price change keeps the id, so it is an update.
        let c = from_csv("Category,Name,Price\nRolls,Sake Futomaki,950\n");
        assert_eq!(a.products[0].id, c.products[0].id);
        assert_ne!(a.products[0].price, c.products[0].price);
    }

    /// Two rows for one dish collapse into one. Say so rather than let the
    /// owner wonder where the other price went.
    #[test]
    fn a_duplicate_dish_is_reported() {
        let csv = "Category,Name,Price\nRolls,Sake,900\nRolls,sake,950\n";
        let d = from_csv(csv);
        assert!(
            d.warnings.iter().any(|w| w.contains("more than once")),
            "{:?}",
            d.warnings
        );
    }

    /// The same dish name in two categories is two dishes, not a duplicate.
    #[test]
    fn the_same_name_in_two_categories_is_two_dishes() {
        let csv = "Category,Name,Price\nRolls,Special,900\nDrinks,Special,300\n";
        let d = from_csv(csv);
        assert!(d.warnings.is_empty(), "{:?}", d.warnings);
        assert_eq!(d.products.len(), 2);
        assert_ne!(d.products[0].id, d.products[1].id);
    }

    #[test]
    fn an_explicit_id_column_is_honoured() {
        let csv = "id,Category,Name,Price\nSKU-01,Rolls,Sake,900\n";
        let d = from_csv(csv);
        assert_eq!(d.products[0].id, "sku-01");
    }

    /// The JSON the draft produces must be parseable and must not be breakable
    /// by a dish name containing a quote.
    #[test]
    fn the_draft_serialises_safely() {
        let csv = "Category,Name,Price\nRolls,\"The \"\"Big\"\" One\",900\n";
        let json = from_csv(csv).as_json();
        assert!(json.contains(r#"\"Big\""#), "the quote must be escaped: {json}");
        assert!(!json.contains(r#""name":"The "Big""#), "unescaped: {json}");
    }
}
