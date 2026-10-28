//! The decision list: open until a signed `decided` note names it.

use super::*;
use crate::stock::StockLog;

fn with(notes: &[&str]) -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for n in notes {
        log.append_note(DECIDE, n).unwrap();
    }
    log
}

#[test]
fn a_decision_needs_a_name_to_close() {
    let mut log = with(&[r#"{"did":"a-1","ctx":"kitchen","kind":"oversold","item":"salmon","qty":-1}"#]);
    assert!(matches!(close(&mut log, "a-1", " "), Err(StockError::Unsigned)));
    assert_eq!(open(&log).len(), 1, "a refused close leaves it open");
    assert!(close(&mut log, "a-1", "chef").unwrap(), "the twin");
    assert!(open(&log).is_empty());
}

#[test]
fn only_the_named_decision_closes() {
    let mut log = with(&[
        r#"{"did":"a-1","ctx":"kitchen","kind":"recount","item":"rice","side":"hub"}"#,
        r#"{"did":"a-2","ctx":"kitchen","kind":"held","why":"x","rec":"{\"k\":\"consumed\"}"}"#,
        r#"{"ctx":"kitchen","kind":"held"}"#,
    ]);
    assert_eq!(open(&log).iter().map(|d| d.did.as_str()).collect::<Vec<_>>(), vec!["a-1", "a-2"], "a note without an id is not a decision");
    assert_eq!(open(&log)[1].rec, r#"{"k":"consumed"}"#, "the held record reads back unescaped");
    close(&mut log, "a-2", "chef").unwrap();
    assert_eq!(open(&log).iter().map(|d| d.did.as_str()).collect::<Vec<_>>(), vec!["a-1"]);
}
