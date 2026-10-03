//! The cube through the routes (W-HIST P2a, P2b): orders placed on a venue in
//! memory, a rotation as the night runs it, the catch-up, and the owner's and
//! the kitchen's numbers read back -- the days the hot log no longer holds
//! included, and every archived day checked against its archive.

use crate::edge::mem::block_on;
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use serde_json::{json, Value};

const DAY: i64 = 86_400_000;

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}
/// One owner per venue.
fn email(slug: &str) -> String {
    format!("{slug}@x.test")
}

/// Place a pickup of `qty` on `slug` at instant `when` and see it through to
/// PICKED_UP, as the owner would. Returns the order id.
fn sold(site: &mut Site, slug: &str, dish: &str, qty: i64, when: i64) -> String {
    site.now_ms = when;
    let t = site.login(slug, &email(slug));
    let r = place_pickup(site, slug, dish, qty);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let id = v["id"].as_str().or(v["order"]["id"].as_str()).expect("order id").to_string();
    for action in ["confirm", "preparing", "ready", "collected"] {
        let r = site.run(
            crate::owner::order_action,
            post(&at(slug, &format!("/api/owner/orders/{id}/action")), &json!({"location_id": slug, "action": action})).bearer(&t).on(slug),
            &[("id", &id)],
        );
        assert_eq!(r.status_code(), 200, "{action}: {}", r.body_str());
    }
    id
}

fn read(site: &Site, slug: &str, handler: &str, query: &str) -> Value {
    let t = site.login(slug, &email(slug));
    let call = get(&at(slug, &format!("/api/owner/analytics{handler}?v=2&{query}"))).bearer(&t).on(slug);
    let r = if handler.is_empty() {
        site.run(crate::services::analytics::handler::analytics, call, &[])
    } else {
        site.run(crate::services::analytics::kitchen::kitchen, call, &[])
    };
    assert_eq!(r.status_code(), 200, "{handler}?{query}: {}", r.body_str());
    r.body_value()
}

fn catch_up(site: &Site, slug: &str, body: Value) -> crate::wire::Reply {
    let t = site.login(slug, &email(slug));
    site.run(crate::services::analytics::handler::history, post(&at(slug, "/api/owner/analytics/history"), &body).bearer(&t).on(slug), &[])
}

/// A venue with a sale 40 days before `now` and one 2 days before, rotated as
/// the night would at `now`: the old one is in an archive, the new one hot.
fn world() -> (Site, i64) {
    let mut site = Site::new();
    let (_, dish) = open_venue(&site, "alpha", &email("alpha"));
    let now = site.now_ms + 100 * DAY;
    sold(&mut site, "alpha", &dish, 2, now - 40 * DAY);
    sold(&mut site, "alpha", &dish, 1, now - 2 * DAY);
    site.now_ms = now;
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
    let v = block_on(crate::hubstore::rotate(&place, now)).unwrap();
    assert_eq!((v["rotated"].as_bool(), v["ordersMoved"].as_u64()), (Some(true), Some(1)), "{v}");
    (site, now)
}

/// P2a, THE 30/62-DAY DEFECT. The hot log keeps 30 days and the kitchen's
/// window allows 62: before the cube, the sale 40 days ago read as no sale.
/// The sales side of the window is the cube's for the archived days now.
#[test]
fn a_62_day_kitchen_window_shows_the_sales_of_day_40_after_rotation() {
    let (site, _) = world();
    // The catch-up is idempotent, so this holds whether or not the rotation's
    // own hook (the hubstore.rs hand-back) already folded the archive.
    let r = catch_up(&site, "alpha", json!({}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!((r.body_value()["folded"].as_u64(), r.body_value()["days"].as_u64()), (Some(1), Some(1)), "{}", r.body_str());

    let k = read(&site, "alpha", "/kitchen", "days=62");
    assert_eq!((k["totals"]["revenue"].as_i64(), k["totals"]["orders"].as_i64()), (Some(2700), Some(2)), "{}", k["totals"]);
    let sold_days: Vec<(&str, i64)> = k["byDay"].as_array().unwrap().iter().filter(|d| d["revenue"].as_i64() > Some(0)).map(|d| (d["day"].as_str().unwrap(), d["revenue"].as_i64().unwrap())).collect();
    assert_eq!(sold_days.len(), 2, "{sold_days:?}");
    assert_eq!(sold_days[0].1, 1800, "day 40's two portions: {sold_days:?}");
    assert_eq!(k["history"]["archivedDays"], 1);
    let dish = &k["dishes"][0];
    assert_eq!((dish["sold"].as_i64(), dish["byDay"].as_array().unwrap().iter().filter(|q| q.as_i64() > Some(0)).count()), (Some(3), 2));
}

/// The owner's numbers are the same before the rotation and after it: the
/// archived day moved from the hot log to the cube and nothing else changed.
#[test]
fn the_owners_numbers_are_the_same_before_and_after_a_rotation() {
    let mut site = Site::new();
    let (_, dish) = open_venue(&site, "alpha", &email("alpha"));
    let now = site.now_ms + 100 * DAY;
    sold(&mut site, "alpha", &dish, 2, now - 40 * DAY);
    sold(&mut site, "alpha", &dish, 1, now - 2 * DAY);
    site.now_ms = now;
    let before = read(&site, "alpha", "", "days=90");
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
    block_on(crate::hubstore::rotate(&place, now)).unwrap();
    assert_eq!(catch_up(&site, "alpha", json!({})).status_code(), 200);
    let after = read(&site, "alpha", "", "days=90");
    assert_eq!(after["contract"], "analytics.owner.v2");
    for k in ["orders", "revenue", "rejected", "averageOrder", "pickup", "byHour", "byChannel", "topProducts"] {
        assert_eq!(before[k], after[k], "{k}");
    }
    let days = |v: &Value| v["byDay"].as_array().unwrap().iter().map(|d| (d["day"].clone(), d["orders"].clone(), d["revenue"].clone())).collect::<Vec<_>>();
    assert_eq!(days(&before), days(&after));
    let archived: Vec<&Value> = after["byDay"].as_array().unwrap().iter().filter(|d| d["archived"].as_i64() > Some(0)).collect();
    assert_eq!(archived.len(), 1, "the day 40 back reads from the cube");
    assert_eq!((before["history"]["archivedDays"].as_i64(), after["history"]["archivedDays"].as_i64()), (Some(0), Some(1)));
    assert_eq!(after["history"]["pending"], json!([]));
}

/// THE VERIFIER: the newest archived day is refolded from its archive and
/// matches byte for byte; its records add up to its number and carry no
/// person. One lek written into the stored row is caught.
#[test]
fn an_archived_day_is_verified_against_its_archive_and_one_lek_is_caught() {
    let (site, _) = world();
    assert_eq!(catch_up(&site, "alpha", json!({})).status_code(), 200);
    let v = read(&site, "alpha", "", "verify=1");
    assert_eq!(v["archived"]["equal"], true, "{v}");
    let recs = v["records"].as_array().unwrap();
    assert_eq!(recs.len(), 1);
    assert_eq!((recs[0]["took"].as_i64(), recs[0]["from"].as_str().map(|s| s.starts_with("log@"))), (Some(1800), Some(true)));
    assert!(!v.to_string().contains("+355"), "no phone in a trace: {v}");

    // Tamper: one lek more in the stored row.
    let obj = site.object("alpha");
    let (gen, bytes) = block_on(obj.cube_image()).unwrap();
    let mut cube = crate::services::analytics::cube::Cube::decode(&bytes).unwrap();
    cube.rows.values_mut().next().unwrap().t += 1;
    assert!(block_on(obj.put_image(crate::services::analytics::cube::IMAGE, gen, &cube.encode())).unwrap().is_some());
    let bad = read(&site, "alpha", "", "verify=1");
    assert_eq!(bad["archived"]["equal"], false, "{bad}");
}

/// A CATCH-UP IS IDEMPOTENT: a second one adds nothing and the numbers stand.
/// Twin: `rebuild` refolds every archive to the same rows.
#[test]
fn a_second_catch_up_adds_nothing_and_a_rebuild_gives_the_same_cube() {
    let (site, _) = world();
    assert_eq!(catch_up(&site, "alpha", json!({})).status_code(), 200);
    let first = block_on(site.object("alpha").cube_image()).unwrap().1;
    let again = catch_up(&site, "alpha", json!({}));
    assert_eq!(again.body_value()["added"], json!([]), "{}", again.body_str());
    assert_eq!(block_on(site.object("alpha").cube_image()).unwrap().1, first, "unchanged");
    let rebuilt = catch_up(&site, "alpha", json!({"rebuild": true}));
    assert_eq!(rebuilt.status_code(), 200, "{}", rebuilt.body_str());
    assert_eq!(block_on(site.object("alpha").cube_image()).unwrap().1, first, "the same rows from the same archives");
}

/// ROTATING AS OF ANOTHER INSTANT is the QA hub's alone. Twin: on the QA hub
/// it rotates and folds in one call.
#[test]
fn rotate_as_of_is_refused_off_the_qa_hub_and_works_on_it() {
    let mut site = Site::new();
    let (_, dish) = open_venue(&site, "alpha", &email("alpha"));
    let r = catch_up(&site, "alpha", json!({"rotateAsOf": site.now_ms + 31 * DAY}));
    assert_eq!(r.status_code(), 403, "{}", r.body_str());
    let _ = dish;
    let qa = crate::services::analytics::handler::QA_VENUE;
    let (_, qdish) = open_venue(&site, qa, &email(qa));
    let t0 = site.now_ms;
    sold(&mut site, qa, &qdish, 1, t0);
    site.now_ms = t0 + DAY;
    let r = catch_up(&site, qa, json!({"rotateAsOf": t0 + 31 * DAY}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    // `added` is empty when the rotation's hook folded the archive first; the
    // cube's totals say the same either way.
    assert_eq!((v["rotated"]["ordersMoved"].as_u64(), v["folded"].as_u64(), v["days"].as_u64()), (Some(1), Some(1), Some(1)), "{v}");
}

/// A body field the route does not know is refused (strict body).
#[test]
fn an_unknown_field_is_refused() {
    let site = Site::new();
    open_venue(&site, "alpha", &email("alpha"));
    assert_eq!(catch_up(&site, "alpha", json!({"rotate": 1})).status_code(), 400);
}

/// THE PARSE IS KEPT PER IMAGE: the same bytes answer the same parse, other
/// bytes (one lek apart) are parsed again -- never another image's rows.
#[test]
fn the_parse_is_kept_for_the_same_bytes_and_never_served_for_other_bytes() {
    use crate::services::analytics::cube::{Cube, DayCube};
    let mut c = Cube::default();
    c.absorb("log@1", std::collections::BTreeMap::from([(20260901, DayCube { d: 20260901, t: 100, ..DayCube::default() })]));
    let a = c.encode();
    let first = super::parsed(&a);
    assert!(std::rc::Rc::ptr_eq(&first, &super::parsed(&a)), "a second read of the same image is not parsed again");
    c.rows.get_mut(&20260901).unwrap().t += 1;
    let b = c.encode();
    let other = super::parsed(&b);
    assert!(!std::rc::Rc::ptr_eq(&first, &other));
    assert_eq!(other.as_ref().as_ref().unwrap().rows[&20260901].t, 101);
    assert!(super::parsed(b"garbage").is_err(), "an unreadable image is its reason");
    assert_eq!(super::parsed(&[]).as_ref().as_ref().unwrap(), &Cube::default(), "no image, no history");
}

/// THE ROTATION'S OWN HOOK (hand-back line in `hubstore::rotate`): the day that
/// leaves the hot log is in the cube the moment the rotation returns, with no
/// catch-up call -- the 62-day window shows day 40 right away.
#[test]
fn the_rotation_itself_folds_the_days_it_moves() {
    let (site, _) = world();
    let k = read(&site, "alpha", "/kitchen", "days=62");
    assert_eq!((k["totals"]["revenue"].as_i64(), k["totals"]["orders"].as_i64()), (Some(2700), Some(2)), "{}", k["totals"]);
    assert_eq!(k["history"]["archivedDays"], 1);
}
