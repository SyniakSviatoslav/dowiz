//! SHEAF S1 pass marks (R-BEBOPDB G.5): every reader takes a grant and a tablet's sale before any
//! writer emits one; an old log folds byte-equal; a day with two offline tablets replays with 0
//! sales beyond on-hand that are not flagged provisional; glue reports the provisional sales.
//! Fixed data, every sale named (memory `no-fuzzers-in-this-repo`).

use super::super::meta::Meta;
use super::super::oldimage_tests::{fold_text, old_history};
use super::super::{decode, StockError, StockEvent, StockLog};
use super::{provisional_of, Rights, HUB, NOTE};
use crate::glue::{glue, Conflict, Side, State};

/// 08:00 on the replayed day; a tablet sale every ten minutes.
const T0: i64 = 1_760_000_000_000;
const MIN: i64 = 60_000;
/// The grants run to 23:00.
const UNTIL: i64 = T0 + 15 * 60 * MIN;

fn received(item: &str, qty: i64) -> StockEvent {
    StockEvent::Received { item: item.into(), qty }
}

fn served(item: &str, qty: i64, o: &str) -> StockEvent {
    StockEvent::Served { item: item.into(), qty, order_id: o.into() }
}

fn shelf() -> StockLog {
    let mut l = StockLog::create_sized(64 * 1024).unwrap();
    l.set_clock(T0);
    l.append_all(&[received("salmon", 2000), received("rice", 6000), received("avocado", 30)]).unwrap();
    l
}

#[test]
fn every_reader_walks_past_a_grant_and_reads_a_tablet_sale_as_the_plain_sale() {
    let mut with = shelf();
    let mut plain = shelf();
    with.grant("kitchen", "salmon", 300, Some(UNTIL)).unwrap();
    assert!(!with.sell("kitchen", "salmon", 200, "o1").unwrap(), "inside the rights");
    assert!(with.sell("kitchen", "salmon", 200, "o2").unwrap(), "past the rights");
    plain.append(&served("salmon", 200, "o1")).unwrap();
    plain.append(&served("salmon", 200, "o2")).unwrap();
    let raw = with.raw();
    let note = raw.iter().find(|r| super::super::notes::is_note(r, NOTE)).expect("the grant is in the chain");
    assert_eq!(decode(note), None, "a grant is not an event: {note}");
    let sale = raw.last().unwrap();
    assert!(sale.contains(r#""prov":1"#) && sale.contains(r#""ctx":"kitchen""#), "{sale}");
    assert_eq!(decode(sale), Some(served("salmon", 200, "o2")), "a tablet sale decodes as the plain one");
    assert_eq!(with.ledger().unwrap(), plain.ledger().unwrap(), "the shelf");
    assert_eq!(with.events(), plain.events(), "the events");
    // The journal's rows are the same, seq included; only the two new meta keys tell them apart.
    let rows = |l: &StockLog| -> Vec<_> {
        let strip = |m: &Meta| Meta { ctx: None, prov: None, ..m.clone() };
        l.journal().unwrap().entries.iter().map(|e| (e.seq, e.ev.clone(), strip(&e.meta), e.before, e.value)).collect()
    };
    assert_eq!(rows(&with), rows(&plain), "the journal");
    assert_eq!(with.cost_book(), plain.cost_book(), "the cost book");
    assert_eq!(with.len(), plain.len() + 1, "but the grant IS in the chain");
    let back = StockLog::load(&with.to_bytes_trimmed()).unwrap();
    assert_eq!(back.quarantined(), 0, "nothing quarantined");
    assert_eq!(back.ledger().unwrap(), plain.ledger().unwrap());
    assert_eq!(back.rights(), with.rights(), "the budgets survive a reload");
    let mut ck = StockLog::load(&with.to_bytes_trimmed()).unwrap();
    ck.checkpoint_now().unwrap();
    assert_eq!(ck.verify_checkpoints(), Ok(1), "a checkpoint over a grant agrees with the fold from genesis");
}

#[test]
fn an_old_log_without_rights_folds_byte_equal_and_grants_move_nothing() {
    let old = old_history();
    assert_eq!(old.rights(), Rights::default(), "an old log holds no rights");
    assert!(old.provisional_sales().is_empty());
    let before = fold_text(&old.journal().unwrap());
    let mut with = StockLog::load(&old.to_bytes_trimmed()).unwrap();
    with.grant("kitchen", "salmon", 1, None).unwrap();
    with.grant("waiter-1", "rice", 1, None).unwrap();
    assert_eq!(fold_text(&with.journal().unwrap()), before, "two grants later, the fold reads the same");
    assert_eq!(with.quarantined(), 0);
}

#[test]
fn the_grant_door_keeps_every_share_inside_what_is_available() {
    let mut l = shelf();
    l.grant("kitchen", "salmon", 1200, None).unwrap();
    let e = l.grant("waiter", "salmon", 801, None).unwrap_err();
    assert!(matches!(e, StockError::OutOfStock { available: 800, .. }), "{e:?}");
    l.grant("waiter", "salmon", 800, None).unwrap();
    assert_eq!(l.rights().may_sell(HUB, "salmon", &l.ledger().unwrap(), None), 0, "the hub keeps nothing");
    l.grant("kitchen", "salmon", 1000, None).unwrap();
    assert_eq!(l.rights().budget("kitchen", "salmon").unwrap().granted, 1000, "a new grant replaces the old");
    assert!(matches!(l.grant(HUB, "salmon", 1, None), Err(StockError::Malformed)), "the hub grants itself nothing");
    assert!(matches!(l.grant(" ", "salmon", 1, None), Err(StockError::Malformed)));
    assert!(matches!(l.grant("kitchen", "salmon", -1, None), Err(StockError::NotPositive { .. })));
    let n = l.len();
    assert!(l.grant("bar", "salmon", 201, None).is_err());
    assert_eq!(l.len(), n, "a refused grant writes nothing");
}

#[test]
fn a_sale_past_its_rights_is_recorded_provisional_and_never_refused() {
    let mut t = shelf();
    t.grant("kitchen", "salmon", 100, Some(UNTIL)).unwrap();
    assert!(!t.sell("kitchen", "salmon", 60, "a").unwrap(), "inside: 60 of 100");
    assert!(!t.sell("kitchen", "salmon", 40, "b").unwrap(), "inside: exactly the rest");
    assert!(t.sell("kitchen", "salmon", 1, "c").unwrap(), "past: provisional");
    assert!(t.sell("waiter", "salmon", 5, "d").unwrap(), "no grant at all: provisional");
    assert!(!t.sell("kitchen", "nori", 5, "e").unwrap(), "an uncounted shelf promises nothing, refuses nothing");
    t.grant("kitchen", "rice", 100, Some(UNTIL)).unwrap();
    t.set_clock(UNTIL + 1);
    assert!(t.sell("kitchen", "rice", 10, "f").unwrap(), "past the grant's last instant: provisional");
    let p = t.provisional_sales();
    assert_eq!(p.iter().map(|p| p.order_id.as_str()).collect::<Vec<_>>(), ["c", "d", "f"]);
    assert_eq!(t.ledger().unwrap().level("salmon").on_hand, 2000 - 106, "every sale took its stock");
    assert_eq!(t.rights().budget("kitchen", "salmon").unwrap().used, 100, "a provisional sale uses no budget");
}

/// One tablet's (or the hub's) run of sales: `n` portions, ten minutes apart. Answers how many
/// were provisional.
fn run(log: &mut StockLog, ctx: &str, item: &str, portion: i64, n: usize, t: &mut i64) -> usize {
    let mut prov = 0;
    for i in 0..n {
        *t += 10 * MIN;
        log.set_clock(*t);
        prov += usize::from(log.sell(ctx, item, portion, &format!("{ctx}-{item}-{i}")).unwrap());
    }
    prov
}

/// The morning: the shelf counted, the two tablets granted their shares at the sync.
fn morning() -> StockLog {
    let mut hub = shelf();
    for (ctx, s, r, a) in [("kitchen", 800, 2000, 12), ("waiter", 600, 2000, 10)] {
        hub.grant(ctx, "salmon", s, Some(UNTIL)).unwrap();
        hub.grant(ctx, "rice", r, Some(UNTIL)).unwrap();
        hub.grant(ctx, "avocado", a, Some(UNTIL)).unwrap();
    }
    hub
}

/// The day, offline: what each context sold. Answers the hub (still holding its own sales and a
/// noon delivery), the two tablets, and the provisional count of each.
fn day(morning: &StockLog) -> (StockLog, StockLog, StockLog, [usize; 3]) {
    let bytes = morning.to_bytes_trimmed();
    let (mut a, mut b) = (StockLog::load(&bytes).unwrap(), StockLog::load(&bytes).unwrap());
    let mut hub = StockLog::load(&bytes).unwrap();
    let (mut ta, mut tb, mut th) = (T0, T0, T0);
    let pa = run(&mut a, "kitchen", "salmon", 80, 12, &mut ta) + run(&mut a, "kitchen", "rice", 150, 12, &mut ta)
        + run(&mut a, "kitchen", "avocado", 1, 12, &mut ta);
    let mut pb = run(&mut b, "waiter", "salmon", 60, 12, &mut tb) + run(&mut b, "waiter", "rice", 150, 13, &mut tb)
        + run(&mut b, "waiter", "avocado", 1, 11, &mut tb);
    let mut late = UNTIL + 30 * MIN;
    pb += run(&mut b, "waiter", "rice", 40, 1, &mut late);
    hub.set_clock(T0 + 4 * 60 * MIN);
    hub.append(&received("rice", 1000)).unwrap();
    let ph = run(&mut hub, HUB, "salmon", 100, 7, &mut th) + run(&mut hub, HUB, "rice", 200, 10, &mut th)
        + run(&mut hub, HUB, "avocado", 1, 9, &mut th);
    (hub, a, b, [ph, pa, pb])
}

/// The tablet's sales after the morning sync, as written: what S2 will send over the wire.
fn tail_of(morning: &StockLog, tablet: &StockLog) -> Vec<String> {
    tablet.raw()[morning.len()..].iter().filter(|r| decode(r).is_some()).cloned().collect()
}

/// The evening: the hub takes each tablet's tail, in the order given, record by record.
fn merged(hub: &StockLog, tails: &[&[String]]) -> StockLog {
    let mut m = StockLog::load(&hub.to_bytes_trimmed()).unwrap();
    for tail in tails {
        for rec in tail.iter() {
            m.write_payload(rec.clone().into_bytes()).unwrap();
        }
    }
    m
}

#[test]
fn a_day_with_two_offline_tablets_has_no_unflagged_sale_beyond_on_hand() {
    let am = morning();
    let (hub, a, b, prov) = day(&am);
    assert_eq!(prov, [2, 2, 4], "hub: 1 salmon + 1 avocado; kitchen: 2 salmon; waiter: 2 salmon, 1 avocado, 1 rice after 23:00");
    let (ta, tb) = (tail_of(&am, &a), tail_of(&am, &b));
    let ab = merged(&hub, &[&ta, &tb]);
    let ba = merged(&hub, &[&tb, &ta]);
    for m in [&ab, &ba] {
        assert_eq!(m.unflagged_oversold().unwrap(), vec![], "0 sales beyond on-hand that are not flagged provisional");
        assert_eq!(m.provisional_sales().len(), 8, "every one of them reaches the hub");
        let led = m.ledger().unwrap();
        assert_eq!(led.level("salmon").on_hand, 2000 - 960 - 720 - 700, "the real shortfall is on the shelf for a person");
        assert_eq!(led.level("avocado").on_hand, 30 - 12 - 11 - 9);
        assert_eq!(led.level("rice").on_hand, 7000 - 1800 - 1990 - 2000);
    }
    assert_eq!(ab.ledger().unwrap(), ba.ledger().unwrap(), "the two merge orders glue to one shelf");

    // THE CHECK CAN FAIL: the same day with nothing granted and nothing flagged.
    let mut bare = shelf();
    let (bytes, base) = (bare.to_bytes_trimmed(), bare.len());
    let mut t = StockLog::load(&bytes).unwrap();
    for i in 0..30 {
        let (log, o) = if i % 2 == 0 { (&mut t, format!("t{i}")) } else { (&mut bare, format!("h{i}")) };
        log.append(&served("salmon", 80, &o)).unwrap();
    }
    let tail: Vec<String> = t.raw()[base..].iter().filter(|r| decode(r).is_some()).cloned().collect();
    let over = merged(&bare, &[&tail]).unflagged_oversold().unwrap();
    assert!(!over.is_empty() && over.iter().all(|o| o.item == "salmon"), "2400 sold of 2000 unflagged: {over:?}");
}

#[test]
fn glue_reports_every_provisional_sale_among_its_conflicts() {
    let am = morning();
    let (hub, a, b, _) = day(&am);
    let g = glue(&hub.to_bytes_trimmed(), &a.to_bytes_trimmed(), None).unwrap();
    let mut got: Vec<(Side, String)> = g
        .conflicts
        .iter()
        .filter_map(|c| match c {
            Conflict::Provisional { side, order_id, .. } => Some((*side, order_id.clone())),
            _ => None,
        })
        .collect();
    got.sort_by(|x, y| x.1.cmp(&y.1));
    let want = [(Side::Hub, "hub-avocado-8"), (Side::Hub, "hub-salmon-6"), (Side::Kitchen, "kitchen-salmon-10"), (Side::Kitchen, "kitchen-salmon-11")];
    assert_eq!(got, want.map(|(s, o)| (s, o.to_string())), "{:?}", g.conflicts);
    assert_eq!(g.conflicts.len(), 4, "nothing else: kitchen and hub together stay on the shelf: {:?}", g.conflicts);
    assert_eq!(g.state(), State::Obstructed, "a provisional sale waits for a person");
    // A tablet syncing while the hub wrote nothing still surfaces its provisional sales.
    let ff = glue(&am.to_bytes_trimmed(), &b.to_bytes_trimmed(), None).unwrap();
    assert_eq!(ff.state(), State::FastForward { ahead: Side::Kitchen });
    let n = ff.conflicts.iter().filter(|c| matches!(c, Conflict::Provisional { ctx, .. } if ctx == "waiter")).count();
    assert_eq!(n, 4, "{:?}", ff.conflicts);
    // And a sale inside its rights is not one.
    let rec = r#"{"k":"served","item":"salmon","qty":1,"order":"x","ctx":"kitchen"}"#;
    assert_eq!(provisional_of(rec), None);
    let m = Meta { ctx: Some("kitchen".into()), prov: Some(1), ..Meta::default() };
    assert!(provisional_of(&super::super::meta::with_meta(&super::super::encode(&served("salmon", 1, "x")), &m)).is_some());
}
