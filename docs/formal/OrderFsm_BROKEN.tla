----------------------------- MODULE OrderFsm_BROKEN --------------------------
\* DELIBERATELY BROKEN twin of `OrderFsm.tla` — the item-10 falsifiability proof
\* (blueprint §4 / acceptance criterion 3: "a deliberately-broken spec variant fails").
\*
\* What is broken: `Successors("Pending")` was extended with the ILLEGAL edge
\* `Pending -> Ready` (not present in `allowed_next`, order_machine.rs:78, and not
\* a set bit in `FSM_ADJ`). TLC must report a `NoIllegalTransition` violation —
\* proving the verifier cannot be silently forged to green.
\*
\* NOTE: the added edge is a forward edge and introduces NO cycle, so TLC still
\* TERMINATES (finite state space) and reports the violation rather than looping.

EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS Pending, Confirmed, Preparing, Ready, InDelivery, Delivered,
          Rejected, Cancelled, Scheduled, PickedUp, Refunding, CompensatedRefund

STATES == {Pending, Confirmed, Preparing, Ready, InDelivery, Delivered,
            Rejected, Cancelled, Scheduled, PickedUp, Refunding, CompensatedRefund}

VARIABLES cur

IsLegal(from, to) ==
    \/ /\ from = Pending     /\ to \in {Confirmed, Rejected, Cancelled}
    \/ /\ from = Confirmed   /\ to \in {Preparing, InDelivery, Refunding}
    \/ /\ from = Preparing   /\ to \in {Ready, Refunding}
    \/ /\ from = Ready       /\ to \in {InDelivery, PickedUp, Refunding}
    \/ /\ from = InDelivery  /\ to \in {Delivered, Refunding}
    \/ /\ from = Refunding   /\ to = CompensatedRefund

\* BROKEN: injected illegal edge `Pending -> Ready` is NOT in `IsLegal`.
Successors(s) == { t \in STATES : IsLegal(s, t) } \cup
    IF s = Pending THEN {Ready} ELSE {}

IsTerminal(s) == s \in {
    Delivered, Rejected, Cancelled, Scheduled, PickedUp, CompensatedRefund
}

Init == cur \in STATES

Next == \/ \E t \in Successors(cur) : cur' = t

Spec == Init /\ [][Next]_cur

TypeOK == cur \in STATES

NoDeadlock == (Successors(cur) = {}) => IsTerminal(cur)

NoIllegalTransition == [][ (cur' # cur) => IsLegal(cur, cur') ]_cur

Reaches(s, t) ==
    LET Dfs(q, seen) ==
        / q = t
        /  u in Successors(q) : ~ (u in seen) / Dfs(u, seen p {u})
    IN  Dfs(s, {s})

HasCycle == / s in STATES : Reaches(s, s)

TerminatesOrCycles == ~HasCycle

=============================================================================
