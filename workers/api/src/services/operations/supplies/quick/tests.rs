//! W-NOM: a list of names becomes ingredients -- ids minted, the form's rules
//! kept, what exists skipped, one bad line refusing the whole list.
use super::*;

fn stored() -> Vec<(String, String)> {
    vec![("salmon".into(), json!({ "id": "salmon", "name": "Salmon", "unit": "g" }).to_string())]
}

#[test]
fn names_become_ingredients_with_their_unit_and_group() {
    let items = vec![
        json!({ "name": "Rice", "unit": "g", "category": "Dry" }),
        json!({ "name": "Soy sauce", "unit": "ml", "category": "Sauces", "packs": [{ "name": "bottle 1 l", "qty": 1000 }] }),
        json!({ "name": "  SALMON " }),
        json!({ "name": "Rice!" }),
        json!({ "name": "Lids", "unit": "unit", "kind": "packaging", "barcode": "4006381333931" }),
    ];
    let a = plan(&items, &stored()).unwrap();
    let ids: Vec<&str> = a.write.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["rice", "soy-sauce", "rice-2", "lids"], "a clash takes a tail, inside the list too");
    assert_eq!(a.existing, vec!["SALMON".to_string()], "a name the venue has is skipped, whatever its case");
    let soy = &a.write[1].1;
    assert_eq!((soy["unit"].as_str(), soy["category"].as_str()), (Some("ml"), Some("Sauces")));
    assert_eq!(soy["packs"][0]["qty"], 1000);
    assert_eq!(soy["active"], true);
    assert_eq!(a.write[3].1["kind"], "packaging");
    assert_eq!(a.write[0].1["name"], "Rice", "the name is kept as typed, trimmed");
}

#[test]
fn one_bad_line_refuses_the_list_and_its_twin_passes() {
    for bad in [json!({ "name": "" }), json!({ "name": "X", "unit": "kg" }), json!({ "name": "X", "barcode": "12" }),
                json!({ "name": "X", "price": 5 }), json!("X"), json!({ "name": "x".repeat(81) })] {
        assert!(plan(&[json!({ "name": "Ok" }), bad.clone()], &[]).is_err(), "{bad}");
    }
    assert!(plan(&[json!({ "name": "x".repeat(80), "unit": "unit" })], &[]).is_ok());
    assert!(plan(&[], &[]).is_err(), "nothing to add is a 400");
    let many: Vec<Value> = (0..=ITEMS_MAX).map(|i| json!({ "name": format!("n{i}") })).collect();
    assert!(plan(&many, &[]).is_err());
    assert_eq!(plan(&many[..ITEMS_MAX], &[]).unwrap().write.len(), ITEMS_MAX);
}
