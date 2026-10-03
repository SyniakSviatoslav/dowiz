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
