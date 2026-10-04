//! W-INT2 #24 / #28: one run of `cloud::nightly` over a fixture venue -- the retention prune
//! removes the old idempotency keys and error records and keeps the new; the chain check says
//! nothing about a good log and names a log with ONE edited cell (no fuzzers: one named byte).
//! (#26, the copy to the venue's bucket and the skip without one, is
//! `cron::route_tests::the_night_copies_a_configured_venue_and_its_witness_agrees_with_the_next_night`.)

use crate::edge::mem::{answer_outbound, block_on};
use crate::edge::site::Site;
use crate::hubdo::host::mem::Stored;
use crate::storefront::route_tests::{open_venue, place_pickup};
use crate::wire::Reply;
use serde_json::json;

const DAY: i64 = 86_400_000;

fn place(site: &Site) -> crate::hubstore::Place {
    crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap()
}

fn errors(site: &Site) -> Vec<serde_json::Value> {
    let ns = site.env().durable_object("HUB").unwrap();
    block_on(crate::errlog::recent(&ns, Some("alpha"), 500)).unwrap().errors
}

/// The answer the idempotency image holds for `key`, if it still holds one.
fn answered(site: &Site, key: &str, now: i64) -> bool {
    let key = key.to_string();
    block_on(crate::hubstore::with_table(&place(site), crate::idempotency::IMAGE_IDEMPOTENCY, crate::idempotency::IDEMPOTENCY_BYTES, move |t| {
        crate::idempotency::verdict::claim(t, &key, "p", now)
    }))
    .unwrap()
        != crate::idempotency::verdict::Seen::Claimed
}

#[test]
fn the_night_prunes_old_keys_and_old_errors_and_keeps_the_new() {
    let site = Site::new();
    open_venue(&site, "alpha", "a@x.test");
    let night = site.now_ms + 30 * DAY;
    let p = place(&site);
    block_on(crate::hubstore::with_table(&p, crate::idempotency::IMAGE_IDEMPOTENCY, crate::idempotency::IDEMPOTENCY_BYTES, move |t| {
        crate::idempotency::verdict::record(t, "old-key", "p", night - 2 * DAY, 200, "{}", "application/json")?;
        crate::idempotency::verdict::record(t, "new-key", "p", night - 3_600_000, 200, "{}", "application/json")
    }))
    .unwrap();
    let stub = p.stub().unwrap();
    for (what, at) in [("an old failure", night - 8 * DAY), ("a new failure", night - 3_600_000)] {
        let rec = json!({"atMs": at, "place": "test.prune", "message": what}).to_string();
        block_on(crate::platform_store::with_log_at(&stub, crate::errlog::image_of(Some("alpha")), move |log| {
            log.append(crate::errlog::KIND, "test.prune", &rec).map_err(|e| worker::Error::RustError(format!("{e:?}")))
        }))
        .unwrap();
    }
    let said = |site: &Site, what: &str| errors(site).iter().any(|e| e["message"] == what);
    assert!(said(&site, "an old failure") && said(&site, "a new failure"));

    answer_outbound(|_| Reply::from_json(&json!({})));
    site.night(night);
    assert!(!said(&site, "an old failure"), "a week-old error survived the night: {:?}", errors(&site));
    assert!(said(&site, "a new failure"), "the prune took a young error");
    assert!(answered(&site, "new-key", night), "the prune took a key inside its window");
    assert!(!answered(&site, "old-key", night), "a key past its window survived the night");
}

#[test]
fn the_night_names_a_log_with_one_edited_cell_and_is_quiet_about_a_good_one() {
    let site = Site::new();
    let (_, dish) = open_venue(&site, "alpha", "a@x.test");
    for _ in 0..2 {
        assert_eq!(place_pickup(&site, "alpha", &dish, 1).status_code(), 200);
    }
    answer_outbound(|_| Reply::from_json(&json!({})));
    let chain = |site: &Site| errors(site).into_iter().filter(|e| e["place"] == "hub.chain").collect::<Vec<_>>();
    site.night(site.now_ms + DAY);
    assert!(chain(&site).is_empty(), "a good log was reported: {:?}", chain(&site));

    // ONE CELL: the last digit of the guest's phone, inside the order log's stored bytes.
    let host = site.world.host("alpha");
    let needle = b"+355690000001";
    let edited = {
        let mut kv = host.kv.borrow_mut();
        let hit = kv.iter_mut().find_map(|(k, v)| match v {
            Stored::Bytes(b) if k.starts_with("c:log:") => b.windows(needle.len()).position(|w| w == needle).map(|at| (b, at)),
            _ => None,
        });
        let (bytes, at) = hit.expect("the order log holds the guest's phone");
        bytes[at + needle.len() - 1] = b'9';
        true
    };
    assert!(edited);
    site.world.restart("alpha");
    site.night(site.now_ms + 2 * DAY);
    let said = chain(&site);
    assert_eq!(said.len(), 1, "{said:?}");
    let msg = said[0]["message"].as_str().unwrap_or("");
    assert!(msg.starts_with("BROKEN CHAIN:"), "{msg}");
}
