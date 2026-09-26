//! The webhook's pure edges: the secret, the venue's language, the stored groups.
use super::*;

#[test]
fn the_secret_must_match_exactly_and_an_empty_one_never_does() {
    assert!(secret_ok("abc123", "abc123"));
    assert!(secret_ok(" abc123 ", "abc123"));
    assert!(!secret_ok("abc123", "abc124"));
    assert!(!secret_ok("abc123", "abc12"));
    assert!(!secret_ok("", ""), "a venue with no secret accepts nothing");
}

#[test]
fn a_new_group_speaks_the_venues_language() {
    assert_eq!(venue_lang(Some(&serde_json::json!({ "default_locale": "sq" }))), "sq");
    assert_eq!(venue_lang(None), "en");
}

#[test]
fn a_legacy_venue_becomes_stored_groups_on_its_first_change() {
    let mut s = dowiz_hub::settings::Settings::create().unwrap();
    s.set(groups::LEGACY_CHAT, "123");
    let list = editable(&s, "sq").unwrap();
    assert_eq!(list.len(), 1);
    assert!(s.get(groups::KEY_GROUPS).is_none(), "reading writes nothing");
    store(&mut s, &list);
    let back = crate::notify::route::groups_of(&s, "sq").unwrap();
    assert!(!back.legacy);
    assert_eq!(back.list, list);
    s.set(groups::KEY_GROUPS, "{broken");
    assert!(editable(&s, "sq").is_err(), "a broken value is refused, never overwritten with nothing");
}

#[test]
fn the_bot_record_reads_back_and_absent_is_empty() {
    let mut s = dowiz_hub::settings::Settings::create().unwrap();
    assert_eq!(bot_of(&s), Bot::default());
    s.set(groups::KEY_BOT, r#"{"username":"dubinbot","hook_ms":5}"#);
    assert_eq!(bot_of(&s), Bot { username: "dubinbot".into(), hook_ms: 5 });
    assert!(dowiz_hub::settings::is_secret(groups::KEY_SECRET), "the webhook secret is redacted everywhere");
}
