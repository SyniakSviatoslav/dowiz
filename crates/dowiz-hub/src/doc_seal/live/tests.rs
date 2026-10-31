//! Kill tests of the live level-1 anchors (W-MG-M6b, R-MAINGOAL-A §7 row M6), every calendar a
//! FAKE in the process: a changed cell -> refusal; a pending calendar -> `Unchecked`, never ok;
//! an offline run writes no ANCH; two calendars merge into one proof and a dead one is named;
//! an upgraded proof attests a block and checks against a headers line; the headers file parses
//! and refuses by line. Nothing here reaches a network -- the live run is `bin/anchor` on a box.

use super::stamp::{bitcoin_heights, pending_uris, stamp_image, stamp_multi, upgrade_file, upgrade_image, Upgraded};
use super::{describe, header_line_from_explorer, verify_image, verify_ots, FileHeaders};
use crate::doc_seal::ots::{self, Attestation, BlockHeaders, Calendar, Op, Proof, Ts};
use crate::doc_seal::{DeviceKey, SealedHub};
use crate::{EventKind, HubError};
use bebop_store::evlog::doc::{Doc, Verdict};
use bebop_store::nodekey::sha256;
use bebop_store::Store;

const JSON: &str = r#"{"id":"o1","total":300}"#;
const A: &str = "https://a.test.calendar";
const B: &str = "https://b.test.calendar";
const DEAD: &str = "https://c.test.calendar";

/// A serialized BARE timestamp (what a calendar answers): a file without its 65-byte head.
fn bare(ts: Ts, msg: &[u8]) -> Vec<u8> {
    ots::serialize(&Proof { digest: msg.to_vec(), ts }).unwrap()[65..].to_vec()
}

/// One fake calendar: append its nonce, sha256, pending at its uri; when `block` is set, an
/// upgrade answers prepend "salt", sha256, Bitcoin attestation at that height.
struct Fake {
    uri: &'static str,
    nonce: &'static [u8],
    block: Option<u64>,
}

impl Fake {
    fn commitment(&self, digest: &[u8]) -> [u8; 32] {
        sha256(&[digest, self.nonce].concat())
    }
    /// The Merkle root the upgraded branch reaches: what a headers line must carry.
    fn root(&self, digest: &[u8]) -> [u8; 32] {
        sha256(&[b"salt".as_slice(), &self.commitment(digest)].concat())
    }
}

struct Pool(Vec<Fake>);

impl Calendar for Pool {
    fn submit(&self, calendar: &str, digest: &[u8; 32]) -> Result<Vec<u8>, String> {
        let f = self.0.iter().find(|f| f.uri == calendar).ok_or(format!("{calendar}: connection refused"))?;
        let pending = Ts { attestations: vec![Attestation::Pending { uri: f.uri.to_string() }], ops: vec![] };
        let ts = Ts { attestations: vec![], ops: vec![(Op::Append(f.nonce.to_vec()), Ts { attestations: vec![], ops: vec![(Op::Sha256, pending)] })] };
        Ok(bare(ts, digest))
    }
    fn upgrade(&self, uri: &str, msg: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let f = self.0.iter().find(|f| f.uri == uri).ok_or(format!("{uri}: connection refused"))?;
        Ok(f.block.map(|h| {
            let at = Ts { attestations: vec![Attestation::Bitcoin { height: h }], ops: vec![] };
            bare(Ts { attestations: vec![], ops: vec![(Op::Prepend(b"salt".to_vec()), Ts { attestations: vec![], ops: vec![(Op::Sha256, at)] })] }, msg)
        }))
    }
}

/// No network at all: what `curl` exit 6 looks like through the seam.
struct Offline;
impl Calendar for Offline {
    fn submit(&self, c: &str, _: &[u8; 32]) -> Result<Vec<u8>, String> {
        Err(format!("{c}: curl exit 6: could not resolve host"))
    }
    fn upgrade(&self, uri: &str, _: &[u8]) -> Result<Option<Vec<u8>>, String> {
        Err(format!("{uri}: curl exit 6: could not resolve host"))
    }
}

fn pool(block_a: Option<u64>) -> Pool {
    Pool(vec![Fake { uri: A, nonce: b"nonce-a", block: block_a }, Fake { uri: B, nonce: b"nonce-b", block: None }])
}

fn sealed(key: &DeviceKey) -> Vec<u8> {
    let mut sh = SealedHub::create(key).unwrap();
    sh.append(EventKind::Placed, "o1", JSON, key, 1_730_000_000_000).unwrap();
    sh.append(EventKind::Advanced, "o1", JSON, key, 1_730_000_001_000).unwrap();
    sh.to_bytes()
}

#[test]
fn two_calendars_merge_into_one_proof_and_a_dead_one_is_named() {
    let digest = sha256(b"a test digest, never customer data");
    let st = stamp_multi(&pool(None), &[A, B, DEAD], &digest).unwrap();
    assert_eq!(st.replies.len(), 3);
    assert_eq!(st.replies[0], (A.to_string(), Ok(A.to_string())));
    assert_eq!(st.replies[1], (B.to_string(), Ok(B.to_string())));
    assert!(matches!(&st.replies[2], (c, Err(e)) if c == DEAD && e.contains("connection refused")), "{:?}", st.replies[2]);
    let p = ots::parse(&st.ots).unwrap();
    assert_eq!(p.digest, digest.to_vec());
    assert_eq!(pending_uris(&p), vec![A.to_string(), B.to_string()], "both branches under one digest");
    assert!(bitcoin_heights(&p).is_empty());
    let dead = stamp_multi(&pool(None), &[DEAD], &digest).unwrap_err();
    assert!(dead.contains("nothing to write"), "{dead}");
    assert!(stamp_multi(&pool(None), &[], &digest).is_err());
}

/// KILL TEST (§7): a fresh stamp is pending; pending is `Unchecked`, never ok, with or without
/// headers, in the image and in the bare file.
#[test]
fn a_pending_calendar_is_unchecked_never_ok() {
    let key = DeviceKey::from_seed(&[7; 32]);
    let image = sealed(&key);
    let (img, st) = stamp_image(&image, &pool(None), &[A, B], 1_730_000_002_000).unwrap();
    assert!(st.replies.iter().all(|r| r.1.is_ok()));
    let h = FileHeaders::parse("900000 0000000000000000000000000000000000000000000000000000000000000001 1700000000000").unwrap();
    let out = verify_image(&img, &[], Some(&h)).unwrap();
    assert_eq!((out.counts.anchors, out.counts.ok, out.counts.bad, out.counts.unchecked), (1, 0, 0, 1), "{:?}", out.checked);
    let (word, why) = describe(&out.checked[0].verdict);
    assert_eq!(word, "unchecked");
    assert!(why.contains(&format!("pending at {A}")) && why.contains(&format!("pending at {B}")), "{why}");
    assert!(out.report.holds() && out.holds(), "the log and seals hold; the anchor is a question not yet answered");
    assert_eq!(out.checked[0].anchor.over, SealedHub::load(&image).unwrap().tip().unwrap(), "over = the tip");
    assert!(matches!(verify_ots(&st.ots, Some(&h)), Verdict::Unchecked(w) if w.contains("pending")));
    assert!(matches!(verify_ots(&st.ots, None), Verdict::Unchecked(_)));
    assert!(matches!(verify_ots(b"not a proof", None), Verdict::Bad(_)));
}

/// KILL TEST (§7): no network -> `Err`, no bytes, no ANCH; and an upgrade offline is an error,
/// never "pending" (pending is a calendar's word, not a dead socket's).
#[test]
fn an_offline_run_writes_no_anch() {
    let key = DeviceKey::from_seed(&[8; 32]);
    let image = sealed(&key);
    let e = stamp_image(&image, &Offline, &[A, B], 1_730_000_002_000).unwrap_err();
    assert!(e.contains("nothing to write") && e.contains("could not resolve host"), "{e}");
    assert_eq!(verify_image(&image, &[], None).unwrap().counts.anchors, 0, "the image has no anchor");
    let (img, st) = stamp_image(&image, &pool(None), &[A], 1_730_000_002_000).unwrap();
    assert!(upgrade_file(&Offline, &st.ots).is_err());
    assert!(upgrade_image(&img, &Offline).is_err());
    assert!(stamp_image(b"not an image", &pool(None), &[A], 0).unwrap_err().contains("not a sealed image"));
}

/// The round trip: pending, then a calendar commits; the file attests a block (PRESENT), the
/// headers line decides (ok / bad), the image gets a NEW ANCH and the old one stays pending.
#[test]
fn an_upgraded_proof_attests_a_block_and_checks_against_headers() {
    let digest = sha256(b"another test digest");
    let p = pool(Some(900_000));
    let st = stamp_multi(&p, &[A, B], &digest).unwrap();
    let up = upgrade_file(&p, &st.ots).unwrap();
    let Upgraded::Attested { ots: new, heights, still_pending } = up else { panic!("{up:?}") };
    assert_eq!((heights.as_slice(), still_pending.as_slice()), ([900_000].as_slice(), [B.to_string()].as_slice()));
    assert!(matches!(upgrade_file(&pool(None), &st.ots), Ok(Upgraded::Pending { uris }) if uris == vec![A.to_string(), B.to_string()]));
    let line = FileHeaders::line(900_000, &p.0[0].root(&digest), 1_700_000_000_000);
    let h = FileHeaders::parse(&line).unwrap();
    assert_eq!(h.merkle_root(900_000), Some(p.0[0].root(&digest)));
    assert!(matches!(verify_ots(&new, Some(&h)), Verdict::Ok { not_after_ms: Some(1_700_000_000_000), .. }), "{:?}", verify_ots(&new, Some(&h)));
    assert!(matches!(verify_ots(&new, None), Verdict::Unchecked(w) if w.contains("no block headers")));
    let wrong = FileHeaders::parse(&FileHeaders::line(900_000, &[9u8; 32], 1)).unwrap();
    assert!(matches!(verify_ots(&new, Some(&wrong)), Verdict::Bad(w) if w.contains("do not reach the Merkle root")));
    assert!(matches!(verify_ots(&st.ots, Some(&h)), Verdict::Unchecked(_)), "the pending file is still pending with headers");

    let key = DeviceKey::from_seed(&[9; 32]);
    let (img, _) = stamp_image(&sealed(&key), &p, &[A, B], 1_730_000_002_000).unwrap();
    let (Some(img2), lines) = upgrade_image(&img, &p).unwrap() else { panic!("nothing upgraded") };
    assert!(lines[0].contains("PRESENT") && lines[0].contains("900000"), "{lines:?}");
    let tip = SealedHub::load(&img).unwrap().tip().unwrap();
    let root = p.0[0].root(&tip);
    let h = FileHeaders::parse(&FileHeaders::line(900_000, &root, 1_700_000_000_000)).unwrap();
    let out = verify_image(&img2, &[], Some(&h)).unwrap();
    assert_eq!((out.counts.anchors, out.counts.ok, out.counts.unchecked, out.counts.bad), (2, 1, 1, 0), "{:?}", out.checked);
    assert!(out.holds());
    let (none, lines) = upgrade_image(&img2, &p).unwrap();
    assert!(none.is_none() && lines.iter().any(|l| l.contains("already attests")), "{lines:?}");
}

/// KILL TEST (§7): one changed cell of the ANCH. Not re-sealed: the verifier REFUSES (BadCrc).
/// Re-sealed by a forger: the proof no longer parses and the anchor is BAD; the image does not hold.
#[test]
fn a_changed_cell_in_an_anchored_image_is_refused() {
    let key = DeviceKey::from_seed(&[10; 32]);
    let (img, _) = stamp_image(&sealed(&key), &pool(None), &[A], 1_730_000_002_000).unwrap();
    assert_eq!(verify_image(&img, &[], None).unwrap().counts.anchors, 1, "control");
    let mut st = Store::from_bytes(&img);
    let a = Doc::open(&st).unwrap().anch.unwrap();
    st.put_cell(a, 9, st.get(a, 9) ^ 1);
    assert!(matches!(verify_image(&st.to_bytes(), &[], None), Err(HubError::BadCrc(_))), "damage is refused by name");
    st.seal(a);
    let out = verify_image(&st.to_bytes(), &[], None).unwrap();
    assert_eq!((out.counts.bad, out.counts.ok), (1, 0), "{:?}", out.checked);
    assert!(!out.holds());
    assert_eq!(describe(&out.checked[0].verdict).0, "BAD");
}

/// Not a test of anything: a FIXTURE WRITER for the live run of `bin/anchor` (M1's
/// `BEBOP_DOC_FIXTURE` shape). `DOWIZ_SEALED_OUT=<path>` writes a two-commit sealed image there;
/// unset, it writes nothing and asserts nothing.
#[test]
fn write_a_sealed_image_when_asked() {
    if let Ok(p) = std::env::var("DOWIZ_SEALED_OUT") {
        std::fs::write(&p, sealed(&DeviceKey::from_seed(&[11; 32]))).unwrap();
    }
}

#[test]
fn the_headers_file_parses_and_refuses_by_line() {
    let text = "# height root(raw) time_ms\n\n1 0101010101010101010101010101010101010101010101010101010101010101 1000 # a comment\n2 0202020202020202020202020202020202020202020202020202020202020202 2000\n";
    let h = FileHeaders::parse(text).unwrap();
    assert_eq!((h.len(), h.merkle_root(1), h.time_ms(2), h.merkle_root(3)), (2, Some([1u8; 32]), Some(2000), None));
    let bad = FileHeaders::parse("1 0101 1000\n2 zz 2000").unwrap_err();
    assert!(bad.starts_with("headers line 1:"), "{bad}");
    assert!(FileHeaders::parse("1 0101010101010101010101010101010101010101010101010101010101010101").is_err(), "two fields");
    let genesis = r#"{"id":"000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f","height":0,"merkle_root":"4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b","timestamp":1231006505}"#;
    let line = header_line_from_explorer(0, genesis).unwrap();
    assert_eq!(line, "0 3ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a 1231006505000", "reversed into raw header order, seconds to ms");
    assert!(header_line_from_explorer(1, genesis).unwrap_err().contains("height 0, not 1"));
    assert!(header_line_from_explorer(0, "{}").unwrap_err().contains("no merkle_root"));
    assert!(header_line_from_explorer(0, "nope").is_err());
}
