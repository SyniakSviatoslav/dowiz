//! The option recipe's route, its pure halves (R13, W-LOST): what the screen
//! reads, what a save changes, and every refusal with its positive twin.

use super::*;

const ROLL: &str = r#"{"id":"roll","price":800,"bom":[{"supply":"salmon","qty":40}],
  "modifierGroups":[{"id":"extra","name":"Extras","min":0,"max":2,"options":[{"id":"xsalmon","name":"Extra salmon","priceDelta":200}]}]}"#;

fn supplies() -> Vec<(String, String)> {
    vec![
        ("salmon".into(), r#"{"id":"salmon","name":"Salmon","unit":"g","kind":"food_ingredient"}"#.into()),
        ("rice-seasoned".into(), format!(r#"{{"id":"rice-seasoned","name":"Rice","unit":"g","kind":"{}"}}"#, dowiz_hub::prep::KIND)),
    ]
}

fn supply(id: &str) -> Option<String> {
    supplies().into_iter().find(|(i, _)| i == id).map(|(_, j)| j)
}

fn body(option: &str, lines: &[(&str, i64)]) -> OptionBomIn {
    OptionBomIn { location_id: "v".into(), option: option.into(), bom: lines.iter().map(|(s, q)| LineIn { supply: s.to_string(), qty: *q }).collect() }
}

#[test]
fn a_save_sets_the_option_recipe_and_the_view_reads_it_back() {
    let mut p: Value = serde_json::from_str(ROLL).unwrap();
    apply(&mut p, &body("xsalmon", &[("salmon", 20)]), &supply).unwrap();
    let v = view("roll", &p.to_string(), &supplies());
    assert_eq!(v["contract"], CONTRACT);
    assert_eq!(v["options"][0]["id"], "xsalmon");
    assert_eq!(v["options"][0]["groupName"], "Extras");
    assert_eq!(v["options"][0]["bom"], json!([{ "supply": "salmon", "qty": 20 }]));
    assert_eq!(v["supplies"].as_array().unwrap().len(), 1, "a semi-finished product is not offered");
    assert_eq!(p["modifierGroups"], serde_json::from_str::<Value>(ROLL).unwrap()["modifierGroups"], "the guest's groups are untouched");
    apply(&mut p, &body("xsalmon", &[]), &supply).unwrap();
    assert_eq!(p, serde_json::from_str::<Value>(ROLL).unwrap(), "cleared: the record is what it was");
}

#[test]
fn what_a_save_refuses() {
    let mut p: Value = serde_json::from_str(ROLL).unwrap();
    let before = p.clone();
    assert!(apply(&mut p, &body("xsalmon", &[("tuna", 20)]), &supply).unwrap_err().contains("unknown supply"));
    assert!(apply(&mut p, &body("xsalmon", &[("rice-seasoned", 20)]), &supply).unwrap_err().contains("semi-finished"));
    assert!(apply(&mut p, &body("nope", &[("salmon", 20)]), &supply).is_err(), "not an option of this dish");
    assert!(apply(&mut p, &body("xsalmon", &[("salmon", -1)]), &supply).is_err());
    assert_eq!(p, before, "nothing changed");
    assert!(serde_json::from_str::<OptionBomIn>(r#"{"location_id":"v","option":"x","bom":[],"phone":"1"}"#).is_err(), "an unknown field is a 400");
}

#[test]
fn a_refusal_crosses_the_closure_and_a_real_error_does_not_pass_for_one() {
    assert_eq!(refusal("RustError: option-bom refused:400:unknown supply tuna"), Some((400, "unknown supply tuna".into())));
    assert_eq!(refusal("option-bom refused:404:not found"), Some((404, "not found".into())));
    assert_eq!(refusal("catalogue serialise failed"), None);
}

#[test]
fn the_object_answers_the_view_and_null_for_an_unknown_dish() {
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    for (id, j) in supplies() {
        cat.set_supply(&id, &j);
    }
    cat.set_product("roll", ROLL);
    let v = fold(&cat, "roll");
    assert_eq!((v["contract"].as_str(), v["options"][0]["id"].as_str()), (Some(CONTRACT), Some("xsalmon")));
    assert_eq!(v["supplies"].as_array().map(Vec::len), Some(1), "the raw supply only");
    assert!(fold(&cat, "nope").is_null(), "the twin: a dish the catalogue does not have");
}
