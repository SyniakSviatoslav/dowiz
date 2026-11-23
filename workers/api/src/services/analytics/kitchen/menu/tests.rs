//! The matrix, on a fixture whose answer is the textbook's.

use super::*;

fn dish(id: &str, sold: i64, price: i64, cost: Option<i64>) -> MenuIn {
    MenuIn {
        id: id.into(),
        name: id.to_uppercase(),
        sold,
        revenue: sold * price,
        stamped: if cost.is_some() { sold } else { 0 },
        stamped_cogs: cost.map_or(0, |c| c * sold),
        today: None,
        costliest: cost.map(|c| ("salmon".into(), "Salmon".into(), c / 2)),
        ..MenuIn::default()
    }
}

fn find<'a>(v: &'a Value, id: &str) -> &'a Value {
    v["dishes"].as_array().unwrap().iter().find(|d| d["id"] == id).unwrap()
}

/// Popularity line: 70 % of 1/5 of 270 portions = 37.8. Average margin over
/// the costed dishes: 114000 / 220 = 518 per portion.
#[test]
fn each_dish_lands_in_the_textbook_quadrant_and_an_uncosted_one_in_none() {
    let rows = vec![
        dish("a", 100, 1000, Some(300)), // popular, 700 a portion
        dish("b", 100, 800, Some(500)),  // popular, 300
        dish("c", 10, 1500, Some(300)),  // rare, 1200
        dish("d", 10, 600, Some(400)),   // rare, 200
        dish("e", 50, 900, None),        // popular, no recipe
    ];
    let v = matrix(&rows);
    let q = |id| find(&v, id)["quadrant"].as_str().map(str::to_string);
    assert_eq!([q("a"), q("b"), q("c"), q("d")], [Some("star".into()), Some("plowhorse".into()), Some("puzzle".into()), Some("dog".into())]);
    let e = find(&v, "e");
    assert_eq!((e["quadrant"].is_null(), e["costUnknown"].as_bool(), e["action"].as_str()), (true, Some(true), Some("cost_unknown")));
    assert_eq!(e["popular"], true, "popularity needs no cost");
    assert!(e.get("cogs").is_none() && e.get("margin").is_none(), "no cost is invented: {e}");
    assert_eq!((v["averageMarginPortion"].as_i64(), v["popularPm"].as_i64(), v["unknown"].as_i64()), (Some(518), Some(140), Some(1)));
    assert_eq!(find(&v, "b")["raiseBy"], 218, "518 - 300");
    assert!(find(&v, "a")["raiseBy"].is_null(), "a star needs no rise");
    assert_eq!(find(&v, "b")["costliest"]["name"], "Salmon");
    let order: Vec<&str> = v["dishes"].as_array().unwrap().iter().map(|d| d["id"].as_str().unwrap()).collect();
    assert_eq!(order, ["a", "b", "c", "d", "e"], "stars first, unknown last");
}

/// A portion sold before stamps takes today's recipe cost, and the row says
/// so; with no recipe cost either, the dish is unknown even if some portions
/// were stamped. Twin: all stamped is "stamped".
#[test]
fn the_cost_is_the_stamp_else_todays_recipe_else_unknown() {
    let mut d = dish("x", 4, 1000, Some(300));
    assert_eq!(cogs(&d), Some((1200, "stamped")));
    d.stamped = 2;
    d.stamped_cogs = 600;
    assert_eq!(cogs(&d), None, "half stamped, no recipe cost: unknown, not half a cost");
    d.today = Some(350);
    assert_eq!(cogs(&d), Some((1300, "mixed")));
    d.stamped = 0;
    d.stamped_cogs = 0;
    assert_eq!(cogs(&d), Some((1400, "today")));
}

/// Nothing sold, or nothing costed: an empty matrix and no average, never a
/// division by zero.
#[test]
fn an_empty_or_uncosted_window_has_no_average() {
    let v = matrix(&[]);
    assert_eq!((v["count"].as_i64(), v["averageMarginPortion"].is_null(), v["popularPm"].is_null()), (Some(0), true, true));
    let v = matrix(&[dish("e", 5, 900, None), dish("z", 0, 900, Some(1))]);
    assert_eq!(v["count"], 1, "a dish not sold is not on the matrix");
    assert!(v["averageMarginPortion"].is_null());
    assert!(find(&v, "e")["quadrant"].is_null());
}

/// THE MENU TODAY: every dish, sold or not, its portion cost from today's
/// recipe against today's price; a dish whose cost is unknown NAMES what is
/// missing -- the unpriced supply, or no recipe. Twin: a priced one has none.
#[test]
fn every_dish_today_has_its_margin_or_names_what_is_missing() {
    use super::super::sales::DishLine;
    let line = |s: &str, qty: i64| DishLine { supply: s.into(), qty: qty.into(), gross_g: Some(qty), net_g: Some(qty), out_g: Some(qty) };
    let sup = |id: &str, cost: Option<i64>| (id.to_string(), Supply { id: id.into(), name: id.to_uppercase(), unit: "g".into(), basis: 100, list_cost: cost, clean_pm: 1000, cook_pm: 1000 });
    let supplies = HashMap::from([sup("salmon", Some(200)), sup("rice", Some(20)), sup("eel", None)]);
    let d = |id: &str, lines: Vec<DishLine>| (id.to_string(), Dish { id: id.into(), name: id.into(), lines, leaves: vec![] });
    let dishes = HashMap::from([d("sake", vec![line("salmon", 100), line("rice", 100)]), d("unagi", vec![line("eel", 50), line("rice", 100), line("ghost", 1)]), d("cola", vec![])]);
    let book = CostBook::default();
    let price = |id: &str| Some(match id { "sake" => 1000, "unagi" => 1200, _ => 200 });
    let v = today(&dishes, &supplies, &book, &price);
    let row = |id: &str| v.iter().find(|r| r["id"] == id).unwrap().clone();
    let sake = row("sake");
    assert_eq!((sake["portionCost"].as_i64(), sake["marginPortion"].as_i64(), sake["foodCostPm"].as_i64()), (Some(220), Some(780), Some(220)));
    assert_eq!(sake["missing"], json!([]));
    let unagi = row("unagi");
    assert!(unagi["portionCost"].is_null() && unagi["marginPortion"].is_null() && unagi["foodCostPm"].is_null(), "no cost is invented: {unagi}");
    assert_eq!(unagi["missing"], json!([{ "id": "eel", "name": "EEL" }, { "id": "ghost", "name": "ghost" }]), "the unpriced supply and the one the catalogue lost");
    assert_eq!((row("cola")["hasRecipe"].as_bool(), row("cola")["missing"].clone()), (Some(false), json!([])));
    assert_eq!(v.iter().map(|r| r["id"].as_str().unwrap()).collect::<Vec<_>>(), ["cola", "sake", "unagi"]);
}

/// The dish sheet's history: which part of the cost is stamped and which
/// is today's recipe, and how many portions the till's lines added.
/// Twin: an unknown cost names what it waits on and invents no cost.
#[test]
fn a_sold_row_says_which_basis_each_cost_has() {
    let mut d = dish("x", 4, 1000, Some(300));
    d.stamped = 1;
    d.stamped_cogs = 300;
    d.today = Some(350);
    d.attributed = 2;
    d.has_recipe = true;
    let v = matrix(&[d]);
    let x = find(&v, "x");
    assert_eq!((x["stamped"].as_i64(), x["stampedCogs"].as_i64(), x["todayPortions"].as_i64(), x["todayCogs"].as_i64()), (Some(1), Some(300), Some(3), Some(1050)));
    assert_eq!((x["attributed"].as_i64(), x["costBasis"].as_str(), x["foodCostPm"].as_i64()), (Some(2), Some("mixed"), Some(337)));
    let mut u = dish("u", 2, 500, None);
    u.missing = vec![("eel".into(), "Eel".into())];
    u.has_recipe = true;
    let v = matrix(&[u]);
    let u = find(&v, "u");
    assert_eq!((u["missing"].clone(), u["todayPortions"].as_i64(), u["hasRecipe"].as_bool()), (json!([{ "id": "eel", "name": "Eel" }]), Some(2), Some(true)));
    assert!(u["todayCogs"].is_null() && u.get("cogs").is_none(), "no cost is invented: {u}");
}
