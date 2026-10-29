//! Tests for `kv.rs`, beside the code (DOWIZ-COMMON-RULES rule 7).
use super::*;

/// A store created, written and read back entirely by Rust: the schema, five entries, a
/// reopen, and the FNV-1a root. The root value is dowiz-core's own — the same constant
/// `InMemoryStore` folds over these entries — so this test fails if either the store
/// format handling or the fold drifts.
/// A NON-ASCII KEY MUST SURVIVE A ROUND TRIP. It did not: keys were written
/// as UTF-8 and read back as Latin-1, so an Albanian or Ukrainian id came
/// back as a different string -- and writing that back damaged it further,
/// so the same dish became a new product on every menu import.
#[test]
fn a_non_ascii_key_survives_the_image() {
    let keys = ["pije-ujë-0-5l", "sushi-sets-durrës-set-24", "страва-суші", "ascii-plain"];
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    Kv::init_bytes(&mut st).expect("init");
    let mut kv = Kv::load(&st).expect("load");
    for k in keys {
        kv.put(k, b"v");
    }
    let bytes = kv.compacted_bytes_fit(256 * 1024).expect("write");

    let back = Kv::load(&Store::from_bytes(&bytes)).expect("reload");
    for k in keys {
        assert!(
            back.get(k).is_some(),
            "key {k:?} did not survive; the image holds {:?}",
            back.keys()
        );
    }
    // And the damage must not compound: a second round trip is identical.
    let mut again = back;
    let twice = again.compacted_bytes_fit(256 * 1024).expect("rewrite");
    let back2 = Kv::load(&Store::from_bytes(&twice)).expect("reload twice");
    assert_eq!(back2.keys(), again.keys(), "a second round trip changed the keys");
}

/// ONE FLIPPED BIT USED TO ABORT THE PROCESS, and this is that bit.
///
/// A key's length lives in a cell of the key index. Setting bit 33 of a
/// length of 1 asks for 8589934601 bytes, and the read of it was a
/// `collect` over that range: `memory allocation of 8589934601 bytes
/// failed`, which is an abort rather than an error -- on a Worker, the
/// whole isolate. The length is now measured against the blob that is
/// really there, so the image is REFUSED instead of believed.
///
/// Written as a fixed corruption rather than a generated one: the bit is
/// the whole point, and a named bit is a test that says what it protects.
#[test]
fn a_key_length_larger_than_the_image_is_refused_not_allocated() {
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    Kv::init_bytes(&mut st).expect("init");
    let mut kv = Kv::load(&st).expect("load");
    kv.put("order/0001", b"pending");
    let bytes = kv.compacted_bytes_fit(256 * 1024).expect("write");

    let mut st = Store::from_bytes(&bytes);
    let root = st.root().expect("root");
    let kidx = st.follow(root, 1).expect("key index");
    // Payload cell 1 of the key index is the first key's LENGTH.
    let len_cell = kidx + 2 + 1;
    assert!(st.cells[len_cell] > 0, "the entry's key length should be positive");
    st.cells[len_cell] |= 1 << 33;
    assert!(Kv::load(&st).is_none(), "a key that does not fit its blob is not a key");

    // The same law for a value, and for the entry COUNT -- a root that
    // claims a million entries over an index holding two cells.
    let mut st = Store::from_bytes(&bytes);
    let root = st.root().expect("root");
    let vidx = st.follow(root, 3).expect("value index");
    st.cells[vidx + 2 + 1] |= 1 << 33;
    assert!(Kv::load(&st).is_none(), "a value that does not fit its blob is not a value");

    let mut st = Store::from_bytes(&bytes);
    let root = st.root().expect("root");
    st.cells[root + 2] = 1_000_000;
    assert!(Kv::load(&st).is_none(), "a count the index cannot hold is not a count");
}

#[test]
fn rust_roundtrip_matches_dowiz_root() {
    let path = std::env::temp_dir().join("bebop_store_kv_roundtrip.store");
    let path = path.to_str().unwrap();
    let mut st = Store::create(path, 1 << 20).expect("create");
    Kv::init(&mut st, path).expect("init");

    let st = Store::open(path).expect("open");
    let mut kv = Kv::load(&st).expect("load");
    assert_eq!(kv.entries.len(), 0, "a fresh KV must be empty");
    assert_eq!(kv.snapshot_root_u64(), FNV_OFFSET, "empty root is the FNV offset basis");

    for (k, v) in [
        ("order/0001", "pending"),
        ("order/0002", "confirmed"),
        ("courier/alpha", "idle"),
        ("zone/north", "{\"cap\":12}"),
        ("order/0003", "delivered"),
    ] {
        kv.put(k, v.as_bytes());
    }
    let mut st = Store::open(path).expect("reopen for write");
    kv.commit_into(&mut st, path).expect("commit");

    let st = Store::open(path).expect("reopen");
    let kv = Kv::load(&st).expect("reload");
    assert_eq!(kv.entries.len(), 5);
    assert_eq!(
        kv.keys(),
        vec!["courier/alpha", "order/0001", "order/0002", "order/0003", "zone/north"],
        "keys must come back sorted"
    );
    assert_eq!(kv.get("order/0002").unwrap(), b"confirmed");
    assert_eq!(kv.snapshot_root(), "fd11fc93f180ca47", "dowiz-core's snapshot_root");
    let _ = std::fs::remove_file(path);
}

/// Overwriting a key must change the root, and restoring the old value must restore it.
#[test]
fn root_is_sensitive_to_every_byte() {
    let path = std::env::temp_dir().join("bebop_store_kv_sensitive.store");
    let path = path.to_str().unwrap();
    let mut st = Store::create(path, 1 << 20).expect("create");
    Kv::init(&mut st, path).expect("init");
    let st = Store::open(path).expect("open");
    let mut kv = Kv::load(&st).expect("load");
    kv.put("a", b"one");
    let before = kv.snapshot_root_u64();
    kv.put("a", b"onf");
    assert_ne!(kv.snapshot_root_u64(), before, "a one-bit value change must move the root");
    kv.put("a", b"one");
    assert_eq!(kv.snapshot_root_u64(), before, "restoring the value restores the root");
    let _ = std::fs::remove_file(path);
}

/// THE CATALOGUE'S SHAPE: 165 entries of 3.2 KB, R §4 of
/// `docs/research/2026-09-28-bebop-dag.md`. Deterministic bytes (an LCG), so
/// every run and every reader sees the same image.
fn catalogue_165() -> Kv {
    let mut kv = Kv { entries: Vec::new() };
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in 0..165 {
        let v: Vec<u8> = (0..3200)
            .map(|_| {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                (x >> 56) as u8
            })
            .collect();
        kv.put(&format!("dish/{i:03}"), &v);
    }
    kv
}

/// The image `compacted_bytes_fit` writes, but in version `v`: v2 is what production
/// writes now, v1 is what every image before DG3 was (the "before" of the measurement).
fn image_v(kv: &Kv, v: i64) -> Vec<u8> {
    let mut cap = 16 * 1024;
    loop {
        let mut st = Store::create_bytes(cap).unwrap();
        let (tx, root) = Kv::stage_init_v(&mut st, v).unwrap();
        st.commit_bytes(&tx, root);
        match kv.stage_commit_into(&mut st) {
            Ok((tx, root)) => {
                st.commit_bytes(&tx, root);
                return st.to_bytes_trimmed();
            }
            Err(StoreError::ArenaFull { .. }) => cap *= 2,
            Err(e) => panic!("{e:?}"),
        }
    }
}

/// DG3's acceptance, measured: the 165 x 3.2 KB image falls from ~4.26 MB (one byte
/// per cell; R §4 measured 4,256,224 B) to <= 600,000 B, and its fold is the SAME
/// number in both, because `snapshot_root` folds bytes, not cells. The put rewrite
/// (the whole-image commit every catalogue write pays) is timed on both.
#[test]
fn v2_bytes_per_entry() {
    let kv = catalogue_165();
    let t = std::time::Instant::now();
    let v1 = image_v(&kv, 1);
    let v1_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = std::time::Instant::now();
    let v2 = kv.compacted_bytes_fit(64 << 20).expect("write");
    let v2_ms = t.elapsed().as_secs_f64() * 1e3;
    let (s1, s2) = (Store::from_bytes(&v1), Store::from_bytes(&v2));
    assert_eq!((Kv::version(&s1), Kv::version(&s2)), (1, 2), "versions");
    let (b1, b2) = (Kv::load(&s1).expect("v1 reload"), Kv::load(&s2).expect("v2 reload"));
    println!(
        "v2_bytes_per_entry: v1 image {} B ({} B/entry) put {v1_ms:.1} ms | v2 image {} B ({} B/entry) put {v2_ms:.1} ms | root {}",
        v1.len(),
        v1.len() / 165,
        v2.len(),
        v2.len() / 165,
        b2.snapshot_root()
    );
    assert_eq!(b1.snapshot_root_u64(), kv.snapshot_root_u64(), "the v1 fold moved");
    assert_eq!(b2.snapshot_root_u64(), b1.snapshot_root_u64(), "v2 fold != v1 fold");
    assert_eq!(b2.entries, kv.entries, "v2 did not give back the entries");
    assert!(v1.len() > 4_000_000, "the v1 control is not the one-byte-per-cell image: {} B", v1.len());
    assert!(v2.len() <= 600_000, "image {} B > 600,000 B", v2.len());
}

/// AN OLD IMAGE STAYS OLD. A v1 image is read, written back through `commit_into`, and
/// is still v1 with the same entries -- a mixed image would be unreadable. A fresh image
/// is v2. Lengths that are not a multiple of 8, and empty values and keys, round-trip.
#[test]
fn v1_stays_v1_and_odd_lengths_round_trip_in_v2() {
    let mut kv = Kv { entries: Vec::new() };
    for (k, v) in [("", &b""[..]), ("a", b"1234567"), ("bb", b"12345678"), ("ccc", b"123456789"), ("dë", b"")] {
        kv.put(k, v);
    }
    for v in [1, 2] {
        let img = image_v(&kv, v);
        let mut st = Store::from_bytes(&img);
        assert_eq!(Kv::version(&st), v);
        let back = Kv::load(&st).expect("reload");
        assert_eq!(back.entries, kv.entries, "v{v} round trip");
        let mut more = back;
        more.put("zz", b"tail");
        let (tx, root) = more.stage_commit_into(&mut st).expect("commit");
        st.commit_bytes(&tx, root);
        assert_eq!(Kv::version(&st), v, "commit_into changed the image's version");
        assert_eq!(Kv::load(&st).expect("reload 2").entries, more.entries);
    }
    // And the production writer makes v2.
    let fresh = kv.compacted_bytes_fit(1 << 20).unwrap();
    assert_eq!(Kv::version(&Store::from_bytes(&fresh)), 2);
}

/// A version this code does not know is REFUSED, on read and on write -- never read
/// with the nearest layout it does know.
#[test]
fn an_unknown_version_is_refused() {
    let mut kv = Kv { entries: Vec::new() };
    kv.put("k", b"value");
    let mut st = Store::from_bytes(&kv.compacted_bytes_fit(1 << 20).unwrap());
    let root = st.root().unwrap();
    st.cells[root + 2 + 5] = 3;
    assert!(Kv::load(&st).is_none(), "v3 read as if it were v2");
    assert!(kv.stage_commit_into(&mut st).is_err(), "v3 written as if it were v2");
}
