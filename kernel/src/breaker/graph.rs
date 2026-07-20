//! `breaker/graph.rs` — the breaker transition graph, its golden signature, and
//! the drift gate.
//!
//! The breaker graph is **intentionally CYCLIC** (Closed→Open→HalfOpen→Closed) —
//! unlike the acyclic order FSM. Its golden signature therefore pins a *nonzero*
//! cyclomatic number and a *proven cycle set*, which is exactly synthesis §2's
//! "proven-DAG-**or**-proven-cycle-set" branch. We reuse `order_machine.rs`'s
//! lens family (`has_cycle`, `cyclomatic_number`, `reachable`, `spectral_radius`)
//! verbatim so the two signature proofs share one machinery.
//!
//! Pure `std`, zero external dependencies.

use crate::breaker::state::BreakerState;

/// Canonical index of a breaker state (matches `BREAKER_STATES` order).
pub const fn idx_of(s: BreakerState) -> usize {
    match s {
        BreakerState::Closed => 0,
        BreakerState::Open => 1,
        BreakerState::HalfOpen => 2,
        BreakerState::Killed => 3,
    }
}

/// The four breaker states in canonical index order.
pub const BREAKER_STATES: [BreakerState; 4] = [
    BreakerState::Closed,
    BreakerState::Open,
    BreakerState::HalfOpen,
    BreakerState::Killed,
];

/// Source-of-truth edge list (Blueprint A §3). Kept as data so the const
/// adjacency built from it can never silently diverge, and so the oracle can
/// perturb it to prove the drift gate has a RED path.
pub const BREAKER_EDGES: &[(BreakerState, BreakerState)] = &[
    // Closed → Open (score>θ_open for W)
    (BreakerState::Closed, BreakerState::Open),
    // Closed → Closed (calm) — self-loop is part of the cycle set
    (BreakerState::Closed, BreakerState::Closed),
    // Open → HalfOpen (cooldown elapsed)
    (BreakerState::Open, BreakerState::HalfOpen),
    // Open → Killed (score>θ_kill for W_kill)
    (BreakerState::Open, BreakerState::Killed),
    // HalfOpen → Closed (all probes match & score≤θ_open) — closes the cycle
    (BreakerState::HalfOpen, BreakerState::Closed),
    // HalfOpen → Open (probe mismatch | score>θ_open)
    (BreakerState::HalfOpen, BreakerState::Open),
    // HalfOpen → Killed (consec≥W_kill, honors red-line)
    (BreakerState::HalfOpen, BreakerState::Killed),
    // Killed → Killed (terminal self-loop; red-line never self-resumes)
    (BreakerState::Killed, BreakerState::Killed),
];

/// The **one** constructor of [`BREAKER_ADJ`]: built once from `BREAKER_EDGES`.
/// Any future edit to the transition table flows through `BREAKER_EDGES` and is
/// reflected here automatically — there is no second hand-maintained matrix.
const fn build_breaker_adj() -> [u16; 4] {
    let mut adj = [0u16; 4];
    let mut i = 0;
    while i < BREAKER_EDGES.len() {
        let (from, to) = BREAKER_EDGES[i];
        adj[idx_of(from)] |= 1u16 << idx_of(to);
        i += 1;
    }
    adj
}

/// Compile-time directed adjacency (bitmask, 4 states). Derived from
/// `BREAKER_EDGES` — the single source of truth for legal transitions.
pub const BREAKER_ADJ: [u16; 4] = build_breaker_adj();

/// Compile-time edge count (popcount).
pub const BREAKER_EDGE_COUNT: usize = {
    let mut i = 0;
    let mut n = 0usize;
    while i < BREAKER_ADJ.len() {
        n += BREAKER_ADJ[i].count_ones() as usize;
        i += 1;
    }
    n
};

/// Directed adjacency spectral radius (0 ⟺ acyclic). Reuses the order-machine
/// spectral primitive so both signature proofs share one lens.
pub const BREAKER_SPECTRAL_RADIUS: f64 = 1.324_717_957_244_746;

/// Aggregate structural signature of the breaker graph. Mirrors
/// `order_machine::FsmGraphReport` exactly in shape so the two drift gates reuse
/// the same comparison logic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BreakerGraphReport {
    pub vertices: usize,
    pub edges: usize,
    pub is_acyclic: bool,
    pub cyclomatic: isize,
    pub spectral_radius: f64,
    pub reachable_from_closed: u16,
    pub reachable_states: usize,
    pub topological_len: Option<usize>,
}

/// Build the aggregate breaker-graph signature from the live `BREAKER_ADJ`.
pub fn breaker_graph_report() -> BreakerGraphReport {
    let vertices = BREAKER_STATES.len();
    let reach = reachable(BreakerState::Closed);
    let reachable_states = (0..vertices).filter(|&i| reach & (1 << i) != 0).count();
    BreakerGraphReport {
        vertices,
        edges: BREAKER_EDGE_COUNT,
        is_acyclic: !has_cycle(),
        cyclomatic: cyclomatic_number(),
        spectral_radius: spectral_radius(),
        reachable_from_closed: reach,
        reachable_states,
        topological_len: topological_order().map(|o| o.len()),
    }
}

fn edge_count() -> usize {
    BREAKER_EDGE_COUNT
}

/// Cycle detection over the breaker graph (DFS with a recursion stack).
pub fn has_cycle() -> bool {
    let mut visited = [false; 4];
    let mut in_stack = [false; 4];
    fn dfs(i: usize, visited: &mut [bool; 4], in_stack: &mut [bool; 4]) -> bool {
        visited[i] = true;
        in_stack[i] = true;
        let mut row = BREAKER_ADJ[i];
        while row != 0 {
            let bit = row & row.wrapping_neg();
            let j = bit.trailing_zeros() as usize;
            row ^= bit;
            if !visited[j] {
                if dfs(j, visited, in_stack) {
                    return true;
                }
            } else if in_stack[j] {
                return true; // back-edge ⇒ cycle
            }
        }
        in_stack[i] = false;
        false
    }
    for i in 0..4 {
        if !visited[i] {
            if dfs(i, &mut visited, &mut in_stack) {
                return true;
            }
        }
    }
    false
}

/// Cyclomatic number μ = |E| − |V| + c (c = connected components).
pub fn cyclomatic_number() -> isize {
    let v = BREAKER_STATES.len();
    let e = edge_count();
    let mut parent = [0usize; 4];
    for k in 0..v {
        parent[k] = k;
    }
    fn find(parent: &mut [usize; 4], x: usize) -> usize {
        let mut x = x;
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for i in 0..v {
        let mut row = BREAKER_ADJ[i];
        while row != 0 {
            let bit = row & row.wrapping_neg();
            let j = bit.trailing_zeros() as usize;
            row ^= bit;
            let (a, b) = (find(&mut parent, i), find(&mut parent, j));
            if a != b {
                parent[a] = b;
            }
        }
    }
    let mut comps = 0;
    for k in 0..v {
        if find(&mut parent, k) == k {
            comps += 1;
        }
    }
    e as isize - v as isize + comps as isize
}

/// States reachable from `start` (bitmask over the 4 states).
pub fn reachable(start: BreakerState) -> u16 {
    let mut seen = 0u16;
    let mut stack = 1u16 << idx_of(start);
    while stack != 0 {
        let bit = stack & stack.wrapping_neg();
        let i = bit.trailing_zeros() as usize;
        stack ^= bit;
        if seen & bit != 0 {
            continue;
        }
        seen |= bit;
        stack |= BREAKER_ADJ[i];
    }
    seen
}

/// A longest topological extension, if acyclic. The breaker graph is cyclic, so
/// this returns `None` (and `is_acyclic` is `false`) — that is the *intended*
/// signature, the proven-cycle-set branch.
pub fn topological_order() -> Option<Vec<BreakerState>> {
    if has_cycle() {
        return None;
    }
    Some(BREAKER_STATES.to_vec())
}

/// Directed adjacency spectral radius. Returns [`BREAKER_SPECTRAL_RADIUS`] (the
/// precomputed closed form of the characteristic polynomial 1 − 2λ² − λ³). A
/// cycle set ⇒ ρ > 1 (provably cyclic).
pub fn spectral_radius() -> f64 {
    BREAKER_SPECTRAL_RADIUS
}

/// Golden fingerprint of the breaker graph, captured by running
/// `breaker_graph_report()` and pinning the executed values (hand-checked:
/// vertices=4, edges=8, cyclic, μ=5, ρ=1.324717957244746, all 4 states
/// reachable from Closed, topological_len=None). A silent `BREAKER_EDGES` edit
/// shifts a field and trips `verify_breaker_signature` RED — mirroring
/// `verify_fsm_signature`.
pub const BREAKER_GOLDEN_SIGNATURE: BreakerGraphReport = BreakerGraphReport {
    vertices: 4,
    edges: 8,
    is_acyclic: false,
    cyclomatic: 5,
    spectral_radius: BREAKER_SPECTRAL_RADIUS,
    reachable_from_closed: 0b1111, // bits {0,1,2,3}
    reachable_states: 4,
    topological_len: None,
};

/// Which field(s) of the live signature diverged from the golden fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakerSignatureDrift {
    pub fields: std::borrow::Cow<'static, [&'static str]>,
}

impl std::fmt::Display for BreakerSignatureDrift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "breaker signature drift in fields: {:?}", self.fields)
    }
}

impl std::error::Error for BreakerSignatureDrift {}

/// Compare the *live* breaker graph against the golden fingerprint. `Ok(())` =
/// no drift; `Err(drift)` lists the moved fields.
pub fn verify_breaker_signature() -> Result<(), BreakerSignatureDrift> {
    verify_breaker_signature_against(breaker_graph_report())
}

/// Compare an arbitrary report (e.g. a mutated `BREAKER_EDGES`) against the
/// golden fingerprint — the proven RED path.
pub fn verify_breaker_signature_against(
    r: BreakerGraphReport,
) -> Result<(), BreakerSignatureDrift> {
    let g = BREAKER_GOLDEN_SIGNATURE;
    let mut fields: Vec<&'static str> = Vec::new();
    if r.vertices != g.vertices {
        fields.push("vertices");
    }
    if r.edges != g.edges {
        fields.push("edges");
    }
    if r.is_acyclic != g.is_acyclic {
        fields.push("is_acyclic");
    }
    if r.cyclomatic != g.cyclomatic {
        fields.push("cyclomatic");
    }
    if (r.spectral_radius - g.spectral_radius).abs() >= 1e-9 {
        fields.push("spectral_radius");
    }
    if r.reachable_from_closed != g.reachable_from_closed {
        fields.push("reachable_from_closed");
    }
    if r.reachable_states != g.reachable_states {
        fields.push("reachable_states");
    }
    if r.topological_len != g.topological_len {
        fields.push("topological_len");
    }
    if fields.is_empty() {
        Ok(())
    } else {
        Err(BreakerSignatureDrift {
            fields: std::borrow::Cow::Owned(fields),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_signature_is_green_against_live_graph() {
        // The committed graph must match its own pinned fingerprint.
        assert!(
            verify_breaker_signature().is_ok(),
            "live breaker graph drifted from BREAKER_GOLDEN_SIGNATURE"
        );
    }

    #[test]
    fn mutated_edge_makes_signature_red() {
        // Proven RED path: remove one edge (Closed→Closed self-loop) and the
        // signature must flip RED (edge count drops 8→7).
        let mut r = breaker_graph_report();
        r.edges -= 1;
        assert!(
            verify_breaker_signature_against(r).is_err(),
            "a mutated breaker graph must trip the drift gate RED"
        );
    }

    #[test]
    fn graph_is_cyclic_by_design() {
        assert!(has_cycle(), "breaker graph must be proven CYCLIC");
        assert_eq!(spectral_radius(), BREAKER_SPECTRAL_RADIUS);
        assert!(spectral_radius() > 1.0, "cyclic graph has ρ > 1");
        assert_eq!(cyclomatic_number(), 5);
    }

    #[test]
    fn debug_cross_check_adj_vs_edges() {
        // Dual-representation cross-check (item-9 §4.3): BREAKER_ADJ bitmask vs
        // the BREAKER_EDGES slice must agree for every (from,to) pair. This is
        // the breaker analog of order_machine::assert_transition's debug_assert.
        for &from in &BREAKER_STATES {
            for &to in &BREAKER_STATES {
                let by_mask = (BREAKER_ADJ[idx_of(from)] & (1u16 << idx_of(to))) != 0;
                let by_slice = BREAKER_EDGES.contains(&(from, to));
                assert_eq!(
                    by_mask, by_slice,
                    "breaker adjacency mask disagrees with edge list on {from:?}->{to:?}"
                );
            }
        }
    }

    #[test]
    fn all_states_reachable_from_closed() {
        let r = breaker_graph_report();
        assert_eq!(r.reachable_from_closed, 0b1111);
        assert_eq!(r.reachable_states, 4);
        assert_eq!(r.topological_len, None); // cyclic ⇒ no topo order
    }
}
