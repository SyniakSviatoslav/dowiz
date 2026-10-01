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

/// W-AUDIT S6 (2026-09-27): two numbers in one cell used to import as
/// their digits joined -- "450/650" as 450650. A thousands group still
/// reads as one number.
#[test]
fn a_cell_with_more_than_one_number_is_refused_and_a_grouped_one_is_not() {
    for raw in ["450/650", "900-1200", "2 x 450", "-900", "0.500", "900 / 1200 lek", "12 34"] {
        assert!(parse_price(raw).is_err(), "{raw:?} must be refused, not joined");
    }
    assert_eq!(parse_price("1 200"), Ok(1200));
    assert_eq!(parse_price("1,200"), Ok(1200));
    assert_eq!(parse_price("1.500 lek"), Ok(1500));
    assert_eq!(parse_price("ALL 900"), Ok(900));
    assert_eq!(parse_price("12 345 678"), Ok(12_345_678));
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
    assert_eq!(d.products[0].description.as_deref(), Some("salmon and rice"));
    assert_eq!(d.products[0].available, Some(true));
    assert_eq!(d.products[2].available, Some(false), "\"no\" must mean unavailable");
    assert_eq!(d.products[2].description.as_deref(), Some(""), "an EMPTY cell is an empty text");
    assert_eq!(d.products[2].category_id, "drinks");
    // Sort order restarts per category, so each category reads top to bottom.
    assert_eq!(d.products[0].sort_order, 0);
    assert_eq!(d.products[1].sort_order, 1);
    assert_eq!(d.products[2].sort_order, 0);
}

/// AUDIT D5: a two-column price sheet says nothing about descriptions or
/// availability. Both come back `None` ("keep what is stored"), never `""`
/// and `true` -- which blanked every description and put every stopped
/// dish back on sale. The draft's JSON says `null`, not a value.
#[test]
fn absent_columns_are_none_not_a_blank_and_on_sale() {
    let d = from_csv("name,price\nSake Nigiri,900\n");
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!((d.products[0].description.clone(), d.products[0].available), (None, None));
    assert!(!d.category_column, "and no category column is said, not assumed");
    assert!(d.as_json().contains(r#""description":null,"price":900,"available":null"#), "{}", d.as_json());
    // TWIN: present columns are values, and the JSON carries them.
    let d = from_csv("name,price,description,available\nSake Nigiri,900,fish,no\n");
    assert_eq!((d.products[0].description.as_deref(), d.products[0].available), (Some("fish"), Some(false)));
    assert!(d.as_json().contains(r#""description":"fish","price":900,"available":false"#), "{}", d.as_json());
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
    assert_eq!(d.products[0].description.as_deref(), Some("rice, nori, salmon"));
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
