//! The kitchen's words: every write is a proposal, and every verb asks for its
//! capability first. Each refusal has a positive twin.

use super::*;
use dowiz_hub::caps::Preset;

fn shelf() -> Vec<Supply> {
    supplies(&[
        ("salmon".into(), json!({ "name": "Salmon fillet", "unit": "g" }).to_string()),
        ("soy".into(), json!({ "name": "Soy sauce", "unit": "ml" }).to_string()),
        ("nori".into(), json!({ "name": "Nori", "unit": "unit" }).to_string()),
    ])
}
fn kitchen() -> Caps { Preset::Kitchen.caps() }
fn verb(o: &Out) -> Option<(&str, &str)> {
    match o { Out::Propose { verb, arg, .. } => Some((verb, arg.as_str())), _ => None }
}

#[test]
fn a_kitchen_is_a_member_of_staff_without_the_room() {
    assert!(is_kitchen(&kitchen()));
    assert!(is_kitchen(&Caps::of(&[Cap::Stock])));
    assert!(!is_kitchen(&Preset::Waiter.caps()), "a waiter keeps the waiter's grammar");
    assert!(!is_kitchen(&Preset::CounterManager.caps()));
    assert!(!is_kitchen(&Caps::of(&[Cap::OpenTill])));
}

#[test]
fn the_shelf_words_in_three_languages() {
    assert_eq!(stock_said("received 4 kg salmon"), Some(Said::Receive { item: "salmon".into(), qty: 4000, unit: Some("g") }));
    assert_eq!(stock_said("прийшло 2000 г лосось"), Some(Said::Receive { item: "лосось".into(), qty: 2000, unit: Some("g") }));
    assert_eq!(stock_said("erdhi 3 nori"), Some(Said::Receive { item: "nori".into(), qty: 3, unit: None }));
    assert_eq!(stock_said("write off 300 g salmon spoiled"), Some(Said::Waste { item: "salmon".into(), qty: 300, unit: Some("g"), reason: Some("spoiled") }));
    assert_eq!(stock_said("waste 300 g salmon spoiled"), Some(Said::Waste { item: "salmon".into(), qty: 300, unit: Some("g"), reason: Some("spoiled") }));
    assert_eq!(stock_said("списати 2 л соус впало"), Some(Said::Waste { item: "соус".into(), qty: 2000, unit: Some("ml"), reason: Some("dropped") }));
    assert_eq!(stock_said("waste 1 salmon"), Some(Said::Waste { item: "salmon".into(), qty: 1, unit: None, reason: None }));
    assert_eq!(stock_said("show me the stock"), Some(Said::Show("stock")));
    assert_eq!(stock_said("покажи кухню"), Some(Said::Show("kitchen")));
    assert_eq!(stock_said("trego menunë"), Some(Said::Show("menu")));
    assert_eq!(stock_said("table 5 paid cash"), None, "not the kitchen's words");
    assert_eq!(stock_said("show me"), None);
}

#[test]
fn the_shelf_words_refuse_what_they_cannot_read() {
    assert_eq!(stock_said("received salmon"), Some(Said::Unclear("how_much")));
    assert_eq!(stock_said("received 2 3 salmon"), Some(Said::Unclear("two_numbers")));
    assert_eq!(stock_said("received 5 kg"), Some(Said::Unclear("which_supply")));
    assert_eq!(stock_said("received 5000 kg salmon"), Some(Said::Unclear("how_much")), "5 tonnes is a typo");
    assert_eq!(stock_said("received and waste 5 salmon"), Some(Said::Unclear("more_than_one")));
    assert_eq!(stock_said("show kitchen stock"), Some(Said::Unclear("more_than_one")));
}

#[test]
fn a_receive_and_a_write_off_are_proposals_the_kitchen_confirms() {
    let r = decide(&Said::Receive { item: "salmon".into(), qty: 4000, unit: Some("g") }, &kitchen(), "en", &shelf());
    assert_eq!(verb(&r), Some(("receive", "salmon|4000")));
    let Out::Propose { readback, extra, .. } = &r else { panic!("{r:?}") };
    assert_eq!(readback, "received onto the shelf: 4000 g Salmon fillet");
    assert_eq!(extra["itemId"], "salmon");
    let w = decide(&Said::Waste { item: "soy sauce".into(), qty: 200, unit: None, reason: Some("spoiled") }, &kitchen(), "uk", &shelf());
    assert_eq!(verb(&w), Some(("waste", "soy|200|spoiled")));
    // The confirmation decodes to the stock route's body.
    assert_eq!(scope::decode("receive", "salmon|4000"), Some(json!({ "itemId": "salmon", "qty": 4000 })));
    assert_eq!(scope::decode("waste", "soy|200|spoiled"), Some(json!({ "itemId": "soy", "qty": 200, "reason": "spoiled" })));
    assert!(scope::is_ours("receive") && scope::is_ours("waste"));
}

#[test]
fn the_shelf_refuses_a_role_without_it_a_missing_reason_and_a_wrong_unit() {
    let rec = Said::Receive { item: "salmon".into(), qty: 1, unit: None };
    assert!(matches!(decide(&rec, &Preset::Waiter.caps(), "en", &shelf()), Out::Refuse(s) if s == "Your role does not do that"));
    let bin = Said::Waste { item: "salmon".into(), qty: 1, unit: None, reason: Some("dropped") };
    assert_eq!(verb(&decide(&bin, &Caps::of(&[Cap::OpenTill]), "en", &shelf())), Some(("waste", "salmon|1|dropped")), "the till holder bins at midnight");
    assert!(matches!(decide(&rec, &Caps::of(&[Cap::OpenTill]), "en", &shelf()), Out::Refuse(_)), "but does not receive");
    let why = Said::Waste { item: "salmon".into(), qty: 1, unit: None, reason: None };
    assert!(matches!(decide(&why, &kitchen(), "en", &shelf()), Out::Refuse(s) if s.starts_with("Why?")));
    let unit = Said::Receive { item: "nori".into(), qty: 1000, unit: Some("g") };
    assert!(matches!(decide(&unit, &kitchen(), "en", &shelf()), Out::Refuse(s) if s.contains("another unit")));
    let none = Said::Receive { item: "tuna".into(), qty: 1, unit: None };
    assert!(matches!(decide(&none, &kitchen(), "en", &shelf()), Out::Refuse(s) if s.contains("«tuna»")));
    assert!(matches!(decide(&Said::Unclear("how_much"), &kitchen(), "sq", &shelf()), Out::Refuse(s) if s == "Sa?"));
}

#[test]
fn show_me_navigates_only_to_a_screen_the_role_opens() {
    assert_eq!(decide(&Said::Show("stock"), &kitchen(), "en", &[]), Out::Now(json!({ "action": "show", "screen": "stock" })));
    assert_eq!(decide(&Said::Show("kitchen"), &kitchen(), "en", &[]), Out::Now(json!({ "action": "show", "screen": "kitchen" })));
    assert!(matches!(decide(&Said::Show("stock"), &Caps::of(&[Cap::Advance]), "en", &[]), Out::Refuse(_)));
    // Ingredients and stock are one screen: the menu word opens it too.
    assert_eq!(decide(&Said::Show("stock"), &Caps::of(&[Cap::Catalog]), "en", &[]), Out::Now(json!({ "action": "show", "screen": "stock" })));
    assert_eq!(stock_said("show me the ingredients"), Some(Said::Show("stock")));
    assert_eq!(stock_said("покажи інгредієнти"), Some(Said::Show("stock")));
    assert!(matches!(decide(&Said::Show("menu"), &Preset::Waiter.caps(), "en", &[]), Out::Refuse(_)));
}

fn pool() -> Vec<Value> {
    vec![json!({ "id": "ord_4821", "status": "PENDING", "created_at_ms": 2 }), json!({ "id": "ord_1133", "status": "PREPARING", "created_at_ms": 1 })]
}

#[test]
fn an_order_verb_needs_advance_and_names_one_ticket() {
    let cmd = classify("ready 4821", 1.0, true, Speaker::Owner);
    assert_eq!(verb(&order(&cmd, &kitchen(), &pool(), "en")), Some(("ready", "ord_4821")));
    assert!(matches!(order(&cmd, &Caps::of(&[Cap::Stock]), &pool(), "en"), Out::Refuse(_)), "the shelf does not move tickets");
    let reject = classify("reject the first", 1.0, true, Speaker::Owner);
    assert_eq!(verb(&order(&reject, &kitchen(), &pool(), "en")), Some(("reject", "ord_1133")), "Q1: the kitchen may reject");
    assert_eq!(order(&Command::Status, &kitchen(), &pool(), "en"), Out::Now(json!({ "action": "status", "open": 2, "waiting": 1 })));
    assert_eq!(order(&Command::Ask("q".into()), &kitchen(), &[], "en"), Out::Now(json!({ "action": "ask", "question": "q" })));
    assert!(matches!(order(&Command::Shift { open: true }, &kitchen(), &[], "en"), Out::Refuse(_)));
    assert!(matches!(order(&Command::Unclear("x"), &kitchen(), &[], "en"), Out::Refuse(s) if s == "x"));
}

#[test]
fn a_spoken_target_resolves_to_one_order_or_is_refused() {
    let p = pool();
    assert_eq!(resolve(&p, &Target::Newest), Ok("ord_4821".into()));
    assert_eq!(resolve(&p, &Target::Oldest), Ok("ord_1133".into()));
    assert_eq!(resolve(&p, &Target::Digits("33".into())), Ok("ord_1133".into()));
    assert!(resolve(&p, &Target::Digits("9".into())).is_err());
    assert!(resolve(&p, &Target::Unsaid).is_err(), "two open: which one?");
    let dup = vec![json!({ "id": "a_11" }), json!({ "id": "b_11" })];
    assert!(resolve(&dup, &Target::Digits("11".into())).is_err(), "ambiguity is refused");
    assert_eq!(resolve(&p[..1], &Target::Unsaid), Ok("ord_4821".into()));
    assert!(resolve(&[], &Target::Newest).is_err());
}

/// "прийняти 4821" is the hub's CONFIRM (accept the order), never a delivery:
/// the shelf's words must not swallow an order verb.
#[test]
fn the_shelf_words_leave_the_order_verbs_alone() {
    for said in ["прийняти 4821", "confirm 4821", "prano 4821", "ready 4821", "86 salmon", "take salmon off", "status"] {
        assert_eq!(stock_said(said), None, "{said}");
    }
}

#[test]
fn a_write_off_reads_back_its_reason_in_the_speakers_language() {
    let w = Said::Waste { item: "salmon".into(), qty: 300, unit: None, reason: Some("dropped") };
    let Out::Propose { readback, .. } = decide(&w, &kitchen(), "sq", &shelf()) else { panic!() };
    assert_eq!(readback, "hiq nga magazina: 300 g Salmon fillet (i rënë)");
    let Out::Propose { readback, .. } = decide(&w, &kitchen(), "uk", &shelf()) else { panic!() };
    assert_eq!(readback, "списати: 300 g Salmon fillet (впало)");
    let r = Said::Receive { item: "nori".into(), qty: 20, unit: None };
    let Out::Propose { readback, .. } = decide(&r, &kitchen(), "sq", &shelf()) else { panic!() };
    assert_eq!(readback, "erdhi në magazinë: 20 unit Nori");
}

/// THE PANEL'S STARTER LINES ARE SENT AS TYPED (`admin/assistant-i18n.js`),
/// so each must mean what its chip says, in every language it is written in.
#[test]
fn the_starter_lines_mean_what_they_say() {
    for status in ["sa porosi presin", "status", "скільки чекає"] {
        assert_eq!(stock_said(status), None, "{status}");
        assert_eq!(grammar::owner(status), None, "{status}");
        assert_eq!(classify(status, 1.0, true, Speaker::Owner), Command::Status, "{status}");
    }
    for (line, screen) in [("trego kuzhinën", "kitchen"), ("show me the kitchen", "kitchen"), ("покажи кухню", "kitchen"),
                           ("trego magazinën", "stock"), ("show me the stock", "stock"), ("покажи склад", "stock")] {
        assert_eq!(stock_said(line), Some(Said::Show(screen)), "{line}");
    }
    for q in ["Cilët përbërës po mbarojnë?", "Which ingredients are running low?", "Яких інгредієнтів мало?"] {
        assert_eq!(stock_said(q), None, "{q}");
        assert_eq!(grammar::owner(q), None, "{q}");
        assert!(matches!(classify(q, 1.0, true, Speaker::Owner), Command::Ask(_)), "{q} must reach the assistant");
    }
}
