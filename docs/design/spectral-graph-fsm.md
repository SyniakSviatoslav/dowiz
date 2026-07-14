# spectral-graph-fsm — Spectral Graph Theory × Order-Lifecycle FSM

> Roadmap item `spectral-graph-fsm`. Grounded strictly in `kernel/src/order_machine.rs`
> (committed functions only — no invented APIs). Operator: SyniakSviatoslav (DeliveryOS).
>
> **Verified-by-Math box**
> - `μ = |E| − |V| + c = 9 − 10 + 2 = 1` ✓ (closed-form, below)
> - `reachable_from_pending = 767 = 0b1011111111` ✓ (closed-form, below)
> - `ρ = 0 ⟺ has_cycle() == false ⟺ topological_order() == Some` ✓ (Perron–Frobenius, below)

---

## 1. Hypothesis: ρ is the drift-pressure alarm; μ is not

The lifecycle is a **directed** graph `G = (V, E)` over the 10 `OrderStatus` vertices with the
9 legal edges from `allowed_next`. We build its **directed adjacency matrix** `A` (entry
`A[i][j] = 1` iff `i → j` is a legal transition — `spectral_radius()` does exactly this, rows
as `u16` bitmasks over `LIFECYCLE_STATES`).

**Hypothesis.** The spectral radius `ρ(A)` — the largest-magnitude eigenvalue of the directed
adjacency — is the correct scalar **drift-pressure alarm** for the lifecycle. Today the live
graph is a DAG, so `ρ = 0` exactly. A future `Reopen` edge that closes a directed cycle
(e.g. `Delivered → Confirmed`) must raise `ρ` from `0` toward `1`, a hard structural signal.

**Why `μ` (cyclomatic) is the wrong lens** — and this is the non-obvious part the operator's
substrate surfaces concretely:

- `cyclomatic_number()` returns `μ = |E| − |V| + c`, the **undirected** cycle rank.
- The current lifecycle is *directed-acyclic* (`has_cycle() == false`, `ρ = 0`) yet already has
  `μ = 1`, because its **undirected** version contains a cycle:
  `Confirmed → Preparing → Ready → InDelivery → Confirmed` (edges `Confirmed→Preparing`,
  `Preparing→Ready`, `Ready→InDelivery`, `Confirmed→InDelivery`).
- Therefore `μ = 1` today and a `Reopen` edge does **not** necessarily move `μ` in a clean way;
  `μ` reacts to *undirected* cycle count, which is unrelated to whether orders can re-enter a
  prior state. `μ` is a useful structural companion, not a drift detector.
- `ρ`, by contrast, is `0` for the DAG and jumps to `1` for a 2-cycle (`Delivered ↔ Confirmed`
  has period 2 ⇒ an eigenvalue of magnitude 1 ⇒ `ρ = 1`). A forward-only `Reopen` of length
  `ℓ` yields `ρ = 1` as well (a pure directed `ℓ`-cycle has spectral radius 1).

So `ρ` is a **directed** structural signal, distinct from `μ`, and distinct from the
combinatorial tests. The three lenses are not redundant; they co-vary only on the acyclicity axis
(see §2). The test `green_spectral_radius_matches_cyclomatic_acyclicity` already asserts the
subtle fact: `cyclomatic_number() > 0` **and** `ρ == 0`, proving the `μ = 1` cycle is *not* a
directed re-open loop.

---

## 2. Cross-validation: the three acyclicity lenses must co-vary

Three independent probes in `order_machine.rs` each answer "is the lifecycle a DAG?":

| Lens | Function | Returns acyclic iff |
|------|----------|--------------------|
| combinatorial DFS | `has_cycle()` | `false` |
| spectral | `spectral_radius()` | `== 0` |
| constructive | `topological_order()` | `== Some` |

**Why they must agree (Perron–Frobenius + linear algebra):**

1. **DAG ⟺ topological order exists.** `topological_order()` is Kahn's algorithm (1962): it
   repeatedly removes in-degree-0 sources. The queue empties and emits all `|V|` vertices
   *iff* no cycle remains. So `topological_order() == Some` ⟺ the graph is a DAG.

2. **DAG ⟺ adjacency is nilpotent.** A DAG induces a strict partial order on `V` (no directed
   cycle ⇒ no closed directed walk). Order the vertices by any topological extension; then `A`
   becomes strictly upper-triangular, hence **nilpotent**: `A^k = 0` for all `k ≥ |V|`.
   Conversely, if `A` is nilpotent, `A^k = 0` ⇒ no closed directed walk of length `k` ⇒ no
   directed cycle ⇒ DAG.

3. **Nilpotent ⟺ ρ = 0.** The spectral radius is the max `|λ|` over eigenvalues. A nilpotent
   matrix has *all* eigenvalues 0, so `ρ(A) = 0`. This is exactly the Perron–Frobenius
   corollary the code comment cites. `spectral_radius()` returns early with `0.0` when the
   power-iteration norm drops below `TOL` — i.e. when `w = A·v` collapses, the nilpotent case.

4. **Non-DAG ⟺ ρ > 0.** A directed cycle of length `ℓ` is a non-negative matrix with an
   eigenvalue `e^{2πi k/ℓ}` (magnitude 1). By Perron–Frobenius the spectral radius of a
   non-negative matrix with a cycle is at least the max cycle mean; a cycle gives `ρ ≥ 1 > 0`.

**Conclusion:** `has_cycle() == false ⟺ spectral_radius() == 0 ⟺ topological_order().is_some()`.
They are three faces of the same theorem, so any silent `allowed_next` edit that breaks one must
break all three. The aggregate `fsm_graph_report()` and the test
`green_spectral_radius_matches_cyclomatic_acyclicity` already enforce this agreement. The
drift gate (`FSM_GOLDEN_SIGNATURE`) captures the *entire* fingerprint, so a flip in any lens is
caught.

---

## 3. Closed-form `reachable_from_pending` and the Scheduled-orphan fingerprint

`reachable(from)` is a BFS over `A` (bitmask `u16`, one bit per vertex, exact — no float/hash),
starting at `1 << idx_of(from)` and propagating successors. `idx_of` numbering:

```
0 Pending | 1 Confirmed | 2 Preparing | 3 Ready | 4 InDelivery
5 Delivered | 6 Rejected | 7 Cancelled | 8 Scheduled | 9 PickedUp
```

**Forward closure from `Pending`** (edges: `Pending→{Confirmed,Rejected,Cancelled}`;
`Confirmed→{Preparing,InDelivery}`; `Preparing→Ready`; `Ready→{InDelivery,PickedUp}`;
`InDelivery→Delivered`; terminals have no out-edges):

```
Pending(0) → Confirmed(1), Rejected(6), Cancelled(7)
Confirmed(1) → Preparing(2), InDelivery(4)
Preparing(2) → Ready(3)
Ready(3) → InDelivery(4), PickedUp(9)
InDelivery(4) → Delivered(5)
```

Reached vertices: `{0,1,2,3,4,5,6,7,9}`. **`Scheduled` (bit 8) is never reached** — it has
**no inbound edges** (`allowed_next` never lists it, and `assert_transition` rejects any
scaffold transition via `is_scaffold`).

**Closed form:**

```
reachable_from_pending = (2^10 − 1) − 2^8        // all 10 bits minus the Scheduled bit
                       = 1023 − 256
                       = 767
                       = 0b1011111111
reachable_states       = popcount(767) = 9
```

Both values match `FSM_GOLDEN_SIGNATURE` (`reachable_from_pending: 767`,
`reachable_states: 9`) and the test `green_reachable_from_pending_covers_active_chain`.

**The Scheduled-orphan as a reachability invariant.** Bit 8 being `0` is the structural
fingerprint of an *unfinished scaffold flow*: `Scheduled` is a declared terminal (`is_terminal`
does **not** include it, but `allowed_next(Scheduled) = []` and nothing points to it). This is a
**telemetry signal, not a bug** — it tells the operator the scheduled-delivery branch is not yet
wired. The invariant is *reachability-detectable*: if someone accidentally adds an inbound edge
(e.g. `Ready → Scheduled`), bit 8 flips to `1`, the mask becomes `1023`, and the gate trips on
`reachable_from_pending`. If `Scheduled` were deleted, `vertices` drops to 9 and the orphan
invariant disappears — also caught. The orphan is therefore a cheap, exact liveness probe for
partial-scaffold drift.

---

## 4. Wiring the drift gate as fail-closed runtime policy

No new kernel APIs are introduced. The gate is the existing const + two existing functions:

- `FSM_GOLDEN_SIGNATURE: FsmGraphReport` — the hand-pinned 2026-07-14 fingerprint.
- `verify_fsm_signature() -> Result<(), FsmSignatureDrift>` — compares `fsm_graph_report()`
  (which calls `has_cycle`, `cyclomatic_number`, `spectral_radius`, `topological_order`,
  `reachable`) against the const; `Err` lists the moved fields.
- `verify_fsm_signature_against(report)` — the same check against an arbitrary report (used by
  tests to simulate drift without mutating the live graph).

**Boot check.** At kernel init, before the event bus accepts traffic, call
`verify_fsm_signature()`. `Err(drift)` ⇒ the committed lifecycle no longer matches the recorded
fingerprint — refuse to start (fail-closed). This catches a bad merge / silent `allowed_next`
edit at the earliest point.

**Post-fold check in `domain::apply_event`.** `domain::apply_event(order, next)`
(`kernel/src/domain.rs:150`) already calls `assert_transition` per fold. After a *successful*
fold, call `verify_fsm_signature()`. A fold is a single-order state change and by construction
cannot alter `allowed_next`/the graph — so this check must *always* return `Ok(())`. If it
returns `Err`, the topology itself changed underneath the running kernel (e.g. a hot-reload or a
stray edit reached production), which is a structural regression, not a per-order error. Treat
it fail-closed: halt the fold, surface `drift.fields`, and refuse further transitions until
resolved.

**Operator-visible action when it trips (the "upgrade trigger"):**

1. The gate fired because a graph field diverged from `FSM_GOLDEN_SIGNATURE` (one of
   `vertices, edges, is_acyclic, cyclomatic, spectral_radius, reachable_from_pending,
   reachable_states, topological_len`). Read `drift.fields` to see which.
2. Decide deliberately: is this a sanctioned lifecycle redesign, or an accidental drift?
   - Accidental / silent: revert the `allowed_next` (or `OrderStatus`) edit. The gate clears
     with no code change to the signature.
   - Sanctioned redesign (e.g. a real `Reopen` feature): **bump `FSM_GOLDEN_SIGNATURE`** to the
     newly computed `fsm_graph_report()` values, and record the rationale next to the const
     (the existing comment block is the place: date + what changed + why). This is the only
     sanctioned way to clear a structural-drift trip.
3. Never suppress the gate (no `#[allow]`, no tolerance widening beyond the existing `1e-9` on
   `spectral_radius` and the exact-equality checks on the integer fields). The gate is a
   fingerprint, and a deliberate redesign *should* trip it — that trip is the recorded decision
   point.

This matches the const's documented contract: "ceiling = signature is hand-pinned; upgrade
trigger = a deliberate lifecycle change that bumps `FSM_GOLDEN_SIGNATURE` with a recorded
rationale."

---

## 5. Limitations (honest)

- **Power-iteration convergence.** `spectral_radius()` converges geometrically with per-step
  ratio `|λ₂/λ₁|` (the gap to the sub-dominant eigenvalue). For the current DAG, `A` is
  nilpotent, so `A^k = 0` for `k ≥ |V| = 10`; `ρ` is therefore detected **exactly** within ≤10
  iterations — the `ITERS = 1000` / `TOL = 1e-12` cap is overkill and yields no float drift at
  `n = 10`. For a *cyclic* graph the convergence rate depends on `|λ₂/λ₁|`; at `n = 10` the gap
  is large enough that `1e-12` is exact-enough for any realistic lifecycle. The gate's
  `1e-9` tolerance on `spectral_radius` (not `== 0`) is what makes the float comparison safe.
- **Fingerprint, not proof of semantic correctness.** The gate detects that the *graph* changed
  (vertices/edges/acyclicity/reachability/spectral radius). It does **not** validate business
  semantics: a sanctioned `Reopen` that trips the gate and gets a signature bump is
  structurally legal but still must be checked for e.g. subtotal recomputation, idempotency, and
  the `Invalid` RED-LINE invariants elsewhere in `apply_event`. `verify_fsm_signature` is a
  topology alarm, not a domain-logic verifier.
- **μ vs ρ are different axes.** `μ` counts *undirected* cycles; `ρ`/`has_cycle`/`topological`
  test *directed* acyclicity. They co-vary only on "is it a DAG?". The current `μ = 1` with
  `ρ = 0` is expected and correct — do not "fix" `μ` to 0. The gate captures `cyclomatic` as a
  field precisely so a *change* in the undirected cycle rank is also caught, even when
  directed-acyclicity is preserved.
- **Scope.** Analysis covers the 10 committed states and 9 committed edges only. Adding a state
  (e.g. wiring `Scheduled` into a real inbound flow) changes `vertices`, `components`, and the
  reachable mask simultaneously — all captured by the gate, but each bump requires a conscious
  signature update with rationale.
