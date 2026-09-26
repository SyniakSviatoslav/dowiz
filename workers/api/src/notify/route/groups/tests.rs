//! The groups model (T2): legacy keys read as groups, owner patches refused
//! rather than guessed.
use super::*;

fn two() -> Vec<Group> {
    vec![
        Group::fresh("kitchen".into(), "-1001".into(), Some(42), "Kitchen".into(), "supergroup".into(), "sq"),
        Group::fresh("owners".into(), "-2002".into(), None, "Owners".into(), "group".into(), "uk"),
    ]
}

#[test]
fn a_venue_that_never_wrote_groups_reads_its_two_old_keys_as_groups() {
    let g = load(None, " 123 ", "", "sq").unwrap();
    assert!(g.legacy);
    assert_eq!(g.list.len(), 1);
    let main = &g.list[0];
    assert_eq!((main.id.as_str(), main.chat.as_str(), main.pii, main.station.as_deref()), ("main", "123", Pii::Full, None));
    assert_eq!(main.mode("order.placed"), Mode::Now);
    assert_eq!(main.mode("order.exception"), Mode::Now);
    assert_eq!(main.mode("stock.low"), Mode::Off, "a legacy venue gets nothing it did not get before");

    let g = load(Some("  "), "123", "456", "en").unwrap();
    assert_eq!(g.list.iter().map(|x| (x.id.as_str(), x.station.as_deref())).collect::<Vec<_>>(), vec![("main", Some("kitchen")), ("bar", Some("bar"))]);
    assert!(load(None, "", "", "en").unwrap().list.is_empty());
}

#[test]
fn stored_groups_win_and_an_unreadable_value_is_an_error_not_silence() {
    let json = serde_json::to_string(&two()).unwrap();
    let g = load(Some(&json), "123", "", "en").unwrap();
    assert!(!g.legacy);
    assert_eq!(g.list, two());
    assert!(load(Some("[{"), "123", "", "en").unwrap_err().contains("unreadable"));
    // `[]` is a real answer: the owner unlinked everything.
    assert!(load(Some("[]"), "123", "", "en").unwrap().list.is_empty());
}

#[test]
fn a_new_group_carries_no_customer_and_a_topic_is_its_target() {
    let g = &two()[0];
    assert_eq!(g.pii, Pii::None);
    assert_eq!(g.target(), "-1001:42");
    assert_eq!(two()[1].target(), "-2002");
    let wire = serde_json::to_value(g).unwrap();
    assert_eq!(wire["pii"], "none");
    assert_eq!(wire["subs"]["order.placed"], "now");
}

#[test]
fn a_group_that_is_not_active_gets_nothing() {
    let mut g = two().remove(0);
    assert_eq!(g.mode("order.placed"), Mode::Now);
    g.state = State::Muted;
    assert_eq!(g.mode("order.placed"), Mode::Off);
    g.state = State::Left;
    assert_eq!(g.mode("order.placed"), Mode::Off);
}

fn patch(json: &str) -> Patch {
    serde_json::from_str(json).unwrap()
}

#[test]
fn an_owner_patch_changes_what_it_names_and_nothing_else() {
    let mut l = two();
    apply(&mut l, "kitchen", &patch(r#"{"lang":"en","pii":"fulfil","subs":{"stock.low":"digest","order.placed":"off"},"quiet":{"from":1380,"to":420},"digest_at":600}"#)).unwrap();
    let g = &l[0];
    assert_eq!((g.lang.as_str(), g.pii, g.digest_at), ("en", Pii::Fulfil, 600));
    assert_eq!(g.mode("stock.low"), Mode::Digest);
    assert_eq!(g.mode("order.placed"), Mode::Off);
    assert_eq!(g.mode("order.amended"), Mode::Now, "untouched");
    assert_eq!(g.quiet, Some(Window { from: 1380, to: 420 }));
    assert_eq!(l[1], two()[1]);
    apply(&mut l, "kitchen", &patch(r#"{"quiet":null}"#)).unwrap();
    assert_eq!(l[0].quiet, None);
    apply(&mut l, "kitchen", &patch(r#"{"lang":"ru"}"#)).unwrap();
    assert_eq!(l[0].lang, "ru", "a fourth language is a row in the words table, not a code change");
}

#[test]
fn an_owner_patch_is_refused_rather_than_guessed() {
    let mut l = two();
    assert!(apply(&mut l, "nope", &patch("{}")).unwrap_err().contains("no group"));
    assert!(apply(&mut l, "kitchen", &patch(r#"{"lang":"xx"}"#)).unwrap_err().contains("language"));
    assert!(apply(&mut l, "kitchen", &patch(r#"{"subs":{"order.teleported":"now"}}"#)).unwrap_err().contains("no event"));
    assert!(apply(&mut l, "kitchen", &patch(r#"{"subs":{"digest.daily":"digest"}}"#)).unwrap_err().contains("summary"));
    assert!(apply(&mut l, "kitchen", &patch(r#"{"quiet":{"from":60,"to":60}}"#)).is_err());
    assert!(apply(&mut l, "kitchen", &patch(r#"{"quiet":{"from":60,"to":1440}}"#)).is_err());
    assert!(apply(&mut l, "kitchen", &patch(r#"{"digest_at":-1}"#)).is_err());
    assert!(serde_json::from_str::<Patch>(r#"{"chat":"-9"}"#).is_err(), "the chat is set by linking, never typed");
    assert_eq!(l, two(), "nothing refused was half-applied");
}

#[test]
fn muting_pauses_and_resumes_but_never_revives_a_group_the_bot_left() {
    let mut l = two();
    apply(&mut l, "kitchen", &patch(r#"{"muted":true}"#)).unwrap();
    assert_eq!(l[0].state, State::Muted);
    apply(&mut l, "kitchen", &patch(r#"{"muted":false}"#)).unwrap();
    assert_eq!(l[0].state, State::Active);
    l[0].state = State::Left;
    apply(&mut l, "kitchen", &patch(r#"{"muted":false}"#)).unwrap();
    assert_eq!(l[0].state, State::Left);
}

#[test]
fn a_slug_is_unique_and_never_empty() {
    let l = two();
    assert_eq!(slug("Kitchen", &l), "kitchen-2");
    assert_eq!(slug("Dubin Bar!", &l), "dubin-bar");
    assert_eq!(slug("Кухня", &l), "group");
}

#[test]
fn a_migration_moves_every_group_on_the_chat_and_left_marks_them() {
    let mut l = two();
    l.push(Group::fresh("kitchen-bar".into(), "-1001".into(), Some(7), "Bar".into(), "supergroup".into(), "sq"));
    assert!(migrate(&mut l, "-1001", "-100999"));
    assert!(!migrate(&mut l, "-1001", "-100999"));
    assert_eq!(l.iter().filter(|g| g.chat == "-100999").count(), 2);
    assert_eq!(left(&mut l, "-100999"), vec!["kitchen".to_string(), "kitchen-bar".to_string()]);
    assert!(left(&mut l, "-100999").is_empty(), "once");
    assert_eq!(l[1].state, State::Active);
}
