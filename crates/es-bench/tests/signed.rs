//! The signed rows measure what they say. Every refusal has its positive twin; the file is real:
//! ML-DSA-65 seals, written and read through the library's own entries. `--features mldsa`.
#![cfg(feature = "mldsa")]
use einstore::evlog::doc::{seal::{read_heads, read_seal}, Doc, DocError};
use einstore::Store;
use es_bench::archive::{formula, inputs, reader_in};
use es_bench::mldsa::{Counts, MlDsaVerifier};
use es_bench::signed::{build, check, merge, seals_for, two_sides};

const N: u64 = 64;
const VS: usize = 32;

/// An OLDER seal where there is one: the last commit's own bytes are under its anchor, so a change
/// there is refused before any signature is looked at, and the test would prove nothing of ML-DSA.
fn flip_a_signature_bit(st: &mut Store) {
    let view = Doc::open(st).unwrap();
    let head = read_heads(st, view.heads)[0].seal.unwrap();
    let at = read_seal(st, head).unwrap().prev.unwrap_or(head);
    st.put_cell(at, 20, st.get(at, 20) ^ 1);
    st.seal(at);
}

#[test]
fn the_rows_file_holds_and_counts_every_seal() {
    let (_, st) = build(N, VS, 0);
    let r = check(&st.to_bytes_trimmed(), &MlDsaVerifier).unwrap();
    assert!(r.doc.holds(), "{:?}", r.doc.counters);
    assert_eq!((r.doc.counters.records, r.doc.counters.seals_ok), (N as usize, seals_for(N)));
    assert_eq!(seals_for(N), 2);
}

/// `verify_counted` is not `verify_signed` with a cheaper name: no seal is judged.
#[test]
fn the_counting_reader_judges_no_seal() {
    let (_, st) = build(N, VS, 0);
    let r = check(&st.to_bytes_trimmed(), &Counts).unwrap();
    assert_eq!((r.doc.counters.records, r.doc.counters.seals_ok), (N as usize, 0));
    assert!(!r.doc.holds(), "a reader with no signature code proves nothing");
}

/// A signature bit flipped and the object resealed: only ML-DSA can see it, so the row's assert
/// fires under `MlDsaVerifier` and `Counts` stays blind. This is what makes the row real work.
#[test]
fn a_flipped_signature_bit_is_seen_only_by_the_real_verifier() {
    let (_, mut st) = build(N, VS, 0);
    let clean = check(&st.to_bytes_trimmed(), &Counts).unwrap().doc.counters;
    flip_a_signature_bit(&mut st);
    let bytes = st.to_bytes_trimmed();
    let r = check(&bytes, &MlDsaVerifier).unwrap();
    assert!(!r.doc.holds() && r.doc.counters.seals_bad > 0, "{:?}", r.doc.counters);
    assert_eq!(r.doc.counters.seals_bad, 1, "the one bit, the one seal");
    assert_eq!(check(&bytes, &Counts).unwrap().doc.counters, clean, "no signature code: every counter as on the clean file");
}

/// The MERGE row's whole work: the union holds, carries one MERGE, loses no record.
#[test]
fn the_merge_row_ends_in_a_file_that_holds() {
    let (mut a, ours, theirs) = two_sides(N, VS);
    let c = check(&merge(&mut a, &ours, &theirs, N, VS).unwrap(), &MlDsaVerifier).unwrap();
    assert!(c.doc.holds(), "{:?}", c.doc.counters);
    assert_eq!((c.doc.counters.records, c.doc.counters.merges, c.doc.counters.seals_ok), (N as usize + 1, 1, 3));
}

/// The row really verifies the other side: one signature bit of theirs and nothing is merged.
#[test]
fn the_merge_row_refuses_a_forged_side() {
    let (mut a, ours, mut theirs) = two_sides(N, VS);
    flip_a_signature_bit(&mut theirs);
    assert_eq!(merge(&mut a, &ours, &theirs, N, VS), Err(DocError::NotWhole { ours: false }));
}

#[test]
fn the_archive_row_refuses_a_reader_that_is_not_the_committed_one() {
    let good = "const WASM_B64 = \"AGFzbQ==\";\nconst WASM_SHA256 = \"";
    let digest = "cbed9d4a11f8c0d2c3b1d0f0f3d4e0a8b3c5c8a7d6e5f4a3b2c1d0e9f8a7b6c5"; // not its hash
    let e = reader_in(&format!("{good}{digest}\";\n")).unwrap_err();
    assert!(e.contains("hashes to"), "{e}");
    assert_eq!(reader_in("nothing here").unwrap_err(), "no WASM_B64 line");
    let (wasm, text) = inputs();
    assert_eq!(&wasm[..4], b"\0asm");
    assert!(!text.is_empty());
    assert_eq!(formula(8, 8), 8 * (10 + 4 + 4));
    assert_eq!(formula(9, 1), 8 * (10 + 5 + 4));
}
