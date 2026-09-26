//! Row 1: the venue finds every thread, newest first, with the kernel's own
//! unread number; a thread that will not replay is shown with its error.
use super::*;

fn row(id: &str, from: &str, seq: i64, kind: &str, body: &str, read: Option<i64>, at: i64) -> MessageRow {
    MessageRow { id: id.into(), from_party: from.into(), seq, kind: kind.into(), body: body.into(), read_through: read, sent_at_ms: at }
}

#[test]
fn every_thread_is_listed_newest_first_with_what_the_venue_has_not_read() {
    let rows = vec![
        ("o1".to_string(), row("1", "CUSTOMER", 1, "TEXT", "no wasabi please", None, 100)),
        ("o1".to_string(), row("2", "CUSTOMER", 2, "TEXT", "and extra ginger", None, 110)),
        ("o2".to_string(), row("3", "CUSTOMER", 1, "TEXT", "where is it?", None, 500)),
        ("o2".to_string(), row("4", "VENUE", 1, "READ", "", Some(1), 510)),
        ("o2".to_string(), row("5", "VENUE", 2, "TEXT", "on its way", None, 520)),
    ];
    let s = summaries(rows);
    assert_eq!(s.iter().map(|t| t["id"].as_str().unwrap()).collect::<Vec<_>>(), vec!["o2", "o1"], "newest activity first");
    assert_eq!(s[0]["unread"], 0, "the venue read through 1 and wrote the last");
    assert_eq!(s[0]["last"]["body"], "on its way");
    assert_eq!(s[0]["count"], 2, "a read mark is not a message");
    assert_eq!(s[1]["unread"], 2);
    assert_eq!(s[1]["last"]["from"], "CUSTOMER");
}

#[test]
fn a_thread_the_kernel_will_not_replay_is_listed_with_its_error_not_dropped() {
    let rows = vec![("bad".to_string(), row("1", "MARTIAN", 1, "TEXT", "hi", None, 1))];
    let s = summaries(rows);
    assert_eq!(s.len(), 1);
    assert!(s[0]["error"].as_str().unwrap().contains("unknown party"));
    // ITS TWIN: a well-formed one has no error.
    let ok = summaries(vec![("o".to_string(), row("1", "CUSTOMER", 1, "TEXT", "hi", None, 1))]);
    assert!(ok[0].get("error").is_none());
    assert!(summaries(Vec::new()).is_empty());
}
