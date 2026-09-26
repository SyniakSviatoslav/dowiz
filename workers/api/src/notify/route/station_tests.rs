//! W0c: the sushi counter is a station of its own. A group that took the
//! station `sushi` gets only the sushi lines; a venue with no sushi group
//! keeps them on the kitchen's ticket. Every refusal has a twin.
use super::*;
use groups::Pii;
use dowiz_hub::tz::{Dst, Zone};
use serde_json::json;

const UTC: Zone = Zone { standard_minutes: 0, dst: Dst::None };
const T0: i64 = 1_768_435_200_000;
const TICKET: &str = "🍣 Dubin — #abcdefgh\n🍽 table 4\n\n1 × Maki — 600 ALL\n1 × Beer — 300 ALL\n1 × Ramen — 900 ALL";

fn group(id: &str, station: Option<&str>) -> Group {
    let mut g = Group::fresh(id.into(), format!("-{id}"), None, id.into(), "group".into(), "en");
    g.pii = Pii::Full;
    g.subs = [("order.placed".to_string(), Mode::Now)].into_iter().collect();
    g.station = station.map(str::to_string);
    g
}

fn line(name: &str, station: &str) -> Value {
    json!({ "name": name, "quantity": 1, "station": station })
}

fn order() -> Entry {
    let lines = vec![line("Maki", "sushi"), line("Beer", "bar"), line("Ramen", "kitchen")];
    routed("o1/route".into(), "order.placed", &json!({ "ticket": TICKET, "lines": lines, "amend": false }), T0)
}

#[test]
fn a_sushi_group_gets_only_the_sushi_lines() {
    let groups = [group("k", Some("kitchen")), group("s", Some("sushi")), group("b", Some("bar"))];
    let p = fan_out(&order(), &groups, UTC, T0);
    let texts: Vec<&str> = p.send.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts.len(), 3);
    assert!(texts[0].contains("[kitchen]\n1 × Ramen") && !texts[0].contains("Maki") && !texts[0].contains("Beer"), "{}", texts[0]);
    assert!(texts[1].contains("[sushi]\n1 × Maki") && !texts[1].contains("Ramen"), "{}", texts[1]);
    assert!(texts[2].contains("[bar]\n1 × Beer"), "{}", texts[2]);
}

/// ITS TWIN: no sushi group -- the sushi line is the kitchen's, never lost.
#[test]
fn with_no_sushi_group_the_sushi_line_rides_with_the_kitchen() {
    let groups = [group("k", Some("kitchen")), group("b", Some("bar"))];
    let p = fan_out(&order(), &groups, UTC, T0);
    assert_eq!(p.send.len(), 2);
    assert!(p.send[0].text.contains("1 × Maki") && p.send[0].text.contains("1 × Ramen"), "{}", p.send[0].text);
    assert!(!p.send[0].text.contains("Beer"));
}

#[test]
fn a_sushi_group_with_no_sushi_line_hears_nothing_and_an_unsplit_order_is_todays_ticket() {
    let groups = [group("k", Some("kitchen")), group("s", Some("sushi"))];
    let lines = vec![line("Ramen", "kitchen")];
    let e = routed("o2/route".into(), "order.placed", &json!({ "ticket": TICKET, "lines": lines, "amend": false }), T0);
    let p = fan_out(&e, &groups, UTC, T0);
    assert_eq!(p.send.iter().map(|e| e.to.as_str()).collect::<Vec<_>>(), vec!["-k"]);
    assert_eq!(p.send[0].text, TICKET);
}

#[test]
fn split_out_names_only_stations_with_a_line_and_a_listening_group() {
    let lines = vec![line("Maki", "sushi"), line("Beer", "bar")];
    let mut muted = group("s", Some("sushi"));
    muted.state = State::Muted;
    assert_eq!(split_out(&lines, &[muted, group("b", Some("bar"))], "order.placed"), vec![crate::bell_route::Station::Bar]);
    let both = split_out(&lines, &[group("s", Some("sushi")), group("b", Some("bar"))], "order.placed");
    assert_eq!(both, vec![crate::bell_route::Station::Sushi, crate::bell_route::Station::Bar]);
}

#[test]
fn an_owner_sets_a_groups_station_from_the_closed_set_and_can_clear_it() {
    let mut list = vec![group("g", None)];
    let p: groups::Patch = serde_json::from_value(json!({ "station": "sushi" })).unwrap();
    groups::apply(&mut list, "g", &p).unwrap();
    assert_eq!(list[0].station.as_deref(), Some("sushi"));
    let bad: groups::Patch = serde_json::from_value(json!({ "station": "grill" })).unwrap();
    assert!(groups::apply(&mut list, "g", &bad).unwrap_err().contains("sushi, kitchen or bar"));
    assert_eq!(list[0].station.as_deref(), Some("sushi"), "a refusal changes nothing");
    let clear: groups::Patch = serde_json::from_value(json!({ "station": null })).unwrap();
    groups::apply(&mut list, "g", &clear).unwrap();
    assert_eq!(list[0].station, None);
    let untouched: groups::Patch = serde_json::from_value(json!({ "title": "x" })).unwrap();
    list[0].station = Some("bar".into());
    groups::apply(&mut list, "g", &untouched).unwrap();
    assert_eq!(list[0].station.as_deref(), Some("bar"), "a patch without station keeps it");
}
