//! The Worker half of the seal: state from the var, the pure seal, the key
//! names — and a round trip through the OPENER's code path
//! (`backup_seal::secret_from_file` + `backup_seal::open`, what `tools/seal-open`
//! runs) on a real hub image, with the witness checked after opening.

use super::super::{keys_to_drop, object_key, stamp_of_key, S3};
use super::*;
use base64::Engine;
use dowiz_core::pq::backup_seal::{keygen, open, secret_from_file};

fn pk_text() -> (Vec<u8>, String) {
    let (sk_file, pk) = keygen(&[5u8; 32], &[6u8; 32]);
    (sk_file, pk.encode())
}

fn fixed() -> SealEntropy {
    SealEntropy { m: [1u8; 32], eph: [2u8; 32], nonce: [3u8; 12] }
}

/// A REAL bebop hub image with a few orders on it, and the census the nightly
/// would have uploaded for it.
fn hub_and_witness() -> (Vec<u8>, crate::witness::Census) {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    for (i, id) in ["o-1", "o-2", "o-3"].iter().enumerate() {
        let order = format!("{{\"id\":\"{id}\",\"total\":{}}}", 1500 + i * 100);
        h.append(dowiz_hub::EventKind::Placed, id, &order, i as u64 + 1, [0u8; 32]).unwrap();
    }
    let mut c = crate::witness::Census {
        at_ms: 1_800_000_000_000,
        venue: "v".into(),
        records: h.len(),
        tip: h.tip(),
        generation: 1,
        ..Default::default()
    };
    c.recount();
    (h.to_bytes(), c)
}

fn bundle_of(image: &[u8]) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "format": "dowiz-hub-backup/1",
        "venue": "v",
        "images": { "log": { "generation": 1, "bytes": image.len(),
            "image": base64::engine::general_purpose::STANDARD.encode(image) } },
        "archives": {},
    }))
    .unwrap()
}

#[test]
fn the_var_decides_the_state() {
    let (_, text) = pk_text();
    assert_eq!(state_from(None), SealState::Off);
    assert_eq!(state_from(Some("  ")), SealState::Off);
    assert!(matches!(state_from(Some(&text)), SealState::On(_)), "positive twin");
    let SealState::Refused(why) = state_from(Some("dwzseal-pk1:abcd")) else { panic!("garbage sealed") };
    assert!(why.contains(PK_VAR), "{why}");
    // A well-formed text whose KEM key fails the §7.2 modulus check is refused too.
    let mut bad = SealPublic::parse(&text).unwrap();
    bad.kem_pk[0] = 0xff;
    bad.kem_pk[1] |= 0x0f;
    assert!(matches!(state_from(Some(&bad.encode())), SealState::Refused(_)));
}

#[test]
fn off_is_plain_and_said_refused_is_an_error_on_seals() {
    let body = b"plain gzip bytes".to_vec();
    assert_eq!(apply(&SealState::Off, true, body.clone()), Ok((body.clone(), false)));
    assert_eq!(describe(&SealState::Off)["sealed"], false);
    assert!(apply(&SealState::Refused("bad".into()), true, body.clone()).is_err());
    let (sk_file, text) = pk_text();
    let on = state_from(Some(&text));
    let (sealed, yes) = apply(&on, true, body.clone()).unwrap();
    assert!(yes && sealed != body);
    let sk = secret_from_file(&sk_file).unwrap();
    assert_eq!(open(&sk, &sealed), Ok((KIND_GZIP, body)));
    assert_eq!(describe(&on)["sealed"], true);
}

/// Seal (the Worker's pure path) → open (the opener's path) on a real hub
/// image: byte-equal, and the witness taken over the PLAINTEXT still matches.
#[test]
fn a_sealed_hub_image_opens_byte_equal_and_matches_its_witness() {
    let (image, witness) = hub_and_witness();
    let bundle = bundle_of(&image);
    let (sk_file, text) = pk_text();
    let SealState::On(pk) = state_from(Some(&text)) else { panic!("key refused") };
    let sealed = seal_with(&pk, false, &bundle, &fixed()).unwrap();
    assert!(!sealed.windows(16).any(|w| bundle.windows(16).next() == Some(w)), "plaintext visible");

    let sk = secret_from_file(&sk_file).unwrap();
    let (kind, opened) = open(&sk, &sealed).unwrap();
    assert_eq!((kind, &opened), (KIND_JSON, &bundle), "opened bundle is byte-equal");

    let v: Value = serde_json::from_slice(&opened).unwrap();
    let b64 = v["images"]["log"]["image"].as_str().unwrap();
    let img = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
    assert_eq!(img, image, "opened image is byte-equal");
    let hub = dowiz_hub::Hub::load(&img).unwrap();
    assert_eq!(hub.tip(), witness.tip, "the uploaded witness's tip is the opened image's tip");
    assert_eq!(hub.len(), witness.records);
    assert!(witness.tip.is_some());

    // The wrong secret key refuses; so does a flipped byte anywhere.
    let (other, _) = keygen(&[7u8; 32], &[8u8; 32]);
    assert!(open(&secret_from_file(&other).unwrap(), &sealed).is_err());
    for i in [0, 8, 600, 1100, 1140, 1165, 1200, sealed.len() - 1] {
        let mut t = sealed.clone();
        t[i] ^= 0x80;
        assert!(open(&sk, &t).is_err(), "flip at {i} opened");
    }
    assert!(open(&sk, &sealed[..sealed.len() - 1]).is_err(), "truncated opened");
}

fn s3() -> S3 {
    S3 {
        endpoint: "https://x".into(),
        region: "auto".into(),
        bucket: "b".into(),
        key: "k".into(),
        secret: "s".into(),
        prefix: "backups".into(),
    }
}

#[test]
fn a_sealed_key_is_named_and_stamped_like_the_plain_one() {
    let at = 1_789_808_707_000;
    let plain = object_key(&s3(), "v", at, true, false);
    let sealed = object_key(&s3(), "v", at, true, true);
    assert_eq!(sealed, format!("{plain}{SUFFIX}"));
    assert!(sealed.ends_with(".json.gz.sealed"), "{sealed}");
    assert!(object_key(&s3(), "v", at, false, true).ends_with(".json.sealed"));
    assert_eq!(stamp_of_key(&sealed), stamp_of_key(&plain));
    assert!(stamp_of_key(&sealed).is_some());
    // Not every ".sealed" is ours: a foreign name stays unrecognised.
    assert_eq!(stamp_of_key("backups/v/notes.sealed"), None);
    assert_eq!(stamp_of_key("backups/v/20260919T090507Z.witness.json.sealed"), None);
}

/// Sealed and plain copies share one rotation: the week buckets count both,
/// the old ones of either kind go, and no witness is ever dropped.
#[test]
fn rotation_treats_sealed_and_plain_copies_alike() {
    const DAY: i64 = 86_400_000;
    let now = 1_800_000_000_000;
    let mut keys: Vec<String> = Vec::new();
    for d in 0..60 {
        // The switch to sealing happened 20 days ago.
        keys.push(object_key(&s3(), "v", now - d * DAY, true, d < 20));
    }
    keys.push(super::super::witness_key(&s3(), "v", now - 50 * DAY));
    let dropped = keys_to_drop(&keys, now);
    for d in 0..7 {
        let k = object_key(&s3(), "v", now - d * DAY, true, true);
        assert!(!dropped.contains(&k), "sealed day {d} dropped");
    }
    assert!(dropped.contains(&object_key(&s3(), "v", now - 40 * DAY, true, false)), "old plain kept");
    assert!(dropped.contains(&object_key(&s3(), "v", now - 13 * DAY, true, true)), "sealed extra in a week kept");
    assert!(!dropped.iter().any(|k| k.contains(".witness.")), "a witness was dropped");
    // 7 daily + 2 weekly survive, whatever their kind.
    assert_eq!(keys.len() - 1 - dropped.len(), 9);
}

/// P3's CHECK. Sixty nightly stamps, one a day at the cron's hour: nothing
/// the rotation keeps is 21 days old or older, so the forget answer's "within
/// 22 days" is true of every copy the bucket still holds (a copy made the
/// night before an erasure is at most 21 days old when it goes).
#[test]
fn sixty_nights_keep_nothing_older_than_twenty_one_days() {
    const DAY: i64 = 86_400_000;
    let now = 1_800_000_000_000;
    let keys: Vec<String> = (0..60).map(|d| object_key(&s3(), "v", now - d * DAY, true, false)).collect();
    let dropped = keys_to_drop(&keys, now);
    let kept: Vec<i64> = (0..60).filter(|d| !dropped.contains(&keys[*d as usize])).collect();
    assert!(kept.iter().all(|d| *d < 21), "kept days {kept:?}");
    assert_eq!(super::super::KEEP_WEEKLY_MS, 21 * DAY);
    // Still a useful set: a week of dailies and two weeklies.
    assert_eq!(kept.len(), 9, "kept days {kept:?}");
}
