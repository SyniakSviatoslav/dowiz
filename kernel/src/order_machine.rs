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
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Delivered | Self::PickedUp | Self::Rejected | Self::Cancelled
        )
    }
}

/// Transition table — identical to the oracle's `TRANSITIONS`.
/// `SCHEDULED` is a scaffold terminal (the scheduled flow is not implemented yet).
fn allowed_next(from: OrderStatus) -> &'static [OrderStatus] {
    use OrderStatus::*;
    match from {
        Pending => &[Confirmed, Rejected, Cancelled],
        Confirmed => &[Preparing, InDelivery],
        Preparing => &[Ready],
        Ready => &[InDelivery, PickedUp],
        InDelivery => &[Delivered],
        Delivered => &[],
        Rejected => &[],
        Cancelled => &[],
        Scheduled => &[],
        PickedUp => &[],
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
        tracing::info_span!("fold_transitions", start = ?start, n_steps = steps.len()).entered();
    let mut cur = start;
    for &next in steps {
        assert_transition(cur, next).map_err(|e| (e, cur))?;
        cur = next;
    }
    tracing::debug!(final_status = ?cur, "order transition fold complete");
    Ok(cur)
}

/// All directed edges of the order-lifecycle graph: `(from, to)` for every legal
/// transition in `allowed_next`. Used by the graph analyses below (cycle / cyclomatic).
fn all_edges() -> Vec<(OrderStatus, OrderStatus)> {
    use OrderStatus::*;
    let mut edges = Vec::new();
    for &from in &[
        Pending, Confirmed, Preparing, Ready, InDelivery, Delivered, Rejected, Cancelled,
        Scheduled, PickedUp,
    ] {
        for &to in allowed_next(from) {
            edges.push((from, to));
        }
    }
    edges
}

/// Cycle detection over the lifecycle graph (DFS with a recursion stack).
/// Returns `true` if some state can be reached again along a directed path — i.e. an
/// order could re-enter a prior state (an undesirable "re-open" loop).
///
/// Graph-theory basis (see research/spectral-graph-fsm): for this 10-state FSM a cycle
/// means the lifecycle is not a strict forward DAG. O(|V| + |E|); trivial at this size.
pub fn has_cycle() -> bool {
    use OrderStatus::*;
    let states = [
        Pending, Confirmed, Preparing, Ready, InDelivery, Delivered, Rejected, Cancelled,
        Scheduled, PickedUp,
    ];
    let edges = all_edges();
    let mut visited = [false; 10];
    let mut in_stack = [false; 10];
    // index helper
    let idx = |s: OrderStatus| -> usize {
        match s {
            Pending => 0,
            Confirmed => 1,
            Preparing => 2,
            Ready => 3,
            InDelivery => 4,
            Delivered => 5,
            Rejected => 6,
            Cancelled => 7,
            Scheduled => 8,
            PickedUp => 9,
        }
    };
    fn dfs(
        s: OrderStatus,
        idx: &dyn Fn(OrderStatus) -> usize,
        edges: &[(OrderStatus, OrderStatus)],
        visited: &mut [bool; 10],
        in_stack: &mut [bool; 10],
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
                    return true; // back-edge ⇒ cycle
                }
            }
        }
        in_stack[i] = false;
        false
    }
    for &s in &states {
        if !visited[idx(s)] {
            if dfs(s, &idx, &edges, &mut visited, &mut in_stack) {
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
    use OrderStatus::*;
    let states = [
        Pending, Confirmed, Preparing, Ready, InDelivery, Delivered, Rejected, Cancelled,
        Scheduled, PickedUp,
    ];
    let edges = all_edges();
    let v = states.len();
    let e = edges.len();
    // connected components via union-find over the undirected version of E
    let mut parent = [0usize; 10];
    for (k, s) in states.iter().enumerate() {
        parent[k] = idx_of(*s);
    }
    fn find(parent: &mut [usize; 10], x: usize) -> usize {
        let mut x = x;
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    fn idx_of(s: OrderStatus) -> usize {
        match s {
            Pending => 0,
            Confirmed => 1,
            Preparing => 2,
            Ready => 3,
            InDelivery => 4,
            Delivered => 5,
            Rejected => 6,
            Cancelled => 7,
            Scheduled => 8,
            PickedUp => 9,
        }
    }
    for &(f, t) in &edges {
        let (a, b) = (find(&mut parent, idx_of(f)), find(&mut parent, idx_of(t)));
        if a != b {
            parent[a] = b;
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
        // μ = |E| − |V| + c. The lifecycle is a DIRECTED acyclic graph (has_cycle()
        // == false), but its undirected version contains one cycle:
        //   Confirmed → Preparing → Ready → InDelivery → Confirmed
        // (via edges Confirmed→Preparing, Preparing→Ready, Ready→InDelivery,
        // Confirmed→InDelivery). So μ = 1, NOT 0. Directed-acyclic ≠ undirected-acyclic
        // — a useful distinction the operator's growth-substrate surfaces concretely.
        assert_eq!(cyclomatic_number(), 1);
    }
}
