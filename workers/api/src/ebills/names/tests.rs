//! The name index: one dish by any of its names, two dishes = nobody.

use super::{how, index, resolve, Match};

fn products() -> Vec<(String, String)> {
    vec![
        ("p-sake".into(), r#"{"id":"p-sake","name":"Sake Roll","price":250}"#.into()),
        ("p-tuna".into(), r#"{"id":"p-tuna","name":"Tuna Roll","price":300}"#.into()),
        ("p-soup".into(), r#"{"id":"p-soup","name":"Supë Miso","price":200}"#.into()),
        ("p-soup2".into(), r#"{"id":"p-soup2","name":"Miso soup (large)","price":260}"#.into()),
    ]
}

fn i18n() -> Vec<(String, String)> {
    vec![
        ("en/product/p-soup/name".into(), "Miso soup".into()),
        ("uk/product/p-soup/name".into(), "Місо суп".into()),
        ("en/product/p-soup2/name".into(), "Miso soup".into()), // the SAME English name on a second dish
        ("en/product/p-sake/description".into(), "Sake Roll".into()), // a description is not a name
        ("en/product/p-gone/name".into(), "Sake Roll".into()), // a row for a dish the catalogue no longer has
        ("en/category/c_1/name".into(), "Tuna Roll".into()), // a category, not a product
    ]
}

#[test]
fn the_venues_own_name_matches_case_whitespace_and_diacritics_apart() {
    let idx = index(&products(), &[]);
    assert_eq!(resolve(&idx, "sake roll"), Some("p-sake".into()));
    assert_eq!(resolve(&idx, "  SAKE   ROLL "), Some("p-sake".into()));
    assert_eq!(resolve(&idx, "Sake-Roll"), Some("p-sake".into()));
    assert_eq!(resolve(&idx, "Supe Miso"), Some("p-soup".into()));
    assert_eq!(resolve(&idx, "supë miso"), Some("p-soup".into()));
}

#[test]
fn a_translated_name_matches_too_and_only_product_name_rows_count() {
    let idx = index(&products(), &i18n());
    assert_eq!(resolve(&idx, "Місо суп"), Some("p-soup".into()));
    // `p-gone` is not in the catalogue: its row adds nothing, so "Sake Roll" stays one dish.
    assert_eq!(resolve(&idx, "Sake Roll"), Some("p-sake".into()));
    // The category row does not make "Tuna Roll" ambiguous.
    assert_eq!(resolve(&idx, "Tuna Roll"), Some("p-tuna".into()));
}

#[test]
fn two_dishes_under_one_name_resolve_to_nobody() {
    let idx = index(&products(), &i18n());
    assert_eq!(how(&idx, "Miso soup"), Match::Ambiguous(2));
    assert_eq!(resolve(&idx, "Miso soup"), None);
    // The positive twin: the same name with one dish under it resolves.
    let one = index(&products()[..3], &i18n()[..2]);
    assert_eq!(how(&one, "Miso soup"), Match::One("p-soup".into()));
}

#[test]
fn an_unknown_or_empty_name_is_none_never_a_guess() {
    let idx = index(&products(), &i18n());
    assert_eq!(how(&idx, "Dragon roll"), Match::None);
    assert_eq!(how(&idx, "   "), Match::None);
    assert_eq!(resolve(&idx, "---"), None);
}

#[test]
fn the_index_is_the_same_whatever_order_the_rows_come_in() {
    let mut p = products();
    let mut t = i18n();
    let a = index(&p, &t);
    p.reverse();
    t.reverse();
    assert_eq!(index(&p, &t), a);
}
