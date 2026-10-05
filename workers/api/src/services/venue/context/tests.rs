//! The moment at the venue (W-SENSE row 4): the band and weekday on the venue's clock, the weather
//! at the VENUE's coordinates from Open-Meteo, and every failure answered as "no weather".

use super::*;
use crate::edge::mem::{answer_outbound, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use serde_json::json;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}
fn meteo(t: f64, p: f64, code: i64) -> Value {
    json!({"latitude": 41.31, "longitude": 19.45, "current": {"time": "2026-10-04T18:00", "temperature_2m": t, "precipitation": p, "weather_code": code}})
}

#[test]
fn the_band_of_the_day_has_five_edges() {
    for (h, b) in [(0, "night"), (4, "night"), (5, "morning"), (10, "morning"), (11, "midday"), (13, "midday"), (14, "afternoon"),
                   (16, "afternoon"), (17, "evening"), (21, "evening"), (22, "night"), (23, "night")] {
        assert_eq!(band(h * 60 + 30), b, "{h}:30");
    }
    assert!(BANDS.contains(&band(-30)), "a negative minute wraps");
}

#[test]
fn open_meteo_current_is_one_of_four_words_and_a_missing_temperature_is_none() {
    assert_eq!(weather_of(&meteo(18.0, 0.4, 61)), Some("rain"));
    assert_eq!(weather_of(&meteo(18.0, 0.0, 95)), Some("rain"), "thunder");
    assert_eq!(weather_of(&meteo(-1.0, 0.2, 73)), Some("cold"), "snow is cold, not rain");
    assert_eq!(weather_of(&meteo(7.0, 0.0, 1)), Some("cold"));
    assert_eq!(weather_of(&meteo(31.0, 0.0, 0)), Some("hot"));
    assert_eq!(weather_of(&meteo(26.0, 0.0, 0)), Some("hot"), "26 is hot");
    assert_eq!(weather_of(&meteo(25.9, 0.0, 2)), Some("mild"));
    assert_eq!(weather_of(&json!({"current": {"precipitation": 1.0}})), None);
    assert_eq!(weather_of(&json!({"error": true, "reason": "x"})), None);
}

#[test]
fn the_url_carries_the_venue_coordinates_rounded_and_nothing_else() {
    let u = weather_url(41.313_456, 19.445_9).unwrap();
    assert_eq!(u, "https://api.open-meteo.com/v1/forecast?latitude=41.31&longitude=19.45&current=temperature_2m,precipitation,weather_code");
    assert_eq!(weather_url(91.0, 0.0), None);
    assert_eq!(weather_url(0.0, 0.0), None, "the null island is a venue that never set its place");
    assert_eq!(weather_url(f64::NAN, 1.0), None);
}

#[test]
fn a_bucket_is_filed_under_its_band_its_kind_of_day_and_its_weather() {
    let b = Bucket { band: "evening", weekday: 3, weather: Some("rain") };
    assert_eq!(b.keys(), ["band:evening", "day:weekday", "wx:rain"]);
    assert_eq!(b.when(), "thu:evening");
    let s = Bucket { band: "midday", weekday: 6, weather: None };
    assert_eq!(s.keys(), ["band:midday", "day:weekend"], "no weather, no weather key");
    // 2026-10-04 is a Sunday; 17:30 in Tirana (UTC+2 in summer) is 15:30 UTC.
    let z = dowiz_hub::tz::from_settings(Some("Europe/Tirane"), None);
    let sun = Bucket::at(z, 1_791_127_800_000, None);
    assert_eq!((sun.band, sun.weekday), ("evening", 6), "{sun:?}");
}

fn set_place(site: &Site, t: &str) {
    let r = site.run(crate::services::venue::place::set_place,
        post(&at("/api/owner/place?location_id=alpha"), &json!({"lat": 41.3131, "lng": 19.4453})).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}
fn ctx_of(site: &Site, q: &str) -> crate::wire::Reply {
    site.run(super::context, get(&at(&format!("/api/public/locations/alpha/context{q}"))).on("alpha"), &[("slug", "alpha")])
}

#[test]
fn the_route_asks_open_meteo_for_the_venue_place_and_answers_the_moment() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    set_place(&site, &t);
    answer_outbound(|c| {
        assert!(c.url().unwrap().to_string().starts_with(OPEN_METEO), "{:?}", c.url());
        crate::wire::Reply::from_json(&meteo(12.0, 1.2, 63))
    });
    let r = ctx_of(&site, "");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!(v["contract"], CONTRACT);
    assert_eq!(v["weather"], "rain", "{v}");
    assert!(BANDS.contains(&v["band"].as_str().unwrap()));
    assert!(WEEKDAYS.contains(&v["weekday"].as_str().unwrap()));
    let asked: Vec<String> = sent().iter().map(|c| c.url().unwrap().to_string()).collect();
    assert!(asked.iter().any(|u| u.contains("latitude=41.31&longitude=19.45")), "the VENUE's place: {asked:?}");
}

#[test]
fn a_provider_failure_is_no_weather_never_an_error() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    set_place(&site, &t);
    answer_outbound(|_| Err(worker::Error::RustError("unreachable".into())));
    let r = ctx_of(&site, "");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["weather"], Value::Null);
    answer_outbound(|_| crate::wire::Reply::from_json(&json!({"error": true})));
    assert_eq!(ctx_of(&site, "").body_value()["weather"], Value::Null, "a malformed answer too");
}

#[test]
fn the_guest_location_has_no_way_in() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    set_place(&site, &t);
    answer_outbound(|_| crate::wire::Reply::from_json(&meteo(20.0, 0.0, 0)));
    for q in ["?lat=48.1&lon=11.5", "?latitude=1", "?x=1"] {
        let r = ctx_of(&site, q);
        assert_eq!(r.status_code(), 400, "{q}: {}", r.body_str());
    }
    for c in sent() {
        let u = c.url().unwrap().to_string();
        assert!(!u.contains("48.1") && !u.contains("11.5"), "a guest coordinate reached the provider: {u}");
    }
    // Positive twin: no query, the venue's moment.
    assert_eq!(ctx_of(&site, "").status_code(), 200);
}

#[test]
fn a_venue_without_a_place_has_no_weather_and_asks_nobody() {
    let site = Site::new();
    open_venue(&site, "alpha", "a@x.test");
    answer_outbound(|_| panic!("no place, no call"));
    let r = ctx_of(&site, "");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["weather"], Value::Null);
}
