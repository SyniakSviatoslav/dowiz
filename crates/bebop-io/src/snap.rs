//! One published generation, as readers hold it (W-ES-P3, design D1; chunked in W-ES-P19a).
//!
//! `Store` is `Vec<i64>` and can never lend `&[u8]` without `unsafe` (forbidden here), so a
//! generation is ALSO kept as its trimmed byte image -- exactly `Store::to_bytes_trimmed` -- cut
//! into CHUNKS of `CHUNK_BYTES`, each an `Arc<Vec<u8>>`. A `ReadTxn` holds the `Arc<Snap>`; a
//! value it returns is a slice of one chunk. A commit builds the next generation's image by
//! PATCHING the previous one with the cells the commit plan writes (`Snap::patched`): a chunk no
//! reader holds is patched in place, a chunk a reader holds is copied FIRST (`Arc::make_mut`), so
//! a reader's bytes never change under it (AC-ES-10) and a commit copies at most the chunks it
//! touches (the superblock pages and the arena tail: 2-3 chunks), never the image. Before P19a
//! the WHOLE image was cloned whenever any reader was alive: 459 us per 4-key commit at 1e4 keys
//! and 5062 us at 1e5 (es-p19.design.md §5.3); `copied_bytes` is the counter that pins the bound.

use std::cell::Cell;
use std::sync::{Arc, OnceLock};

use bebop_kv::zc::{Checked, Image, KvIn};

use crate::db::{Generation, Verify};
use crate::plan::Step;
use crate::refusal::Refusal;
use crate::{Cells, Store, Superblock, ARENA};

/// Bytes per chunk: 64 KiB = 8192 cells. The first chunk holds both superblock pages and the
/// start of the arena; a 4-key tree commit (~1030 cells) touches it and one or two tail chunks.
pub(crate) const CHUNK_BYTES: usize = 64 * 1024;

thread_local! {
    /// Bytes `Snap::patched` COPIED on this thread because a reader still held them. Test
    /// instrumentation (a commit runs on its caller's thread): with a reader alive across k
    /// commits it must stay below k x 3 x `CHUNK_BYTES`, never k x image.
    static COPIED: Cell<u64> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn copied_bytes() -> u64 {
    COPIED.with(Cell::get)
}

/// A generation's bytes, shared: `len` bytes in chunks of `CHUNK_BYTES` (the last one shorter).
#[derive(Clone)]
pub struct SnapImage {
    chunks: Arc<Vec<Arc<Vec<u8>>>>,
    len: usize,
}

impl SnapImage {
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> SnapImage {
        let len = bytes.len();
        let chunks = bytes.chunks(CHUNK_BYTES).map(|c| Arc::new(c.to_vec())).collect();
        SnapImage { chunks: Arc::new(chunks), len }
    }

    #[cfg(test)]
    pub(crate) fn byte_len(&self) -> usize {
        self.len
    }

    /// The image flat (`ReadTxn::image`, `Db::snapshot_bytes`, tests): O(image).
    pub(crate) fn to_vec(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.len);
        for c in self.chunks.iter() {
            v.extend_from_slice(c);
        }
        v
    }

    /// Bytes chunk `i` holds in an image of `len` bytes.
    fn chunk_len(i: usize, len: usize) -> usize {
        len.saturating_sub(i * CHUNK_BYTES).min(CHUNK_BYTES)
    }

    /// Chunk `c`, writable: in place when no reader holds it, else a copy (counted).
    fn own(c: &mut Arc<Vec<u8>>) -> &mut Vec<u8> {
        if Arc::get_mut(c).is_none() {
            COPIED.with(|n| n.set(n.get() + c.len() as u64));
        }
        Arc::make_mut(c)
    }

    /// Resize to `len` bytes (chunks added zero-filled, or cut), then write every `Step::Write`
    /// range from `st`. Only the chunks a step touches are written, and only those a reader
    /// holds are copied first.
    pub(crate) fn patch(&mut self, st: &Store, len: usize, steps: &[Step]) -> Result<(), Refusal> {
        let chunks = Arc::make_mut(&mut self.chunks);
        let n = len.div_ceil(CHUNK_BYTES);
        chunks.truncate(n);
        for (i, c) in chunks.iter_mut().enumerate() {
            let want = Self::chunk_len(i, len);
            if c.len() != want {
                Self::own(c).resize(want, 0);
            }
        }
        while chunks.len() < n {
            let i = chunks.len();
            chunks.push(Arc::new(vec![0; Self::chunk_len(i, len)]));
        }
        self.len = len;
        for s in steps {
            if let Step::Write { off, n } = *s {
                let end = off.checked_add(n).ok_or(Refusal::Plan("a step's range overflows"))?;
                let src = st.cells.get(off..end).ok_or(Refusal::Plan("a step names cells the staged image does not hold"))?;
                if end.checked_mul(8).is_none_or(|e| e > len) {
                    return Err(Refusal::Plan("a step writes past the generation's trimmed image"));
                }
                // A cell never straddles chunks (CHUNK_BYTES is a multiple of 8): copy chunk by chunk.
                let (mut b, mut src) = (off * 8, src);
                while !src.is_empty() {
                    let (ci, o) = (b / CHUNK_BYTES, b % CHUNK_BYTES);
                    let take = ((CHUNK_BYTES - o) / 8).min(src.len());
                    let (now, rest) = src.split_at(take);
                    let chunk = chunks.get_mut(ci).ok_or(Refusal::Plan("a step writes past the generation's chunks"))?;
                    let dst = Self::own(chunk).get_mut(o..o + take * 8).ok_or(Refusal::Plan("a step writes past a chunk"))?;
                    for (d, c) in dst.chunks_exact_mut(8).zip(now) {
                        d.copy_from_slice(&c.to_le_bytes());
                    }
                    b += take * 8;
                    src = rest;
                }
            }
        }
        Ok(())
    }
}

impl Cells for SnapImage {
    #[inline]
    fn cell_at(&self, i: usize) -> i64 {
        match i.checked_mul(8).and_then(|a| self.bytes_at(a, 8)) {
            Some(w) => i64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]),
            None => 0,
        }
    }
    fn n_cells(&self) -> usize {
        self.len / 8
    }
}

impl Image for SnapImage {
    /// `len` bytes from `at`, borrowed when they sit inside one chunk; a run that straddles two
    /// chunks is `None` and the reader gathers it (bebop-kv `bytes_of`).
    fn bytes_at(&self, at: usize, len: usize) -> Option<&[u8]> {
        if at.checked_add(len)? > self.len {
            return None;
        }
        let (ci, o) = (at / CHUNK_BYTES, at % CHUNK_BYTES);
        self.chunks.get(ci)?.get(o..o.checked_add(len)?)
    }
}

/// A published generation.
pub(crate) struct Snap {
    pub(crate) kv: KvIn<SnapImage>,
    pub(crate) img: SnapImage,
    pub(crate) sb: Superblock,
    pub(crate) capacity: i64,
    /// The image flat, made on the first `bytes()` (tests and the FFI); never by a commit.
    flat: OnceLock<Vec<u8>>,
}

impl Snap {
    /// Read `bytes` as a KV generation (`build_img` over one chunking of them).
    pub(crate) fn build(bytes: Vec<u8>, checked: Option<Checked>, depth: Verify) -> Result<Snap, Refusal> {
        Snap::build_img(SnapImage::from_bytes(bytes), checked, depth)
    }

    /// Read `img` as a KV generation. `checked` = these bytes extend ones already opened (a
    /// commit of ours): no crc, the key order carried over. Otherwise `depth` says how much is
    /// proved here: `Anchor` trusts what recovery proved (`KvIn::open_lazy`), `Reachable` checks
    /// every object the root reaches, `WholeArena` also walks every object of the arena.
    pub(crate) fn build_img(img: SnapImage, checked: Option<Checked>, depth: Verify) -> Result<Snap, Refusal> {
        let sb = crate::format::pick_checked_in(&img)?;
        let root = crate::root_in(&img).ok_or(Refusal::NotKv)?;
        let ver = bebop_kv::delta::version_in(&img, root);
        let v4 = ver == bebop_kv::tree::VERSION_TREE && bebop_kv::tree::v4_here();
        if ver > bebop_kv::delta::VERSION_DELTA && !v4 {
            return Err(Refusal::UnknownVersion(ver));
        }
        let kv = match (checked, depth) {
            (Some(ck), _) => KvIn::reopen(img.clone(), ck),
            (None, Verify::Anchor) => KvIn::open_lazy(img.clone()),
            (None, _) => KvIn::open(img.clone()),
        }
        .map_err(Refusal::from)?;
        if checked.is_none() && depth == Verify::WholeArena {
            crate::sim::walk(&img, ARENA, sb.arena_used.max(0) as usize).map_err(|at| Refusal::ObjectBad { at })?;
        }
        let capacity = img.cell_at(sb.at + 12);
        Ok(Snap { kv, img, sb, capacity, flat: OnceLock::new() })
    }

    pub(crate) fn generation(&self) -> Generation {
        Generation(self.sb.generation)
    }

    /// The image flat, borrowed (made once, on demand).
    pub(crate) fn bytes(&self) -> &[u8] {
        self.flat.get_or_init(|| self.img.to_vec())
    }

    /// The image of the generation `st` stages, from `old`'s by the commit plan's writes:
    /// resized to `st`'s trimmed length, then every `Step::Write` range copied from `st`. Takes
    /// `old` by value so that, when no reader holds it, the chunks are patched in place; a
    /// reader that holds it keeps its chunks and the writer gets copies of the ones it touches.
    pub(crate) fn patched(old: Arc<Snap>, st: &Store, steps: &[Step]) -> Result<SnapImage, Refusal> {
        let mut img = match Arc::try_unwrap(old) {
            Ok(snap) => {
                let Snap { kv, img, .. } = snap;
                drop(kv);
                img
            }
            Err(shared) => shared.img.clone(),
        };
        img.patch(st, trimmed_cells(st) * 8, steps)?;
        Ok(img)
    }
}

/// `Store::to_bytes_trimmed`'s length, in cells.
pub(crate) fn trimmed_cells(st: &Store) -> usize {
    let at = match st.pick() {
        Some(sb) => sb.arena_used.max(ARENA as i64) as usize,
        None => st.cells.len(),
    };
    at.min(st.cells.len())
}

#[cfg(test)]
mod tests;
