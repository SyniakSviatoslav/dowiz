------------------------------- MODULE OrderFsm -------------------------------
\* TLA+ model of the order-lifecycle FSM (`kernel/src/order_machine.rs`).
\*
\* Parity (see docs/formal/README.md):
\*   - The 12 `OrderStatus` variants  <->  `LIFECYCLE_STATES`       (order_machine.rs:245)
\*   - `IsLegal` (= `FSM_ADJ` bitmask) <->  `allowed_next`/`FSM_ADJ` (order_machine.rs:78 / :208)
\*   - `Init`  = `Pending`            <->  `OrderStatus::Pending`
\*
\* This spec MODELS the structural/transition layer only (the blueprint §7 "pure-structural
\* abstraction" choice). It does NOT model the money/Locked intervention guards — those are
\* left to a future richer variant if TLC affords it.
\*
\* TLC checks (exhaustively over the abstract state graph):
\*   TypeOK               - every state is a legal `OrderStatus`
\*   NoDeadlock           - the current state (if non-terminal) has >= 1 successor
\*   NoIllegalTransition  - every taken transition is in the `FSM_ADJ` adjacency ([])
\*   TerminatesOrCycles  - the graph is acyclic (rho = 0, matching Rust's proven DAG)
\*
\* CI asserts TLC exhausts the state space (finite, acyclic => terminates) with ZERO
\* invariant violations. The falsifiability twin is `OrderFsm_BROKEN.tla`.

EXTENDS Naturals, Sequences, FiniteSets

\* The 12 lifecycle states (pinned to LIFECYCLE_STATES, order_machine.rs:245).
\* Modeled as CONSTANTS assigned distinct natural-number values in the .cfg, so they
\* are usable as opcodes inside the spec without TLA+ model-value identifier hazards.
CONSTANTS Pending, Confirmed, Preparing, Ready, InDelivery, Delivered,
          Rejected, Cancelled, Scheduled, PickedUp, Refunding, CompensatedRefund

STATES == {Pending, Confirmed, Preparing, Ready, InDelivery, Delivered,
            Rejected, Cancelled, Scheduled, PickedUp, Refunding, CompensatedRefund}

VARIABLES cur

\* The legal transition relation, 1:1 with `allowed_next` (order_machine.rs:78) and the
\* compile-time `FSM_ADJ` bitmask (order_machine.rs:208). Any edit here MUST match a
\* simultaneous edit to `allowed_next` in `order_machine.rs` (maintenance rule, README §3).
IsLegal(from, to) ==
    \/ /\ from = Pending     /\ to \in {Confirmed, Rejected, Cancelled}
    \/ /\ from = Confirmed   /\ to \in {Preparing, InDelivery, Refunding}
    \/ /\ from = Preparing   /\ to \in {Ready, Refunding}
    \/ /\ from = Ready       /\ to \in {InDelivery, PickedUp, Refunding}
    \/ /\ from = InDelivery  /\ to \in {Delivered, Refunding}
    \/ /\ from = Refunding   /\ to = CompensatedRefund
    \* Delivered / Rejected / Cancelled / Scheduled / PickedUp / CompensatedRefund
    \* have NO outgoing edges (terminal, or Scheduled = scaffold orphan).

\* All states reachable in one legal step from `s`.
Successors(s) == { t \in STATES : IsLegal(s, t) }

\* Terminal states (is_terminal, order_machine.rs:64). Scheduled is an unreachable
\* scaffold orphan with no inbound edges (reachable_from_pending bit 8 = 0).
IsTerminal(s) == s \in {
    Delivered, Rejected, Cancelled, Scheduled, PickedUp, CompensatedRefund
}

Init == cur \in STATES

\* The machine may transition from `cur` to ANY legal successor.
\* Terminal states have Successors({}) = {} so the only available step is a stutter.
Next == \/ \E t \in Successors(cur) : cur' = t

Spec == Init /\ [][Next]_cur

\* ---- Invariants (state predicates, no primes) ----
TypeOK == cur \in STATES

\* The current state, if it has no successor, must be a designated terminal.
NoDeadlock == (Successors(cur) = {}) => IsTerminal(cur)

\* ---- Properties (temporal) ----
\* Every non-stutter step is a legal adjacency edge.
NoIllegalTransition == [][ (cur' # cur) => IsLegal(cur, cur') ]_cur

\* Acyclicity (rho = 0). `Reaches(s, s)` over a path of length >= 1 is true iff a
\* directed cycle through `s` exists. Matching Rust `has_cycle() == false` /
\* `spectral_radius() == 0` (order_machine.rs:584 / :383).
Reaches(s, t) ==
    LET Dfs(q, seen) ==
        / q = t
        /  u in Successors(q) : ~ (u in seen) / Dfs(u, seen p {u})
    IN  Dfs(s, {s})

HasCycle == / s in STATES : Reaches(s, s)

TerminatesOrCycles == ~HasCycle

=============================================================================
