//! W-ES-P19a: the chunked snapshot. A reader alive across commits costs the writer the chunks
//! it touches, never the image (`copied_bytes`); a reader's chunks never change under it.

use super::*;
use crate::{Db, Detached, Durability, Options};

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i.wrapping_mul(31) ^ (i >> 8)) as u8).collect()
}

#[test]
fn a_chunked_image_reads_like_the_flat_one() {
    let len = CHUNK_BYTES * 5 / 2;
    let flat = pattern(len);
    let img = SnapImage::from_bytes(flat.clone());
    assert_eq!(img.byte_len(), len);
    assert_eq!(img.n_cells(), len / 8);
    assert_eq!(img.to_vec(), flat);
    for i in [0, 1, CHUNK_BYTES / 8 - 1, CHUNK_BYTES / 8, CHUNK_BYTES / 8 + 1, len / 8 - 1] {
        let w = &flat[i * 8..i * 8 + 8];
        assert_eq!(img.cell_at(i), i64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]), "cell {i}");
    }
    assert_eq!(img.cell_at(len / 8), 0, "past the end reads 0");
    assert_eq!(img.bytes_at(CHUNK_BYTES - 16, 16), Some(&flat[CHUNK_BYTES - 16..CHUNK_BYTES]), "a run inside a chunk is borrowed");
    assert_eq!(img.bytes_at(CHUNK_BYTES - 8, 16), None, "a run across two chunks is gathered, not borrowed");
    assert_eq!(img.bytes_at(len - 4, 8), None, "a run past the end is None");
}

/// `patch` copies exactly the chunks a holder shares, grows by whole chunks, and leaves the
/// holder's bytes as they were.
#[test]
fn patch_copies_only_the_chunks_a_reader_holds() {
    let len0 = CHUNK_BYTES * 5 / 2;
    let flat = pattern(len0);
    let mut img = SnapImage::from_bytes(flat.clone());
    let held = img.clone();
    let st = Store { cells: vec![0x1122_3344_5566_7788u64 as i64; CHUNK_BYTES * 7 / 16] };
    let before = copied_bytes();
    // one cell in chunk 2, and the image grows to 3.5 chunks
    let len1 = CHUNK_BYTES * 7 / 2;
    let off = (CHUNK_BYTES * 2 + 64) / 8;
    img.patch(&st, len1, &[Step::Write { off, n: 1 }]).unwrap();
    assert_eq!(copied_bytes() - before, CHUNK_BYTES as u64 / 2, "only the held half-chunk 2 was copied (then grown)");
    assert_eq!(img.byte_len(), len1);
    assert_eq!(img.cell_at(off), 0x1122_3344_5566_7788u64 as i64);
    assert_eq!(held.to_vec(), flat, "the holder's bytes changed under it");
    assert_eq!(&img.to_vec()[..off * 8], &flat[..off * 8]);
    assert_eq!(img.to_vec()[len0..].iter().filter(|&&b| b != 0).count(), 0, "new room is zero-filled");
    // no holder: patched in place, nothing copied
    drop(held);
    let before = copied_bytes();
    img.patch(&st, len1, &[Step::Write { off: 3, n: 2 }]).unwrap();
    assert_eq!(copied_bytes() - before, 0, "a chunk nobody holds is patched in place");
    assert_eq!(img.cell_at(4), 0x1122_3344_5566_7788u64 as i64);
    // a step past the trimmed image is refused, not written
    assert!(matches!(img.patch(&st, len1, &[Step::Write { off: len1 / 8 - 1, n: 2 }]), Err(Refusal::Plan(_))));
}

#[cfg(feature = "tree")]
const KEYS: usize = 4000;
#[cfg(feature = "tree")]
const VAL: usize = 400;

#[cfg(feature = "tree")]
fn big_tree_db() -> Db<Detached> {
    let db = Db::create_detached(Options::detached().create_tree(true).initial_bytes(8 << 20)).unwrap();
    let mut w = db.begin_write().unwrap();
    for i in 0..KEYS {
        w.put(format!("key{i:08}").as_bytes(), &vec![i as u8; VAL]).unwrap();
    }
    w.commit(Durability::None).unwrap();
    db
}

/// `k` 4-key commits; with `readers`, a fresh reader is taken before each and held across it,
/// as an application with readers always present does (the P1 `reader_held=true` loop).
#[cfg(feature = "tree")]
fn commits(db: &Db<Detached>, k: usize, readers: bool) {
    let mut held = None;
    for c in 0..k {
        if readers {
            held = Some(db.begin_read());
        }
        let mut w = db.begin_write().unwrap();
        for j in 0..4 {
            let i = (c * 977 + j * 1013) % KEYS;
            w.put(format!("key{i:08}").as_bytes(), &vec![(c + j) as u8; VAL]).unwrap();
        }
        w.commit(Durability::None).unwrap();
    }
    drop(held);
}

/// RED PROOF for P19a: before it, `Snap::patched` cloned the WHOLE image whenever a reader held
/// the latest generation (k commits = k x image: here 100 x ~1.9 MB = 190 MB); now at most the
/// chunks a commit touches (the superblock chunk and the arena tail), 3 x CHUNK_BYTES per commit.
/// The mutant `for c in chunks.iter_mut() { Self::own(c); }` at the top of `patch` (copy every
/// shared chunk = the old shape) goes RED here: measured 2026-10-30, 100 x image copied.
#[cfg(feature = "tree")]
#[test]
fn a_commit_with_a_reader_alive_copies_chunks_not_the_image() {
    let db = big_tree_db();
    let image = db.snapshot_bytes().len() as u64;
    assert!(image > 20 * CHUNK_BYTES as u64, "the fixture must be many chunks: {image} bytes");
    let r = db.begin_read();
    let g = r.generation();
    let probe = r.get(b"key00000007").unwrap().unwrap().into_owned();
    let before = copied_bytes();
    let k = 100;
    commits(&db, k, true);
    let copied = copied_bytes() - before;
    assert!(copied > 0, "a reader was alive, so the touched chunks had to be copied");
    assert!(copied <= (k * 3 * CHUNK_BYTES) as u64, "copied {copied} bytes over {k} commits: more than 3 chunks per commit (the whole image would be {})", k as u64 * image);
    assert_eq!(r.generation(), g);
    assert_eq!(r.get(b"key00000007").unwrap().unwrap(), &probe[..], "a reader saw a commit");
    drop(r);
    assert_eq!(db.snapshot_bytes(), db.writer_image(), "the published snapshot is the writer's image");
}

/// The positive twin: with no reader alive the chunks are patched in place and nothing is copied.
#[cfg(feature = "tree")]
#[test]
fn a_commit_with_no_reader_copies_nothing() {
    let db = big_tree_db();
    let before = copied_bytes();
    commits(&db, 50, false);
    assert_eq!(copied_bytes() - before, 0, "nobody held the chunks");
    assert_eq!(db.snapshot_bytes(), db.writer_image());
    db.verify(crate::Verify::WholeArena).unwrap();
}
