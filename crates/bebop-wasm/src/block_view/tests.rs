//! The in-place view against `decode` (this crate's DG9 reader) on DG7's fixture FILES, the
//! menu lookup against the decoded columns, and every refusal with its positive twin.

use super::*;
use crate::block::{decode, vals};

fn fixture(name: &str) -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dowiz-hub/fixtures/blocks").join(format!("{name}.dwb"));
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn reseal(mut b: Vec<u8>) -> Vec<u8> {
    let end = b.len() - 4;
    let c = bebop_store::crc32(&b[..end]);
    b[end..].copy_from_slice(&c.to_le_bytes());
    b
}

const NAMES: [&str; 4] = ["menu_prices", "bom", "stock_levels", "names"];

/// Every element read in place is the value `decode` copied out; the line is the decode
/// line's first half, on every fixture (the empty blocks too).
#[test]
fn the_view_reads_what_decode_copies() {
    let t = table();
    for name in NAMES {
        for suffix in ["", "_small", "_empty"] {
            let b = fixture(&format!("{name}{suffix}"));
            let d = decode(&b, &t).unwrap();
            let v = View::new(&b, &t).unwrap();
            assert_eq!((v.name(), v.n, v.nnz), (t[d.schema].name, d.n, d.nnz));
            for (c, col) in d.cols.iter().enumerate() {
                let read: Vec<i64> = (0..v.len(c).unwrap()).map(|i| v.at(c, i).unwrap()).collect();
                assert_eq!(&read, col, "{name}{suffix} column {c}");
                assert_eq!(v.at(c, col.len()), None, "one past the end is None, not a read");
            }
            assert_eq!(v.at(d.cols.len(), 0), None, "a column that does not exist is None");
            assert_eq!(v.vals(), vals(&d));
            let full = crate::block::line(&b);
            assert!(full.starts_with(&format!("{} rt=", line(&b))), "{full} vs {}", line(&b));
        }
    }
}

/// A corrupted block is refused by the view with decode's own refusal; the twin is read.
#[test]
fn the_view_refuses_what_decode_refuses() {
    let m = fixture("menu_prices");
    assert!(line(&m).starts_with("block menu_prices n=165 "), "the twin: {}", line(&m));
    for (at, x) in [(0, 1), (4, 1), (6, 1), (8, 1), (16, 1), (40, 1), (42, 1), (48, 8)] {
        let mut b = m.clone();
        b[at] ^= x;
        let b = reseal(b);
        let want = crate::block::line(&b);
        assert!(want.starts_with("block refused="), "flip {at}: {want}");
        assert_eq!(line(&b), want, "flip {at}");
    }
    assert_eq!(line(&m[..27]), "block refused=too_short col=-");
}

#[test]
fn str_at_reads_names_and_nothing_else() {
    let t = table();
    let n = fixture("names_small");
    let v = View::new(&n, &t).unwrap();
    let d = decode(&n, &t).unwrap();
    for i in 0..v.n as usize {
        let s = v.str_at(i).unwrap();
        assert_eq!(k64(s.as_bytes()) as i64, d.cols[0][i], "row {i}: the id is the K64 of the bytes");
    }
    assert_eq!(v.str_at(v.n as usize), None, "past the last row");
    let m = fixture("menu_prices_small");
    assert_eq!(View::new(&m, &t).unwrap().str_at(0), None, "a price block has no text");
}

/// The 165-dish catalogue: every dish resolves, confirmed by its bytes, to decode's price.
#[test]
fn menu_prices_every_dish_as_decoded() {
    let t = table();
    let (p, n) = (fixture("menu_prices"), fixture("names"));
    let menu = Menu::new(View::new(&p, &t).unwrap(), View::new(&n, &t).unwrap()).unwrap();
    let (dp, dn) = (decode(&p, &t).unwrap(), decode(&n, &t).unwrap());
    let mut seen = 0;
    for (row, key) in dp.cols[DISH].iter().enumerate() {
        let at = dn.cols[0].iter().position(|k| k == key).expect("every dish is named");
        let id = menu.names.str_at(at).unwrap();
        assert_eq!(menu.row_of(id), Some(row));
        assert_eq!(menu.price_of(id), Some((dp.cols[PRICE][row], dp.cols[TAX_PPM][row])));
        seen += 1;
    }
    assert_eq!(seen, 165);
    assert_eq!(menu.price_of("no-such-dish"), None);
    assert!(menu_line(&p, &n).starts_with("menu rows=165 resolved=165 fold="), "{}", menu_line(&p, &n));
}

/// RT K-1: a key whose bytes in `names` are not the product's answers None, never a price.
#[test]
fn a_name_whose_bytes_moved_resolves_nothing() {
    let t = table();
    let (p, n) = (fixture("menu_prices"), fixture("names"));
    let v = View::new(&n, &t).unwrap();
    let id = Menu::new(View::new(&p, &t).unwrap(), View::new(&n, &t).unwrap()).unwrap().names.str_at(0).unwrap().to_string();
    let first = v.lay[NAME_BYTES].0;
    let mut b = n.clone();
    b[first] = if b[first] == b'x' { b'y' } else { b'x' };
    let b = reseal(b);
    let menu = Menu::new(View::new(&p, &t).unwrap(), View::new(&b, &t).unwrap()).unwrap();
    assert_eq!(menu.price_of(&id), None, "{id}: the key is there, the bytes are not");
    assert!(menu_line(&p, &b).starts_with("menu rows=165 resolved=164 "), "{}", menu_line(&p, &b));
}

/// The native reader's half of gate.sh's menu agreement: the lines `oracle.py --menu` printed
/// for DG7's fixtures (2026-10-02), held here so `cargo test` is a reader of its own.
#[test]
fn menu_lines_are_the_oracles() {
    assert_eq!(menu_line(&fixture("menu_prices"), &fixture("names")), "menu rows=165 resolved=165 fold=3c337ae044a59385");
    assert_eq!(menu_line(&fixture("menu_prices_small"), &fixture("names_small")), "menu rows=5 resolved=5 fold=c29d19fdc5202790");
}

#[test]
fn menu_refuses_the_wrong_blocks_and_a_modifier_past_names() {
    let (p, n) = (fixture("menu_prices"), fixture("names"));
    assert_eq!(menu_line(&n, &p), "menu refused=mismatch col=-");
    assert_eq!(menu_line(&p, &fixture("names_small")), "menu refused=bad_offsets col=mods_col");
    assert_eq!(menu_line(&p, &n[..20]), "menu refused=too_short col=-");
    assert!(menu_line(&p, &n).starts_with("menu rows=165 "), "the twin reads");
    assert_eq!(menu_line(&fixture("menu_prices_empty"), &fixture("names_empty")), format!("menu rows=0 resolved=0 fold={FNV_OFFSET:016x}"));
}

#[test]
fn bw_view_and_bw_menu_answer_statuses_not_traps() {
    let (p, n) = (fixture("menu_prices_small"), fixture("names_small"));
    let mut out = [0u8; 256];
    let k = unsafe { bw_view(p.as_ptr(), p.len(), out.as_mut_ptr(), out.len()) };
    assert_eq!(&out[..k as usize], line(&p).as_bytes());
    let k = unsafe { bw_menu(p.as_ptr(), p.len(), n.as_ptr(), n.len(), out.as_mut_ptr(), out.len()) };
    assert_eq!(&out[..k as usize], menu_line(&p, &n).as_bytes());
    assert_eq!(unsafe { bw_view(core::ptr::null(), 4, out.as_mut_ptr(), 256) }, -crate::abi::NULL_ARG);
    assert_eq!(unsafe { bw_menu(p.as_ptr(), p.len(), core::ptr::null(), 3, out.as_mut_ptr(), 256) }, -crate::abi::NULL_ARG);
    assert_eq!(unsafe { bw_view(p.as_ptr(), p.len(), core::ptr::null_mut(), 256) }, -crate::abi::NULL_ARG);
    assert_eq!(unsafe { bw_view(p.as_ptr(), p.len(), out.as_mut_ptr(), 8) }, -1, "a short buffer is a status");
}
