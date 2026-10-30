//! W-ES-HARD (es-p19.design §3, R-EMBED-STORE §4.3): THE ENUMERATED CRASH HARNESS FOR OPTIMISTIC
//! COMMITS. N write transactions open together on one generation (P19b: any number may be). The
//! harness enumerates every COMMIT ORDER (N!) x every CONFLICT PATTERN of a fixed table (`patterns`)
//! x every way to CUT the commit sequence into flush groups (2^(N-1) compositions: the commits are
//! `Durability::Normal`, a `Db::flush(Data)` closes each group) and runs every group's record
//! through `harness::run`: every power-loss state recovers the generation before the group or the
//! one after it -- never one between (the `expect` list holds only those two) -- with the exact
//! map. In memory it checks that the section accepts and refuses exactly what the model says and
//! that the map readers see after every commit is the model's (a replay onto a transaction's base
//! instead of the current root would lose the earlier commit's key here).
//!
//! THE MODEL is `model::serial`: a `BTreeMap` and the validation rule written by hand -- a
//! transaction is refused iff an earlier ACCEPTED one in the order wrote a key it read
//! (`WriteTxn::remove` reads the key it removes; a blind `put` reads nothing) -- never the code
//! under test. Nothing is drawn at random: `permutations`, `cuts` and `patterns` are the whole
//! space for N <= 4.

use std::fmt;
use std::io;
use std::sync::{Arc, Mutex};

use super::crash::{Model, Tear};
use super::harness::{run, Opts};
use super::testkit::{next, replay, Map, CAP};
use super::{SimDisk, SimOp};
use crate::{Db, Device, Durability, Options, Refusal, Store, Verify, WriteTxn};

pub mod model;
pub use model::*;

pub const POWER: Opts = Opts { model: Model::Power(Tear::Half), pick_only: false, old_intact: true };

/// A `SimDisk` a `Db` writes while the harness reads its record (the lock is never held across
/// a device call, so there is nothing to deadlock).
#[derive(Clone)]
pub struct Shared(pub Arc<Mutex<SimDisk>>);

impl Shared {
    pub fn new(base: Vec<i64>) -> Shared {
        Shared(Arc::new(Mutex::new(SimDisk::new(base))))
    }
    fn disk(&self) -> std::sync::MutexGuard<'_, SimDisk> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    /// The durable base and the record since it; then the record is settled.
    pub fn take(&self) -> (Vec<i64>, Vec<SimOp>) {
        let mut d = self.disk();
        let r = (d.base().to_vec(), d.ops().to_vec());
        d.settle();
        r
    }
}

impl Device for Shared {
    fn len_cells(&self) -> io::Result<usize> {
        self.disk().len_cells()
    }
    fn read_cells(&self, off: usize, n: usize) -> io::Result<Vec<i64>> {
        self.disk().read_cells(off, n)
    }
    fn write_cells(&mut self, off: usize, cells: &[i64]) -> io::Result<()> {
        self.disk().write_cells(off, cells)
    }
    fn barrier(&mut self) -> io::Result<()> {
        self.disk().barrier()
    }
    fn fresh(&self) -> io::Result<Shared> {
        Ok(Shared::new(Vec::new()))
    }
    fn publish(&mut self, staged: Shared) -> io::Result<()> {
        let s = staged.disk().clone();
        self.disk().publish(s)
    }
}

/// Which image the schedule runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Image {
    V3,
    #[cfg(feature = "tree")]
    V4,
}

/// The six keys, each "0", as a v3 image (16 KiB) or a v4 tree at `MIN_FANOUT` (64 KiB: no
/// compaction can be asked for inside a schedule, so every group's record is its flush alone).
pub fn base_image(img: Image) -> Store {
    match img {
        Image::V3 => super::testkit::kv_image(&KEYS.map(|k| (k, "0")), CAP),
        #[cfg(feature = "tree")]
        Image::V4 => {
            let db = Db::create(SimDisk::new(Vec::new()), options(img).initial_bytes(64 * 1024).create_tree(true)).expect("creates");
            let mut w = db.begin_write().expect("opens");
            for k in KEYS {
                w.put(k.as_bytes(), b"0").expect("puts");
            }
            w.commit(Durability::Data).expect("commits");
            let mut d = db.into_device().expect("the device");
            d.settle();
            Store { cells: d.base().to_vec() }
        }
    }
}

/// No automatic compaction, no automatic checkpoint: the cuts decide the groups.
pub fn options(img: Image) -> Options {
    let o = Options::default().compaction(bebop_kv::delta::MAX_DELTAS, 0).checkpoint_every(0);
    match img {
        Image::V3 => o,
        #[cfg(feature = "tree")]
        Image::V4 => o.tree(bebop_kv::tree::MIN_FANOUT, 0),
    }
}

/// One schedule: a pattern's transactions, the order they commit in, the group sizes.
pub struct Schedule<'a> {
    pub pattern: &'static str,
    pub txns: &'a [Txn],
    pub order: &'a [usize],
    pub cut: &'a [usize],
}

impl fmt::Display for Schedule<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} N={} order {:?} groups {:?}", self.pattern, self.txns.len(), self.order, self.cut)
    }
}

/// What one schedule ran: the groups that reached the device and the crash states checked.
#[derive(Debug, Default)]
pub struct Outcome {
    pub groups: usize,
    pub states: usize,
}

fn open_all<'db>(db: &'db Db<Shared>, txns: &[Txn]) -> Result<Vec<Option<WriteTxn<'db, Shared>>>, Refusal> {
    txns.iter()
        .map(|t| {
            let mut w = db.begin_write()?;
            for r in t.reads {
                w.get(r.as_bytes())?;
            }
            for (k, v) in t.writes {
                match v {
                    Some(v) => w.put(k.as_bytes(), v.as_bytes())?,
                    None => {
                        w.remove(k.as_bytes())?;
                    }
                }
            }
            Ok(Some(w))
        })
        .collect()
}

fn visible(db: &Db<Shared>) -> Map {
    replay(&Store::from_bytes(&db.snapshot_bytes()))
}

/// RUN ONE SCHEDULE through the real section, the model being `serial`: `run_schedule_with`'s
/// identity record.
pub fn run_schedule(img: Image, s: &Schedule, opts: Opts) -> Result<Outcome, String> {
    run_schedule_with(img, s, opts, &serial, &|ops| ops.to_vec())
}

/// `run_schedule` against any model and with every group's record passed through `mutate`
/// before the crash harness sees it (the RED proofs).
pub fn run_schedule_with(img: Image, s: &Schedule, opts: Opts, model: &ModelFn, mutate: &dyn Fn(&[SimOp]) -> Vec<SimOp>) -> Result<Outcome, String> {
    let e = |what: String| format!("{s}: {what}");
    let base = base_image(img);
    let m0 = replay(&base);
    let order: Vec<Txn> = s.order.iter().map(|&i| s.txns[i]).collect();
    let want = model(&m0, &order);
    let disk = Shared::new(base.cells.clone());
    let db = Db::open(disk.clone(), options(img)).map_err(|x| e(format!("open: {x:?}")))?;
    let g0 = db.durable().get();
    let mut open = open_all(&db, s.txns).map_err(|x| e(format!("a transaction refused before commit: {x:?}")))?;
    let (mut p, mut acc, mut out) = (0usize, 0usize, Outcome::default());
    let (mut g_before, mut m_before) = (g0, m0);
    for (gi, &size) in s.cut.iter().enumerate() {
        for _ in 0..size {
            let w = open[s.order[p]].take().expect("each transaction commits once");
            match (want.accepted[p], w.commit(Durability::Normal)) {
                (true, Ok(g)) => {
                    acc += 1;
                    if g.get() != g0 + acc as i64 {
                        return Err(e(format!("commit #{p} is generation {}, not {}", g.get(), g0 + acc as i64)));
                    }
                }
                (false, Err(Refusal::Conflict { .. })) => {}
                (m, got) => return Err(e(format!("commit #{p}: the model says accepted={m}, the section {got:?}"))),
            }
            let vis = visible(&db);
            if vis != want.maps[acc] {
                return Err(e(format!("after commit #{p} readers see {vis:?}, the model says {:?}", want.maps[acc])));
            }
            p += 1;
        }
        db.flush(Durability::Data).map_err(|x| e(format!("flush of group {gi}: {x:?}")))?;
        let g_after = g0 + acc as i64;
        if db.durable().get() != g_after {
            return Err(e(format!("after group {gi} the durable generation is {}, not {g_after}", db.durable().get())));
        }
        let (durable, ops) = disk.take();
        if ops.is_empty() {
            if g_after != g_before {
                return Err(e(format!("group {gi} moved the generation to {g_after} and wrote nothing")));
            }
            continue;
        }
        let expect = [(g_before, m_before.clone()), (g_after, want.maps[acc].clone())];
        let r = run(&durable, &mutate(&ops), opts, &expect, &replay, &next);
        if !r.green() {
            return Err(e(format!("group {gi}: {} of {} states fail; first {:?}", r.failures.len(), r.states, r.first())));
        }
        out.groups += 1;
        out.states += r.states;
        (g_before, m_before) = (g_after, want.maps[acc].clone());
    }
    if db.snapshot_bytes() != db.writer_image() {
        return Err(e("the published snapshot is not the writer's image".into()));
    }
    db.verify(Verify::WholeArena).map_err(|x| e(format!("verify: {x:?}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests;
