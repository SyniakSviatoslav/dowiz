//! The wasm32 block reader against DG7's fixture FILES (crates/dowiz-hub/fixtures/blocks), and
//! its refusals, each with a positive twin. gate.sh runs the same code under node over the
//! fixtures and 4,000 random blocks; these tests hold the native build of it honest.

use super::*;

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

/// B-2 on this side: the four keys are the ones DG7's table and oracle.py --schemas print.
#[test]
fn table_parses_and_keys_are_dg7s() {
    let t = table();
    assert_eq!(t.len(), 4, "every schema string parses");
    let keys: Vec<u64> = t.iter().map(|s| s.k64).collect();
    assert_eq!(keys, [0x8bbf_0924_0000_0063, 0xee38_2c13_0000_002f, 0x959a_f446_0000_0031, 0x9130_70fc_0000_002a]);
    assert_eq!(t.iter().map(|s| s.name).collect::<Vec<_>>(), NAMES);
    assert_eq!(t[0].cols[5], ("mods_val", CSR_VAL, 1), "an i64 after a CSR u32 is the CSR val");
}

/// B-6: every fixture decodes and `encode(decode(b)) == b` byte for byte, the empty block too.
#[test]
fn every_fixture_round_trips() {
    let t = table();
    for name in NAMES {
        for suffix in ["", "_small", "_empty"] {
            let b = fixture(&format!("{name}{suffix}"));
            let d = decode(&b, &t).unwrap_or_else(|r| panic!("{name}{suffix}: {r:?}"));
            assert_eq!(t[d.schema].name, name);
            assert_eq!(encode(&d, &t), b, "{name}{suffix}: encode(decode(b)) == b");
            assert_eq!(decode(&encode(&d, &t), &t).unwrap(), d, "{name}{suffix}: decode(encode(x)) == x");
            assert!(line(&b).contains(" rt=ok "), "{name}{suffix}: {}", line(&b));
        }
    }
    assert_eq!(decode(&fixture("menu_prices"), &t).unwrap().n, 165);
    assert_eq!(decode(&fixture("bom_empty"), &t).unwrap().n, 0);
}

/// One refusal per header region, each reached (crc re-sealed) and each against the untouched twin.
#[test]
fn every_region_refuses_by_name() {
    let t = table();
    let m = fixture("menu_prices");
    assert!(decode(&m, &t).is_ok(), "the twin: the untouched fixture decodes");
    let flip = |at: usize, x: u8| {
        let mut b = m.clone();
        b[at] ^= x;
        decode(&reseal(b), &t).err()
    };
    let r = |code, col| Some(Refused { code, col });
    assert_eq!(flip(0, 1), r("bad_magic", "-"));
    assert_eq!(flip(4, 1), r("bad_version", "-"));
    assert_eq!(flip(16, 1), r("unknown_schema", "-"));
    assert_eq!(flip(6, 1), r("bad_ncols", "-"));
    assert_eq!(flip(8, 1), r("bad_length", "dish"));
    assert_eq!(flip(40, 1), r("bad_type", "price"));
    assert_eq!(flip(42, 1), r("bad_unit", "price"));
    assert_eq!(flip(43, 1), r("bad_reserved", "price"));
    assert_eq!(flip(48, 1), r("bad_alignment", "price"));
    assert_eq!(flip(48, 8), r("bad_offsets", "price"));
    let mut crc = m.clone();
    *crc.last_mut().unwrap() ^= 1;
    assert_eq!(decode(&crc, &t).err(), r("bad_crc", "-"));
    assert_eq!(decode(&m[..27], &t).err(), r("too_short", "-"));
    let (_, _, _, lay) = check(&m, &t).unwrap();
    let pad = lay[4].0 + lay[4].1;
    assert_ne!(pad % 8, 0, "the 165-dish mods_col (nnz 261) leaves padding before mods_val");
    assert_eq!(flip(pad, 1), r("bad_padding", "mods_val"));
}

/// The VALUE checks: a row_ptr past nnz, a text offset past its bytes, invalid UTF-8, a NUL.
#[test]
fn offsets_and_text_values_refuse() {
    let t = table();
    let (m, n) = (fixture("menu_prices"), fixture("names"));
    let (_, _, _, ml) = check(&m, &t).unwrap();
    let (_, _, _, nl) = check(&n, &t).unwrap();
    let mut b = m.clone();
    b[ml[3].0 + 4..ml[3].0 + 8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(decode(&reseal(b), &t).err(), Some(no("bad_offsets", "mods_ptr")));
    let mut b = n.clone();
    b[nl[2].0 + 4..nl[2].0 + 8].copy_from_slice(&0x7fff_ffffu32.to_le_bytes());
    assert_eq!(decode(&reseal(b), &t).err(), Some(no("bad_offsets", "off")));
    for bad in [0xFF, 0x00] {
        let mut b = n.clone();
        b[nl[1].0] = bad;
        assert_eq!(decode(&reseal(b), &t).err(), Some(no("not_utf8", "bytes")), "byte {bad:#x}");
    }
    let mut s = fixture("stock_levels");
    s[12] = 1;
    assert_eq!(decode(&reseal(s), &t).err(), Some(no("bad_nnz", "-")));
    assert!(decode(&n, &t).is_ok() && decode(&fixture("stock_levels"), &t).is_ok(), "the twins decode");
}

#[test]
fn bw_block_answers_statuses_not_traps() {
    let m = fixture("bom_small");
    let mut out = [0u8; 256];
    let n = unsafe { bw_block(m.as_ptr(), m.len(), out.as_mut_ptr(), out.len()) };
    assert_eq!(&out[..n as usize], line(&m).as_bytes());
    assert_eq!(unsafe { bw_block(core::ptr::null(), 4, out.as_mut_ptr(), 256) }, -crate::abi::NULL_ARG);
    assert_eq!(unsafe { bw_block(m.as_ptr(), m.len(), out.as_mut_ptr(), 8) }, -1, "a short buffer is a status");
    let empty = unsafe { bw_block(core::ptr::null(), 0, out.as_mut_ptr(), 256) };
    assert_eq!(&out[..empty as usize], b"block refused=too_short col=-");
}
