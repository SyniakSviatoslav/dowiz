//! R6's CHECK, verbatim from the research: salmon 100 g gross at cleanPm 550
//! -> net 55, out 55; rice 100 g dry at cookPm 2200 -> out 220; a typed out
//! beats the default.

use super::*;
use serde_json::json;

#[test]
fn salmon_is_cleaned_and_rice_grows() {
    let salmon = json!({ "unit": "g", "cleanPm": 550 });
    let w = weights("g", 100, &salmon, None, None);
    assert_eq!((w.gross, w.net, w.out), (Some(100), Some(55), Some(55)));
    assert_eq!((w.yield_pm(), w.loss_pm()), (Some(550), Some(450)));
    let rice = json!({ "unit": "g", "cookPm": 2200 });
    let w = weights("g", 100, &rice, None, None);
    assert_eq!((w.net, w.out, w.loss_pm()), (Some(100), Some(220), Some(-1200)), "a negative loss is growth");
}

/// A weighed number on the line beats the supply's default, and is remembered
/// as the owner's; the other one still follows from it.
#[test]
fn a_typed_weight_beats_the_default() {
    let salmon = json!({ "unit": "g", "cleanPm": 550, "cookPm": 900 });
    let w = weights("g", 100, &salmon, Some(60), None);
    assert_eq!((w.net, w.out, w.net_set, w.out_set), (Some(60), Some(54), Some(60), None));
    let w = weights("g", 100, &salmon, None, Some(40));
    assert_eq!((w.net, w.out, w.out_set), (Some(55), Some(40), Some(40)));
}

/// A piece weighs its `weightPerUnit`; a piece with none has no weight, and
/// no default can invent one. Millilitres are grams.
#[test]
fn pieces_and_millilitres() {
    let egg = json!({ "unit": "unit", "weightPerUnit": 60, "cleanPm": 880 });
    let w = weights("unit", 2, &egg, None, None);
    assert_eq!((w.gross, w.net, w.out), (Some(120), Some(106), Some(106)));
    let w = weights("unit", 2, &json!({ "unit": "unit" }), None, None);
    assert_eq!((w.gross, w.out, w.yield_pm()), (None, None, None));
    assert_eq!(weights("ml", 30, &json!({}), None, None).out, Some(30));
}

/// A default outside its range is not a default; 1000 is.
#[test]
fn an_impossible_default_reads_as_none() {
    for bad in [json!(0), json!(-5), json!(1001), json!("550")] {
        assert_eq!(pm_of(&json!({ "cleanPm": bad }), "cleanPm", CLEAN_MAX), PM);
    }
    assert_eq!(pm_of(&json!({ "cookPm": 2200 }), "cookPm", COOK_MAX), 2200);
    assert_eq!(pm_of(&json!({ "cookPm": 5001 }), "cookPm", COOK_MAX), PM);
}

/// Half up, in integers.
#[test]
fn scaling_rounds_half_up() {
    assert_eq!(scale(1, 500), 1);
    assert_eq!(scale(1, 499), 0);
    assert_eq!(scale(333, 550), 183);
}

/// The edible share: net over gross, else the default.
#[test]
fn the_edible_share() {
    let w = weights("g", 200, &json!({ "cleanPm": 500 }), None, None);
    assert_eq!(w.edible(), 0.5);
    let w = weights("unit", 1, &json!({ "cleanPm": 800 }), None, None);
    assert_eq!(w.edible(), 0.8, "no gross weight: the default");
}

/// A typed weight that is not one is refused; its twin passes.
#[test]
fn a_typed_weight_is_checked() {
    assert!(check(Some(-1), None, Some(100)).is_err());
    assert!(check(None, Some(GRAMS_MAX + 1), Some(100)).is_err());
    assert!(check(Some(101), None, Some(100)).unwrap_err().contains("more than the gross"));
    assert!(check(Some(100), Some(300), Some(100)).is_ok(), "cooking may grow it");
    assert!(check(Some(50), None, None).is_ok(), "no gross weight to compare with");
}
