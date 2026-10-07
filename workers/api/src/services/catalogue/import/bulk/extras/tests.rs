//! The supplies file's extra columns: read beside the hub's parser, keyed by
//! the same id it mints, a bad value a warning and never a guess.

use super::*;

#[test]
fn losses_and_the_pack_are_read_by_the_rows_id() {
    let csv = "id,name,unit,clean_pm,cook_pm,pack,pack_qty\n\
               pack-salmon,Salmon,g,550,,box,5000\n\
               pack-rice,\"Rice, sushi\",g,,2200,thes,10000\n\
               ,Nori Sheets,unit,,,,\n";
    let (x, warn) = read(csv);
    assert!(warn.is_empty(), "{warn:?}");
    assert_eq!(x["pack-salmon"], Extra { clean_pm: Some(550), cook_pm: None, pack: Some(Pack { name: "box".into(), qty: 5000 }) });
    assert_eq!(x["pack-rice"].cook_pm, Some(2200), "a quoted name with a comma is one cell");
    assert!(!x.contains_key("nori-sheets"), "a row with no extras is not listed");
}

#[test]
fn a_value_that_is_not_one_is_a_warning_and_left_out() {
    let csv = "name;unit;clean_pm;cook_pm;pack_qty\nSalmon;g;0;9999;-5\nTuna;g;700;;\n";
    let (x, warn) = read(csv);
    assert_eq!(warn.len(), 3, "{warn:?}");
    assert!(warn[0].contains("row 2") && warn[0].contains("clean_pm"));
    assert!(!x.contains_key("salmon"), "every value of the row refused: nothing kept");
    // The positive twin, in the same file: a good value is kept, keyed by the slugged name.
    assert_eq!(x["tuna"].clean_pm, Some(700));
}

/// Through the import's own turn: the dry run shows the extras and writes
/// nothing; Apply writes them through the form's `check` + `record`.
#[test]
fn the_import_turn_previews_then_writes_the_extras() {
    use super::super::{turn, BulkIn, Kind, Turn};
    use dowiz_hub::catalog::Catalog;
    let mut cat = Catalog::create().unwrap();
    cat.set_location(&serde_json::json!({ "id": "v1", "currency_code": "ALL" }).to_string());
    let text = "id,name,unit,kind,clean_pm,cook_pm,pack,pack_qty\npack-rice,Sushi rice,g,food_ingredient,,2200,10 kg,10000\npack-salmon,Salmon,g,food_ingredient,900,,,\n";
    let input = |apply| BulkIn { text: text.into(), kind: Kind::Supplies, hundredths: false, apply, retire: false, now_ms: 1_790_000_000_000, by: String::new() };
    let Turn::Shown(dry) = turn(&mut cat, &input(false)) else { panic!("a dry run shows") };
    let rice = dry["rows"].as_array().unwrap().iter().find(|r| r["id"] == "pack-rice").unwrap().clone();
    assert_eq!((rice["cookPm"].clone(), rice["pack"]["qty"].clone()), (serde_json::json!(2200), serde_json::json!(10000)));
    assert!(cat.supply("pack-rice").is_none(), "the dry run wrote nothing");
    let Turn::Written(_) = turn(&mut cat, &input(true)) else { panic!("Apply writes") };
    let r: serde_json::Value = serde_json::from_str(&cat.supply("pack-rice").unwrap()).unwrap();
    assert_eq!((r["cookPm"].clone(), r["packs"].clone()), (serde_json::json!(2200), serde_json::json!([{ "name": "10 kg", "qty": 10000 }])));
    let s: serde_json::Value = serde_json::from_str(&cat.supply("pack-salmon").unwrap()).unwrap();
    assert_eq!((s["cleanPm"].clone(), s.get("packs")), (serde_json::json!(900), None), "no pack column value: no pack");
    // Idempotent: the same file again is the same two records.
    let Turn::Written(_) = turn(&mut cat, &input(true)) else { panic!("again") };
    assert_eq!(cat.supplies().len(), 2);
}

#[test]
fn a_file_without_the_columns_reads_nothing() {
    let (x, warn) = read("name,unit,cost\nSalmon,g,2500\n");
    assert!(x.is_empty() && warn.is_empty());
    assert_eq!(cells(r#"a,"b ""c""",d"#, ','), vec!["a", "b \"c\"", "d"]);
}
