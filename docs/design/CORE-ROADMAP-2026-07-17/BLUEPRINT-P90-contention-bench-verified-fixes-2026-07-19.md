# BLUEPRINT P90 — Contention-bench verified lock fixes: registration + open ends (2026-07-19)

> **Standalone PLANNING blueprint (dowiz-kernel `budget.rs`/`token_bucket.rs`; bebop2 bus).** One
> coherent unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Research
> source: `docs/research/OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md` (exists **only** on local
> branch `perf/contention-bench-2026-07-18`, worktree `/root/dowiz-perf-contention`, commits
> `8c865805b` + `8256dbffb`). Discharges S2 §0's R17 fold-in obligation; scoped in
> `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` §2. Format precedent:
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
>
> **One sentence:** three flagged lock sites (S1 A1/A2/A3, GATED-bench-first by E12) were benchmarked
> under real multi-threaded contention; two fixes landed with measured wins and a new falsifier
> (`budget.rs` `Mutex<f64>`→`AtomicU64` CAS at **2.0×**; `token_bucket.rs` monotonic-clock hoist at
> **+6–18%**), two flagged sites produced evidence-backed **non-findings**, one algorithm swap (GCRA)
> is benched-but-operator-gated, and one bebop bus fix is verified-but-commit-blocked — so P90 is a
> **registration-and-ruling** unit whose "code" is already written and whose remaining work is three
> operator decisions plus a merge, not new implementation.

---

## VERDICT (stated up front, per session research discipline)

**REGISTER-AND-RULE — the engineering is done; the remaining surface is decisions, not code.** This
blueprint does **not** ask a worker to implement anything new. It exists to (a) record the measured
results as roadmap truth so no future pass re-benches or re-specifies them, (b) carry the **three**
open ends to their owners, and (c) prove — via the harness that already exists — that the evidence is
sufficient to close them. Three sub-verdicts, each falsifiable:

1. **Two fixes SHIP AS-IS (already landed on branch, evidence-backed).** The `budget.rs` atomic-CAS
   rewrite (2.0× @1t, never loses) and the `token_bucket.rs` clock-hoist (+6–18%) are behaviour-
   preserving, carry a new degrade-closed falsifier, and pass `cargo test --lib` (637 green on the
   branch). Their only open action is the **merge/push** of `perf/contention-bench-2026-07-18`
   (OD-2/W3-2). Until merged, dowiz `main` still runs the slower `Mutex` code (verified this pass:
   `budget.rs:133`, `token_bucket.rs:29`) **and** lacks the contended benches S1 P80-C1 calls for.

2. **GCRA does NOT ship without an operator ruling (OD-1/W3-1).** The lock-free GCRA rewrite of
   `token_bucket` measures 1.3–3.6× (3.66× @8t) but is an **algorithm swap on a DoS/rate-limit
   security primitive**, and the realistic dispatch path (one `try_acquire` then a long LLM call) has
   low real contention (@1t the gap is only 1.29×). Per never-bypass-human-gates: **default if unruled
   = NOT shipped; the Mutex + clock-hoist stands.** The bench stands ready as evidence either way.

3. **The bebop bus fix is release-blocked by C3, NOT by P90 (OD-3/W3-3).** The bus `G-C1` fix is
   *done and verified* (bebop lib 443 green; re-entrancy-no-deadlock + order-preservation tests) but
   could not be committed: bebop HEAD (`986646a`) sits in a **pre-existing C3 HARD-law-red state**
   (`ci-no-ungated-keygen.sh` fails on ungated constant-seed keygen, unrelated to this change) and
   `--no-verify` was correctly denied by the environment. It is preserved as an applyable patch
   (`bebop-bus-G-C1-fix.patch`). **P76's blueprint must ABSORB the patch, not re-implement it** —
   P90 only records that landing is C3-gated, not effort-gated.

**Non-goal (anti-scope):** P90 does not re-specify the contended benches for P80, does not re-derive
the E10/E12 rejections, and does not re-implement the bus fix for P76. It cites them.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." The measured *numbers* below are quoted from the branch-only
> results doc (they cannot be re-derived without the worktree — that is the §6 verification step, not a
> claim made here). The *current-main code state* the numbers act on **was read from source this pass**.

### 0.1 What main runs today (the "before", read this pass)

| Site | Current-main state | Cite (verified this pass) |
|---|---|---|
| `budget.rs` compute-budget port | `budget: Mutex<ComputeBudget>` wrapping `spent: f64` / `ceiling: f64`; `BudgetedJobPort` "wraps an instance in a `Mutex` for the threaded port surface" | `kernel/src/budget.rs:133` (`Mutex<ComputeBudget>`), `:85-88`, `:19` (`use std::sync::Mutex`) |
| `token_bucket.rs` rate-limiter | `inner: Mutex<Inner>` holding `tokens: f64` + `last_refill: Instant`; the lazy refill reads `Instant::now()` **inside** the locked section | `kernel/src/token_bucket.rs:29` (`Mutex<Inner>`), `:17-18`, `:49` (`let now = Instant::now();`), header `:12-14` |

Both are the exact `Mutex` sites S1 A1/A2 flagged and E12 ruled GATED-bench-first. The fixes exist
**only** on the branch; main is unchanged. This is the concrete cost of the un-merged branch (§5.2).

### 0.2 Where the fixes, benches, and evidence actually live (push/merge state, verified this pass)

| Artifact | Location | State |
|---|---|---|
| Budget CAS fix + token_bucket clock-hoist + GCRA bench + non-finding benches | worktree `/root/dowiz-perf-contention`, local branch `perf/contention-bench-2026-07-18`, commit `8c865805b` | **unpushed, unmerged** |
| `docs/research/OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md` (the results doc) | same branch, commit `8256dbffb` | **unpushed** (exists nowhere on main) |
| `kernel/benches/contention.rs` (new `[[bench]]`, N∈{1,2,4,8} threads on ONE shared object) | same branch | **unpushed** |
| `bebop-bus-G-C1-fix.patch` (the bus fix, verified-not-committed) | file in that worktree's `docs/research/` | **uncommittable** in bebop-repo until C3 resolves |
| dowiz `origin/main` | remote | at `4b30c9b4c`; the entire local main line above it (P57–P74 wave + `a857cd71a` slot_arena) is also unpushed (OD-4, out of P90 scope) |

Per the worktree/remote-push precedent (confirmed data loss, 2026-07-18): **worktree dirs are
disposable, a pushed branch is truth** — the merge/push decision (§5.1) is time-sensitive.

### 0.3 The measured numbers (branch-only, `8c865805b`; kernel `cargo test --lib` = 637 green on branch)

| Site | Decision | Measured numbers (from the branch results doc) |
|---|---|---|
| `budget.rs` (S1 A2/E12) | **FIXED** — `Mutex<f64>` → lock-free `AtomicU64` CAS; the ceiling is **re-checked every retry** (degrade-closed preserved); new falsifier `budget_atomic_never_over_grants` (8 threads racing for exactly-ceiling grants) | **2.0× @1t, 1.28× @2–4t, tie @8t** — no thread count where the `Mutex` wins |
| `token_bucket.rs` (S1 A1/E12) | **PARTIAL FIX** — monotonic clock read hoisted **outside** the lock; same algorithm, same over-grant invariant, **zero test change** | **+6–18% under contention** |
| GCRA lock-free rewrite of `token_bucket` | **BENCHED, NOT SHIPPED — OPERATOR-GATED** (`contended_token_bucket/gcra_atomic`) | **1.3–3.6× (3.66× @8t)** — but @1t only **1.29×**; algorithm swap on a DoS/rate-limit security primitive |
| admission seen-set (S1 A3) + bebop `hybrid_gate` seen-set (S1 E10) | **NO ACTION — contention proven negligible** | raw lock is contended (sharded ~9× faster @8t) but **with realistic per-frame crypto (~3µs stand-in, itself 10–50× cheaper than a real verify) before the O(1) insert, mutex and 16-shard set CONVERGE at every thread count** — E10/E12's caution vindicated by measurement |
| bebop bus publish (S1 A3-Tier-A / P76 G-C1) | **FIXED + VERIFIED, COMMIT-BLOCKED** — snapshot-`Arc`-handles-under-lock, dispatch **outside** the lock; re-entrancy-no-deadlock + order-preservation tests; **bebop lib 443 green** | commit denied: bebop HEAD `986646a` fails `ci-no-ungated-keygen.sh` (pre-existing C3, unrelated); preserved as `bebop-bus-G-C1-fix.patch` |

**Honesty note (carried verbatim from the source, not softened):** the two "NO ACTION" rows are
*load-bearing negative results*, not gaps — they are the measured proof that E10/E12's "don't shard on
a raw-lock microbenchmark, measure the realistic path" caution was correct. Recording them is a
deliverable, exactly as in S1 §6.

### 0.4 Primitives this uses are all in `std` / already in-tree — zero new deps (standard §2 item 19)

| Need | Primitive | Note |
|---|---|---|
| lock-free budget counter | `std::sync::atomic::AtomicU64` + `compare_exchange_weak` | `f64` ceiling encoded via `to_bits`/`from_bits` (bit-exact, no precision change) |
| monotonic clock | `std::time::Instant` (already used, `token_bucket.rs:18`) | only the *read site* moves; the clock source is unchanged |
| GCRA (if ever ruled in) | single-`AtomicU64` theoretical-arrival-time (TAT) CAS loop | textbook GCRA; no crate |
| contended bench harness | `std::thread` + a shared `Arc<T>`, N∈{1,2,4,8} | `kernel/benches/contention.rs`, new `[[bench]]` — the substrate P80-C1 asked for |

No dependency is added and no primitive is invented. The atomic pattern is the same one already used
across the kernel's degrade-closed surfaces.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P90 uses it — and what it does NOT take |
|---|---|---|
| **Lock-free counter via CAS loop** (Treiber/`AtomicU64` compare-exchange) | replace a `Mutex`-guarded scalar with an atomic + retry loop; each retry re-reads and re-validates | **Adopt** for `budget.rs`: the ceiling is re-checked on *every* CAS retry, so an over-ceiling grant is unrepresentable even under a race (§4.1, falsifier). **NOT taken:** lock-free everywhere — the seen-sets stayed `Mutex` because measurement said sharding buys nothing on the realistic path (§4.4). |
| **Critical-section minimisation / clock-hoist** | do expensive/blocking reads (syscalls, clock) *outside* the lock; hold the lock only for the state mutation | **Adopt verbatim** for `token_bucket.rs`: `Instant::now()` moves out of the locked refill; the lock now guards only the `tokens`/`last_refill` update. Same algorithm, same invariant. **NOT taken:** removing the lock entirely — that is the GCRA path, which is a *different, gated* decision. |
| **GCRA (Generic Cell Rate Algorithm)** — single-timestamp (TAT) rate limiter | one atomic "theoretical arrival time"; a CAS decides admit/reject with no held lock | **Benched, held for a ruling.** GCRA is the correct lock-free rate-limiter shape, but swapping it in is an *algorithm change on a security primitive*; P90 keeps the evidence and defers the swap to the operator (§5, OD-1). **NOT taken by default.** |
| **Contended microbenchmark methodology (N-thread hammer on one shared object)** | measure at N∈{1,2,4,8} threads sharing one object, with and without a realistic per-op cost stand-in | **Adopt** as the harness (`kernel/benches/contention.rs`). The realistic-cost stand-in (the ~3µs crypto proxy before the seen-set insert) is what turned A3/E10 from "9× sharded win" into "converges — no action" — the method is the finding. |

---

## 2. Scope — what P90 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P90 OWNS

1. **Registration of the measured results** (§0.3) as roadmap truth — the ledger deltas in §5.3, so no
   future pass re-benches or re-litigates E10/E12.
2. **Carrying the three open ends to their owners** (§5): the GCRA ruling (OD-1), the branch merge/push
   (OD-2), and the C3-resolution precondition for the bus patch (OD-3).
3. **A verification plan** (§6) that either confirms the existing bench evidence is sufficient to close
   each open end, or names precisely what is still missing.
4. **The cross-reference wiring** so P76 absorbs the bus patch and P80 treats the contended benches as
   already-done — preventing duplicated work.

### 2.2 P90 does NOT own (anti-scope — prevents collision & re-work)

- **Re-specifying the contended benches** — they exist on-branch (`kernel/benches/contention.rs`).
  **P80's blueprint must cross-reference them as landed, NOT re-write the `[[bench]]`** (S1 P80-C1's
  contended-lock sub-item is satisfied *by the merge*).
- **Re-implementing the bebop bus fix** — it is done + verified as `bebop-bus-G-C1-fix.patch`.
  **P76's blueprint must ABSORB the patch, not re-code it** (S3 §2 open-end 3).
- **Resolving the C3 ungated-keygen red state** — that is an operator/council-gated crypto item
  (OD-3/W3-3) owned by the P85/P76 bebop lane; P90 only names it as the precondition to landing the
  bus patch.
- **The GCRA swap itself** — held for OD-1; P90 owns the *ruling-carrying*, not the swap.
- **The wider unpushed-main push decision (OD-4)** — pushing `a857cd71a` and the P57–P74 wave is a
  general repo-safety action, tracked in the master ledger, not P90's unit.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree / on branch):** `kernel/src/budget.rs`, `kernel/src/token_bucket.rs`
(the fix targets); `kernel/benches/contention.rs` (the harness, on-branch); the results doc
(branch-only); `bebop-bus-G-C1-fix.patch` (worktree file). **Consumers:** any caller of the
`BudgetedJobPort` threaded surface and the `TokenBucket` rate-limiter — semantics are **identical**
before/after (behaviour-preserving fixes), so no consumer changes. **Blocks:** nothing — P76's bus
half is release-blocked by C3, not by P90.

### 2.4 Honest reconciliation with the E10/E12 rejections (standard §2 item 6)

The E10/E12 verdict ("GATED-bench-first; do not shard on a raw-lock microbenchmark") is **not
overturned — it is exercised and *confirmed*.** P90 records: E12 → benches now exist, `budget`
shipped on-branch, `token_bucket` partial-shipped, GCRA operator-gated. E10 (`HybridGate.seen`) →
upgraded from "no evidence" to "measured negligible." The rejections were right; the measurement
proved it.

---

## 3. Predefined types & constants — named for the record (standard §2 item 4)

These types/consts **already exist on the branch** — naming them here makes the blueprint executable
by a reviewer with zero session context and pins the falsifier names into the DoD (§9).

```rust
// kernel/src/budget.rs  (as landed on perf/contention-bench-2026-07-18)

/// Lock-free compute-budget counter. Replaces `Mutex<ComputeBudget>` on the threaded port surface.
/// `spent` is an f64 encoded into an AtomicU64 via to_bits/from_bits (bit-exact — NOT a precision
/// change). Every debit is a CAS loop; the ceiling is re-checked on each retry so an over-ceiling
/// grant is unrepresentable under a race (degrade-closed preserved).
pub struct AtomicBudget {
    spent_bits: AtomicU64,   // f64::to_bits(spent)
    ceiling: f64,            // immutable after construction
}
// debit(amount) -> bool:
//   loop { let cur = f64::from_bits(spent_bits.load(Acquire));
//          let next = cur + amount;
//          if next > ceiling { return false; }              // degrade-closed: refuse over-ceiling
//          if spent_bits.compare_exchange_weak(cur.to_bits(), next.to_bits(), AcqRel, Acquire).is_ok()
//             { return true; } }                            // retry re-reads AND re-checks ceiling

// kernel/src/token_bucket.rs  (partial fix as landed)
//   the ONLY change: `let now = Instant::now();` is computed BEFORE `inner.lock()`, then passed in;
//   the lock guards solely the `tokens`/`last_refill` mutation. Same algorithm, same invariant.

// kernel/benches/contention.rs  (new [[bench]])
//   contended_budget/{mutex,atomic}          @ N ∈ {1,2,4,8}
//   contended_token_bucket/{mutex,hoist,gcra_atomic}  @ N ∈ {1,2,4,8}
//   contended_seen_set/{mutex,sharded16} WITH ~3µs crypto stand-in  @ N ∈ {1,2,4,8}
```

**Falsifier tests (already on branch — these are the DoD spine, §9):**
`budget_atomic_never_over_grants` (8 threads, exactly-ceiling grants, asserts total granted ≤ ceiling);
the existing `token_bucket` over-grant tests (**unchanged** — proving the hoist is behaviour-preserving).

**Constants:** none new are introduced by the shipped fixes. GCRA, *if* ruled in, would add a single
`emission_interval` / `burst_tolerance` pair — named in §5.1, not defined here (it does not ship by
default).

---

## 4. The landed work — spec → the RED falsifier that exists → measured GREEN (standard §2 items 2,3,5)

Each item is stated as it *actually landed on the branch*. The RED test named for each already exists
and already passes on-branch; P90's job is to confirm reproducibility (§6), not to author them.

### 4.1 Budget CAS — FIXED (ships as-is)

- **Spec:** the threaded compute-budget must (i) never grant past its ceiling even under a race, and
  (ii) not serialise unrelated debits behind a mutex. Replace `Mutex<f64>` with an `AtomicU64`-encoded
  `f64` and a CAS loop that re-checks the ceiling on every retry.
- **Falsifier (RED before, GREEN after):** `budget_atomic_never_over_grants` — 8 threads race to debit
  toward an exactly-ceiling total; asserts the sum of *successful* grants never exceeds the ceiling.
  This is the degrade-closed invariant made machine-checkable (item 6).
- **Measured:** 2.0× @1t, 1.28× @2–4t, tie @8t — **no regime where the `Mutex` wins**, so it is a
  strict improvement with no downside contention profile.
- **Behaviour preservation:** the ceiling semantics are identical; only the concurrency mechanism
  changed. The `f64`→`AtomicU64` encoding is `to_bits`/`from_bits` — bit-exact, not a numeric change.

### 4.2 token_bucket clock-hoist — PARTIAL FIX (ships as-is)

- **Spec:** hold the lock only for the `tokens`/`last_refill` mutation; read the monotonic clock
  outside it. The rate-limit algorithm and the over-grant invariant are untouched.
- **Test:** **zero test change** — the existing over-grant tests stay green, which is itself the proof
  that the hoist is behaviour-preserving (a changed test would signal a changed invariant).
- **Measured:** +6–18% under contention. Called "partial" because the lock still exists (the full
  lock-removal is the gated GCRA path, §4.3).

### 4.3 GCRA lock-free rewrite — BENCHED, NOT SHIPPED (operator-gated, OD-1)

- **Spec (held, not applied):** replace the whole `Mutex<Inner>` token bucket with a single-`AtomicU64`
  GCRA TAT and a CAS admit/reject — no held lock at all.
- **Bench:** `contended_token_bucket/gcra_atomic` — 1.3–3.6× (3.66× @8t), but **@1t only 1.29×**.
- **Why it does not ship by default:** it is an *algorithm swap on a DoS/rate-limit **security**
  primitive*, and the realistic dispatch path (one `try_acquire` then a long LLM call) has low real
  contention — the 8-way hammer that produces 3.6× is not representative of production. Shipping a
  security-primitive algorithm change for a non-representative win is an operator decision
  (never-bypass-human-gates). **Default if unruled: NOT shipped.** The bench stands as the evidence
  for either ruling.

### 4.4 Seen-sets — NO ACTION (non-finding, recorded permanently)

- **admission seen-set (A3) + bebop `hybrid_gate` seen-set (E10):** the raw lock IS contended
  (16-shard ~9× faster @8t on a bare microbenchmark), **but** with a realistic ~3µs per-frame crypto
  stand-in before the O(1) insert, the mutex and the 16-shard set **converge at every thread count** —
  the crypto dominates, the lock is off the critical path. **Action: none.** This is the measured
  vindication of E10/E12's "measure the realistic path" caution and is recorded so it is never
  re-scanned without new evidence (a real per-frame cost <<3µs would be the reopening trigger).

### 4.5 bebop bus publish — FIXED + VERIFIED, COMMIT-BLOCKED (belongs to P76)

- **Spec / fix:** snapshot the subscriber `Arc` handles *under* the lock, then dispatch *outside* it —
  removes the publish-under-lock re-entrancy/deadlock hazard while preserving delivery order.
- **Tests:** re-entrancy-no-deadlock + order-preservation; **bebop lib 443 green**.
- **Why it is not committed:** bebop HEAD `986646a` fails `ci-no-ungated-keygen.sh` (pre-existing C3,
  unrelated to the bus change — a clean worktree at HEAD with zero crypto edits still trips it), and
  `--no-verify` was correctly denied by the environment's permission classifier. Preserved as
  `bebop-bus-G-C1-fix.patch`. **P76 absorbs this patch; P90 only records that landing is C3-gated.**

---

## 5. The three open ends — carried in full (standard §2 item 18)

### 5.1 OD-1 / W3-1 — the GCRA ruling (operator)

**Decision:** ship the lock-free GCRA rewrite of `token_bucket`, or keep the Mutex + clock-hoist.
**Evidence in hand:** `contended_token_bucket/gcra_atomic` = 1.3–3.6× (3.66× @8t), 1.29× @1t.
**The tension, stated honestly:** the win is real but only under a non-representative 8-way hammer; it
is an algorithm change on a security primitive; the realistic dispatch path is near-uncontended.
**If ruled IN:** the swap adds a single `emission_interval`/`burst_tolerance` pair and must carry its
own over-grant + burst-tolerance falsifiers (rate-limiting correctness is a security property — a
GCRA off-by-one is a rate-limit bypass) **and** go through the §8 independent review, because it is a
new algorithm on a DoS primitive. **If ruled OUT (default):** the Mutex + clock-hoist stands, the
bench is retained as the record. **Nothing about this ruling blocks §4.1/§4.2 shipping.**

### 5.2 OD-2 / W3-2 — push/merge `perf/contention-bench-2026-07-18` (operator)

The budget CAS fix, the token_bucket clock-hoist, the contention bench harness, and the results doc
**exist only on that local branch.** Until merged: (a) dowiz `main` runs the slower `Mutex` code
(`budget.rs:133`, `token_bucket.rs:29`, verified this pass); (b) `main` lacks the contended benches
S1 P80-C1 calls for; (c) the work is one worktree-deletion from loss (the 2026-07-18 precedent).
**Merging satisfies P80's contended-bench sub-item as already-done** — the P80 writing pass must
then NOT re-specify those benches. **Default if unruled:** stays local — but that is *against* the
push-after-milestone precedent and should be ruled promptly.

### 5.3 OD-3 / W3-3 — resolve C3 before the bus patch lands (operator/council)

The C3 red state (ungated `pq_dsa`/`pq_kem` constant-seed keygen — open operator/council-gated crypto
work, **predating this wave**) freezes ALL hook-respecting commits on the bebop branch. Until it is
resolved (or an explicit operator `--no-verify` ruling for the bus patch is recorded), P76's bus half
stays a patch file. **Concrete, non-hypothetical cost:** the bus fix (443 green) could not land purely
because of C3. This is also P85's named precondition — resolving C3 is a *shared* freeze-breaker for
the whole bebop lane (master ledger §3), not a P90-local item.

### 5.4 Ledger deltas this registers (cite, don't re-derive — standard §2 item 7)

- S1 **E12** → exercised: benches now exist; budget shipped (on-branch), token_bucket partial-shipped,
  GCRA operator-gated.
- S1 **E10** (`HybridGate.seen`) → upgraded from "no evidence" to "measured negligible."
- S2 §0 **R17 fold-in obligation → DISCHARGED** (feeds P88's CPU-domain boundary exactly as S2 §4.4
  anticipated: the data moves specific CPU sites; the GPU-domain atomicity default is untouched).
- **P80** contended-lock sub-item → satisfied by the OD-2 merge (do not re-specify).
- **P76** bus half → absorb `bebop-bus-G-C1-fix.patch` (do not re-implement).

---

## 6. Verification plan — is the existing evidence sufficient? (standard §2 items 2, 10)

The task question: *confirm the existing benchmark evidence is sufficient, or name what is still
needed.* Answer, per open end:

| Open end | Is the evidence sufficient to close it? | Verification step (what a reviewer runs) |
|---|---|---|
| §4.1 budget CAS ships | **Yes.** A strict win at every N + a degrade-closed falsifier is complete evidence for a behaviour-preserving fix. | In the worktree: `cargo bench --bench contention -- contended_budget` (confirm 2.0×/1.28×/tie reproduces on the reviewer's hardware, ±noise) and `cargo test --lib budget_atomic_never_over_grants` (GREEN). |
| §4.2 clock-hoist ships | **Yes.** +6–18% with **zero test change** is the strongest behaviour-preservation evidence there is. | `cargo bench --bench contention -- contended_token_bucket/hoist`; `cargo test --lib` (637 green, unchanged over-grant tests). |
| §4.3 GCRA ruling | **Evidence sufficient to *decide*; NOT sufficient to *ship without review*.** The 1.3–3.6× vs 1.29×@1t data is enough for the operator to rule. If ruled IN, new falsifiers + §8 review are still required. | Present the `gcra_atomic` bench numbers alongside the 1t figure to the operator; if IN, author GCRA over-grant/burst falsifiers RED-first and route §8. |
| §4.4 seen-set non-finding | **Yes — the non-finding IS the result.** Convergence-with-realistic-cost closes it permanently. | `cargo bench --bench contention -- contended_seen_set` (confirm mutex ≈ sharded with the ~3µs stand-in). Reopening trigger only if a real per-frame cost <<3µs is measured. |
| §5.2 merge | Evidence complete; blocked on a *ruling*, not data. | Operator ruling. |
| §5.3 bus patch | Fix verified (443 green); blocked on C3 resolution, not on P90 data. | `git apply --check bebop-bus-G-C1-fix.patch` after C3 clears (belongs to P76). |

**Gaps that remain (named, per the task):**
1. **Cross-hardware reproduction.** All numbers are single-machine. The `budget`/`token_bucket` fixes
   are strict-win/behaviour-preserving so this is a *confidence* check, not a blocker. For the GCRA
   *ruling*, a second-machine run is worth having because the decision hinges on a contention profile
   that varies by core count/topology — flagged, not required.
2. **P75's bench-regression gate is not yet wired** (P75 is SKETCH-ONLY). Once P75 lands, these benches
   should register into its bench-id/baseline schema so a future regression surfaces automatically
   (item 14). Until then, the benches are run-on-demand, not gated. This is a P75 dependency, not a
   P90 gap.
3. **GCRA falsifiers do not yet exist** (correctly — the swap does not ship by default). They are only
   *needed* if OD-1 rules IN.

**Verdict of §6:** for the two shipping fixes and the two non-findings, the existing evidence is
**sufficient** — P90 needs no new measurement to close them, only the OD-2 merge. For GCRA, the
evidence is sufficient to *rule* but a ruled-in swap owes new falsifiers + review. For the bus patch,
the blocker is C3, not evidence.

---

## 7. Adversarial self-check — real effort to break the claims (standard §2 items 3, 5)

### 7.1 Can the atomic budget over-grant under a race?
The whole point of the falsifier `budget_atomic_never_over_grants`: 8 threads race for exactly-ceiling
grants; the CAS re-reads *and re-checks the ceiling* on every retry, so a thread that lost a race
recomputes against the winner's new `spent`. An over-ceiling grant is **unrepresentable** — the CAS
either sees the up-to-date `spent` (and refuses) or fails and retries. The classic bug (check-then-act
race) cannot occur because the check is *inside* the CAS retry, not before it. If a reviewer builds a
counterexample thread schedule that over-grants, the fix is RED and does not ship — that is the gate.

### 7.2 Does the clock-hoist change the over-grant invariant?
It must not — and the proof is that **the over-grant tests are unchanged and green**. The refill math
is a pure function of `(now, last_refill, rate, capacity)`; moving *where `now` is read* cannot change
the result as long as the mutated state (`tokens`, `last_refill`) is still updated atomically under the
lock. The one subtle hazard — reading `now` far before acquiring the lock, so a stale `now` under-
refills — is bounded (monotonic clock, sub-microsecond hoist distance) and is *conservative* (under-
refill = stricter rate limit = fail-safe for a DoS primitive), never an over-grant.

### 7.3 Is the GCRA bench representative? (the reason it is gated)
**No — and P90 says so.** 3.66× @8t on a hammer that shares one bucket across 8 hot threads is not the
dispatch path, where one `try_acquire` precedes a multi-second LLM call (@1t = 1.29×). Presenting the
8-way number as the headline would be the misleading move; P90 presents *both* and lets the operator
rule. This is the honest-benchmark discipline (report the representative case, not the flattering one).

### 7.4 Could the "NO ACTION" seen-set decision be wrong?
Only if the ~3µs crypto stand-in over-states the real per-frame cost. The stand-in is deliberately
**10–50× cheaper** than a real ML-DSA verify — so it *under*-states the crypto dominance, making the
"lock is off the critical path" conclusion *conservative*. If the real cost were lower than 3µs (e.g. a
future symmetric fast-path — see P92), the decision reopens; that trigger is recorded (§4.4).

### 7.5 What if the branch is never merged?
Then main permanently runs the slower `Mutex` code and lacks the contended benches — and the work is
one worktree-deletion from loss (the 2026-07-18 data-loss precedent is why OD-2 is time-sensitive).
This is the failure mode §5.2 exists to prevent; the mitigation is *rule OD-2 promptly*, not "leave it
local indefinitely."

---

## 8. Review / landing gate (standard §2 items 5, 6)

The shipping fixes (§4.1/§4.2) are **behaviour-preserving with falsifiers** — they do not require the
full independent-adversarial-crypto review that P92/P85 demand, because they change concurrency
mechanism, not a security algorithm or a signature path. Their gate is: falsifier GREEN + strict-win
bench reproduced + `cargo test --lib` 637 green + the OD-2 merge ruling.

**GCRA is the exception.** If OD-1 rules it IN, it becomes a *new algorithm on a DoS/rate-limit
security primitive* and inherits the crypto-adjacent review discipline: RED-first over-grant + burst-
tolerance falsifiers, and an independent reviewer whose mandate is to **produce a rate-limit bypass or
prove its impossibility**, not read-and-approve (the B4/SSR-2020 lesson — unit-green is necessary, not
sufficient, on a security primitive).

**The bus patch** carries its own review inside P76; P90 does not re-gate it, it only records that
landing is C3-blocked.

---

## 9. DoD — falsifiable, machine-checkable where the artifact is code (standard §2 item 2)

| # | Done when… | Falsifier / check |
|---|---|---|
| D1 | the budget CAS fix's degrade-closed invariant is proven under contention | `budget_atomic_never_over_grants` GREEN (8-thread, ≤ ceiling) |
| D2 | the budget CAS fix is a strict win (no regime where Mutex wins) | `contended_budget/{mutex,atomic}` bench: atomic ≥ mutex at N∈{1,2,4,8} |
| D3 | the clock-hoist is behaviour-preserving | `token_bucket` over-grant tests **unchanged + GREEN**; `contended_token_bucket/hoist` shows +6–18% |
| D4 | the seen-set non-finding is recorded as permanent (E10 upgraded) | `contended_seen_set` bench: mutex ≈ sharded WITH the ~3µs stand-in — the ledger delta (§5.4) filed |
| D5 | the GCRA ruling (OD-1) is **recorded either way** | a written ruling exists; if IN, GCRA falsifiers RED-first + §8 review attested; if OUT, the bench is retained as record |
| D6 | the branch merge/push (OD-2) is **ruled either way** | branch merged to main, OR an explicit no-merge ruling recorded with a named re-review date |
| D7 | P80/P76 cross-references are wired so neither re-does landed work | P80 blueprint cites `kernel/benches/contention.rs` as landed; P76 blueprint cites `bebop-bus-G-C1-fix.patch` as absorb-not-reimplement |
| D8 | the C3 precondition (OD-3) is **named with an owner** | the bus-patch-blocked-on-C3 fact recorded in P76/P85's blueprint with C3 as the named freeze-breaker |
| D-NOREG | merging the branch does not regress the kernel suite | `cargo test --lib` = 637 green on the merge result (matches the branch) |

**DoD nature (honest):** unlike a build blueprint, P90's DoD is **rulings-recorded + branch-merged +
cross-refs-wired**, not "new code green." D1–D4/D-NOREG are the code checks (already GREEN on-branch —
verification is reproduction, §6); D5–D8 are the decision/registration items that are the *actual*
remaining work.

---

## 10. Benchmarks + telemetry + the measure-first posture (standard §2 item 10)

### 10.1 The benches that exist (on-branch)
`kernel/benches/contention.rs`, new `[[bench]]`, N∈{1,2,4,8} threads on ONE shared object:
`contended_budget/{mutex,atomic}`, `contended_token_bucket/{mutex,hoist,gcra_atomic}`,
`contended_seen_set/{mutex,sharded16}` with the ~3µs crypto stand-in. This harness **is** the
contended-lock coverage S1 P80-C1 asked for — P80 cross-references it (§5.4), does not re-author it.

### 10.2 Telemetry
Once P75's bench-regression gate lands, these benches register into its bench-id/baseline schema so a
future regression (e.g. a refactor that re-introduces a hot lock) surfaces automatically, not at review
time (item 14). Until P75, they are run-on-demand — a named P75 dependency (§6 gap 2), not a P90 gap.

### 10.3 Measure-first posture (already honoured)
P90 is the *product* of a measure-first pass: E12 ruled the sites GATED-bench-first, the benches ran,
and action was taken **only where numbers justified it** (budget/token_bucket shipped; seen-sets did
not; GCRA held for a ruling). There is no measure-first *gate to run* here — the measurement already
happened; §6 is its reproduction check.

---

## 11. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16)

- **Hazard-safety as math (item 6):** the unsafe state (an over-ceiling grant / over-refill under a
  race) is made **unrepresentable**: the budget CAS re-checks the ceiling inside every retry (§7.1);
  the clock-hoist can only ever *under*-refill (fail-safe for a DoS primitive, §7.2). Argued from the
  CAS/lock structure, not a prose assurance.
- **Schemas & scaling axis (item 8):** scaling axis = **concurrent debits/acquires per second per
  shared object** and **thread count N**. `AtomicBudget` is O(1) state, contention-free at low N,
  CAS-retry-bounded at high N (measured tie @8t — no pathological retry storm). The shape changes only
  if a single budget object is hammered by ≳10² hot threads (not the dispatch profile); stated, not
  timeless.
- **Isolation / bulkhead (item 11):** each fix is *local to one primitive* — `budget.rs` and
  `token_bucket.rs` are independent, and a bug in one cannot affect the other. The degrade-closed
  refusal (budget) and stricter-rate-limit (token_bucket under-refill) are the bulkhead failure modes:
  both fail *closed* (deny), never open (over-grant). The seen-sets were left untouched precisely to
  avoid a change with no measured benefit (blast-radius minimisation).
- **Mesh awareness (item 12):** **node-local, honestly.** `budget.rs`/`token_bucket.rs` are per-node
  primitives; nothing here gossips or touches the transport. The bebop bus fix is intra-process
  pub/sub, not mesh. No payload/frequency budget applies. Stated, not stretched.
- **Rollback / self-healing as math (item 13):** **Self-termination** = the budget/rate-limit invariant
  boundary — a bad grant is unrepresentable past the ceiling, not a supervisor's choice. **Snapshot
  re-entry / self-healing = NOT claimed** — these are stateless-per-op primitives; a failed CAS simply
  retries against fresh state (bounded), which is not error-correcting recovery. Claiming self-healing
  here would be false.
- **Error-propagation / smart index (item 14):** the bug class this could introduce (a concurrency
  regression that re-serialises or over-grants) is caught by `budget_atomic_never_over_grants` (compile-
  and-test time) and — once P75 lands — by the bench-regression gate (a lock re-introduced on the hot
  path shows as a bench regression, not a runtime surprise).
- **Living-memory awareness (item 15):** **N/A, honestly** — budget/rate-limit counters are transient
  per-op state, deliberately not persisted; they are the opposite of living memory. Stated.
- **Tensor/spectral (item 16):** **N/A, honestly** — a CAS loop and a clock-hoist are not linear
  algebra; forcing `spectral.rs` here would be over-engineering (ponytail). Stated.
- **Linux discipline (item 9):** **REINFORCES** the existing degrade-closed lock patterns
  (`bounded_drainer.rs`/`budget.rs`) with a lock-free variant that keeps the same fail-closed semantics;
  **ALREADY-EQUIVALENT** on the critical-section-minimisation idiom (the clock-hoist *is* that idiom);
  **DOES-NOT-TRANSFER** — no new subsystem, no new concurrency primitive beyond `std::sync::atomic`.

---

## 12. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Polarity / no-middle:** a grant is either *within ceiling* or *refused* — there is no partial/over-
  ceiling middle state (the CAS admits a boolean, never a degraded grant). A rate token is *available or
  not*; the hoist does not introduce a "maybe."
- **Cause & Effect:** every atomic transition has a single well-ordered cause (the winning CAS); a lost
  race is *caused* to retry against the up-to-date state — no effect (grant) exists without a
  ceiling-checked cause.
- **Rhythm (measure-first):** action was taken in rhythm with the evidence — where the numbers justified
  it (budget/token_bucket) and *not* where they did not (seen-sets), with the swing held for a ruling
  (GCRA). The decision follows the measurement, not the instinct.

---

## 13. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (main `Mutex` state read this pass: `budget.rs:133`, `token_bucket.rs:29`; numbers quoted as branch-only, reproduced in §6) |
| 2 | Falsifiable DoD | §9 (D1–D-NOREG; D1–D4 code checks, D5–D8 rulings-recorded) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per site; the falsifier exists before the shipped state is accepted) |
| 4 | Predefined types & constants | §3 (`AtomicBudget`, the bench names, the falsifier names) |
| 5 | Adversarial/breaking tests | §4.1 falsifier, §7 (self-attack on over-grant/hoist/bench-representativeness), §8 (GCRA review-if-ruled-in) |
| 6 | Hazard-safety from structure | §11 (over-grant unrepresentable via CAS ceiling re-check; hoist under-refill is fail-safe) |
| 7 | Links to docs & memory | §14 |
| 8 | Schemas with scaling axis | §11 (debits/sec, thread N; retry-bounded) |
| 9 | Linux engineering discipline | §11 (REINFORCES/ALREADY-EQUIVALENT/DOES-NOT-TRANSFER verdict) |
| 10 | Benchmarks + telemetry | §10 (the on-branch harness; P75-gate registration; measure-first already honoured) |
| 11 | Isolation / bulkhead | §11 (per-primitive locality; both fail closed) |
| 12 | Mesh awareness | §11 (node-local, stated honestly) |
| 13 | Rollback/self-heal as math | §11 (self-termination = invariant boundary; self-healing NOT claimed) |
| 14 | Error-propagation / smart index | §11 (falsifier + P75 bench-gate catch a re-introduced hot lock) |
| 15 | Living-memory awareness | §11 (N/A — transient counters, stated) |
| 16 | Tensor/spectral where applicable | §11 (N/A, stated honestly) |
| 17 | Regression tracking | §9 D1/D3 (the falsifier + unchanged over-grant tests are permanent regression guards); register into REGRESSION-LEDGER on merge |
| 18 | Clear worker instructions | §14 |
| 19 | Reuse-first, upgrade-if-needed | §0.4 (only `std::sync::atomic`), §1 (adopt CAS/hoist/GCRA idioms), §2.2 (anti-scope: don't re-bench/re-implement) |
| 20 | Hermetic principles | §12 |

---

## 14. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md` (branch-only; the measured source).
- `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` §2 (P90 scope + the three open ends), §6 (W3-1/2/3).
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.1 (A1/A2/A3), §6 (E10/E12 rejections — cite, don't
  re-derive).
- `MASTER-STATUS-LEDGER-2026-07-19.md` (I3/Contention row; OD-1/OD-2/OD-3/OD-4; §3 wave-0/1 sequence).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Memory: `worktree-remote-push-collision-avoidance-2026-07-18.md` (the OD-2 urgency),
  `performance-priority-over-minimal-change-2026-07-17.md` (why the scoped perf work is sanctioned),
  `never-bypass-human-gates-2026-06-29.md` (OD-1 GCRA ruling is operator's), `crypto-safe-first-pass-
  2026-07-14.md` (B4/SSR-2020 — the review discipline GCRA inherits *if* ruled in).

**Existing code this blueprint registers (exact targets):**
- **ALREADY-EDITED on `perf/contention-bench-2026-07-18`** — `kernel/src/budget.rs` (CAS),
  `kernel/src/token_bucket.rs` (clock-hoist), `kernel/benches/contention.rs` (new `[[bench]]`). P90
  does **not** re-edit them; it registers + verifies (§6).
- **PATCH FILE (bebop-repo, C3-blocked)** — `bebop-bus-G-C1-fix.patch`. **P76 absorbs it; do not
  re-implement.**
- **DO NOT TOUCH** — the seen-set lock sites (A3/E10): the measured non-finding says leave them
  `Mutex` (§4.4).

**For the worker/operator with zero session context — exact acceptance path:**
1. **Rule OD-2 (merge/push `perf/contention-bench-2026-07-18`).** Merging lands the two shipping fixes +
   the harness and satisfies P80's contended-bench sub-item. Default-local is *against* the push
   precedent — rule promptly.
2. **On merge, run** `cargo test --lib` (expect 637 green) and `cargo bench --bench contention` to
   reproduce the numbers on the target hardware (§6). If a fix is not a strict win / the falsifier is
   RED on the merge result, STOP — do not ship that fix.
3. **Rule OD-1 (GCRA), recording the decision either way.** If IN: author GCRA over-grant + burst
   falsifiers RED-first and route the §8 independent review before it ships. If OUT: retain the bench
   as the record; the Mutex + clock-hoist stands.
4. **Record OD-3** — the bus patch is C3-blocked, not effort-blocked. Name C3 resolution as the shared
   freeze-breaker in P76/P85's blueprint; the bus fix (`bebop-bus-G-C1-fix.patch`, 443 green) lands via
   P76 once C3 clears.
5. **Wire cross-refs (D7):** ensure P80 cites `kernel/benches/contention.rs` as landed and P76 cites
   the bus patch as absorb-not-reimplement — this is the whole point of registering P90.
6. **Register the falsifier + unchanged over-grant tests into `docs/regressions/REGRESSION-LEDGER.md`**
   on merge, so a re-introduced hot lock is caught permanently (item 17).
7. **Anti-scope:** never re-bench the seen-sets without a measured sub-3µs per-frame cost; never
   re-specify the contended benches in P80; never re-implement the bus fix in P76; never ship GCRA
   without OD-1 IN + §8 review.
