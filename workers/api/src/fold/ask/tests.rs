//! THE RELAY (W-OWNER-CPU): what the Worker hands on is what the object answered -- status,
//! content type and body, byte for byte -- and a refusal keeps the object's status (`relay`) or
//! becomes the caller's `Err` (`relay_ok`), as `json` made it. In memory there is no hop, so the
//! relayed reply IS the object's reply; on the platform its body is held outside wasm
//! (`wire/held.rs`), which these tests cannot reach and `is_relayed` names.

use super::{relay, relay_ok};
use crate::edge::mem::block_on;
use crate::edge::site::Site;
use crate::hubstore::Place;
use crate::storefront::route_tests::{open_venue, place_pickup};

#[test]
fn a_relay_hands_on_the_objects_status_content_type_and_bytes_unchanged() {
    let site = Site::new();
    let (_t, dish) = open_venue(&site, "alpha", "a@x.test");
    place_pickup(&site, "alpha", &dish, 2);
    let place = Place::of_authorised(&site.ctx(&[]), "alpha").expect("place");
    for path in ["/fold/catalogue?q=categories", "/fold/catalogue?q=owner_products", "/fold/venue", "/fold/orders"] {
        let direct = site.fold("alpha", path);
        assert_eq!(direct.status_code(), 200, "{path}: {}", direct.body_str());
        assert!(!direct.body().is_empty(), "{path}: the oracle answers something");
        let relayed = block_on(relay(&place, &format!("https://hub{path}"))).expect("relay");
        assert_eq!(relayed.status_code(), direct.status_code(), "{path}");
        assert_eq!(relayed.headers().get("content-type").ok().flatten(), direct.headers().get("content-type").ok().flatten(), "{path}");
        assert_eq!(relayed.headers().get("x-generation").ok().flatten(), direct.headers().get("x-generation").ok().flatten(), "{path}: the generation travels");
        assert_eq!(relayed.body(), direct.body(), "{path}: byte for byte");
        assert!(!relayed.is_relayed(), "{path}: in memory there is no hop, so the bytes are here to compare");
        let ok = block_on(relay_ok(&place, &format!("https://hub{path}"))).expect("relay_ok");
        assert_eq!(ok.body(), direct.body(), "{path}: relay_ok is the same answer on a 200");
    }
}

#[test]
fn a_refusal_keeps_its_status_through_relay_and_is_an_err_through_relay_ok() {
    let site = Site::new();
    let (_t, _dish) = open_venue(&site, "alpha", "a@x.test");
    let place = Place::of_authorised(&site.ctx(&[]), "alpha").expect("place");
    let url = "https://hub/fold/catalogue?q=no-such-node";
    let direct = site.fold("alpha", "/fold/catalogue?q=no-such-node");
    assert_eq!(direct.status_code(), 404, "the oracle refuses");
    let r = block_on(relay(&place, url)).expect("relay carries a refusal");
    assert_eq!(r.status_code(), 404);
    assert_eq!(r.body(), direct.body(), "the object's own words");
    let e = block_on(relay_ok(&place, url)).err().expect("relay_ok refuses");
    assert!(e.to_string().contains("404") && e.to_string().contains(url), "{e}");
}
