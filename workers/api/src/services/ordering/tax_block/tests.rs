//! The block, with the lek figures of blueprint §3.2 — ported, not re-derived.

use super::*;
use crate::services::ordering::tax_cfg::VenueTax;
use dowiz_core::tax::RatePpm;

const V20: VenueTax =
    VenueTax { default: RatePpm(200_000), inclusive: true, fee: RatePpm(200_000) };

fn order(items: Value, total: i64) -> Value {
    json!({ "order_id": "o1", "items": items, "total": total })
}

fn coffees() -> Value {
    json!([{ "product_id": "coffee", "quantity": 3, "unit_price": 250 }])
}

/// Example 1: three coffees at 250 lek, 20 % inclusive → 125, not 126.
#[test]
fn example_1_the_block_is_per_group_125_not_126() {
    let mut e = order(coffees(), 750);
    stamp(&mut e, &V20, 0, 0, 0).expect("taxed");
    assert_eq!(e["tax"]["groups"], json!([{ "rate_ppm": 200000, "base": 750, "tax": 125, "lines": 1 }]));
    assert_eq!(e["tax"]["total"], 125);
    assert_eq!(e["tax"]["inclusive"], true);
    assert_eq!(e["items"][0]["vat_ppm"], 200000, "the line carries the rate it was charged at");
    assert_eq!(e["total"], 750, "inclusive: the total does not change meaning");
}

/// Example 2 + §3.4: the promo basket. Base is `subtotal − cut`, 675 → 112.
#[test]
fn example_2_the_discounted_base_is_taxed_and_the_fee_is_its_own_group() {
    let mut e = order(coffees(), 750 - 75 + 300);
    stamp(&mut e, &V20, 300, 0, 75).expect("taxed");
    let t = &e["tax"];
    assert_eq!(t["groups"][0]["base"], 675, "subtotal - cut");
    assert_eq!(t["groups"][0]["tax"], 112, "round the NET, not the tax");
    assert_eq!(t["fee"], json!({ "rate_ppm": 200000, "base": 300, "tax": 50 }));
    assert_eq!(t["total"], 162);
    assert_eq!(t["discount_allocated"], json!([75]));
}

#[test]
fn a_lines_own_rate_wins_and_makes_its_own_group() {
    let items = json!([
        { "product_id": "roll", "quantity": 1, "unit_price": 900 },
        { "product_id": "water", "quantity": 2, "unit_price": 150, "vat_ppm": 60000 },
    ]);
    let mut e = order(items, 1200);
    stamp(&mut e, &V20, 0, 0, 0).expect("taxed");
    assert_eq!(e["items"][1]["vat_ppm"], 60000);
    assert_eq!(e["items"][0]["vat_ppm"], 200000);
    assert_eq!(e["tax"]["groups"].as_array().unwrap().len(), 2);
    assert_eq!(e["tax"]["groups"][0]["rate_ppm"], 60000, "ascending by rate");
}

#[test]
fn exclusive_prices_add_the_tax_to_the_stored_total() {
    let v = VenueTax { default: RatePpm(88_750), inclusive: false, fee: RatePpm(0) };
    let mut e = order(json!([{ "product_id": "b", "quantity": 1, "unit_price": 1000 }]), 1000);
    stamp(&mut e, &v, 0, 0, 0).expect("taxed");
    assert_eq!(e["tax"]["total"], 89, "8.875% of 1000 = 88.75, rounded half-up");
    assert_eq!(e["total"], 1089);
}

#[test]
fn an_inclusive_total_that_disagrees_with_equation_6_is_refused_by_name() {
    let mut e = order(coffees(), 751);
    let why = stamp(&mut e, &V20, 0, 0, 0).expect_err("two totals must not be stored");
    assert!(why.contains("751") && why.contains("750"), "{why}");
    assert!(e.get("tax").is_none(), "nothing written on a refusal");
}

#[test]
fn a_line_that_cannot_be_read_is_a_refusal_not_a_zero() {
    for bad in [
        json!([{ "product_id": "x", "quantity": 1 }]),
        json!([{ "product_id": "x", "unit_price": 100 }]),
        json!([{ "product_id": "x", "quantity": 1, "unit_price": 100, "vat_ppm": 0.2 }]),
        json!("not a list"),
    ] {
        let mut e = order(bad.clone(), 100);
        assert!(stamp(&mut e, &V20, 0, 0, 0).is_err(), "{bad}");
    }
}
