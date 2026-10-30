//! `walk_marked_where` IS `walk_marked` FILTERED BY A PREFIX -- record for record, mark for
//! mark -- in v1 and v2, for named and unnamed actors, and for headers that lie about the
//! payload length. Seeded, deterministic logs; no fuzzing (operator rule).
use crate::{EvLog, Record};
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
        let r = Record { id: [i as u8; 32], prev: [0; 32], actor_pubkey: actor, actor_seq: i as u64, flags: 0, payload: framed(1 + (i % 4) as u8, &id, &json) };
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

/// A `FLAG_SHA256` RECORD IS MATCHED ON ITS BODY (M0): the sixteen nonce bytes in front of its
/// payload are skipped, so a head at `at` means body byte `at` on a flagged record and payload
/// byte `at` on a legacy one, in the same walk -- and the reference is `walk_marked` filtered
/// the same way. A flagged record shorter than its nonce has no body and matches nothing but
/// the empty head.
#[test]
fn a_flagged_record_is_matched_on_its_body_behind_the_nonce() {
    use crate::{FLAG_SHA256, NONCE_LEN};
    let mut st = Store::create_bytes(1 << 20).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    for i in 0..60u32 {
        let id = format!("o{}", i % 7);
        let body = framed(1 + (i % 4) as u8, &id, &"j".repeat(i as usize % 11));
        let (flags, payload) = if i % 2 == 0 {
            let mut p = format!("{:016}", i).into_bytes();
            p.extend_from_slice(&body);
            (FLAG_SHA256, p)
        } else {
            (0, body)
        };
        let actor = if i % 3 == 0 { [0x5A; 32] } else { [0u8; 32] };
        let payload = if i == 10 { payload[..9].to_vec() } else { payload }; // a flagged stub: no body
        EvLog::append_bytes(&mut st, &Record { id: [i as u8; 32], prev: [0; 32], actor_pubkey: actor, actor_seq: i as u64, flags, payload }).unwrap();
    }
    let body_of = |r: &Record| -> Vec<u8> { if r.flags & FLAG_SHA256 != 0 { r.payload.get(NONCE_LEN..).unwrap_or(&[]).to_vec() } else { r.payload.clone() } };
    for (at, head) in heads(7) {
        let want: Vec<(Record, Option<u32>)> = EvLog::walk_marked(&st)
            .into_iter()
            .filter(|(r, _)| at.checked_add(head.len()).and_then(|e| body_of(r).get(at..e).map(|b| b == head.as_slice())) == Some(true))
            .collect();
        assert_eq!(EvLog::walk_marked_where(&st, at, &head), want, "at={at} head={head:?}");
    }
    let hits = EvLog::walk_marked_where(&st, 1, b"\x02o3");
    assert!(hits.iter().any(|(r, _)| r.flags == FLAG_SHA256) && hits.iter().any(|(r, _)| r.flags == 0), "both kinds matched on the same head");
    EvLog::scan(&st, |p| {
        let r = p.record();
        let body = body_of(&r);
        assert_eq!(p.len(), body.len(), "pos {}", p.pos);
        assert!((0..=body.len()).all(|j| p.byte(j) == body.get(j).copied()));
        false
    });
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

/// `scan` IS `walk_marked`, one `Peek` per record: the same position, the same unpacked
/// record, the same mark, the same bytes -- over v1, v2, a flipped byte and lying lengths.
/// And a `visit` that says stop ends the walk after that record.
#[test]
fn scan_peeks_are_walk_marked() {
    let mut cases = vec![log(false, 300, 40), log(true, 150, 20)];
    let mut flipped = log(false, 200, 30);
    let o = nth(&flipped, 9);
    flipped.cells[o + 2 + 13] ^= 0x100;
    cases.push(flipped);
    let mut lying = log(false, 120, 15);
    let o = nth(&lying, 4);
    lying.cells[o + 2 + 1] = 0xFFFF_FFFF;
    cases.push(lying);
    for st in &cases {
        let want = EvLog::walk_marked(st);
        let mut got = Vec::new();
        EvLog::scan(st, |p| {
            let r = p.record();
            assert_eq!(p.len(), r.payload.len());
            assert!((0..=r.payload.len()).all(|j| p.byte(j) == r.payload.get(j).copied()));
            got.push((p.pos, r, p.crc_bad()));
            false
        });
        assert_eq!(got.len(), want.len());
        for (i, ((pos, r, bad), (wr, wbad))) in got.into_iter().zip(want).enumerate() {
            assert_eq!((pos, &r, bad), (i, &wr, wbad), "record {i}");
        }
        let mut seen = 0;
        EvLog::scan(st, |p| {
            seen += 1;
            p.pos == 5
        });
        assert_eq!(seen, 6, "stops after the record visit accepted");
    }
}
