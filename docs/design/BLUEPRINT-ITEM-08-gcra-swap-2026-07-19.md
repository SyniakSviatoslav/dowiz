# BLUEPRINT — Item 8: GCRA lock-free TokenBucket swap (decision package → real swap)

**Date:** 2026-07-19 · **Status:** BLUEPRINT (no code in this commit)
**Roadmap:** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §C item 8, §0 ruling table
**Ruling (§0, recorded 2026-07-19):** *"GCRA lock-free TokenBucket swap — **ADOPT** — gated behind
the differential oracle + Kani interleaving proof already scoped in item 8; built and tested
before it ships."*
**Synthesis source:** `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §1.3 (line 63) —
differential oracle over a randomized schedule + "a loom-style or model-checked interleaving
argument (Kani can bound-check the atomic transition function — CI-time, not linked)".
**Item-6 dependency (verified on `exec/space-grade-tier0-2026-07-19`, `ae2da4a9d`):** the
hardening-gate manifest already designates this surface and blocks the swap:
`docs/audits/hardening/HOT-PATHS.tsv:23` (`@ZONE kernel/src/token_bucket.rs`) and line 38 —
gap column `MISSING(item-8):GCRA-atomic-vs-mutex-differential-oracle+item3`;
`docs/audits/hardening/CHECKLIST.md:67-68`: *"item 8's atomic-GCRA swap cannot merge without the
mutex-vs-atomic parity oracle."* This blueprint is the design that flips that row.

---

## 1. Ground truth — the REAL current `kernel/src/token_bucket.rs` (worktree, 174 lines)

### 1.1 Public API (complete — three methods, nothing else)

| Method | Signature | Line | Contract |
|---|---|---|---|
| `new` | `pub fn new(capacity: f64, refill_rate: f64) -> Self` | 44 | starts FULL (`tokens = capacity`) |
| `try_acquire` | `pub fn try_acquire(&self, n: f64) -> bool` | 92 | lazy refill, then grant iff `tokens >= n` (decrement on success); `false` ⇒ caller degrades closed. **Weighted** — `n` is a real cost, not a unit cell |
| `available` | `pub fn available(&self) -> f64` | 115 | telemetry/tests read; refills lazily first |

There is **no `release`** (the earlier-session correction stands — `TokenBucket::release` never
existed) and no other public surface.

### 1.2 Concurrency model today: **a mutex, not lock-free**

`Mutex<Inner>` over `{ tokens: f64, last_refill: Instant }` (lines 27-40). So "the swap" is a real
algorithm-and-synchronization swap, not a tune. Load-bearing properties of the incumbent, each of
which the swap must either preserve or explicitly retire:

1. **Coupled critical section** (header, lines 12-15): refill+decrement is one atomic section —
   "no lost sub-unit time, no CAS races" — this is what the over-grant invariant leans on.
2. **Monotonic clock only** (`Instant`, never wall-clock) — NTP jump cannot bypass the throttle
   (line 15).
3. **Clock read hoisted OUTSIDE the lock** (lines 17-25, 93-95): stale `now` +
   `saturating_duration_since` ⇒ smaller elapsed ⇒ conservative refill — degrade-closed under
   contention. Bench evidence: `benches/contention.rs` `mutex_clock_outside` (~15% at 8-way).
4. **A6 poison-cascade recovery** (lines 83-103): `lock().unwrap_or_else(|e| e.into_inner())` +
   chaos seam `ChaosSite::TokenBucketCritical` (line 103). Proven by
   `chaos.rs::a6_poisoned_lock_recovers_degrade_closed` (chaos.rs:699-736).
5. **The GCRA candidate already exists and is already measured**: header lines 17-25 record the
   single-`AtomicU64`-TAT GCRA at **2.5-3.6× under 2-8-way contention**, deliberately left
   OPERATOR-GATED (ref `docs/research/OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md`). The §0
   ruling resolves that gate: ADOPT, with proof.

### 1.3 Existing test coverage (the item-6 audit's "token_bucket behavioral tests")

Three in-file tests (`token_bucket.rs:123-174`), registered in the hardening manifest as the row's
`min_tests = 3` floor (HOT-PATHS.tsv:38, filter `token_bucket_`):

- `token_bucket_grants_within_capacity` (129) — 3×3.0 granted from capacity 10, 4th refused.
- `token_bucket_refills_over_time` (142) — drain, refuse, sleep ~20 ms, grant again.
- `token_bucket_never_over_grants_under_refill` (154) — **the F33 falsifier**: 5000 acquires,
  asserts `granted ≤ capacity + rate·elapsed + 1e-6`. This is the invariant the whole package
  orbits.

Plus one cross-module behavioral consumer: `chaos.rs::a6_poisoned_lock_recovers_degrade_closed`.

### 1.4 Callers (production contract the swap must not move)

- `ports/agent/admission.rs:275-293` — sharded + global admission, `try_acquire(1.0)` (this is the
  genuinely concurrent abuse-control surface — the reason GCRA is worth having).
- `agent/loop.rs:164` — per-iteration budget, `try_acquire(1.0)`.
- `bounded_drainer.rs:73` — **weighted**: `try_acquire(self.cost_per_unit)`.
- Test-pinned contract shapes (all in `#[cfg(test)]` but pinning the public semantics):
  `refill_rate = 0.0` (pure countdown budget — bounded_drainer.rs:198,208,227,242; agent/loop.rs
  483, 508), `capacity = 0.0` (loop.rs:483 — always-refuse), `capacity = f64::INFINITY`
  (loop.rs:640 — always-grant). **These degenerate parameters are in-contract.**

---

## 2. GCRA mapping design

### 2.1 Standard GCRA, stated once

GCRA (ATM Forum TM 4.0 / ITU-T I.371, virtual-scheduling form) keeps one scalar, the
**Theoretical Arrival Time (TAT)**. Parameters: **emission interval** `T = 1/rate` (here
`npt = 1e9 / refill_rate` nanos-per-token) and **burst tolerance** `τ` (here
`B = capacity · npt` nanos). Arrival at time `t` requesting weight `n` (cost `c = n·npt`):

```
allow_at = max(TAT, t)
conformant  ⇔  allow_at + c ≤ t + B          # equivalently TAT ≤ t + B − c
on grant:   TAT ← allow_at + c
on reject:  TAT unchanged
```

**Exact equivalence to the incumbent (real arithmetic):** define
`tokens(t) = (t + B − max(TAT, t)) / npt`. Then a grant of `n` drops it by exactly `n`; while
idle it rises at `rate` tokens/sec and clamps at `capacity` once `t ≥ TAT`; the conformance test
is exactly `tokens(t) ≥ n`; a fresh bucket (`TAT = 0`, `t = 0`) holds exactly `capacity`. The two
algorithms are the same fluid limiter in different coordinates — which is why a drop-in behind the
same API is legitimate, and why divergence can only come from *representation* (u64 nanos vs f64
tokens), not semantics.

### 2.2 The candidate already in-tree — and four contract hazards it does NOT yet handle

`benches/contention.rs:219-262` `GcraBucket` is the measured candidate: `tat: AtomicU64` (nanos
since a per-bucket `base: Instant`), clock read OUTSIDE the CAS, `compare_exchange_weak` retry
loop, `Relaxed` ordering. Its own doc-comment is honest: *"NOT proposed as a drop-in without an
invariant re-proof."* The re-proof surfaces four hazards (this is the real design work; all four
are exactly the arithmetic-edge fault class the synthesis assigns to Kani, §5 line 151):

| # | Hazard | Where it bites | Design decision |
|---|---|---|---|
| H1 | `refill_rate = 0.0` ⇒ `npt = ∞`, `cost = n·∞ = ∞`, `(∞) as u64` saturates to `u64::MAX`, `allow_at + cost` **overflows u64** (panic in debug, wrap in release — wrap would over-grant) | in-contract countdown buckets (§1.4) | **Two-arm representation** (§2.3): `rate == 0.0` selects a lock-free CAS *countdown* arm — no TAT at all. `contention.rs:81-113` `AtomicBudget` (f64-bit-cast CAS accumulator, ceiling re-checked on every retry) is the proven in-tree shape for this arm |
| H2 | `capacity = f64::INFINITY` ⇒ `B = ∞` ⇒ `limit = ∞` — always grant, but `TAT` still marches up and would eventually overflow | in-contract always-grant buckets | all u64 additions become `saturating_add`; conformance compare done in u64 with the saturated limit (`∞ as u64 = u64::MAX`) — always conformant, TAT pinned at MAX, no wrap |
| H3 | Cost quantization: `(n·npt) as u64` **truncates** ⇒ each grant under-charged by <1 ns ⇒ GCRA marginally *more* permissive than the real-valued limiter — wrong direction for a rate limiter | every weighted acquire | round cost **up**: `cost = ceil(n·npt)` — quantization error becomes conservative (degrade-closed), never permissive |
| H4 | `capacity = 0.0` ⇒ `B = 0` ⇒ conformant only if `c = 0` | in-contract always-refuse buckets | falls out correctly with H3 (`ceil` keeps any `n > 0` cost ≥ 1 ns > B = 0); pinned by an oracle class, no special case |

(`refill_rate = ∞` ⇒ `npt = 0` ⇒ cost 0 ⇒ always conformant — matches the incumbent's instant
refill; NaN parameters are out-of-contract for both impls and get a `debug_assert!` in `new`.)

### 2.3 The swap, concretely — **no public API change**

`new(capacity, refill_rate)`, `try_acquire(n) -> bool`, `available() -> f64` all keep their exact
signatures and documented semantics. Internally:

```rust
pub struct TokenBucket {
    capacity: f64,
    refill_rate: f64,
    repr: Repr,
    base: Instant,                       // monotonic epoch (property 2 preserved)
}
enum Repr {
    Gcra { npt: f64, burst_nanos: u64, tat: AtomicU64 },   // refill_rate > 0
    Countdown { granted_bits: AtomicU64 },                  // refill_rate == 0: f64-bit CAS accumulator
}
```

- `try_acquire(n)` = clock sample (`self.base.elapsed()`, outside the CAS — property 3 preserved:
  a stale `now` only shrinks the conformance window, conservative) → pure transition function →
  `compare_exchange_weak` loop. The transition function is extracted as a **free, pure fn**
  `gcra_decide(tat: u64, now: u64, cost: u64, burst: u64) -> Option<u64>` — this single fn is what
  Kani proves (§4) and what the oracle drives.
- `available()` = pure read: `((now + B − max(tat, now)) / npt).clamp(0.0, capacity)` (Gcra arm) /
  `(capacity − granted).max(0.0)` (Countdown arm). Improvement over today: no write, no lock.
- **Time seam (new scope, private):** the incumbent calls `Instant::now()` directly — there is
  **no time abstraction today**, so the oracle needs one. Add `pub(crate) fn try_acquire_at(&self,
  n: f64, now_nanos: u64) -> bool` (and `available_at`); the public methods are one-line wrappers
  that sample the clock. No public contract change; this is the entire new surface.
- **Mutex implementation retained forever as the test-only reference** — this is the roadmap's own
  P2 requirement verbatim (roadmap line 188: *"GCRA adopted, mutex implementation retained forever
  as test-only differential oracle — precisely P2's 'parity-pinned divergence'"*). Concretely: the
  current `refill_locked` + `try_acquire` arithmetic moves verbatim into
  `#[cfg(any(test, feature = "tb-oracle"))] struct ReferenceBucket` with the same
  `try_acquire_at` seam. **Anchor step (red→green discipline): land `ReferenceBucket` + a parity
  test against the LIVE mutex impl first, in a commit BEFORE the swap** — so the reference is
  pinned to today's behavior while today's behavior is still on `main`, not reconstructed after.
- **A6 / chaos seam:** the mutex disappears ⇒ the poison-cascade fault class disappears *by
  construction* (state mutates only via a completed CAS; a panic at the seam leaves TAT
  untouched). Keep `ChaosSite::TokenBucketCritical` (seam moves between clock read and CAS loop)
  and keep the A6 test with its rationale rewritten: it now proves "panic mid-acquire leaves a
  consistent, grantable bucket" — same assertion shape, stronger reason. `unwrap_or_else(into_inner)`
  goes away with the mutex.
- **eqc third leg (optional, not gating):** synthesis §26(d) (line 456) names the GCRA transition
  fn an "ideal third leg of the differential oracle" via `emit_fixed_rust` — scalar arithmetic in
  eqc's verified subset. Ledger as a follow-up; NOT required to close item 8.

---

## 3. Differential-oracle design (the merge-blocking artifact — HOT-PATHS.tsv:38)

**Definition:** one schedule = `(capacity, rate, [(Δt_nanos, n); N])`. Feed the IDENTICAL virtual
timestamp sequence (cumulative `Δt`, via the `try_acquire_at` seam — **no sleeps, no real clock,
deterministic and fast**) to (a) `ReferenceBucket` (today's mutex arithmetic) and (b) the GCRA
`TokenBucket`. Sequential driving is sufficient and correct here: the oracle pins *algorithmic*
equivalence; *concurrency* is §4's job. Assert per schedule:

1. **Decision parity off-boundary:** decisions must be identical for every op where the request is
   not within an ε-band of the boundary (`|reference.available_at(t) − n| > ε`, ε = 1e-6 tokens —
   the F33 test's own tolerance). **Honest limitation, stated up front:** exact all-ops equality is
   unattainable — the reference accumulates f64 rounding, GCRA quantizes to whole nanoseconds
   (H3's `ceil`); at exact boundaries the two approximations may legitimately split. Boundary-band
   ops are counted and logged, never silently skipped; the band is the assertion's published scope.
2. **Shared F33 ceiling, both impls, every schedule:** `granted ≤ capacity + rate·t_virtual + ε` —
   the invariant holds regardless of which side of a boundary either impl lands on.
3. **`available` parity:** `|ref.available_at(t) − gcra.available_at(t)| ≤ ε + rate·1e-9` at every
   step (the second term = one-nanosecond quantization).
4. **One-sided conservatism of quantization:** GCRA's granted-total ≤ reference's granted-total +
   ε on every schedule (H3's `ceil` makes GCRA the conservative side; a flip is a bug).

**Schedule generator:** stratified parameter classes × randomized ops —
`rate ∈ {0, 1e-3, 1.0, 100.0, 1e12, ∞} × capacity ∈ {0, 1.0, 10.0, 1e6, ∞}` (every §1.4
degenerate contract shape is a named class), `n ∈ {0.001 … 10.0}` including exact-boundary probes,
`Δt` mixing 0 (burst), sub-npt (sub-unit-time — the "no lost sub-unit time" property), and long
idle (clamp-at-capacity). Deterministic seeded generator, ~10⁴ schedules; follow the item-31 house
pattern (fixed corpus + seeded fuzz over the real distribution; proptest is already precedented as
a dev-dep — dev-deps sit outside the `-e no-dev` zero-dep gate surface, `Cargo.toml:177-185`).

**Oracle self-test (ct_gate planted-leak pattern, ct_gate.rs:14-19 — a gate that cannot reject
proves nothing):** two deliberately-broken GCRA variants must go RED in the same invocation:
(i) drop the `max(TAT, now)` (idle time accumulates as debt → over-grant after idle — caught by
assertion 2); (ii) `floor` instead of `ceil` on cost at `rate = 1e12` (quantization flips
permissive — caught by assertion 4).

**Wiring:** new test module `token_bucket::oracle` matched by the existing manifest filter
`token_bucket_` (or its own row); flip HOT-PATHS.tsv:38 — gap `MISSING(item-8):…` → `-`, bump
`min_tests` from 3 to the real new count, checklist column gains item-3 via a **live debug
cross-check**: in `#[cfg(debug_assertions)]` builds `TokenBucket` keeps a shadow
`(granted_total, birth)` pair and `debug_assert!`s the F33 ceiling on every grant — the falsifier
runs in every debug execution, the FSM_ADJ dual-representation house pattern. (Decision-equality
as the debug cross-check is rejected: boundary splits would make it flaky; the invariant check is
the honest form.)

---

## 4. Kani interleaving check — honest scope

**Capability statement first (this blueprint does not oversell):** Kani does **not** model
multi-threaded executions — `std::thread::spawn` and interleaving exploration are outside its
support; loom/shuttle are the tools that enumerate interleavings, and they cap out at small thread
counts. The synthesis already words this correctly (§1.3: "Kani can bound-check the atomic
transition function"). So the "interleaving check" decomposes honestly into three legs:

**Leg 1 — Kani proves the transition function, machine-checked (the named gate artifact).**
`#[cfg(kani)]` harness over the pure `gcra_decide` with **nondeterministic pre-state** — and this
is the trick that makes a sequential proof cover concurrency: `kani::any::<u64>()` for `tat`
subsumes every value any interleaving of any number of callers could have produced.

```rust
#[cfg(kani)]
#[kani::proof]
fn gcra_step_sound() {
    let (tat, now, cost, burst): (u64, u64, u64, u64) = kani::any();
    if let Some(new_tat) = gcra_decide(tat, now, cost, burst) {
        assert!(new_tat >= tat);                          // L1: TAT never regresses (no lost debit)
        assert!(new_tat >= now.saturating_add(cost));     // L2: full cost charged from `now`
        assert!(new_tat <= now.saturating_add(burst));    // L3: conformance — never beyond burst
    }
    // L4 (implicit, Kani checks every path): no overflow, no panic — for ALL 2^256 inputs.
}
```

A second tiny harness covers the Countdown arm (`spent' = spent + n`, grant ⇒ `spent' ≤ capacity`,
monotone; single-step f64 — within Kani/CBMC float support).

**Leg 2 — linearizability bridge, a short paper argument (stated in the module docs, not
hand-waved):** the entire shared state is ONE `AtomicU64`; the only mutation is a CAS whose
compare operand is the loaded value. By single-location cache coherence (which the C++11/Rust
memory model guarantees even at `Relaxed`), all successful CASes form a total modification order,
and each transition's pre-state equals its loaded value. Therefore **every concurrent history is
exactly a sequence of `gcra_decide` steps over that order** — L1-L3, proven for arbitrary
pre-state, hold at every step of every interleaving, for **unbounded** caller count (stronger than
any bounded loom exploration on this axis). The window bound then follows by a five-line
induction: grants k = 1..m in a window with clock reads `t_k ∈ [s, e]` (monotone `Instant` is
global, and a stale retry `now` is only smaller — conservative): L1+L2 give
`TAT_m ≥ t_1 + Σ c_k`; L3 at the last grant gives `TAT_m ≤ e + B`; hence
`Σ c_k ≤ (e − s) + B`, i.e. **granted ≤ rate·elapsed + capacity — F33, under any interleaving.**

**Leg 3 — what honestly stays a stress test:** the memory-ordering claim in Leg 2 (Relaxed
suffices for a single cell) is argued, not machine-checked. Mitigations, cheapest first:
(a) extend the existing F33 test to N = 8 real threads hammering one bucket, asserting the shared
ceiling — std-only, lands with the swap; (b) optional: a `loom` dev-dep harness (2-3 threads ×
2 ops) to machine-check the ordering argument — precedented as a dev-dep but **add-only-if-the-
operator-wants-it**; not required to close, ledgered in the manifest gap column if skipped.

---

## 5. Sequencing call re item 7 (Kani wiring)

**Verified facts:** `kani`/`cargo-kani` are **not installed** on this host; there is **no** Kani
reference in `kernel/Cargo.toml` or `.github/workflows/`. Item 7 (roadmap: "Kani wiring — Keccak,
FSM graph algorithms, NTT arithmetic, GCRA transition") has not started.

**The call: item 8 does NOT wait for item 7 — but it DOES wait for the Kani toolchain.** Split the
dependency honestly in two:

1. What item 8 needs from "Kani" is **one self-contained harness on one pure function** (§4 Leg 1,
   ~40 lines, `#[cfg(kani)]` in `token_bucket.rs`) plus the ability to run
   `cargo kani --harness gcra_step_sound` once, locally, with the result recorded in the decision
   package. Toolchain bootstrap = `cargo install kani-verifier && cargo kani setup` — an hours-
   scale shared prerequisite, not item 7's arc.
2. What item 7 owns is the **standing CI job** and the breadth (Keccak/FSM/NTT). Item 8's harness
   is deliberately written to become **item 7's pilot customer**: when item 7 lands its CI wiring,
   it lifts the already-proven GCRA harness into the job unchanged, and the hardening-gate row
   gains a deterministic re-executed item-4 check (CHECKLIST.md:60-61's own upgrade path).

**Consequence for "built and tested before it ships" (§0 ruling):** the swap may NOT merge on the
oracle alone — the ruling names both legs. So the merge gate for the swap commit is: (a) oracle
green incl. planted-leak RED-proof (§3), (b) `cargo kani` local run green on both harnesses with
output archived in the package, (c) 8-thread stress green, (d) full kernel suite + hardening-gate
green with the flipped manifest row. If the operator prefers zero new toolchain installs this
week, item 8 honestly stalls at "oracle done, swap staged, Kani leg pending" — it does not ship on
a weakened gate.

### Execution order (each commit green)

1. **C1** — time seam + `ReferenceBucket` (verbatim mutex arithmetic) + anchor parity test against
   the live impl. No behavior change.
2. **C2** — differential oracle (schedules, 4 assertions, planted-leak self-test) driving
   `ReferenceBucket` vs a not-yet-wired `GcraCandidate`. Manifest row updated, gap still ledgered.
3. **C3** — Kani harnesses `#[cfg(kani)]` + toolchain bootstrap + archived local proof output.
4. **C4** — **the swap**: `Repr` lands inside `TokenBucket`, mutex arm deleted from production
   code (retained as `ReferenceBucket`), chaos seam relocated, A6 test rationale rewritten,
   8-thread stress added, HOT-PATHS.tsv:38 gap → `-`, debug shadow cross-check on. All three
   existing behavioral tests must pass UNCHANGED — they are part of the oracle.
5. **C5 (item 7, later)** — lift C3's harness into the standing Kani CI job.

**Exit criterion (roadmap item 8 verbatim):** "the NEEDS-OPERATOR-DECISION gate resolves with
evidence" — the gate note in `token_bucket.rs:17-25` is replaced by a pointer to this package and
the ruling; the ADOPT ships as C4.
