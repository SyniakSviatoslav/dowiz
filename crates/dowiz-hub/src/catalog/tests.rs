use super::*;

#[test]
fn a_catalogue_round_trips_through_bytes() {
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"name":"Dubin & Sushi","currency":"ALL"}"#);
    c.set_category("cat_chef", r#"{"name":"Chef's Picks"}"#);
    c.set_product("p1", r#"{"name":"Sake Futomaki","price":900}"#);
    c.set_product("p2", r#"{"name":"Ebi Futomaki","price":850}"#);
    let bytes = c.to_bytes().unwrap();

    let back = Catalog::load(&bytes).unwrap();
    assert!(back.location().unwrap().contains("Dubin"));
    assert_eq!(back.products().len(), 2);
    assert_eq!(back.categories().len(), 1);
    assert!(back.product("p1").unwrap().contains("900"));
}

#[test]
fn products_and_categories_do_not_leak_into_each_other() {
    let mut c = Catalog::create().unwrap();
    c.set_product("x", "{}");
    c.set_category("x", "{}");
    let bytes = c.to_bytes().unwrap();
    let back = Catalog::load(&bytes).unwrap();
    // Same id, different namespaces: a prefix collision here would show one
    // as the other.
    assert_eq!(back.products().len(), 1);
    assert_eq!(back.categories().len(), 1);
    assert_eq!(back.products()[0].0, "x");
    assert_eq!(back.categories()[0].0, "x");
}

/// The root is a fingerprint of the CONTENT, so an identical menu on two
/// hubs is checkably identical and a changed price is checkably different.
#[test]
fn the_root_follows_the_content() {
    let mut a = Catalog::create().unwrap();
    a.set_product("p1", r#"{"price":900}"#);
    let _ = a.to_bytes().unwrap();

    let mut b = Catalog::create().unwrap();
    b.set_product("p1", r#"{"price":900}"#);
    let _ = b.to_bytes().unwrap();
    assert_eq!(a.root(), b.root(), "same menu, same root");

    b.set_product("p1", r#"{"price":950}"#);
    let _ = b.to_bytes().unwrap();
    assert_ne!(a.root(), b.root(), "one changed price must change the root");
}

/// A deleted promo must be gone from the IMAGE, not just from the in-memory
/// entries. The commit rewrites all four arrays, so a delete that only
/// dropped the entry would still be readable after a reload.
#[test]
fn a_deleted_promo_does_not_come_back_after_a_reload() {
    let mut c = Catalog::create().unwrap();
    c.set_promo("SAVE10", r#"{"code":"SAVE10","kind":"percent","value":10}"#);
    c.set_promo("WELCOME", r#"{"code":"WELCOME","kind":"fixed","value":300}"#);
    let _ = c.to_bytes().unwrap();

    assert!(c.remove_promo("SAVE10"));
    assert!(!c.remove_promo("SAVE10"), "removing it twice is not a second delete");
    let bytes = c.to_bytes().unwrap();

    let back = Catalog::load(&bytes).unwrap();
    assert_eq!(back.promos().len(), 1);
    assert!(back.promo("SAVE10").is_none(), "the deleted code is readable after reload");
    assert!(back.promo("WELCOME").is_some());
}

#[test]
fn load_refuses_a_non_store() {
    assert!(matches!(Catalog::load(&[0u8; 4096]), Err(HubError::NotAHub)));
}

// ── A VENUE THE SIZE OF DUBIN (2026-09-24) ──────────────────────────────────

/// The rest of the live catalogue (location, categories, texts), in bytes.
const REST: usize = 7_200;

/// A dish shaped like the live ones. The 73 with a recipe are the sushi, with
/// a description and an ingredient list (477 B, their live mean); the other
/// 92 are drinks and snacks without (246 B), so the menu's mean is the live
/// 348 B. `recipe` replaces the typed ingredient list with what the Worker's
/// `set_bom` stores.
fn live_dish(i: usize, recipe: Option<String>) -> String {
    let head = format!(
        r#"{{"allergens":null,"available":true,"categoryId":"futomaki","id":"item-{i:03}","imageUrl":"/media/{:064x}.jpg","name":"Sake Futomaki {i}","price":1000,"sortOrder":{i},"weightG":243"#,
        i * 7919
    );
    if i > 73 {
        return format!("{head}}}");
    }
    let sushi = r#","description":"oriz sushi, nori, salmon, krem djathi, kastravec, tobiko","nutrition":{"approx":true,"carbs":45,"fat":18,"kcal":408,"protein":17},"tags":["salmon"]"#;
    let typed = r#","ingredients":["oriz sushi","nori","salmon","krem djathi","kastravec","tobiko"]"#;
    format!("{head}{sushi}{}}}", recipe.as_deref().unwrap_or(typed))
}

/// A supply record with every key the Worker's form writes when the file
/// gives it (no `supplier`, no `weightPerUnit`: those are left out).
fn live_supply(i: usize) -> String {
    format!(
        r#"{{"active":true,"carbsPer100":32.0,"category":"Peshk","costPerBasis":180,"fatPer100":0.3,"id":"supply-{i:02}x","kcalPer100":145.0,"kind":"food_ingredient","lowAt":3000,"name":"Supply {i:02}x","nutritionConfirmed":false,"proteinPer100":2.6,"unit":"g"}}"#
    )
}

/// The 165 dishes and the rest of the live catalogue: 513 per mille before
/// any recipe, as `/api/owner/health` reported it.
fn dubin() -> Catalog {
    let mut c = Catalog::create().unwrap();
    c.set_location(&format!(r#"{{"id":"v1","currency_code":"ALL","rest":"{}"}}"#, "x".repeat(REST)));
    for i in 1..=165 {
        c.set_product(&format!("item-{i:03}"), &live_dish(i, None));
    }
    c
}

/// Six lines of a dish as the catalogue stores them, and what the Worker's
/// `set_bom` stores beside them (derived names, cost, the four marks).
fn recipe_keys(d: usize) -> String {
    let lines: Vec<crate::stock::BomLine> = (0..6)
        .map(|l| crate::stock::BomLine { supply: format!("supply-{:02}x", (d * 5 + l) % 72), qty: 20 + 7 * l as i64 })
        .collect();
    let names: Vec<String> = lines.iter().map(|l| format!(r#""Supply {}""#, &l.supply[7..])).collect();
    format!(
        r#","bom":{},"cost":301,"ingredients":[{}],"ingredientsDerived":true,"nutritionComplete":true,"nutritionDerived":true,"weightDerived":true"#,
        bom::to_json(&lines),
        names.join(",")
    )
}

#[test]
fn a_full_menu_with_recipes_fits_the_catalogue() {
    let mut c = dubin();
    let mean = c.products().iter().map(|(_, j)| j.len()).sum::<usize>() / 165;
    assert!((340..=360).contains(&mean), "the live dishes are 348 B on average; these are {mean}");
    let before = c.projected().unwrap();
    assert!(before.fits);
    let at = before.usage.used_per_mille();
    assert!((505..=520).contains(&at), "the live venue read 513 per mille; this one reads {at}");
    for i in 0..75 {
        c.set_supply(&format!("supply-{i:02}x"), &live_supply(i));
    }
    for d in 1..=73 {
        c.set_product(&format!("item-{d:03}"), &live_dish(d, Some(recipe_keys(d))));
    }
    let after = c.projected().unwrap();
    assert!(after.fits, "{after:?}");
    assert!(after.usage.used_per_mille() < 850, "73 recipes must leave room: {}", after.usage.used_per_mille());
    // The projection is the save: the image written reads the same number.
    let bytes = c.to_bytes().unwrap();
    assert_eq!(Catalog::load(&bytes).unwrap().usage().used_cells, after.usage.used_cells);
}

/// A catalogue past its ceiling is REPORTED past it (over 1000 per mille,
/// `fits: false`) and the save it predicts is refused. Twin: the one above.
#[test]
fn a_projection_past_the_ceiling_says_it_would_not_fit() {
    let mut c = dubin();
    c.set_product("huge", &format!(r#"{{"blob":"{}"}}"#, "y".repeat(90_000)));
    let p = c.projected().unwrap();
    assert!(!p.fits);
    assert!(p.usage.used_per_mille() > 1000, "{p:?}");
    assert!(c.to_bytes().is_err(), "the save the projection predicts is refused");
}

#[test]
fn a_projection_beyond_four_ceilings_is_the_stores_own_error() {
    let mut c = Catalog::create().unwrap();
    c.set_product("huge", &"z".repeat(600_000));
    assert!(matches!(c.projected(), Err(e) if e.arena_full().is_some()));
}

/// The lean writer and the ledger's reader agree line for line, a quote in
/// an id included.
#[test]
fn a_lean_bom_reads_back_through_the_ledger() {
    let lines = vec![
        crate::stock::BomLine { supply: "salmon".into(), qty: 40 },
        crate::stock::BomLine { supply: "rice \"sushi\"".into(), qty: 90 },
    ];
    let json = bom::to_json(&lines);
    assert_eq!(json, r#"[{"supply":"salmon","qty":40},{"supply":"rice \"sushi\"","qty":90}]"#);
    assert_eq!(crate::stock::bom_of(&format!(r#"{{"bom":{json}}}"#)), lines);
    assert_eq!(bom::to_json(&[]), "[]");
}
