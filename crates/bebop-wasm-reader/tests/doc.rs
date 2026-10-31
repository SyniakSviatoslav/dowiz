// SPDX-License-Identifier: MIT OR Apache-2.0
//! The DOC through this reader (W-MG-M5): the committed `doc` fixture (toy ids: counted
//! `broken`, honestly), a DOC written in memory with sha256 ids and a SPEC (counted `chained`,
//! its reader blob found by the six-line bootstrap), the ABI's three new calls, and a RED per
//! claim. The sha256 is held to FIPS 180-4's vectors first: everything else leans on it.

use bebop_image::{Store, ARENA};
use bebop_log::doc::spec::{SpecIn, DIGEST_SPEC};
use bebop_log::doc::{Doc as DocRoot, Key, Signer};
use bebop_log::{Record, FLAG_SHA256};
use bebop_wasm_reader::abi::*;
use bebop_wasm_reader::sha256::sha256;
use bebop_wasm_reader::{open, status, Kind, Refusal};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn sha256_matches_the_fips_vectors() {
    assert_eq!(hex(&sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(hex(&sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")), "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    assert_eq!(hex(&sha256(&[b'a'; 1000])), "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3");
    // 55 and 56 bytes: the two padding branches
    assert_eq!(hex(&sha256(&[0u8; 55])), "02779466cdec163811d078815c633f21901413081449002f24aa3e80f0b88ef7");
    assert_eq!(hex(&sha256(&[0u8; 56])), "d4817aa5497628e7c77e6b606107042bbba3130888c5f47a375e6179be789fbb");
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../bebop-wasm/fixtures/{name}.store", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

/// The committed fixture: a DOC root over three TOY-id records (bebop-log's test mix, not
/// sha256), one head, one genesis key, three toy seals. This reader opens it (kind 5, n 3)
/// and counts what it finds: three records, none under a scheme it knows -> `broken 3`; three
/// seals counted, none checked. The honest line about a toy file.
#[test]
fn the_doc_fixture_opens_and_is_counted_honestly() {
    let mut d = open(&fixture("doc")).expect("doc.store opens");
    assert_eq!(d.kind, Kind::Doc);
    assert_eq!(d.len(), 3);
    let expected = std::fs::read_to_string(format!("{}/../bebop-wasm/fixtures/doc.expected", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let want_root: i64 = expected.lines().find_map(|l| l.strip_prefix("doc_root=")).unwrap().trim().parse().unwrap();
    assert_eq!(d.fold, want_root, "the LOG fold doc.expected pins (python wrote it first)");
    assert_eq!(d.heads.len(), 1);
    assert_eq!((d.heads[0].seq, d.heads[0].tip), (3, d.records[2].id));
    assert_eq!(d.spec, None, "a working image has no SPEC");
    let c = d.verify_doc().unwrap().unwrap();
    assert_eq!((c.records, c.chained, c.legacy, c.redacted, c.broken), (3, 0, 0, 0, 3));
    assert_eq!((c.seals, c.forks, c.heads, c.keys, c.admitted, c.checked, c.holds), (3, 0, 1, 1, 1, 0, 0));
    assert_eq!((c.segments, c.cut, c.cut_redacted, c.declared), (0, 0, 0, 0), "a working image: nothing cut");
    assert!(d.anchors.is_empty());
    assert!(d.records.windows(2).all(|w| w[0].actor_seq < w[1].actor_seq), "records come oldest first");
}

struct Toy;
impl Signer for Toy {
    fn actor_id(&self) -> [u8; 32] {
        [0xB2; 32]
    }
    fn alg(&self) -> i64 {
        1
    }
    fn public_key(&self) -> Vec<u8> {
        vec![0xB2; 32]
    }
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        sha256(msg).to_vec()
    }
}

/// A self-describing DOC with dowiz-hub's id scheme (`sha256(prev || payload)`, 16-byte nonce,
/// FLAG_SHA256), three commits, `wasm` embedded as its reader.
fn described(wasm: &[u8], text: &[u8]) -> Vec<u8> {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let key = Key { actor_id: Toy.actor_id(), alg: 1, valid_from: 0, revoked_at: 0, admit_rec: None, pk: Toy.public_key() };
    let spec = SpecIn { wasm, wasm_sha256: sha256(wasm), text, abi: ABI_VERSION as i64 };
    DocRoot::init_described_bytes(&mut st, 0, &[key], &spec).unwrap();
    let mut tip = [0u8; 32];
    for i in 0..3u64 {
        let payload = format!("nonce-{i:0>10}event-{i}").into_bytes();
        let mut m = tip.to_vec();
        m.extend_from_slice(&payload);
        let id = sha256(&m);
        let r = Record { id, prev: tip, actor_pubkey: Toy.actor_id(), actor_seq: i + 1, flags: FLAG_SHA256, payload };
        DocRoot::commit_bytes(&mut st, &r, &Toy, 1_700_000_000_000 + i as i64, None).unwrap();
        tip = id;
    }
    st.to_bytes_trimmed()
}

#[test]
fn a_described_doc_with_sha256_ids_is_chained_and_its_reader_is_found() {
    let wasm: Vec<u8> = (0..1500u32).map(|i| (i * 13 % 256) as u8).collect();
    let text = b"the format text";
    let b = described(&wasm, text);
    let mut d = open(&b).expect("opens");
    assert_eq!(d.kind, Kind::Doc);
    let s = d.spec.expect("SPEC named");
    assert_eq!((s.at, s.abi, s.wasm_sha256), (ARENA, ABI_VERSION as i64, sha256(&wasm)));
    assert_eq!(d.spec_wasm().as_deref(), Some(wasm.as_slice()));
    assert_eq!(d.spec_text().as_deref(), Some(&text[..]));
    let c = d.verify_doc().unwrap().unwrap();
    assert_eq!((c.records, c.chained, c.broken, c.seals, c.checked, c.holds), (3, 3, 0, 3, 0, 0), "a toy seal is never ok to a reader with no verifier");
    // the six-line bootstrap over the bytes, as examples/read.mjs does it
    let cell = |i: usize| i64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap());
    assert_eq!((cell(ARENA) as u64 >> 32) as i64, DIGEST_SPEC);
    let w = (ARENA as i64 + cell(ARENA + 3)) as usize;
    let len = cell(w + 2) as usize;
    assert_eq!(&b[(w + 3) * 8..(w + 3) * 8 + len], wasm.as_slice());
}

/// RED, one bit each: a record payload -> BAD_CRC naming the record (open refuses); HEADS ->
/// BAD_CRC naming the table; the reader blob -> the DOC still opens and `spec_wasm` is None
/// (a host must then say the file's reader does not read, never run a different one).
#[test]
fn one_flipped_bit_is_refused_by_name() {
    let wasm = vec![7u8; 300];
    let clean = described(&wasm, b"t");
    assert!(open(&clean).is_ok());
    let cell = |b: &[u8], i: usize| i64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap());
    // the newest record: the DOC root's LOG ref (cell 1) -> log root's LAST ref (cell 1)
    let sb = [0usize, 512].into_iter().filter(|&at| cell(&clean, at) == 3554557610294396226).max_by_key(|&at| cell(&clean, at + 2)).unwrap();
    let pt = cell(&clean, sb + 3) as usize;
    let doc = cell(&clean, pt + 18) as usize;
    let log = (doc as i64 + cell(&clean, doc + 2 + 1)) as usize;
    let rec = (log as i64 + cell(&clean, log + 2 + 1)) as usize;
    let mut b = clean.clone();
    b[8 * (rec + 2 + 12 + 4)] ^= 1; // first payload cell (after flags and the 4 actor cells)
    assert_eq!(open(&b).err(), Some(Refusal::BadCrc { obj: rec }), "the record is named");

    let heads = (doc as i64 + cell(&clean, doc + 2 + 3)) as usize;
    let mut b = clean.clone();
    b[8 * (heads + 2 + 5)] ^= 1; // the head's seq
    assert_eq!(open(&b).err(), Some(Refusal::BadCrc { obj: heads }), "HEADS is named");

    let w = (ARENA as i64 + cell(&clean, ARENA + 3)) as usize;
    let mut b = clean.clone();
    b[(w + 3) * 8 + 100] ^= 1;
    let d = open(&b).expect("the DOC opens: the blob is off its reachable path");
    assert_eq!(d.spec_wasm(), None, "but the reader it embeds does not read");
}

/// RED FOR THE COUNTER ITSELF: a forger edits the OLDEST record's payload and RE-SEALS its crc.
/// The bytes are whole (open says OK, the whole arena verifies), but the id no longer hashes:
/// `chained` 3 -> 2, `broken` 0 -> 1, `holds` 0. A reader that still said chained=3 would call a
/// broken file healthy. The oldest record sits outside the last commit's anchor range, so it is
/// the counter and nothing else that sees this.
#[test]
fn a_forged_record_with_a_resealed_crc_is_counted_broken_not_refused() {
    let clean = described(&[9u8; 40], b"t");
    let mut st = Store::from_bytes(&clean);
    let cell = |b: &[u8], i: usize| i64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap());
    let sb = [0usize, 512].into_iter().filter(|&at| cell(&clean, at) == 3554557610294396226).max_by_key(|&at| cell(&clean, at + 2)).unwrap();
    let doc = cell(&clean, cell(&clean, sb + 3) as usize + 18) as usize;
    let log = (doc as i64 + cell(&clean, doc + 2 + 1)) as usize;
    let mut old = (log as i64 + cell(&clean, log + 2 + 1)) as usize;
    while cell(&clean, old + 2 + 2) != 0 {
        old = (old as i64 + cell(&clean, old + 2 + 2)) as usize;
    }
    st.put_cell(old, 16, st.get(old, 16) ^ 1);
    st.seal(old);
    let forged = st.to_bytes();
    let mut d = open(&forged).expect("whole bytes open: the forgery is not a crc failure");
    assert_eq!(d.verify_arena(), Ok(()));
    let c = d.verify_doc().unwrap().unwrap();
    assert_eq!((c.records, c.chained, c.broken, c.holds), (3, 2, 1, 0), "the edited record no longer hashes to its id");
}

/// THE ABI, natively, for a DOC: kind 5, the log calls answer, the KV calls answer NOT_FOUND
/// (no tree), heads rows, counts cells, spec frame, the two texts, and NOT_A_DOC on a KV.
#[test]
fn the_doc_abi_round_trips_natively() {
    unsafe {
        let wasm = vec![3u8; 64];
        let img = described(&wasm, b"FORMAT");
        assert_eq!(bw_open(img.as_ptr(), img.len()), status::OK);
        assert_eq!(bw_kind(), KIND_DOC);
        assert_eq!(bw_len(), 3);
        assert_eq!(bw_log_len(), 3);
        let mut buf = vec![0u8; 8192];
        let n = bw_log_at(0, buf.as_mut_ptr(), buf.len()) as usize;
        let plen = u64::from_le_bytes(buf[8..16].try_into().unwrap()) as usize;
        assert_eq!(n, 112 + plen);
        assert_eq!(bw_get(b"x".as_ptr(), 1, buf.as_mut_ptr(), buf.len()), -(status::NOT_FOUND as i64));
        assert_eq!(bw_scan_next(0, buf.as_mut_ptr(), buf.len()), 0);
        let n = bw_heads(buf.as_mut_ptr(), buf.len()) as usize;
        assert_eq!(n, HEAD_ROW);
        assert_eq!(&buf[..32], &[0xB2u8; 32]);
        assert_eq!(u64::from_le_bytes(buf[32..40].try_into().unwrap()), 3);
        assert_eq!(bw_verify(1), status::OK);
        assert_eq!(bw_verify(2), status::OK);
        let n = bw_counts(buf.as_mut_ptr(), buf.len()) as usize;
        assert_eq!(n, 8 * COUNTS_CELLS);
        let c: Vec<i64> = (0..COUNTS_CELLS).map(|i| i64::from_le_bytes(buf[8 * i..8 * i + 8].try_into().unwrap())).collect();
        assert_eq!(c, vec![3, 3, 0, 0, 0, 3, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0], "twelve M1 cells, four M3/M4 cells appended");
        buf[0] = 0xAA;
        assert_eq!(bw_counts(buf.as_mut_ptr(), 8), 8 * COUNTS_CELLS as i64, "too small: the length, nothing written");
        assert_eq!(buf[0], 0xAA);
        let n = bw_spec(buf.as_mut_ptr(), buf.len()) as usize;
        assert_eq!(n, 40 + 6);
        assert_eq!(i64::from_le_bytes(buf[..8].try_into().unwrap()), ABI_VERSION as i64);
        assert_eq!(&buf[8..40], &sha256(&wasm));
        assert_eq!(&buf[40..46], b"FORMAT");
        let n = bw_unchecked_text(buf.as_mut_ptr(), buf.len()) as usize;
        assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), status::UNCHECKED);
        assert_eq!(bw_status_text(status::NOT_A_DOC, buf.as_mut_ptr(), buf.len()) > 0, true);

        // a KV: the DOC calls answer by name
        let kv = fixture("kv3");
        assert_eq!(bw_open(kv.as_ptr(), kv.len()), status::OK);
        assert_eq!(bw_counts(buf.as_mut_ptr(), buf.len()), -(status::NOT_A_DOC as i64));
        assert_eq!(bw_heads(buf.as_mut_ptr(), buf.len()), 0);
        assert_eq!(bw_spec(buf.as_mut_ptr(), buf.len()), 0);
        assert_eq!(bw_anchors(buf.as_mut_ptr(), buf.len()), 0, "a KV has no anchors");
        // the committed fixture through the ABI: a working image, no SPEC
        let doc = fixture("doc");
        assert_eq!(bw_open(doc.as_ptr(), doc.len()), status::OK);
        assert_eq!(bw_kind(), KIND_DOC);
        assert_eq!(bw_spec(buf.as_mut_ptr(), buf.len()), 0);
        bw_close();
    }
}
