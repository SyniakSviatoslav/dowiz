//! Linking by code, and the service messages that keep state (T3).
use super::*;
use crate::notify::route::groups::{Mode, Pii};
use serde_json::json;

const NOW: i64 = 1_000_000;

fn pending() -> Pending {
    mint([0, 1, 2, 3, 30, 31, 32, 255], "owner-1", NOW)
}

fn link_msg(text: &str, chat: i64, topic: Option<i64>) -> Value {
    let mut m = json!({ "message_id": 5, "from": { "id": 777, "first_name": "Ana" },
        "chat": { "id": chat, "title": "Dubin Kitchen", "type": "supergroup" }, "text": text });
    if let Some(t) = topic {
        m["is_topic_message"] = json!(true);
        m["message_thread_id"] = json!(t);
    }
    json!({ "update_id": 1, "message": m })
}

#[test]
fn a_code_is_eight_unambiguous_characters_and_lives_ten_minutes() {
    let p = pending();
    assert_eq!(p.code, "ABCD89A9");
    assert!(p.code.chars().all(|c| !"01OI".contains(c)));
    assert_eq!(p.exp_ms, NOW + TTL_MS);
    assert!(matches(&p, "abcd-89a9", NOW + TTL_MS), "typed with a dash, lower case: still the code");
    assert!(!matches(&p, "ABCD89A9", NOW + TTL_MS + 1), "expired");
    assert!(!matches(&p, "ABCD89A", NOW), "short");
    assert!(!matches(&p, "ABCD89AJ", NOW), "wrong");
}

#[test]
fn link_and_start_commands_are_read_with_the_topic_and_the_linker() {
    let u = parse(&link_msg("/link@DubinBot ABCD-89A9", -100123, Some(42)), "dubinbot");
    assert_eq!(u, Update::Link { chat: "-100123".into(), thread: Some(42), title: "Dubin Kitchen".into(),
        kind: "supergroup".into(), code: "ABCD-89A9".into(), from: 777 });
    assert!(matches!(parse(&link_msg("/start ABCD89A9", -1, None), "dubinbot"), Update::Link { thread: None, .. }));
}

#[test]
fn a_command_for_another_bot_or_anything_else_is_ignored() {
    assert_eq!(parse(&link_msg("/link@OtherBot ABCD89A9", -1, None), "dubinbot"), Update::Ignore);
    assert_eq!(parse(&link_msg("/link", -1, None), "dubinbot"), Update::Ignore);
    assert_eq!(parse(&link_msg("hello there", -1, None), "dubinbot"), Update::Ignore);
    assert_eq!(parse(&json!({ "update_id": 3 }), "dubinbot"), Update::Ignore);
}

#[test]
fn a_migration_is_read_from_either_service_message() {
    let old = json!({ "message": { "chat": { "id": -55 }, "migrate_to_chat_id": -100999 } });
    let new = json!({ "message": { "chat": { "id": -100999 }, "migrate_from_chat_id": -55 } });
    let want = Update::Migrated { from: "-55".into(), to: "-100999".into() };
    assert_eq!(parse(&old, "b"), want);
    assert_eq!(parse(&new, "b"), want);
}

#[test]
fn a_member_update_is_read() {
    let u = json!({ "my_chat_member": { "chat": { "id": -7 }, "new_chat_member": { "status": "kicked" } } });
    assert_eq!(parse(&u, "b"), Update::Member { chat: "-7".into(), status: "kicked".into() });
}

#[test]
fn the_right_code_links_the_group_once_with_no_customer_data() {
    let mut list = Vec::new();
    let p = pending();
    let u = parse(&link_msg("/link ABCD89A9", -100123, Some(42)), "b");
    let e = apply(&mut list, Some(&p), &u, "sq", "Dubin", NOW);
    assert!(e.changed && e.consumed);
    assert_eq!(e.reply, Some(("-100123:42".into(), "✅ U lidh me Dubin".into())));
    let g = &list[0];
    assert_eq!((g.id.as_str(), g.chat.as_str(), g.thread, g.lang.as_str(), g.pii), ("dubin-kitchen", "-100123", Some(42), "sq", Pii::None));
    assert_eq!(g.linked, Some(Linked { at_ms: NOW, by: "owner-1".into(), tg_user: 777 }));
    assert_eq!(g.mode("order.placed"), Mode::Now);
}

#[test]
fn a_wrong_expired_or_absent_code_links_nothing_and_says_so() {
    let u = parse(&link_msg("/link ZZZZZZZZ", -1, None), "b");
    for (p, now) in [(Some(pending()), NOW), (None, NOW)] {
        let mut list = Vec::new();
        let e = apply(&mut list, p.as_ref(), &u, "en", "Dubin", now);
        assert!(!e.changed && !e.consumed && list.is_empty());
        assert!(e.reply.unwrap().1.starts_with("✗ This code"));
    }
    let good = parse(&link_msg("/link ABCD89A9", -1, None), "b");
    let mut list = Vec::new();
    assert!(!apply(&mut list, Some(&pending()), &good, "en", "D", NOW + TTL_MS + 1).changed, "expired");
}

#[test]
fn relinking_a_group_the_bot_left_revives_it_and_a_full_venue_refuses_a_new_one() {
    let p = pending();
    let u = parse(&link_msg("/link ABCD89A9", -1, None), "b");
    let mut list = Vec::new();
    apply(&mut list, Some(&p), &u, "en", "D", NOW);
    list[0].state = State::Left;
    let e = apply(&mut list, Some(&p), &u, "en", "D", NOW);
    assert!(e.changed && list.len() == 1 && list[0].state == State::Active);

    let mut full: Vec<Group> = (0..groups::MAX_GROUPS as i64)
        .map(|i| Group::fresh(format!("g{i}"), format!("-{i}0"), None, String::new(), String::new(), "en"))
        .collect();
    let e = apply(&mut full, Some(&p), &parse(&link_msg("/link ABCD89A9", -999, None), "b"), "en", "D", NOW);
    assert!(!e.consumed && full.len() == groups::MAX_GROUPS);
}

#[test]
fn a_kick_marks_the_group_left_a_promotion_changes_nothing_a_migration_moves_it() {
    let mut list = vec![Group::fresh("k".into(), "-55".into(), None, String::new(), String::new(), "en")];
    let promo = Update::Member { chat: "-55".into(), status: "administrator".into() };
    assert_eq!(apply(&mut list, None, &promo, "en", "D", NOW), Effect::default());
    let mv = Update::Migrated { from: "-55".into(), to: "-100999".into() };
    assert!(apply(&mut list, None, &mv, "en", "D", NOW).changed);
    assert_eq!(list[0].chat, "-100999");
    let kick = Update::Member { chat: "-100999".into(), status: "kicked".into() };
    assert!(apply(&mut list, None, &kick, "en", "D", NOW).changed);
    assert_eq!(list[0].state, State::Left);
    assert!(!apply(&mut list, None, &kick, "en", "D", NOW).changed, "once");
}
