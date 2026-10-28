//! The verbatim doors: the hub's decides each record and holds the refused ones by position; the
//! tablet's refuses a replay that does not end at the tip it was promised.

use super::*;

fn line(ev: &StockEvent) -> String {
    encode(ev)
}

#[test]
fn the_hub_adopts_what_its_ledger_takes_and_holds_the_rest_by_position() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 5 }).unwrap();
    let recs = vec![
        line(&StockEvent::Reserved { item: "rice".into(), qty: 9, order_id: "o".into() }),
        line(&StockEvent::Served { item: "rice".into(), qty: 2, order_id: "o".into() }),
        r#"{"k":"note","note":"x"}"#.to_string(),
    ];
    let start = log.ledger_since().unwrap();
    let (a, led) = log.adopt_from(&recs, start).unwrap();
    assert_eq!(led, log.ledger().unwrap(), "the ledger handed back is the one a fold of the log gives");
    assert_eq!(a.merged, 1);
    assert_eq!(a.held.iter().map(|h| h.0).collect::<Vec<_>>(), vec![0, 2], "9 reserved of 5 counted, and a note");
    let last = log.since(&[0u8; 32]).unwrap();
    assert_eq!(last.last().unwrap(), &recs[1], "written as received, byte for byte");
}

#[test]
fn since_answers_none_for_a_base_this_chain_never_held() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 5 }).unwrap();
    assert!(log.since(&[9u8; 32]).is_none());
    assert_eq!(log.since(&log.tip_id()).unwrap().len(), 0, "the twin: the tip has nothing above it");
}

#[test]
fn a_replay_that_misses_the_tip_is_refused() {
    let mut a = StockLog::create_sized(64 * 1024).unwrap();
    a.append(&StockEvent::Received { item: "rice".into(), qty: 5 }).unwrap();
    let recs = a.since(&[0u8; 32]).unwrap();
    let refs: Vec<&str> = recs.iter().map(String::as_str).collect();
    let mut b = StockLog::create_sized(64 * 1024).unwrap();
    assert!(b.replay_raw(&refs, &[1u8; 32]).is_err());
    let mut c = StockLog::create_sized(64 * 1024).unwrap();
    assert!(c.replay_raw(&refs, &a.tip_id()).is_ok(), "the twin lands on the same tip");
}

#[test]
fn a_glued_tail_defers_its_checkpoint_but_never_past_twice_the_cadence() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_checkpoint_every(10);
    log.append(&StockEvent::Received { item: "rice".into(), qty: 500 }).unwrap();
    let served = |n: usize, tag: &str| -> Vec<String> {
        (0..n).map(|i| line(&StockEvent::Served { item: "rice".into(), qty: 1, order_id: format!("{tag}-{i}") })).collect()
    };
    let start = log.ledger_since().unwrap();
    log.adopt_from(&served(15, "a"), start).unwrap();
    assert_eq!(log.verify_checkpoints().unwrap(), 0, "16 records after none: past the cadence, under twice it -- deferred");
    let start = log.ledger_since().unwrap();
    assert_eq!(start.1, 16);
    log.adopt_from(&served(5, "b"), start).unwrap();
    assert_eq!(log.verify_checkpoints().unwrap(), 1, "21 >= 2 x 10: the glue writes it now");
    let (_, since) = log.ledger_since().unwrap();
    assert_eq!(since, 0, "and the fold starts from it");
}

#[test]
fn the_night_writes_a_due_checkpoint_and_nothing_else() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_checkpoint_every(usize::MAX);
    for i in 0..12 {
        log.append(&StockEvent::Received { item: format!("i{i}"), qty: 1 }).unwrap();
    }
    log.set_checkpoint_every(20);
    let n = log.len();
    assert!(!log.checkpoint_if_due().unwrap(), "12 since none, cadence 20: not due");
    assert_eq!(log.len(), n, "and nothing written");
    log.set_checkpoint_every(10);
    assert!(log.checkpoint_if_due().unwrap(), "12 >= 10: due");
    assert_eq!((log.len(), log.verify_checkpoints().unwrap()), (n + 1, 1));
    assert!(!log.checkpoint_if_due().unwrap(), "just written: not due again");
}
