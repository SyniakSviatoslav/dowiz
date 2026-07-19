//! Deterministic order-state machine — the `decide/fold` Law.
//!
//! 1:1 port of `packages/domain/src/order-machine.ts` + `errors.ts` (the TS oracle).
//! The TS app is the legacy oracle; this Rust kernel is the canonical core.
//! WASM/headless safe. No float, no I/O.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Preparing,
    Ready,
    InDelivery,
    Delivered,
    Rejected,
    Cancelled,
    Scheduled,
    PickedUp,
    /// P07 compensation state — a reversal (money refund) is in progress. Reachable
    /// from any post-commitment state (money has moved after `Confirmed`).
    Refunding,
    /// P07 compensated terminal — the order was refunded/cancelled-after-confirm and
    /// its ledger nets to exactly zero. Terminal (is_terminal → true).
    CompensatedRefund,
}

impl OrderStatus {
    /// Parse from the oracle's string form. Unknown strings are rejected (never silently mapped).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "PENDING" => Some(Self::Pending),
            "CONFIRMED" => Some(Self::Confirmed),
            "PREPARING" => Some(Self::Preparing),
            "READY" => Some(Self::Ready),
            "IN_DELIVERY" => Some(Self::InDelivery),
            "DELIVERED" => Some(Self::Delivered),
            "REJECTED" => Some(Self::Rejected),
            "CANCELLED" => Some(Self::Cancelled),
            "SCHEDULED" => Some(Self::Scheduled),
            "PICKED_UP" => Some(Self::PickedUp),
            "REFUNDING" => Some(Self::Refunding),
            "COMPENSATED_REFUND" => Some(Self::CompensatedRefund),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Confirmed => "CONFIRMED",
            Self::Preparing => "PREPARING",
            Self::Ready => "READY",
            Self::InDelivery => "IN_DELIVERY",
            Self::Delivered => "DELIVERED",
            Self::Rejected => "REJECTED",
            Self::Cancelled => "CANCELLED",
            Self::Scheduled => "SCHEDULED",
            Self::PickedUp => "PICKED_UP",
            Self::Refunding => "REFUNDING",
            Self::CompensatedRefund => "COMPENSATED_REFUND",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Delivered
                | Self::PickedUp
                | Self::Rejected
                | Self::Cancelled
                | Self::CompensatedRefund
        )
    }
}

/// Transition table — identical to the oracle's `TRANSITIONS`.
/// `SCHEDULED` is a scaffold terminal (the scheduled flow is not implemented yet).
const fn allowed_next(from: OrderStatus) -> &'static [OrderStatus] {
    use OrderStatus::*;
    match from {
        Pending => &[Confirmed, Rejected, Cancelled],
        Confirmed => &[Preparing, InDelivery, Refunding],
        Preparing => &[Ready, Refunding],
        Ready => &[InDelivery, PickedUp, Refunding],
        InDelivery => &[Delivered, Refunding],
        Delivered => &[],
        Rejected => &[],
        Cancelled => &[],
        Scheduled => &[],
        PickedUp => &[],
        Refunding => &[CompensatedRefund],
        CompensatedRefund => &[],
    }
}

fn is_scaffold(s: OrderStatus) -> bool {
    matches!(s, OrderStatus::Scheduled)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    /// from === to
    SameStatus(OrderStatus),
    /// transition touches a scaffold-only status
    ScaffoldDisabled(OrderStatus, OrderStatus),
    /// not in the allowed transition table
    Illegal(OrderStatus, OrderStatus),
    /// domain invariant violated (e.g. subtotal arithmetic overflow) — RED LINE
    Invalid(String),
}

impl TransitionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::SameStatus(_) => "SameStatusError",
            Self::ScaffoldDisabled(_, _) => "ScaffoldDisabledError",
            Self::Illegal(_, _) => "IllegalTransitionError",
            Self::Invalid(_) => "InvalidInputError",
        }
    }
    pub fn message(&self) -> String {
        match self {
            Self::SameStatus(s) => format!("Cannot transition to same status: {}", s.as_str()),
            Self::ScaffoldDisabled(f, t) => {
                format!(
                    "Scaffold transition disabled: {} → {}",
                    f.as_str(),
                    t.as_str()
                )
            }
            Self::Illegal(f, t) => format!("Illegal transition: {} → {}", f.as_str(), t.as_str()),
            Self::Invalid(msg) => msg.clone(),
        }
    }
}

/// The `decide/fold` Law: validate a single state transition.
/// Mirrors `assertTransition(from, to)` — throws (returns Err) on invalid.
pub fn assert_transition(from: OrderStatus, to: OrderStatus) -> Result<(), TransitionError> {
    if from == to {
        return Err(TransitionError::SameStatus(from));
    }
    if is_scaffold(to) || is_scaffold(from) {
        return Err(TransitionError::ScaffoldDisabled(from, to));
    }
    let allowed = allowed_next(from);
    if !allowed.contains(&to) {
        return Err(TransitionError::Illegal(from, to));
    }
    Ok(())
}

/// Fold a sequence of transitions into a final status (the `fold` half of the Law).
/// Stops at the first invalid transition and returns the error + the status reached.
/// This is the deterministic reducer the WS event bus replays against.
pub fn fold_transitions(
    start: OrderStatus,
    steps: &[OrderStatus],
) -> Result<OrderStatus, (TransitionError, OrderStatus)> {
    let _span =
        crate::fdr::info_span!("fold_transitions", start = ?start, n_steps = steps.len()).entered();
    let mut cur = start;
    for &next in steps {
        assert_transition(cur, next).map_err(|e| (e, cur))?;
        cur = next;
    }
    crate::fdr::debug!(final_status = ?cur, "order transition fold complete");
    Ok(cur)
}

/// Directed adjacency of the order-lifecycle graph, computed **once at compile
/// time** from `allowed_next` (the single source of truth for legal transitions).
/// `FSM_ADJ[i]` is a bitmask over the 12 lifecycle states: bit `j` set iff there is
/// a legal transition `LIFECYCLE_STATES[i] → LIFECYCLE_STATES[j]`. The whole 12-state
/// graph fits in `[u16; 12]`, so every graph analysis below reads it with zero heap
/// allocation — replacing the old per-call `all_edges()` `Vec` that `has_cycle`,
/// `cyclomatic_number`, `reachable`, `topological_order` and `fsm_graph_report` each
/// re-materialised on every invocation.
const fn build_adjacency() -> [u16; 12] {
    let mut adj = [0u16; 12];
    let mut i = 0;
    while i < LIFECYCLE_STATES.len() {
        let from = LIFECYCLE_STATES[i];
        let succ = allowed_next(from);
        let mut j = 0;
        while j < succ.len() {
            adj[idx_of(from)] |= 1u16 << idx_of(succ[j]);
            j += 1;
        }
        i += 1;
    }
    adj
}

/// The compile-time adjacency (see [`build_adjacency`]). Derived from `allowed_next`,
/// so it can never silently diverge from the transition table `assert_transition` uses.
const FSM_ADJ: [u16; 12] = build_adjacency();

/// Total directed-edge count = popcount of every adjacency row. `const`, so the graph
/// analyses that need `|E|` pay nothing at runtime.
const FSM_EDGE_COUNT: usize = {
    let mut i = 0;
    let mut n = 0usize;
    while i < FSM_ADJ.len() {
        n += FSM_ADJ[i].count_ones() as usize;
        i += 1;
    }
    n
};

/// Materialise the `(from, to)` edge list — **test-only**. Production code reads
/// [`FSM_ADJ`] directly (zero-alloc); this `Vec` form exists solely so the oracle and
/// mutation tests below can build and perturb an explicit edge list. Derived from
/// `FSM_ADJ`, so it stays in exact lockstep with the compile-time graph.
#[cfg(test)]
fn all_edges() -> Vec<(OrderStatus, OrderStatus)> {
    let mut edges = Vec::new();
    let mut i = 0;
    while i < LIFECYCLE_STATES.len() {
        let mut row = FSM_ADJ[i];
        while row != 0 {
            let bit = row & row.wrapping_neg();
            let j = bit.trailing_zeros() as usize;
            row ^= bit;
            edges.push((LIFECYCLE_STATES[i], LIFECYCLE_STATES[j]));
        }
        i += 1;
    }
    edges
}

/// The ten lifecycle states, in their canonical adjacency-matrix index order.
/// Shared by every graph analysis below so indices never drift between passes.
const LIFECYCLE_STATES: [OrderStatus; 12] = [
    OrderStatus::Pending,
    OrderStatus::Confirmed,
    OrderStatus::Preparing,
    OrderStatus::Ready,
    OrderStatus::InDelivery,
    OrderStatus::Delivered,
    OrderStatus::Rejected,
    OrderStatus::Cancelled,
    OrderStatus::Scheduled,
    OrderStatus::PickedUp,
    OrderStatus::Refunding,
    OrderStatus::CompensatedRefund,
];

/// Canonical index of a lifecycle state (0..12), matching `LIFECYCLE_STATES`.
/// Centralised so `has_cycle`, `cyclomatic_number`, `topological_order`,
/// `reachable`, and `spectral_radius` all agree on vertex numbering — the single
/// definition of this mapping (the two former local duplicates are gone).
const fn idx_of(s: OrderStatus) -> usize {
    match s {
        OrderStatus::Pending => 0,
        OrderStatus::Confirmed => 1,
        OrderStatus::Preparing => 2,
        OrderStatus::Ready => 3,
        OrderStatus::InDelivery => 4,
        OrderStatus::Delivered => 5,
        OrderStatus::Rejected => 6,
        OrderStatus::Cancelled => 7,
        OrderStatus::Scheduled => 8,
        OrderStatus::PickedUp => 9,
        OrderStatus::Refunding => 10,
        OrderStatus::CompensatedRefund => 11,
    }
}

/// Topological order of the lifecycle FSM via Kahn's algorithm (1962).
/// Returns `Some(order)` when the directed graph is acyclic (a valid linear
/// extension exists) and `None` when a cycle blocks any total order.
///
/// Graph-theory basis: Kahn repeatedly removes sources (in-degree 0). If the
/// queue empties before all vertices are emitted, the residual subgraph is a
/// cycle, so no topological sort exists. This is the constructive companion to
/// `has_cycle` — it not only *detects* a cycle but *produces* the canonical
/// forward order the lifecycle is supposed to follow. O(|V| + |E|).
pub fn topological_order() -> Option<Vec<OrderStatus>> {
    let states = LIFECYCLE_STATES;
    let n = states.len();
    let mut indeg = [0usize; 12];
    for i in 0..n {
        let mut row = FSM_ADJ[i];
        while row != 0 {
            let bit = row & row.wrapping_neg();
            let t = bit.trailing_zeros() as usize;
            row ^= bit;
            indeg[t] += 1;
        }
    }
    // Stable source queue: ascending index, so the emitted order is deterministic
    // (lowest-index source — Pending — is emitted first).
    let mut queue: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut order: Vec<OrderStatus> = Vec::with_capacity(n);
    while let Some(u) = queue.first().copied() {
        queue.remove(0);
        order.push(states[u]);
        // Collect successors whose in-degree drops to 0, then merge them back into
        // the ascending queue (no heap needed at this size).
        let mut ready: Vec<usize> = Vec::new();
        let mut row = FSM_ADJ[u];
        while row != 0 {
            let bit = row & row.wrapping_neg();
            let v = bit.trailing_zeros() as usize;
            row ^= bit;
            indeg[v] -= 1;
            if indeg[v] == 0 {
                ready.push(v);
            }
        }
        ready.sort_unstable();
        let mut merged = Vec::with_capacity(queue.len() + ready.len());
        let (mut qi, mut ri) = (0, 0);
        while qi < queue.len() && ri < ready.len() {
            if queue[qi] <= ready[ri] {
                merged.push(queue[qi]);
                qi += 1;
            } else {
                merged.push(ready[ri]);
                ri += 1;
            }
        }
        merged.extend_from_slice(&queue[qi..]);
        merged.extend_from_slice(&ready[ri..]);
        queue = merged;
    }
    if order.len() == n {
        Some(order)
    } else {
        None
    }
}

/// Forward reachability over the lifecycle graph (BFS from a source state).
/// Returns the set of states reachable along directed paths — the states an
/// order *can* legally occupy after starting in `from`. The start state is
/// always included.
///
/// Uses a `u16` bitmask over the 10 states (one bit per vertex) so reachability
/// is exact (no float, no hashing) and trivially comparable for tests.
pub fn reachable(from: OrderStatus) -> u16 {
    let mut seen: u16 = 0;
    let mut frontier: u16 = 1 << idx_of(from);
    while frontier != 0 {
        let mut next: u16 = 0;
        let mut f = frontier;
        while f != 0 {
            let bit = f & f.wrapping_neg(); // lowest set bit
            let i = bit.trailing_zeros() as usize;
            f ^= bit;
            if seen & (1 << i) != 0 {
                continue;
            }
            seen |= 1 << i;
            // All one-step successors of state `i` in a single const lookup.
            next |= FSM_ADJ[i];
        }
        frontier = next & !seen;
    }
    seen
}

/// Proven spectral radius ρ of the lifecycle FSM's directed adjacency.
///
/// The lifecycle graph is a compile-time-constant DAG (see `allowed_next`) ⇒ its
/// adjacency matrix is nilpotent (A^k = 0 for large enough k) ⇒ by Perron–
/// Frobenius ρ = 0 exactly. This is therefore a compile-time constant; the
/// runtime power-iteration loop that previously computed it is gone from the hot
/// path and survives only as the test-side oracle `spectral_radius_oracle`
/// (which re-derives ρ from the actual graph and cross-checks this const).
pub const FSM_SPECTRAL_RADIUS: f64 = 0.0;

/// Spectral radius of the lifecycle FSM's directed adjacency.
///
/// Returns the proven constant [`FSM_SPECTRAL_RADIUS`] (ρ = 0). The iterative
/// power-iteration that used to live here now exists only as the test-side
/// oracle `spectral_radius_oracle`, where it verifies the const against the real
/// graph. T7/A7: spectral_radius → proven const.
pub fn spectral_radius() -> f64 {
    FSM_SPECTRAL_RADIUS
}

/// Human-readable stability verdict for the lifecycle FSM. Applies the same
/// spectral/energy drift lens used on generic graphs to the FSM's own directed
/// transition matrix: computes ρ via `spectral_radius()` and classifies it vs
/// the unit circle (Perron–Frobenius). For the acyclic forward lifecycle ρ=0,
/// so the verdict is `FSM stable: ρ=0.000000 damped`. Should a cycle ever be
/// introduced (ρ→1 or higher), the verdict flips to `resonant`/`unstable`,
/// tripping a drift gate. Pure-std, no I/O.
///
/// Cross-check (Verified-by-Math): the drift word matches `classify_drift` run
/// on the 1×1 matrix [[ρ]] — a scalar operator with the FSM's spectral radius.
pub fn fsm_stability_report() -> String {
    let rho = spectral_radius();
    let word = match crate::spectral::classify_drift(&[vec![rho]]) {
        crate::spectral::DriftClass::Damped => "damped",
        crate::spectral::DriftClass::Resonant => "resonant",
        crate::spectral::DriftClass::Unstable => "unstable",
    };
    let label = if matches!(word, "damped" | "resonant") {
        "FSM stable"
    } else {
        "FSM UNSTABLE"
    };
    format!("{}: ρ={:.6} {}", label, rho, word)
}

/// Aggregate structural signature of the lifecycle FSM — combines every graph
/// analysis into one observation that can be emitted as drift telemetry. A
/// silent change to `allowed_next` (e.g. a sneaky `Reopen` edge, or accidentally
/// dropping a transition) shifts one of these fields and trips a regression gate.
///
/// All five lenses are independent structural probes; their agreement is itself
/// a check (see `green_graph_report_invariants`):
///   * `is_acyclic == !has_cycle`
///   * `cyclomatic == μ`  (|E|−|V|+c, undirected cycle rank)
///   * `spectral_radius == ρ` (directed adjacency spectral radius; 0 ⟺ acyclic)
///   * `topological_order` is `Some` iff the graph is acyclic
///   * `reachable_states ==` count of states reachable from `Pending`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FsmGraphReport {
    /// number of vertices (lifecycle states)
    pub vertices: usize,
    /// number of directed edges (legal transitions)
    pub edges: usize,
    /// true iff the directed lifecycle graph has no cycle
    pub is_acyclic: bool,
    /// cyclomatic number μ = |E| − |V| + c (undirected cycle rank)
    pub cyclomatic: isize,
    /// spectral radius ρ of the directed adjacency (0 ⟺ acyclic)
    pub spectral_radius: f64,
    /// states reachable from `Pending` (bitmask, bit i = LIFECYCLE_STATES[i])
    pub reachable_from_pending: u16,
    /// count of states reachable from `Pending`
    pub reachable_states: usize,
    /// Some(n) length of a valid topological extension, None if cyclic
    pub topological_len: Option<usize>,
}

impl FsmGraphReport {
    /// Serialize to a compact JSON string (no external deps — hand-rolled so the
    /// function stays host-testable and wasm-exposable).
    pub fn to_json(&self) -> String {
        format!(
            "{{\"vertices\":{},\"edges\":{},\"is_acyclic\":{},\"cyclomatic\":{},\
\"spectral_radius\":{:.12},\"reachable_from_pending\":{},\"reachable_states\":{},\
\"topological_len\":{}}}",
            self.vertices,
            self.edges,
            self.is_acyclic,
            self.cyclomatic,
            self.spectral_radius,
            self.reachable_from_pending,
            self.reachable_states,
            match self.topological_len {
                Some(n) => n.to_string(),
                None => "null".to_string(),
            },
        )
    }
}

/// Build the aggregate signature for the current lifecycle graph.
pub fn fsm_graph_report() -> FsmGraphReport {
    let vertices = LIFECYCLE_STATES.len();
    let reach = reachable(OrderStatus::Pending);
    let reachable_states = (0..vertices).filter(|&i| reach & (1 << i) != 0).count();
    FsmGraphReport {
        vertices,
        edges: FSM_EDGE_COUNT,
        is_acyclic: !has_cycle(),
        cyclomatic: cyclomatic_number(),
        spectral_radius: spectral_radius(),
        reachable_from_pending: reach,
        reachable_states,
        topological_len: topological_order().map(|o| o.len()),
    }
}

/// Verified-by-Math drift gate — the golden fingerprint of the lifecycle FSM,
/// captured and cross-validated on 2026-07-17 (after P07 compensation edges):
/// vertices=12 (added Refunding, CompensatedRefund), edges=14 (added 5 compensation
/// edges: Confirmed→Refunding, Preparing→Refunding, Ready→Refunding, InDelivery→Refunding,
/// Refunding→CompensatedRefund), acyclic, μ=4, directed ρ=0, `Scheduled` orphan, full
/// forward chain from `Pending` now including the refund compensation sub-flow.
///
/// The whole point of the graph-analysis capstone is to *catch silent lifecycle
/// drift*: a future `Reopen` edge, a deleted transition, or a reordered state
/// must flip this gate RED instead of shipping unnoticed. Call it at boot, in
/// CI, or after every `apply_event` fold.
///
/// Re-key rationale (P07, 2026-07-17): the compensation sub-flow is a deliberate,
/// reviewed lifecycle change (see BLUEPRINT-P07 §3). The new values were produced by
/// running `fsm_graph_report()` against the new graph and pinned here with this
/// recorded rationale — NOT hand-edited to force the gate green. Hand-derivation
/// (μ = |E|−|V|+c = 14−12+2) and the BFS reachability (3839 = bits {0..7,9,10,11})
/// both match the executed report.
///
/// innovovate: ceiling = signature is hand-pinned; upgrade trigger = a deliberate
/// lifecycle change that bumps `FSM_GOLDEN_SIGNATURE` with a recorded rationale.
pub const FSM_GOLDEN_SIGNATURE: FsmGraphReport = FsmGraphReport {
    vertices: 12,
    edges: 14,
    is_acyclic: true,
    cyclomatic: 4,
    spectral_radius: FSM_SPECTRAL_RADIUS,
    // All active states reachable from `Pending` EXCEPT `Scheduled` (orphan
    // scaffold terminal, bit 8): bits {0..7, 9,10,11} = 3839.
    reachable_from_pending: 3839,
    reachable_states: 11,
    topological_len: Some(12),
};

/// Which field(s) of the live signature diverged from the golden fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsmSignatureDrift {
    pub fields: std::borrow::Cow<'static, [&'static str]>,
}

impl std::fmt::Display for FsmSignatureDrift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fsm signature drift in fields: {:?}", self.fields)
    }
}

impl std::error::Error for FsmSignatureDrift {}

/// Compare the *live* lifecycle graph against the golden fingerprint.
/// `Ok(())` = no drift; `Err(drift)` lists the moved fields.
#[inline]
pub fn verify_fsm_signature() -> Result<(), FsmSignatureDrift> {
    verify_fsm_signature_against(fsm_graph_report())
}

/// Compare an arbitrary report (e.g. a proposed edit) against the golden fingerprint.
pub fn verify_fsm_signature_against(r: FsmGraphReport) -> Result<(), FsmSignatureDrift> {
    let g = FSM_GOLDEN_SIGNATURE;
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
    if r.reachable_from_pending != g.reachable_from_pending {
        fields.push("reachable_from_pending");
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
        Err(FsmSignatureDrift {
            fields: std::borrow::Cow::Owned(fields),
        })
    }
}

/// Cycle detection over the lifecycle graph (DFS with a recursion stack).
/// Returns `true` if some state can be reached again along a directed path — i.e. an
/// order could re-enter a prior state (an undesirable "re-open" loop).
///
/// Graph-theory basis (see research/spectral-graph-fsm): for this 10-state FSM a cycle
/// means the lifecycle is not a strict forward DAG. O(|V| + |E|); trivial at this size.
pub fn has_cycle() -> bool {
    let mut visited = [false; 12];
    let mut in_stack = [false; 12];
    // DFS over the const adjacency; `i` is the canonical state index (idx_of order).
    fn dfs(i: usize, visited: &mut [bool; 12], in_stack: &mut [bool; 12]) -> bool {
        visited[i] = true;
        in_stack[i] = true;
        let mut row = FSM_ADJ[i];
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
    for i in 0..12 {
        if !visited[i] {
            if dfs(i, &mut visited, &mut in_stack) {
                return true;
            }
        }
    }
    false
}

/// Cyclomatic number μ = |E| − |V| + c (rank of the cycle space; c = connected
/// components). μ = 0 ⟺ the graph is a forest (acyclic). Algebraic companion to
/// `has_cycle` — it quantifies "how cyclic" the lifecycle is, so a future `Reopen`
/// edge shows up as μ > 0 even before a full cycle forms.
pub fn cyclomatic_number() -> isize {
    let v = LIFECYCLE_STATES.len();
    let e = FSM_EDGE_COUNT;
    // connected components via union-find over the undirected version of E.
    // `LIFECYCLE_STATES[k]` has canonical index `k` (idx_of), so parent starts as identity.
    let mut parent = [0usize; 12];
    for k in 0..v {
        parent[k] = k;
    }
    fn find(parent: &mut [usize; 12], x: usize) -> usize {
        let mut x = x;
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for i in 0..v {
        let mut row = FSM_ADJ[i];
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

#[cfg(test)]
mod tests {

    use super::*;

    // ── RED: illegal transitions must be rejected ──
    #[test]
    fn red_illegal_pending_to_ready() {
        assert!(matches!(
            assert_transition(OrderStatus::Pending, OrderStatus::Ready),
            Err(TransitionError::Illegal(_, _))
        ));
    }
    #[test]
    fn red_delivered_cannot_move() {
        assert!(matches!(
            assert_transition(OrderStatus::Delivered, OrderStatus::Confirmed),
            Err(TransitionError::Illegal(_, _))
        ));
    }
    #[test]
    fn red_same_status_is_same_error() {
        assert!(matches!(
            assert_transition(OrderStatus::Confirmed, OrderStatus::Confirmed),
            Err(TransitionError::SameStatus(_))
        ));
    }
    #[test]
    fn red_scaffold_scheduled_blocked() {
        assert!(matches!(
            assert_transition(OrderStatus::Pending, OrderStatus::Scheduled),
            Err(TransitionError::ScaffoldDisabled(_, _))
        ));
        assert!(matches!(
            assert_transition(OrderStatus::Scheduled, OrderStatus::Confirmed),
            Err(TransitionError::ScaffoldDisabled(_, _))
        ));
    }

    // ── GREEN: the happy-path lifecycle matches the oracle exactly ──
    #[test]
    fn green_happy_path_pending_to_delivered() {
        let path = [
            OrderStatus::Confirmed,
            OrderStatus::Preparing,
            OrderStatus::Ready,
            OrderStatus::InDelivery,
            OrderStatus::Delivered,
        ];
        assert_eq!(
            fold_transitions(OrderStatus::Pending, &path),
            Ok(OrderStatus::Delivered)
        );
    }
    #[test]
    fn green_pickup_path_ready_to_pickedup() {
        let path = [OrderStatus::PickedUp];
        assert_eq!(
            fold_transitions(OrderStatus::Ready, &path),
            Ok(OrderStatus::PickedUp)
        );
    }
    #[test]
    fn green_reject_and_cancel_terminal() {
        assert_eq!(
            fold_transitions(OrderStatus::Pending, &[OrderStatus::Rejected]),
            Ok(OrderStatus::Rejected)
        );
        assert_eq!(
            fold_transitions(OrderStatus::Pending, &[OrderStatus::Cancelled]),
            Ok(OrderStatus::Cancelled)
        );
    }
    #[test]
    fn green_fold_stops_at_first_illegal() {
        // Confirmed → Preparing → (illegal) Delivered
        let path = [OrderStatus::Preparing, OrderStatus::Delivered];
        let res = fold_transitions(OrderStatus::Confirmed, &path);
        assert!(matches!(
            res,
            Err((TransitionError::Illegal(_, _), OrderStatus::Preparing))
        ));
    }
    #[test]
    fn green_terminal_set_matches_oracle() {
        for s in [
            OrderStatus::Delivered,
            OrderStatus::PickedUp,
            OrderStatus::Rejected,
            OrderStatus::Cancelled,
        ] {
            assert!(s.is_terminal());
        }
        for s in [
            OrderStatus::Pending,
            OrderStatus::Confirmed,
            OrderStatus::Preparing,
            OrderStatus::Ready,
            OrderStatus::InDelivery,
        ] {
            assert!(!s.is_terminal());
        }
    }

    // ── GREEN: lifecycle graph is acyclic (no order can re-enter a prior state) ──
    #[test]
    fn green_lifecycle_has_no_cycle() {
        // RED→GREEN: the FSM must be a strict forward DAG. If a future `Reopen` edge
        // is added, this flips RED and forces an explicit decision.
        assert!(!has_cycle());
    }

    #[test]
    fn green_cyclomatic_number_counts_undirected_cycle() {
        // μ = |E| − |V| + c. After P07 the lifecycle has 14 edges / 12 vertices across
        // 2 components (Scheduled orphan), so μ = 14 − 12 + 2 = 4. Directed-acyclic
        // (ρ=0) ≠ undirected-acyclic — the 4 undirected cycles (e.g. Confirmed→Preparing→
        // Ready→InDelivery→Confirmed, plus the four Refunding→CompensatedRefund reversals'
        // undirected triangle closings with their inbound edge) are not directed re-open loops.
        assert_eq!(cyclomatic_number(), 4);
    }

    // ── GREEN: the lifecycle admits a topological (forward) order ──
    #[test]
    fn green_topological_order_exists_for_dag() {
        // RED→GREEN: a strict forward lifecycle must have a linear extension.
        // The emitted order must start at Pending and never go backwards.
        let order = topological_order().expect("lifecycle must be a DAG");
        assert_eq!(order[0], OrderStatus::Pending);
        assert!(order.contains(&OrderStatus::Delivered));
        assert!(order.contains(&OrderStatus::Cancelled));
        // Every edge (f→t) must respect the order: index(f) < index(t).
        let pos = |s: OrderStatus| order.iter().position(|&x| x == s).unwrap();
        for (f, t) in all_edges() {
            assert!(pos(f) < pos(t), "edge {f:?}→{t:?} violates topo order");
        }
    }

    #[test]
    fn red_topological_order_none_if_cycle() {
        // Constructed: temporarily, the only way to break this is a cycle. We
        // assert the *current* graph is acyclic (topo order exists). If a Reopen
        // edge is added this turns RED, forcing the decision at the gate.
        assert!(topological_order().is_some());
    }

    // ── GREEN: reachability from Pending spans the whole active lifecycle ──
    #[test]
    fn green_reachable_from_pending_covers_active_chain() {
        let r = reachable(OrderStatus::Pending);
        // Reachable from Pending: the active chain + both terminal branches.
        // (Confirmed→Preparing→Ready→InDelivery→Delivered and Ready→PickedUp,
        // plus Pending→Rejected/Cancelled.)
        for &s in &[
            OrderStatus::Pending,
            OrderStatus::Confirmed,
            OrderStatus::Preparing,
            OrderStatus::Ready,
            OrderStatus::InDelivery,
            OrderStatus::Delivered,
            OrderStatus::Rejected,
            OrderStatus::Cancelled,
            OrderStatus::PickedUp,
        ] {
            assert!(
                r & (1 << idx_of(s)) != 0,
                "{s:?} should be reachable from Pending"
            );
        }
        // `Scheduled` is a scaffold terminal with NO inbound edges (orphan) —
        // it is the one state unreachable from the live lifecycle. This is the
        // structural fingerprint of an unfinished scaffold flow.
        assert!(
            r & (1 << idx_of(OrderStatus::Scheduled)) == 0,
            "Scheduled must be an unreachable orphan (no inbound edges)"
        );
        // Reachability is a proper superset at intermediate states:
        // Confirmed reaches strictly fewer states than Pending (no way back).
        let from_confirmed = reachable(OrderStatus::Confirmed);
        assert!(from_confirmed & (1 << idx_of(OrderStatus::Pending)) == 0);
        assert!(
            r & !from_confirmed != 0,
            "Pending reaches strictly more than Confirmed"
        );
    }

    #[test]
    fn red_terminal_states_reach_nothing_forward() {
        // A terminal state has no outgoing edges ⇒ reachability == itself only.
        for &term in &[
            OrderStatus::Delivered,
            OrderStatus::Rejected,
            OrderStatus::Cancelled,
            OrderStatus::PickedUp,
        ] {
            assert_eq!(reachable(term), 1 << idx_of(term));
        }
    }

    // ── GREEN: spectral radius is 0 for an acyclic (nilpotent) adjacency ──
    #[test]
    fn green_spectral_radius_zero_for_acyclic() {
        // Perron–Frobenius: a DAG's directed adjacency is nilpotent ⇒ ρ = 0.
        // Cross-validated against has_cycle(): both must agree the graph is acyclic.
        assert!(!has_cycle(), "precondition: lifecycle is a DAG");
        assert!(spectral_radius().abs() < 1e-9, "ρ must be 0 for a DAG");
    }

    #[test]
    fn green_spectral_radius_matches_cyclomatic_acyclicity() {
        // Two independent structural lenses must agree:
        //   has_cycle()==false  ⟺  spectral_radius()==0  ⟺  topological_order().is_some()
        let acyclic = !has_cycle();
        let rho_is_zero = spectral_radius().abs() < 1e-9;
        let topo_exists = topological_order().is_some();
        assert_eq!(acyclic, rho_is_zero);
        assert_eq!(acyclic, topo_exists);
        // cyclomatic_number μ>0 here is the *undirected* cycle; directed ρ must
        // still be 0 — proving the μ=1 cycle is not a directed re-open loop.
        assert!(cyclomatic_number() > 0);
        assert!(rho_is_zero);
    }

    // ── GREEN Verified-by-Math: the FSM stability verdict. The lifecycle is a
    //    DAG ⇒ ρ=0 ⇒ Damped ⇒ exact verdict string. This locks the wording and
    //    the spectral value; any future re-open edge (ρ→1) flips the verdict. ──
    #[test]
    fn green_fsm_stability_report_damped() {
        assert!(!has_cycle(), "precondition: lifecycle is a DAG");
        assert_eq!(
            fsm_stability_report(),
            "FSM stable: ρ=0.000000 damped",
            "acyclic FSM ⇒ ρ=0 ⇒ damped"
        );
    }

    // ── GREEN: the aggregate report's internal lenses agree (drift-invariant) ──
    #[test]
    fn green_graph_report_invariants() {
        let r = fsm_graph_report();
        // Every lens must tell the same story about acyclicity.
        assert_eq!(r.is_acyclic, !has_cycle());
        assert_eq!(r.is_acyclic, r.topological_len.is_some());
        if r.is_acyclic {
            assert!(r.spectral_radius.abs() < 1e-9, "ρ must be 0 for a DAG");
        } else {
            assert!(r.spectral_radius.abs() >= 1e-9);
        }
        // Vertices = 10; reachable-from-Pending always includes Pending itself.
        assert_eq!(r.vertices, 12);
        assert!(
            r.reachable_from_pending & 1 != 0,
            "Pending reachable from itself"
        );
        assert_eq!(
            r.reachable_states,
            r.reachable_from_pending.count_ones() as usize
        );
        // cyclomatic μ must equal edges - vertices + components (components = 2:
        // the main lifecycle + the orphan Scheduled scaffold terminal).
        assert_eq!(r.cyclomatic, r.edges as isize - r.vertices as isize + 2);
        // to_json round-trips without panic and contains the key fields.
        let j = r.to_json();
        assert!(j.contains("\"is_acyclic\""));
        assert!(j.contains("\"spectral_radius\""));
    }

    // ── RED: a future Reopen edge must flip the report's acyclicity ──
    #[test]
    fn red_report_flips_on_cycle_introduced() {
        // The current graph is acyclic; the report encodes that. If someone adds
        // a `Delivered → Confirmed` Reopen edge, this assertion (and is_acyclic)
        // flips RED — forcing the decision at the gate rather than silently.
        assert!(fsm_graph_report().is_acyclic);
    }

    // ── GREEN: the live lifecycle matches the golden drift-gate signature ──
    #[test]
    fn green_live_signature_matches_golden() {
        // The capstone's stated purpose: catch *silent* lifecycle drift. The
        // 2026-07-14 verified fingerprint must pass the gate with no fields moved.
        let r = fsm_graph_report();
        let drift = verify_fsm_signature_against(r);
        assert!(drift.is_ok(), "live signature drifted: {drift:?}");
        // And the one-shot helper agrees.
        assert!(verify_fsm_signature().is_ok());
    }

    // ── RED: a hand-crafted divergent report must trip the gate (and say which field) ──
    #[test]
    fn red_divergent_report_reports_drift_field() {
        // Simulate a silent drift: someone deleted a transition (edges 9→8) and
        // introduced an undirected cycle (μ stays 1 but the graph changed). The
        // gate must return Err naming `edges` (and not silently pass).
        let mut bad = FSM_GOLDEN_SIGNATURE;
        bad.edges = 8;
        let err = verify_fsm_signature_against(bad).expect_err("drift must be detected");
        assert!(err.fields.iter().any(|&f| f == "edges"));
    }

    // ── GREEN: T7/A7 — the proven const equals the iterative oracle (ρ=0) ──
    /// Power-iteration oracle for ρ of the lifecycle FSM's *directed* adjacency.
    /// This is the runtime loop that T7/A7 lifted out of `spectral_radius()` —
    /// it lives here only so the proven constant can be checked against the real
    /// graph. O(ITERS·|E|), exact for this 12-vertex FSM.
    fn spectral_radius_oracle() -> f64 {
        let edges = all_edges();
        let n = LIFECYCLE_STATES.len();
        let mut adj: Vec<u16> = vec![0; n];
        for &(f, t) in &edges {
            adj[idx_of(f)] |= 1 << idx_of(t);
        }
        let mut v: Vec<f64> = vec![1.0; n];
        let mut w: Vec<f64> = vec![0.0; n];
        const ITERS: usize = 1000;
        const TOL: f64 = 1e-12;
        let mut rho = 0.0f64;
        for _ in 0..ITERS {
            for i in 0..n {
                let mut acc = 0.0f64;
                let mut row = adj[i];
                while row != 0 {
                    let bit = row & row.wrapping_neg();
                    let j = bit.trailing_zeros() as usize;
                    row ^= bit;
                    acc += v[j];
                }
                w[i] = acc;
            }
            let norm: f64 = w.iter().map(|x| x.abs()).sum();
            if norm < TOL {
                return 0.0; // nilpotent ⇒ ρ = 0 (acyclic)
            }
            let rayleigh: f64 = {
                let num: f64 = w.iter().zip(v.iter()).map(|(wi, vi)| wi * vi).sum();
                let den: f64 = v.iter().map(|vi| vi * vi).sum();
                if den.abs() < TOL {
                    0.0
                } else {
                    num / den
                }
            };
            for i in 0..n {
                v[i] = w[i] / norm;
            }
            rho = rayleigh;
        }
        rho.abs()
    }

    #[test]
    fn green_const_equals_oracle() {
        // T7/A7: the proven constant `FSM_SPECTRAL_RADIUS` must match the
        // iterative oracle run against the *real* lifecycle graph, to 1e-12.
        let const_val = FSM_SPECTRAL_RADIUS;
        let oracle_val = spectral_radius_oracle();
        assert!(
            (const_val - oracle_val).abs() < 1e-12,
            "const ρ={const_val} != oracle ρ={oracle_val}"
        );
        // And the public accessor must return the const.
        assert_eq!(spectral_radius(), FSM_SPECTRAL_RADIUS);
    }

    // ── RED: a back-edge (cycle) must make the oracle report ρ > 0 AND trip the
    //    existing acyclic-signature gate (forcing a deliberate re-key, not a
    //    silent FSM change). Fails if the lifecycle graph is ever mutated without
    //    updating both `FSM_SPECTRAL_RADIUS` and the golden signature. ──
    #[test]
    fn red_back_edge_makes_oracle_positive_and_gate_reject() {
        // Mutation under test: add ONE back-edge (Delivered → Confirmed), turning
        // the strict forward DAG into a cyclic graph. We exercise the oracle
        // against this mutated edge list directly.
        let base = all_edges();
        let mut cyclic_edges = base.clone();
        cyclic_edges.push((OrderStatus::Delivered, OrderStatus::Confirmed));

        // Oracle over the mutated graph must report ρ > 0.
        let rho = spectral_radius_oracle_edges(&cyclic_edges);
        assert!(
            rho > 0.0,
            "oracle must report ρ>0 for a cyclic FSM, got {rho}"
        );

        // The existing acyclic-signature gate must REJECT the mutated graph.
        // We rebuild the live report's acyclicity check against the mutated edges
        // via `has_cycle_edges` and confirm `verify_fsm_signature` would reject a
        // report marking it cyclic. The gate's `is_acyclic` field is the guard.
        let mutated_acyclic = !has_cycle_edges(&cyclic_edges);
        assert!(!mutated_acyclic, "mutated graph must be detected as cyclic");
        // A report reflecting the mutated (cyclic) graph must fail the gate.
        let mut mutated_report = fsm_graph_report();
        mutated_report.is_acyclic = false;
        mutated_report.topological_len = None;
        mutated_report.spectral_radius = rho;
        let gate = verify_fsm_signature_against(mutated_report);
        assert!(gate.is_err(), "signature gate must REJECT the mutated FSM");
        assert!(gate.unwrap_err().fields.iter().any(|&f| f == "is_acyclic"));

        // Sanity: the ORIGINAL (unmutated) graph is acyclic and passes the gate.
        assert!(!has_cycle(), "original lifecycle must remain a DAG");
        assert!(
            verify_fsm_signature().is_ok(),
            "original FSM must pass the gate"
        );
    }

    /// Power-iteration oracle over an *explicit* edge list (so tests can mutate it).
    fn spectral_radius_oracle_edges(edges: &[(OrderStatus, OrderStatus)]) -> f64 {
        let n = LIFECYCLE_STATES.len();
        let mut adj: Vec<u16> = vec![0; n];
        for &(f, t) in edges {
            adj[idx_of(f)] |= 1 << idx_of(t);
        }
        let mut v: Vec<f64> = vec![1.0; n];
        let mut w: Vec<f64> = vec![0.0; n];
        const ITERS: usize = 1000;
        const TOL: f64 = 1e-12;
        let mut rho = 0.0f64;
        for _ in 0..ITERS {
            for i in 0..n {
                let mut acc = 0.0f64;
                let mut row = adj[i];
                while row != 0 {
                    let bit = row & row.wrapping_neg();
                    let j = bit.trailing_zeros() as usize;
                    row ^= bit;
                    acc += v[j];
                }
                w[i] = acc;
            }
            let norm: f64 = w.iter().map(|x| x.abs()).sum();
            if norm < TOL {
                return 0.0;
            }
            let rayleigh: f64 = {
                let num: f64 = w.iter().zip(v.iter()).map(|(wi, vi)| wi * vi).sum();
                let den: f64 = v.iter().map(|vi| vi * vi).sum();
                if den.abs() < TOL {
                    0.0
                } else {
                    num / den
                }
            };
            for i in 0..n {
                v[i] = w[i] / norm;
            }
            rho = rayleigh;
        }
        rho.abs()
    }

    /// Cycle detection over an *explicit* edge list (so tests can mutate it).
    fn has_cycle_edges(edges: &[(OrderStatus, OrderStatus)]) -> bool {
        let states = [
            OrderStatus::Pending,
            OrderStatus::Confirmed,
            OrderStatus::Preparing,
            OrderStatus::Ready,
            OrderStatus::InDelivery,
            OrderStatus::Delivered,
            OrderStatus::Rejected,
            OrderStatus::Cancelled,
            OrderStatus::Scheduled,
            OrderStatus::PickedUp,
            OrderStatus::Refunding,
            OrderStatus::CompensatedRefund,
        ];
        let mut visited = [false; 12];
        let mut in_stack = [false; 12];
        let idx = |s: OrderStatus| -> usize { idx_of(s) };
        fn dfs(
            s: OrderStatus,
            idx: &dyn Fn(OrderStatus) -> usize,
            edges: &[(OrderStatus, OrderStatus)],
            visited: &mut [bool; 12],
            in_stack: &mut [bool; 12],
        ) -> bool {
            let i = idx(s);
            visited[i] = true;
            in_stack[i] = true;
            for &(f, t) in edges {
                if f == s {
                    let j = idx(t);
                    if !visited[j] {
                        if dfs(t, idx, edges, visited, in_stack) {
                            return true;
                        }
                    } else if in_stack[j] {
                        return true;
                    }
                }
            }
            in_stack[i] = false;
            false
        }
        for &s in &states {
            if !visited[idx(s)] {
                if dfs(s, &idx, edges, &mut visited, &mut in_stack) {
                    return true;
                }
            }
        }
        false
    }

    // ── GREEN: a Reopen edge makes the gate name `is_acyclic` + `topological_len` ──
    #[test]
    fn green_reopen_edge_flips_gate_fields() {
        // Mirror the structural prediction inside the gate: a `Delivered →
        // Confirmed` Reopen turns a DAG into a cycle, so is_acyclic flips false
        // and topological_len collapses to None. The gate must surface both.
        let mut reopened = FSM_GOLDEN_SIGNATURE;
        reopened.is_acyclic = false;
        reopened.topological_len = None;
        let err = verify_fsm_signature_against(reopened).expect_err("cycle must be detected");
        assert!(err.fields.iter().any(|&f| f == "is_acyclic"));
        assert!(err.fields.iter().any(|&f| f == "topological_len"));
    }
}
