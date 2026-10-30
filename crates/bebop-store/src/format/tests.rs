//! T-ES-08 (every constant of FORMAT-BEBOP-IMAGE-v2.md re-derived, and the document's own
//! constant table read back), T-ES-03 (the PartTab crc is checked before `begin`), the
//! anchor (D2) and the limits (D7, FORMAT §10), each refusal with its positive twin.

use super::*;
use crate::kv::delta::{append_delta_with, Appended, Op, MAX_DELTAS};
use crate::kv::Kv;
use crate::{crc32, crc32_cells, Store, StoreError, View, MIN_BYTES, PARTTAB_CELLS};

/// `append_delta` with the dead-cell trigger off: these images are small, and the test is of
/// the format, not of when the writer prefers to compact.
pub(crate) fn append_delta(st: &mut Store, ops: &[Op]) -> Result<Appended, StoreError> {
    append_delta_with(st, ops, MAX_DELTAS, 0)
}

/// Three commits: a compacted v2 base, then a put and a remove as delta records.
pub(crate) fn img3() -> Store {
    let mut kv = Kv { entries: Vec::new() };
    for i in 0..5 {
        kv.put(&format!("p:{i:02}"), format!("value-{i}-{}", "x".repeat(20 + i)).as_bytes());
    }
    let mut st = Store::from_bytes(&kv.compacted_bytes_fit(1 << 20).unwrap());
    assert!(matches!(append_delta(&mut st, &[Op::Put("p:00", b"new")]).unwrap(), Appended::Delta(2)));
    assert!(matches!(append_delta(&mut st, &[Op::Remove("p:01")]).unwrap(), Appended::Delta(3)));
    st
}

/// Recompute a superblock's cell-15 crc after changing one of its cells on purpose.
pub(crate) fn resb(st: &mut Store, at: usize) {
    st.cells[at + SB_CRC] = crc32_cells(&st.cells, at, 15) as i64;
}

#[test]
fn magic_is_bebopst1_little_endian() {
    assert_eq!(MAGIC.to_le_bytes(), *b"BEBOPST1");
    assert_eq!(MAGIC_BYTES, *b"BEBOPST1");
}

#[test]
fn superblocks_parttabs_and_arena_sit_on_their_own_pages() {
    assert_eq!(SB_A * 8, 0);
    assert_eq!(SB_B * 8, 4096, "superblock B starts the second 4 KiB page");
    assert_eq!(ARENA * 8, 2 * 4096, "the arena starts the third page");
    assert_eq!(SB_CELLS * 8, 128, "a superblock is 128 bytes, inside one 512-byte sector");
    assert_eq!(PARTTAB_CELLS, 16 + 3, "P = 1: a superblock copy + (root, used, gen)");
    assert_eq!(PARTTAB_OBJ_CELLS, 21);
    for page in [SB_A, SB_B] {
        assert!(page + PT_PAGE_OFFSET + PARTTAB_OBJ_CELLS <= page + 512, "the PartTab fits its page tail");
    }
    assert_eq!((PT_ROOT, PT_CURSOR, PT_GEN), (18, 19, 20));
    assert_eq!(MIN_BYTES, (ARENA + 1) * 8);
    assert_eq!(FORMAT_VERSION, 2);
}

/// The PartTab of a commit from superblock `sb` lands at `(512 - sb) + 16`, and its three
/// partition cells are the root, the cursor and the generation the superblock names.
#[test]
fn each_commit_writes_its_parttab_into_the_other_page() {
    let mut st = Store::create_bytes(1 << 16).unwrap();
    for (from, gen) in [(SB_A, 1), (SB_B, 2), (SB_A, 3)] {
        let mut tx = st.begin().unwrap();
        assert_eq!(tx.sb, from);
        let root = st.alloc(&mut tx, 2, 7).unwrap();
        st.seal(root);
        assert_eq!(st.commit_bytes(&tx, root), gen);
        let sb = st.pick_checked().unwrap();
        let pt = sb.root as usize;
        assert_eq!(sb.at, SB_B - from);
        assert_eq!(pt, sb.at + PT_PAGE_OFFSET, "gen {gen}: the PartTab sits in its own superblock's page");
        assert_eq!(st.cells[pt + PT_ROOT], root as i64);
        assert_eq!(st.cells[pt + PT_CURSOR], sb.arena_used);
        assert_eq!(st.cells[pt + PT_GEN], gen);
        assert_eq!(st.obj_len(pt), PARTTAB_CELLS);
        assert_eq!(st.cells[sb.at + SB_VERSION], FORMAT_VERSION);
        assert_eq!((st.cells[sb.at + 13], st.cells[sb.at + 14]), (0, 0), "S-1");
    }
}

/// T-ES-08's review gate: the document's constant table, read back. A constant renamed or
/// re-valued on either side without the other turns this red.
#[test]
fn the_format_document_states_every_constant_with_the_codes_value() {
    let doc = include_str!("../../../../docs/FORMAT-BEBOP-IMAGE-v2.md");
    let code: &[(&str, i128)] = &[
        ("MAGIC", MAGIC as i128), ("FORMAT_VERSION", FORMAT_VERSION as i128), ("SB_A", SB_A as i128),
        ("SB_B", SB_B as i128), ("SB_CELLS", SB_CELLS as i128), ("ARENA", ARENA as i128),
        ("MIN_BYTES", MIN_BYTES as i128), ("PARTTAB_CELLS", PARTTAB_CELLS as i128),
        ("PARTTAB_OBJ_CELLS", PARTTAB_OBJ_CELLS as i128), ("PT_PAGE_OFFSET", PT_PAGE_OFFSET as i128),
        ("PT_ROOT", PT_ROOT as i128), ("PT_CURSOR", PT_CURSOR as i128), ("PT_GEN", PT_GEN as i128),
        ("SB_ANC", SB_ANC as i128), ("MAX_OBJ_CELLS", MAX_OBJ_CELLS as i128),
        ("MAX_GENERATION", MAX_GENERATION as i128), ("MAX_ANCHOR_MARK", MAX_ANCHOR_MARK as i128),
        ("MAX_KEY_BYTES", MAX_KEY_BYTES as i128), ("MAX_VALUE_BYTES", MAX_VALUE_BYTES as i128),
        ("MAX_PAD_CELLS", crate::MAX_PAD_CELLS as i128), ("MAX_DELTAS", crate::kv::delta::MAX_DELTAS as i128),
        ("DEAD_DIV", crate::kv::delta::DEAD_DIV as i128), ("DIGEST_KV_ROOT", crate::kv::DIGEST_KV_ROOT as i128),
        ("DIGEST_ARR_I64", crate::kv::DIGEST_ARR_I64 as i128),
        // KV v4 (§9.1, W-ES-P8C); the digests are derived in bebop-kv's tree/tests.rs.
        ("VERSION_TREE", crate::kv::tree::VERSION_TREE as i128), ("DIGEST_KV4_LEAF", crate::kv::tree::DIGEST_KV4_LEAF as i128),
        ("DIGEST_KV4_NODE", crate::kv::tree::DIGEST_KV4_NODE as i128), ("DIGEST_KV4_VAL", crate::kv::tree::DIGEST_KV4_VAL as i128),
        ("MAX_TREE_DEPTH", crate::kv::tree::MAX_TREE_DEPTH as i128), ("FANOUT", crate::kv::tree::FANOUT as i128),
        // PROJ OUT layouts (§9.3, LV-INC-A): kinds 1/2, 3 and 4, and the widest OUTZ row.
        ("DIGEST_OUT", crate::proj::DIGEST_OUT as i128), ("DIGEST_OUTB", crate::proj::kinds::DIGEST_OUTB as i128),
        ("DIGEST_OUTZ", crate::proj::kinds::DIGEST_OUTZ as i128), ("MAX_WIDTH", crate::proj::kinds::MAX_WIDTH as i128),
    ];
    // Rows of the form "| `NAME` | <decimal> | ...".
    let rows: Vec<(String, i128)> = doc
        .lines()
        .filter_map(|l| {
            let mut f = l.split('|').map(str::trim).skip(1);
            let name = f.next()?.strip_prefix('`')?.strip_suffix('`')?.to_string();
            let v = f.next()?.replace('_', "").parse::<i128>().ok()?;
            Some((name, v))
        })
        .collect();
    for (name, v) in code {
        let got: Vec<i128> = rows.iter().filter(|(n, _)| n == name).map(|(_, v)| *v).collect();
        assert_eq!(got, vec![*v], "the document's row for {name}");
    }
    for (name, _) in &rows {
        assert!(code.iter().any(|(n, _)| n == name), "the document states {name}, which no test derives");
    }
}

/// T-ES-03: one bit of the PartTab's cursor cell is refused by `pick`, before `begin` can
/// trust it. RED before W-ES-P0: `pick` returned the superblock and `begin` opened a
/// transaction at the flipped cursor.
#[test]
fn a_flipped_parttab_cursor_bit_is_refused_before_begin() {
    let mut st = img3();
    let sb = st.pick_checked().unwrap();
    let pt = sb.root as usize;
    st.cells[pt + PT_CURSOR] ^= 1 << 3;
    let e = st.pick_checked().unwrap_err();
    assert!(matches!(e, PickError::PartTabBad { at, pt: p, got: Some(_), .. } if at == sb.at && p == pt), "{e:?}");
    assert!(matches!(st.begin(), Err(StoreError::Image(PickError::PartTabBad { .. }))));
    assert_eq!(View::new(&st.to_bytes()).pick(), None, "the borrowed reader refuses it too");
}

#[test]
fn a_clean_parttab_passes_and_begin_opens_at_its_cursor() {
    let st = img3();
    let sb = st.pick_checked().unwrap();
    let tx = st.begin().unwrap();
    assert_eq!(tx.mark, st.cells[sb.root as usize + PT_CURSOR]);
}

/// D2: every commit this crate writes carries bebop's anchor, and the anchor is the
/// formula re-derived over the BYTES with the byte-wise crc (a second code path).
#[test]
fn the_rust_writer_anchors_every_commit() {
    let mut st = Store::create_bytes(1 << 16).unwrap();
    Kv::init_bytes(&mut st).unwrap();
    let mut prev = ARENA as i64;
    for i in 0..4 {
        if i > 0 {
            let a = append_delta(&mut st, &[Op::Put("k", format!("v{i}").as_bytes())]).unwrap();
            assert_eq!(a, Appended::Delta(i as i64 + 1), "a delta commit, not a compaction");
        }
        let sb = st.pick_checked().unwrap();
        let used = sb.arena_used;
        let bytes = st.to_bytes();
        let want = ((prev as u64) << 32 | crc32(&bytes[prev as usize * 8..used as usize * 8]) as u64) as i64;
        assert_eq!(st.cells[sb.at + SB_ANC], want, "commit {i}");
        assert_eq!(anchor_in(&st, &sb), Anchor::Holds { mark: prev as usize, used: used as usize });
        prev = used;
    }
}

/// A changed cell inside the last commit's range is what the anchor exists to see; the
/// Rust readers still pick the image (the anchor is the reopen's check, not `pick`'s).
#[test]
fn a_torn_last_commit_fails_its_anchor() {
    let mut st = img3();
    let sb = st.pick_checked().unwrap();
    let last = (sb.arena_used - 1) as usize;
    st.cells[last] ^= 1 << 9;
    assert!(matches!(anchor_in(&st, &sb), Anchor::Torn { got: Some(_), .. }));
    assert_eq!(st.pick_checked(), Ok(sb));
}

/// Readers accept `anc` = 0 (every image before W-ES-P0) and read it to the same entries.
#[test]
fn an_unanchored_image_reads_exactly_as_an_anchored_one() {
    let mut st = img3();
    let before = Kv::load(&st).unwrap().entries;
    let at = st.pick_checked().unwrap().at;
    st.cells[at + SB_ANC] = 0;
    resb(&mut st, at);
    let sb = st.pick_checked().unwrap();
    assert_eq!(anchor_in(&st, &sb), Anchor::Absent);
    assert_eq!(Kv::load(&st).unwrap().entries, before);
    // The frozen fixtures were written before any Rust anchor: they pick, and nothing is torn.
    for img in [&include_bytes!("../../../bebop-wasm/fixtures/kv.store")[..], &include_bytes!("../../../bebop-wasm/fixtures/kv2.store")[..]] {
        let st = Store::from_bytes(img);
        let sb = st.pick_checked().unwrap();
        assert!(!matches!(anchor_in(&st, &sb), Anchor::Torn { .. }), "{:?}", anchor_in(&st, &sb));
    }
}

#[test]
fn a_mark_past_its_32_bit_field_writes_no_anchor() {
    let st = Store::create_bytes(MIN_BYTES).unwrap();
    assert_eq!(anchor_of(&st, MAX_ANCHOR_MARK + 1, MAX_ANCHOR_MARK + 1), 0);
    assert_ne!(anchor_of(&st, MAX_ANCHOR_MARK, MAX_ANCHOR_MARK), 0, "the bound itself anchors");
    assert_eq!(anchor_of(&st, ARENA as i64 - 1, ARENA as i64), 0, "below the arena: none");
}

/// D7: the commit that would cross 2^32 - 1 generations is refused; the one that reaches it is not.
#[test]
fn the_generation_limit_refuses_at_the_bound() {
    let mut st = Store::create_bytes(MIN_BYTES).unwrap();
    st.cells[SB_A + SB_GEN] = MAX_GENERATION - 1;
    resb(&mut st, SB_A);
    assert_eq!(st.begin().unwrap().next_gen, MAX_GENERATION);
    st.cells[SB_A + SB_GEN] = MAX_GENERATION;
    resb(&mut st, SB_A);
    assert!(matches!(st.begin(), Err(StoreError::Limit { what: "generation", .. })));
}

#[test]
fn the_object_length_limit_refuses_past_32_bits_and_negative() {
    let mut st = Store::create_bytes(MIN_BYTES).unwrap();
    let mut tx = st.begin().unwrap();
    for len in [-1, MAX_OBJ_CELLS + 1, i64::MAX] {
        assert!(matches!(st.alloc(&mut tx, len, 0), Err(StoreError::Limit { what: "object cells", .. })), "{len}");
    }
    // At the bound the length rule accepts; the arena (1 cell) is what says no.
    assert!(matches!(st.alloc(&mut tx, MAX_OBJ_CELLS, 0), Err(StoreError::ArenaFull { .. })));
}

#[test]
fn the_key_limit_refuses_one_byte_past_the_bound() {
    let ok = "k".repeat(MAX_KEY_BYTES);
    let long = "k".repeat(MAX_KEY_BYTES + 1);
    let mut kv = Kv { entries: Vec::new() };
    kv.put(&ok, b"v");
    // Room for a second 64 KiB record: `_fit` would stop at the first capacity that holds one.
    let mut st = Store::from_bytes(&kv.compacted_bytes(1 << 20).unwrap());
    assert_eq!(Kv::load(&st).unwrap().entries[0].0.len(), MAX_KEY_BYTES);
    assert!(matches!(append_delta(&mut st, &[Op::Put(&ok, b"w")]), Ok(Appended::Delta(_))));
    let e = append_delta(&mut st, &[Op::Put(&long, b"w")]).unwrap_err();
    assert!(matches!(e, StoreError::Limit { what: "key bytes", got, .. } if got == MAX_KEY_BYTES as i64 + 1), "{e:?}");
    kv.put(&long, b"v");
    assert!(matches!(kv.compacted_bytes_fit(1 << 20), Err(StoreError::Limit { what: "key bytes", .. })));
}

/// The value bound by length alone: a 1 GiB value is not allocated in a test on this box.
#[test]
fn the_value_limit_refuses_one_byte_past_the_bound() {
    assert!(crate::kv::check_entry(MAX_KEY_BYTES, MAX_VALUE_BYTES).is_ok());
    let e = crate::kv::check_entry(1, MAX_VALUE_BYTES + 1).unwrap_err();
    assert!(matches!(e, StoreError::Limit { what: "value bytes", .. }), "{e:?}");
}

/// D4: `create` leaves the file synced and still byte-identical to `create_bytes`, with a
/// bare file name too (its directory is "", synced as ".").
#[test]
fn create_syncs_and_accepts_a_bare_file_name() {
    let dir = std::env::temp_dir().join("bebop_create_sync");
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("s.store");
    let st = Store::create(p.to_str().unwrap(), MIN_BYTES).unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), st.to_bytes());
    assert!(crate::sync_parent_dir("bare-name.store").is_ok(), "a bare name syncs '.'");
    let _ = std::fs::remove_dir_all(&dir);
}
