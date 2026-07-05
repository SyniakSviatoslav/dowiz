# S2-AUTH CUTOVER AMENDMENT — Re-ratification RESOLVE

> **Verdict: the signed S2 cutover decision is UPHELD-AND-TIGHTENED, not overturned. Adopt Option C2
> (atomic-flip + refresh-quiesce), NOT bare-atomic and NOT C1 (family-sticky). No ETHICAL-STOP.**
> Status 🟡 — NOT RATIFIED until the operator signs §3. Re-ratification seats: S2-breaker (1 CRIT / 4
> HIGH), S2-counsel (RATIFY-WITH-CONDITION), lead. Both seats CONVERGE on C2 once the breaker's
> technical finding is in — counsel voted C1-primary with "C2-quiesce as fallback iff the breaker
> confirms it closes the split"; the breaker REFUTED C1 (uncomputable) and CONFIRMED C2 → the fallback
> condition is met → C2.

## The decorrelation that decided it
Counsel (ethics/process lens) voted to preserve the signed canary as **C1 family-sticky**
(`consistent-hash(family_id)→stack`), reading it as the signed decision "tightened, not overturned."
The S2-breaker (adversarial lens, verified vs live source) proved **C1 is not implementable**: `family_id`
is in NO JWT claim and NO refresh token — owner refresh is opaque `randomBytes` looked up by sha256
(`auth.ts:243-247`), courier's key is a per-rotation `sessionId` (`courier/auth.ts:453`); every
gate-computable proxy is per-TOKEN, so the concurrent-refresh split survives rotation (**AC1 CRIT**). C1's
own canary-WIDEN is `UPDATE cutover_flags + NOTIFY` → rides the SAME 1-5s pooler split it condemns
(**AH1**). What actually serializes cross-stack refreshes is **gate-iv** (shared-DB atomic UPDATE +
SQL-clock `now()`, `auth.ts:266,277`) — which BOTH postures require and NEITHER strengthens (**AH4**).
→ C1 is refuted; C2 (atomic + quiesce) is the implementable carrier of the amendment's sound instinct,
and it sidesteps AC1 entirely (quiesce needs no family key).

## 1. Frozen revision set (amendment REVs)
- **AR-1 (AQ1 — the posture).** REJECT bare-atomic (trip-wire is post-commit — fires AFTER the
  irreversible family-revoke; §3a CONFIRMED by both seats). REJECT C1 family-sticky (AC1: no
  gate-computable family key exists). **ADOPT C2 = atomic per-surface flip + refresh-quiesce**: briefly
  drain/hold in-flight refreshes across the flip boundary so no single family is served by two stacks
  concurrently. **gate-iv (byte-identical refresh SQL + SQL-clock `interval '5 seconds'`) is a HARD
  prerequisite in EVERY posture** — it, not the arm, is what serializes concurrent refreshes.
- **AR-2 (AQ2 — verification-parity is a HARD ordering interlock, the sharpest concrete bug).** The
  breaker found a GENUINE one-directional round-trip (**AH3**): body-kid + leeway **60 on Node vs 0 on
  Rust** (`jwt.ts:105-107`) → a pre-flip token verifies on Rust but **401s on Node**. Before ANY authed
  surface (S2/S3/S4/S5/S7-auth) flips, a **flag-interlock** must prove Node↔Rust body-kid round-trip in
  BOTH directions with leeway reconciled + strict-claims. §5's parity gate was documentary — MECHANIZE
  it (a flag the front-door reads; no flip while red). This gates the ordering of every authed flip.
- **AR-3 (AQ3 — courier/S7 invariant: RATIFY, but MECHANICAL not documentary).** `courier/auth.ts`
  mints the same body-kid RS256 token with the same `courier_sessions` family-revoke, on an INDEPENDENT
  per-surface flag with its own pooler window (**AH2**) — so a courier family can split/revoke mid-shift
  regardless of S2's posture. Bind the cutover invariant to the **token SHAPE**, not the route-owner
  accident, and enforce it MECHANICALLY: S7-auth's flip must be interlocked with the same quiesce +
  parity gate (AR-1/AR-2), not merely "documented to match." Counsel's dignity ground is recorded: the
  courier is the least-powerful, most-surveilled actor (mid-delivery, carrying cash) — the invariant
  protects the person who can least absorb a false revoke.
- **AR-4 (AQ4 — the recovery path, counsel's unasked question).** Every prior seat optimized the
  FREQUENCY of false revokes; none wrote recovery for whoever one HITS. RATIFY the per-flip cleanup
  runbook AND extend it: the owner path (support-mediated re-login at a desk) is inadequate for a
  soft-revoked courier mid-shift — "a support ticket is not recovery, it is abandonment." The runbook
  MUST include a fast **in-shift courier re-auth** written for the courier's reality. family-revoke is
  NOT rollback-recoverable → the runbook is a named precondition per auth-family flip.
- **AR-5 (process — the precedent, recorded).** A 2-seat mechanism-council REVISED a 4-seat SIGNED
  red-line decision. The precedent is named: **a signed red-line decision is deterministic AUTHORITY; a
  mechanism-council challenge is an advisory SIGNAL that routes BACK to the signing seats** — it cannot
  silently supersede. This was NOT a caught bypass: the cutover-council's own R-1 self-flagged and
  routed to 🔴 (process working correctly). This amendment IS that route-back functioning. gate-iv's
  author (S2-breaker) re-ran the check — the right seat closed the loop.

## 2. Open item folded (not left dangling)
The breaker's parting question — "what gate-computable, family-stable key can the front-door read?" — is
**MOOT under C2**: C2 quiesces refreshes across an atomic boundary and needs no canary routing key. It is
only live if a future seat resurrects a canary; recorded as such, not as an unresolved blocker.

## 3. 🔴 OPERATOR SIGN-OFF (blocks the auth cutover; nothing flips until signed)
AQ1 (C2 atomic+quiesce; bare-atomic + C1 both rejected; gate-iv hard prereq) · AQ2 (verification-parity
flag-interlock BOTH directions, leeway reconciled, before any authed flip) · AQ3 (S7 courier invariant,
mechanical interlock, token-shape-bound) · AQ4 (recovery runbook incl. in-shift courier re-auth) ·
AR-5 (the signed-decision-supersession precedent, for the record).

## 4. Cutover DoD deltas (auth surfaces)
C2 quiesce proven (no family served by two stacks across the flip boundary) · gate-iv byte-identical +
SQL-clock verified · verification-parity round-trip GREEN both directions (leeway reconciled) as a flag
interlock · S7-auth flip interlocked with S2 (same quiesce + parity) · per-family-flip recovery runbook
authored incl. courier in-shift re-auth. The auth flip stays a human go/no-go (irreversible family-revoke).
