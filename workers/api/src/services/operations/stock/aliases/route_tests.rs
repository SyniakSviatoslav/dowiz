//! The invoice flow through the real routes (W-OCR P10/P11): a supplier card, the
//! confirmed lines written by the existing receipt door one by one, the aliases
//! remembered beside them, and the Stock screen reading them back -- while the
//! other venue's owner is refused.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn door(site: &Site, token: &str, kind: &str, body: Value) -> crate::wire::Reply {
    site.run(crate::services::operations::stock::stock_move, post(&at(&format!("/api/owner/stock/{kind}")), &body).bearer(token).on("alpha"), &[("kind", kind)])
}

#[test]
fn a_confirmed_invoice_is_received_and_its_aliases_come_back_on_the_stock_screen() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let b = site.venue("beta", "b@x.test");
    let r = site.run(
        crate::services::operations::supplies::set_supply,
        post(&at("/api/owner/supplies"), &json!({"id": "salmon", "name": "Salmon", "unit": "g"})).bearer(&a).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(door(&site, &a, "supplier", json!({"card": {"name": "Peshku i Detit"}})).status_code(), 200);
    // The line as the photo sheet sends it: the invoice's total, the supplier, the paper.
    let r = door(&site, &a, "received", json!({"item": "salmon", "qty": 2500, "total": 4500, "supplier": "Peshku i Detit", "doc": "F-118"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let r = door(&site, &a, "alias", json!({"card": {"supplier": "peshku-i-detit", "nipt": "K12345678A", "lines": [{"text": "Salmon file", "item": "salmon"}]}}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["written"], 2);
    assert_eq!(door(&site, &a, "alias", json!({"card": {"supplier": "peshku-i-detit", "lines": [{"text": "x", "item": "salmon"}], "rating": 5}})).status_code(), 400, "strict body");
    let stock = site.run(crate::services::operations::stock::stock, get(&at("/api/owner/stock")).bearer(&a).on("alpha"), &[]).body_value();
    assert_eq!(stock["supplierAliases"]["peshku-i-detit"]["lines"]["salmon file"], "salmon", "{stock}");
    assert_eq!(stock["supplierAliases"]["peshku-i-detit"]["nipt"], "K12345678A");
    assert!(stock["supplies"].to_string().contains("2500"), "received: {}", stock["supplies"]);
    let foreign = site.run(
        crate::services::operations::stock::stock_move,
        post(&at("/api/owner/stock/alias?location_id=alpha"), &json!({"card": {"supplier": "peshku-i-detit", "lines": []}})).bearer(&b).on("alpha"),
        &[("kind", "alias")],
    );
    assert!(foreign.status_code() >= 400, "beta wrote alpha's aliases: {}", foreign.body_str());
}
