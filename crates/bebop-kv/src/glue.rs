//! GLUING: the single contract of the write path (operator decision 2026-10-30, W-ES-P19).
//!
//! Modelled on dowiz-hub/src/glue.rs (sheaf S0, the stock log): two SECTIONS of one object over a
//! common BASE; a section's RESTRICTION to the overlap is what it changed since the base (its
//! keys; for a tree node, the slot spans it rewrote); the sections GLUE when their restrictions do
//! not DISAGREE -- then their deltas commute and the glued section is the base plus both, in
//! either order (bebop-lang/formal/Bebop/Glue.lean `glue_commutes`, instantiated by `occ_glues`,
//! `group_glues`, `merge_glues`). A disagreement is an OBSTRUCTION, and a NAMED resolver answers
//! it: `Refuse` (the default; bebop-io's commit section turns it into `Refusal::Conflict`), keep
//! one side, or build the glued piece itself (the tree 3-way merge's `Descend`: into the one child
//! both sides rewrote, the same operation one level down). GLUING DETECTS; RESOLVING IS NAMED.
//!
//! INSTANCES. (1) OCC, bebop-io's commit section: base = the generation the transaction read;
//! left = the commits since it (each one's WriteSet); right = the transaction's ReadSet (which
//! holds its own keys); a shared key = `Refuse` = `Conflict`. (2) The tree 3-way merge (the path
//! copy pre-built against an older root, rebased onto the current one): base = a node's old
//! slots; restrictions = each side's replacement spans; an intersecting span = `Descend`. (3)
//! P18 sync and the offline kitchen: hub and tablet tails over their common ancestor, the
//! obstruction a `delta::Conflict` for a person (dowiz-hub glue.rs keeps its own walk; it can
//! call this operation on its restrictions without depending on anything above bebop-kv).
//!
//! NOT ONE LINEAR HISTORY (operator 2026-10-30: one file that merges without a server, proves its
//! history, and reads without the library). A gluing is PAIRWISE OVER A COMMON BASE, and bases
//! compose: two devices' sections glue over their common ancestor, the result glues with a third
//! device's over ITS common ancestor with that result -- a DAG of glues, as the hub's ancestor
//! walk already is. Nothing here names a device or a total order: `Side` is the instance's
//! label for the two sections of ONE gluing, the commit-log ring in bebop-io is one device's
//! own sequence, and a later SIGNED MERGE EVENT per device (who glued what over which base) is
//! a record the instance appends, not a change to this operation. That record is not built here.
//!
//! COST. One pass over the two restrictions (|left| x |right| `disagrees`, the same compares the
//! commit section made before this contract existed); no allocation unless a resolver glues.
//! Measured on the p4 grid before and after (es-p19b.design.md §2.3). Not on reads: a read is one
//! section, there is nothing to glue. The format's bytes are untouched.

use alloc::vec::Vec;

/// Which section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// One element of a restriction, and when two of them disagree: the same key; two slot spans
/// that intersect.
pub trait Touch {
    fn disagrees(&self, other: &Self) -> bool;
}

impl Touch for Vec<u8> {
    fn disagrees(&self, other: &Self) -> bool {
        self == other
    }
}
impl Touch for &[u8] {
    fn disagrees(&self, other: &Self) -> bool {
        self == other
    }
}
impl Touch for u64 {
    fn disagrees(&self, other: &Self) -> bool {
        self == other
    }
}

/// A half-open span of slot positions `[lo, hi)` of the base node one side replaced. An
/// insertion (`lo == hi`) touches both neighbours, so two spans disagree when they intersect
/// after that widening -- the rule the tree merge applied before it was named here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
}

impl Span {
    fn widened(&self) -> (usize, usize) {
        if self.hi > self.lo { (self.lo, self.hi) } else { (self.lo.saturating_sub(1), self.lo + 1) }
    }
    /// Exactly one old slot, replaced by exactly one new one on both sides: what `Descend` needs.
    pub fn single(&self) -> bool {
        self.hi == self.lo + 1
    }
}

impl Touch for Span {
    fn disagrees(&self, other: &Self) -> bool {
        let (a, b) = (self.widened(), other.widened());
        a.0 < b.1 && b.0 < a.1
    }
}

/// How the two sections relate once glued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Neither changed anything.
    Same,
    /// Only one side wrote: the glued section is that side's.
    FastForward(Side),
    /// Both wrote and nothing disagreed: the deltas commute.
    Commutes,
    /// Something disagreed and the resolver answered every obstruction.
    Resolved,
}

/// A resolver's answer for one disagreeing pair.
pub enum Resolve<T> {
    /// The obstruction stands: the gluing stops (`Obstruction`).
    Refuse,
    /// Keep this side's piece, drop the other's.
    Keep(Side),
    /// The resolver built the glued piece itself.
    Glued(T),
}

/// A NAMED resolver, called once per disagreeing pair. It may keep state (what it has already
/// answered) and refuse a second obstruction on the same piece.
pub trait Resolver<R: Touch> {
    type Out;
    fn resolve(&mut self, left: &R, right: &R) -> Resolve<Self::Out>;
}

/// THE DEFAULT RESOLVER: every obstruction is refused. bebop-io names it `Refusal::Conflict`.
pub struct Refuse;

impl<R: Touch> Resolver<R> for Refuse {
    type Out = ();
    fn resolve(&mut self, _l: &R, _r: &R) -> Resolve<()> {
        Resolve::Refuse
    }
}

/// The pair the resolver refused: indices into `left` and `right`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Obstruction {
    pub left: usize,
    pub right: usize,
}

/// The gluing's answer: the state, every piece the resolver built (`(left, right, piece)`) and
/// every side it kept.
#[derive(Debug)]
pub struct Glued<T> {
    pub state: State,
    pub glued: Vec<(usize, usize, T)>,
    pub kept: Vec<(usize, usize, Side)>,
}

/// THE OPERATION. `left` and `right` are the two restrictions; every pair that disagrees goes to
/// `resolver`; the first `Refuse` ends the gluing as `Err` naming the pair. The glued section's
/// ASSEMBLY (splicing the pieces into the base) is the instance's, as it is in dowiz-hub.
pub fn glue<R: Touch, V: Resolver<R>>(left: &[R], right: &[R], resolver: &mut V) -> Result<Glued<V::Out>, Obstruction> {
    let state = match (left.is_empty(), right.is_empty()) {
        (true, true) => State::Same,
        (false, true) => State::FastForward(Side::Left),
        (true, false) => State::FastForward(Side::Right),
        (false, false) => State::Commutes,
    };
    let mut g = Glued { state, glued: Vec::new(), kept: Vec::new() };
    for (i, l) in left.iter().enumerate() {
        for (j, r) in right.iter().enumerate() {
            if !l.disagrees(r) {
                continue;
            }
            match resolver.resolve(l, r) {
                Resolve::Refuse => return Err(Obstruction { left: i, right: j }),
                Resolve::Keep(s) => g.kept.push((i, j, s)),
                Resolve::Glued(t) => g.glued.push((i, j, t)),
            }
            g.state = State::Resolved;
        }
    }
    Ok(g)
}

/// Do the two restrictions agree, i.e. would `glue` with `Refuse` succeed? This is OCC's
/// validation: `agree(&commit.writes, &txn.reads)` for every commit since the base.
pub fn agree<R: Touch>(left: &[R], right: &[R]) -> bool {
    glue(left, right, &mut Refuse).is_ok()
}

#[cfg(test)]
mod tests;
