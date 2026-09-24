//! THE LIVE HARNESS: today's real ebills.al answers through the real parsers,
//! the real mapper and the real import -- read from files a read-only fetch
//! left on disk, never from the network (a native test has no `Fetch`).
//! `#[ignore]`d: it needs `EBILLS_LIVE_DIR`, and prints COUNTS only -- the
//! bodies hold a waiter's name, the TIN and the signing certificate (§1.7).
//!
//!   EBILLS_LIVE_DIR=<dir> [EBILLS_MENU=<dowiz public menu json>]
//!   [EBILLS_MAPPING_OUT=<file>] cargo test --lib ebills::live -- --ignored --nocapture

use super::import::{decide, Lookups, Mapped, Waiting};
use super::map::{floor, to_order, to_paid};
use super::state::{norm, suggest};
use super::{classify, parse_detail, parse_items, parse_list, parse_tables, Kind};
use crate::hubdo::OrderView;
use serde_json::{json, Value};

fn body(dir: &str, name: &str) -> Option<(u16, String)> {
    let raw = std::fs::read_to_string(format!("{dir}/{name}")).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    Some((v["status"].as_u64()? as u16, v["body"].as_str()?.to_string()))
}

/// Word-set overlap of two normalised names, in percent (harness only).
fn overlap(a: &str, b: &str) -> u32 {
    let (x, y): (Vec<&str>, Vec<&str>) = (a.split(' ').collect(), b.split(' ').collect());
    let common = x.iter().filter(|w| y.contains(w)).count() as u32;
    let all = (x.len() + y.len()) as u32 - common;
    if all == 0 { 0 } else { common * 100 / all }
}

#[test]
#[ignore]
fn todays_real_answers_parse_map_and_import() {
    let Ok(dir) = std::env::var("EBILLS_LIVE_DIR") else { return };
    let (s, list_body) = body(&dir, "list.json").expect("list.json");
    assert_eq!(s, 200);
    let list = parse_list(&list_body).expect("the real list parses");
    let bills = list.sales.iter().filter(|x| classify(x) == Ok(Kind::Bill)).count();
    println!("list: {} rows, {} bills, {} others (open-table courses and counter sales)", list.sales.len(), bills, list.sales.len() - bills);

    let mut sales: Vec<Mapped> = Vec::new();
    let mut refused: Vec<String> = Vec::new();
    for row in list.sales.iter().filter(|x| classify(x) == Ok(Kind::Bill)) {
        match to_paid(row) {
            Ok(paid) => sales.push(Mapped::Bill { sale_id: row.id, paid }),
            Err(e) => refused.push(format!("{}: {e:?}", row.id)),
        }
    }
    let (mut courses, mut counter, mut gone) = (0, 0, 0);
    let mut names: Vec<(String, String, i64)> = Vec::new();
    let mut files: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()?.file_name().into_string().ok()).filter(|n| n.starts_with("sale-")).collect();
    files.sort();
    for f in files {
        let (status, b) = body(&dir, &f).unwrap();
        if status == 404 {
            gone += 1;
            continue;
        }
        let sale = parse_detail(&b).unwrap_or_else(|e| panic!("{f}: the real detail does not parse: {e}"));
        match classify(&sale) {
            Ok(Kind::Course) => courses += 1,
            Ok(Kind::CounterSale) => counter += 1,
            _ => {}
        }
        let m = match classify(&sale) {
            Ok(Kind::Bill) => to_paid(&sale).map(|paid| Mapped::Bill { sale_id: sale.id, paid }),
            _ => to_order(&sale, "loc-live").map(|envelope| Mapped::Order { sale_id: sale.id, envelope }),
        };
        match m {
            Ok(m) => sales.push(m),
            Err(e) => refused.push(format!("{}: {e:?}", sale.id)),
        }
        for r in sale.sale_records.iter().flatten() {
            if let (Some(code), Ok(p)) = (r.item_in_sale.as_ref().and_then(|i| i.item_code.clone()), super::lek(r.price, "p")) {
                names.push((code, r.item_name.clone(), p));
            }
        }
    }
    println!("details: {courses} courses, {counter} counter sales, {gone} gaps (404); mapping refused {}", refused.len());
    for r in &refused {
        println!("  refused {r}");
    }

    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let mut stock = dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap();
    let none = |_: &str| None;
    let look = Lookups { map: &none, product: &none };
    let out = decide(&mut hub, &mut stock, &[], &look, Waiting::default(), &sales, 1_790_200_000_000).expect("import");
    println!("import: {} placed, {} paid (courses), {} bills waiting, {} refused", out.placed, out.paid, out.pending.len(), out.refused.len());
    for b in &out.pending {
        println!("  waiting: bill {} table {} total {}", b.sale_id, b.paid["table"], b.paid["total"]);
    }
    let listed: Vec<OrderView> = crate::hubstore::orders_state(&hub).into_iter().map(OrderView::of_event).collect();
    for v in &listed {
        let o: Value = serde_json::from_str(&v.order_json).unwrap();
        let lines: i64 = o["items"].as_array().unwrap().iter().map(|i| i["unit_price"].as_i64().unwrap() * i["quantity"].as_i64().unwrap()).sum();
        assert_eq!(o["total"].as_i64(), Some(lines - o["discount"].as_i64().unwrap()), "law 3 on {}", v.order_id);
    }
    if let Ok(p) = std::env::var("EBILLS_DUMP_ORDERS") {
        let all: Vec<Value> = listed.iter().filter_map(|v| serde_json::from_str(&v.order_json).ok()).collect();
        std::fs::write(p, serde_json::to_string(&all).unwrap()).unwrap();
    }
    let bytes = hub.to_bytes_trimmed();
    let again = decide(&mut hub, &mut stock, &listed, &look, Waiting { bills: out.pending.clone(), leads: out.leads.clone() }, &sales, 1_790_200_300_000).expect("re-import");
    assert_eq!((again.placed, again.paid), (0, 0), "the same day twice places and pays nothing");
    assert_eq!(hub.to_bytes_trimmed(), bytes, "and writes nothing");
    let mut codes: Vec<String> = out.seen.iter().map(|s| s.0.clone()).collect();
    codes.sort();
    codes.dedup();
    println!("codes sold in the window: {} (all unmatched until the owner maps them)", codes.len());

    if let Some((200, t)) = body(&dir, "tables.json") {
        let rows = floor(&parse_tables(&t).expect("the real floor parses")).expect("whole lek");
        println!("floor: {} tables, {} occupied", rows.len(), rows.iter().filter(|r| r.occupied).count());
    }
    let items = body(&dir, "items.json").map(|(_, b)| parse_items(&b).expect("the real menu parses")).unwrap_or_default();
    println!("till menu: {} items with a code and a whole-lek price", items.len());

    let (Ok(menu), Ok(out_path)) = (std::env::var("EBILLS_MENU"), std::env::var("EBILLS_MAPPING_OUT")) else { return };
    let m: Value = serde_json::from_str(&std::fs::read_to_string(menu).unwrap()).unwrap();
    let products: Vec<(String, String, i64)> = m["categories"].as_array().unwrap().iter()
        .flat_map(|c| c["products"].as_array().cloned().unwrap_or_default())
        .map(|p| (p["id"].as_str().unwrap_or("").to_string(), p["name"].as_str().unwrap_or("").to_string(), p["price"].as_i64().unwrap_or(-1)))
        .collect();
    let (mut rows, mut apply) = (Vec::new(), Vec::new());
    let (mut exact, mut fuzzy) = (0, 0);
    for (code, name, price) in &items {
        let sold = codes.contains(code);
        let sug = suggest(name, *price, &products);
        let row = match sug.iter().find(|s| s.1) {
            Some((pid, _)) => { exact += 1; apply.push(json!({ "code": code, "product_id": pid })); json!({ "code": code, "ebills_name": name, "ebills_price": price, "product_id": pid, "confidence": "exact", "sold_in_window": sold }) }
            None => {
                let best = products.iter().map(|p| (overlap(&norm(name), &norm(&p.1)), p)).filter(|(o, _)| *o >= 50).max_by_key(|(o, p)| (*o, p.2 == *price));
                match best.or_else(|| sug.first().and_then(|s| products.iter().find(|p| p.0 == s.0)).map(|p| (100, p))) {
                    Some((o, p)) => { fuzzy += 1; json!({ "code": code, "ebills_name": name, "ebills_price": price, "product_id": p.0, "dowiz_name": p.1, "dowiz_price": p.2, "confidence": "fuzzy", "overlap_pct": o, "price_agrees": p.2 == *price, "sold_in_window": sold }) }
                    None => json!({ "code": code, "ebills_name": name, "ebills_price": price, "product_id": null, "confidence": "none", "sold_in_window": sold }),
                }
            }
        };
        rows.push(row);
    }
    println!("mapping: {} till items -> {exact} exact, {fuzzy} fuzzy, {} none", items.len(), items.len() - exact - fuzzy);
    std::fs::write(out_path, serde_json::to_string_pretty(&json!({ "venue": "dubin-sushi", "apply_exact": apply, "rows": rows })).unwrap()).unwrap();
}
