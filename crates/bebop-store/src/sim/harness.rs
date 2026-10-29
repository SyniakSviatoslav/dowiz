//! THE HARNESS: every crash state of a recorded flush, recovered and checked (§4.3). In each:
//!
//! 1. `recover` answers Ok (or, with `pick_only`, `pick` does -- the Lean model's recovery);
//! 2. the generation it opens is one the caller expects, and the image REPLAYS to exactly what
//!    the caller says that generation holds (AC-ES-13: replay(k) or replay(k) + ops);
//! 3. the opened generation's whole arena walks: every object fits and passes its crc;
//! 4. with `old_intact`, no cell the durable generation names has changed (AC-ES-12);
//! 5. a further commit from that state, through the real `Writer`, succeeds and the device
//!    then recovers to the generation after the one opened.
//!
//! A failure names its crash point (`Point`'s ordinal and shape); `Report::first` is the
//! RED proof's line.

use std::fmt::Debug;

use super::crash::{each_state, Model, Point};
use super::{recover, walk, SimDisk};
use crate::format::{pick_checked_in, PT_PAGE_OFFSET, SB_CELLS};
use crate::iodev::{Durability, Durable, Writer};
use crate::{Cells, Store, ARENA};

/// What to check.
#[derive(Debug, Clone, Copy)]
pub struct Opts {
    pub model: Model,
    /// Recover with `pick` alone (Lean's `recover`), not anchor + fallback.
    pub pick_only: bool,
    /// Check that the durable generation's cells are untouched (single-commit flushes).
    pub old_intact: bool,
}

/// A crash point and what went wrong there.
#[derive(Debug, Clone)]
pub struct Failure {
    pub point: Point,
    pub why: String,
}

/// Every state, and every failure.
#[derive(Debug, Clone)]
pub struct Report {
    pub states: usize,
    pub failures: Vec<Failure>,
}

impl Report {
    pub fn green(&self) -> bool {
        self.failures.is_empty()
    }
    /// "<point>: <why>" of the first failure.
    pub fn first(&self) -> Option<String> {
        self.failures.first().map(|f| format!("{}: {}", f.point, f.why))
    }
}

/// What one generation must replay to.
pub type Expect<R> = (i64, R);

/// Run every crash state of `ops` over the durable `base`.
pub fn run<R: PartialEq + Debug>(
    base: &[i64],
    ops: &[super::SimOp],
    opts: Opts,
    expect: &[Expect<R>],
    replay: &dyn Fn(&Store) -> R,
    next: &dyn Fn(&mut Store) -> Result<(), String>,
) -> Report {
    let mut failures = Vec::new();
    let states = each_state(base, ops, opts.model, |p, img| {
        if let Err(why) = check(base, img, opts, expect, replay, next) {
            failures.push(Failure { point: p.clone(), why });
        }
    });
    Report { states, failures }
}

/// The five checks on one state.
fn check<R: PartialEq + Debug>(
    base: &[i64],
    img: &[i64],
    opts: Opts,
    expect: &[Expect<R>],
    replay: &dyn Fn(&Store) -> R,
    next: &dyn Fn(&mut Store) -> Result<(), String>,
) -> Result<(), String> {
    let mut st = Store { cells: img.to_vec() };
    let (sb, hidden) = if opts.pick_only {
        (pick_checked_in(&st).map_err(|e| format!("pick refused: {e:?}"))?, None)
    } else {
        let r = recover(&st).map_err(|e| format!("recover refused: {e:?}"))?;
        (r.sb, r.hidden)
    };
    if let Some(h) = hidden {
        st.cells[h] = 0;
    }
    let gens: Vec<i64> = expect.iter().map(|e| e.0).collect();
    let Some((_, want)) = expect.iter().find(|e| e.0 == sb.generation) else {
        return Err(format!("recovered generation {} is not one of {gens:?}", sb.generation));
    };
    let got = replay(&st);
    if got != *want {
        return Err(format!("generation {} replays to {got:?}, not {want:?}", sb.generation));
    }
    walk(&st, ARENA, sb.arena_used.max(0) as usize)
        .map_err(|at| format!("generation {}: the arena walk fails at cell {at}", sb.generation))?;
    if opts.old_intact {
        old_intact(base, img)?;
    }
    if opts.pick_only {
        return Ok(());
    }
    let mut w = Writer::open(SimDisk::new(img.to_vec())).map_err(|e| format!("reopen for the next commit: {e:?}"))?;
    let g = w.durable().gen;
    next(w.stage().map_err(|e| format!("{e:?}"))?)?;
    w.flush(Durability::Data).map_err(|e| format!("the next commit's flush: {e:?}"))?;
    let after = Store { cells: w.into_device().cache() };
    let again = recover(&after).map_err(|e| format!("after the next commit, recover refused: {e:?}"))?;
    if again.sb.generation != g + 1 {
        return Err(format!("the next commit from generation {g} recovers as {}", again.sb.generation));
    }
    Ok(())
}

/// Every cell the durable generation of `base` names -- its superblock, its PartTab, its
/// arena -- is the same in `img`.
fn old_intact(base: &[i64], img: &[i64]) -> Result<(), String> {
    let b = Store { cells: base.to_vec() };
    let d = Durable::of(&b).map_err(|e| format!("the base has no generation: {e:?}"))?;
    let used = b.cell_at(d.at + crate::format::SB_USED).max(0) as usize;
    let pt = d.at + PT_PAGE_OFFSET;
    let ranges = [(d.at, d.at + SB_CELLS), (pt, pt + crate::format::PARTTAB_OBJ_CELLS), (ARENA, used)];
    for (lo, hi) in ranges {
        for i in lo..hi {
            if base.get(i) != img.get(i) {
                return Err(format!("cell {i} of the durable generation {} changed", d.gen));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
