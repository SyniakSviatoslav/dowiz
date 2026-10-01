use super::*;

fn post(id: &str, key: &str, state: State, at: i64) -> Post {
    Post {
        id: id.into(),
        subject_key: key.into(),
        subject_tag: "new".into(),
        text: "Sake Futomaki is on the menu again.".into(),
        channel: Channel::Telegram,
        state,
        created_ms: at,
        decided_ms: 0,
        error: String::new(),
    }
}

#[test]
fn posts_survive_the_byte_image() {
    let mut p = Posts::create().expect("create");
    p.put(&post("p1", "back:item-01", State::Draft, 100));
    p.put(&post("p2", "new:item-02", State::Published, 200));
    let bytes = p.to_bytes().expect("bytes");

    let p = Posts::load(&bytes).expect("load");
    assert_eq!(p.all().len(), 2);
    // Newest first.
    assert_eq!(p.all()[0].id, "p2");
    assert_eq!(p.drafts().len(), 1);
    assert_eq!(p.get("p1").unwrap().state, State::Draft);
    assert_eq!(p.get("nope"), None);
}

/// The property that stops a venue posting the same thing every hour.
#[test]
fn a_fact_already_put_to_the_owner_is_not_drafted_again() {
    let mut p = Posts::create().expect("create");
    p.put(&post("p1", "back:item-01", State::Draft, 100));
    assert!(p.already_seen("back:item-01"));
    assert!(!p.already_seen("back:item-02"));

    // REJECTED counts too. A fact the owner turned down coming back an hour
    // later is how an assistant becomes something people switch off.
    p.put(&post("p2", "new:item-09", State::Rejected, 200));
    assert!(p.already_seen("new:item-09"));
    // And so does published.
    p.put(&post("p3", "top:item-03", State::Published, 300));
    assert!(p.already_seen("top:item-03"));
}

#[test]
fn a_subject_key_is_stable_and_distinct() {
    let a = Subject::BackOnTheMenu { product: "item-01".into() };
    let b = Subject::BackOnTheMenu { product: "item-01".into() };
    assert_eq!(a.key(), b.key());
    assert_ne!(a.key(), Subject::NewDish { product: "item-01".into() }.key());
    // The count does NOT enter the key: "most ordered" is one fact about a
    // dish, and a different number next week is not a new thing to say.
    assert_eq!(
        Subject::MostOrdered { product: "x".into(), orders: 4 }.key(),
        Subject::MostOrdered { product: "x".into(), orders: 9 }.key()
    );
}

#[test]
fn a_fact_reads_as_a_sentence() {
    assert!(Subject::BackOnTheMenu { product: "Sake Futomaki".into() }
        .fact()
        .contains("available again"));
    assert!(Subject::MostOrdered { product: "Ebi Maki".into(), orders: 12 }
        .fact()
        .contains("12 times"));
    assert_eq!(Subject::Reopened.fact(), "the restaurant is open again");
}

/// The last gate. Each of these is something a marketing-trained model
/// reaches for by default and that the restaurant would then have to honour.
#[test]
fn invented_offers_are_refused() {
    for bad in [
        "Sake Futomaki is back! 20% off today only.",
        "Our sushi is back. Limited time discount!",
        "Суші повернулись — знижка 10%!",
        "Ebi Maki is back, and the first one is free today",
        "Zbritje 30% sot!",
        "Hurry, the sushi is back",
        "Поспішайте, суші знову в меню",
    ] {
        assert!(unusable(bad).is_some(), "let through: {bad}");
    }
}

#[test]
fn an_honest_post_passes() {
    for good in [
        "Sake Futomaki is on the menu again. We missed it too.",
        "Суші сет знову в меню — сьогодні готуємо.",
        "Ebi Maki was the one you ordered most this week. Thank you.",
        "Byrek me spinaq është sërish në meny.",
        "We are open again. 🙂",
    ] {
        assert_eq!(unusable(good), None, "wrongly refused: {good}");
    }
}

#[test]
fn nothing_and_a_wall_of_text_are_both_refused() {
    assert!(unusable("").is_some());
    assert!(unusable("   \n  ").is_some());
    assert!(unusable(&"word ".repeat(200)).is_some());
}

/// The prompt must carry the fact and the language, and nothing that invites
/// the model to choose a subject.
#[test]
fn the_prompt_carries_one_fact_and_a_language() {
    let p = prompt_for(
        &Subject::BackOnTheMenu { product: "Sake Futomaki".into() },
        "Dubin & Sushi",
        "uk",
    );
    assert!(p.contains("Dubin & Sushi"));
    assert!(p.contains("Ukrainian"));
    assert!(p.contains("Sake Futomaki"));
    assert!(p.contains("FACT:"));
    assert!(prompt_for(&Subject::Reopened, "V", "sq").contains("Albanian"));
    assert!(prompt_for(&Subject::Reopened, "V", "en").contains("English"));
    assert!(prompt_for(&Subject::Reopened, "V", "ru").contains("LANGUAGE: Russian"));
    assert!(prompt_for(&Subject::Reopened, "V", "de").contains("LANGUAGE: English"));
}

/// The system prompt has to forbid the two failure modes by name.
#[test]
fn the_instructions_ban_what_the_gate_checks() {
    for banned in ["discount", "limited time", "NEVER invent"] {
        assert!(SYSTEM_POST.contains(banned), "missing {banned}");
    }
}

#[test]
fn the_catalogue_snapshot_round_trips() {
    let mut p = Posts::create().expect("create");
    assert!(p.catalogue_snapshot().is_empty(), "nothing remembered yet");
    p.set_catalogue_snapshot(&[("a".into(), true), ("b".into(), false)]);
    let bytes = p.to_bytes().expect("bytes");
    let back = Posts::load(&bytes).expect("load").catalogue_snapshot();
    assert_eq!(back, vec![("a".to_string(), true), ("b".to_string(), false)]);
}

/// The first run must say NOTHING. Otherwise a venue switching this on is
/// handed a draft about every dish on a menu that has not changed.
#[test]
fn the_first_run_is_silent() {
    let current = vec![
        ("a".to_string(), "Sake".to_string(), true),
        ("b".to_string(), "Ebi".to_string(), true),
    ];
    let got = derive_subjects(&[], &current, None, false, true, &|_| false);
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_dish_returning_and_a_dish_arriving_are_both_noticed() {
    let previous = vec![("a".to_string(), false), ("b".to_string(), true)];
    let current = vec![
        ("a".to_string(), "Sake".to_string(), true),   // came back
        ("b".to_string(), "Ebi".to_string(), true),    // unchanged
        ("c".to_string(), "Uni".to_string(), true),    // brand new
    ];
    let got = derive_subjects(&previous, &current, None, false, true, &|_| false);
    assert!(got.contains(&Subject::BackOnTheMenu { product: "Sake".into() }), "{got:?}");
    assert!(got.contains(&Subject::NewDish { product: "Uni".into() }), "{got:?}");
    assert_eq!(got.len(), 2, "an unchanged dish is not news: {got:?}");
}

/// A dish going OFF the menu is not something to announce.
#[test]
fn a_dish_disappearing_is_not_a_post() {
    let previous = vec![("a".to_string(), true)];
    let current = vec![("a".to_string(), "Sake".to_string(), false)];
    assert!(derive_subjects(&previous, &current, None, false, true, &|_| false).is_empty());
}

#[test]
fn one_order_is_not_a_trend() {
    let prev = vec![("a".to_string(), true)];
    let cur = vec![("a".to_string(), "Sake".to_string(), true)];
    let few = derive_subjects(&prev, &cur, Some(("a".into(), 2)), false, true, &|_| false);
    assert!(few.is_empty(), "two orders is noise: {few:?}");
    let many = derive_subjects(&prev, &cur, Some(("a".into(), 7)), false, true, &|_| false);
    assert_eq!(many, vec![Subject::MostOrdered { product: "Sake".into(), orders: 7 }]);
}

#[test]
fn reopening_is_noticed_only_on_the_transition() {
    let p = vec![("a".to_string(), true)];
    let c = vec![("a".to_string(), "Sake".to_string(), true)];
    assert!(derive_subjects(&p, &c, None, true, true, &|_| false).contains(&Subject::Reopened));
    assert!(!derive_subjects(&p, &c, None, false, true, &|_| false).contains(&Subject::Reopened));
    assert!(!derive_subjects(&p, &c, None, true, false, &|_| false).contains(&Subject::Reopened));
}

/// Anything already put to the owner is filtered out, whatever they said.
#[test]
fn subjects_already_seen_are_dropped() {
    let previous = vec![("a".to_string(), false)];
    let current = vec![("a".to_string(), "Sake".to_string(), true)];
    let seen = |k: &str| k == "back:Sake";
    assert!(derive_subjects(&previous, &current, None, false, true, &seen).is_empty());
}

#[test]
fn a_hostile_draft_cannot_forge_a_record() {
    let mut p = Posts::create().expect("create");
    let mut evil = post("p1", "k", State::Draft, 1);
    evil.text = r#"x","state":"published"#.into();
    p.put(&evil);
    assert_eq!(p.get("p1").unwrap().state, State::Draft, "state must not move");
}
