//! Every rule of BLUEPRINT-LAST-MILE §3.3.2, each by the name its table gives.

use super::*;

/// 2026-09-22 12:00 local.
const NOW: i64 = 1_790_078_400_000;

fn menu() -> Vec<(String, String)> {
    vec![
        ("rolls-salmon-roll".into(), "Salmon roll".into()),
        ("rolls-tuna-roll".into(), "Tuna roll".into()),
        ("drinks-water".into(), "Water".into()),
    ]
}

fn opts<'a>(products: &'a [(String, String)], existing: &'a [(String, String)]) -> Opts<'a> {
    Opts {
        currency: "ALL",
        cost_scale: Some(CostScale::Major),
        unit_hint: None,
        local_now_ms: NOW,
        products,
        existing_supplies: existing,
    }
}

const ING: &str = "name,unit,cost\nSalmon,kg,1800\nRice,kg,200\nNori,pcs,15\nSoy sauce,l,400\n";

fn run(ing: &str, rec: &str) -> RecipeDraft {
    let m = menu();
    from_csv(ing, rec, &opts(&m, &[]))
}

fn bom(d: &RecipeDraft, pid: &str) -> Vec<(String, i64)> {
    d.recipes
        .iter()
        .find(|r| r.product_id == pid)
        .map(|r| r.lines.iter().map(|l| (l.supply.clone(), l.qty)).collect())
        .unwrap_or_default()
}

#[test]
fn a_kilogram_becomes_a_thousand_grams() {
    let d = run(ING, "dish,ingredient,qty,unit\nSalmon roll,Salmon,0.04,kg\nSalmon roll,Nori,1,pcs\n");
    assert_eq!(bom(&d, "rolls-salmon-roll"), vec![("salmon".into(), 40), ("nori".into(), 1)], "{:?}", d.warnings);
    let salmon = d.supplies.iter().find(|s| s.id == "salmon").unwrap();
    assert_eq!(salmon.unit, "g");
    // 1800 lek per kg is 180 lek per 100 g, the catalogue's basis.
    assert_eq!(salmon.cost_per_basis, Some(180));
    let soy = d.supplies.iter().find(|s| s.id == "soy-sauce").unwrap();
    assert_eq!((soy.unit, soy.cost_per_basis), ("ml", Some(40)));
}

#[test]
fn an_unknown_unit_is_refused() {
    let d = run("name,unit\nSalmon,kg\nSake,bottle\n", "");
    assert!(d.supplies.iter().all(|s| s.id != "sake"));
    assert!(d.warnings.iter().any(|w| w.contains("row 3") && w.contains("bottle")), "{:?}", d.warnings);
    // In a recipe line too: the dish is refused whole, naming the row.
    let d = run(ING, "dish,ingredient,qty,unit\nSalmon roll,Salmon,40,oz\n");
    assert!(d.recipes.is_empty());
    assert_eq!(d.without_recipe, vec!["rolls-salmon-roll".to_string()]);
    assert!(d.warnings.iter().any(|w| w.contains("row 2") && w.contains("oz")), "{:?}", d.warnings);
}

#[test]
fn a_fractional_gram_is_refused_not_rounded() {
    let d = run(ING, "dish,ingredient,qty,unit\nSalmon roll,Salmon,0.0425,kg\n");
    assert!(d.recipes.is_empty(), "42.5 g must not become 42 or 43");
    assert_eq!(d.without_recipe, vec!["rolls-salmon-roll".to_string()]);
    assert!(d.warnings.iter().any(|w| w.contains("42.5") && w.contains("refused")), "{:?}", d.warnings);
    // 0.043 kg is 43 g exactly, and is fine.
    let d = run(ING, "dish,ingredient,qty,unit\nSalmon roll,Salmon,0.043,kg\n");
    assert_eq!(bom(&d, "rolls-salmon-roll"), vec![("salmon".into(), 43)]);
    // Over the bound is refused too.
    let d = run(ING, "dish,ingredient,qty,unit\nSalmon roll,Salmon,101,kg\n");
    assert!(d.recipes.is_empty());
    // A comma with three digits after it is a guess either way.
    let d = run(ING, "dish;ingredient;qty;unit\nSalmon roll;Salmon;1,500;g\n");
    assert!(d.recipes.is_empty(), "{:?}", d.recipes);
    // A cost that comes to half a lek per 100 g is refused, not rounded.
    let d = run("name,unit,cost\nSalmon,kg,1805\n", "");
    assert_eq!(d.supplies[0].cost_per_basis, None);
    assert!(d.warnings.iter().any(|w| w.contains("1805") && w.contains("refused")), "{:?}", d.warnings);
}

#[test]
fn gross_is_the_reserved_quantity() {
    let d = run(ING, "dish,ingredient,brutto,netto,unit\nSalmon roll,Salmon,50,40,g\n");
    assert_eq!(bom(&d, "rolls-salmon-roll"), vec![("salmon".into(), 50)]);
    // A line with only a net quantity is refused rather than reserved as gross.
    let d = run(ING, "dish,ingredient,qty,net,unit\nSalmon roll,Salmon,,40,g\n");
    assert!(d.recipes.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("net")), "{:?}", d.warnings);
}

#[test]
fn net_and_yield_survive_on_the_line() {
    let d = run(ING, "dish,ingredient,gross,net,yield,unit\nSalmon roll,Salmon,50,40,38,g\n");
    let l = &d.recipes[0].lines[0];
    assert_eq!((l.qty, l.net, l.yield_), (50, Some(40), Some(38)));
    let j = d.as_json();
    assert!(j.contains(r#"{"supply":"salmon","qty":50,"net":40,"yield":38}"#), "{j}");
}

const SAUCE: &str = "dish,ingredient,qty,unit,batch\n\
    Spicy mayo,Mayo,800,g,1 kg\n\
    Spicy mayo,Sriracha,200,g,\n\
    Salmon roll,Salmon,40,g,\n\
    Salmon roll,Spicy mayo,15,g,\n\
    Salmon roll,Rice,100,g,\n";

#[test]
fn a_prepack_is_flattened_and_reported() {
    let ing = "name,unit\nSalmon,g\nRice,g\nMayo,g\nSriracha,g\n";
    let d = run(ing, SAUCE);
    // 15 g of a 1 kg batch: 12 g mayo and 3 g sriracha — exact.
    assert_eq!(
        bom(&d, "rolls-salmon-roll"),
        vec![("salmon".into(), 40), ("mayo".into(), 12), ("sriracha".into(), 3), ("rice".into(), 100)],
        "{:?}",
        d.warnings
    );
    assert!(d.flattened.iter().any(|f| f.contains("Salmon roll") && f.contains("Spicy mayo") && f.contains("2 line")), "{:?}", d.flattened);
    // The sauce is not a supply and not a dish: nothing reserves "spicy-mayo".
    assert!(d.supplies.iter().all(|s| s.id != "spicy-mayo"));
    assert!(d.recipes.iter().all(|r| r.lines.iter().all(|l| l.supply != "spicy-mayo")));
    // 10 g of it would be 8 g + 2 g; 7 g would be 5.6 g of mayo: refused.
    let d = run(ing, &SAUCE.replace("Spicy mayo,15,g", "Spicy mayo,7,g"));
    assert!(d.recipes.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("mayo") && w.contains("5.6")), "{:?}", d.warnings);
}

#[test]
fn a_prepack_cycle_is_refused() {
    let ing = "name,unit\nSalmon,g\nOil,g\n";
    let rec = "dish,ingredient,qty,unit,batch\n\
        Sauce a,Sauce b,100,g,1000 g\n\
        Sauce a,Oil,900,g,\n\
        Sauce b,Sauce a,100,g,1000 g\n\
        Sauce b,Oil,900,g,\n\
        Salmon roll,Salmon,40,g,\n\
        Salmon roll,Sauce a,10,g,\n";
    let d = run(ing, rec);
    assert!(d.recipes.is_empty());
    assert_eq!(d.without_recipe, vec!["rolls-salmon-roll".to_string()]);
    assert!(d.warnings.iter().any(|w| w.contains("sauce-a → sauce-b → sauce-a")), "{:?}", d.warnings);
}

#[test]
fn an_unknown_supply_leaves_the_dish_without_a_recipe() {
    let d = run(ING, "dish,ingredient,qty,unit\nSalmon roll,Salmon,40,g\nSalmon roll,Wasabi,5,g\nTuna roll,Rice,90,g\n");
    // No partial bom: the salmon line alone would be a wrong reservation.
    assert!(bom(&d, "rolls-salmon-roll").is_empty());
    assert_eq!(d.without_recipe, vec!["rolls-salmon-roll".to_string()]);
    assert!(d.warnings.iter().any(|w| w.contains("row 3") && w.contains("Wasabi")), "{:?}", d.warnings);
    // The rest of the file still imports.
    assert_eq!(bom(&d, "rolls-tuna-roll"), vec![("rice".into(), 90)]);
}

#[test]
fn a_foreign_currency_is_refused_not_converted() {
    let d = run("name,unit,cost,currency\nSalmon,kg,443.45,UAH\n", "dish,ingredient,qty,unit\nSalmon roll,Salmon,40,g\n");
    assert!(d.supplies.is_empty() && d.recipes.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("UAH") && w.contains("ALL")), "{:?}", d.warnings);
    // A currency word beside the number counts too.
    let d = run("name,unit,cost\nSalmon,kg,18 EUR\n", "");
    assert!(d.supplies.is_empty());
    // The venue's own is fine, and hundredths ("kopecks") are scaled exactly.
    let m = menu();
    let mut o = opts(&m, &[]);
    o.currency = "EUR";
    o.cost_scale = Some(CostScale::Hundredths);
    let d = from_csv("name,unit,cost\nSalmon,kg,1850\n", "", &o);
    // 18.50 EUR per kg = 1850 cents per kg = 185 cents per 100 g.
    assert_eq!(d.supplies[0].cost_per_basis, Some(185), "{:?}", d.warnings);
    // Not said is not guessed: no cost, one warning asking.
    o.cost_scale = None;
    let d = from_csv("name,unit,cost\nSalmon,kg,1850\n", "", &o);
    assert_eq!(d.supplies[0].cost_per_basis, None);
    assert!(d.warnings.iter().any(|w| w.contains("cost=hundredths")), "{:?}", d.warnings);
}

#[test]
fn only_the_card_in_force_today_is_imported() {
    let rec = "dish,ingredient,qty,unit,from,to\n\
        Salmon roll,Salmon,30,g,2025-01-01,2026-01-01\n\
        Salmon roll,Salmon,40,g,2026-01-01,\n\
        Salmon roll,Rice,100,g,2026-01-01,\n\
        Tuna roll,Rice,90,g,2027-01-01,\n";
    let d = run(ING, rec);
    assert_eq!(bom(&d, "rolls-salmon-roll"), vec![("salmon".into(), 40), ("rice".into(), 100)]);
    assert!(bom(&d, "rolls-tuna-roll").is_empty());
    assert!(d.warnings.iter().any(|w| w.starts_with("2 dated card")), "{:?}", d.warnings);
    // Two cards in force at once is a contradiction, refused.
    let d = run(ING, "dish,ingredient,qty,unit,from\nSalmon roll,Salmon,30,g,2025-01-01\nSalmon roll,Salmon,40,g,01.01.2026\n");
    assert!(d.recipes.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("more than one card")), "{:?}", d.warnings);
}

#[test]
fn a_vanished_supply_is_retired_not_deleted() {
    let m = menu();
    let existing = vec![("salmon".to_string(), "g".to_string()), ("eel".to_string(), "g".to_string())];
    let d = from_csv(ING, "", &opts(&m, &existing));
    assert_eq!(d.retired, vec!["eel".to_string()]);
    // An unreadable file retires nothing.
    let d = from_csv("nome;unita\nSalmon;kg\n", "", &opts(&m, &existing));
    assert!(d.retired.is_empty() && d.supplies.is_empty());
}

#[test]
fn the_same_files_twice_give_the_same_draft() {
    let a = run(ING, "dish,ingredient,qty,unit\nSalmon roll,Salmon,40,g\n");
    let b = run(ING, "dish,ingredient,qty,unit\nSalmon roll,Salmon,40,g\n");
    assert_eq!(a.as_json(), b.as_json());
    assert!(a.as_json().starts_with(r#"{"supplies":[{"id":"salmon","name":"Salmon","unit":"g""#));
}

#[test]
fn a_dish_not_on_the_menu_is_named() {
    let d = run(ING, "dish,ingredient,qty,unit\nEel roll,Salmon,40,g\n");
    assert!(d.recipes.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("Eel roll") && w.contains("not on the menu")), "{:?}", d.warnings);
}

#[test]
fn a_line_with_no_unit_asks_unless_a_hint_is_given() {
    let m = menu();
    let d = from_csv(ING, "dish,ingredient,qty\nSalmon roll,Salmon,40\n", &opts(&m, &[]));
    assert!(d.recipes.is_empty());
    let mut o = opts(&m, &[]);
    o.unit_hint = Some("g");
    let d = from_csv(ING, "dish,ingredient,qty\nSalmon roll,Salmon,40\n", &o);
    assert_eq!(bom(&d, "rolls-salmon-roll"), vec![("salmon".into(), 40)]);
}

#[test]
fn a_supply_whose_unit_changes_is_refused() {
    let m = menu();
    let existing = vec![("nori".to_string(), "g".to_string())];
    let d = from_csv(ING, "dish,ingredient,qty,unit\nSalmon roll,Nori,1,pcs\n", &opts(&m, &existing));
    assert!(d.supplies.iter().all(|s| s.id != "nori"));
    assert!(d.warnings.iter().any(|w| w.contains("Nori") && w.contains("change meaning")), "{:?}", d.warnings);
    assert_eq!(d.without_recipe, vec!["rolls-salmon-roll".to_string()]);
}
