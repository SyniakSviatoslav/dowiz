//! SPEC: first object, found without a parser, the blobs round-trip, the cost is the blobs
//! plus three headers, a working DOC is untouched, and every claim has its RED.

use super::super::tests::{doc_with, key_of, Dev, ToyIds, ToySigner, ToyVerifier};
use super::super::verify::{verify, Trust};
use super::super::{Doc, DocError, Ids, FLAG_SELF_DESCRIBING};
use super::{read_blob, read_spec, SpecIn, DIGEST_BLOB, DIGEST_SPEC, LAYOUT_BLOB, LAYOUT_SPEC, SPEC_CELLS};
use crate::{Record, Store, ARENA, FLAG_SHA256};
use dowiz_core::sha256_hw::sha256_hw as sha256;

fn st_digest(layout: &str) -> i64 {
    let d = sha256(layout.as_bytes());
    i64::from(u32::from_be_bytes([d[28], d[29], d[30], d[31]]))
}

/// A stand-in reader (any bytes; the format does not look inside) and a stand-in format text.
fn spec_in<'a>(wasm: &'a [u8], text: &'a [u8]) -> SpecIn<'a> {
    SpecIn { wasm, wasm_sha256: sha256(wasm), text, abi: 1 }
}
fn fake_wasm(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 7 % 251) as u8).collect()
}

fn described(n: usize, wasm: &[u8], text: &[u8]) -> (Store, Dev<ToySigner>) {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let mut owner = Dev::new(ToySigner { id: [0xA1; 32] });
    Doc::init_described_bytes(&mut st, 0, &[key_of(&owner.s)], &spec_in(wasm, text)).unwrap();
    for i in 0..n {
        owner.commit(&mut st, format!("event-{i}").as_bytes(), None).unwrap();
    }
    (st, owner)
}

#[test]
fn the_two_digests_are_the_layouts_sha256_low_32_bits() {
    assert_eq!(st_digest(LAYOUT_SPEC), DIGEST_SPEC);
    assert_eq!(st_digest(LAYOUT_BLOB), DIGEST_BLOB);
}

#[test]
fn spec_is_the_first_object_and_the_blobs_round_trip() {
    let wasm = fake_wasm(1001);
    let text = b"FORMAT text, 37 bytes of it, exactly.";
    let (st, _) = described(3, &wasm, text);
    assert_eq!(Doc::spec_first(&st), Some(ARENA), "SPEC sits at cell 1024");
    let s = Doc::spec(&st).expect("the DOC names its SPEC");
    assert_eq!(s.at, ARENA);
    assert_eq!(s.abi, 1);
    assert_eq!(s.wasm_sha256, sha256(&wasm));
    assert_eq!(read_blob(&st, s.wasm.unwrap()).as_deref(), Some(wasm.as_slice()));
    assert_eq!(read_blob(&st, s.text.unwrap()).as_deref(), Some(&text[..]));
    let view = Doc::open(&st).unwrap();
    assert_eq!(view.flags & FLAG_SELF_DESCRIBING, FLAG_SELF_DESCRIBING, "bit 2 says self-describing");
    assert_eq!(view.spec, Some(ARENA));
}

/// THE BOOTSTRAP OVER RAW BYTES, as `examples/read.mjs` does it: no `Store`, six reads.
#[test]
fn a_host_with_no_parser_finds_the_reader_by_the_documented_six_lines() {
    let wasm = fake_wasm(777);
    let (st, _) = described(1, &wasm, b"t");
    let b = st.to_bytes_trimmed();
    let cell = |i: usize| i64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap());
    assert_eq!((cell(ARENA) as u64 >> 32) as i64, DIGEST_SPEC, "h0's high half names SPEC");
    assert_eq!(cell(ARENA) & 0xFFFF_FFFF, SPEC_CELLS as i64);
    let w = (ARENA as i64 + cell(ARENA + 2 + 1)) as usize;
    let len = cell(w + 2) as usize;
    assert_eq!(len, wasm.len());
    assert_eq!(&b[(w + 3) * 8..(w + 3) * 8 + len], wasm.as_slice());
}

#[test]
fn a_described_doc_verifies_exactly_as_a_plain_one() {
    let (plain, _) = doc_with(3);
    let (desc, _) = described(3, &fake_wasm(64), b"text");
    let a = verify(&plain, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap();
    let b = verify(&desc, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap();
    assert_eq!(a.counters, b.counters);
    assert_eq!(b.counters.seals_ok, 3);
    assert!(b.holds());
}

/// §2f's price, to the byte: SPEC (2 + 8 cells) and two blobs (2 + 1 + ceil(len / 8) each).
#[test]
fn the_cost_of_self_description_is_the_blobs_plus_three_headers() {
    let (wasm, text) = (fake_wasm(65_000), vec![b'f'; 30_001]);
    let (plain, _) = doc_with(2);
    let (desc, _) = described(2, &wasm, &text);
    let cost = desc.to_bytes_trimmed().len() - plain.to_bytes_trimmed().len();
    let want = 8 * ((2 + SPEC_CELLS) + (2 + 1 + wasm.len().div_ceil(8)) + (2 + 1 + text.len().div_ceil(8)));
    assert_eq!(cost, want, "cost of SPEC = {cost} bytes for a {}-byte reader and {}-byte text", wasm.len(), text.len());
}

#[test]
fn a_working_image_has_no_spec_and_a_used_store_is_refused() {
    let (mut st, owner) = doc_with(1);
    assert_eq!(Doc::spec(&st), None);
    assert_eq!(Doc::spec_first(&st), None);
    let err = Doc::init_described_bytes(&mut st, 0, &[key_of(&owner.s)], &spec_in(b"w", b"t")).err();
    assert_eq!(err, Some(DocError::Corrupt("SPEC must be the first object: the store is not fresh")));
}

/// RED: one byte of the reader blob flipped -> the blob's crc fails, `spec_first` is `None`;
/// SPEC's own digest flipped -> `read_spec` is `None`; a damaged length claim is bounded.
#[test]
fn a_flipped_reader_byte_and_a_wrong_digest_are_refused_by_name() {
    let wasm = fake_wasm(300);
    let (st, _) = described(1, &wasm, b"t");
    let s = Doc::spec(&st).unwrap();
    let w = s.wasm.unwrap();
    let mut cells = st.to_bytes_trimmed();
    cells[(w + 3) * 8 + 17] ^= 0x10;
    let bad = Store::from_bytes(&cells);
    assert_eq!(read_blob(&bad, w), None, "a flipped reader byte fails the blob's crc");
    assert_eq!(Doc::spec_first(&bad), None);
    assert!(Doc::open(&bad).is_ok(), "the DOC itself still opens: the blob is not on its reachable path");

    let mut cells = st.to_bytes_trimmed();
    cells[ARENA * 8 + 4] ^= 1;
    let bad = Store::from_bytes(&cells);
    assert_eq!(read_spec(&bad, ARENA), None);
    assert_eq!(Doc::spec_first(&bad), None);

    let mut cells = st.to_bytes_trimmed();
    cells[(w + 2) * 8..(w + 3) * 8].copy_from_slice(&(1i64 << 40).to_le_bytes());
    let bad = Store::from_bytes(&cells);
    assert_eq!(read_blob(&bad, w), None, "a length past MAX_BLOB_BYTES is refused, never allocated");
}

/// Real sha256 ids (dowiz-hub's scheme, `chain/scheme.rs`): `sha256(prev ‖ payload)`, the
/// payload led by a 16-byte nonce, `FLAG_SHA256` set. The archive fixture is written with it
/// so the embedded reader counts `chained`, not `broken`, on a file written the way dowiz writes.
struct ShaIds;
impl Ids for ShaIds {
    fn chained(&self, prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
        let mut m = prev.to_vec();
        m.extend_from_slice(payload);
        sha256(&m)
    }
}

/// `crates/bebop-wasm/fixtures/archive.store` -- NOT committed (it embeds the reader and would
/// churn with it): tools/verify/gate.sh writes it when asked, with the reader just built:
/// `BEBOP_ARCHIVE_FIXTURE=<out> BEBOP_READER_WASM=<wasm> BEBOP_FORMAT_TEXT=<FORMAT.md>`.
/// Deterministic: a re-run with the same inputs is byte-identical. One device, three
/// sha256-chained, toy-sealed commits; the doc.store fixture's shape with the real id scheme.
#[test]
fn write_the_archive_fixture_when_asked() {
    let wasm = std::env::var("BEBOP_READER_WASM").ok().map(|p| std::fs::read(p).expect("reader wasm")).unwrap_or_else(|| fake_wasm(2048));
    let text = std::env::var("BEBOP_FORMAT_TEXT").ok().map(|p| std::fs::read(p).expect("format text")).unwrap_or_else(|| b"FORMAT".to_vec());
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let owner = ToySigner { id: [0xA1; 32] };
    Doc::init_described_bytes(&mut st, 0, &[key_of(&owner)], &spec_in(&wasm, &text)).unwrap();
    let mut tip = [0u8; 32];
    for i in 0..3u64 {
        let mut payload = format!("nonce-{i:0>10}").into_bytes();
        payload.extend_from_slice(format!("event-{i}").as_bytes());
        let id = ShaIds.chained(&tip, &payload);
        let r = Record { id, prev: tip, actor_pubkey: owner.id, actor_seq: i + 1, flags: FLAG_SHA256, payload };
        Doc::commit_bytes(&mut st, &r, &owner, 1_700_000_000_000 + i as i64, None).unwrap();
        tip = id;
    }
    let rep = verify(&st, &ShaIds, &ToyVerifier, Trust::Genesis).unwrap();
    assert_eq!((rep.counters.records, rep.counters.chained, rep.counters.broken, rep.counters.seals_ok), (3, 3, 0, 3));
    assert_eq!(Doc::spec_first(&st), Some(ARENA));
    if let Ok(p) = std::env::var("BEBOP_ARCHIVE_FIXTURE") {
        std::fs::write(&p, st.to_bytes_trimmed()).unwrap();
    }
}

/// `archive_bytes` is the sentence "an archive is a copy with SPEC first": a working image goes
/// in, SPEC lands at the arena's first cell, every record comes out verbatim, and an archive
/// is not archived again.
#[test]
fn an_archive_is_a_verbatim_copy_with_spec_first() {
    let (src, _owner) = super::super::tests::doc_with(3);
    let mut dst = Store::create_bytes(1 << 20).unwrap();
    Doc::archive_bytes(&src, &mut dst, &spec_in(b"\0asm", b"format")).unwrap();
    let view = Doc::open(&dst).unwrap();
    assert_eq!(view.spec, Some(crate::ARENA), "SPEC is not the first object");
    assert_ne!(view.flags & FLAG_SELF_DESCRIBING, 0);
    assert_eq!(
        crate::EvLog::walk_until_at(&dst, view.log, |_| false),
        crate::EvLog::walk_until_at(&src, Doc::open(&src).unwrap().log, |_| false)
    );
    let mut again = Store::create_bytes(1 << 20).unwrap();
    assert!(Doc::archive_bytes(&dst, &mut again, &spec_in(b"\0asm", b"format")).is_err());
}

/// RFC 9162 Merkle tree hash over the cut ids (dowiz-hub's `bebop_store::merkle` shape: leaf =
/// sha256(0x00 ‖ id), node = sha256(0x01 ‖ l ‖ r), the split at the largest power of two below n),
/// so `cut_into` can name a segment by a root a stranger can recompute.
fn mth(ids: &[[u8; 32]]) -> [u8; 32] {
    match ids.len() {
        0 => sha256(b""),
        1 => {
            let mut m = vec![0u8];
            m.extend_from_slice(&ids[0]);
            sha256(&m)
        }
        n => {
            let mut k = 1;
            while k * 2 < n {
                k *= 2;
            }
            let mut m = vec![1u8];
            m.extend_from_slice(&mth(&ids[..k]));
            m.extend_from_slice(&mth(&ids[k..]));
            sha256(&m)
        }
    }
}
struct ShaTreeIds;
impl Ids for ShaTreeIds {
    fn chained(&self, prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
        ShaIds.chained(prev, payload)
    }
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        Some(mth(ids))
    }
    /// dowiz-hub's framing: a FLAG_SHA256 payload is led by the 16-byte nonce; the TRUNCATE body
    /// sits after it (the reader's `ids.rs` strips the same sixteen bytes).
    fn body<'a>(&self, flags: i64, payload: &'a [u8]) -> &'a [u8] {
        if flags & FLAG_SHA256 != 0 {
            payload.get(16..).unwrap_or(&[])
        } else {
            payload
        }
    }
}

/// One sha256-id, nonce-led, FLAG_SHA256 commit of `body` by `owner` at `seq`; returns the new tip.
fn sha_commit(st: &mut Store, owner: &ToySigner, tip: [u8; 32], seq: u64, body: &[u8]) -> [u8; 32] {
    let mut payload = format!("nonce-{seq:0>10}").into_bytes();
    payload.extend_from_slice(body);
    let id = ShaIds.chained(&tip, &payload);
    let r = Record { id, prev: tip, actor_pubkey: owner.id, actor_seq: seq, flags: FLAG_SHA256, payload };
    Doc::commit_bytes(st, &r, owner, 1_700_000_000_000 + seq as i64, None).unwrap();
    id
}

/// THE CUT FIXTURES (W-MG-M5b), written when asked, deterministic: a working DOC with five sha256
/// commits is cut to its newest two (`Doc::cut_into`), the owner commits the TRUNCATE (seq 6), and
/// the result is ARCHIVED with SPEC first (`Doc::archive_bytes`) -> `BEBOP_CUT_FIXTURE`. The twin
/// `BEBOP_CUT_UNDECLARED_FIXTURE` is the same cut archived BEFORE the TRUNCATE: a segment nobody
/// declared, which every reader must refuse by name (`CutUndeclared` / status 75).
#[test]
fn write_the_cut_fixtures_when_asked() {
    let wasm = std::env::var("BEBOP_READER_WASM").ok().map(|p| std::fs::read(p).expect("reader wasm")).unwrap_or_else(|| fake_wasm(2048));
    let text = std::env::var("BEBOP_FORMAT_TEXT").ok().map(|p| std::fs::read(p).expect("format text")).unwrap_or_else(|| b"FORMAT".to_vec());
    let owner = ToySigner { id: [0xA1; 32] };
    let mut st = Store::create_bytes(1 << 20).unwrap();
    Doc::init_bytes(&mut st, 0, &[key_of(&owner)]).unwrap();
    let mut tip = [0u8; 32];
    for i in 1..=5u64 {
        tip = sha_commit(&mut st, &owner, tip, i, format!("event-{i}").as_bytes());
    }
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 2, &ShaTreeIds).unwrap();
    assert_eq!((seg.first, seg.last, seg.count()), (0, 2, 3));
    let mut undeclared = Store::create_bytes(1 << 20).unwrap();
    Doc::archive_bytes(&fresh, &mut undeclared, &spec_in(&wasm, &text)).unwrap();
    assert_eq!(verify(&undeclared, &ShaTreeIds, &ToyVerifier, Trust::Genesis).err(), Some(DocError::CutUndeclared { root: seg.root }));

    sha_commit(&mut fresh, &owner, tip, 6, &seg.truncate_body());
    let mut declared = Store::create_bytes(1 << 20).unwrap();
    Doc::archive_bytes(&fresh, &mut declared, &spec_in(&wasm, &text)).unwrap();
    let c = verify(&declared, &ShaTreeIds, &ToyVerifier, Trust::Genesis).unwrap().counters;
    assert_eq!((c.records, c.chained, c.broken, c.seals_ok, c.seals_bad), (3, 3, 0, 3, 0), "{c:?}");
    assert_eq!((c.segments, c.cut, c.cut_redacted, c.declared), (1, 3, 0, 0), "{c:?}");
    assert_eq!(Doc::spec_first(&declared), Some(ARENA));
    if let Ok(p) = std::env::var("BEBOP_CUT_FIXTURE") {
        std::fs::write(&p, declared.to_bytes_trimmed()).unwrap();
    }
    if let Ok(p) = std::env::var("BEBOP_CUT_UNDECLARED_FIXTURE") {
        std::fs::write(&p, undeclared.to_bytes_trimmed()).unwrap();
    }
}
