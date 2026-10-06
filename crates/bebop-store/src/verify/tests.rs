//! Tests for `verify.rs` (W-CRC): one named corrupted byte per reader, the frozen images,
//! and the superseded-cells counter (D.1 #4). No fuzzing (operator rule): each defect is
//! locked by one named cell.
use super::*;
use crate::evlog::Record;

fn kv_image() -> Store {
    let mut kv = Kv { entries: Vec::new() };
    kv.put("dish/0001", b"{\"name\":\"maki\",\"price\":900}");
    kv.put("dish/0002", b"{\"name\":\"nigiri\",\"price\":1200}");
    kv.put("venue", b"sushi");
    Store::from_bytes(&kv.compacted_bytes_fit(1 << 20).unwrap())
}

/// Flip ONE BYTE (byte 1 of payload cell `i`) of the object at `obj`.
fn flip(st: &mut Store, obj: usize, i: usize) {
    st.cells[obj + 2 + i] ^= 0x100;
}

/// ONE FLIPPED PAYLOAD BYTE IN ANY OF THE FIVE KV OBJECTS IS A NAMED REFUSAL. Before
/// W-CRC a byte in the value blob came back as a different value, with nothing to say so.
#[test]
fn kv_one_flipped_payload_byte_is_a_named_bad_crc() {
    let clean = kv_image();
    let root = clean.root().unwrap();
    let mut objs = vec![("root", root, 0usize)];
    for (name, i) in [("kidx", 1), ("kblob", 2), ("vidx", 3), ("vblob", 4)] {
        objs.push((name, clean.follow(root, i).unwrap(), 0));
    }
    assert!(Kv::load_checked(&clean).is_ok(), "the clean image reads");
    for (name, obj, cell) in objs {
        let mut st = Store::from_bytes(&clean.to_bytes());
        flip(&mut st, obj, cell);
        match Kv::load_checked(&st) {
            Err(KvError::BadCrc(b)) => {
                assert_eq!(b.obj, obj, "{name}: the refusal names the object");
                assert!(b.got.is_some() && b.got != Some(b.want), "{name}: both numbers");
            }
            other => panic!("{name}: expected BadCrc, got {:?}", other.map(|k| k.entries.len())),
        }
        assert!(Kv::load(&st).is_none(), "{name}: `load` never hands back a changed value");
    }
}

/// The value byte the old reader would have returned CHANGED -- this is the silent wrong
/// answer, shown, and then refused.
#[test]
fn kv_a_changed_value_byte_was_silently_read_and_is_now_refused() {
    let mut st = kv_image();
    let vblob = st.follow(st.root().unwrap(), 4).unwrap();
    flip(&mut st, vblob, 0);
    let silent = Kv::decode(&st).expect("the unchecked decode still reads it");
    assert_ne!(silent.get("dish/0001").unwrap(), b"{\"name\":\"maki\",\"price\":900}".to_vec());
    assert!(matches!(Kv::load_checked(&st), Err(KvError::BadCrc(b)) if b.obj == vblob));
}

/// A header whose length does not fit the image is a refusal with no payload hash
/// (`got: None`) -- never a panic, never an allocation.
#[test]
fn an_object_claiming_more_than_the_image_is_refused_not_panicked() {
    let mut st = kv_image();
    let root = st.root().unwrap();
    let vblob = st.follow(root, 4).unwrap();
    st.cells[vblob] |= 0xFFFF_FFF0; // the low half of h0 is the length
    assert_eq!(st.check_obj(vblob), Err(BadCrc { obj: vblob, want: (st.cells[vblob + 1] >> 32) as u32, got: None }));
    assert!(matches!(Kv::load_checked(&st), Err(KvError::BadCrc(b)) if b.obj == vblob && b.got.is_none()));
}

fn rec(n: u8, payload: &[u8]) -> Record {
    Record { id: [n; 32], prev: [n.wrapping_sub(1); 32], actor_pubkey: [0; 32], actor_seq: n as u64, payload: payload.to_vec() }
}

fn log_image(n: u8) -> Store {
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    for i in 1..=n {
        EvLog::append_tip_bytes(&mut st, &rec(i, b"{\"kind\":\"placed\",\"total\":1500}")).unwrap();
    }
    st
}

/// ONE FLIPPED PAYLOAD BYTE IN ANY RECORD, OR IN THE ROOT, IS A NAMED REFUSAL from the
/// walk every log loader runs (`dowiz_hub::chain_is_whole`) and from `walk_checked`.
#[test]
fn evlog_one_flipped_payload_byte_is_a_named_bad_crc() {
    let clean = log_image(5);
    assert_eq!(EvLog::chain_crc(&clean), Ok(Some(5)));
    let root = clean.root().unwrap();
    let mut objs = vec![("root", root)];
    let mut cur = clean.follow(root, 1);
    while let Some(o) = cur {
        objs.push(("record", o));
        cur = clean.follow(o, 2);
    }
    assert_eq!(objs.len(), 6);
    for (name, obj) in objs {
        let mut st = Store::from_bytes(&clean.to_bytes());
        // Payload cell 12 of a v2 record is its first payload cell; the root's cell 0 is n.
        let cell = if name == "root" { 0 } else { 12 };
        flip(&mut st, obj, cell);
        let e = EvLog::chain_crc(&st).expect_err(name);
        assert_eq!(e.obj, obj, "{name} at {obj}: the refusal names it");
        assert_eq!(EvLog::walk_checked(&st).map(|r| r.len()), Err(e), "{name}: walk_checked agrees");
    }
}

/// A chain whose records overlap -- the second record's `next` aimed back at the newest --
/// stops at the image's budget and reports "never ends", never hashing image x step_cap.
#[test]
fn evlog_a_looping_chain_is_none_not_a_quadratic_hash() {
    let mut st = log_image(3);
    let newest = st.follow(st.root().unwrap(), 1).unwrap();
    let older = st.follow(newest, 2).unwrap();
    st.cells[older + 2 + 2] = newest as i64 - older as i64;
    assert_eq!(EvLog::chain_crc(&st).map_err(|e| e.obj), Err(older), "unsealed: the edited record is named");
    // Re-sealed, every crc is good and only the loop is left.
    st.seal(older);
    assert_eq!(EvLog::chain_crc(&st), Ok(None), "a loop is reported, with every crc good");
}

/// EVERY FROZEN IMAGE IN THE TREE STILL LOADS. Written by bebop (kv.bp, store.bp) and by
/// the hub on 2026-09-23/24, long before this check existed: a crc check that refused one
/// would be a venue outage on deploy.
#[test]
fn the_frozen_fixtures_pass_the_check() {
    for (name, b) in [
        ("kv.store v1", &include_bytes!("../../../bebop-wasm/fixtures/kv.store")[..]),
        ("kv2.store v2", &include_bytes!("../../../bebop-wasm/fixtures/kv2.store")[..]),
    ] {
        let st = Store::from_bytes(b);
        let kv = Kv::load_checked(&st).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(kv.entries.len(), 5, "{name}");
        assert_eq!(kv.snapshot_root_u64() as i64, -211109995167561145, "{name}: the fold is unchanged");
    }
    for (name, b, n) in [
        ("proj.store (bebop store.bp)", &include_bytes!("../../../bebop-wasm/fixtures/proj.store")[..], Some(9)),
        ("decide/amend.log", &include_bytes!("../../../bebop-wasm/fixtures/decide/amend.log")[..], None),
        ("decide/amend.stock", &include_bytes!("../../../bebop-wasm/fixtures/decide/amend.stock")[..], None),
        ("decide/pay.log", &include_bytes!("../../../bebop-wasm/fixtures/decide/pay.log")[..], None::<usize>),
    ] {
        let st = Store::from_bytes(b);
        let len = EvLog::len(&st);
        assert!(len > 0, "{name}: a log with records");
        assert_eq!(EvLog::chain_crc(&st), Ok(Some(n.unwrap_or(len))), "{name}");
    }
}

/// SUPERSEDED CELLS ARE COUNTED (D.1 #4): every KV commit makes the previous root and its
/// four arrays dead, so the counter rises by exactly their size; a compaction writes a
/// store with nothing dead in it.
#[test]
fn kv_superseded_cells_rise_with_each_commit_and_compaction_clears_them() {
    let p = std::env::temp_dir().join("bebop_wcrc_sup.store");
    let p = p.to_str().unwrap();
    let mut st = Store::create(p, 256 * 1024).unwrap();
    Kv::init(&mut st, p).unwrap();
    let mut kv = Kv::load(&st).unwrap();
    let mut last = st.pick().unwrap().superseded_cells;
    assert_eq!(last, 0, "a fresh schema has nothing dead");
    for i in 0..4 {
        let root = st.root().unwrap();
        let mut old = 2 + st.obj_cells(root) as i64;
        for a in 1..=4 {
            old += 2 + st.obj_cells(st.follow(root, a).unwrap()) as i64;
        }
        kv.put(&format!("k{i}"), b"value");
        kv.commit_into(&mut st, p).unwrap();
        let sb = st.pick().unwrap();
        assert_eq!(sb.superseded_cells - last, old, "commit {i}: the old five objects, exactly");
        last = sb.superseded_cells;
        // live + superseded is every cell the arena handed out.
        assert_eq!(sb.live_cells + sb.superseded_cells, sb.arena_used - crate::ARENA as i64, "commit {i}");
    }
    let fresh = Store::from_bytes(&kv.compacted_bytes(256 * 1024).unwrap());
    let sb = fresh.pick().unwrap();
    assert_eq!(sb.superseded_cells, 0, "compaction leaves nothing dead");
    assert_eq!(sb.live_cells, sb.arena_used - crate::ARENA as i64);
    assert_eq!(Kv::load_checked(&fresh).unwrap().entries, kv.entries);
    let _ = std::fs::remove_file(p);
}

/// An append supersedes exactly the old root (8 cells + 2 header in v2); records never die.
#[test]
fn evlog_each_append_supersedes_exactly_the_old_root() {
    let mut st = log_image(0);
    for i in 1..=4u8 {
        let before = st.pick().unwrap().superseded_cells;
        EvLog::append_tip_bytes(&mut st, &rec(i, b"x")).unwrap();
        assert_eq!(st.pick().unwrap().superseded_cells - before, 2 + 8, "append {i}");
    }
    let before = st.pick().unwrap().superseded_cells;
    EvLog::set_tip_bytes(&mut st, &[9u8; 32]).unwrap();
    assert_eq!(st.pick().unwrap().superseded_cells - before, 10, "a tip move supersedes the root too");
}
