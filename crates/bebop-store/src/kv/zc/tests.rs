//! Tests for the zero-copy reader (W-ZC). One named corrupted cell per defect (no fuzzing,
//! operator rule); every equality is against `Kv::load` + `Kv::get` on the SAME bytes.
use super::*;
use crate::kv::Kv;
use crate::{Store, StoreError, View};

/// An image of `kv` in version `v` (1 = every image before DG3, 2 = production).
fn image_v(kv: &Kv, v: i64) -> Vec<u8> {
    if v >= 2 {
        return kv.compacted_bytes_fit(64 << 20).unwrap();
    }
    let mut cap = 16 * 1024;
    loop {
        let mut st = Store::create_bytes(cap).unwrap();
        let (tx, root) = Kv::stage_init_v(&mut st, v).unwrap();
        st.commit_bytes(&tx, root);
        match kv.stage_commit_into(&mut st) {
            Ok((tx, root)) => {
                st.commit_bytes(&tx, root);
                return st.to_bytes_trimmed();
            }
            Err(StoreError::ArenaFull { .. }) => cap *= 2,
            Err(e) => panic!("{e:?}"),
        }
    }
}

fn lcg(x: &mut u64) -> u8 {
    *x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*x >> 56) as u8
}

/// 165 dishes of `size` bytes plus a venue record and categories: the catalogue's shape.
fn catalogue(size: usize) -> Kv {
    let mut kv = Kv { entries: Vec::new() };
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in 0..165 {
        let v: Vec<u8> = (0..size).map(|_| lcg(&mut x)).collect();
        kv.put(&format!("product:item-{i:03}"), &v);
    }
    for c in ["rolls", "sushi", "drinks"] {
        kv.put(&format!("category:{c}"), c.as_bytes());
    }
    kv.put("location", b"{\"id\":\"v1\"}");
    kv
}

/// Small shapes that stress the edges: empty key, empty value, shared prefixes,
/// non-ASCII keys, lengths that are not a multiple of 8.
fn edges() -> Kv {
    let mut kv = Kv { entries: Vec::new() };
    for (k, v) in [("", &b""[..]), ("a", b"1234567"), ("ab", b"12345678"), ("abc", b"123456789"), ("dë", b""), ("ujë", b"water"), ("z", b"last")] {
        kv.put(k, v);
    }
    kv
}

/// Every image the walk covers: (name, bytes). The frozen fixtures are the bytes bebop
/// and the hub wrote long before this reader existed.
fn images() -> Vec<(String, Vec<u8>)> {
    let mut out = vec![
        ("kv.store v1 fixture".to_string(), include_bytes!("../../../../bebop-wasm/fixtures/kv.store").to_vec()),
        ("kv2.store v2 fixture".to_string(), include_bytes!("../../../../bebop-wasm/fixtures/kv2.store").to_vec()),
    ];
    for v in [1, 2] {
        out.push((format!("empty v{v}"), image_v(&Kv { entries: Vec::new() }, v)));
        out.push((format!("edges v{v}"), image_v(&edges(), v)));
    }
    out.push(("catalogue 165 x 2.5 KB v2".into(), image_v(&catalogue(2500), 2)));
    out.push(("catalogue 165 x 300 B v1".into(), image_v(&catalogue(300), 1)));
    out
}

/// Probe keys around every stored key: the key itself, one before it, one after it.
fn probes(kv: &Kv) -> Vec<Vec<u8>> {
    let mut p: Vec<Vec<u8>> = vec![b"".to_vec(), b"\0".to_vec(), b"\xff\xff".to_vec(), b"product:".to_vec(), b"product:item-999".to_vec()];
    for (k, _) in &kv.entries {
        let k = k.as_bytes().to_vec();
        p.push(k.clone());
        let mut longer = k.clone();
        longer.push(0);
        p.push(longer);
        if !k.is_empty() {
            p.push(k[..k.len() - 1].to_vec());
        }
    }
    p
}

/// THE EQUALITY WALK. On every image: every stored key (first and last included) and
/// every probe answers byte for byte what `Kv::get` answers, every prefix what a filter
/// over `Kv::load` gives, the fold is the same number, and the walk visits EXACTLY as
/// many entries as `Kv::load` decoded -- a reader that skips the last entry fails here.
#[test]
fn zc_equals_kv_load_on_every_image() {
    for (name, bytes) in images() {
        let st = Store::from_bytes(&bytes);
        let kv = Kv::load_checked(&st).unwrap_or_else(|e| panic!("{name}: Kv::load {e:?}"));
        let view = View::new(&bytes);
        let zc = KvIn::open(&view).unwrap_or_else(|e| panic!("{name}: open {e:?}"));
        assert!(zc.sorted(), "{name}: every writer sorts");
        assert_eq!(zc.len(), kv.entries.len(), "{name}: count");
        let mut walked = 0;
        for i in 0..zc.len() {
            assert_eq!(&*zc.key(i).unwrap(), kv.entries[i].0.as_bytes(), "{name}: key {i}");
            assert_eq!(&*zc.value(i).unwrap(), &kv.entries[i].1[..], "{name}: value {i}");
            walked += 1;
        }
        assert_eq!(walked, kv.entries.len(), "{name}: the walk skipped entries");
        for p in probes(&kv) {
            let want = std::str::from_utf8(&p).ok().and_then(|s| kv.get(s));
            assert_eq!(zc.get(&p).unwrap().map(|c| c.into_owned()), want, "{name}: get {:?}", String::from_utf8_lossy(&p));
            assert_eq!(kv_get_in(&view, &p).unwrap(), want, "{name}: kv_get_in");
            assert_eq!(kv_get_in(&st, &p).unwrap(), want, "{name}: kv_get_in over Store");
        }
        for pre in ["", "a", "ab", "product:", "category:", "loc", "zz", "ujë"] {
            let want: Vec<(Vec<u8>, Vec<u8>)> =
                kv.entries.iter().filter(|(k, _)| k.starts_with(pre)).map(|(k, v)| (k.as_bytes().to_vec(), v.clone())).collect();
            assert_eq!(kv_prefix_in(&view, pre.as_bytes()).unwrap(), want, "{name}: prefix {pre:?}");
        }
        assert_eq!(zc.snapshot_root_u64().unwrap(), kv.snapshot_root_u64(), "{name}: fold");
        let again = KvIn::reopen(&view, zc.checked()).unwrap();
        for (k, v) in &kv.entries {
            assert_eq!(again.get(k.as_bytes()).unwrap().as_deref(), Some(&v[..]), "{name}: reopen {k}");
        }
    }
}

/// `Kv::get` is a binary search now; it must still answer every key and every probe.
#[test]
fn kv_get_binary_search_finds_first_last_and_misses() {
    for kv in [edges(), catalogue(16)] {
        for (i, (k, v)) in kv.entries.iter().enumerate() {
            assert_eq!(kv.get(k).as_ref(), Some(v), "entry {i} of {} ({k:?})", kv.entries.len());
        }
        assert_eq!(kv.get("zzzz"), None);
        assert_eq!(kv.get("product:item-999"), None);
    }
}

/// A ONE-BYTE FLIP IN ANY OF THE FIVE OBJECTS is the same named refusal `load_checked`
/// gives -- the value blob included: a crc-unchecked value read would hand back the
/// changed byte as data.
#[test]
fn zc_a_flipped_byte_is_bad_crc_naming_the_object() {
    let clean = image_v(&catalogue(64), 2);
    let st = Store::from_bytes(&clean);
    let root = st.root().unwrap();
    let mut objs = vec![("root", root)];
    for (n, i) in [("kidx", 1), ("kblob", 2), ("vidx", 3), ("vblob", 4)] {
        objs.push((n, st.follow(root, i).unwrap()));
    }
    for (name, obj) in objs {
        let mut b = clean.clone();
        b[(obj + 2) * 8 + 1] ^= 1;
        let want = Kv::load_checked(&Store::from_bytes(&b)).map(|k| k.entries.len());
        assert!(matches!(want, Err(KvError::BadCrc(x)) if x.obj == obj), "{name}: load_checked {want:?}");
        match kv_get_in(&View::new(&b), b"product:item-000") {
            Err(KvError::BadCrc(x)) => assert_eq!(x.obj, obj, "{name}: names the object"),
            other => panic!("{name}: zero-copy read gave {other:?}"),
        }
    }
}

/// Rewrite one payload cell and re-seal, so the crc passes and only the claim is wrong.
fn forge(bytes: &[u8], arr: usize, cell: usize, val: i64) -> Vec<u8> {
    let mut st = Store::from_bytes(bytes);
    let root = st.root().unwrap();
    let obj = if arr == 0 { root } else { st.follow(root, arr).unwrap() };
    st.cells[obj + 2 + cell] = val;
    st.seal(obj);
    st.to_bytes_trimmed()
}

/// LENGTHS ARE CLAIMS. A value offset past the image, a key length of 2^33+9, a count
/// past the index: each is an `Err`, never a panic, through `open` AND through `reopen`
/// (which skips the index pass, so the bound must hold at the read itself).
#[test]
fn zc_an_unbounded_length_is_an_error_not_a_panic() {
    let clean = image_v(&catalogue(64), 2);
    let ck = KvIn::open(View::new(&clean)).unwrap().checked();
    for (name, arr, cell, val) in [
        ("value offset past the image", 3, 0, 1 << 40),
        ("value length past the blob", 3, 1, 8_589_934_601),
        ("key length 2^33+9", 1, 1, 8_589_934_601),
        ("negative key offset", 1, 0, -5),
        ("count past the index", 0, 0, 1 << 30),
    ] {
        let b = forge(&clean, arr, cell, val);
        assert_eq!(Kv::load_checked(&Store::from_bytes(&b)).map(|k| k.entries.len()), Err(KvError::NotKv), "{name}: load");
        let v = View::new(&b);
        assert_eq!(kv_get_in(&v, b"product:item-000").map(|_| ()), Err(KvError::NotKv), "{name}: open");
        if let Ok(r) = KvIn::reopen(&v, ck) {
            let got = (0..r.len()).map(|i| r.key(i).and(r.value(i)).map(|_| ())).find(|x| x.is_err());
            assert_eq!(got, Some(Err(KvError::NotKv)), "{name}: reopen reads it as an error");
        }
    }
    // A truncated image: no superblock fits, no root.
    assert!(kv_get_in(&View::new(&clean[..clean.len() / 2]), b"location").is_err(), "a truncated image read");
}

/// AN UNSORTED IMAGE (none is written here, but an image is a claim) is detected by
/// `open` and read by scan, answering exactly `Kv::get`.
#[test]
fn zc_an_unsorted_image_falls_back_to_a_scan() {
    let kv = Kv { entries: vec![("m".into(), b"1".to_vec()), ("b".into(), b"2".to_vec()), ("z".into(), b"3".to_vec()), ("a".into(), b"4".to_vec())] };
    let b = kv.compacted_bytes_fit(1 << 20).unwrap();
    let v = View::new(&b);
    let zc = KvIn::open(&v).unwrap();
    assert!(!zc.sorted(), "the order check missed an unsorted index");
    for k in ["m", "b", "z", "a", "c"] {
        assert_eq!(zc.get(k.as_bytes()).unwrap().map(|c| c.into_owned()), kv.get(k), "{k}");
    }
    assert_eq!(kv_prefix_in(&v, b"").unwrap().len(), 4);
}

/// O(log n), MEASURED IN CELLS READ: one `get` on the 165 x 2.5 KB image after `reopen`
/// reads the root, ~8 keys and the one value -- not the ~52,000-cell image.
#[test]
fn zc_a_get_reads_log_n_keys_and_one_value() {
    let b = image_v(&catalogue(2500), 2);
    let v = View::new(&b);
    let ck = KvIn::open(&v).unwrap().checked();
    let before = v.reads();
    let r = KvIn::reopen(&v, ck).unwrap();
    let got = r.get(b"product:item-164").unwrap().unwrap();
    assert_eq!(got.len(), 2500);
    let reads = v.reads() - before;
    println!("zc_a_get_reads_log_n_keys_and_one_value: {reads} cells read of {}", b.len() / 8);
    assert!(reads < 600, "{reads} cells read for one get");
}
