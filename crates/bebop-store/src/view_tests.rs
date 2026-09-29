//! The borrowed reader (`View` + the `*_in` rules) and the owned one (`Store`'s methods) must
//! answer every read rule alike -- they are two bodies of the same rules (see lib.rs, DG5).

use crate::evlog::{EvLog, Record};
use crate::*;

fn image() -> Vec<u8> {
    let mut st = Store::create_bytes(1 << 16).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    let mut prev = [0u8; 32];
    for i in 0..6u8 {
        let r = Record { id: [i + 1; 32], prev, actor_pubkey: [0; 32], actor_seq: i as u64, payload: vec![i; 20] };
        prev = r.id;
        EvLog::append_tip_bytes(&mut st, &r).unwrap();
    }
    st.to_bytes_trimmed()
}

fn agree(img: &[u8], what: &str) {
    let st = Store::from_bytes(img);
    // `from_bytes` pads a trimmed image to its capacity; the view is given the SAME cells, so
    // the comparison is of the rules, not of the padding (see `View`'s doc).
    let full: Vec<u8> = st.cells.iter().flat_map(|c| c.to_le_bytes()).collect();
    let v = View::new(&full);
    assert_eq!(st.pick(), v.pick(), "{what}: pick");
    assert_eq!(st.root(), v.root(), "{what}: root");
    for at in [SB_A, SB_B] {
        assert_eq!(st.sb_valid(at), sb_valid_in(&v, at), "{what}: sb_valid {at}");
    }
    // Every object header the arena walk meets, and the refs out of each.
    let used = st.pick().map_or(ARENA, |s| s.arena_used.max(ARENA as i64) as usize);
    let mut o = ARENA;
    while o + 2 <= used {
        assert_eq!(st.obj_len(o), obj_len_in(&v, o), "{what}: obj_len {o}");
        assert_eq!(st.obj_cells(o), obj_cells_in(&v, o), "{what}: obj_cells {o}");
        assert_eq!(st.obj_digest(o), obj_digest_in(&v, o), "{what}: obj_digest {o}");
        assert_eq!(st.obj_crc_ok(o), obj_crc_ok_in(&v, o), "{what}: obj_crc_ok {o}");
        for i in 0..4 {
            assert_eq!(st.get(o, i), get_in(&v, o, i), "{what}: get {o} {i}");
            assert_eq!(st.follow(o, i), follow_in(&v, o, i), "{what}: follow {o} {i}");
        }
        let len = st.obj_len(o).max(1) as usize;
        o += 2 + len;
    }
}

#[test]
fn the_view_and_the_store_answer_alike() {
    let img = image();
    agree(&img, "clean");
    // Corruptions a reader must survive: a cut image, a flipped header length, a wild ref.
    agree(&img[..img.len() - 8], "cut");
    let mut b = img.clone();
    b[ARENA * 8 + 3] ^= 0x40;
    agree(&b, "header");
    let mut c = img.clone();
    let at = (ARENA + 2 + 2) * 8;
    c[at..at + 8].copy_from_slice(&(i64::MAX - 3).to_le_bytes());
    agree(&c, "wild ref");
}
