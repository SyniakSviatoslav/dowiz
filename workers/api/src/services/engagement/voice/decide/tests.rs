use super::*;
use crate::services::engagement::voice::grammar::{owner as heard_owner, waiter as heard};
use serde_json::json;

fn dish(id: &str, names: &[&str], available: bool) -> Dish {
    Dish { id: id.into(), names: names.iter().map(|s| s.to_string()).collect(), available }
}
fn menu() -> Vec<Dish> {
    vec![
        dish("marg", &["Margherita", "Маргарита", "Margarita"], true),
        dish("pep", &["Pepperoni"], true),
        dish("cola", &["Cola"], true),
        dish("tira", &["Tiramisu"], false),
    ]
}
fn round(id: &str, status: &str, total: i64, paid: i64) -> Value {
    json!({ "id": id, "seq": 3, "status": status, "total": total, "paid": paid, "payment_status": "unpaid" })
}
fn room_of(rounds: Vec<Value>) -> Vec<Value> {
    vec![json!({ "sitting_id": "s5", "table": "5", "rounds": rounds })]
}
fn all() -> Caps {
    Caps::of(&Cap::ALL)
}
fn only(c: &[Cap]) -> Caps {
    Caps::of(c)
}
fn run(t: &str, caps: Caps, sittings: &[Value], draft: Option<&Draft>) -> Out {
    let m = menu();
    waiter(&heard(t), &Room { lang: "en", caps, sittings, menu: &m, draft })
}
fn refused(o: &Out) -> &str {
    match o {
        Out::Refuse(s) => s,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn adding_to_a_round_the_kitchen_has_not_taken_is_a_proposal() {
    let sittings = room_of(vec![round("r1", "PENDING", 900, 0)]);
    let o = run("add 2 margherita to table 5", all(), &sittings, None);
    assert_eq!(
        o,
        Out::Propose {
            verb: "add",
            arg: "r1|3|marg|2".into(),
            readback: "add 2 × Margherita to table 5".into(),
            extra: json!({ "orderId": "r1", "table": "5" }),
        }
    );
}

#[test]
fn adding_where_no_round_is_open_builds_the_phones_round_at_once() {
    let o = run("add cola to table 7", all(), &[], None);
    assert_eq!(o, Out::Now(json!({ "action": "draft_add", "table": "7", "productId": "cola", "name": "Cola", "quantity": 1 })));
    // No table said: the round being built names it.
    let draft = Draft { table: "7".into(), items: vec![] };
    assert!(matches!(run("add cola", all(), &[], Some(&draft)), Out::Now(_)));
    // And neither: asked, not guessed.
    assert_eq!(refused(&run("add cola", all(), &[], None)), "Which table?");
    // A round being built for table 7 is not re-pointed at table 8.
    let busy = Draft { table: "7".into(), items: vec![("cola".into(), 1)] };
    assert_eq!(refused(&run("add cola to table 8", all(), &[], Some(&busy))), say::line("other_table", "en"));
}

#[test]
fn a_dish_that_is_off_sale_unknown_or_ambiguous_is_refused() {
    assert_eq!(refused(&run("add tiramisu to table 5", all(), &[], None)), "That dish is off sale");
    assert_eq!(refused(&run("add sushi to table 5", all(), &[], None)), "No dish by that name «sushi»");
    let two = vec![dish("a", &["Pizza A"], true), dish("b", &["Pizza B"], true)];
    let o = waiter(&heard("add pizza to table 5"), &Room { lang: "en", caps: all(), sittings: &[], menu: &two, draft: None });
    assert_eq!(refused(&o), "Several dishes fit: Pizza A, Pizza B");
}

/// The capability law: what the role cannot do, voice cannot either.
#[test]
fn a_waiter_without_the_capability_is_refused_and_one_with_it_is_not() {
    let sittings = room_of(vec![round("r1", "READY", 1500, 0)]);
    let no_pay = only(&[Cap::TakeOrders]);
    assert_eq!(refused(&run("table 5 paid cash", no_pay, &sittings, None)), "Your role does not take payments");
    assert!(matches!(run("table 5 paid cash", only(&[Cap::TakePayment]), &sittings, None), Out::Propose { verb: "pay", .. }));
    let kitchen = only(&[Cap::Advance]);
    for t in ["add cola to table 5", "open table 5", "send the round", "status"] {
        assert_eq!(refused(&run(t, kitchen, &sittings, None)), "Your role does not take orders", "{t}");
    }
    assert!(matches!(run("open table 5", only(&[Cap::TakeOrders]), &[], None), Out::Now(_)));
}

#[test]
fn paid_proposes_exactly_what_is_owed() {
    let sittings = room_of(vec![round("r1", "READY", 1500, 400)]);
    let o = run("table 5 paid card", all(), &sittings, None);
    assert_eq!(
        o,
        Out::Propose {
            verb: "pay",
            arg: "r1|3|1100|card".into(),
            readback: "table 5: paid by card".into(),
            extra: json!({ "orderId": "r1", "table": "5", "amount": 1100, "method": "card" }),
        }
    );
    assert_eq!(refused(&run("table 9 paid cash", all(), &sittings, None)), "That table is not open");
}

#[test]
fn sending_proposes_the_phones_round_read_back_by_name() {
    let draft = Draft { table: "5".into(), items: vec![("marg".into(), 2), ("cola".into(), 1)] };
    let o = run("send the round", all(), &[], Some(&draft));
    assert_eq!(
        o,
        Out::Propose {
            verb: "place",
            arg: "5|marg*2,cola*1".into(),
            readback: "send to table 5: 2 × Margherita, 1 × Cola".into(),
            extra: json!({ "table": "5" }),
        }
    );
    assert!(matches!(run("send table 5", all(), &[], Some(&draft)), Out::Propose { .. }));
    assert_eq!(refused(&run("send table 6", all(), &[], Some(&draft))), say::line("other_table", "en"));
    assert_eq!(refused(&run("send", all(), &[], None)), "The round is empty");
    let empty = Draft { table: "5".into(), items: vec![] };
    assert_eq!(refused(&run("send", all(), &[], Some(&empty))), "The round is empty");
}

#[test]
fn a_round_that_does_not_match_the_menu_is_refused() {
    let bad = |items: Vec<(String, u32)>, table: &str| {
        let d = Draft { table: table.into(), items };
        run("send", all(), &[], Some(&d))
    };
    assert_eq!(refused(&bad(vec![("ghost".into(), 1)], "5")), "The round on this phone cannot be read");
    assert_eq!(refused(&bad(vec![("cola".into(), 0)], "5")), "The round on this phone cannot be read");
    assert_eq!(refused(&bad(vec![("tira".into(), 1)], "5")), "That dish is off sale «Tiramisu»");
    assert_eq!(refused(&bad(vec![("cola".into(), 1)], " ")), "Which table?");
    assert_eq!(refused(&bad(vec![("cola".into(), 1)], "5|6")), "The round on this phone cannot be read");
    assert_eq!(refused(&bad(vec![("cola".into(), 1); DRAFT_MAX + 1], "5")), "The round on this phone cannot be read");
}

#[test]
fn status_and_open_run_at_once_and_the_owners_verbs_are_not_a_waiters() {
    let sittings = room_of(vec![round("r1", "PENDING", 1, 0)]);
    assert_eq!(run("status", all(), &sittings, None), Out::Now(json!({ "action": "status", "open": 1, "waiting": 1 })));
    assert_eq!(run("open table 5 for 4", all(), &[], None), Out::Now(json!({ "action": "open", "table": "5", "guests": 4 })));
    let m = menu();
    let r = Room { lang: "en", caps: all(), sittings: &[], menu: &m, draft: None };
    assert_eq!(refused(&waiter(&Said::Venue { state: "busy" }, &r)), "That is not a room command");
    assert_eq!(refused(&waiter(&Said::Unclear("which_dish"), &r)), "Which dish?");
}

#[test]
fn the_owner_proposes_the_stop_list_and_the_venue_state() {
    let m = menu();
    let o = owner(&heard_owner("зніми маргариту з продажу").unwrap(), "uk", &m);
    assert_eq!(
        o,
        Out::Propose { verb: "dish_off", arg: "marg".into(), readback: "зняти з продажу: Margherita".into(), extra: json!({ "productId": "marg" }) }
    );
    // A dish already off may be put back.
    assert!(matches!(owner(&heard_owner("put tiramisu back").unwrap(), "en", &m), Out::Propose { verb: "dish_on", .. }));
    let v = owner(&heard_owner("venue busy").unwrap(), "en", &m);
    assert_eq!(v, Out::Propose { verb: "venue", arg: "busy".into(), readback: "the venue: busy".into(), extra: json!({ "state": "busy" }) });
    assert_eq!(refused(&owner(&Said::Unclear("which_dish"), "sq", &m)), "Cila pjatë?");
    assert_eq!(refused(&owner(&Said::Status, "en", &m)), "That is not a room command");
    let bad = vec![dish("a|b", &["Soup"], true)];
    assert_eq!(refused(&owner(&Said::DishSale { dish: "soup".into(), on: false }, "en", &bad)), "No dish by that name");
}
