//! Which side a line lands on: the three rules, in order, and the words in
//! the venue's four languages (W-DRINKS-KITCHEN).

use super::*;
use serde_json::json;

// ── rule 1: a category's default from its name, in any language ─────────────

/// THE REAL MENU (dubin-sushi, read 2026-11-04): six of twenty-one categories
/// are the bar. Each one is drinks by its name alone; every other one is kitchen.
#[test]
fn the_venues_own_categories_split_as_the_owner_would() {
    for drinks in ["Kokteje", "Kafeteria", "Freskuese", "Alkool", "Birra", "Lengje Frutash"] {
        assert_eq!(category_default(drinks), Side::Drinks, "{drinks}");
    }
    for kitchen in ["Zgjedhjet e shefit", "Sete", "Bowls", "Supa", "Nigiri", "Philadelphia", "California", "Futomaki", "Signature", "Role të ngrohta", "Volcano", "Role vegjetariane", "Snacks", "Premium", "Maki"] {
        assert_eq!(category_default(kitchen), Side::Kitchen, "{kitchen}");
    }
}

#[test]
fn drinks_words_are_read_in_sq_en_uk_and_ru_and_everything_else_is_kitchen() {
    for d in ["Pije", "Bar", "Drinks", "Coffee & tea", "Напої", "Напитки", "Beer", "Birra", "Wine", "Verë", "Cocktails", "Shisha", "Hookah", "Nargjile", "Кава", "Чай", "Кальян", "Пиво", "Вино"] {
        assert_eq!(category_default(d), Side::Drinks, "{d}");
    }
    for k in ["Sushi", "Десерти", "Ëmbëlsira", "Sweet", "Hot rolls", "Soups", "", "Barbecue"] {
        assert_eq!(category_default(k), Side::Kitchen, "{k}");
    }
}

/// `sake` is the salmon on a sushi menu; "hot sake" the dish is never a drink by its word.
#[test]
fn sake_is_not_a_drinks_word() {
    assert_eq!(of_words("Sake Futomaki"), Some(Side::Kitchen));
    assert_eq!(of_words("hot sake"), None);
}

// ── rule 1: the owner's flag wins over the name; an old record reads by its name ──

#[test]
fn the_owners_flag_wins_over_the_name_and_an_old_record_reads_by_its_name() {
    assert_eq!(of_category(&json!({ "name": "Kafeteria" })), Side::Drinks, "old record, no flag: by name");
    assert_eq!(of_category(&json!({ "name": "Kafeteria", "side": "kitchen" })), Side::Kitchen, "the owner said kitchen");
    assert_eq!(of_category(&json!({ "name": "Snacks", "side": "drinks" })), Side::Drinks, "the owner said drinks");
    assert_eq!(of_category(&json!({ "name": "Snacks", "side": "garbage" })), Side::Kitchen, "an unreadable flag is no flag");
    assert!(flag_set(&json!({ "name": "Snacks", "side": "drinks" })));
    assert!(!flag_set(&json!({ "name": "Snacks" })));
    assert!(!flag_set(&json!({ "name": "Snacks", "side": "garbage" })));
}

fn catalogue() -> dowiz_hub::catalog::Catalog {
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    cat.set_category("kafeteria", r#"{"id":"kafeteria","name":"Kafeteria","sortOrder":1}"#);
    cat.set_category("snacks", r#"{"id":"snacks","name":"Snacks","sortOrder":2,"side":"drinks"}"#);
    cat.set_category("maki", r#"{"id":"maki","name":"Maki","sortOrder":3}"#);
    cat.set_product("espresso", r#"{"id":"espresso","name":"Espresso","categoryId":"kafeteria","price":100}"#);
    cat.set_product("chips", r#"{"id":"chips","name":"Rice chips","categoryId":"snacks","price":250}"#);
    cat.set_product("p-roll", r#"{"id":"p-roll","name":"Sake roll","categoryId":"maki","price":900}"#);
    cat.set_product("orphan", r#"{"id":"orphan","name":"Lajthiza 0.5","categoryId":"gone","price":100}"#);
    cat
}

#[test]
fn a_dish_takes_its_categorys_side_flag_first_then_name() {
    let s = Sides::of(&catalogue());
    assert_eq!(s.of_line(Some("espresso"), "whatever", None), Some(Side::Drinks), "Kafeteria by name");
    assert_eq!(s.of_line(Some("chips"), "Rice chips", None), Some(Side::Drinks), "the owner flagged Snacks as drinks");
    assert_eq!(s.of_line(Some("p-roll"), "Sake roll", None), Some(Side::Kitchen));
    // A dish whose category is gone reads the category id as a name: kitchen unless it says drinks.
    assert_eq!(s.of_line(Some("orphan"), "Lajthiza 0.5", None), Some(Side::Kitchen));
}

// ── rule 2: a till line with no dish takes the till's own group ─────────────

#[test]
fn a_till_line_with_no_dish_takes_the_tills_group_by_the_category_rule() {
    let s = Sides::of(&catalogue());
    assert_eq!(s.of_line(Some("ebills:56"), "kafe", Some("kafeteria")), Some(Side::Drinks));
    assert_eq!(s.of_line(Some("ebills:164"), "Shisha", Some("Shisha")), Some(Side::Drinks));
    assert_eq!(s.of_line(Some("ebills:199"), "Tom Yum ", Some("Sushi")), Some(Side::Kitchen));
    assert_eq!(s.of_line(Some("ebills:185"), "croissant", Some("sweet")), Some(Side::Kitchen));
    // The till's group outranks the name: a "Mojito" the till files under Sushi is kitchen, loudly the till's doing.
    assert_eq!(s.of_line(Some("ebills:149"), "Mojito", Some("Sushi")), Some(Side::Kitchen));
    // ... and a matched dish outranks the till's group.
    assert_eq!(s.of_line(Some("p-roll"), "Sake roll", Some("alkool")), Some(Side::Kitchen));
    // An empty group is no group.
    assert_eq!(s.of_line(Some("ebills:149"), "Mojito", Some("  ")), Some(Side::Drinks));
}

// ── rule 3: the name, or unclassified ───────────────────────────────────────

/// THE 38 TILL NAMES NO DISH ANSWERED TO (dubin-durres, 30 days to 2026-11-04),
/// read by their words alone -- what a line imported before the till's group
/// was kept gets. Three stay unclassified and are shown as such, never guessed.
#[test]
fn the_real_unmatched_till_names_read_by_their_words() {
    let s = Sides::default();
    let drinks = ["kafe", "Shisha", "Glintwine", "makiato", "Analcolic", "salep", "Mojito", "Punsh", "pinacolada", "Caj Bio V", "Pornstar Martini", "Negroni", "Hugo Spritz", "Milk"];
    let kitchen = ["salmon burger ", "panko shrimp", "Tom Yum ", "Maguro Ebi", "Shrimp Burger", "bowl chiken", "Tuna Burger", "Ebi Cloud", "Tuna Crown", "Patate", "Ebi Volcano Premium ", "Yin-Yang Tuna Avocado Roll", "Rice Chips", "Spicy Crab Kimbap", "Maki Shrimp", "Maki sreams ", "Chuka salad", "croissant", "Sous", "dragon & tiger", "tuna Burger"];
    let open = ["hot sake", "whitte russian", "ruby no hitomi "];
    for n in drinks {
        assert_eq!(s.of_line(Some("ebills:0"), n, None), Some(Side::Drinks), "{n}");
    }
    for n in kitchen {
        assert_eq!(s.of_line(Some("ebills:0"), n, None), Some(Side::Kitchen), "{n}");
    }
    for n in open {
        assert_eq!(s.of_line(Some("ebills:0"), n, None), None, "{n}");
    }
    assert_eq!(drinks.len() + kitchen.len() + open.len(), 38);
}

#[test]
fn an_item_is_read_off_the_log_line() {
    let s = Sides::of(&catalogue());
    assert_eq!(s.of_item(&json!({ "product_id": "espresso", "name": "kafe" })), Some(Side::Drinks));
    assert_eq!(s.of_item(&json!({ "product_id": "ebills:56", "name": "kafe", "ebills_category": "kafeteria" })), Some(Side::Drinks));
    assert_eq!(s.of_item(&json!({ "product_id": "ebills:9", "name": "ruby no hitomi" })), None);
    assert_eq!(s.of_item(&json!({ "product_id": "", "name": "Negroni" })), Some(Side::Drinks), "an empty id is no id");
}

// ── the check's group ───────────────────────────────────────────────────────

#[test]
fn a_check_with_any_kitchen_line_is_kitchen_all_drinks_is_drinks_only_else_unclassified() {
    use Side::*;
    assert_eq!(group([Some(Kitchen), Some(Drinks)]), Group::Kitchen);
    assert_eq!(group([Some(Kitchen), None]), Group::Kitchen);
    assert_eq!(group([Some(Drinks), Some(Drinks)]), Group::DrinksOnly);
    assert_eq!(group([Some(Drinks), None]), Group::Unclassified, "a drink and an unknown line is not drinks-only");
    assert_eq!(group([None]), Group::Unclassified);
    assert_eq!(group(Vec::<Option<Side>>::new()), Group::Unclassified, "no line, no side");
}
