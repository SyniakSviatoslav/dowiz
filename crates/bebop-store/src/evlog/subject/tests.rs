//! `walk_marked_where` IS `walk_marked` FILTERED BY A PREFIX -- record for record, mark for
//! mark -- in v1 and v2, for named and unnamed actors, and for headers that lie about the
//! payload length. Seeded, deterministic logs; no fuzzing (operator rule).
use crate::evlog::{EvLog, Record};
use crate::Store;

/// The reference: what the scan must equal.
fn filtered(st: &Store, at: usize, head: &[u8]) -> Vec<(Record, Option<u32>)> {
    EvLog::walk_marked(st)
        .into_iter()
        .filter(|(r, _)| at.checked_add(head.len()).and_then(|e| r.payload.get(at..e)) == Some(head))
        .collect()
}

/// dowiz's framing: [kind][id_len][id][json].
fn framed(kind: u8, id: &str, json: &str) -> Vec<u8> {
    let mut p = vec![kind, id.len() as u8];
    p.extend_from_slice(id.as_bytes());
    p.extend_from_slice(json.as_bytes());
    p
}

/// `n` records over `orders` ids that share prefixes (`o1`, `o10`, `o100`), every third one
/// with a named actor, payload lengths crossing the 8-byte cell boundary at every offset.
fn log(v1: bool, n: u32, orders: u32) -> Store {
    let mut st = Store::create_bytes(1 << 22).unwrap();
    if v1 {
        EvLog::init_v1_bytes(&mut st).unwrap();
    } else {
        EvLog::init_bytes(&mut st).unwrap();
    }
    let mut x: u64 = 0x0C4A_1A_2026_1006;
    for i in 0..n {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let id = format!("o{}", (x >> 33) as u32 % orders);
        let json = "j".repeat((x >> 20) as usize % 19);
        let actor = if i % 3 == 0 { [0x5A; 32] } else { [0u8; 32] };
        let r = Record { id: [i as u8; 32], prev: [0; 32], actor_pubkey: actor, actor_seq: i as u64, payload: framed(1 + (i % 4) as u8, &id, &json) };
        EvLog::append_bytes(&mut st, &r).unwrap();
    }
    st
}

fn heads(orders: u32) -> Vec<(usize, Vec<u8>)> {
    let mut out: Vec<(usize, Vec<u8>)> = (0..orders + 3)
        .map(|k| {
            let id = format!("o{k}");
            let mut h = vec![id.len() as u8];
            h.extend_from_slice(id.as_bytes());
            (1, h)
        })
        .collect();
    out.push((0, Vec::new())); // the empty prefix keeps every record
    out.push((1, vec![2])); // a length byte alone
    out.push((40, b"never".to_vec())); // past the end of most payloads
    out.push((usize::MAX, vec![1])); // an offset that overflows
    out
}

fn assert_same(st: &Store, orders: u32, what: &str) {
    for (at, head) in heads(orders) {
        assert_eq!(EvLog::walk_marked_where(st, at, &head), filtered(st, at, &head), "{what}: at={at} head={head:?}");
    }
}

#[test]
fn equals_walk_marked_filtered_v2() {
    let st = log(false, 600, 120);
    assert_eq!(EvLog::version(&st), 2);
    assert_same(&st, 120, "v2");
    // and it found something: an id that is a prefix of another is not that other
    let one = EvLog::walk_marked_where(&st, 1, b"\x02o1");
    assert!(!one.is_empty() && one.iter().all(|(r, _)| &r.payload[1..4] == b"\x02o1"));
}

#[test]
fn equals_walk_marked_filtered_v1() {
    let st = log(true, 300, 60);
    assert_eq!(EvLog::version(&st), 1);
    assert_same(&st, 60, "v1");
}

/// The `at`-th record of a newest-first walk.
fn nth(st: &Store, at: usize) -> usize {
    let mut o = st.follow(st.root().unwrap(), 1).unwrap();
    for _ in 0..at {
        o = st.follow(o, 2).unwrap();
    }
    o
}

/// A changed payload byte: the record still matches its prefix, and comes back MARKED,
/// exactly as `walk_marked` marks it. A changed byte IN the prefix moves it out of the
/// match on both sides.
#[test]
fn a_quarantined_record_is_marked_the_same() {
    for (v1, cell) in [(false, 13usize), (false, 12), (true, 17), (true, 16)] {
        let mut st = log(v1, 200, 30);
        let o = nth(&st, 7);
        st.cells[o + 2 + cell] ^= 0x100;
        assert!(st.check_obj(o).is_err());
        assert_same(&st, 30, &format!("flipped v1={v1} cell={cell}"));
    }
}

/// A header that LIES about the payload length, both ways: claiming four billion bytes
/// and claiming two. The scan must see exactly the bytes `read_at` would hand back.
#[test]
fn a_lying_length_is_bounded_the_same() {
    for v1 in [false, true] {
        for claim in [i64::MAX, 0xFFFF_FFFF, 2, 1, 0, -5] {
            let mut st = log(v1, 120, 15);
            for at in [3usize, 4, 50] {
                let o = nth(&st, at);
                st.cells[o + 2 + 1] = claim;
            }
            assert_same(&st, 15, &format!("v1={v1} claim={claim}"));
        }
    }
}

/// An empty log and a store with no log at all answer nothing, without panicking.
#[test]
fn nothing_to_walk_is_nothing() {
    let st = Store::create_bytes(64 * 1024).unwrap();
    assert!(EvLog::walk_marked_where(&st, 1, b"\x02o1").is_empty());
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    assert!(EvLog::walk_marked_where(&st, 0, b"").is_empty());
}
