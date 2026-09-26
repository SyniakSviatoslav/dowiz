//! W0a: the lots a group is warned about -- near their date, still on the
//! shelf, soonest first; none when there are none.
use super::*;
use dowiz_hub::stock::lots::Lot;

fn lot(item: &str, expiry: Option<i64>, left: i64) -> Lot {
    Lot { item: item.into(), code: "L".into(), expiry, supplier: None, doc: None, at: None, unit_cost: None, per: None, received: left.max(1), left, seq: 0 }
}

fn named(id: &str) -> Option<Supply> {
    Some(Supply { name: id.to_uppercase(), unit: "g".into(), low_at: 0 })
}

#[test]
fn soon_lists_only_lots_on_the_shelf_within_the_window_soonest_first() {
    let a = lot("salmon", Some(20260928), 300);
    let b = lot("tuna", Some(20260926), 100);
    let far = lot("rice", Some(20261201), 5000);
    let gone = lot("eel", Some(20260926), 0);
    let undated = lot("nori", None, 50);
    let lots = [&a, &b, &far, &gone, &undated];
    let d = soon(&lots, 20260926, &named).unwrap();
    let names: Vec<&str> = d["items"].as_array().unwrap().iter().map(|i| i["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["TUNA", "SALMON"]);
    assert_eq!(d["items"][0]["expiry"], "2026-09-26");
    // ITS TWIN: nothing near, nothing said.
    assert!(soon(&[&far, &undated], 20260926, &named).is_none());
}
