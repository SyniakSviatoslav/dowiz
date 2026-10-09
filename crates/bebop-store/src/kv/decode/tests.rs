//! Tests and the bench for `kv/decode.rs` (W-KVDEC).
use super::{blob_bytes, key_of};
use crate::kv::delta::{append_delta, Op};
use crate::kv::{fnv1a, Kv, FNV_OFFSET, FNV_PRIME};
use crate::{Store, StoreError};

/// THE OLD COPY, verbatim (kv.rs `blob_byte` before W-KVDEC): byte `b` of a blob, one
/// cell load per byte. The oracle every equivalence test below compares against.
fn blob_byte_old(st: &Store, blob: usize, v: i64, b: usize) -> u8 {
    if v >= 2 {
        (st.get(blob, b >> 3) >> (8 * (b & 7))) as u8
    } else {
        st.get(blob, b) as u8
    }
}

fn copy_old(st: &Store, blob: usize, ver: i64, off: usize, len: usize) -> Option<Vec<u8>> {
    Some((0..len).map(|j| blob_byte_old(st, blob, ver, off + j)).collect())
}

/// The old FNV, verbatim: one byte per step.
fn fnv_old(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Old and new decode of `st` must agree: both refuse, or both give the same entries.
fn same(st: &Store, what: &str) -> bool {
    let new = Kv::decode(st).map(|k| k.entries);
    let old = Kv::decode_with(st, copy_old).map(|k| k.entries);
    assert_eq!(new, old, "{what}: the cell-wise decode differs from the byte-wise one");
    new.is_some()
}

/// `kv` written in version `v` (1 or 2), as kv/tests.rs `image_v` writes it.
fn image_v(kv: &Kv, v: i64) -> Store {
    let mut cap = 16 * 1024;
    loop {
        let mut st = Store::create_bytes(cap).unwrap();
        let (tx, root) = Kv::stage_init_v(&mut st, v).unwrap();
        st.commit_bytes(&tx, root);
        match kv.stage_commit_into(&mut st) {
            Ok((tx, root)) => {
                st.commit_bytes(&tx, root);
                return Store::from_bytes(&st.to_bytes_trimmed());
            }
            Err(StoreError::ArenaFull { .. }) => cap *= 2,
            Err(e) => panic!("{e:?}"),
        }
    }
}

/// Keys and values of every length 0..=17 at every alignment: each tail of 1..7 bytes, a
/// range that starts mid-cell and ends mid-cell, one that spans a cell boundary.
fn odd_kv() -> Kv {
    let mut kv = Kv { entries: Vec::new() };
    for n in 0..=17usize {
        let v: Vec<u8> = (0..n).map(|j| (0x80 + 13 * n + j) as u8).collect();
        kv.put(&format!("k{n:02}{}", "é".repeat(n % 3)), &v);
    }
    kv
}

/// A dubin-size catalogue: 165 products of ~3.3 KB (Albanian, so multi-byte UTF-8), 12
/// categories and a location -- ~548 KB of content, the shape of docs/research
/// 2026-09-28-bebop-dag.md §4 (165 x 3.2 KB). Deterministic.
pub(super) fn dubin_kv() -> Kv {
    let mut kv = Kv { entries: Vec::new() };
    kv.put("location", "{\"name\":\"Dubin & Sushi\",\"slug\":\"dubin-sushi\",\"tz\":\"Europe/Tirane\"}".repeat(20).as_bytes());
    for c in 0..12 {
        kv.put(&format!("category:c_{c}"), format!("{{\"name\":\"Kategoria {c}\",\"sortOrder\":{c}}}").as_bytes());
    }
    for n in 0..165 {
        let mut v = format!("{{\"name\":\"Roll {n} me salmon dhe avokado\",\"price\":{},\"description\":\"", 700 + 10 * n);
        let mut i = 0;
        while v.len() < 3290 + n % 7 {
            v.push_str(&format!("Oriz sushi, salmon i freskët, avokado, susam i pjekur {n}.{i} -- çdo ditë. "));
            i += 1;
        }
        v.truncate(v.char_indices().map(|(j, _)| j).take_while(|&j| j <= 3290 + n % 7).last().unwrap());
        v.push_str("\"}");
        kv.put(&format!("product:p_{n}-ujë"), v.as_bytes());
    }
    kv
}

/// The image bytes of `dubin_kv`, as `Catalog::to_bytes` writes them (compacted, v2).
pub(super) fn dubin_image() -> Vec<u8> {
    dubin_kv().compacted_bytes_fit(4 << 20).expect("the fixture fits")
}

/// MEASURED, not asserted:
/// `cargo test --release --lib kv::decode::tests::bench -- --ignored --nocapture`.
/// `load` is exactly `Catalog::load` (dowiz-hub catalog.rs:90-97): `Store::from_bytes`,
/// `pick`, then `Kv::load_checked` (crc of every object, then `decode`).
#[test]
#[ignore]
fn bench_decode_dubin() {
    let bytes = dubin_image();
    let st = Store::from_bytes(&bytes);
    let kv = Kv::decode(&st).expect("decodes");
    let content: usize = kv.entries.iter().map(|(k, v)| k.len() + v.len()).sum();
    println!("image={}B entries={} content={}B root={:016x}", bytes.len(), kv.entries.len(), content, kv.snapshot_root_u64());
    const N: u32 = 40;
    let med = |f: &mut dyn FnMut()| -> f64 {
        let mut runs: Vec<f64> = (0..9)
            .map(|_| {
                let t = std::time::Instant::now();
                for _ in 0..N {
                    f();
                }
                t.elapsed().as_nanos() as f64 / N as f64 / 1000.0
            })
            .collect();
        runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        runs[runs.len() / 2]
    };
    let decode = med(&mut || {
        std::hint::black_box(Kv::decode(std::hint::black_box(&st)));
    });
    let root = med(&mut || {
        std::hint::black_box(std::hint::black_box(&kv).snapshot_root_u64());
    });
    let root_old = med(&mut || {
        let mut h = FNV_OFFSET;
        for (k, v) in &std::hint::black_box(&kv).entries {
            for part in [&(k.len() as u64).to_le_bytes()[..], k.as_bytes(), &(v.len() as u64).to_le_bytes()[..], v] {
                h = fnv_old(h, part);
            }
        }
        std::hint::black_box(h);
    });
    let decode_old = med(&mut || {
        std::hint::black_box(Kv::decode_with(std::hint::black_box(&st), copy_old));
    });
    let from = med(&mut || {
        std::hint::black_box(Store::from_bytes(std::hint::black_box(&bytes)));
    });
    let load = med(&mut || {
        let s = Store::from_bytes(std::hint::black_box(&bytes));
        assert!(s.pick().is_some());
        std::hint::black_box(Kv::load_checked(&s).ok());
    });
    println!(
        "BENCH median-of-9 (N={N}) us: decode={decode:.1} (bytewise copy {decode_old:.1}) snapshot_root_u64={root:.1} \
         (bytewise fnv {root_old:.1}) from_bytes={from:.1} catalog_load={load:.1} share_of_10ms={:.1}%",
        load / 100.0
    );
}

/// (a) the dubin catalogue, (b) an empty KV, (c) v1, v2 and v3 images, (d) every tail
/// length 1..7 at every alignment: the new copy reads exactly what the old one read.
#[test]
fn cellwise_decode_equals_bytewise_on_every_version_and_tail() {
    let dubin = Store::from_bytes(&dubin_image());
    assert!(same(&dubin, "dubin v2"));
    assert_eq!(Kv::decode(&dubin).unwrap().snapshot_root_u64(), 0xabc9_ed39_eb57_5eef, "the root measured before W-KVDEC");
    let mut empty = Store::create_bytes(64 * 1024).unwrap();
    Kv::init_bytes(&mut empty).unwrap();
    assert!(same(&empty, "empty v2"));
    assert!(Kv::decode(&empty).unwrap().entries.is_empty());
    for v in [1, 2] {
        let st = image_v(&odd_kv(), v);
        assert_eq!(Kv::version(&st), v);
        assert!(same(&st, &format!("odd tails v{v}")));
        assert_eq!(Kv::decode(&st).unwrap().entries, odd_kv().entries, "v{v} reads back what was written");
        assert!(same(&image_v(&Kv { entries: Vec::new() }, v), &format!("empty v{v}")));
    }
    let mut v3 = image_v(&odd_kv(), 2);
    append_delta(&mut v3, &[Op::Put("k03", b"abcdefg"), Op::Put("zz", b"x"), Op::Remove("k07")]).unwrap();
    assert_eq!(Kv::version(&v3), 3);
    assert!(same(&v3, "v3 with a chain"));
}

/// (e) truncated and corrupt images: every refusal the byte-wise decode made, the cell-wise
/// one makes, and every image it read, this reads the same. A positive twin is in the
/// test above; here the dubin image and the odd-tails image are cut and bit-flipped.
#[test]
fn cellwise_decode_refuses_exactly_what_bytewise_refused() {
    let (mut refused, mut read) = (0, 0);
    for bytes in [dubin_image(), image_v(&odd_kv(), 2).to_bytes_trimmed(), image_v(&odd_kv(), 1).to_bytes_trimmed()] {
        for cut in (0..bytes.len()).step_by(bytes.len() / 97 + 1).chain([bytes.len() - 1, bytes.len() - 8]) {
            if same(&Store::from_bytes(&bytes[..cut]), &format!("cut at {cut}")) { read += 1 } else { refused += 1 }
        }
        let base = Store::from_bytes(&bytes);
        let root = base.root().unwrap();
        // Every cell of the root and of both index arrays' first 16 cells, set to values
        // that are each one of the claims decode bounds: 0, -1, +-1 off, bit 33, i64::MAX.
        let mut at: Vec<usize> = (root..root + 8).collect();
        for i in [1, 3] {
            let a = base.follow(root, i).unwrap();
            at.extend(a..a + 18);
        }
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        for _ in 0..64 {
            x ^= x << 13; x ^= x >> 7; x ^= x << 17;
            at.push(x as usize % base.cells.len());
        }
        for &c in &at {
            let old = base.cells[c];
            for new in [0, -1, old.wrapping_add(1), old.wrapping_sub(1), old | (1 << 33), i64::MAX, old ^ 0xff] {
                let mut st = Store { cells: base.cells.clone() };
                st.cells[c] = new;
                if same(&st, &format!("cell {c} = {new}")) { read += 1 } else { refused += 1 }
            }
        }
    }
    // Both arms really ran: a test whose corrupt images were all refused (or all read)
    // would compare nothing interesting.
    assert!(refused > 100 && read > 100, "refused={refused} read={read}");
}

/// The copy itself at every (offset, length) in a 4-cell v2 blob and a 20-cell v1 one.
#[test]
fn blob_bytes_equals_blob_byte_at_every_offset_and_length() {
    let st = image_v(&odd_kv(), 2);
    let root = st.root().unwrap();
    let vblob = st.follow(root, 4).unwrap();
    let bytes = st.obj_cells(vblob) * 8;
    for off in 0..bytes {
        for len in 0..=(bytes - off) {
            assert_eq!(blob_bytes(&st, vblob, 2, off, len), copy_old(&st, vblob, 2, off, len), "v2 off={off} len={len}");
        }
    }
    let st = image_v(&odd_kv(), 1);
    let vblob = st.follow(st.root().unwrap(), 4).unwrap();
    for off in 0..20 {
        for len in 0..20 {
            assert_eq!(blob_bytes(&st, vblob, 1, off, len), copy_old(&st, vblob, 1, off, len), "v1 off={off} len={len}");
        }
    }
}

/// `key_of` is `from_utf8_lossy` -- on valid keys and on damaged ones alike.
#[test]
fn key_of_is_from_utf8_lossy() {
    for b in [&b""[..], b"ascii", "ujë-страва".as_bytes(), b"\xff\xfe", b"a\xc3", b"\xe2\x82", b"ok\x80ok"] {
        assert_eq!(key_of(b.to_vec()), String::from_utf8_lossy(b).into_owned(), "{b:?}");
    }
}

/// `fnv1a` is FNV-1a one byte at a time over EVERY byte: every length 0..=40 (each tail
/// 0..7 after 0..5 whole cells), from two seeds, and the dubin catalogue's whole root. It
/// guards any future per-cell rewrite (W-KVDEC measured one and kept the byte loop).
#[test]
fn fnv_is_bytewise_over_every_byte() {
    let data: Vec<u8> = (0..40u32).map(|i| (i * 37 + 11) as u8).collect();
    for n in 0..=data.len() {
        for seed in [FNV_OFFSET, 0x1234_5678_9abc_def0] {
            assert_eq!(fnv1a(seed, &data[..n]), fnv_old(seed, &data[..n]), "len {n}");
        }
    }
    let kv = dubin_kv();
    let mut h = FNV_OFFSET;
    for (k, v) in &kv.entries {
        for part in [&(k.len() as u64).to_le_bytes()[..], k.as_bytes(), &(v.len() as u64).to_le_bytes()[..], v] {
            h = fnv_old(h, part);
        }
    }
    assert_eq!(kv.snapshot_root_u64(), h);
}
