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

/// AUDIT D35: the form may not change what a supply is counted in; the CSV
/// importer already refused it. The twins: the same unit, no unit in the
/// body, and a brand-new supply are all allowed.
#[test]
fn the_form_refuses_a_unit_change_as_the_importer_does() {
    let old = json!({ "id": "salmon", "unit": "g" });
    let mut b = body("salmon");
    b.unit = Some("unit".into());
    let why = unit_change(&b, &old).expect("g -> unit must refuse");
    assert!(why.contains("salmon is counted in g") && why.contains("to unit"), "{why}");
    b.unit = Some("g".into());
    assert_eq!(unit_change(&b, &old), None, "the same unit");
    b.unit = None;
    assert_eq!(unit_change(&b, &old), None, "a body without a unit keeps the stored one");
    b.unit = Some("ml".into());
    assert_eq!(unit_change(&b, &json!({})), None, "a new supply takes any unit");
}

/// A field with no value is not stored at all: absent reads as null to every
/// reader, and the catalogue pays a cell per byte of `"supplier":null`.
#[test]
fn a_field_with_no_value_is_left_out_of_the_record() {
    let mut b = body("kuti");
    b.unit = Some("unit".into());
    let r = record("kuti", &b, &json!({}));
    let keys: Vec<&str> = r.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["active", "category", "id", "kind", "lowAt", "name", "nutritionConfirmed", "unit"]);
    assert_eq!(r["supplier"], Value::Null, "absent reads as null");
    // TWIN: a value given is stored.
    b.weight_per_unit = Some(12.0);
    b.supplier = Some("Metro".into());
    let r = record("kuti", &b, &json!({}));
    assert_eq!((r["weightPerUnit"].clone(), r["supplier"].clone()), (json!(12.0), json!("Metro")));
}
