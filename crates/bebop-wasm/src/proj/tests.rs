//! LV-INC-A: the native reader of the two kind 3/4 fixtures, read to the numbers their
//! `.expected` files derive (gate.sh greps the first two tests' names to count this reader).
use super::*;
use bebop_store::Store;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/fixtures/{name}.store", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

fn expected(name: &str) -> (i64, i64, i64) {
    let text = std::fs::read_to_string(format!("{}/fixtures/{name}.expected", env!("CARGO_MANIFEST_DIR"))).expect("expected");
    let f = |k: &str| -> i64 { text.lines().find_map(|l| l.strip_prefix(k)).expect(k).trim().parse().expect("an integer") };
    (f("n="), f("root="), f("memo="))
}

fn read(name: &str) -> (i64, i64, i64) {
    let p = proj_view(&fixture(name)).unwrap_or_else(|s| panic!("{name}: status {s}"));
    (p.n, p.fold, p.memo)
}

#[test]
fn proj4_fixture_reads_to_expected() {
    assert_eq!(read("proj4"), expected("proj4"));
    let (_, root, memo) = read("proj4");
    assert_eq!(root, memo, "the kind-4 memo IS the KV's Z-set");
}

#[test]
fn proj4_log_fixture_reads_to_expected() {
    assert_eq!(read("proj4.log"), expected("proj4.log"));
}

/// The address of `p`'s OUT payload cell `i`, in bytes, and the crc re-sealed after an edit.
fn edit_out(img: &mut [u8], f: impl Fn(&View) -> (usize, usize), i: usize, x: i64) {
    let (out, len) = f(&View::new(img));
    let at = (out + 2 + i) * 8;
    let old = i64::from_le_bytes(img[at..at + 8].try_into().unwrap());
    img[at..at + 8].copy_from_slice(&(old ^ x).to_le_bytes());
    let crc = bebop_store::crc32(&img[(out + 2) * 8..(out + 2 + len) * 8]) as u64;
    let h1 = i64::from_le_bytes(img[(out + 1) * 8..(out + 2) * 8].try_into().unwrap()) as u64;
    img[(out + 1) * 8..(out + 2) * 8].copy_from_slice(&(((crc << 32) | (h1 & 0xFFFF_FFFF)) as i64).to_le_bytes());
    // W-ES-P10 (FORMAT §7 phase P2): the OUT lies inside the LAST commit's range, so a reader of
    // bytes would refuse the stale anchor (LAST_COMMIT) before the memo rule under test is reached;
    // an honest writer re-anchors, so this edit does too, and the memo rule is what answers.
    use bebop_store::format::{anchor_of, SB_ANC, SB_CRC, SB_USED};
    let mut st = Store::from_bytes(img);
    let sb = st.pick().expect("a live superblock").at;
    let mark = ((st.cells[sb + SB_ANC] as u64) >> 32) as i64;
    st.cells[sb + SB_ANC] = anchor_of(&st, mark, st.cells[sb + SB_USED]);
    st.cells[sb + SB_CRC] = bebop_store::crc32_cells(&st.cells, sb, 15) as i64;
    let n = img.len();
    img.copy_from_slice(&st.to_bytes()[..n]);
}

fn first_out(v: &View) -> (usize, usize) {
    let p = lookup(v, get_in(v, projtab(v).unwrap(), 1)).unwrap();
    (p.out, bebop_store::obj_len_in(v, p.out) as usize)
}

/// A memo whose rows were changed and re-sealed (crc VALID) but whose value was not: the
/// value no longer digests its own rows -- refused, not read. Positive twin: the untouched
/// fixture reads (above); and the same for kind 3's bytes.
#[test]
fn a_memo_that_lies_about_its_own_out_is_refused() {
    let mut img = fixture("proj4");
    edit_out(&mut img, first_out, 4, 1);
    assert_eq!(proj_view(&img), Err(BAD_MEMO));
    let mut img = fixture("proj4.log");
    edit_out(&mut img, first_out, 3, 1);
    assert_eq!(proj_view(&img), Err(BAD_MEMO));
}

/// A kind-4 memo behind its KV (a delta appended by the Rust writer after it) is stale.
#[test]
fn a_kind4_memo_behind_its_kv_is_refused() {
    let mut st = Store::from_bytes(&fixture("proj4"));
    let op = [bebop_store::kv::delta::Op::Put("zone/east", b"new")];
    bebop_store::kv::delta::append_delta_with(&mut st, &op, 32, 0).unwrap();
    assert_eq!(proj_view(&st.to_bytes_trimmed()), Err(STALE_MEMO));
}

#[test]
fn bw_proj_reads_proj4() {
    let img = fixture("proj4");
    let mut out = [0i64; 3];
    assert_eq!(unsafe { bw_proj(img.as_ptr(), img.len(), out.as_mut_ptr()) }, crate::abi::OK);
    assert_eq!((out[0], out[1], out[2]), expected("proj4"));
}
