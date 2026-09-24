//! The one supply write, as the form and the bulk import both reach it.

use super::*;

fn body(id: &str) -> SupplyIn {
    SupplyIn { id: id.into(), ..SupplyIn::default() }
}

#[test]
fn a_unit_outside_the_three_is_refused_and_the_three_are_not() {
    let mut b = body("salmon");
    b.unit = Some("kg".into());
    assert_eq!(check(&b), Err("unit is g, ml or unit".to_string()));
    for u in ["g", "ml", "unit"] {
        b.unit = Some(u.into());
        assert_eq!(check(&b), Ok("salmon".to_string()), "{u}");
    }
    assert!(check(&body("  ")).is_err(), "an id is required");
    let mut b = body("salmon");
    b.cost_per_basis = Some(-1);
    assert!(check(&b).is_err());
}

#[test]
fn a_field_the_body_does_not_carry_keeps_what_was_there() {
    let old = json!({ "id": "salmon", "name": "Salmon", "unit": "g", "lowAt": 3000, "category": "Fish", "costPerBasis": 240, "kcalPer100": 208, "active": false });
    let mut b = body("salmon");
    b.cost_per_basis = Some(250);
    b.supplier = Some(" Tregu i peshkut ".into());
    let r = record("salmon", &b, &old);
    assert_eq!((r["costPerBasis"].clone(), r["kcalPer100"].clone(), r["lowAt"].clone()), (json!(250), json!(208), json!(3000)));
    assert_eq!((r["category"].clone(), r["supplier"].clone()), (json!("Fish"), json!("Tregu i peshkut")));
    // Written again is kept: a retired supply comes back unless told otherwise.
    assert_eq!(r["active"], json!(true));
    // TWIN: a new supply gets the defaults.
    let r = record("rice", &body("rice"), &json!({}));
    assert_eq!((r["name"].clone(), r["unit"].clone(), r["kind"].clone()), (json!("rice"), json!("g"), json!("food_ingredient")));
}
