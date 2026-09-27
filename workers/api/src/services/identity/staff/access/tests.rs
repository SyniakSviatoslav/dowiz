use super::*;
use crate::auth::{self, Principal};
use dowiz_hub::caps::Preset;
use serde_json::json;

const LIB: &str = include_str!("../../../../lib.rs");

fn staff(p: Preset, at: &str) -> Principal {
    Principal::Staff { person_id: "p1".into(), active_location_id: at.into(), session_id: "s1".into(), caps: p.caps() }
}

/// Every `(method, path)` `lib.rs` registers under `/api/owner` or `/api/staff`.
fn lib_routes() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for m in ["get", "post", "put", "delete"] {
        let open = format!(".{m}_async(\"");
        for (i, _) in LIB.match_indices(&open) {
            let rest = &LIB[i + open.len()..];
            let path = &rest[..rest.find('"').unwrap()];
            if path.starts_with("/api/owner/") || path.starts_with("/api/staff/") {
                out.push((m.to_string(), path.to_string()));
            }
        }
    }
    out
}

/// A NEW ROUTE ARRIVES DECIDED. Each owner/staff route has one row, and each
/// row is a route that exists.
#[test]
fn every_owner_and_staff_route_has_exactly_one_row() {
    let lib = lib_routes();
    assert!(lib.len() > 100, "the parse of lib.rs found only {} routes", lib.len());
    for (m, p) in &lib {
        let n = ROUTES.iter().filter(|r| r.0 == m && r.1 == p).count();
        assert_eq!(n, 1, "{m} {p}: {n} rows in access::ROUTES (add its kitchen decision)");
    }
    for r in ROUTES {
        let routed = lib.iter().any(|(m, p)| m == r.0 && p == r.1);
        let pending = HANDED_BACK.iter().any(|(m, p)| *m == r.0 && *p == r.1);
        assert!(routed || pending, "{} {} is not a route any more", r.0, r.1);
    }
}

/// THE TABLE, ASKED OF THE REAL DOOR. A kitchen token passes every row the
/// table opens to it and is refused (403) every staff row it does not; an
/// owner passes every staff row; another venue's kitchen is 404 everywhere.
#[test]
fn each_row_does_to_a_kitchen_token_what_it_says() {
    let owner = Principal::Owner { user_id: "o".into(), active_location_id: Some("v".into()) };
    let mut opened = 0;
    for (m, p, door, k) in ROUTES {
        match (door, k) {
            (Door::Staff(f), Yes | Read) => {
                opened += 1;
                assert!(auth::room_admits_any(&staff(Preset::Kitchen, "v"), "v", f).is_ok(), "kitchen refused {m} {p}");
                assert_eq!(auth::room_admits_any(&staff(Preset::Kitchen, "w"), "v", f).map(|_| ()).map_err(|e| e.0), Err(404), "{m} {p} crossed venues");
            }
            (Door::Staff(f), No) => {
                assert_eq!(auth::room_admits_any(&staff(Preset::Kitchen, "v"), "v", f).map(|_| ()).map_err(|e| e.0), Err(403), "kitchen got {m} {p}");
            }
            (Door::Owner, k) => assert_eq!(*k, No, "{m} {p}: the owner's door cannot open to the kitchen"),
            (Door::Own, k) => assert_eq!(*k, Yes, "{m} {p}: a person's own door is theirs"),
        }
        if let Door::Staff(f) = door {
            assert!(auth::room_admits_any(&owner, "v", f).is_ok(), "owner refused {m} {p}");
        }
    }
    assert!(opened >= 25, "only {opened} routes open to the kitchen");
}

/// The twin: a waiter holds none of the kitchen's words, so every row the
/// kitchen gets that the waiter's two words do not name refuses the waiter.
#[test]
fn a_waiter_is_refused_the_kitchens_rows() {
    for (m, p, door, k) in ROUTES {
        let Door::Staff(f) = door else { continue };
        if matches!(k, No) {
            continue;
        }
        assert_eq!(auth::room_admits_any(&staff(Preset::Waiter, "v"), "v", f).map(|_| ()).map_err(|e| e.0), Err(403), "waiter got {m} {p}");
    }
}

/// The routes this lane opened ask the family the table names, and narrow
/// their answer for staff. Read from the handlers' own source.
#[test]
fn the_opened_handlers_ask_their_family_and_narrow() {
    let print = include_str!("../../../orders/print.rs");
    let floor = include_str!("../../../../booking/floor.rs");
    let day = include_str!("../../../../booking/venue.rs");
    let settings = include_str!("../../../venue/settings.rs");
    let numbers = include_str!("../../../analytics/kitchen.rs");
    for (src, needles, what) in [
        (print, &["guard::PASS"][..], "print jobs"),
        (floor, &["guard::PASS"][..], "floor plan"),
        (day, &["guard::PASS", "bookings_for_kitchen"][..], "bookings"),
        (settings, &["guard::PASS", "settings_for_kitchen", "kitchen_may_set"][..], "settings"),
        (numbers, &["guard::NUMBERS", "numbers_for_kitchen"][..], "kitchen numbers"),
    ] {
        for n in needles {
            assert!(src.contains(n), "{what}: its handler does not name {n}");
        }
    }
}

/// The report really carries the keys stripped here, so the strip is not a
/// no-op on a renamed field.
#[test]
fn the_report_names_every_revenue_key() {
    let report = include_str!("../../../analytics/kitchen/report.rs");
    for k in REVENUE_KEYS {
        assert!(report.contains(&format!("\"{k}\"")), "report.rs no longer writes {k}");
    }
}

fn has_key(v: &Value, k: &str) -> bool {
    match v {
        Value::Object(m) => m.contains_key(k) || m.values().any(|x| has_key(x, k)),
        Value::Array(a) => a.iter().any(|x| has_key(x, k)),
        _ => false,
    }
}

fn report() -> Value {
    json!({
        "totals": { "orders": 4, "revenue": 9000, "cogs": 3000, "foodCostPm": 333, "margin": 6000, "wasteValue": 120 },
        "byDay": [{ "day": "2026-09-27", "orders": 4, "revenue": 9000, "cogs": 3000, "foodCostPm": 333, "waste": 120 }],
        "dishes": [{ "id": "d", "name": "Maki", "sold": 4, "revenue": 9000, "portionCost": 750, "cogs": 3000,
                     "margin": 6000, "marginPortion": 1500, "foodCostPm": 333, "hasRecipe": true }],
        "ingredients": [{ "id": "rice", "used": 400, "cost": 300 }],
    })
}

#[test]
fn kitchen_numbers_lose_revenue_and_keep_cost() {
    let out = numbers_for_kitchen(report());
    for k in REVENUE_KEYS {
        assert!(!has_key(&out, k), "{k} reached the kitchen");
    }
    assert_eq!(out["scope"], "kitchen");
    assert_eq!(out["totals"]["cogs"], 3000);
    assert_eq!(out["totals"]["wasteValue"], 120);
    assert_eq!(out["dishes"][0]["portionCost"], 750);
    assert_eq!(out["dishes"][0]["sold"], 4);
    assert_eq!(out["byDay"][0]["orders"], 4);
    assert_eq!(out["ingredients"][0]["used"], 400);
}

/// The twin: the owner's answer is the report itself -- every revenue key the
/// kitchen loses is there to lose.
#[test]
fn the_owners_numbers_carry_what_the_kitchen_loses() {
    let r = report();
    for k in REVENUE_KEYS {
        assert!(has_key(&r, k), "the sample report has no {k}");
    }
}

#[test]
fn a_booking_reaches_the_kitchen_without_a_person() {
    let rows = vec![json!({ "id": "b1", "slotMin": 1200, "party": 4, "name": "Ana Hoxha", "phone": "+355 69 000",
                            "occasion": "birthday", "zoneId": "z", "tableN": 3, "status": "CONFIRMED",
                            "next": ["SEATED"], "email": "a@x.al" })];
    let out = bookings_for_kitchen(rows.clone());
    let s = out[0].to_string();
    for gone in ["Ana Hoxha", "+355 69 000", "a@x.al", "SEATED"] {
        assert!(!s.contains(gone), "{gone} reached the kitchen: {s}");
    }
    assert_eq!(out[0]["party"], 4);
    assert_eq!(out[0]["slotMin"], 1200);
    assert_eq!(out[0]["occasion"], "birthday");
    assert_eq!(out[0]["tableN"], 3);
    assert_eq!(out[0]["next"], json!([]));
    // The twin: the owner's row, untouched, still has them.
    assert!(rows[0].to_string().contains("Ana Hoxha"));
}

#[test]
fn staff_settings_are_the_printer_and_nothing_else() {
    let values = json!({ "print.kitchen": "pass-1", "ai.token": "sk-secret", "notify.telegram.token": "123:abc" });
    let known = vec![json!({ "key": "print.kitchen" }), json!({ "key": "ai.token", "secret": true })];
    let out = settings_for_kitchen(&values, known);
    let s = out.to_string();
    assert!(!s.contains("sk-secret") && !s.contains("123:abc") && !s.contains("ai.token"), "{s}");
    assert_eq!(out["values"]["print.kitchen"], "pass-1");
    assert_eq!(out["known"].as_array().map(Vec::len), Some(1));
    assert!(kitchen_may_set("print.kitchen"));
    assert!(!kitchen_may_set("ai.token"));
    assert!(!kitchen_may_set("tax.default_ppm"));
    // Every kitchen setting is a declared one.
    for k in KITCHEN_SETTINGS {
        assert!(dowiz_hub::settings::KNOWN.iter().any(|x| x.key == k), "{k} is not a declared setting");
    }
}
