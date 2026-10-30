//! The reader answers what bebop.bin answers, refuses what it must, and the C
//! surface says the same thing as the Rust one.
//!
//! `fixtures/kv.store` is the ONE image every reader in `gate.sh` is asked
//! about: bebop created its schema (`kv.bp i`), Rust wrote five entries
//! (`kvdemo`), and `fixtures/kv.expected` records what `kv.bin n` and
//! `kv.bin h` printed for it on this box. A number here that is not in that
//! file is a number nobody else has agreed to.

use bebop_store::evlog::{EvLog, Record};
use bebop_store::Store;
use bebop_wasm::{abi, kv_view, log_view, Refusal};

fn fixture() -> Vec<u8> {
    fixture_named("kv")
}

fn fixture_named(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/fixtures/{name}.store", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

fn expected() -> (i64, i64) {
    expected_named("kv")
}

/// `n=` and `root=` from `fixtures/<name>.expected`, parsed rather than copied.
fn expected_named(name: &str) -> (i64, i64) {
    let text = std::fs::read_to_string(format!("{}/fixtures/{name}.expected", env!("CARGO_MANIFEST_DIR")))
        .expect("expected");
    let field = |k: &str| -> i64 {
        text.lines()
            .find_map(|l| l.strip_prefix(k))
            .unwrap_or_else(|| panic!("{k} missing from {name}.expected"))
            .trim()
            .parse()
            .expect("an integer")
    };
    (field("n="), field("root="))
}

/// The v1 image (one byte per cell) -- frozen: every image before DG3 is this.
/// `gate.sh` greps these two test names to say which image the native reader
/// agreed on.
#[test]
fn kv_fixture_reads_to_what_bebop_bin_printed() {
    let (n, root) = expected();
    let v = kv_view(&fixture()).expect("a KV image bebop.bin reads must read here");
    assert_eq!(v.n, n, "entry count");
    assert_eq!(v.root, root, "kv.bin h");
}

/// The v2 image (eight bytes to a cell, DG3): the same five entries as `kv.store`,
/// so the SAME root -- the fold is over bytes, not cells.
#[test]
fn kv2_fixture_reads_to_what_bebop_bin_printed() {
    let (n, root) = expected_named("kv2");
    let v = kv_view(&fixture_named("kv2")).expect("a v2 KV image must read here");
    assert_eq!(v.n, n, "entry count");
    assert_eq!(v.root, root, "kv.bin h");
    assert_eq!((n, root), expected(), "v2 and v1 of the same entries fold to one root");
    let img = fixture_named("kv2");
    let st = bebop_store::Store::from_bytes(&img);
    assert_eq!(bebop_store::kv::Kv::version(&st), 2, "kv2.store is not a v2 image");
    assert_eq!(bebop_store::kv::Kv::version(&Store::from_bytes(&fixture())), 1, "kv.store is not v1");
}

/// The v3 image (W-DELTA): kv2.store's entries + a delta chain (order/0002 changed, zone/south
/// added, courier/alpha removed). bebop.bin replays it too (kv.bp kv3_fold).
#[test]
fn kv3_fixture_reads_to_what_bebop_bin_printed() {
    let (n, root) = expected_named("kv3");
    let v = kv_view(&fixture_named("kv3")).expect("a v3 KV image must read here");
    assert_eq!((v.n, v.root), (n, root), "kv3.expected (re-derived by oracle.py)");
    assert_ne!(root, expected().1, "the chain changed nothing");
    assert_eq!(bebop_store::kv::Kv::version(&Store::from_bytes(&fixture_named("kv3"))), 3, "kv3.store is not v3");
}

/// THE BIT THAT ABORTED A PROCESS: bit 33 of a key's length. `bebop-store`
/// refuses it; this layer must say so as a refusal, not a trap.
#[test]
fn a_flipped_length_bit_is_a_refusal_not_a_panic() {
    let mut st = Store::from_bytes(&fixture());
    let root = st.root().expect("root");
    let kidx = st.follow(root, 1).expect("key index");
    st.cells[kidx + 2 + 1] |= 1 << 33;
    // W-CRC: the changed cell is refused by its crc first, by name...
    assert_eq!(kv_view(&st.to_bytes()), Err(Refusal::BadCrc { obj: kidx }));
    // ...and re-sealed (the length is a lie the writer itself sealed), the bound refuses it.
    st.seal(kidx);
    assert_eq!(kv_view(&st.to_bytes()), Err(Refusal::NotAKv));
}

/// MEASURED, AND NOT WHAT THE TEST FIRST CLAIMED. An image cut one cell below
/// the live superblock's `arena_used` does not refuse: `bebop-store::pick`
/// drops the superblock whose arithmetic no longer fits (`sb_fits`) and takes
/// the OTHER one -- generation 1, the empty KV that `kv.bp i` created. So a
/// KV image truncated in transit reads as its previous generation, silently,
/// with `n = 0` and the FNV offset basis for a root. The log family refuses
/// this through `chain_is_whole`; the KV family has no such check, and
/// `oracle.py` refuses it (status 1) where Rust falls back. The blueprint
/// records the divergence; this test pins what Rust does today so a change
/// is a red line and not a surprise.
#[test]
fn a_cut_below_the_arena_is_refused_not_read_as_the_previous_generation() {
    let bytes = fixture();
    let cut = &bytes[..bytes.len() - 8];
    // It read as generation 1 -- the empty schema, n=0 -- until the newest
    // valid superblock became the only candidate (`Store::pick`). The Python
    // reader refused these bytes all along; now both readers agree.
    assert_eq!(kv_view(cut), Err(Refusal::NoSuperblock), "a truncation is not an older image");
    // Cut below superblock B and its PartTab page (cells 512..548): nothing
    // valid remains and the reader says so.
    assert_eq!(kv_view(&bytes[..520 * 8]), Err(Refusal::NoSuperblock));
    // And bytes that are not an image at all.
    assert_eq!(kv_view(&[0u8; 64]), Err(Refusal::NoSuperblock));
    assert_eq!(kv_view(&[]), Err(Refusal::NoSuperblock));
}

/// W-F1: `fixtures/ptaway.store` is kv3 with the live superblock (B) naming a whole, sealed
/// PartTab at the OTHER page's home, 16, instead of its own, 528 (`fixtures/ptaway.expected`).
/// Every reader refuses it (FORMAT §3.1 `PartTabMisplaced`); `gate.sh` greps this name.
#[test]
fn ptaway_fixture_is_refused() {
    let bytes = fixture_named("ptaway");
    assert_eq!(kv_view(&bytes), Err(Refusal::NoSuperblock));
    let st = Store::from_bytes(&bytes);
    assert!(matches!(st.pick_checked(), Err(bebop_store::format::PickError::PartTabMisplaced { at: 512, pt: 16 })));
}

/// W-ES-P8A: `fixtures/kv4stub.store` is kv3 with its KV root's version cell (payload 5) set to 4
/// and the root re-sealed (`fixtures/kv4stub.expected`). Since W-ES-P8D this reader DECODES v4, so
/// the stub is a v4 CLAIM THAT DOES NOT HOLD (cells 1-4 of a v4 root must be zero; these name kv3's
/// arrays): NOT_A_KV, the word a v3 chain that does not hold gets. `gate.sh` greps this name. The
/// library's `Kv::load_checked` WITHOUT the `tree` feature (this crate's build) still says
/// UnknownVersion, as P8A made it. The control below puts 3 back and reads kv3.
#[test]
fn kv4stub_fixture_is_refused() {
    let bytes = fixture_named("kv4stub");
    assert_eq!(kv_view(&bytes), Err(Refusal::NotAKv));
    let st = Store::from_bytes(&bytes);
    let want = Err(bebop_store::verify::KvError::UnknownVersion { v: 4 });
    assert_eq!(bebop_store::kv::Kv::load_checked(&st).map(|_| ()), want, "Kv::load_checked");
    let view = bebop_store::View::new(&bytes);
    assert_eq!(bebop_store::kv::zc::KvIn::open(&view).map(|_| ()), want, "KvIn::open");
    assert_eq!(bebop_store::kv::zc::KvIn::open_lazy(&view).map(|_| ()), want, "KvIn::open_lazy");
    let mut out = [0i64; 2];
    assert_eq!(unsafe { abi::bw_kv(bytes.as_ptr(), bytes.len(), out.as_mut_ptr()) }, abi::NOT_A_KV);
    // The control: version 3 back in the same root, re-sealed, reads to exactly kv3.expected.
    let mut st = Store::from_bytes(&bytes);
    let root = st.root().expect("a root");
    st.cells[root + 2 + 5] = 3;
    st.seal(root);
    let v = kv_view(&st.to_bytes()).expect("the stub with version 3 back is kv3");
    assert_eq!((v.n, v.root), expected_named("kv3"), "only the version cell differs from kv3");
}

/// W-ES-P8D: the four VALUE fixtures of KV v4 (the COW B+tree, fixtures/conform/src/kv4.rs), each
/// read to exactly its `.expected`. kv4small is kv3max migrated, so the fold law across versions is
/// asserted too: the same n and root as kv3.expected's sibling kv3max. `gate.sh` greps this name.
#[test]
fn kv4_fixtures_read_to_what_the_rust_writer_folded() {
    for name in ["kv4small", "kv4deep", "kv4del", "kv4zero"] {
        let (n, root) = expected_named(name);
        let v = kv_view(&fixture_named(name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!((v.n, v.root), (n, root), "{name}");
        let (mut out, bytes) = ([0i64; 2], fixture_named(name));
        assert_eq!(unsafe { abi::bw_kv(bytes.as_ptr(), bytes.len(), out.as_mut_ptr()) }, abi::OK, "{name} abi");
        assert_eq!(out, [n, root], "{name} abi cells");
    }
    assert_eq!(expected_named("kv4small"), expected_named("kv3max"), "the fold law: migrated = v3");
    let empty = kv_view(&fixture_named("kv4zero")).expect("kv4zero");
    assert_eq!((empty.n, empty.root), expected_named("kv0"), "an empty map folds alike in every version");
}

/// W-ES-P8D: the four REFUSAL fixtures of KV v4, each refused BY NAME before anything past the
/// broken object is read: a crc (node, value) is BadCrc at that object; a structural rule (a forward
/// ref, a leaf where a node must be) is a v4 claim that does not hold, NotAKv. Per fixture the cell
/// is the one `bebop-conform why` named when the fixture was written (the .expected quotes it).
#[test]
fn kv4_corrupted_fixtures_are_refused_by_name() {
    let cases = [
        ("kv4badn", Refusal::BadCrc { obj: 3726 }, abi::BAD_CRC),
        ("kv4badv", Refusal::BadCrc { obj: 1210 }, abi::BAD_CRC),
        ("kv4badf", Refusal::NotAKv, abi::NOT_A_KV),
        ("kv4badd", Refusal::NotAKv, abi::NOT_A_KV),
    ];
    for (name, want, code) in cases {
        let bytes = fixture_named(name);
        assert_eq!(kv_view(&bytes), Err(want), "{name}");
        let mut out = [0i64; 2];
        assert_eq!(unsafe { abi::bw_kv(bytes.as_ptr(), bytes.len(), out.as_mut_ptr()) }, code, "{name} abi");
    }
}

#[test]
fn a_kv_image_is_not_a_log_and_a_log_is_not_a_kv() {
    assert_eq!(log_view(&fixture()), Err(Refusal::NotALog));
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut st).expect("init");
    assert_eq!(kv_view(&st.to_bytes()), Err(Refusal::NotAKv));
}

fn rec(n: u8, payload: &[u8]) -> Record {
    Record {
        id: [n; 32],
        prev: [n.wrapping_sub(1); 32],
        actor_pubkey: [0u8; 32],
        actor_seq: n as u64,
        payload: payload.to_vec(),
    }
}

/// The fold is over the RECORDS, so it survives a trim and moves when a
/// payload does; and a chain that lost a record is refused with both numbers.
#[test]
fn the_log_fold_is_stable_across_a_trim_and_sensitive_to_a_byte() {
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut st).expect("init");
    for (i, body) in [b"alpha".as_slice(), b"beta", b"gamma"].iter().enumerate() {
        EvLog::append_tip_bytes(&mut st, &rec(i as u8 + 1, body)).expect("append");
    }
    let full = log_view(&st.to_bytes()).expect("a fresh log reads");
    assert_eq!(full.len, 3);
    let trimmed = log_view(&st.to_bytes_trimmed()).expect("a trimmed log reads");
    assert_eq!(full, trimmed, "the fold is over records, not over zeros");

    // Change one payload byte in place: same length, CRC field untouched. Until W-CRC
    // only the fold noticed; now the crc refuses it by name. Re-sealed (a different
    // record, honestly written), the fold moves -- which is the point of having one.
    let root = st.root().unwrap();
    let newest = st.follow(root, 1).unwrap();
    st.cells[newest + 2 + 12] ^= 1;
    assert_eq!(log_view(&st.to_bytes()), Err(Refusal::BadCrc { obj: newest }));
    st.seal(newest);
    let moved = log_view(&st.to_bytes()).expect("still a log");
    assert_ne!(moved.fold, full.fold, "one byte must move the fold");
    st.cells[newest + 2 + 12] ^= 1;
    st.seal(newest);

    // A root that claims three while the chain is cut to two: refused, with
    // both numbers, the way `Hub::load` refuses it.
    let second = st.follow(newest, 2).unwrap();
    st.cells[second + 2 + 2] = 0;
    st.seal(second);
    assert_eq!(
        log_view(&st.to_bytes()),
        Err(Refusal::Truncated { claimed: 3, chained: Some(2) })
    );
}

/// The C surface: node calls exactly these, with exactly these codes.
#[test]
fn the_abi_says_what_the_rust_surface_says() {
    let (n, root) = expected();
    let bytes = fixture();
    let mut out = [0i64; 2];
    let status = unsafe { abi::bw_kv(bytes.as_ptr(), bytes.len(), out.as_mut_ptr()) };
    assert_eq!(status, abi::OK);
    assert_eq!(out, [n, root]);

    let status = unsafe { abi::bw_log(bytes.as_ptr(), bytes.len(), out.as_mut_ptr()) };
    assert_eq!(status, abi::NOT_A_LOG);

    let status = unsafe { abi::bw_kv(bytes.as_ptr(), 8, out.as_mut_ptr()) };
    assert_eq!(status, abi::NO_SUPERBLOCK);

    let status = unsafe { abi::bw_kv(core::ptr::null(), 0, out.as_mut_ptr()) };
    assert_eq!(status, abi::NULL_ARG);

    // The allocator pair round-trips a buffer the reader can use.
    let p = abi::bw_alloc(bytes.len());
    assert!(!p.is_null());
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
        assert_eq!(abi::bw_kv(p, bytes.len(), out.as_mut_ptr()), abi::OK);
        abi::bw_free(p, bytes.len());
    }
    assert_eq!(out, [n, root]);
    assert_eq!(abi::bw_abi_version(), abi::ABI_VERSION);
}

/// THE NODE KEY, NATIVE READER (DG2): `bebop_store::nodekey` builds the three
/// frames of `fixtures/key.expected` and must print exactly its numbers. The
/// line format is the one every reader in gate.sh prints; gate.sh greps these
/// three test names to count the native reader.
fn key_line_native(name: &str) -> String {
    use bebop_store::nodekey::{fixtures, hex};
    let f = fixtures::named(name).expect("a fixture key.expected names");
    format!("key {name} len={} k64={} k256={}", f.len(), f.k64(), hex(&f.k256()))
}

fn key_line_expected(name: &str) -> String {
    let text = std::fs::read_to_string(format!("{}/fixtures/key.expected", env!("CARGO_MANIFEST_DIR")))
        .expect("key.expected");
    let get = |k: &str| -> String {
        let pre = format!("{name}_{k}=");
        text.lines()
            .find_map(|l| l.strip_prefix(pre.as_str()))
            .unwrap_or_else(|| panic!("{pre} missing from key.expected"))
            .trim()
            .to_string()
    };
    format!("key {name} len={} k64={} k256={}", get("len"), get("k64"), get("k256"))
}

#[test]
fn key_compile_frame_agrees_with_key_expected() {
    use bebop_store::nodekey::fixtures::{compile, FN_SOURCE};
    // RT K-3, derived: tag + five length prefixes + the five fields' bytes.
    assert_eq!(compile().len(), 1 + 5 * 8 + 8 + FN_SOURCE.len() + 8 + 8 + 8);
    assert_eq!(key_line_native("compile"), key_line_expected("compile"));
}

#[test]
fn key_proj_frame_agrees_with_key_expected() {
    assert_eq!(key_line_native("proj"), key_line_expected("proj"));
}

#[test]
fn key_empty_frame_agrees_with_key_expected() {
    assert_eq!(key_line_native("empty"), key_line_expected("empty"));
}

/// The wasm32 reader's frames, asked natively too: the same numbers through the
/// C surface `harness.mjs --key` calls (gate.sh runs the wasm32 build of it).
#[test]
fn bw_key_cells_agree_with_the_native_frames() {
    use bebop_store::nodekey::fixtures;
    for (i, name) in ["compile", "proj", "empty"].iter().enumerate() {
        let f = fixtures::named(name).unwrap();
        let c = bebop_wasm::nodekey::cells(i as i32).unwrap();
        let mut d = [0u8; 32];
        for w in 0..4 {
            d[8 * w..8 * w + 8].copy_from_slice(&c[2 + w].to_be_bytes());
        }
        assert_eq!((c[0] as usize, c[1], d), (f.len(), f.k64(), f.k256()), "{name}");
    }
}
