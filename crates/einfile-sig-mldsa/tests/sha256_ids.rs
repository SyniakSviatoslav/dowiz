// SPDX-License-Identifier: MIT OR Apache-2.0
//! `Sha256Ids`, the ready-made id scheme, against what the rest of the library already computes:
//! the segment root against `einstore::merkle`, the id against sha256 of `id_message`, and the
//! nonce and tombstone rules of FORMAT §9.4, each with a control.

use einfile_log::doc::{id_message, Ids};
use einfile_log::{Record, FLAG_SHA256, NONCE_LEN};
use einfile_sig_mldsa::{actor_id, Sha256Ids};
use einstore::merkle;

fn ids(n: usize) -> Vec<[u8; 32]> {
    (0..n).map(|i| actor_id(&[b'i', i as u8])).collect()
}

#[test]
fn the_segment_root_is_the_facades_rfc9162_tree() {
    for n in 0..=17 {
        let ids = ids(n);
        let leaves: Vec<_> = ids.iter().map(|id| merkle::leaf_hash(id)).collect();
        assert_eq!(Sha256Ids.tree_root(&ids), Some(merkle::root(&leaves)), "{n} ids");
    }
    // control: the empty tree is sha256 of nothing (RFC 9162 §2.1.1)
    let empty: Vec<String> = Sha256Ids.tree_root(&[]).unwrap().iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(empty.concat(), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn the_id_is_sha256_of_the_id_message() {
    let r = Record { id: [0; 32], prev: [3; 32], actor_pubkey: [9; 32], actor_seq: 4, flags: FLAG_SHA256, payload: b"nonce-0123456789body".to_vec() };
    assert_eq!(Sha256Ids.chained(&r), actor_id(&id_message(r.flags, &r.actor_pubkey, r.actor_seq, &r.prev, &r.payload)));
    let mut other = r.clone();
    other.actor_seq = 5;
    assert_ne!(Sha256Ids.chained(&r), Sha256Ids.chained(&other), "control: the seq is in the id");
}

#[test]
fn the_nonce_and_the_tombstone_follow_format_9_4() {
    let mut p = vec![0x7Fu8; NONCE_LEN];
    p.extend_from_slice(&[0x01, 0xAA]);
    assert_eq!(Sha256Ids.body(FLAG_SHA256, &p), &[0x01, 0xAA], "behind the nonce");
    assert_eq!(Sha256Ids.body(0, &p), p.as_slice(), "no flag, no nonce");
    assert!(!Sha256Ids.redacted(FLAG_SHA256, &p), "control: kind 0x01 is not a tombstone");
    p[NONCE_LEN] |= 0x80;
    assert!(Sha256Ids.redacted(FLAG_SHA256, &p), "the high bit of the kind byte");
    assert!(!Sha256Ids.redacted(FLAG_SHA256, &p[..NONCE_LEN]), "an empty tail is no tombstone");
    assert!(!Sha256Ids.redacted(0, &[0x7F, 0x80]), "unflagged: the first byte is the kind, and 0x7F has no mark");
}
