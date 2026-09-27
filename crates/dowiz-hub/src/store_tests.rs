//! W-AUDIT S1 / S2 (2026-09-27): NAMED CORRUPTED CELLS in the store beneath
//! the hub -- no fuzzer (repo rule), one damaged cell per case, through the
//! real `Hub::load` / `Hub::append` / `EvLog::walk`.
//!
//! S1: the PartTab's write cursor (`pt + 19`) was the one cell the write path
//! trusted without a check. S2: records are data and may overlap, so the sum
//! of what a walk reads must be bounded by the image, not each record alone.

use crate::{EventKind, Hub, HubError};
use bebop_store::evlog::EvLog;
use bebop_store::{Store, StoreError};

fn hub_with(n: usize) -> Vec<u8> {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    for i in 0..n {
        h.append(EventKind::Placed, &format!("o{i}"), r#"{"id":"o","status":"PENDING"}"#, i as u64 + 1, [0u8; 32])
            .unwrap();
    }
    h.to_bytes()
}

/// S1. Four lies in one cell: negative (a usize of 1.8e19 and a PANIC on the
/// next append), past the arena, inside the superblock, and behind the newest
/// record (the next append written OVER it). Each loads -- the chain is whole
/// -- and each is refused at the write, loudly, with the store's own word.
#[test]
fn a_damaged_write_cursor_refuses_the_next_append_instead_of_panicking() {
    let bytes = hub_with(3);
    let st = Store::from_bytes(&bytes);
    let pt = st.parttab().expect("a live PartTab");
    let honest = st.cells[pt + 19];
    for (bad, why) in [(-8, "negative"), (1 << 40, "past the arena"), (5, "inside the superblock"), (honest - 20, "behind the newest record")] {
        let mut st = Store::from_bytes(&bytes);
        st.cells[pt + 19] = bad;
        let mut h = Hub::load(&st.to_bytes()).expect("the chain is whole; only the cursor lies");
        let r = h.append(EventKind::Placed, "o9", "{}", 9, [0u8; 32]);
        assert!(matches!(r, Err(HubError::Store(StoreError::Corrupt(_)))), "cursor {why} ({bad}): {r:?}");
    }
    // Twin: the honest cursor appends.
    let mut h = Hub::load(&bytes).unwrap();
    h.append(EventKind::Placed, "o9", "{}", 9, [0u8; 32]).expect("an honest image appends");
    assert_eq!(h.len(), 4);
}

/// S2. Six records whose headers each claim the rest of the image, and whose
/// payload lengths claim more than everything. `Hub::load` passes (the chain
/// is whole; `chain_len` reads no payload) and the first read used to
/// allocate six times the image. Now the whole walk reads at most the image.
#[test]
fn overlapping_records_cannot_read_more_than_the_image() {
    let bytes = hub_with(6);
    let mut st = Store::from_bytes(&bytes);
    let mut cur = st.follow(st.root().unwrap(), 1);
    let mut n = 0;
    while let Some(obj) = cur {
        cur = st.follow(obj, 2);
        st.cells[obj] = (st.cells[obj] & !0xFFFF_FFFF) | 0xFFFF_FFFF;
        st.cells[obj + 2 + 1] = 1 << 40;
        n += 1;
    }
    assert_eq!(n, 6, "the six records were found by following the chain");
    let lying = Store::from_bytes(&st.to_bytes());
    assert!(Hub::load(&st.to_bytes()).is_ok(), "the chain is still whole; only the lengths lie");
    let read: usize = EvLog::walk(&lying).iter().map(|r| r.payload.len()).sum();
    assert!(read <= bytes.len(), "a walk read {read} bytes out of an image of {}", bytes.len());
    // Twin: an honest image's records read back whole.
    let honest = Store::from_bytes(&bytes);
    assert_eq!(EvLog::walk(&honest).len(), 6);
    assert!(EvLog::walk(&honest).iter().all(|r| !r.payload.is_empty()));
}
