//! Telegram's answers, classified (T1): each refusal has a positive twin.
use super::*;
use crate::outbox::Entry;

fn entry(to: &str, tries: u32) -> Entry {
    let mut e = Entry::new("o1/telegram".into(), "telegram", to.into(), "hi".into(), 1_000);
    e.tries = tries;
    e
}

#[test]
fn a_429_waits_retry_after_and_does_not_count_a_try() {
    let f = classify(429, r#"{"ok":false,"error_code":429,"description":"Too Many Requests: retry after 7","parameters":{"retry_after":7}}"#);
    assert_eq!(f, Fail::RetryAfter(7));
    assert_eq!(outcome(&entry("-100", 2), &Err(f), 50_000), Outcome::Retry { tries: 2, next_at_ms: 57_000 });
}

#[test]
fn a_429_without_a_number_still_waits_rather_than_hammering() {
    assert_eq!(classify(429, "not json"), Fail::RetryAfter(5));
}

#[test]
fn an_ordinary_failure_counts_a_try_on_the_backoff() {
    let f = classify(500, r#"{"ok":false,"description":"Internal Server Error"}"#);
    assert_eq!(f, Fail::Other("Internal Server Error".into()));
    assert_eq!(outcome(&entry("-100", 0), &Err(f.clone()), 0), Outcome::Retry { tries: 1, next_at_ms: 10_000 });
    assert_eq!(outcome(&entry("-100", 5), &Err(f), 0), Outcome::Abandon { after: 6 });
}

#[test]
fn a_403_is_gone_for_the_whole_chat_and_names_it_without_the_thread() {
    let f = classify(403, r#"{"ok":false,"error_code":403,"description":"Forbidden: bot was kicked from the supergroup chat"}"#);
    assert!(matches!(f, Fail::Gone(ref d) if d.contains("kicked")));
    assert_eq!(
        outcome(&entry("-1001:42", 0), &Err(f), 0),
        Outcome::Gone { chat: "-1001".into(), why: "Forbidden: bot was kicked from the supergroup chat".into() }
    );
}

#[test]
fn chat_not_found_and_a_deleted_topic_are_gone_but_a_bad_request_is_not() {
    assert!(matches!(classify(400, r#"{"description":"Bad Request: chat not found"}"#), Fail::Gone(_)));
    assert!(matches!(classify(400, r#"{"description":"Bad Request: message thread not found"}"#), Fail::Gone(_)));
    assert!(matches!(classify(400, r#"{"description":"Bad Request: message text is empty"}"#), Fail::Other(_)));
}

#[test]
fn a_migrated_group_rewrites_the_chat_and_keeps_the_topic() {
    let f = classify(400, r#"{"ok":false,"description":"Bad Request: group chat was upgraded to a supergroup chat","parameters":{"migrate_to_chat_id":-1009876}}"#);
    assert_eq!(f, Fail::Migrated(-1009876));
    assert_eq!(outcome(&entry("-55:3", 1), &Err(f.clone()), 0), Outcome::Moved { to: "-1009876:3".into() });
    assert_eq!(outcome(&entry("-55", 1), &Err(f), 0), Outcome::Moved { to: "-1009876".into() });
}

#[test]
fn a_sent_message_is_sent() {
    assert_eq!(outcome(&entry("-1", 3), &Ok(()), 0), Outcome::Sent);
}

#[test]
fn a_target_is_chat_or_chat_colon_thread_and_legacy_ids_have_no_thread() {
    assert_eq!(target_of("-1001234567890:42"), Target { chat: "-1001234567890".into(), thread: Some(42) });
    assert_eq!(target_of(" 12345 "), Target { chat: "12345".into(), thread: None });
    assert_eq!(target_of("@dubin"), Target { chat: "@dubin".into(), thread: None });
    assert_eq!(target_of(":7"), Target { chat: ":7".into(), thread: None });
    assert_eq!(target_text("-1", Some(9)), "-1:9");
    assert_eq!(target_text("-1", None), "-1");
}

#[test]
fn the_body_carries_the_topic_only_when_there_is_one_and_cuts_the_text() {
    let b = send_body(&target_of("-1:42"), "x");
    assert_eq!(b["message_thread_id"], 42);
    assert_eq!(b["chat_id"], "-1");
    let long = "y".repeat(TEXT_MAX + 10);
    let b = send_body(&target_of("-1"), &long);
    assert!(b.get("message_thread_id").is_none());
    assert_eq!(b["text"].as_str().unwrap().chars().count(), TEXT_MAX);
}

#[test]
fn a_fail_speaks_telegrams_words() {
    assert!(Fail::RetryAfter(3).words().contains("retry after 3"));
    assert!(Fail::Migrated(-9).words().contains("-9"));
    assert_eq!(Fail::Gone("x".into()).words(), "x");
}
