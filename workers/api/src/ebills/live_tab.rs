//! THE TILL TAB FROM THE REAL TILL, NATIVELY (W-EBILLS-TAB proof, 2026-11-03).
//! `wrangler dev --local` cannot run on the box (workerd's tcmalloc wants a
//! 48-bit address space; the box's kernel gives 39 bits), so the proof runs
//! the REAL pipeline under `cargo test` instead: the till's answers from
//! files a read-only fetch left on disk (`live.rs`'s layout), the real
//! parsers, the real mapper, the real import with the real NAME MATCH
//! against the venue's real menu, the real month fold (`sales::month`), the
//! real analytics answer (`handler::answer_with`) -- and writes what the
//! Worker's routes would answer, for a stand-in HTTP layer to serve to the
//! real console in a real browser. `#[ignore]`d; prints COUNTS only.
//!
//!   EBILLS_LIVE_DIR=<dir> EBILLS_MENU=<public menu json> EBILLS_OUT_DIR=<dir>
//!   [EBILLS_RECIPE=<product id>] cargo test --lib ebills::live_tab -- --ignored --nocapture

use super::import::{decide, Lookups, Mapped, Waiting};
use super::map::{to_order, to_paid};
use super::{classify, names, parse_detail, sales, Kind};
use crate::hubdo::OrderView;
use crate::services::orders::status;
use dowiz_hub::tz::{start_of_local_day_ms, DEFAULT};
use serde_json::{json, Value};

const LOC: &str = "qa-durres";

fn body(dir: &str, name: &str) -> Option<(u16, String)> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(format!("{dir}/{name}")).ok()?).ok()?;
    Some((v["status"].as_u64()? as u16, v["body"].as_str()?.to_string()))
}

#[test]
#[ignore]
fn the_month_tab_and_the_analytics_from_the_real_tills_answers() {
    let (Ok(dir), Ok(menu), Ok(out)) = (std::env::var("EBILLS_LIVE_DIR"), std::env::var("EBILLS_MENU"), std::env::var("EBILLS_OUT_DIR")) else { return };
    let recipe = std::env::var("EBILLS_RECIPE").ok();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;

    // THE REAL MENU as the venue's catalogue: names and prices, one optional tech card.
    let m: Value = serde_json::from_str(&std::fs::read_to_string(menu).unwrap()).unwrap();
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    let mut products: Vec<(String, String)> = Vec::new();
    for c in m["categories"].as_array().into_iter().flatten() {
        let cid = c["id"].as_str().unwrap_or("c").to_string();
        // The keys the editor writes (`catalog_edit.rs`): `sortOrder`, `categoryId`.
        cat.set_category(&cid, &json!({ "id": cid, "name": c["name"], "sortOrder": c["sortOrder"] }).to_string());
        for p in c["products"].as_array().into_iter().flatten() {
            let id = p["id"].as_str().unwrap_or("").to_string();
            let mut pj = json!({ "id": id, "name": p["name"], "price": p["price"], "available": true, "categoryId": cid });
            if recipe.as_deref() == Some(id.as_str()) {
                pj["bom"] = json!([{ "supply": "qa-salmon", "qty": 40 }]);
            }
            cat.set_product(&id, &pj.to_string());
            products.push((id, pj.to_string()));
        }
    }
    let idx = names::index(&products, &[]);
    let by_name = |n: &str| names::resolve(&idx, n);
    let product = |p: &str| products.iter().find(|(id, _)| id == p).map(|(_, j)| j.clone());
    let none = |_: &str| None;
    let look = Lookups { map: &none, by_name: &by_name, product: &product, supply: &none };

    // EVERY SALE THE READ-ONLY FETCH LEFT: the real mapper decides what each is.
    let mut files: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()?.file_name().into_string().ok()).filter(|n| n.starts_with("sale-")).collect();
    files.sort_by_key(|f| f.trim_start_matches("sale-").trim_end_matches(".json").parse::<i64>().unwrap_or(0));
    let (mut mapped, mut gone, mut ours, mut refused) = (Vec::<Mapped>::new(), 0, 0, Vec::<String>::new());
    let (mut bills, mut courses, mut counter) = (0, 0, 0);
    for f in &files {
        let (status, b) = body(&dir, f).unwrap();
        if status == 404 { gone += 1; continue; }
        let sale = parse_detail(&b).unwrap_or_else(|e| panic!("{f}: the real detail does not parse: {e}"));
        if super::ours(&sale) { ours += 1; continue; }
        let m = match classify(&sale) {
            Ok(Kind::Bill) => { bills += 1; to_paid(&sale).map(|paid| Mapped::Bill { sale_id: sale.id, paid }) }
            Ok(Kind::Course) => { courses += 1; to_order(&sale, LOC).map(|envelope| Mapped::Order { sale_id: sale.id, envelope }) }
            Ok(Kind::CounterSale) => { counter += 1; to_order(&sale, LOC).map(|envelope| Mapped::Order { sale_id: sale.id, envelope }) }
            Err(e) => Err(e),
        };
        match m { Ok(m) => mapped.push(m), Err(e) => refused.push(format!("{}: {e:?}", sale.id)) }
    }
    println!("till files: {} ids, {gone} gone (404), {ours} ours (dowiz-sent), {bills} bills, {courses} courses, {counter} counter sales, {} refused by the mapper", files.len(), refused.len());
    for r in refused.iter().take(12) { println!("  refused {r}"); }

    // THE REAL IMPORT, in one turn, into a fresh venue.
    let mut hub = dowiz_hub::Hub::create_sized(4 * 1024 * 1024).unwrap();
    let mut stock = dowiz_hub::stock::StockLog::create_sized(1024 * 1024).unwrap();
    let o = decide(&mut hub, &mut stock, &[], &look, Waiting::default(), &mapped, now).expect("import");
    println!("import: {} placed, {} paid (courses), {} bills still waiting, {} refused, {} till lines seen", o.placed, o.paid, o.pending.len(), o.refused.len(), o.seen.len());
    let listed: Vec<OrderView> = crate::hubstore::orders_state(&hub).into_iter().map(OrderView::of_event).collect();
    let orders: Vec<Value> = listed.iter().filter_map(|v| serde_json::from_str(&v.order_json).ok()).collect();
    let (mut by_map, mut by_name_n, mut open) = (0, 0, 0);
    for it in orders.iter().flat_map(|o| o["items"].as_array().cloned().unwrap_or_default()) {
        match (it["product_id"].as_str().map(|p| p.starts_with("ebills:")), it["matched_by"].as_str()) {
            (Some(false), Some("name")) => by_name_n += 1,
            (Some(false), _) => by_map += 1,
            _ => open += 1,
        }
    }
    println!("lines: {by_name_n} matched by name, {by_map} by mapping, {open} with no dish");
    if let Some(r) = &recipe {
        let led = stock.ledger().expect("ledger");
        println!("tech card on {r}: qa-salmon level {:?} (40 g per portion served), short {:?}", led.level("qa-salmon"), dowiz_hub::stock::short(&led));
    }

    // WHAT THE ROUTES WOULD ANSWER.
    std::fs::create_dir_all(&out).unwrap();
    let sides = crate::services::analytics::side::Sides::of(&cat);
    let tab = sales::month(&listed, DEFAULT, now, &sides);
    println!("tab: {} orders in 30 days, revenue {}, average check {}, unmatched share {} ppm, {} unmatched names", tab["count"], tab["revenue"], tab["average_check"], tab["unmatched"]["share_ppm"], tab["unmatched"]["names"].as_array().map_or(0, Vec::len));
    // W-DRINKS-KITCHEN: the split the owner asked for, as the tab answers it.
    let sp = &tab["sides"];
    for k in ["kitchen", "drinks", "unclassified"] {
        println!("sides.{k}: sales {} ({} pm), quantity {}, top {}", sp[k]["sales"], sp[k]["sharePm"], sp[k]["quantity"], sp[k]["top"].as_array().map_or(0, Vec::len));
    }
    for k in ["kitchen", "drinksOnly", "unclassified"] {
        println!("checks.{k}: {} orders, revenue {}, average check {}", sp["orders"][k]["orders"], sp["orders"][k]["revenue"], sp["orders"][k]["averageCheck"]);
    }
    std::fs::write(format!("{out}/tab.json"), tab.to_string()).unwrap();
    // The categories sheet's answer (`catalog_edit::categories_view`), for the stand-in.
    std::fs::write(format!("{out}/categories.json"), crate::catalog_edit::categories_view(&cat).to_string()).unwrap();
    let cold = |_: i64, _: i64| Ok(std::collections::BTreeMap::new());
    let v2 = crate::services::analytics::handler::answer_with(listed.clone(), &cat, LOC, now, (Some("30"), None, None), &cold).expect("analytics");
    println!("analytics(30d): orders {}, revenue {}, channels {}", v2["orders"], v2["revenue"], v2["channels"]);
    std::fs::write(format!("{out}/analytics-v2.json"), v2.to_string()).unwrap();
    std::fs::write(format!("{out}/analytics-v1.json"), crate::services::analytics::handler::v1_of(v2).to_string()).unwrap();
    // Today's tiles, by the dashboard's rule (`owner/dashboard.rs::tiles`): orders since local midnight, `venue_took`.
    let day_start = start_of_local_day_ms(DEFAULT, now);
    let today: Vec<&Value> = orders.iter().filter(|o| o["created_at_ms"].as_i64().unwrap_or(0) >= day_start).collect();
    let revenue: i64 = today.iter().map(|o| status::venue_took(o["total"].as_i64().unwrap_or(0), o["tip"].as_i64().unwrap_or(0), o["status"].as_str().unwrap_or(""))).sum();
    std::fs::write(format!("{out}/dashboard.json"), json!({ "count": today.len(), "revenue": revenue, "pending": 0, "active": 0 }).to_string()).unwrap();
    std::fs::write(format!("{out}/orders.json"), json!({ "orders": orders, "generation": 1, "full": true }).to_string()).unwrap();
    println!("today: {} orders, {revenue} lek; wrote tab.json, analytics-v1/v2.json, dashboard.json, orders.json to {out}", today.len());
}
