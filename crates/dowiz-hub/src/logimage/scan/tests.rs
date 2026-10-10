//! `about` and `recent_checked` ARE the full walk filtered and cut (W-LOOPB, R-LOOPS row 4):
//! on a clean log and with ONE NAMED CORRUPTED CELL before, at and after the match. No fuzzing
//! (operator rule): fixed logs, fixed cells.
use crate::logimage::{Entry, LogImage};
use crate::Quarantined;
use bebop_store::Store;

const K: &str = "wallet.leg";

/// 60 records: every fourth a `wallet.leg`, the rest `chat.msg`, subjects `acct_0..acct_6`.
fn bytes() -> Vec<u8> {
    let mut l = LogImage::create().unwrap();
    for i in 0..60 {
        let kind = if i % 4 == 0 { K } else { "chat.msg" };
        l.append(kind, &format!("acct_{}", i % 7), &format!(r#"{{"i":{i},"amount":{},"note":"topup at the counter"}}"#, 250 * (i % 9))).unwrap();
    }
    l.to_bytes()
}

/// The object of the `at`-th record of a newest-first walk.
fn nth(st: &Store, at: usize) -> usize {
    let mut o = st.follow(st.root().unwrap(), 1).unwrap();
    for _ in 0..at {
        o = st.follow(o, 2).unwrap();
    }
    o
}

/// Payload cell `cell` of the `at`-th record XORed with `mask`; re-sealed or not.
fn changed(b: &[u8], at: usize, cell: usize, mask: i64, reseal: bool) -> Vec<u8> {
    let mut st = Store::from_bytes(b);
    let o = nth(&st, at);
    st.cells[o + 2 + cell] ^= mask;
    if reseal {
        st.seal(o);
    }
    st.to_bytes()
}

/// The queries the Worker asks: one subject's newest, a few, all; a kind alone; nothing.
fn queries() -> Vec<(&'static str, Option<String>, usize)> {
    let mut q = Vec::new();
    for s in 0..8 {
        for limit in [1, 3, usize::MAX] {
            q.push((K, Some(format!("acct_{s}")), limit));
        }
    }
    q.extend([(K, None, 1), (K, None, 5), (K, None, usize::MAX), ("chat.msg", None, 2), ("chat.msg", Some("acct_2".into()), 4)]);
    q.extend([("nope", None, 9), (K, Some("acct_1".into()), 0), ("\u{FFFD}", None, 3), (K, Some("x".repeat(300)), 1)]);
    q
}

fn reference(l: &LogImage, n: usize) -> Result<Vec<Entry>, Quarantined> {
    match l.quarantined().into_iter().next() {
        Some(q) => Err(q),
        None => Ok(l.entries().into_iter().take(n).collect()),
    }
}

/// The cases, around the newest `acct_3` leg (record 52, `at` 7; the older one is record 24,
/// `at` 35). Payload bytes: 0 kind length, 1..=10 kind, 11 subject length, 12..=17 subject,
/// 18.. JSON; cell 12 holds bytes 0..8, cell 14 bytes 16..24, cell 15 bytes 24..32.
///   * clean;
///   * JSON byte 25 changed, NOT re-sealed (crc fails, the prefix still matches): at the match,
///     before it (newer: `at` 1, a chat) and after it (older: the other `acct_3` leg);
///   * subject byte 17 changed, not re-sealed (crc fails AND the prefix no longer matches);
///   * kind-length byte 0 changed and RE-SEALED (crc holds, framing broken): at the match and at
///     the oldest record (`at` 59).
fn cases() -> Vec<(&'static str, Vec<u8>)> {
    let b = bytes();
    vec![
        ("clean", b.clone()),
        ("json byte at the match", changed(&b, 7, 15, 0x100, false)),
        ("json byte before the match", changed(&b, 1, 15, 0x100, false)),
        ("json byte after the match", changed(&b, 35, 15, 0x100, false)),
        ("subject byte at the match", changed(&b, 7, 14, 0x100, false)),
        ("kind length, sealed, oldest", changed(&b, 59, 12, 0xF0, true)),
        ("kind length, sealed, at the match", changed(&b, 7, 12, 0xF0, true)),
        // Byte 11 (cell 13, bits 24..32) is the SUBJECT length of a `wallet.leg` record (record 20,
        // `at` 39): 6 -> 246 runs past the payload, the `subject-framing` refusal.
        ("subject length, sealed, older", changed(&b, 39, 13, 0xF0 << 24, true)),
    ]
}

#[test]
fn about_equals_the_full_walk_filtered_with_one_corrupted_cell_anywhere() {
    for (what, b) in cases() {
        let l = LogImage::load(&b).unwrap_or_else(|e| panic!("{what}: {e:?}"));
        if what != "clean" {
            assert_eq!(l.quarantined().len(), 1, "{what}: exactly the one named cell is quarantined");
        }
        for (kind, subject, limit) in queries() {
            let (got, want) = (l.about(kind, subject.as_deref(), limit), l.about_all(kind, subject.as_deref(), limit));
            assert_eq!(got, want, "{what}: about({kind}, {subject:?}, {limit})");
        }
        // The fixture is what the cases say it is: the newest acct_3 leg is record 52 unless
        // that is the record the case damaged, and then it is the older one (24).
        let newest3 = l.about(K, Some("acct_3"), 1).first().map(|e| e.seq);
        assert_eq!(newest3, Some(if what.contains("at the match") { 24 } else { 52 }), "{what}");
    }
}

#[test]
fn recent_checked_equals_quarantined_first_then_entries_take() {
    for (what, b) in cases() {
        let l = LogImage::load(&b).unwrap();
        for n in [0, 1, 3, 10, 60, 99] {
            assert_eq!(l.recent_checked(n), reference(&l, n), "{what}: n={n}");
        }
    }
    // The refusal names the NEWEST bad record even when the window ends before it.
    let l = LogImage::load(&changed(&bytes(), 59, 12, 0xF0, true)).unwrap();
    let q = l.recent_checked(2).unwrap_err();
    assert_eq!((q.at, q.reason), (59, "kind-framing"));
    let l = LogImage::load(&changed(&bytes(), 39, 13, 0xF0 << 24, true)).unwrap();
    let q = l.recent_checked(2).unwrap_err();
    assert_eq!((q.at, q.reason), (39, "subject-framing"));
}
