//! P3 (W-PERF card): the object's five stock-writing turns outside placement
//! and advance -- amend, transfer, refund, returned food, the till import --
//! read the stock log CLOCKED, so every record they write carries `at` and a
//! report dates it by itself rather than guessing it from an order.
//!
//! A source check, because the five sites are async turns no native test can
//! stand up: the clock is an argument each one must pass, and a new turn that
//! reads the unclocked log is the regression.

const ROOM: &str = include_str!("../room.rs");
const REFUND: &str = include_str!("../refund.rs");
const EBILLS: &str = include_str!("../ebills.rs");
const TURN: &str = include_str!("../stock_turn.rs");

fn calls(src: &str, pat: &str) -> usize {
    src.lines().filter(|l| !l.trim_start().starts_with("//")).filter(|l| l.contains(pat)).count()
}

#[test]
fn every_stock_writing_turn_reads_the_log_clocked() {
    let clocked = "self.stock_log_at(input.now_ms).await?";
    assert_eq!(calls(ROOM, clocked), 2, "amend and transfer");
    assert_eq!(calls(REFUND, clocked), 2, "refund and the returned-food choice");
    assert_eq!(calls(EBILLS, clocked), 1, "the till import");
    // The unclocked read survives in three places only: inside `stock_log_at`
    // itself, the stock movement (whose `turn::run` sets the clock), and the
    // owner's ingredients reset, which only counts the old log and writes no
    // record into it -- it replaces the image with an empty one.
    let bare = "self.stock_log().await?";
    assert_eq!((calls(ROOM, bare), calls(REFUND, bare), calls(EBILLS, bare)), (1, 0, 0));
    assert_eq!(calls(TURN, bare), 2);
    let run = include_str!("../../services/operations/stock/turn.rs");
    assert_eq!(calls(run, "log.set_clock(input.now_ms);"), 1, "turn::run sets the clock");
}

/// What the clock buys, through the real log: a record written after
/// `set_clock` carries `at`; one written without it does not.
#[test]
fn a_clocked_log_dates_what_it_writes() {
    use dowiz_hub::stock::{StockEvent, StockLog};
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.append(&StockEvent::Received { item: "rice".into(), qty: 1 }).unwrap();
    s.set_clock(1_790_000_000_000);
    s.append(&StockEvent::Received { item: "rice".into(), qty: 1 }).unwrap();
    let ats: Vec<Option<i64>> = s.journal().unwrap().entries.iter().map(|e| e.meta.at).collect();
    assert_eq!(ats, vec![None, Some(1_790_000_000_000)]);
}
