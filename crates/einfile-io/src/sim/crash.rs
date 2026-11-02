//! THE CRASH POINTS of a recorded write sequence, every one named (R-EMBED-STORE §4.3).
//!
//! POWER LOSS (`Model::Power`). The record is cut into WINDOWS at its barriers. Before window
//! w every earlier window is durable. Inside it, the writes touch a set T of 512-byte sectors
//! (64 cells); after a crash each sector of T is old or new, independently, and at most one
//! of the old ones may be TORN (part new, part old). Enumerated:
//!   |T| <= EXHAUSTIVE: every subset of T, and for each, every not-new sector torn in each
//!                      `Tear` shape;
//!   |T| >  EXHAUSTIVE: the eight NAMED classes of §4.3 (all, empty, only-first, only-last,
//!                      all-but-first, all-but-last, even, odd) x a tear of the first or the
//!                      last not-new sector. An explicit bound, not a sample.
//! Plus one point after the last barrier, when the record ends with one.
//!
//! PROCESS CRASH (`Model::Process`): one page cache, writes land in issue order, so the
//! states are the prefixes of the writes. What `Durability::None` promises.
//!
//! A PUBLISH (`SimOp::Publish`, W-ES-HARD: a compaction into a sibling file) under power loss.
//! `Synced`: the old file with any subset of the window's writes (the rename not durable), or the
//! new file whole; it ends its window (the directory fsync is a barrier). `Deferred`: the old file
//! keeps the name and gets no more writes -- every later write goes to the sibling -- so until the
//! next barrier the only state at the name is the old file (with the subsets of the writes BEFORE
//! the publish); that barrier syncs the sibling, then renames it: the old file whole or the new
//! file whole with every write since the publish. `RenamedUnsynced` (the mutant) renames at once:
//! the new file EMPTY is a state, and the generations durable before it are gone.

use std::fmt;

use super::{apply, PublishHow, SimOp};

/// Cells in one 512-byte sector.
pub const SECTOR_CELLS: usize = 64;
/// Up to this many sectors per window every subset is enumerated.
pub const EXHAUSTIVE: usize = 6;

/// The shapes a torn sector may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tear {
    /// No torn sectors.
    None,
    /// Half new, half old -- both halves (the doc names only "first half new").
    Half,
    /// Every cut at a cell, new before it or new after it (126 shapes): T-ES-22's model.
    Cells,
}

/// Which crash model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Power(Tear),
    Process,
}

/// A torn sector: the first `n` cells new (`front`) or the last `n` cells new.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Torn {
    pub sector: usize,
    pub front: bool,
    pub n: usize,
}

/// One named crash point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Point {
    pub ordinal: usize,
    /// Barriers durable before the crash.
    pub window: usize,
    /// Sectors that landed whole (Process model: writes that landed).
    pub new: Vec<usize>,
    pub torn: Option<Torn>,
    /// The named class, above `EXHAUSTIVE`.
    pub class: Option<&'static str>,
    /// After the last barrier / a process-crash prefix.
    pub kind: &'static str,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{} {} after {} barrier(s): new {:?}", self.ordinal, self.kind, self.window, self.new)?;
        if let Some(c) = self.class {
            write!(f, " ({c})")?;
        }
        if let Some(t) = self.torn {
            write!(f, " torn sector {} {} {} cells new", t.sector, if t.front { "first" } else { "last" }, t.n)?;
        }
        Ok(())
    }
}

/// Visit every crash state of `ops` over the durable `base`. Returns how many there were.
pub fn each_state(base: &[i64], ops: &[SimOp], model: Model, mut visit: impl FnMut(&Point, &[i64])) -> usize {
    let mut ord = 0usize;
    let mut emit = |p: Point, img: &[i64]| {
        visit(&Point { ordinal: ord, ..p }, img);
        ord += 1;
    };
    // A deferred publish's sibling: what the writer writes until the barrier that names it.
    let mut staged: Option<Vec<i64>> = None;
    if model == Model::Process {
        let mut img = base.to_vec();
        let blank = |n: Vec<usize>| Point { ordinal: 0, window: 0, new: n, torn: None, class: None, kind: "process crash" };
        emit(blank(vec![]), &img);
        for (i, op) in ops.iter().enumerate() {
            match op {
                SimOp::Write { off, cells } => {
                    apply(staged.as_mut().unwrap_or(&mut img), *off, cells);
                    emit(blank(vec![i]), &img);
                }
                SimOp::Publish { cells, how: PublishHow::Deferred } => {
                    staged = Some(cells.clone());
                    emit(Point { kind: "process crash after the deferred publish", ..blank(vec![i]) }, &img);
                }
                SimOp::Publish { cells, .. } => {
                    img = cells.clone();
                    staged = None;
                    emit(Point { kind: "process crash after the publish", ..blank(vec![i]) }, &img);
                }
                SimOp::Barrier => {
                    if let Some(s) = staged.take() {
                        img = s;
                        emit(Point { kind: "process crash after the barrier's rename", ..blank(vec![i]) }, &img);
                    }
                }
            }
        }
        return ord;
    }
    let Model::Power(tear) = model else { unreachable!() };
    let point = |window: usize, kind: &'static str| Point { ordinal: 0, window, new: vec![], torn: None, class: None, kind };
    let mut durable = base.to_vec();
    let mut window = 0usize;
    let mut writes: Vec<(usize, &[i64])> = Vec::new();
    let mut ended = false;
    for op in ops.iter().map(Some).chain([None]) {
        match op {
            Some(SimOp::Write { off, cells }) => {
                match staged.as_mut() {
                    Some(s) => apply(s, *off, cells),
                    None => writes.push((*off, cells.as_slice())),
                }
                ended = false;
            }
            Some(SimOp::Publish { cells, how }) => {
                // The old file, with every subset of the writes it got in this window: what a
                // crash leaves while the rename is not durable. What landed in it is gone with
                // the rename, so the pending image is not kept.
                if writes.is_empty() {
                    emit(point(window, "publish: the old file"), &durable);
                } else {
                    window_states(&durable, &writes, window, tear, &mut emit);
                    writes.clear();
                }
                match how {
                    PublishHow::Synced => {
                        durable = cells.clone();
                        staged = None;
                        emit(point(window, "publish: the new file"), &durable);
                        window += 1;
                    }
                    PublishHow::Deferred => staged = Some(cells.clone()),
                    PublishHow::RenamedUnsynced => {
                        emit(point(window, "publish renamed unsynced: the new file EMPTY"), &vec![0; cells.len()]);
                        durable = cells.clone();
                        staged = None;
                        emit(point(window, "publish renamed unsynced: the new file"), &durable);
                    }
                }
                ended = false;
            }
            Some(SimOp::Barrier) | None => {
                if !writes.is_empty() {
                    let pending = window_states(&durable, &writes, window, tear, &mut emit);
                    durable = pending;
                    writes.clear();
                }
                if let (true, Some(s)) = (op.is_some(), staged.take()) {
                    // The deferred rename: the sibling synced whole first, then the name moves.
                    emit(point(window, "publish deferred: the old file"), &durable);
                    durable = s;
                    emit(point(window, "publish deferred: the new file"), &durable);
                }
                if op.is_some() {
                    window += 1;
                    ended = true;
                }
            }
        }
    }
    if ended {
        let p = Point { ordinal: 0, window, new: vec![], torn: None, class: None, kind: "after the last barrier" };
        emit(p, &durable);
    }
    ord
}

/// Every state of one window; returns the image with the whole window landed.
fn window_states(durable: &[i64], writes: &[(usize, &[i64])], window: usize, tear: Tear, emit: &mut impl FnMut(Point, &[i64])) -> Vec<i64> {
    let mut pending = durable.to_vec();
    let mut t: Vec<usize> = Vec::new();
    for &(off, cells) in writes {
        apply(&mut pending, off, cells);
        let last = (off + cells.len().max(1) - 1) / SECTOR_CELLS;
        t.extend(off / SECTOR_CELLS..=last);
    }
    t.sort_unstable();
    t.dedup();
    let mut old = durable.to_vec();
    old.resize(pending.len(), 0);
    for (mask, class) in subsets(t.len()) {
        let new: Vec<usize> = (0..t.len()).filter(|&i| mask[i]).map(|i| t[i]).collect();
        let mut img = old.clone();
        for &s in &new {
            copy(&mut img, &pending, s * SECTOR_CELLS, SECTOR_CELLS);
        }
        let base = Point { ordinal: 0, window, new: new.clone(), torn: None, class, kind: "power loss" };
        emit(base.clone(), &img);
        let rest: Vec<usize> = t.iter().copied().filter(|s| !new.contains(s)).collect();
        let torn_at: Vec<usize> = match (class, rest.first(), rest.last()) {
            (None, _, _) => rest.clone(),
            (Some(_), Some(&a), Some(&b)) if a != b => vec![a, b],
            (Some(_), Some(&a), _) => vec![a],
            _ => vec![],
        };
        for s in torn_at {
            for tr in shapes(tear, s) {
                let mut timg = img.clone();
                let lo = s * SECTOR_CELLS + if tr.front { 0 } else { SECTOR_CELLS - tr.n };
                copy(&mut timg, &pending, lo, tr.n);
                emit(Point { torn: Some(tr), ..base.clone() }, &timg);
            }
        }
    }
    pending
}

/// `n` cells of `from` into `to` at `lo`, within both.
fn copy(to: &mut [i64], from: &[i64], lo: usize, n: usize) {
    let hi = (lo + n).min(from.len()).min(to.len());
    if lo < hi {
        to[lo..hi].copy_from_slice(&from[lo..hi]);
    }
}

/// The torn shapes of sector `s`.
fn shapes(tear: Tear, s: usize) -> Vec<Torn> {
    let cuts: Vec<usize> = match tear {
        Tear::None => vec![],
        Tear::Half => vec![SECTOR_CELLS / 2],
        Tear::Cells => (1..SECTOR_CELLS).collect(),
    };
    cuts.iter().flat_map(|&n| [Torn { sector: s, front: true, n }, Torn { sector: s, front: false, n }]).collect()
}

/// The subsets of `t` sectors, one flag per sector: all of them up to `EXHAUSTIVE`, else the
/// eight classes.
fn subsets(t: usize) -> Vec<(Vec<bool>, Option<&'static str>)> {
    if t <= EXHAUSTIVE {
        return (0..1u32 << t).map(|m| ((0..t).map(|i| m >> i & 1 == 1).collect(), None)).collect();
    }
    let class = |name: &'static str, f: &dyn Fn(usize) -> bool| ((0..t).map(f).collect(), Some(name));
    vec![
        class("all", &|_| true),
        class("empty", &|_| false),
        class("only-first", &|i| i == 0),
        class("only-last", &|i| i == t - 1),
        class("all-but-first", &|i| i != 0),
        class("all-but-last", &|i| i != t - 1),
        class("even", &|i| i % 2 == 0),
        class("odd", &|i| i % 2 == 1),
    ]
}

#[cfg(test)]
mod tests;
