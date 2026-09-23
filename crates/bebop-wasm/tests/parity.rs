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
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/kv.store")).expect("fixture")
}

/// `n=` and `root=` from `fixtures/kv.expected`, parsed rather than copied.
fn expected() -> (i64, i64) {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/kv.expected"))
        .expect("expected");
    let field = |k: &str| -> i64 {
        text.lines()
            .find_map(|l| l.strip_prefix(k))
            .unwrap_or_else(|| panic!("{k} missing from kv.expected"))
            .trim()
            .parse()
            .expect("an integer")
    };
    (field("n="), field("root="))
}

#[test]
fn the_fixture_reads_to_what_bebop_bin_printed() {
    let (n, root) = expected();
    let v = kv_view(&fixture()).expect("a KV image bebop.bin reads must read here");
    assert_eq!(v.n, n, "entry count");
    assert_eq!(v.root, root, "kv.bin h");
}

/// THE BIT THAT ABORTED A PROCESS: bit 33 of a key's length. `bebop-store`
/// refuses it; this layer must say so as a refusal, not a trap.
#[test]
fn a_flipped_length_bit_is_a_refusal_not_a_panic() {
    let mut st = Store::from_bytes(&fixture());
    let root = st.root().expect("root");
    let kidx = st.follow(root, 1).expect("key index");
    st.cells[kidx + 2 + 1] |= 1 << 33;
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
fn a_cut_below_the_arena_falls_back_to_the_previous_generation() {
    let bytes = fixture();
    let cut = &bytes[..bytes.len() - 8];
    let v = kv_view(cut).expect("generation 1 is still a valid KV");
    assert_eq!(v.n, 0, "generation 1 is the empty schema");
    assert_eq!(v.root, bebop_store::kv::FNV_OFFSET as i64, "the empty fold");
    // Cut below superblock B and its PartTab page (cells 512..548): nothing
    // valid remains and the reader says so.
    assert_eq!(kv_view(&bytes[..520 * 8]), Err(Refusal::NoSuperblock));
    // And bytes that are not an image at all.
    assert_eq!(kv_view(&[0u8; 64]), Err(Refusal::NoSuperblock));
    assert_eq!(kv_view(&[]), Err(Refusal::NoSuperblock));
}

#[test]
fn a_kv_image_is_not_a_log_and_a_log_is_not_a_kv() {
    assert_eq!(log_view(&fixture()), Err(Refusal::NotALog));
    let mut st = Store::create_bytes(64 * 1024);
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
    let mut st = Store::create_bytes(64 * 1024);
    EvLog::init_bytes(&mut st).expect("init");
    for (i, body) in [b"alpha".as_slice(), b"beta", b"gamma"].iter().enumerate() {
        EvLog::append_tip_bytes(&mut st, &rec(i as u8 + 1, body)).expect("append");
    }
    let full = log_view(&st.to_bytes()).expect("a fresh log reads");
    assert_eq!(full.len, 3);
    let trimmed = log_view(&st.to_bytes_trimmed()).expect("a trimmed log reads");
    assert_eq!(full, trimmed, "the fold is over records, not over zeros");

    // Change one payload byte in place: same length, same CRC field untouched,
    // so only the fold notices -- which is the point of having one.
    let root = st.root().unwrap();
    let newest = st.follow(root, 1).unwrap();
    st.cells[newest + 2 + 12] ^= 1;
    let moved = log_view(&st.to_bytes()).expect("still a log");
    assert_ne!(moved.fold, full.fold, "one byte must move the fold");
    st.cells[newest + 2 + 12] ^= 1;

    // A root that claims three while the chain is cut to two: refused, with
    // both numbers, the way `Hub::load` refuses it.
    let second = st.follow(newest, 2).unwrap();
    st.cells[second + 2 + 2] = 0;
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
