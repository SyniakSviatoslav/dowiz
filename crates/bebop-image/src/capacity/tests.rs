//! W-MG-TIDY: `image_bytes` is a number from the image, and every writer that rebuilds or grows
//! hands it to `create_bytes`. A claim past the writers' bound must not reach an allocation.
use crate::*;
use crate::crc32_cells;

/// A store whose live superblock claims `claim` capacity cells, crc and all.
fn claiming(claim: i64) -> Store {
    let mut st = Store::create_bytes(1 << 16).unwrap();
    st.cells[SB_A + 12] = claim;
    st.cells[SB_A + 15] = crc32_cells(&st.cells, SB_A, 15) as i64;
    Store::from_bytes(&st.to_bytes_trimmed())
}

#[test]
fn a_claim_past_the_bound_is_named_and_never_sized_by() {
    for claim in [i64::MAX, 1 << 50, (MAX_PAD_CELLS - ARENA) as i64 + 1] {
        let st = claiming(claim);
        assert!(st.pick().is_some(), "control: the superblock is valid");
        assert!(matches!(st.writer_bytes(), Err(StoreError::Limit { what: "image capacity", got, max }) if got == claim && max == (MAX_PAD_CELLS - ARENA) as i64), "{:?}", st.writer_bytes());
        assert_eq!(st.image_bytes(), st.cells.len() * 8, "what it holds, not what it claims");
        assert!(Store::create_bytes(st.image_bytes().max(MIN_BYTES)).is_ok());
    }
}

#[test]
fn a_claim_at_the_bound_is_the_writers_number() {
    let at = (MAX_PAD_CELLS - ARENA) as i64;
    let st = claiming(at);
    assert_eq!(st.image_bytes(), MAX_PAD_CELLS * 8);
    assert_eq!(st.writer_bytes().unwrap(), MAX_PAD_CELLS * 8);
    let small = claiming(4096);
    assert_eq!(small.image_bytes(), (ARENA + 4096) * 8);
}

/// A negative claim sizes by the cells held, as it always did.
#[test]
fn a_negative_claim_sizes_by_the_cells_held() {
    let st = claiming(-1);
    assert_eq!(st.image_bytes(), st.cells.len() * 8);
}
