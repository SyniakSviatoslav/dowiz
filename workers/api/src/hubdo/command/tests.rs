//! `POST /fold/command` (BN4): the object runs the whole request in its own turn, and only
//! for the venue it is.

use super::{Command, Kind};
use crate::edge::mem::block_on;
use crate::edge::site::Site;
use crate::wire::{Call, Reply};
use serde_json::json;
use worker::Method;

fn venue_id(site: &Site, slug: &str) -> String {
    block_on(crate::hubstore::Place::of_slug(&site.ctx(&[]), slug)).expect("registry").venue
}

fn command(venue: &str, dish: &str) -> Command {
    let body = json!({
        "items": [{"product_id": dish, "quantity": 1}],
        "contact": {"name": "Guest", "phone": "+355690000001"},
        "fulfilment": {"kind": "pickup"}, "payment": "cash",
    });
    Command {
        kind: Kind::Place,
        venue: venue.to_string(),
        params: vec![("slug".into(), "alpha".into())],
        method: "POST".into(),
        now_ms: crate::edge::site::T0,
        url: "https://alpha.dowiz.org/api/public/locations/alpha/orders".into(),
        headers: vec![("host".into(), "alpha.dowiz.org".into())],
        body: body.to_string(),
    }
}

/// Send `body` to the object named `to`, through the stub a Worker would hold.
fn to_object(site: &Site, to: &str, body: Vec<u8>) -> worker::Result<Reply> {
    let stub = site.env().durable_object("HUB")?.id_from_name(to)?.get_stub()?;
    block_on(stub.fetch_with_request(Call::new("https://hub/fold/command", Method::Post)?.with_body(body)))
}

fn generation(site: &Site, venue: &str) -> serde_json::Value {
    site.fold(venue, "/fold/generation").body_value()
}

#[test]
fn a_command_runs_the_placement_in_the_venues_own_turn() {
    let site = Site::new();
    let (_t, dish) = crate::storefront::route_tests::open_venue(&site, "alpha", "alpha@example.com");
    let alpha = venue_id(&site, "alpha");
    let r = to_object(&site, &alpha, serde_json::to_vec(&command(&alpha, &dish)).unwrap()).expect("answered");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["subtotal"], json!(900), "priced by the catalogue: {}", r.body_str());
}

#[test]
fn a_command_naming_another_venue_is_refused_and_writes_nothing() {
    let site = Site::new();
    let (_t, dish) = crate::storefront::route_tests::open_venue(&site, "alpha", "alpha@example.com");
    crate::storefront::route_tests::open_venue(&site, "beta", "beta@example.com");
    let (alpha, beta) = (venue_id(&site, "alpha"), venue_id(&site, "beta"));
    let before = (generation(&site, &alpha), generation(&site, &beta));
    // Sent to beta's object, naming alpha: beta must not run alpha's placement.
    let r = to_object(&site, &beta, serde_json::to_vec(&command(&alpha, &dish)).unwrap()).expect("answered");
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    assert_eq!((generation(&site, &alpha), generation(&site, &beta)), before, "no order anywhere");
}

#[test]
fn a_command_with_a_field_it_does_not_name_is_refused_and_writes_nothing() {
    let site = Site::new();
    let (_t, dish) = crate::storefront::route_tests::open_venue(&site, "alpha", "alpha@example.com");
    let alpha = venue_id(&site, "alpha");
    let before = generation(&site, &alpha);
    let mut v = serde_json::to_value(command(&alpha, &dish)).unwrap();
    v["as_venue"] = json!("beta");
    assert!(to_object(&site, &alpha, serde_json::to_vec(&v).unwrap()).is_err(), "an unknown field is an error");
    assert_eq!(generation(&site, &alpha), before);
}
