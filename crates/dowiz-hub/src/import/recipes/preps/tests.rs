//! R1 (lane W-PF2): the importer writes REAL semi-finished products. Every
//! test calls the real reader (`from_csv` / `recipes_against`); every refusal
//! has its positive twin; the flat file's bytes are pinned from the code as
//! it was BEFORE this row (captured 2026-09-29 at main 620e2464).

use super::super::*;

const NOW: i64 = 1_790_078_400_000;

fn menu() -> Vec<(String, String)> {
    vec![("rolls-salmon-roll".into(), "Salmon roll".into()), ("rolls-tuna-roll".into(), "Tuna roll".into()), ("rolls-philadelphia".into(), "Philadelphia".into())]
}

fn opts<'a>(products: &'a [(String, String)], preps: bool) -> Opts<'a> {
    Opts { currency: "ALL", cost_scale: Some(CostScale::Major), unit_hint: None, local_now_ms: NOW, products, existing_supplies: &[], preps }
}

fn run(ing: &str, rec: &str, preps: bool) -> RecipeDraft {
    let m = menu();
    from_csv(ing, rec, &opts(&m, preps))
}

fn bom(d: &RecipeDraft, pid: &str) -> Vec<(String, i64)> {
    d.recipes.iter().find(|r| r.product_id == pid).map(|r| r.lines.iter().map(|l| (l.supply.clone(), l.qty)).collect()).unwrap_or_default()
}

const FLAT_ING: &str = "name,unit,cost\nSalmon,kg,1800\nRice,kg,200\nNori,pcs,15\nSoy sauce,l,400\nMayo,g,1\n";
const FLAT_REC: &str = "dish,ingredient,gross,net,yield,unit\n\
    Salmon roll,Salmon,50,40,38,g\n\
    Salmon roll,Rice,100,,,g\n\
    Salmon roll,Nori,1,,,pcs\n\
    Salmon roll,Rice,20,,,g\n\
    Tuna roll,Rice,90,,,g\n\
    Tuna roll,Mayo,0.005,,,kg\n";

/// `from_csv(FLAT_ING, FLAT_REC)` as the importer answered it BEFORE R1, byte
/// for byte (printed by a capture test run on the unmodified tree).
const FLAT_BEFORE: &str = r#"{"supplies":[{"id":"salmon","name":"Salmon","unit":"g","kind":null,"category":"","costPerBasis":180,"kcalPer100":null,"proteinPer100":null,"fatPer100":null,"carbsPer100":null,"lowAt":null,"weightPerUnit":null,"supplier":null},{"id":"rice","name":"Rice","unit":"g","kind":null,"category":"","costPerBasis":20,"kcalPer100":null,"proteinPer100":null,"fatPer100":null,"carbsPer100":null,"lowAt":null,"weightPerUnit":null,"supplier":null},{"id":"nori","name":"Nori","unit":"unit","kind":null,"category":"","costPerBasis":15,"kcalPer100":null,"proteinPer100":null,"fatPer100":null,"carbsPer100":null,"lowAt":null,"weightPerUnit":null,"supplier":null},{"id":"soy-sauce","name":"Soy sauce","unit":"ml","kind":null,"category":"","costPerBasis":40,"kcalPer100":null,"proteinPer100":null,"fatPer100":null,"carbsPer100":null,"lowAt":null,"weightPerUnit":null,"supplier":null},{"id":"mayo","name":"Mayo","unit":"g","kind":null,"category":"","costPerBasis":100,"kcalPer100":null,"proteinPer100":null,"fatPer100":null,"carbsPer100":null,"lowAt":null,"weightPerUnit":null,"supplier":null}],"recipes":[{"productId":"rolls-salmon-roll","dish":"Salmon roll","bom":[{"supply":"salmon","qty":50,"net":40,"yield":38},{"supply":"rice","qty":120,"net":null,"yield":null},{"supply":"nori","qty":1,"net":null,"yield":null}]},{"productId":"rolls-tuna-roll","dish":"Tuna roll","bom":[{"supply":"rice","qty":90,"net":null,"yield":null},{"supply":"mayo","qty":5,"net":null,"yield":null}]}],"withoutRecipe":[],"retired":[],"flattened":[],"warnings":[]}"#;

#[test]
fn a_flat_export_reads_exactly_as_before_in_either_mode() {
    assert_eq!(run(FLAT_ING, FLAT_REC, true).as_json(), FLAT_BEFORE);
    assert_eq!(run(FLAT_ING, FLAT_REC, false).as_json(), FLAT_BEFORE);
    assert!(run(FLAT_ING, FLAT_REC, true).preps.is_empty());
}

const ING: &str = "name,unit\nSalmon,g\nRice,g\nMayo,g\nSriracha,g\nWater,ml\nVinegar,ml\nSalt,g\nSugar,g\n";
const SAUCE: &str = "dish,ingredient,qty,unit,batch\n\
    Spicy mayo,Mayo,800,g,1 kg\n\
    Spicy mayo,Sriracha,200,g,\n\
    Salmon roll,Salmon,40,g,\n\
    Salmon roll,Spicy mayo,15,g,\n\
    Salmon roll,Rice,100,g,\n";

#[test]
fn a_prepack_becomes_a_semi_finished_product_the_dish_names() {
    let d = run(ING, SAUCE, true);
    assert_eq!(d.preps, vec![DraftPrep { id: "spicy-mayo".into(), name: "Spicy mayo".into(), unit: "g", lines: vec![("mayo".into(), 800), ("sriracha".into(), 200)], yield_qty: 1000 }], "{:?}", d.warnings);
    assert_eq!(bom(&d, "rolls-salmon-roll"), vec![("salmon".into(), 40), ("spicy-mayo".into(), 15), ("rice".into(), 100)]);
    assert!(d.flattened.is_empty() && d.warnings.is_empty(), "{:?} {:?}", d.flattened, d.warnings);
    assert!(d.as_json().ends_with(r#","preps":[{"id":"spicy-mayo","name":"Spicy mayo","unit":"g","yield":1000,"lines":[{"item":"mayo","qty":800},{"item":"sriracha","qty":200}]}]}"#), "{}", d.as_json());
    // The twin: without the flag the same file flattens, as it always did.
    let flat = run(ING, SAUCE, false);
    assert!(flat.preps.is_empty());
    assert_eq!(bom(&flat, "rolls-salmon-roll"), vec![("salmon".into(), 40), ("mayo".into(), 12), ("sriracha".into(), 3), ("rice".into(), 100)]);
}

#[test]
fn a_portion_that_would_split_a_gram_is_fine_once_the_sauce_is_real() {
    // 7 g of the sauce is 5.6 g of mayo: flattening refuses the dish ...
    let rec = SAUCE.replace("Spicy mayo,15,g", "Spicy mayo,7,g");
    assert!(run(ING, &rec, false).recipes.is_empty());
    // ... a real semi-finished product carries it exactly (the ledger's carry, `stock::carry`).
    assert_eq!(bom(&run(ING, &rec, true), "rolls-salmon-roll"), vec![("salmon".into(), 40), ("spicy-mayo".into(), 7), ("rice".into(), 100)]);
}

#[test]
fn a_nested_card_is_created_children_first() {
    let rec = "dish,ingredient,qty,unit,batch\n\
        Rice seasoned,Rice,1000,g,2100 g\n\
        Rice seasoned,Water,1100,ml,\n\
        Rice seasoned,Mitsukan,250,g,\n\
        Mitsukan,Vinegar,800,ml,1000 g\n\
        Mitsukan,Salt,50,g,\n\
        Mitsukan,Sugar,150,g,\n\
        Philadelphia,Rice seasoned,130,g,\n\
        Philadelphia,Salmon,60,g,\n";
    let d = run(ING, rec, true);
    let ids: Vec<&str> = d.preps.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["mitsukan", "rice-seasoned"], "{:?}", d.warnings);
    assert_eq!(d.preps[1].lines, vec![("rice".into(), 1000), ("water".into(), 1100), ("mitsukan".into(), 250)]);
    assert_eq!(d.preps[1].yield_qty, 2100);
    assert_eq!(bom(&d, "rolls-philadelphia"), vec![("rice-seasoned".into(), 130), ("salmon".into(), 60)]);
}

#[test]
fn a_card_that_cannot_be_real_is_flattened_and_said() {
    // Half a gram on the card: not a card line; the dish takes the leaves.
    let rec = "dish,ingredient,qty,unit,batch\n\
        Spicy mayo,Mayo,1999.5,g,2 kg\n\
        Spicy mayo,Sriracha,0.5,g,\n\
        Salmon roll,Spicy mayo,4000,g,\n";
    let d = run(ING, rec, true);
    assert!(d.preps.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("\"Spicy mayo\" is not created") && w.contains("1999.5")), "{:?}", d.warnings);
    assert_eq!(bom(&d, "rolls-salmon-roll"), vec![("mayo".into(), 3999), ("sriracha".into(), 1)]);
    // The twin: whole grams make a real one.
    let d = run(ING, &rec.replace("1999.5", "1999").replace("0.5,g", "1,g").replace("2 kg", "2000 g"), true);
    assert_eq!(d.preps.len(), 1, "{:?}", d.warnings);
}

#[test]
fn a_cycle_creates_neither_and_the_dish_is_refused_by_its_path() {
    let rec = "dish,ingredient,qty,unit,batch\n\
        Sauce a,Sauce b,100,g,1000 g\n\
        Sauce a,Mayo,900,g,\n\
        Sauce b,Sauce a,100,g,1000 g\n\
        Sauce b,Mayo,900,g,\n\
        Salmon roll,Salmon,40,g,\n\
        Salmon roll,Sauce a,10,g,\n";
    let d = run(ING, rec, true);
    assert!(d.preps.is_empty());
    assert_eq!(d.without_recipe, vec!["rolls-salmon-roll".to_string()]);
    assert!(d.warnings.iter().any(|w| w.contains("sauce-a → sauce-b → sauce-a")), "{:?}", d.warnings);
    assert!(d.warnings.iter().any(|w| w.contains("\"Sauce a\" is not created: semi-finished products name each other")), "{:?}", d.warnings);
}

fn catalogue(kind: Option<&str>) -> Vec<DraftSupply> {
    let mut v: Vec<DraftSupply> = ["salmon", "rice", "mayo", "sriracha"].iter().map(|s| DraftSupply::known(s, s, "g").unwrap()).collect();
    let mut sauce = DraftSupply::known("spicy-mayo", "Spicy mayo", "g").unwrap();
    sauce.kind = kind.map(str::to_string);
    v.push(sauce);
    v
}

#[test]
fn a_second_import_updates_the_semi_finished_product_it_made() {
    let m = menu();
    let o = opts(&m, true);
    let again = recipes_against(SAUCE, &o, &catalogue(Some("prep")));
    assert_eq!(again.preps.len(), 1, "{:?}", again.warnings);
    assert_eq!(bom(&again, "rolls-salmon-roll"), vec![("salmon".into(), 40), ("spicy-mayo".into(), 15), ("rice".into(), 100)]);
    // Idempotent: the same file against the same catalogue, the same draft.
    assert_eq!(again.as_json(), recipes_against(SAUCE, &o, &catalogue(Some("prep"))).as_json());
    // The twin: a RAW supply of that name is a clash, refused as before.
    let clash = recipes_against(SAUCE, &o, &catalogue(None));
    assert!(clash.warnings.iter().any(|w| w.contains("is both a supply and a semi-finished product")), "{:?}", clash.warnings);
}

#[test]
fn a_dish_may_name_the_kitchens_semi_finished_product_without_redefining_it() {
    let m = menu();
    let d = recipes_against("dish,ingredient,qty,unit\nSalmon roll,Salmon,40,g\nSalmon roll,Spicy mayo,15,g\n", &opts(&m, true), &catalogue(Some("prep")));
    assert!(d.preps.is_empty());
    assert_eq!(bom(&d, "rolls-salmon-roll"), vec![("salmon".into(), 40), ("spicy-mayo".into(), 15)], "{:?}", d.warnings);
}
