//! B7: log readers re-walk and re-crc the chain that `load` already verified.
//!   (a) `dowiz_hub::read::Hub::events` = `EvLog::walk_marked` (a pointer walk, a full read walk,
//!       `check_obj` crc per record) + decode of every record. Called per command write
//!       (`hubdo.rs:951 put_log` -> `Written::Log(hub.events())`) and per cold fold.
//!       REPLACEMENT: `EvLog::walk` (one read walk, no second crc) + the same decode -- legal only
//!       on a log whose load-time `chain_crc` passed with no quarantined record (the bad-set is
//!       known at load, as `StockLog` keeps it).
//!   (b) `LogImage::about(kind, Some(subject), 1)` (wallet.rs:308/372, channels.rs:596) =
//!       `entries()` of EVERYTHING then `take(1)`. REPLACEMENT: `walk_until` stopping at the first
//!       record whose subject matches, decoding only that one.
//! Equivalence: identical event lists / identical first entry.
use bebop_store::evlog::EvLog;
use bebop_store::Store;
use dowiz_hub::logimage::{Entry, LogImage};
use dowiz_hub::{Event, EventKind, Hub};
use serde_json::json;

fn order_json(i: usize, status: &str) -> String {
    json!({
        "id": format!("o_{i}"), "location_id": "loc_sushi", "status": status, "created_at_ms": 1_782_900_000_000i64 + i as i64 * 60_000,
        "total": 1500 + (i as i64 % 13) * 250, "currency": "ALL",
        "items": (0..4).map(|k| json!({ "product_id": format!("p_{}", (i * 7 + k) % 165), "name": format!("Roll {} me salmon", (i * 7 + k) % 165), "quantity": 1, "unit_price": 700 })).collect::<Vec<_>>(),
        "contact": { "name": format!("Guest {i}"), "phone": "+355690000000" },
        "fulfilment": { "kind": "delivery", "address": { "line": "Rruga Taulantia 12" } },
    })
    .to_string()
}

/// `read.rs` decode of a Hub record: payload = [kind][id_len][id bytes][order json].
fn decode_hub(r: &bebop_store::evlog::Record) -> Option<Event> {
    let p = &r.payload;
    let kind = EventKind::from_u8(*p.first()?)?;
    let idl = *p.get(1)? as usize;
    let id = std::str::from_utf8(p.get(2..2 + idl)?).ok()?;
    let j = std::str::from_utf8(p.get(2 + idl..)?).ok()?;
    Some(Event { kind, order_id: id.to_string(), order_json: j.to_string(), seq: r.actor_seq })
}

/// `logimage.rs` decode: [klen][kind][slen][subject][json].
fn decode_journal(p: &[u8], seq: u64) -> Option<Entry> {
    let kl = *p.first()? as usize;
    let k = p.get(1..1 + kl)?;
    let sl = *p.get(1 + kl)? as usize;
    let s = p.get(2 + kl..2 + kl + sl)?;
    let j = p.get(2 + kl + sl..)?;
    Some(Entry { kind: String::from_utf8_lossy(k).into_owned(), subject: String::from_utf8_lossy(s).into_owned(), json: String::from_utf8_lossy(j).into_owned(), seq })
}

fn same(a: &Event, b: &Event) -> bool {
    a.kind as u8 == b.kind as u8 && a.order_id == b.order_id && a.order_json == b.order_json && a.seq == b.seq
}

fn us(n: u128) -> f64 {
    n as f64 / 1e3
}

pub fn run(reps: usize) {
    for &events in &[200usize, 600, 2000] {
        // ── (a) the order log ──
        let mut hub = Hub::create_sized(8 * 1024 * 1024).unwrap();
        for i in 0..events {
            let (kind, st) = match i % 3 { 0 => (EventKind::Placed, "PENDING"), 1 => (EventKind::Advanced, "CONFIRMED"), _ => (EventKind::Paid, "PAID") };
            hub.append(kind, &format!("o_{}", i / 3), &order_json(i / 3, st), i as u64, [0u8; 32]).unwrap();
        }
        let bytes = hub.to_bytes_trimmed();
        let hub = Hub::load(&bytes).unwrap(); // load = chain_crc of every record (kept as the base cost)
        let store = Store::from_bytes(&bytes); // the same store the Hub holds; outside the timed region
        let cur = hub.events();
        let rep: Vec<Event> = EvLog::walk(&store).iter().filter_map(decode_hub).collect();
        assert_eq!(cur.len(), rep.len());
        assert!(cur.iter().zip(&rep).all(|(a, b)| same(a, b)), "B7a events must be identical");
        println!("B7a EQUIV ok: {} events, log {} KB", cur.len(), bytes.len() / 1024);
        let (_, t_load, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(Hub::load(&bytes).unwrap()); }));
        let (_, t_cur, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(hub.events()); }));
        let (_, t_walk, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(EvLog::walk(&store)); }));
        let (_, t_rep, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(EvLog::walk(&store).iter().filter_map(decode_hub).collect::<Vec<_>>()); }));
        println!("B7a events={events}: Hub::load(crc) {:.0} us | CURRENT events() {:.0} us | walk only {:.0} us | REPLACE walk+decode {:.0} us | {:.1}x on events(); load+events {:.0} -> {:.0} us ({:.1}% -> {:.1}% of 10 ms)",
            us(t_load), us(t_cur), us(t_walk), us(t_rep), us(t_cur) / us(t_rep).max(0.001), us(t_load + t_cur), us(t_load + t_rep), us(t_load + t_cur) / 100.0, us(t_load + t_rep) / 100.0);

        // ── (b) a journal-style LogImage: about(kind, subject, 1) ──
        let mut log = LogImage::create_sized(8 * 1024 * 1024).unwrap();
        for i in 0..events {
            let kind = if i % 4 == 0 { "wallet.leg" } else { "chat.msg" };
            log.append(kind, &format!("acct_{}", i % 37), &json!({ "i": i, "amount": 250 * (i as i64 % 9), "note": "topup via cash at the counter" }).to_string()).unwrap();
        }
        let jb = log.to_bytes();
        let log = LogImage::load(&jb).unwrap();
        let jstore = Store::from_bytes(&jb);
        let want_kind = "wallet.leg";
        let want_subject = format!("acct_{}", 4 * 7 % 37); // some subject that has wallet.leg records
        let cur1 = log.about(want_kind, Some(&want_subject), 1);
        let n = log.len() as u64;
        let walked = EvLog::walk_until(&jstore, |r| decode_journal(&r.payload, 0).is_some_and(|e| e.kind == want_kind && e.subject == want_subject));
        let rep1: Vec<Entry> = walked.last().and_then(|r| decode_journal(&r.payload, n - walked.len() as u64)).into_iter().collect();
        assert_eq!(cur1, rep1, "B7b about(..,1) must be identical");
        println!("B7b EQUIV ok: about({want_kind}, {want_subject}, 1) -> seq {:?}; stopped after {} of {} records", cur1.first().map(|e| e.seq), walked.len(), events);
        let (_, t_cur, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(log.about(want_kind, Some(&want_subject), 1)); }));
        let (_, t_rep, _) = super::median_ns(reps, || super::time_ns(|| {
            let walked = EvLog::walk_until(&jstore, |r| decode_journal(&r.payload, 0).is_some_and(|e| e.kind == want_kind && e.subject == want_subject));
            std::hint::black_box(walked.last().and_then(|r| decode_journal(&r.payload, n - walked.len() as u64)));
        }));
        println!("B7b records={events}: CURRENT about(..,1) {:.0} us | REPLACE walk_until {:.1} us | {:.0}x", us(t_cur), us(t_rep), us(t_cur) / us(t_rep).max(0.001));
    }
}
