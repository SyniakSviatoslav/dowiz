//! GOLDENS PINNED FROM THE OLD CODE (W-DELTA, 2026-10-06), before the delta chain existed.
//!
//! The constants below were printed by THIS file run against the untouched tree (kv.rs with
//! eager compaction only, no `kv/delta.rs`), output in the lane's scratchpad
//! `delta-golden-old.out`. They pin two promises of the delta change:
//!
//!   * an image written by the old code reads to the same entries and the same fold, and
//!   * compaction is unchanged: the same entries compact to the same BYTES (so every reader
//!     that read a compacted image before reads one after, and the fold is byte-equal).
//!
//! FNV-1a 64 over the whole image is the pin (bebop-store has no sha256 and takes no deps).

use super::*;

/// 165 dishes x ~2.5 KB, `product:dNNN` keys, plus three categories and a location --
/// the catalogue's shape (report F.1). Deterministic (an LCG): every run, every reader.
pub(crate) fn golden_catalogue() -> Kv {
    let mut kv = Kv { entries: Vec::new() };
    let mut x: u64 = 0x5DEE_CE66_D1CE_4E5B;
    let mut byte = || {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        b'a' + ((x >> 59) as u8 % 26)
    };
    for i in 0..165 {
        let body: String = (0..2400).map(|_| byte() as char).collect();
        let v = format!("{{\"id\":\"d{i:03}\",\"price\":{},\"d\":\"{body}\"}}", 300 + i * 10);
        kv.put(&format!("product:d{i:03}"), v.as_bytes());
    }
    for c in ["hot", "maki", "sets"] {
        kv.put(&format!("category:{c}"), format!("{{\"id\":\"{c}\"}}").as_bytes());
    }
    kv.put("location", b"{\"name\":\"Golden\",\"currency\":\"ALL\"}");
    kv
}

pub(crate) fn fnv_bytes(b: &[u8]) -> u64 {
    fnv1a(FNV_OFFSET, b)
}

/// The eager (non-compacted) path the bins use: init + three `commit_into` generations.
fn eager_image(kv: &Kv) -> Vec<u8> {
    let mut st = Store::create_bytes(4 << 20).unwrap();
    Kv::init_bytes(&mut st).unwrap();
    let mut k = Kv::load(&st).unwrap();
    for (key, v) in kv.entries.iter().take(40) {
        k.put(key, v);
    }
    let (tx, root) = k.stage_commit_into(&mut st).unwrap();
    st.commit_bytes(&tx, root);
    k.remove("product:d003");
    let (tx, root) = k.stage_commit_into(&mut st).unwrap();
    st.commit_bytes(&tx, root);
    k.put("product:d010", b"{\"price\":999}");
    let (tx, root) = k.stage_commit_into(&mut st).unwrap();
    st.commit_bytes(&tx, root);
    st.to_bytes_trimmed()
}

/// Printed by the OLD code (delta-golden-old.out); see the module.
const COMPACT_LEN: usize = 417208;
const COMPACT_FNV: u64 = 0x074f6c778d6464d2;
const COMPACT_ROOT: u64 = 0x8b94f0a99a553e58;
const EAGER_LEN: usize = 269528;
const EAGER_FNV: u64 = 0x96301cac3567caee;
const EAGER_ROOT: u64 = 0x0e7480bf47b04d9b;

#[test]
fn golden_old_images_and_compaction_are_unchanged() {
    let kv = golden_catalogue();
    let compact = kv.compacted_bytes_fit(10 << 20).unwrap();
    let eager = eager_image(&kv);
    let cr = Kv::load(&Store::from_bytes(&compact)).unwrap().snapshot_root_u64();
    let er = Kv::load(&Store::from_bytes(&eager)).unwrap().snapshot_root_u64();
    println!(
        "GOLDEN compact len={} fnv={:#018x} root={:#018x} | eager len={} fnv={:#018x} root={:#018x}",
        compact.len(),
        fnv_bytes(&compact),
        cr,
        eager.len(),
        fnv_bytes(&eager),
        er
    );
    assert_eq!((compact.len(), fnv_bytes(&compact), cr), (COMPACT_LEN, COMPACT_FNV, COMPACT_ROOT), "compaction output moved");
    assert_eq!(cr, kv.snapshot_root_u64(), "the fold of the compacted image");
    assert_eq!((eager.len(), fnv_bytes(&eager), er), (EAGER_LEN, EAGER_FNV, EAGER_ROOT), "the old eager image reads differently");
    assert_eq!(Kv::version(&Store::from_bytes(&compact)), 2, "compaction writes v2");
}
