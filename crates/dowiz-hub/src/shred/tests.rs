//! DG10, RED first (BLUEPRINT-BEBOP-DAG-2026-09-28 §4): crypto-shredding for
//! new logs. Every test calls the real `shred` functions and the real `Hub`;
//! nothing here re-implements the rule it checks.

use super::*;
use crate::{EventKind, Hub};
use bebop_store::evlog::EvLog;
use bebop_store::nodekey::{Frame, TAG_OBJECT};

const ACTOR: [u8; 32] = [0xA1; 32];
/// The subject: a `customer_key` (16 hex, an HMAC in the Worker), never a phone.
const WHO: &str = "a1b2c3d4e5f60718";
const OTHER: &str = "f1e2d3c4b5a69788";
/// The person's phone in the three spellings the P2 CHECK searches for.
const SPELLINGS: [&str; 3] = ["+355691234567", "355691234567", "0691234567"];

fn hex(s: &str) -> Vec<u8> {
    crate::crypto::unhex(&s.split_whitespace().collect::<String>()).unwrap()
}

/// A fresh nonce per seal, as the Worker draws one from `getRandomValues`.
fn nonce(i: u8) -> [u8; NONCE_LEN] {
    [i; NONCE_LEN]
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// An order whose personal fields went through `seal_field` at write time.
fn order(t: &KeyTable, id: &str, subject: &str, phone: &str, n: u8) -> String {
    let phone = seal_field(t, subject, phone, nonce(n)).unwrap();
    let name = seal_field(t, subject, "Arta", nonce(n.wrapping_add(100))).unwrap();
    format!(r#"{{"id":"{id}","status":"PENDING","total":1800,"contact":{{"name":"{name}","phone":"{phone}"}}}}"#)
}

/// A hot log and one archive: the person has orders on both sides of a
/// rotation, and another customer's order sits between them.
fn venue() -> (KeyTable, Hub, Vec<u8>) {
    let mut t = KeyTable::new();
    t.ensure(WHO, [0x11; KEY_LEN]).unwrap();
    t.ensure(OTHER, [0x22; KEY_LEN]).unwrap();
    let mut h = Hub::create_sized(1 << 20).unwrap();
    let mut seq = 0;
    for (i, (id, who, phone)) in [
        ("ord_a", WHO, SPELLINGS[0]),
        ("ord_b", OTHER, "+355690000001"),
        ("ord_c", WHO, SPELLINGS[1]),
        ("ord_d", WHO, SPELLINGS[2]),
    ]
    .into_iter()
    .enumerate()
    {
        seq += 1;
        h.append(EventKind::Placed, id, &order(&t, id, who, phone, i as u8), seq, ACTOR).unwrap();
        seq += 1;
        h.append(EventKind::Advanced, id, r#"{"_d":true,"status":"DELIVERED"}"#, seq, ACTOR).unwrap();
    }
    // ord_a and ord_b go to cold storage; ord_c and ord_d stay hot.
    let archive = h.rotate(|id| id == "ord_c" || id == "ord_d").unwrap();
    (t, h, archive)
}

/// Every sealed string in a log's events.
fn sealed_in(h: &Hub) -> Vec<String> {
    let mut out = Vec::new();
    for e in h.events() {
        let mut rest = e.order_json.as_str();
        while let Some(at) = rest.find(PREFIX) {
            let s = &rest[at..];
            let end = s.find('"').unwrap_or(s.len());
            out.push(s[..end].to_string());
            rest = &s[end..];
        }
    }
    out
}

/// `K256` of every record (its node frame: id, prev, payload), oldest first.
fn k256s(h: &Hub) -> Vec<[u8; 32]> {
    let mut walked = EvLog::walk(&h.store);
    walked.reverse();
    walked
        .iter()
        .map(|r| Frame::new(TAG_OBJECT).bytes(&r.id).bytes(&r.prev).bytes(&r.payload).k256())
        .collect()
}

#[test]
fn kat_aes256gcm_vector_from_core() {
    // GCM spec (McGrew-Viega) Test Case 15: AES-256, 64-byte plaintext, no AAD.
    // Cross-checked 2026-10-01 against node's OpenSSL `aes-256-gcm`.
    let key: [u8; 32] = hex("feffe9928665731c6d6a8f9467308308 feffe9928665731c6d6a8f9467308308").try_into().unwrap();
    let iv: [u8; 12] = hex("cafebabefacedbaddecaf888").try_into().unwrap();
    let pt = hex("d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72
                  1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255");
    let want = hex("522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa
                    8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662898015ad
                    b094dac5d93471bdec1a502270e3cc6c");
    let mut t = KeyTable::new();
    t.ensure(WHO, key).unwrap();
    // Through the shred path itself: the envelope carries nonce || ct || tag.
    let env = seal_bytes(&t, WHO, &pt, iv).unwrap();
    let body = crate::crypto::unhex(env.rsplit(':').next().unwrap()).unwrap();
    assert_eq!(&body[..NONCE_LEN], &iv[..], "the nonce leads the envelope");
    assert_eq!(&body[NONCE_LEN..], &want[..], "ct || tag byte-exact against TC15");
    assert_eq!(open_bytes(&t, &env).unwrap(), pt, "and it opens");
    // Its twin: one flipped bit is refused, never opened as something else.
    let mut bad = body.clone();
    bad[NONCE_LEN] ^= 1;
    let bad_env = format!("{PREFIX}{WHO}:{}", crate::crypto::hex(&bad));
    assert_eq!(open_bytes(&t, &bad_env), Err(ShredError::Tampered));
}

#[test]
fn forget_makes_field_unreadable_everywhere() {
    let (mut t, mut hot, archive_bytes) = venue();
    let archive = Hub::load(&archive_bytes).unwrap();

    // THE POSITIVE TWIN: before forget every sealed field of theirs opens, and
    // in every copy.
    let before: Vec<String> = sealed_in(&hot).into_iter().chain(sealed_in(&archive)).collect();
    let mine: Vec<&String> = before.iter().filter(|s| subject_of(s) == Some(WHO)).collect();
    // The archive is the image as it stood at rotation (all four orders);
    // the hot log kept ord_c and ord_d. Phone + name on each: 2 * (3 + 2).
    assert_eq!(mine.len(), 10, "every copy of every sealed field of theirs");
    let opened: Vec<String> = mine.iter().map(|s| open_field(&t, s).unwrap()).collect();
    for p in SPELLINGS {
        assert!(opened.iter().any(|o| o == p), "{p} readable before forget");
    }
    let key = *t.key(WHO).unwrap();

    let out = hot.shred_forget(&mut t, &[&archive], WHO, "owner", 1_790_000_000_000, 99).unwrap();
    assert_eq!(out, Shredded { dropped: true, sealed: 10, declared: true });

    // Unreadable in the hot log AND the archive; the other customer is not.
    for s in &mine {
        assert_eq!(open_field(&t, s), Err(ShredError::Shredded), "{s}");
    }
    let theirs: Vec<String> = sealed_in(&hot).into_iter().chain(sealed_in(&archive)).filter(|s| subject_of(s) == Some(OTHER)).collect();
    assert_eq!(theirs.len(), 2);
    let theirs: Vec<String> = theirs.iter().map(|s| open_field(&t, s).unwrap()).collect();
    assert_eq!(theirs, ["Arta", "+355690000001"], "the other customer still opens (name, then phone)");

    // THE BYTE SEARCH (P2 CHECK): every image, every archived block, the key
    // table -- no spelling of the phone, and not the key either.
    let images = [hot.to_bytes(), archive_bytes.clone(), archive.to_bytes(), t.to_bytes()];
    for img in &images {
        for p in SPELLINGS {
            assert!(!contains(img, p.as_bytes()), "{p} found in an image");
        }
        assert!(!contains(img, &key), "the dropped key's bytes are still in an image");
    }

    // A second run changes nothing and declares nothing twice.
    let again = hot.shred_forget(&mut t, &[&archive], WHO, "owner", 1_790_000_000_001, 100).unwrap();
    assert_eq!(again, Shredded { dropped: false, sealed: 10, declared: false });
}

#[test]
fn block_hash_unchanged_by_forget() {
    let (mut t, mut hot, archive_bytes) = venue();
    let archive = Hub::load(&archive_bytes).unwrap();
    let (hot_before, arch_before, tip) = (k256s(&hot), k256s(&archive), hot.tip().unwrap());

    hot.shred_forget(&mut t, &[&archive], WHO, "owner", 1_790_000_000_000, 99).unwrap();

    let hot_after = k256s(&hot);
    assert_eq!(hot_after.len(), hot_before.len() + 1, "forget only appends its declaration");
    assert_eq!(&hot_after[..hot_before.len()], &hot_before[..], "no hot block moved");
    assert_eq!(k256s(&archive), arch_before, "no archived block moved");
    assert_eq!(archive.to_bytes_trimmed(), archive_bytes, "the archive is byte-identical (cmp)");
    assert!(hot.holds(&tip), "last night's witness still holds");

    // Law 9 (`conservation.mjs` `chain.redacted == declared`, native side):
    // shredding redacts nothing in place, so it declares zero tombstones.
    let c = hot.chain_check();
    assert_eq!((c.broken, c.redacted), (0, 0), "{c:?}");
    assert_eq!(c.redacted, hot.declared(), "redacted == declared");
    let d = hot.events().into_iter().find(|e| e.kind == EventKind::Forgotten).unwrap();
    assert_eq!(d.order_id, format!("cust:{WHO}"));
    assert_eq!(crate::minijson::int_field(&d.order_json, "shredded"), Some(10));
}

#[test]
fn key_table_holds_no_personal_bytes() {
    let (t, _, _) = venue();
    let bytes = t.to_bytes();
    for p in SPELLINGS {
        assert!(!contains(&bytes, p.as_bytes()), "{p} in the key table");
    }
    // No run of 7+ digits anywhere: a phone cannot hide in it unspelled.
    let longest = bytes.split(|b| !b.is_ascii_digit()).map(<[u8]>::len).max().unwrap_or(0);
    assert!(longest < 7, "a digit run of {longest} in the key table");
    // A subject that is not a customer key is refused, so nothing personal
    // can be filed as one; its twin, a real key, is accepted above.
    let mut t2 = KeyTable::new();
    assert_eq!(t2.ensure(SPELLINGS[0], [1; KEY_LEN]), Err(ShredError::Subject));
    assert_eq!(t2.ensure("abcdef0123abcdef", [1; KEY_LEN]).map(|_| t2.len()), Ok(1));
    assert_eq!(t2.ensure(OTHER, [0; KEY_LEN]), Err(ShredError::WeakKey), "an all-zero key is not a key");
    // The table round-trips, and a damaged one is refused by name.
    let back = KeyTable::load(&bytes).unwrap();
    assert_eq!(back.to_bytes(), bytes);
    let mut bad = bytes.clone();
    let at = bad.len() - 5;
    bad[at] ^= 1;
    assert_eq!(KeyTable::load(&bad).err(), Some(ShredError::Corrupt("crc")));
}
