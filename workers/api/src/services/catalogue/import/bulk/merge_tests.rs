//! NO LOSS THROUGH THE FILE (2026-10-16): a recipes file that names one
//! supply on two rows of a dish lands ONE line with both quantities, never
//! the first row alone.

use super::tests::supplied_for_merge as supplied;
use super::*;

const NOW: i64 = 1_790_078_400_000;

fn product(cat: &Catalog, id: &str) -> Value {
    serde_json::from_str(&cat.product(id).unwrap()).unwrap()
}

#[test]
fn a_file_naming_a_supply_twice_lands_both_quantities() {
    let mut cat = supplied();
    let file = "dish,ingredient,qty,unit\nMaki Salmon,Salmon,35,g\nMaki Salmon,Oriz sushi,90,g\nMaki Salmon,Salmon,15,g\n";
    let (d, rows) = read(&cat, file, Kind::Recipes, CostScale::Major, NOW);
    assert_eq!(rows.len(), 1, "{:?}", d.warnings);
    assert_eq!(apply_recipes(&mut cat, &d), Ok(1));
    let p = product(&cat, "maki-salmon");
    let reserved: Vec<(String, i64)> = dowiz_hub::stock::bom_of(&p.to_string()).into_iter().map(|l| (l.supply, l.qty)).collect();
    assert_eq!(reserved, vec![("salmon".into(), 50), ("oriz-sushi".into(), 90)], "35 + 15 g of salmon");
    // TWIN: the same dish with the salmon on one row lands the same line.
    let mut once = supplied();
    let (d, _) = read(&once, "dish,ingredient,qty,unit\nMaki Salmon,Salmon,50,g\nMaki Salmon,Oriz sushi,90,g\n", Kind::Recipes, CostScale::Major, NOW);
    apply_recipes(&mut once, &d).unwrap();
    assert_eq!(product(&once, "maki-salmon")["bom"], p["bom"]);
}
