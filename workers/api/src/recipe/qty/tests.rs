//! A quantity read from its text, shown back, compared with whole units.

use super::*;
use serde_json::json;

#[test]
fn text_is_read_exactly_in_milli_units() {
    assert_eq!(parse("40").unwrap().milli, 40_000);
    assert_eq!(parse(" 40.5 ").unwrap().milli, 40_500);
    assert_eq!(parse("40,5").unwrap().milli, 40_500);
    assert_eq!(parse("0.001").unwrap().milli, 1);
    assert_eq!(parse(".5").unwrap().milli, 500);
    assert_eq!(parse("1500").unwrap().milli, 1_500_000);
    assert_eq!(parse("-2").unwrap().milli, -2_000);
    assert_eq!(parse("0.0005").unwrap_err(), "\"0.0005\": at most 3 decimals");
    for bad in ["", "abc", "1.2.3", "1e3", "40 g", "--1"] {
        assert!(parse(bad).is_err(), "{bad:?} is not a quantity");
    }
}

#[test]
fn json_numbers_and_strings_both_read_and_a_fourth_decimal_is_refused() {
    let q = |v: serde_json::Value| serde_json::from_value::<Qty>(v);
    assert_eq!(q(json!(40)).unwrap().milli, 40_000);
    assert_eq!(q(json!(40.5)).unwrap().milli, 40_500);
    assert_eq!(q(json!(0.1)).unwrap().milli, 100);
    assert_eq!(q(json!(1234.567)).unwrap().milli, 1_234_567);
    assert_eq!(q(json!("0,25")).unwrap().milli, 250);
    assert!(q(json!(0.0005)).unwrap_err().to_string().contains("3 decimals"));
    assert!(q(json!(true)).is_err());
    assert!(q(json!(null)).is_err());
}

#[test]
fn shown_as_the_shortest_exact_text_and_as_a_json_number() {
    assert_eq!(Qty::whole(40).show(), "40");
    assert_eq!(Qty::milli(40_500).show(), "40.5");
    assert_eq!(Qty::milli(1).show(), "0.001");
    assert_eq!(Qty::milli(-2_250).show(), "-2.25");
    assert_eq!(Qty::whole(40).json(), json!(40));
    assert_eq!(Qty::milli(40_500).json(), json!(40.5));
    assert_eq!(Qty::milli(1).json(), json!(0.001));
    assert_eq!(serde_json::to_value(Qty::milli(500)).unwrap(), json!(0.5));
}

#[test]
fn whole_rounded_and_the_hubs_millionths_agree() {
    assert_eq!(Qty::milli(40_500).uq(), 40_500_000);
    assert_eq!(Qty::of_micro(773_810).milli, 774, "half up to a thousandth");
    assert_eq!(Qty::of_micro(500).milli, 1);
    assert_eq!(Qty::milli(40_500).rounded(), 41);
    assert_eq!(Qty::milli(40_499).rounded(), 40);
    assert_eq!(Qty::milli(500).rounded(), 1);
    assert_eq!(Qty::whole(40).as_whole(), Some(40));
    assert_eq!(Qty::milli(40_500).as_whole(), None);
}

#[test]
fn a_quantity_compares_with_whole_base_units_as_the_old_bounds_do() {
    assert!(Qty::milli(500) > 0);
    assert!(!(Qty::milli(0) > 0));
    assert!(Qty::milli(100_000_001) > 100_000);
    assert!(!(Qty::whole(100_000) > 100_000));
    assert!(Qty::whole(3) == 3);
}

#[test]
fn stored_lines_read_whole_qty_or_exact_uq() {
    assert_eq!(Qty::from_stored(&json!({ "supply": "a", "qty": 40 })), Some(Qty::whole(40)));
    assert_eq!(Qty::from_stored(&json!({ "supply": "a", "uq": 500_000 })), Some(Qty::milli(500)));
    assert_eq!(Qty::from_stored(&json!({ "supply": "a", "qty": 40.5 })), None, "a fraction written as qty is not a line (the hub skips it too)");
    assert_eq!(Qty::from_stored(&json!({ "supply": "a" })), None);
}

#[test]
fn the_shelf_takes_a_decimal_that_is_whole_and_refuses_one_that_is_not() {
    #[derive(Deserialize, Debug)]
    struct Body {
        #[serde(default, deserialize_with = "whole_opt")]
        qty: Option<i64>,
    }
    let read = |v: serde_json::Value| serde_json::from_value::<Body>(v);
    assert_eq!(read(json!({ "qty": "1500" })).unwrap().qty, Some(1500));
    assert_eq!(read(json!({ "qty": "1500.000" })).unwrap().qty, Some(1500));
    assert_eq!(read(json!({ "qty": 1500 })).unwrap().qty, Some(1500));
    assert_eq!(read(json!({})).unwrap().qty, None);
    let e = read(json!({ "qty": 2.5 })).unwrap_err().to_string();
    assert!(e.contains("2.5: the shelf counts whole units"), "{e}");
}
