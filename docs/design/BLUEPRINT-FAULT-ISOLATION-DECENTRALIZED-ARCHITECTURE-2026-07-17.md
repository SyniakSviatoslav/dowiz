# BLUEPRINT — Fault-Isolated Decentralized Architecture (Phase 27) — 2026-07-17

> Status: PROPOSED (design + audit artifact only; no production code in this change).
> Author: master-systems-architect pass, 2026-07-17, branch `feat/harness-llm-backend`.
> Style contract: plain evidence-grounded prose, no metaphor; every load-bearing claim carries a
> `file:line` cite, a primary-source URL, or an explicit **(proposal)** / **(unverified)** tag.
> Protocol: Detailed Planning Protocol (AGENTS.md §"Detailed Planning Protocol") — ground truth
> first, explicit dependencies, inline DECART, falsifiable done-checks, 2-question doubt audit,
> Anu/Ananke check.
> Audit provenance: Part 1's findings were produced by two decorrelated code-audit passes this
> session (kernel/src + llm-adapters; engine/src + tools) plus a web-grounded research pass; every
> finding was verified by reading the cited lines, and every cite below is spot-checkable.

---

## 0. Scope banner — what this is and is not

**The ask:** an event-driven, spec-driven, DoD-gated architecture, decentralized and autonomous,
such that one error in any component/layer can never propagate to, pollute, or stop any other
component/layer — with dynamic adapters/bridges, error containment, and noise filtering.

**The honest headline:** this repo already practices most of that doctrine, deliberately, in its
best components — the RCI degradation ladder (advisory/fail-open, STALE-not-wrong, hook no-ops if
the binary is missing: `realtime-change-intelligence-2026-07-17/proposal.md` §7), GapWire's
single-drainer/deadletter topology (`BLUEPRINT-EVENT-DRIVEN-ORCHESTRATOR-2026-07-17.md` §3.3),
P24's SPSC fail-on-full-with-drop-counter rings, the H1 typed-durability-pole fix
(`event_log.rs:188`, `hydra.rs:856-889`), and `spool.rs`/`bounded_drainer.rs`'s bounded,
degrade-closed queues. **This blueprint is therefore not a new framework.** It does three things:

1. **Audit (§1):** names every place the doctrine is violated today, ranked, with `file:line`.
2. **Research (§2):** grounds the doctrine in the established patterns and math (OTP supervision,
   circuit breaker/bulkhead, reliability block algebra, EWMA/φ-accrual noise filtering) so the
   design decisions are derivable, not asserted.
3. **Design (§3-§5):** one new kernel primitive (`CircuitBreaker`, sibling of `TokenBucket`),
   the insertion points at real seams, and a standing rule (§6, proposed for AGENTS.md) that makes
   the doctrine structural for every future blueprint — tied to the existing SDD pipeline
   (`.specify/constitution.md` + `openspec/` propose→apply→archive), not a second one.

**"Never propagates" stated honestly, up front.** Absolute never does not exist in a system that
shares a host, a disk, and a power supply. What the math in §2.3 licenses is: failures of
components A and B are made *statistically independent* by removing every shared mutable coupling
between them, at which point the probability of joint failure is the product of two small numbers
instead of the sum. This blueprint's real content is the enumeration and removal of the specific
couplings that exist today (§1.3), plus the named residual common causes that remain (one host,
one disk, one operator — §7 Q2).

**Scope rule inherited unchanged:** dev-plane isolation machinery is advisory for runtime hubs;
M5/M9 hub autonomy and the P10 kill-switch remain the operator's overrides (RCI proposal §0).

---

## 1. PART 1 — Audit: concrete long-term stability/performance findings

### 1.1 What is already done right (recorded so it is not re-invented)

| Pattern | Where (verified) | Why it is the model |
|---|---|---|
| Two failure poles, typed, never conflated | `event_log.rs:188` fallible `EventStore::insert` → `StoreError{Open,Write,Flush,Sync}`; `hydra.rs:856-889` updates memory only after `sync_all` succeeds; `CommitError::Store` vs `::Rejected` kept distinct (`event_log.rs:339-361`) | Durability plane fails **closed** with a typed error; a lost write can never masquerade as `Committed` (RED-proof tests `event_log.rs:701-716`, `hydra.rs:707-724`) |
| Advisory plane fails **open** | `tools/loop-signals/check.sh` (hook no-ops if binary missing); RCI §7 (STALE snapshot kept, hooks pass, one alarm line) | A dev-plane organ's death never touches the commit path |
| Bounded queue + degrade-closed drain | `spool.rs:70-82` (`append` returns `None` when full — never blocks, never grows), `bounded_drainer.rs` (k-per-tick, TokenBucket-gated) | Backpressure is visible, not absorbed |
| One-bad-item quarantine | `async-spool/src/main.rs:186-201` deadletter-on-parse-failure; GapWire §3.3 torn-line honesty | Malformed input is preserved evidence, not a loop-killer (but see A1 — the *send-failure* case is NOT covered) |
| Single-writer-by-construction | GapWire §3.3 (producers append unordered idempotent lines; exactly one drainer folds); P24 §1.5 (SPSC per ring) | The RCI H1 chain-fork class is structurally unreachable |
| Poison-tolerant shared state | `retrieval/memory_store.rs:66-90` — every accessor degrades on a poisoned lock (`.lock().ok()?`, `"lock_poisoned"` sentinel) | The correct in-process containment pattern, applied exactly once in the tree |
| Process-per-hub tenant boundary | `DELIVERY-FLOWS-BACKEND-AUDIT-...-2026-07-17.md` §4: zero tenant fields in kernel types; M5/M10 make the boundary a process boundary | The strongest bulkhead already in canon: shared-nothing isolation-by-instance |

### 1.2 Findings (ranked; each with disposition)

**CRITICAL**

- **A1 — Head-of-line blocking wedges the live alerting pipeline forever.**
  `tools/telemetry/rust-spool/src/main.rs:240-247`: the drainer always retries `entries[0]` and
  only removes it on success; there is **no deadletter path for a permanently-rejected send**
  (bot removed from chat, invalid `chat_id`) — the same head item is retried every ~2 s forever
  and every independent message behind it is starved. `tools/async-spool/src/main.rs:366-381` has
  the same wedge for `Sendable` entries past `MAX_ATTEMPTS` (they stay queued, retried forever).
  This is the production alerting path — `tools/telemetry/lib.sh:35-43` auto-launches
  `telemetry-spool`. One poisoned message silently kills *all future operator visibility*, which
  is precisely the failure class this whole blueprint exists to forbid.
  **Disposition: fix directly (Wave F0, §5) — attempt-cap → deadletter-and-advance, mirroring the
  parse-failure path that already exists three functions away.**

- **A2 — `FileBlockStore::put` panics on transient disk I/O.**
  `kernel/src/backup.rs:198,209,217`: `panic!` on `create_dir`/`write`/`rename` failure, because
  the `BlockStore` trait's `put(&mut self, id, bytes) -> bool` has no error pole — the identical
  infallible-port disease H1 just cured on `EventStore` (there the impl *swallowed*; here it
  *panics* — the two ways a missing failure pole always resolves). No caller outside `backup.rs`
  today (grep-verified), but this is the disk half of the public trait the module doc offers to
  future backends; it lands the day anything real is pointed at it.
  **Disposition: fix directly (Wave F0) — `put -> Result<(), StoreError>`, reusing H1's exact
  error taxonomy; the "every port is fallible" rule (§3.1) generalizes it.**

**HIGH**

- **A3 — Unbounded LLM response cache.** `CachingBackend` wraps every backend in
  `Arc<Mutex<MemStore>>` (`llm-adapters/src/cache.rs:36-44`), and `MemStore::put`
  (`backup.rs:78-86`) is a bare `HashMap` insert — no eviction, TTL, or size cap (grep-verified).
  Every distinct prompt tuple is cached as a full response body for process lifetime: a real
  memory leak under a long-running dispatcher. **Disposition: independently found and OWNED by
  the concurrent Phase 26 blueprint
  (`BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md` §1.4, "leak-shaped finding #1" —
  byte-bounded LRU store behind the existing `BlockStore` trait). Convergent double-detection
  recorded as corroboration, not duplicated here; this blueprint keeps only the *dispatcher
  bulkhead* half (A4), which P26 does not cover.**
- **A4 — `Dispatcher`'s concurrency bound is dead code.** `workers` is stored
  (`llm-adapters/src/dispatch.rs:60,70`) and never read; `dispatch()` does `thread::spawn`
  unconditionally per call (`dispatch.rs:89`). `TokenBucket` bounds *volume over time*, not
  *concurrent in-flight* — the doc's claimed bulkhead does not exist. **Disposition: Wave F1 —
  semaphore on `workers` (the bulkhead §3.3 names); appended to HARNESS-LLM-BACKEND.md.**
- **A5 — One missing I/O timeout.** `tools/telemetry/topics/src/main.rs:66` `ureq::post` with no
  timeout — the only unconfigured ureq site in the repo (the other five are configured:
  `transport.rs:85` 120 s, `transport.rs:113` 10 s, `rust-spool:164` 10 s, `async-spool:285,310`).
  A connect-then-silent endpoint blocks the caller indefinitely. **Disposition: Wave F0 —
  one-line fix + the §6 rule makes timeout-per-blocking-call a standing requirement.**
- **A6 — Every append-only store grows forever; two are growing in production now.**
  `MemEventStore` (`event_log.rs:208-220`), `FileEventStore` (`hydra.rs:743-748`; `open()`
  replays the *entire* JSONL into memory, `hydra.rs:754-786`), `KnowledgeSpine`
  (`spine.rs:122-124`) — zero compaction/retention methods (grep-verified: no
  `compact|prune|evict|truncate|rotate` beyond cosmetic hits). Live evidence of the same class in
  tools: `tools/telemetry/logs/metric.jsonl` = 2,758,165 bytes and actively written today with no
  rotation anywhere in `lib.sh`/`governance.sh`/`report.sh`; async-spool's deadletter file is
  append-only with no retention (`async-spool/src/main.rs:107-111,186-201`). Deliberate
  durability is not the issue; the absence of any *stated* retention/snapshot story is.
  **Disposition: kernel stores → P12 (durable storage) already owns snapshot/compaction
  semantics — named there as a hard requirement, not silently deferred; telemetry logs +
  deadletter retention → appended to P24's blueprint (it already designs RRD tiers — the gap is
  that today's files are outside them).**
- **A7 — `FixedTimestep::seen_dts` unbounded Vec on the designated frame loop.**
  `engine/src/loop_.rs:29-33,70`: pushes every dt forever; only consumers are running max/min
  (`loop_.rs:85-92`) — two scalars would do. ~180K entries/hour at 50 Hz. Not yet wired into
  `wasm/` (verified), so blast radius today is zero — but it is built for exactly the hot path it
  would leak on. **Disposition: Wave F0 — replace Vec with two scalar accumulators.**
- **A8 — `ci-truth` runs suites with no timeout.** `tools/ci-truth/src/main.rs:342-358`
  `.output()` with no deadline, sequential at `:418-419`: one hung kernel test blocks the entire
  verification pass, including the engine suite behind it. **Disposition: Wave F0 — subprocess
  deadline (the RCI §7 "5 s timeout on Command" discipline, scaled for test runs).**
- **A9 — Unvalidated caller-controlled compute at the wasm boundary.**
  `engine/src/field_frame.rs:218-225` `compose(scene, eq, w, h, steps)` iterates `steps` and
  allocates `w*h` with zero clamps, exposed at `wasm/src/lib.rs:57-59`. **Disposition: Wave F1 —
  clamp at the port (§3.3 validation bulkhead).**

**MEDIUM**

- **A10 — Exported-API panics on structural invariants.** `causal.rs:839`
  (`.expect("...superset c-component must exist")`) and `causal.rs:807-808` inside `id()`,
  reachable via public `identify_causal_effect` (`lib.rs:176`); zero callers today — inert, but a
  panic that would cross straight into any future caller. **Disposition: Wave F1 — return
  `Option`/typed error at the public boundary.**
- **A11 — `partial_cmp().unwrap()` sort pattern ×15** across `wasm.rs`, `csr.rs`, `spectral.rs`,
  `householder.rs`, `spectral_cache.rs`, `retrieval/diffusion.rs:137`; the tree already contains
  the fix once (`spectral_cache.rs:125` `.unwrap_or(Equal)`). NaN in one caller's data panics an
  unrelated module's sort. **Disposition: Wave F1 — mechanical sweep to `total_cmp` or
  `unwrap_or(Equal)`.**
- **A12 — Lock-poisoning inconsistency.** `token_bucket.rs:63,75`, `budget.rs:144,153`,
  `llm-adapters/src/cache.rs:94,104` all `.lock().unwrap()`; critical sections are panic-free
  *today*, but `CachingBackend`'s `Arc<Mutex<S>>` is shared across all Dispatcher worker threads —
  the moment any future `S` panics inside the lock (e.g. A2's panicking store), one caller
  poisons the cache for every thread. `memory_store.rs` proves the house fix. **Disposition:
  Wave F1 — poisoning-discipline sweep (§3.4 rule).**
- **A13 — TLS accept loop kills the whole server on one accept error.**
  `tools/native-spa-server/src/main.rs:101-102` `listener.accept().await?` — fd exhaustion under
  a connection storm exits the listener (the plain-HTTP axum path is fine — per-connection
  isolation verified). **Disposition: Wave F1 — log-and-continue on transient accept errors.**
- **A14 — `deep-clean` spawns `tar`/`zstd` with no timeout** (`tools/deep-clean/src/main.rs:444-489`). **Disposition: Wave F1.**
- **A15 — `skillspector-rs` recompiles all 402 rule regexes per scanned file**
  (`tools/skillspector-rs/src/engine.rs:180-183`, plus per-match recompile at `:159`) — O(files ×
  402) compilations; pure speed, no correctness risk. **Disposition: Wave F1 — compile once.**
- **A16 — `ParticlePool::new(0)` panics on first `spawn`** (`engine/src/widget_store.rs:98-110`);
  no production caller yet. **Disposition: Wave F1 — reject zero capacity at construction.**

Recorded clean (so they are not re-flagged): `spool.rs` bounded with real cleanup;
`bounded_drainer.rs` degrade-closed; no one-bad-item-aborts-batch `?` loops found in kernel
drain paths; engine has zero global mutable state and zero non-test unwraps beyond two documented
fail-closed asserts; exactly two kernel statics, both panic-free at init (`recall.rs:306`,
`wasm.rs:51`); `kalman.rs:260`'s panicking `gain()` is dead outside its test while the hot-path
`update()` fails closed.

### 1.3 The four cross-cutting classes (what the findings have in common)

1. **Infallible ports** (A2; pre-H1 `EventStore`): a port trait with no error pole forces every
   real impl to either swallow or panic. Rule: **every port is fallible** (§3.1).
2. **Unbounded append** (A3, A6, A7; async-spool deadletter): anything that only ever grows needs
   a stated cap/eviction/rotation/snapshot story at design time, even if the story is "operator
   rotates manually, alarmed at N MB."
3. **Missing deadline** (A5, A8, A14): any blocking call to another process/host without a
   timeout donates the caller's liveness to the callee.
4. **Shared-fate coupling** (A1's head-of-line queue, A4's unbounded shared spawn, A12's
   poisonable shared lock, A13's shared accept loop): independent work items or callers coupled
   through one mutable point, so one failure is every caller's failure.

---

## 2. PART 2 — Research: the established patterns and the math

(Web-grounded this session; primary sources cited inline.)

### 2.1 Erlang/OTP "let it crash" + supervision trees — and the honest Rust mapping

Armstrong's thesis names fault isolation as *the* essential problem: "We do not want the errors in
one module to adversely affect the behaviour of a module which does not have any errors," and
makes share-nothing process isolation the precondition — "as soon as two processes share any
common resource... the possibility exists that a software error in one of the processes will
corrupt the shared resource" (*Making Reliable Distributed Systems in the Presence of Software
Errors*, 2003, https://www.erlang.org/download/armstrong_thesis_2003.pdf, ch. 2). The
error-handling philosophy — "let it crash," "do not program defensively" — works because "the
error-handling code and the code which has the error execute within different threads of
control" (ch. 4): workers do the job; supervisors, holding no business state, observe exits and
restart. OTP mechanics (https://www.erlang.org/doc/system/sup_princ.html): `one_for_one` /
`one_for_all` / `rest_for_one` restart strategies, and bounded restart intensity — more than
`MaxR` restarts in `MaxT` seconds terminates the supervisor itself, escalating upward: a
recursive backstop against infinite crash loops.

**What this codebase gets free vs. what is a real gap:**

| OTP property | Status here |
|---|---|
| Share-nothing between units | **Mostly free**: ownership forbids aliased mutable state in safe Rust; process-per-hub (§1.1) is genuine MMU-level isolation, stronger than BEAM's. |
| Errors handled by a separate observer | **Partially present**: drainer processes are auto-relaunched (`lib.sh:35-43`); GapWire §4.3 plans heartbeat ledger lines. **Gap:** no restart-intensity policy anywhere — a crash-looping drainer relaunches forever with no MaxR/MaxT escalation. |
| Failure of one unit cannot corrupt another | **Violated in-process** exactly where state is shared: Mutex poisoning (A12) is the cross-contamination Armstrong's rule exists to prevent — `std::sync::Mutex` docs: a panic while holding the lock leaves it poisoned for every other thread (https://doc.rust-lang.org/std/sync/struct.Mutex.html). `catch_unwind` exists but its own docs warn it cannot catch `panic = "abort"` panics (https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) — process isolation, not unwinding, is the reliable boundary. |
| Supervision machinery | **Absent as a library**; present as OS substrate (process relaunch). §3.4 closes the gap with policy, not a framework. |

**Verdict:** the architecture does not need an actor framework. It needs (a) the restart-intensity
policy OTP would give the supervisors it already has, (b) the poisoning discipline that restores
share-nothing semantics where `Arc<Mutex<_>>` is deliberately used, and (c) fallible ports so a
unit's death is an observable `Err`, never an unwinding panic crossing a module boundary.

### 2.2 Circuit breaker + bulkhead — and why Hystrix's deprecation matters

Fowler's canonical breaker (https://martinfowler.com/bliki/CircuitBreaker.html): **closed** (calls
pass; failures counted), **open** once failures cross a threshold (calls fail fast without
touching the dependency), **half-open** after a reset timeout (one trial call; success closes,
failure re-opens). Netflix deprecated Hystrix in favor of "more adaptive implementations that
react to an application's real time performance rather than pre-configured settings"
(https://github.com/Netflix/Hystrix — maintenance-mode notice), pointing to resilience4j and
adaptive concurrency limits. Two lessons transfer directly: (1) resilience4j computes failure
*rate over a sliding window* gated by `minimumNumberOfCalls` — a small sample can never trip the
breaker (https://resilience4j.readme.io/docs/circuitbreaker); (2) it deliberately separates the
breaker (failure exposure) from the **bulkhead** (concurrency bound) — semaphore bulkhead for the
cheap general case, fixed-thread-pool bulkhead when the caller's own threads must never block
(https://resilience4j.readme.io/docs/bulkhead). Microsoft's bulkhead write-up states the problem
in Armstrong's terms for services: partition consumers/resources so "excessive load or failure in
a service" cannot cascade (https://learn.microsoft.com/en-us/azure/architecture/patterns/bulkhead).
Netflix's replacement, concurrency-limits, is explicitly TCP-congestion-control math — adaptive
limit as congestion window, gradient over two EWMAs (https://github.com/Netflix/concurrency-limits)
— i.e., the breaker/limiter is a **feedback controller**, which is §2.4's point.

**Mapping to this codebase:** the port/adapter layer (`LlmBackend` + `transport.rs`, the planned
`SocialPoster` (P22), `EventStore`/`BlockStore`, the spool drainers' Telegram sends) is exactly
the structural slot these patterns were invented for. Verified current state: timeouts exist at
5 of 6 call sites (A5); **no circuit-breaking exists anywhere** (a dead Telegram endpoint is
retried every 2-3.5 s forever — A1 is the degenerate no-breaker case); the one designed
concurrency bulkhead is dead code (A4). `TokenBucket` (rate) exists and is the right *sibling*:
rate, concurrency, and failure-exposure are three orthogonal admission decisions (§3.2).

### 2.3 The reliability math — what isolation actually buys

Standard identities (https://en.wikipedia.org/wiki/Availability,
https://en.wikipedia.org/wiki/Reliability_block_diagram): availability A = MTBF/(MTBF+MTTR).
**Series** (any component's failure fails the system): A_sys = ∏ A_i — always weaker than the
weakest link. **Parallel/isolated** (system fails only if all fail): unavailability multiplies,
A_sys = 1 − ∏ (1−A_i). Fault isolation is the operation that moves a dependency out of the
series path: with a breaker + fallback, the caller's availability is no longer multiplied by the
dependency's. Worked example from this audit: today the telemetry pipeline is *series through the
queue head* (A1) — P(alert N delivered) ≤ P(every earlier message deliverable); one permanently
undeliverable message drives delivery availability of everything behind it to zero. With
attempt-cap → deadletter, messages become independent: each message's delivery depends only on
its own endpoint outcome.

**The binding caveat, stated by the same source:** the product formulas hold only under
*statistical independence*; common-cause failures (shared disk, shared process, shared lock)
silently invalidate them. This is why §1.3 class 4 is the audit's center of gravity: every shared
mutable coupling removed is what converts the arithmetic from series to parallel. The residual
common causes that remain — one host, one disk, one systemd, one operator — are named in §7 Q2,
not hidden.

**Breaker-as-control-loop:** Google SRE frames cascading failure as positive feedback (retries
amplifying load — https://sre.google/sre-book/addressing-cascading-failures/) and client-side
adaptive throttling as a closed loop over locally-observed `requests`/`accepts`
(https://sre.google/sre-book/handling-overload/) — computed from local state only, which is the
same LOCAL-DECISION rule P25 §3.1 already binds this repo to. The half-open probe is the loop's
measurement action on the plant; hysteresis (trip fast, close only after k probe successes) is
the standard anti-oscillation choice, and it is the same shape as P25's admission hysteresis.

### 2.4 Noise filtering — reuse the math already in-tree

Production breakers avoid tripping on transients with (a) a minimum sample floor and (b) a
windowed/decayed aggregate instead of a raw counter (§2.2). The oldest battle-tested form of (b)
is the EWMA: RFC 6298's SRTT ← (1−α)·SRTT + α·R′ with α = 1/8, plus a smoothed deviation term
(https://www.rfc-editor.org/rfc/rfc6298) — smooth a noisy per-sample signal before triggering a
costly action. **This is byte-for-byte `ema_next` (`geo.rs:39`)** — the scalar steady-state
Kalman (`kalman.rs:3-6`) this repo already uses for courier kinematics and RCI's error organ. No
new statistics are needed for the trip decision: EMA of the per-call failure indicator +
`min_calls` floor + threshold is the resilience4j-equivalent filter built from an existing
kernel function.

Two stronger detectors are named and deliberately deferred, each with a trigger:
- **Wilson lower bound** on windowed failure rate (already the floor used by RCI's backtest and
  `csr.rs:387-427` scorers): trip only when the *lower confidence bound* clears the threshold —
  strictly fewer false trips at small n. Trigger: measured false-trip rate of the EMA breaker on
  a real adapter exceeds ~1/week.
- **φ-accrual failure detection** (Hayashibara et al. 2004; Akka/Cassandra production use —
  https://doc.akka.io/libraries/akka-core/current/typed/failure-detector.html): a continuous
  suspicion level from the fitted inter-arrival distribution of heartbeats, instead of a binary
  timeout. Wrong tool for per-call adapter errors; the right tool for **hub↔hub peer liveness**
  (P9/P10 wire), where heartbeat inter-arrival is the native signal. Trigger: P9/P10
  implementation reaches peer-failure detection.
- The Markov attractor's entropy/Foster-Lyapunov measures stay in the agent-behavior domain
  (tool-outcome streams) — a different signal domain; no merge (same separation P24 §1.6 draws).

---

## 3. PART 3 — Design

### 3.1 The two-pole doctrine, made explicit (it already exists — name it, enforce it)

Every component declares which failure pole it is on, and the poles are never mixed:

- **Fail-closed (correctness plane):** durability, money, law/decide, crypto. A failure is a
  typed `Err` that stops *this operation only*, with state unadvanced (H1 pattern:
  `event_log.rs:293-312` — `?` before `set_tip`). Never a panic across a public boundary, never
  a silent swallow.
- **Fail-open (advisory/observability plane):** telemetry, RCI, hooks, gap emission, breaker
  snapshots. A failure degrades to STALE/no-op with one logged line and a drop counter; it never
  blocks the plane above it (RCI §7; P24 fail-on-full rings; `check.sh`).

**Structural rule that falls out of A2 + pre-H1 history: every port trait is fallible.** An
infallible port signature (`fn put(..) -> bool`, pre-H1 `fn insert(..)`) *forces* its impls to
panic or swallow — both poles collapse. New rule (§6): a port trait whose methods can touch I/O
must return `Result<_, TypedError>`; "cannot fail" requires a comment proving it, not a shrug.

### 3.2 The primitive — `kernel/src/breaker.rs` (proposal)

Sibling of `TokenBucket` (`token_bucket.rs`), same construction idiom: plain std, `Mutex<Inner>`,
monotonic `Instant` (NTP-immune), all-or-nothing admission, zero I/O, zero deps. `TokenBucket`
bounds **rate**; the semaphore (§3.3) bounds **concurrency**; `CircuitBreaker` bounds **failure
exposure**. Three orthogonal admission primitives, one idiom.

```rust
pub enum BreakerState { Closed, Open, HalfOpen }

pub struct BreakerPolicy {
    pub alpha: f64,           // EMA smoothing for the failure signal (geo.rs::ema_next)
    pub trip_threshold: f64,  // trip when ema >= threshold ...
    pub min_calls: u32,       // ... AND at least this many calls seen since last transition
    pub open_cooldown_ms: u64,// Open -> HalfOpen after this cooldown
    pub probe_successes: u32, // consecutive HalfOpen successes required to Close (hysteresis)
}

pub enum Admit { Yes, Probe, No { retry_after_ms: u64 } }

pub struct CircuitBreaker { policy: BreakerPolicy, inner: Mutex<Inner> }
// Inner: { state, ema_fail: f64, calls_since_transition: u32,
//          probe_ok_streak: u32, last_transition: Instant }

impl CircuitBreaker {
    pub fn new(policy: BreakerPolicy) -> Self;
    pub fn admit(&self) -> Admit;                 // pure read + lazy time-based transition
    pub fn record_ok(&self) -> Option<Transition>;
    pub fn record_err(&self) -> Option<Transition>; // returns Some on state change ONLY
    pub fn snapshot(&self) -> BreakerSnapshot;     // {state, ema_fail, calls, since_ms} for telemetry
}

// Pure decision core, unit-testable with injected time (no Instant, no Mutex):
pub fn step(inner: &InnerData, policy: &BreakerPolicy, now_ms: u64, ev: Event) -> (InnerData, Option<Transition>);
```

Design decisions, each derived from §2:
- **Trip filter = `ema_next` + `min_calls` floor** (§2.4): transient noise cannot trip; a small
  sample cannot trip. Constants are policy-as-data (per-adapter TOML/genesis field), not code —
  the same policy-as-data shape as GapWire's `TriagePolicy` and P10's `HubPolicy`, so P15's
  future self-revision path applies without redesign.
- **Only *transitions* are events.** `record_*` returns `Some(Transition)` on state change only;
  per-call failures never leave the primitive. This is the noise filter at the emission boundary:
  a flapping dependency produces a handful of transition events, never a failure-per-call flood.
- **Poisoning discipline (A12 fix, applied at birth):** the critical section is provably
  panic-free arithmetic, and the lock is taken with
  `.lock().unwrap_or_else(|e| e.into_inner())` — a documented, deliberate poison-recovery,
  legitimate exactly because the section cannot leave `Inner` torn. This is the rule §3.4 sweeps
  through `token_bucket.rs`/`budget.rs`/`cache.rs`.
- **Degrade direction is per-plane:** an advisory-plane caller treats a poisoned/unavailable
  breaker as `Closed` (pass-through, fail-open); a correctness-plane caller treats it as `Open`
  (refuse, fail-closed). The primitive exposes state; the *caller's plane* (§3.1) picks the
  default — never the primitive.

### 3.3 Bulkheads at the real seams (each tied to an audit finding)

| Seam | Bulkhead (proposal) | Closes |
|---|---|---|
| Dispatcher → LLM backend | Make `workers` real: a counting semaphore (std `Mutex`+`Condvar` or atomic permit count) capping in-flight spawns; over-cap ⇒ typed `Busy` refusal, degrade-closed | A4 |
| Spool drainers → Telegram/endpoints | Already one-queue-per-destination (a bulkhead); add attempt-cap → deadletter-and-advance so one item cannot absorb the lane | A1, A6-deadletter |
| wasm/browser → engine | Clamp `w`, `h`, `steps` at the `#[wasm_bindgen]` port with documented maxima; reject zero-capacity pools at construction | A9, A16 |
| kernel ↔ engine | Already clean (no shared mutable state, engine has zero statics — audit §1.2 clean list); keep it structural: the §6 rule forbids introducing one |
| hub ↔ hub (P9/P10) | Process-per-hub stays the primary bulkhead (§1.1); per-peer `CircuitBreaker` in the transport with policy from `HubPolicy` fields; φ-accrual deferred to peer-liveness (§2.4) | future |
| dev-plane → runtime | Unchanged: advisory/fail-open, SCOPE RULE (§0) | — |

### 3.4 Supervision without a framework

- **Restart intensity (the OTP MaxR/MaxT gap):** every long-lived drainer/orchestrator process
  gets a supervisor policy — under systemd, `Restart=on-failure` + `StartLimitBurst`/
  `StartLimitIntervalSec` (the direct MaxR/MaxT analog) + `RestartSec` backoff; for the
  `lib.sh:35-43`-style auto-relaunch path, the same bound implemented as a relaunch counter file
  with a cooldown. Exceeding the bound = stop relaunching + one Blocker line into the async-spool
  queue (operator sees it; the failed lane stays down instead of crash-looping). **(unverified:
  whether a systemd unit for `telemetry-spool` exists today or only the lib.sh auto-launch —
  checked at implementation time; the policy applies to whichever supervisor is real.)**
- **Liveness observation:** GapWire's heartbeat-ledger-line pattern (§4.3 there) becomes the
  norm for every drainer: one heartbeat line per N minutes in its own ledger; absence is
  detectable by the P24 gauge surface without any new mechanism.
- **Panic discipline:** (a) panics must not cross public module boundaries on correctness planes
  — fallible ports (§3.1) make the compiler enforce the pole; (b) the poisoning rule: any
  `Mutex` shared across threads either has a provably panic-free critical section (documented,
  `into_inner` recovery — §3.2) or degrades per `memory_store.rs:66-90`; (c) process isolation,
  not `catch_unwind`, is the trusted boundary (§2.1 — `catch_unwind` misses `panic = "abort"`).

### 3.5 Event wiring — no fourth mechanism

Breaker/bulkhead/supervision events ride the three event systems built today, per plane:

- **Snapshots (gauges):** `BreakerSnapshot` per adapter → P24 ring as counter/gauge events
  (fail-on-full, drop-counted — a telemetry stall can never block an adapter call).
- **Transitions (facts):** `Closed→Open` on a named-critical adapter ⇒ one `GapEvent`
  (`kind: BlockedDependency`, severity from policy) into the GapWire queue — idempotent by
  content-id, so a flap re-emitting the same transition dedups; GapWire's existing triage decides
  Telegram vs. register-only. Emission is fire-and-forget spool append (µs), fail-open.
- **Explanations:** every transition event carries the P24 capsule discipline — inputs (ema,
  calls, threshold, window), not a bare "breaker opened."

### 3.6 Spec-driven + DoD-gated — extending the existing pipeline, not inventing one

- **Spec-driven:** the SDD pipeline is live (`.specify/constitution.md` — "SDD pipeline is
  mandatory before non-trivial code: constitution→spec→plan→tasks→analyze→implement→converge";
  `openspec/` propose→apply→archive with `config.yaml`). This blueprint adds one artifact rule
  (§6): every openspec proposal for a component that calls another process/host/component MUST
  contain a **Fault Containment** section declaring the five items in §6. Enforced where
  proposals are already reviewed — no new tooling required on day one; a proposal-template lint
  is the named follow-up.
- **DoD-gated:** constitution rule 4 ("RED→GREEN or not done") applied to isolation claims
  specifically: every containment claim ships with a fault-injection test proven RED first —
  kill/hang/reject the dependency, assert the caller degrades per its declared pole. The H1
  `FaultyStore` tests (`event_log.rs:701-716`) are the house precedent. §5's done-checks are all
  of this form.

---

## 4. DECART — the breaker mechanism (new integration class: reliability primitive)

| Candidate | Native fit | Falsifiable | Cost | Supply chain | Reversibility | Verdict |
|---|---|---|---|---|---|---|
| resilience4j-style external crate (`failsafe`, `recloser`, …) | ✗ crates.io egress 403 (recorded live probe, P15 §9); several assume tokio, contradicting the recorded no-tokio DECART (`dispatch.rs:1-8`) | crate tests exist, unauditable here | small | blocked + unvetted | moderate | REJECT (adopt the *sliding-window/min-calls concepts*, credited §2.2) |
| Adaptive concurrency limits (Netflix-style gradient) | native possible | hard to falsify without load rigs | high | none | poor (tuning surface) | DEFER — named trigger: a measured overload incident static breakers + bulkheads fail to contain |
| φ-accrual detector now | native possible | needs heartbeat streams that exist only at the P9/P10 seam | moderate | none | good | DEFER to peer-liveness (§2.4 trigger) |
| **Minimal native `breaker.rs` + semaphore + policy sweep** ← | ✓ std-only, mirrors `token_bucket.rs` idiom; reuses `ema_next` | pure `step()` core, table-testable; every insertion has a RED fault-injection check | one small module + line-level fixes | zero new deps | trivial (delete module; call sites revert to direct calls) | **ADOPT (build)** |

**Probe (strongest argument against building):** hand-rolled reliability primitives rot when
their constants are wrong and nobody measures. Mitigation is structural: constants are
policy-as-data with stated defaults; snapshots flow to P24 so trip/flap rates are *measured*; the
two upgrade paths (Wilson, adaptive limits) are pre-named with triggers, so outgrowing v1 is a
planned step, not a rewrite.

---

## 5. Build plan — waves + falsifiable done-checks (each RED-first)

```
F0 (independent, start immediately): A1 deadletter-and-advance · A2 fallible BlockStore ·
     A5 timeout · A7 scalar accumulators · A8 ci-truth deadline
F1 (independent of each other; F1a-c after F0 lands where same-file):
     F1a breaker.rs + policy   F1b Dispatcher semaphore (A4; A3's cache cap is P26's unit —
                                    coordinate, don't double-build)
     F1c clamp sweep (A9/A16) + panic/poison sweep (A10/A11/A12) + A13/A14/A15
F2 (needs F1a + GapWire W1): transition→GapEvent wiring + P24 snapshot wiring
F3 (needs P9/P10 work to exist): per-peer breaker + φ-accrual — NOT schedulable here
```

- **F0-A1:** integration test: queue = [permanently-rejected msg, good msg]; drainer must deliver
  the good msg and move the bad one to `.deadletter` within the attempt cap. RED today (good msg
  never sends — verified by code reading; the test freezes the finding).
- **F0-A2:** H1-style `FaultyStore` test on `BlockStore`: full-disk/EROFS injection ⇒ typed `Err`,
  memory state unadvanced, **no panic** (`catch_unwind` harness asserts UnwindSafe not breached).
- **F0-A5/A8:** hang-injection (a listener that accepts and never responds; a test binary that
  sleeps) ⇒ caller returns timeout error within deadline+ε.
- **F1a:** `cargo test -p dowiz-kernel breaker::` — table tests over the pure `step()`: min_calls
  floor (9/9 failures below floor ⇒ still Closed), EMA trip, cooldown→HalfOpen, probe-streak
  close, flap ⇒ exactly 2 transition events; property: `Admit::No` never returned in Closed.
- **F1b:** spawn 3×`workers` concurrent dispatches against a slow fake backend ⇒ in-flight never
  exceeds `workers` (observable via the fake's gauge); cache: insert past byte cap ⇒ size stays
  ≤ cap, eviction counted.
- **F2:** kill the fake backend ⇒ within policy window: exactly one `GapEvent` in the queue
  (idempotent under flap), snapshot visible in P24 ring, dispatcher calls fail fast (`Admit::No`)
  instead of stacking timeouts.
- **Sweep checks (F1c):** grep-gates runnable in CI: zero `partial_cmp(...).unwrap()` in
  `kernel/src` + `engine/src`; zero `.lock().unwrap()` outside the documented
  panic-free-`into_inner` pattern; every `ureq` call site carries `.timeout(`.

---

## 6. Proposed AGENTS.md addition (clearly marked — NOT applied by this blueprint; operator merges)

> ### Fault-isolation-by-default rule (proposed, 2026-07-17)
>
> Every future blueprint, and every openspec proposal for a component that calls another
> process, host, or component, MUST declare a **Fault Containment** section with five items:
> (1) **Pole** — fail-open (advisory plane) or fail-closed (correctness plane), per the two-pole
> doctrine; (2) **Deadline** — every blocking call names its timeout; (3) **Bulkhead** — the
> resource boundary (own process / own queue / semaphore cap) that keeps its failure out of
> other lanes, and the cap's value; (4) **Breaker or refusal** — the `CircuitBreaker` policy for
> repeated dependency failure, or an explicit "none, because <reason>"; (5) **Growth bound** —
> for any append-only structure, the stated cap/eviction/rotation/snapshot story. Every port
> trait that can touch I/O is fallible (`Result` with a typed error) — an infallible signature
> requires a written proof it cannot fail. Each containment claim ships with a RED-first
> fault-injection done-check (kill/hang/reject the dependency; assert the declared degradation).
> A blueprint or proposal without this section is incomplete under the Detailed Planning
> Protocol, exactly as an unclassified wave is under the Phase-25 rule. Grounding and audit
> precedent: `BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md`.

---

## 7. 2-question doubt audit

**Q1 — what would make this design wrong?** If the breaker guards dependencies whose real
failure mode is not "erroring endpoint" but "slow degradation below timeout" — a backend
answering in 119 s under a 120 s timeout never trips anything while destroying throughput. Named
honestly: v1 trips on *failures* (errors + timeouts), not latency drift. The P24 latency-anomaly
detector (EMA + innovation on per-site durations) is the organ that sees drift; the F2 wiring
gives it a path to *advise* (a GapEvent), and the adaptive-limits DECART row is the named upgrade
if a real incident shows static timeouts + failure-tripping insufficient. Second honest risk:
policy constants (α, thresholds, cooldowns) are v1 guesses — same posture as P25's PSI
thresholds; mis-tuning is cheap because the breaker only ever converts calls into fast typed
refusals on the advisory-configured adapters, and the correctness plane never depends on a
breaker being closed.

**Q2 — least-verified load-bearing claim?** That the audit is *complete* — two subagent passes
covered kernel/llm-adapters/engine/tools, but `wasm/`, `metric-core/`, `agent-governance*/`, and
the bebop-repo crates were out of scope this session; the four §1.3 classes are defined so the
sweep checks (§5 grep-gates) catch recurrences mechanically, but un-audited crates may hold
instances not yet listed. Also unverified: whether `telemetry-spool` runs under systemd or only
the lib.sh launcher (§3.4, flagged inline) — the restart-intensity design deliberately works for
either. Residual common causes accepted and named: one host, one disk, one systemd instance, one
operator; process-level isolation cannot remove them — only the P12 backup/restore floor and the
mesh's multi-node future (P9/P13) address host-level fate-sharing.

---

## 8. Anu / Ananke check

**Anu (does it follow?):** every insertion point derives from a specific audit finding with a
live cite, not from the pattern catalog ("breakers are good" appears nowhere as a reason — A1/A4/
A5's measured absences do); the math section's claims are primary-sourced and the one place the
formulas could mislead (independence assumption) is stated as the design's own center of gravity
rather than footnoted; the DECART rejects external crates on recorded evidence (egress 403,
no-tokio precedent), not taste. No sibling-doc contradiction found: this design consumes RCI's
degradation ladder, GapWire's topology, P24's rings, P25's LOCAL-DECISION/hysteresis, H1's poles,
and the delivery-flows audit's process-per-hub boundary — and contradicts none of them. Weakest
Anu link, named: §3.4's systemd-vs-lib.sh uncertainty (flagged inline, resolved at
implementation).

**Ananke (is the good outcome structural?):** fallible ports — compiler-enforced once the trait
changes (a swallow no longer compiles). Deadlines/poison/NaN-sort — grep-gates in CI (§5), not
review vigilance. One-bad-item quarantine — a state machine property with a RED test, not drainer
politeness. Transition-only emission — the primitive's return type, so a flood is unrepresentable.
Budget/bulkhead — refusal types, degrade-closed. The named non-structural residues: (a) the §6
rule binds only if the operator merges it — until then it is convention, stated plainly; (b)
policy constants require measurement to earn trust — structurally mitigated by P24 snapshots
making trip/flap rates observable, but observation still requires someone to look; flagged as the
same class of named Ananke debt GapWire's Q1 carries.

---

## Appendix — phase-table registration

Registered in `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8.9 as **Phase 27**. The
fresh-check rule earned its keep live: an early-session grep found 26 free, but at registration
time a concurrent session had already taken **P26** (§8.8, memory optimization) — the number was
re-derived from the roadmap's then-current state, and the collision is recorded here instead of
silently renumbered. Substantive overlap with P26 is exactly one finding (A3, convergently
detected by both passes — §1.2 records the ownership split). Depends on: **nothing hard** (F0/F1
are standalone fixes + one new pure kernel module); **soft**: 24 (snapshot surface — pre-P24 the
snapshots simply have no ring to land in and stay queryable via `snapshot()`), GapWire W1
(transition events — pre-W1 transitions log locally only), 26 (A3's cache cap lands there),
P9/P10 (F3 per-peer breakers — not schedulable until that seam exists). Off-critical-path lane,
same class as P5/P8/P11/P12/P24/P25/P26. Wave classification (P25 rule): F0/F1 are
CPU-bound-local at verification (cargo test), F2's integration checks likewise; no
I/O-bound-dispatch lanes beyond the ordinary agent work.
