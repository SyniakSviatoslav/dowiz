//! W-ES-P19b: optimistic writers. Any number open; the section validates each one's read set
//! against the commits since its base (gluing: `Refusal::Conflict` is the default resolver) and
//! replays it on the CURRENT root. Every refusal test has its positive twin.

use std::ops::Bound::{Excluded, Included, Unbounded};

use super::*;
use crate::Detached;

fn db() -> Db<Detached> {
    let db = Db::create_detached(Options::detached()).unwrap();
    let mut w = db.begin_write().unwrap();
    for (k, v) in [(b"a", b"1"), (b"c", b"3"), (b"x", b"9")] {
        w.put(k, v).unwrap();
    }
    w.commit(Durability::None).unwrap();
    db
}

fn get(db: &Db<Detached>, k: &[u8]) -> Option<Vec<u8>> {
    db.begin_read().get(k).unwrap().map(|b| b.into_owned())
}

/// Two writers open at once; disjoint keys: both commit, generations in commit order, and the
/// second one's path was built on the first one's root (nothing lost).
#[test]
fn two_open_writers_with_disjoint_keys_both_commit() {
    let db = db();
    let g = db.begin_read().generation().get();
    let mut w1 = db.begin_write().unwrap();
    let mut w2 = db.begin_write().unwrap();
    assert_eq!(w1.get(b"a").unwrap().unwrap(), &b"1"[..]);
    w1.put(b"a", b"11").unwrap();
    assert_eq!(w2.get(b"c").unwrap().unwrap(), &b"3"[..]);
    w2.put(b"c", b"33").unwrap();
    assert_eq!(w2.commit(Durability::None).unwrap().get(), g + 1, "the later-opened one commits first");
    assert_eq!(w1.commit(Durability::None).unwrap().get(), g + 2, "and the earlier one glues onto it");
    assert_eq!(get(&db, b"a").as_deref(), Some(&b"11"[..]));
    assert_eq!(get(&db, b"c").as_deref(), Some(&b"33"[..]), "the first commit survived the second (RED: a replay onto the base)");
    assert_eq!(get(&db, b"x").as_deref(), Some(&b"9"[..]));
}

/// A key read here and written by a later commit: `Conflict { with }` names that commit; the
/// twins: a BLIND write (nothing read) never conflicts, and a read of an untouched key passes.
#[test]
fn a_stale_read_is_refused_by_the_commit_it_lost_to() {
    let db = db();
    let mut w1 = db.begin_write().unwrap();
    assert_eq!(w1.get(b"x").unwrap().unwrap(), &b"9"[..]);
    w1.put(b"a", b"from-x").unwrap();
    let mut w2 = db.begin_write().unwrap();
    w2.put(b"x", b"10").unwrap();
    let g2 = w2.commit(Durability::None).unwrap().get();
    assert!(matches!(w1.commit(Durability::None), Err(Refusal::Conflict { with }) if with == g2));
    assert_eq!(get(&db, b"a").as_deref(), Some(&b"1"[..]), "the refused transaction wrote nothing");
    let mut blind = db.begin_write().unwrap();
    blind.put(b"x", b"11").unwrap();
    assert!(blind.commit(Durability::None).is_ok(), "a blind write has no read set");
    let mut other = db.begin_write().unwrap();
    assert_eq!(other.get(b"c").unwrap().unwrap(), &b"3"[..]);
    other.put(b"c", b"4").unwrap();
    let mut mid = db.begin_write().unwrap();
    mid.put(b"a", b"2").unwrap();
    mid.commit(Durability::None).unwrap();
    assert!(other.commit(Durability::None).is_ok(), "c was read, only a was written meanwhile");
}

/// PHANTOMS: T1 scans [a, c] and writes x; T2 inserts b (inside the range) and commits first;
/// T1 is refused -- the range it read now holds a key it never saw. Twins: an insert OUTSIDE the
/// range passes; `len()` reads everything and conflicts with any write; a prefix scan is a range.
#[test]
fn a_scanned_range_catches_an_insert_inside_it() {
    let db = db();
    let mut t1 = db.begin_write().unwrap();
    let seen: Vec<Vec<u8>> = t1.scan(Included(b"a"), Included(b"c")).unwrap().map(|e| e.unwrap().0.into_owned()).collect();
    assert_eq!(seen, vec![b"a".to_vec(), b"c".to_vec()]);
    t1.put(b"x", b"sum").unwrap();
    let mut t2 = db.begin_write().unwrap();
    t2.put(b"b", b"2").unwrap();
    let g2 = t2.commit(Durability::None).unwrap().get();
    assert!(matches!(t1.commit(Durability::None), Err(Refusal::Conflict { with }) if with == g2), "the phantom b");
    let mut t3 = db.begin_write().unwrap();
    let _ = t3.scan(Excluded(b"a"), Excluded(b"c")).unwrap().count();
    t3.put(b"x", b"sum").unwrap();
    let mut t4 = db.begin_write().unwrap();
    t4.put(b"d", b"4").unwrap();
    t4.commit(Durability::None).unwrap();
    assert!(t3.commit(Durability::None).is_ok(), "d is outside (a, c)");
    let mut t5 = db.begin_write().unwrap();
    assert!(t5.len().unwrap() > 0);
    t5.put(b"x", b"n").unwrap();
    let mut t6 = db.begin_write().unwrap();
    t6.put(b"zz", b"far").unwrap();
    t6.commit(Durability::None).unwrap();
    assert!(matches!(t5.commit(Durability::None), Err(Refusal::Conflict { .. })), "len() read everything");
    let mut t7 = db.begin_write().unwrap();
    let _ = t7.prefix(b"z").unwrap().count();
    t7.put(b"x", b"p").unwrap();
    let mut t8 = db.begin_write().unwrap();
    t8.put(b"zy", b"in-prefix").unwrap();
    t8.commit(Durability::None).unwrap();
    assert!(matches!(t7.commit(Durability::None), Err(Refusal::Conflict { .. })), "zy is under prefix z");
}

/// THREE THREADS, read-modify-write on 16 shared counters through `Db::write` (which retries on
/// `Conflict`): every increment lands exactly once. RED when validation is skipped (lost
/// updates) or when the replay runs on a transaction's base instead of the current root.
#[test]
fn three_writers_through_db_write_lose_no_update() {
    let db = Db::create_detached(Options::detached()).unwrap();
    let keys: Vec<Vec<u8>> = (0..16u8).map(|i| vec![b'k', i]).collect();
    let mut w = db.begin_write().unwrap();
    for k in &keys {
        w.put(k, &0u64.to_le_bytes()).unwrap();
    }
    w.commit(Durability::None).unwrap();
    let per_thread = 200usize;
    std::thread::scope(|s| {
        for t in 0..3u64 {
            let (db, keys) = (&db, &keys);
            s.spawn(move || {
                let mut x = 0x9E37_79B9u64 ^ (t + 1);
                for _ in 0..per_thread {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    let k = &keys[(x % 16) as usize];
                    db.write(Durability::None, |w| {
                        let v = w.get(k)?.map(|b| u64::from_le_bytes(b[..8].try_into().unwrap())).unwrap_or(0);
                        w.put(k, &(v + 1).to_le_bytes())
                    })
                    .unwrap();
                }
            });
        }
    });
    let r = db.begin_read();
    let total: u64 = keys.iter().map(|k| u64::from_le_bytes(r.get(k).unwrap().unwrap()[..8].try_into().unwrap())).sum();
    assert_eq!(total, 3 * per_thread as u64, "an increment was lost");
    assert_eq!(db.snapshot_bytes(), db.writer_image());
    db.verify(Verify::WholeArena).unwrap();
}

/// A transaction whose base fell off the commit log (older than LOG_HORIZON commits) is refused
/// outright: it cannot be proved clean. The twin: one commit younger passes.
#[test]
fn a_base_older_than_the_log_horizon_is_refused() {
    let db = db();
    let mut old = db.begin_write().unwrap();
    assert!(old.get(b"a").unwrap().is_some());
    old.put(b"a", b"old").unwrap();
    let mut young = None;
    for i in 0..=super::commit::LOG_HORIZON {
        if i == 1 {
            let mut y = db.begin_write().unwrap();
            assert!(y.get(b"c").unwrap().is_some());
            y.put(b"c", b"young").unwrap();
            young = Some(y);
        }
        let mut w = db.begin_write().unwrap();
        w.put(b"x", &(i as u32).to_le_bytes()).unwrap();
        w.commit(Durability::None).unwrap();
    }
    assert!(matches!(old.commit(Durability::None), Err(Refusal::Conflict { .. })), "past the horizon");
    assert!(young.unwrap().commit(Durability::None).is_ok(), "one inside the horizon, nothing it read was written");
}

/// A panic inside a transaction's body poisons nothing (the batch was private; nothing touched
/// the image); a panic INSIDE the section still does, through the writer's lock.
#[test]
fn a_panic_in_a_transaction_body_poisons_nothing() {
    let db = db();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut w = db.begin_write().unwrap();
        w.put(b"a", b"never").unwrap();
        panic!("a panic inside a write transaction");
    }));
    assert!(r.is_err());
    assert!(!db.poisoned());
    assert_eq!(get(&db, b"a").as_deref(), Some(&b"1"[..]));
    let mut w = db.begin_write().unwrap();
    w.put(b"a", b"after").unwrap();
    w.commit(Durability::None).unwrap();
    assert_eq!(get(&db, b"a").as_deref(), Some(&b"after"[..]));
    let _ = Unbounded::<&[u8]>;
}
