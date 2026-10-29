//! `KvImage` against the decoded `Kv` it replaced: the same answers after the same writes,
//! through saves and loads, in both save modes.
use super::*;
use bebop_store::kv::Kv;
use bebop_store::Store;

fn script(img: &mut KvImage, oracle: &mut Kv, round: u8) {
    for i in 0..6u8 {
        let k = format!("p:{}", (i * 7 + round) % 9);
        if (i + round) % 4 == 0 {
            assert_eq!(img.remove(&k), oracle.remove(&k), "remove {k}");
        } else {
            let v = format!("v{round}-{i}");
            img.put(&k, v.as_bytes());
            oracle.put(&k, v.as_bytes());
        }
    }
}

fn same(img: &KvImage, oracle: &Kv) {
    assert_eq!(img.keys(), oracle.keys());
    for k in oracle.keys() {
        assert_eq!(img.get(&k), oracle.get(&k), "{k}");
    }
    let want: Vec<(String, Vec<u8>)> = oracle.entries.iter().filter(|(k, _)| k.starts_with("p:")).cloned().collect();
    assert_eq!(img.prefixed("p:"), want);
    assert_eq!(img.snapshot_root(), oracle.snapshot_root());
}

#[test]
fn a_kv_image_answers_what_the_decoded_kv_answers_through_saves_and_loads() {
    for save in [Save::Delta, Save::Compact] {
        let mut img = KvImage::create(crate::CEILING_BYTES, save).unwrap();
        let mut oracle = Kv { entries: Vec::new() };
        let mut deltas = 0;
        for round in 0..8u8 {
            script(&mut img, &mut oracle, round);
            same(&img, &oracle);
            let bytes = img.to_bytes().unwrap();
            let ver = Kv::version(&Store::from_bytes(&bytes));
            match save {
                Save::Compact => assert_eq!(ver, 2, "a Compact holder wrote a delta"),
                // the delta writer may compact on its own trigger (dead cells); it must append sometimes
                Save::Delta if round > 0 => deltas += usize::from(ver == 3),
                Save::Delta => assert_eq!(ver, 2, "a fresh holder's first save compacts"),
            }
            img = KvImage::load(&bytes, crate::CEILING_BYTES, save).unwrap();
            same(&img, &oracle);
        }
        assert!(save == Save::Compact || deltas > 0, "a Delta holder never appended");
    }
}

#[test]
fn a_save_with_nothing_pending_returns_the_image_as_it_is() {
    let mut img = KvImage::create(crate::CEILING_BYTES, Save::Compact).unwrap();
    img.put("a", b"1");
    let once = img.to_bytes().unwrap();
    assert_eq!(img.to_bytes().unwrap(), once);
    let mut again = KvImage::load(&once, crate::CEILING_BYTES, Save::Compact).unwrap();
    assert_eq!(again.to_bytes().unwrap(), once);
}

#[test]
fn a_key_past_the_limit_refuses_every_save_by_name_and_a_failed_save_keeps_the_writes() {
    let mut img = KvImage::create(crate::CEILING_BYTES, Save::Delta).unwrap();
    img.put("ok", b"1");
    img.put(&"k".repeat(65_536), b"v");
    let e = img.to_bytes().unwrap_err();
    assert!(matches!(e, HubError::Store(bebop_store::StoreError::Limit { what: "key bytes", .. })), "{e:?}");
    assert_eq!(img.get("ok"), Some(b"1".to_vec()), "a refused save dropped the pending writes");
}

#[test]
fn a_changed_byte_in_the_newest_commit_is_bad_crc_not_a_fallback() {
    let mut img = KvImage::create(crate::CEILING_BYTES, Save::Delta).unwrap();
    img.put("a", b"1");
    let base = img.to_bytes().unwrap();
    let mut img = KvImage::load(&base, crate::CEILING_BYTES, Save::Delta).unwrap();
    img.put("a", b"2");
    let mut bytes = img.to_bytes().unwrap();
    let n = bytes.len();
    bytes[n - 1] ^= 1;
    assert!(matches!(KvImage::load(&bytes, crate::CEILING_BYTES, Save::Delta), Err(HubError::BadCrc(_))));
}
