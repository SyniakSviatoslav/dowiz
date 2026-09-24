//! F1 (ROADMAP-2026-09-22, Wave F): supplies and recipes in bulk, in the three
//! languages a venue's spreadsheet is written in. Every test calls the real
//! reader; every refusal has its positive twin.

use super::super::{from_csv, CostScale, DraftSupply, Opts, RecipeDraft};
use super::recipes_against;

const NOW: i64 = 1_790_078_400_000;

fn menu() -> Vec<(String, String)> {
    vec![("maki-salmon".into(), "Maki Salmon".into()), ("maki-cucumber".into(), "Maki Cucumber".into())]
}

fn opts(products: &[(String, String)]) -> Opts<'_> {
    Opts {
        currency: "ALL",
        cost_scale: Some(CostScale::Major),
        unit_hint: None,
        local_now_ms: NOW,
        products,
        existing_supplies: &[],
    }
}

fn supplies(csv: &str) -> RecipeDraft {
    let m = menu();
    from_csv(csv, "", &opts(&m))
}

fn cost(d: &RecipeDraft, id: &str) -> Option<i64> {
    d.supplies.iter().find(|s| s.id == id).and_then(|s| s.cost_per_basis)
}

#[test]
fn three_languages_of_header_read_the_same_supply() {
    let sq = "emri;lloji;njësia;kalori;kosto;minimumi\nSalmon;food;kg;208;2400;3 kg\n";
    let en = "name,kind,unit,kcal,cost,low_at\nSalmon,food,kg,208,2400,3 kg\n";
    let uk = "назва;тип;одиниця;калорії;собівартість;мінімум\nSalmon;food;kg;208;2400;3 kg\n";
    let read: Vec<RecipeDraft> = [sq, en, uk].into_iter().map(supplies).collect();
    for d in &read {
        assert!(d.warnings.is_empty(), "{:?}", d.warnings);
        let s = &d.supplies[0];
        // 2400 lek per kg is 240 per 100 g; 3 kg of threshold is 3000 g.
        assert_eq!((s.id.as_str(), s.unit, s.cost_per_basis, s.kcal, s.low_at), ("salmon", "g", Some(240), Some(208.0), Some(3000)));
        assert_eq!(s.kind.as_deref(), Some("food_ingredient"));
    }
    assert_eq!(read[0].as_json(), read[1].as_json());
    assert_eq!(read[1].as_json(), read[2].as_json());
}

#[test]
fn a_grouped_price_and_a_plain_one_are_the_same_minor_units() {
    // Semicolons, because a comma-separated file cannot hold "1.200,00" unquoted.
    let grouped = supplies("name;unit;cost\nSalmon;kg;1.200,00\nRice;kg;1,200.00\nTuna;kg;\"1 200\"\n");
    let plain = supplies("name;unit;cost\nSalmon;kg;1200\nRice;kg;1200\nTuna;kg;1200\n");
    for id in ["salmon", "rice", "tuna"] {
        assert_eq!(cost(&grouped, id), Some(120), "{id}: {:?}", grouped.warnings);
        assert_eq!(cost(&grouped, id), cost(&plain, id));
    }
    // A million, grouped twice, is still exact.
    let d = supplies("name;unit;cost\nSaffron;kg;1.200.000\n");
    assert_eq!(cost(&d, "saffron"), Some(120_000), "{:?}", d.warnings);
}

#[test]
fn a_single_separator_before_three_digits_is_refused_not_guessed() {
    // "1.200" is 1200 in Durrës and 1.2 in London: the cost is left out, named.
    let d = supplies("name;unit;cost\nSalmon;kg;1.200\n");
    assert_eq!(cost(&d, "salmon"), None);
    assert!(d.warnings.iter().any(|w| w.contains("row 2") && w.contains("1.200") && w.contains("thousand")), "{:?}", d.warnings);
    // In a currency with cents, 1.2 per kg would come out EXACT (12 cents per
    // 100 g) -- so only the refusal stands between the file and a price a
    // thousand times too small.
    let m = menu();
    let mut o = opts(&m);
    o.currency = "EUR";
    let d = from_csv("name;unit;cost\nSalmon;kg;1.200\n", "", &o);
    assert_eq!(cost(&d, "salmon"), None, "{:?}", d.supplies);
    // TWIN: grouped with its decimal, the same file is exact: 1200 EUR per kg.
    let d = from_csv("name;unit;cost\nSalmon;kg;1.200,00\n", "", &o);
    assert_eq!(cost(&d, "salmon"), Some(12_000), "{:?}", d.warnings);
    // So is a group that is not three digits, and two separators of one kind
    // around a decimal of another kind in the wrong order.
    for bad in ["1.20,00", "12.00.000,5", "1,200,00.5.0"] {
        let d = supplies(&format!("name;unit;cost\nSalmon;kg;{bad}\n"));
        assert_eq!(cost(&d, "salmon"), None, "{bad}");
    }
    // TWIN: a decimal that is not three digits reads as a decimal.
    let d = supplies("name;unit;cost\nSalmon;kg;1200,5\n");
    assert!(d.warnings.iter().any(|w| w.contains("refused")), "half a lek per 100 g: {:?}", d.warnings);
    let d = supplies("name;unit;cost\nSalmon;kg;18,50\n");
    assert_eq!(d.supplies[0].cost_per_basis, None, "1.85 lek per 100 g is refused, not rounded");
}

#[test]
fn a_unit_outside_g_ml_unit_is_a_warning_naming_the_row() {
    let d = supplies("name,unit\nSalmon,g\nSoy sauce,ml\nBox,unit\nSake,cup\nRice,kg\n");
    let ids: Vec<&str> = d.supplies.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, vec!["salmon", "soy-sauce", "box", "rice"], "{:?}", d.warnings);
    assert_eq!(d.warnings.len(), 1, "{:?}", d.warnings);
    assert!(d.warnings[0].contains("row 5") && d.warnings[0].contains("Sake") && d.warnings[0].contains("cup"), "{:?}", d.warnings);
}

#[test]
fn per_says_how_much_the_cost_is_for() {
    let d = supplies("name,unit,cost,per\nNori,g,4500,1 kg\nBox,unit,420,12\nMilk,ml,90,1 l\nSalt,g,30,\n");
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(cost(&d, "nori"), Some(450), "4500 per 1000 g is 450 per 100 g");
    assert_eq!(cost(&d, "box"), Some(35), "420 per 12 pieces is 35 a piece");
    assert_eq!(cost(&d, "milk"), Some(9));
    assert_eq!(cost(&d, "salt"), Some(3000), "no per: the cost is for one gram, as before");
    // A per in another base is refused with the row, and the supply still imports.
    let d = supplies("name,unit,cost,per\nNori,g,4500,1 l\n");
    assert_eq!((d.supplies.len(), cost(&d, "nori")), (1, None));
    assert!(d.warnings.iter().any(|w| w.contains("row 2") && w.contains("per")), "{:?}", d.warnings);
}

fn catalogue() -> Vec<DraftSupply> {
    [("oriz-sushi", "Oriz sushi", "g"), ("nori", "Nori", "g"), ("salmon", "Salmon", "g"), ("kuti", "Kuti", "unit")]
        .into_iter()
        .filter_map(|(i, n, u)| DraftSupply::known(i, n, u))
        .collect()
}

#[test]
fn recipes_match_dish_and_supply_by_normalised_name() {
    let m = menu();
    let rec = "dish,ingredient,qty,unit\nmaki  SALMON,oriz sushi,90,g\nMaki Salmon,NORI,2,g\nMaki Salmon,Salmon,35,g\nMaki Salmon,Kuti,1,unit\n";
    let d = recipes_against(rec, &opts(&m), &catalogue());
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    let lines: Vec<(&str, i64)> = d.recipes[0].lines.iter().map(|l| (l.supply.as_str(), l.qty)).collect();
    assert_eq!(d.recipes[0].product_id, "maki-salmon");
    assert_eq!(lines, vec![("oriz-sushi", 90), ("nori", 2), ("salmon", 35), ("kuti", 1)]);
    // This step changes no supply and retires none.
    assert!(d.supplies.is_empty() && d.retired.is_empty());
}

#[test]
fn an_unmatched_name_is_named_and_its_dish_left_out() {
    let m = menu();
    let rec = "страва;інгредієнт;кількість;од.\nMaki Salmon;Salmon;35;g\nMaki Salmon;Wasabi;5;g\nMaki Cucumber;Oriz sushi;90;g\nMaki Tuna;Oriz sushi;90;g\n";
    let d = recipes_against(rec, &opts(&m), &catalogue());
    // The unknown supply: named with its row, the dish gets no recipe at all.
    assert!(d.warnings.iter().any(|w| w.contains("row 3") && w.contains("Wasabi")), "{:?}", d.warnings);
    assert_eq!(d.without_recipe, vec!["maki-salmon".to_string()]);
    // The unknown dish: named, left out.
    assert!(d.warnings.iter().any(|w| w.contains("Maki Tuna") && w.contains("not on the menu")), "{:?}", d.warnings);
    // TWIN: the dish whose every line matched still lands.
    assert_eq!(d.recipes.len(), 1);
    assert_eq!(d.recipes[0].product_id, "maki-cucumber");
}

#[test]
fn a_catalogue_record_in_another_unit_is_not_matchable() {
    assert!(DraftSupply::known("x", "X", "kg").is_none());
    assert!(DraftSupply::known("x", "X", "unit").is_some());
    let m = menu();
    let d = recipes_against("dish,ingredient,qty,unit\nMaki Salmon,Salmon,35,g\n", &opts(&m), &[]);
    assert!(d.recipes.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("supplies file first")), "{:?}", d.warnings);
}

/// The sample pair the owner is shown (`tools/recipes/samples/`) reads CLEAN
/// through the real parser, so the format it teaches cannot rot.
#[test]
fn the_sample_pair_reads_clean() {
    let sup = include_str!("../../../../../../tools/recipes/samples/supplies.csv");
    let rec = include_str!("../../../../../../tools/recipes/samples/recipes.csv");
    let dishes: Vec<(String, String)> = ["Sake Futomaki", "Philadelphia Classic", "California Classic", "Maki Salmon", "Maki Cucumber"]
        .into_iter()
        .map(|n| (crate::import::slug(n), n.to_string()))
        .collect();
    let d = from_csv(sup, "", &opts(&dishes));
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.supplies.len(), 10);
    assert!(d.supplies.iter().all(|s| s.cost_per_basis.is_some()), "every sample cost is exact");
    let catalogue: Vec<DraftSupply> = d.supplies.iter().filter_map(|s| DraftSupply::known(&s.id, &s.name, s.unit)).collect();
    let r = recipes_against(rec, &opts(&dishes), &catalogue);
    assert!(r.warnings.is_empty() && r.without_recipe.is_empty(), "{:?}", r.warnings);
    assert_eq!(r.recipes.len(), 5);
    assert_eq!(r.recipes.iter().map(|x| x.lines.len()).sum::<usize>(), 29);
}
