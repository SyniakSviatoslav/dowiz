use super::*;

fn add(dish: &str, qty: u32, table: Option<u32>) -> Said {
    Said::Add { dish: dish.into(), qty, table }
}

#[test]
fn a_table_is_opened_with_or_without_its_guests() {
    assert_eq!(waiter("open table 5 for 4"), Said::Open { table: 5, guests: Some(4) });
    assert_eq!(waiter("Відкрий стіл 5 на чотири"), Said::Open { table: 5, guests: Some(4) });
    assert_eq!(waiter("hap tavolinën 5 për 4 veta"), Said::Open { table: 5, guests: Some(4) });
    assert_eq!(waiter("open table 12"), Said::Open { table: 12, guests: None });
    // Open with no table number is a question back, not a guess.
    assert_eq!(waiter("open table"), Said::Unclear("which_table"));
    assert_eq!(waiter("open"), Said::Unclear("which_table"));
}

#[test]
fn a_dish_is_added_with_its_count_and_table() {
    assert_eq!(waiter("add 2 margherita to table 5"), add("margherita", 2, Some(5)));
    assert_eq!(waiter("add two cola"), add("cola", 2, None));
    assert_eq!(waiter("додай дві маргарити на стіл 5"), add("маргарити", 2, Some(5)));
    assert_eq!(waiter("додай колу х3"), add("колу", 3, None));
    assert_eq!(waiter("shto dy margarita në tavolinën 5"), add("margarita", 2, Some(5)));
    assert_eq!(waiter("add pizza four cheese"), add("pizza cheese", 4, None));
    assert_eq!(waiter("add tiramisu"), add("tiramisu", 1, None));
}

#[test]
fn an_add_missing_its_dish_or_with_two_counts_is_refused() {
    assert_eq!(waiter("add 2 to table 5"), Said::Unclear("which_dish"));
    assert_eq!(waiter("add 2 cola 3"), Said::Unclear("two_numbers"));
    assert_eq!(waiter("add cola to table"), Said::Unclear("which_table"));
}

#[test]
fn the_round_is_sent_with_or_without_a_table() {
    assert_eq!(waiter("send the round"), Said::Send { table: None });
    assert_eq!(waiter("відправ замовлення на стіл 3"), Said::Send { table: Some(3) });
    assert_eq!(waiter("dërgo porosinë"), Said::Send { table: None });
}

#[test]
fn paid_needs_a_table_and_exactly_one_method() {
    assert_eq!(waiter("table 5 paid cash"), Said::Paid { table: 5, method: Method::Cash });
    assert_eq!(waiter("стіл 5 оплатив карткою"), Said::Paid { table: 5, method: Method::Card });
    assert_eq!(waiter("tavolina 5 paguar kesh"), Said::Paid { table: 5, method: Method::Cash });
    assert_eq!(waiter("tavolina 7 paguar me kartë"), Said::Paid { table: 7, method: Method::Card });
    assert_eq!(waiter("table 5 paid"), Said::Unclear("cash_or_card"));
    assert_eq!(waiter("table 5 paid cash and card"), Said::Unclear("cash_or_card"));
    assert_eq!(waiter("paid cash"), Said::Unclear("which_table"));
    assert_eq!(Method::Card.as_str(), "card");
    assert_eq!(Method::Cash.as_str(), "cash");
}

#[test]
fn two_commands_in_one_breath_are_refused() {
    assert_eq!(waiter("add cola and send"), Said::Unclear("more_than_one"));
    assert_eq!(waiter("open table 5 table 5 paid cash"), Said::Unclear("more_than_one"));
}

#[test]
fn a_question_is_status_and_anything_else_is_not_a_room_command() {
    assert_eq!(waiter("how many tables"), Said::Status);
    assert_eq!(waiter("скільки столів"), Said::Status);
    assert_eq!(waiter("what a lovely day"), Said::Unclear("not_room"));
    assert_eq!(waiter("  "), Said::Unclear("nothing"));
}

#[test]
fn the_owner_takes_a_dish_off_and_puts_it_back() {
    let off = |d: &str| Some(Said::DishSale { dish: d.into(), on: false });
    let on = |d: &str| Some(Said::DishSale { dish: d.into(), on: true });
    assert_eq!(owner("margherita is off"), off("margherita"));
    assert_eq!(owner("take the tiramisu off the menu"), off("tiramisu"));
    assert_eq!(owner("margherita sold out"), off("margherita"));
    assert_eq!(owner("зніми маргариту з продажу"), off("маргариту"));
    assert_eq!(owner("hiq margaritën nga menuja"), off("margaritën"));
    assert_eq!(owner("put margherita back on sale"), on("margherita"));
    assert_eq!(owner("поверни маргариту в продаж"), on("маргариту"));
    assert_eq!(owner("rikthe margaritën përsëri"), on("margaritën"));
    assert_eq!(owner("stop"), Some(Said::Unclear("which_dish")));
    assert_eq!(owner("take margherita off and put cola back"), Some(Said::Unclear("more_than_one")));
}

#[test]
fn the_owner_sets_the_venue_state() {
    assert_eq!(owner("venue busy"), Some(Said::Venue { state: "busy" }));
    assert_eq!(owner("закрий заклад"), Some(Said::Venue { state: "closed" }));
    assert_eq!(owner("hap lokalin"), Some(Said::Venue { state: "open" }));
    assert_eq!(owner("open or close the venue"), Some(Said::Unclear("more_than_one")));
    // The venue named with no state is not ours; the hub's grammar reads it.
    assert_eq!(owner("скільки у закладі замовлень"), None);
}

/// The hub's own verbs are not captured here: they fall through to it.
#[test]
fn order_verbs_are_left_to_the_hubs_grammar() {
    for t in ["підтверди останнє", "готово", "reject 4821", "how many are waiting", "konfirmo të fundit"] {
        assert_eq!(owner(t), None, "{t}");
    }
}

/// Every phrase the "By voice" lessons (docs/learn/lessons/owner/O19.yaml,
/// waiter/W12.yaml) teach, in all three languages, does what the lesson says.
#[test]
fn the_lessons_phrases_work() {
    use dowiz_hub::voice::{classify, Command, Speaker, Target};
    let hub = |t: &str| {
        assert_eq!(owner(t), None, "{t} must reach the hub's grammar");
        classify(t, 0.9, true, Speaker::Owner)
    };
    let digits = || Target::Digits("4821".into());
    for t in ["accept the last one", "прийми останнє", "prano të fundit"] {
        assert_eq!(hub(t), Command::Order { verb: "confirm", target: Target::Newest }, "{t}");
    }
    for t in ["ready 4821", "готово 4821", "gati 4821"] {
        assert_eq!(hub(t), Command::Order { verb: "ready", target: digits() }, "{t}");
    }
    for t in ["reject 4821", "відхили 4821", "refuzo 4821"] {
        assert_eq!(hub(t), Command::Order { verb: "reject", target: digits() }, "{t}");
    }
    for t in ["how many are waiting", "скільки чекає", "sa janë në pritje"] {
        assert_eq!(hub(t), Command::Status, "{t}");
    }
    for (t, on) in [("margherita is off", false), ("зніми маргариту з продажу", false), ("hiq margaritën nga menuja", false),
                    ("put margherita back on sale", true), ("поверни маргариту в продаж", true), ("rikthe margaritën", true)] {
        assert!(matches!(owner(t), Some(Said::DishSale { on: o, .. }) if o == on), "{t}");
    }
    for (t, s) in [("venue busy", "busy"), ("заклад зайнятий", "busy"), ("lokali i zënë", "busy"),
                   ("close the venue", "closed"), ("закрий заклад", "closed"), ("mbyll lokalin", "closed")] {
        assert_eq!(owner(t), Some(Said::Venue { state: s }), "{t}");
    }
    for t in ["open table 5 for 4", "відкрий стіл 5 на чотири", "hap tavolinën 5 për 4 veta"] {
        assert_eq!(waiter(t), Said::Open { table: 5, guests: Some(4) }, "{t}");
    }
    for t in ["add 2 margherita to table 5", "додай дві маргарити на стіл 5", "shto dy margarita në tavolinën 5"] {
        assert!(matches!(waiter(t), Said::Add { qty: 2, table: Some(5), .. }), "{t}");
    }
    for t in ["add cola", "додай колу", "shto një kola"] {
        assert!(matches!(waiter(t), Said::Add { qty: 1, table: None, .. }), "{t}");
    }
    for t in ["send the round", "відправ замовлення", "dërgo porosinë"] {
        assert_eq!(waiter(t), Said::Send { table: None }, "{t}");
    }
    for t in ["table 5 paid cash", "стіл 5 оплатив готівкою", "tavolina 5 paguar kesh"] {
        assert_eq!(waiter(t), Said::Paid { table: 5, method: Method::Cash }, "{t}");
    }
    for t in ["table 5 paid card", "стіл 5 оплата карткою", "tavolina 5 paguar me kartë"] {
        assert_eq!(waiter(t), Said::Paid { table: 5, method: Method::Card }, "{t}");
    }
    for t in ["status", "скільки столів", "statusi"] {
        assert_eq!(waiter(t), Said::Status, "{t}");
    }
}
