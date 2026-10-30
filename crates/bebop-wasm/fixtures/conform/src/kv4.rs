// SPDX-License-Identifier: MIT OR Apache-2.0
//! THE KV v4 FIXTURES (W-ES-P8D, design es-p8 §P8d; FORMAT §9.1 v4), WRITTEN BY THE RUST WRITER
//! (bebop-kv's `tree`, cargo feature `tree` on in this crate only). Deterministic: a re-run is
//! byte-identical. Every one is judged by tools/gates/conformance.sh on all five readers.
//!
//!   kv4small  kv3max's 27 entries migrated (`migrate_to_tree`, FANOUT 32): ONE leaf, depth 1, and
//!             the SAME n and fold as kv3max -- the fold law across versions.
//!   kv4deep   150 keys put by 6 path-copy commits at MIN_FANOUT (`commit_tree_bytes`), interleaved so
//!             leaves split in the middle and the root grows: depth >= 3, values 0..40 bytes, old
//!             subtrees shared by address with earlier generations.
//!   kv4del    kv4deep, then 3 commits that remove 110 keys (small leaves merge, the root collapses)
//!             and overwrite 5.
//!   kv4zero   the empty v4 map: DEPTH 0, TREE 0, n 0 -- the fold is the FNV offset basis, as kv0.
//!   kv4badn   kv4deep, ONE bit flipped in an INTERNAL node's key area (crc not re-sealed): 10.
//!   kv4badv   kv4deep, bit 0 of the first value cell of the first non-empty value: 10.
//!   kv4badf   kv4deep, the top node's child-0 ref made FORWARD (+5) and the node re-sealed: rule 1, 2.
//!   kv4badd   kv4deep, the root's DEPTH claim + 1 and the root re-sealed: rule 2, 2.
//!   kv4stale  kv4badf WITHOUT the re-anchor (W-ES-P10, FORMAT §7 phase P2): LAST_COMMIT, 41.
//!   kv4old    kv4deep, one bit in a REACHABLE object of an OLDER commit (below the anchor's mark): 10,
//!             and the lazy open's documented guarantee under `conformance.sh --lazy`.
//! Structural refusals (rules 1-5 of FORMAT §9.1 v4) are NOT_A_KV (2) on every reader; a crc is 10.
use bebop_image::Store;
use bebop_kv::delta::KeyOp;
use bebop_kv::tree::{commit_tree_bytes, migrate_to_tree, stage_tree, TreeIn, FANOUT, MIN_FANOUT};

/// The smallest fanout the writer accepts: the deepest tree for 150 keys, every split and merge
/// (W-ES-HARD: the one-child-node writer defect this was 8 to avoid was fixed by W-ES-P8E).
const B: usize = MIN_FANOUT;

fn deep_key(i: usize) -> String {
    format!("k/{:03}{}", (i * 37) % 150, if i % 7 == 0 { "/longer-tail" } else { "" })
}
fn deep_val(i: usize, round: usize) -> Vec<u8> {
    let n = [0usize, 1, 7, 8, 9, 40][(i + round) % 6];
    (0..n).map(|j| (i * 13 + j * 5 + round) as u8).collect()
}

/// Apply `ops` (any order, one per key) as one tree commit at fanout B.
fn commit(st: &mut Store, mut ops: Vec<(Vec<u8>, Option<Vec<u8>>)>) {
    ops.sort_by(|a, b| a.0.cmp(&b.0));
    let kops: Vec<KeyOp> = ops
        .iter()
        .map(|(k, v)| match v {
            Some(v) => KeyOp::Put(k, v),
            None => KeyOp::Remove(k),
        })
        .collect();
    commit_tree_bytes(st, &kops, B).expect("tree commit");
}

/// (top, depth) of the live tree.
fn head(bytes: &[u8]) -> (usize, usize, usize) {
    let st = Store::from_bytes(bytes);
    let root = st.root().expect("root");
    let t = TreeIn::at(&st, root).expect("v4");
    (root, t.top(), t.depth())
}

/// Child 0 of an internal node (payload cell 1 = C_0, an object-relative ref).
fn child0(st: &Store, node: usize) -> usize {
    (node as i64 + st.get(node, 1)) as usize
}

fn reseal(st: &mut Store, obj: usize) {
    st.seal(obj);
}

/// Re-anchor the live commit (FORMAT §7: superblock cell SB_ANC = `(mark << 32) | crc32(cells[mark..
/// arena_used))`, then the superblock's own crc) after a cell inside its range was changed. Without
/// this the ANCHOR answers before the tree rule the fixture exists to test -- measured 2026-10-30:
/// bebop.bin's st_open fell back to generation k-1 (`kv.bin g` 5, n=125) and the C ABI refused
/// LAST_COMMIT (41) on kv4badf/kv4badd, while native/wasm32/python, which have no anchor logic, said
/// 2. A torn last commit is `torn`'s and --prove's business; these four are one corrupted cell each.
fn reanchor(st: &mut Store) {
    use bebop_image::format::{anchor_of, SB_ANC, SB_CRC, SB_USED};
    let sb = st.pick().expect("a live superblock").at;
    let mark = st.cells[sb + SB_ANC] >> 32;
    let used = st.cells[sb + SB_USED];
    st.cells[sb + SB_ANC] = anchor_of(&*st, mark, used);
    st.cells[sb + SB_CRC] = bebop_image::crc32_cells(&st.cells, sb, 15) as i64;
}

pub fn write(dir: &str, kv3max: &[u8], say: &dyn Fn(&str, &[u8])) {
    let p = |f: &str| format!("{dir}/{f}.store");
    let put = |name: &str, b: &[u8]| {
        std::fs::write(p(name), b).expect("write fixture");
        say(name, b);
    };

    let small = migrate_to_tree(&Store::from_bytes(kv3max), 1 << 20, FANOUT).expect("migrate");
    assert_eq!(head(&small).2, 1, "kv4small must be one leaf");
    put("kv4small", &small);

    let mut st = Store::create_bytes(1 << 20).expect("create");
    for round in 0..6 {
        let ops = (0..150).filter(|i| i % 6 == round).map(|i| (deep_key(i).into_bytes(), Some(deep_val(i, round)))).collect();
        commit(&mut st, ops);
    }
    let deep = st.to_bytes_trimmed();
    let (_, _, d) = head(&deep);
    assert!(d >= 3, "kv4deep must be at least 3 deep, is {d}");
    put("kv4deep", &deep);

    for round in 0..3 {
        let mut ops: Vec<(Vec<u8>, Option<Vec<u8>>)> =
            (0..150).filter(|i| i % 4 != 0 && i % 3 == round).map(|i| (deep_key(i).into_bytes(), None)).collect();
        ops.extend((0..150).filter(|i| i % 4 == 0 && i % 30 == round * 4).map(|i| (deep_key(i).into_bytes(), Some(deep_val(i, 9)))));
        commit(&mut st, ops);
    }
    let del = st.to_bytes_trimmed();
    put("kv4del", &del);

    let mut z = Store::create_bytes(16 * 1024).expect("create");
    let (tx, root) = stage_tree(&mut z, &[], B).expect("empty tree");
    z.commit_bytes(&tx, root);
    put("kv4zero", &z.to_bytes_trimmed());

    let (root, top, depth) = head(&deep);
    let base = Store::from_bytes(&deep);
    let inner = child0(&base, top); // depth >= 3: an internal node
    let mut leaf = inner;
    while base.obj_digest(leaf) != bebop_kv::tree::DIGEST_KV4_LEAF {
        leaf = child0(&base, leaf);
    }
    let cut = |f: &dyn Fn(&mut Store)| {
        let mut s = Store::from_bytes(&deep);
        f(&mut s);
        let b = s.to_bytes();
        b[..deep.len()].to_vec()
    };

    let last = inner + 2 + base.obj_cells(inner) - 1;
    put("kv4badn", &cut(&|s| {
        s.cells[last] ^= 1;
        reanchor(s);
    }));

    let m = (base.get(leaf, 0) as u64 & 0xFFFF) as usize;
    let val = (0..m).map(|i| (leaf as i64 + base.get(leaf, 2 + 2 * i)) as usize).find(|&v| base.get(v, 0) > 0).expect("a non-empty value");
    println!("kv4badn: internal node {inner} cell {last} bit 0; kv4badv: value {val} (leaf {leaf}) cell {} bit 0", val + 3);
    put("kv4badv", &cut(&|s| {
        s.cells[val + 3] ^= 1;
        reanchor(s);
    }));

    put("kv4badf", &cut(&|s| {
        s.cells[top + 2 + 1] = 5;
        reseal(s, top);
        reanchor(s);
    }));
    println!("kv4badf: top {top} payload cell 1 = +5; kv4badd: root {root} DEPTH {depth} -> {}", depth + 1);
    put("kv4badd", &cut(&|s| {
        s.cells[root + 2 + 7] = depth as i64 + 1;
        reseal(s, root);
        reanchor(s);
    }));

    // W-ES-P10 (FORMAT §7 phase P2): the two fixtures of the LAST-COMMIT rule.
    //   kv4stale  kv4badf WITHOUT the re-anchor: the top node re-sealed over its changed byte, so
    //             every object checks, but the anchor was recorded over the old bytes -> 41 on every
    //             reader (the case that got three answers before P10).
    //   kv4old    a bit flipped in a REACHABLE object of an OLDER commit (below the anchor's mark):
    //             the anchor still holds, so an open at Verify::Anchor succeeds and reads it, and
    //             verify(Reachable) -- what the five conformance readers do -- names it: 10.
    put("kv4stale", &cut(&|s| {
        s.cells[top + 2 + 1] = 5;
        reseal(s, top);
    }));
    let old = old_reachable(&base);
    println!("kv4stale: top {top} payload cell 1 = +5, re-sealed, anchor NOT recomputed; kv4old: object {old} (below the mark) cell {} bit 0", old + 2);
    put("kv4old", &cut(&|s| {
        s.cells[old + 2] ^= 1;
    }));
}

/// The first object below the live anchor's mark (an OLDER commit's) that the KV reaches: found by
/// trying each, a bit flipped, against the native reader, which must name exactly it.
fn old_reachable(base: &Store) -> usize {
    use bebop_image::format::{anchor_in, Anchor, SB_ANC};
    use bebop_kv::tree::TreeError;
    use bebop_kv::verify::KvError;
    let sb = base.pick().expect("a live superblock");
    let mark = ((base.cells[sb.at + SB_ANC] as u64) >> 32) as usize;
    assert!(mark > bebop_image::ARENA, "kv4deep's last commit must not be its first");
    let mut o = bebop_image::ARENA;
    while o < mark {
        let len = base.obj_cells(o);
        if len > 0 {
            let mut s = Store { cells: base.cells.clone() };
            s.cells[o + 2] ^= 1;
            assert!(matches!(anchor_in(&s, &sb), Anchor::Holds { .. }), "the flip at {o} must not touch the anchor");
            if matches!(bebop_kv::Kv::load_checked(&s), Err(KvError::Tree(TreeError::BadCrc(b))) if b.obj == o) {
                return o;
            }
        }
        o += 2 + len;
    }
    panic!("no reachable object below the mark {mark}");
}
