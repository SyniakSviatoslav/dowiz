//! Print routing by station (§7 item 7): the split, the ids, and the one
//! promise that matters most — a venue with no bar chat gets today's bell.

use super::*;
use crate::command::amend::Op;
use serde_json::json;

const FULL: &str = "🍣 Dubin — #r1\n🍽 table 7\n\n1 × Maki — 600 ALL\n1 × Beer — 300 ALL\n\n💰 900 ALL · cash";

fn maki() -> Value {
    json!({"product_id": "maki", "name": "Maki", "quantity": 1, "unit_price": 600, "station": "kitchen"})
}
fn beer() -> Value {
    json!({"product_id": "beer", "name": "Beer", "quantity": 1, "unit_price": 300, "station": "bar"})
}

#[test]
fn a_round_with_a_kitchen_and_a_bar_line_is_two_disjoint_groups() {
    let lines = vec![beer(), maki()];
    let g = split_by_station(&lines);
    assert_eq!(g.len(), 2);
    assert_eq!((g[0].0, g[0].1.clone()), (Station::Kitchen, vec![maki()]));
    assert_eq!((g[1].0, g[1].1.clone()), (Station::Bar, vec![beer()]));
    assert_eq!(g[0].1.len() + g[1].1.len(), lines.len(), "every line in exactly one group");
}

#[test]
fn a_line_with_no_or_an_unknown_station_is_the_kitchens() {
    let lines = vec![json!({"product_id": "a", "quantity": 1}), json!({"product_id": "b", "station": "grill"})];
    let g = split_by_station(&lines);
    assert_eq!(g.len(), 1);
    assert_eq!((g[0].0, g[0].1.len()), (Station::Kitchen, 2));
}

#[test]
fn the_stations_have_distinct_ids_and_the_kitchen_keeps_todays() {
    assert_eq!(entry_id("r1", "telegram", Station::Kitchen), "r1/telegram", "today's id");
    assert_eq!(entry_id("r1", "telegram", Station::Bar), "r1/telegram/bar");
}

#[test]
fn targets_split_only_when_a_bar_chat_is_set_and_a_bar_line_exists() {
    let both = split_by_station(&[maki(), beer()]);
    assert_eq!(targets(&both, None), vec![(Station::Kitchen, None)]);
    assert_eq!(targets(&both, Some("  ")), vec![(Station::Kitchen, None)], "blank is unset");
    assert_eq!(targets(&both, Some("-100bar")), vec![(Station::Kitchen, None), (Station::Bar, Some("-100bar"))]);
    let kitchen_only = split_by_station(&[maki()]);
    assert_eq!(targets(&kitchen_only, Some("-100bar")), vec![(Station::Kitchen, None)]);
}

/// THE PROMISE: no bar chat → ONE entry, today's id, today's chat, the text
/// byte for byte, whatever stations the lines carry.
#[test]
fn a_venue_with_no_bar_chat_gets_exactly_todays_one_bell() {
    let t = telegram_tickets("r1", FULL, &[maki(), beer()], None, "-100kitchen", "");
    assert_eq!(t, vec![Ticket { id: "r1/telegram".into(), to: "-100kitchen".into(), text: FULL.into() }]);
}

/// ITS TWIN: with a bar chat, the same round is two tickets with disjoint lines.
#[test]
fn a_venue_with_a_bar_chat_gets_one_ticket_per_station() {
    let t = telegram_tickets("r1", FULL, &[maki(), beer()], None, "-100kitchen", "-100bar");
    assert_eq!(t.len(), 2);
    assert_eq!((t[0].id.as_str(), t[0].to.as_str()), ("r1/telegram", "-100kitchen"));
    assert_eq!((t[1].id.as_str(), t[1].to.as_str()), ("r1/telegram/bar", "-100bar"));
    assert!(t[0].text.contains("1 × Maki") && !t[0].text.contains("Beer"), "{}", t[0].text);
    assert!(t[1].text.contains("1 × Beer") && !t[1].text.contains("Maki"), "{}", t[1].text);
    assert!(t[1].text.starts_with("🍣 Dubin — #r1\n🍽 table 7\n\n[bar]\n"), "header travels: {}", t[1].text);
}

#[test]
fn a_bar_only_round_at_a_split_venue_rings_only_the_bar() {
    let t = telegram_tickets("r1", FULL, &[beer()], None, "-100kitchen", "-100bar");
    assert_eq!(t.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["r1/telegram/bar"]);
}

#[test]
fn a_venue_with_no_chat_at_all_queues_no_telegram() {
    assert!(telegram_tickets("r1", FULL, &[maki(), beer()], None, " ", "").is_empty());
    // With only a bar chat, the bar still rings and the kitchen has nowhere to go, as today.
    let t = telegram_tickets("r1", FULL, &[maki(), beer()], None, "", "-100bar");
    assert_eq!(t.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["r1/telegram/bar"]);
}

/// AN AMENDMENT THAT ADDS A BAR LINE queues a bar entry, and no kitchen entry
/// because no kitchen line changed — under an id that cannot overwrite the
/// placement's bell.
#[test]
fn an_amendment_adding_a_bar_line_queues_a_bar_entry_only() {
    let ops = vec![Op::Add { line: beer() }, Op::Remove { line: 0 }];
    let added = added_lines(&ops);
    assert_eq!(added, vec![beer()], "only the added lines ring");
    let head = amend_header("r1", &json!({"fulfilment": {"table": "7"}}));
    let t = telegram_tickets("r1", &head, &added, Some(42), "-100kitchen", "-100bar");
    assert_eq!(t.len(), 1);
    assert_eq!((t[0].id.as_str(), t[0].to.as_str()), ("r1/amend/42/telegram/bar", "-100bar"));
    assert_eq!(t[0].text, "+ #r1 — table 7\n\n[bar]\n1 × Beer\n");
    // Twin: kitchen and bar lines added → both rung, ids distinct from placement's.
    let t = telegram_tickets("r1", &head, &[maki(), beer()], Some(42), "-100kitchen", "-100bar");
    let ids: Vec<&str> = t.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, vec!["r1/amend/42/telegram", "r1/amend/42/telegram/bar"]);
}

#[test]
fn an_amendment_at_a_venue_with_no_bar_chat_is_one_kitchen_ticket_or_none() {
    let t = telegram_tickets("r1", "+ #r1", &[beer()], Some(7), "-100kitchen", "");
    assert_eq!(t, vec![Ticket { id: "r1/amend/7/telegram".into(), to: "-100kitchen".into(), text: "+ #r1\n\n1 × Beer\n".into() }]);
    assert!(telegram_tickets("r1", "+ #r1", &[], Some(7), "-100kitchen", "-100bar").is_empty(), "nothing added, nothing rung");
}

#[test]
fn an_owner_may_set_kitchen_or_bar_and_nothing_else() {
    assert_eq!(Station::from_wire("bar"), Ok(Station::Bar));
    assert_eq!(Station::from_wire("kitchen"), Ok(Station::Kitchen));
    assert!(Station::from_wire("Bar ").is_err());
    assert!(Station::from_wire("grill").is_err());
}

#[test]
fn the_stored_orders_lines_are_read_back() {
    let o = json!({"id": "r1", "items": [maki(), beer()]}).to_string();
    assert_eq!(lines_of(&o), vec![maki(), beer()]);
    assert!(lines_of("not json").is_empty());
}

/// A DISH EDIT keeps the station unless the owner moved it (the console sends
/// `station` only when changed); moving it is its twin.
#[test]
fn a_dish_edit_without_a_station_keeps_the_dish_at_the_bar() {
    let mut p = json!({"id": "beer", "price": 300, "station": "bar"});
    p["price"] = json!(350); // what `update_product` does to an edited field
    edit_station(&mut p, None);
    assert_eq!((p["station"].clone(), Station::of_line(&p)), (json!("bar"), Station::Bar));
    edit_station(&mut p, Some(Station::Kitchen));
    assert_eq!(Station::of_line(&p), Station::Kitchen, "the owner moved it");
    edit_station(&mut p, Some(Station::Bar));
    assert_eq!(Station::of_line(&p), Station::Bar);
}

/// The line carries `station` only for the bar, and what it carries is read
/// back by the split: stamp -> store -> lines_of -> split is one path.
#[test]
fn a_stamped_line_survives_the_stored_order_and_splits_at_the_bar() {
    let mut beer_line = json!({"product_id": "beer", "name": "Beer", "quantity": 1});
    let mut maki_line = json!({"product_id": "maki", "name": "Maki", "quantity": 1});
    stamp_line(&mut beer_line, Station::Bar);
    stamp_line(&mut maki_line, Station::Kitchen);
    assert!(maki_line.get("station").is_none(), "absent IS the kitchen");
    let stored = json!({"id": "r1", "items": [maki_line, beer_line]}).to_string();
    let g = split_by_station(&lines_of(&stored));
    assert_eq!(g.iter().map(|(s, l)| (*s, l.len())).collect::<Vec<_>>(), vec![(Station::Kitchen, 1), (Station::Bar, 1)]);
}
