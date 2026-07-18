# BLUEPRINT P-H — Ops / Telemetry / Benchmarks / Regression (2026-07-17, Wave 2)

> Wave-2 phase blueprint under `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
> Grounded in the Wave-1 audit `P-H-audit-telemetry-regression-benchmarks.md` (same directory);
> every code cite below was RE-verified live this pass on `feat/harness-llm-backend`, not inherited.
> Style contract: no metaphor; every load-bearing claim carries a `file:line` cite or is tagged
> **(proposal)**. Executable by an agent with zero prior session context (§2 item 18).
>
> **Scope in one sentence:** build the chaos/fault-injection harness (the one genuine greenfield),
> fix the one real CI bug (`ci.yml:23`), CI-gate the existing criterion benchmarks, and
> prune/migrate the regression ledger — while re-deriving NOTHING that P24/P25 already decided.

---

## Why this layer exists (context for a reader with zero session history)

Layer H is the layer that watches the other layers. Every phase A–G ships correctness claims —
"this gate fails closed," "this path never regresses," "this invariant holds under fault." Layer H
is where those claims stop being prose and become **machine-checked, permanently, in CI**. Its
governing worry is the failure mode that kills long-lived autonomous systems: a safety property
that was true when it was written and quietly stopped being true, with nothing to notice. So this
layer builds the instruments that make silence impossible — a chaos/fault-injection harness that
deliberately breaks each invariant and proves it holds anyway, a benchmark gate that turns a
performance regression into a red build, a fixed CI bug that was silently passing a step that did
nothing, and a regression ledger that keeps every past fix pinned forever.

The discipline that shapes every choice here is **reuse over re-derivation**: two sibling
blueprints (P24 native telemetry, P25 wave scheduling) already own all runtime telemetry and all
concurrency/scheduling, so Layer H folds them in *by reference* and adds exactly four things on top
(§0 lists them). Even the chaos harness is not invented from scratch — it generalizes a
fault-injection double (`FaultyStore`) the audit's own grep initially missed, so the repo ends with
one injection authority, not two. The problem Layer H solves, in one line: **make every other
layer's safety claim falsifiable and continuously falsified in CI, so a regression is a red build
and never a silent surprise.**

---

## 0. DO NOT RE-DERIVE — the two sibling blueprints that already own half this phase

Per standard §2 item 19 (reuse-first) and the Wave-1 audit's Area-4 verdict, these are folded in
**by reference**. A worker executing this blueprint must not re-open any decision inside them:

- **P24** `docs/design/BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md` —
  owns ALL runtime telemetry: the SPSC ring flight recorder (`kernel/src/ring.rs`, single-writer
  by construction per RCI H1), two-tier emission (`SiteAgg` aggregates + anomaly/1-in-N events),
  the `ExplainedAnomaly` capsule with PSI cause-attribution, RRD bounded history (fixed 4.5 MB),
  and the host-gauge consolidation onto `statvfs64` FFI. Its build plan W1a–W3 stands unchanged.
- **P25** `docs/design/BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md` — owns ALL
  concurrency/scheduling: the corrected host truth (4 physical cores × 2 SMT, not 8), the C/D/L
  work-class split, `kernel/src/admission.rs` (pure local admission fn over PSI/procfs), the
  LOCAL-DECISION and CORE-BOUND rules, and D_max=16. Its W1–W4 stand unchanged.

What P-H adds on top of them is exactly four things (§2–§5 below): the chaos harness, the
`ci.yml:23` fix, the benchmark CI gate, and the ledger migration. Where a chaos scenario touches
P24/P25 territory (F5, F6 in §2.3), this blueprint supplies only the *injection mechanism*; the
assertion stays owned by the sibling's own build plan (P25 W4, P24 §3.2 property tests).

---

## 1. Ground truth (re-verified this pass; two corrections to the Wave-1 audit)

| Fact | Cite (live) |
|---|---|
| CI bug: `bash selftest-telemetry.sh` invoked, file does not exist (`ls tools/telemetry/*.sh` = `governance.sh lib.sh report.sh` only) | `.github/workflows/ci.yml:23` |
| The real self-test is a dispatcher subcommand: `selftest)` → `TELEMETRY_NO_TG=1 log_event selftest … && echo "selftest: local JSONL write OK"` | `tools/telemetry/telemetry:472-474`; executable bit set (`-rwx--x--x`), bash shebang `telemetry:1` |
| Drift-gate exists and is wired into the commit path: `commit_after_decide_drift_gate` runs `classify_drift` (spectral ρ check) BEFORE `decide`; Unstable ⇒ `CommitError::Rejected` pre-persist; `intervention == true` lifts it | `kernel/src/event_log.rs:389-419`; consumer `kernel/src/hydra.rs:244` |
| Event log is content-addressed: id = sha3(prev, actor_pubkey, actor_seq, payload); replay = structural no-op | `kernel/src/event_log.rs:1-23` |
| The store seam is a trait: `EventStore { contains, insert -> Result<(), StoreError>, set_tip, … }` | `kernel/src/event_log.rs:182-204` |
| **Audit correction 1 — a fault-injection *precedent* DOES exist** (the audit's grep for `chaos\|fault_inject\|failpoint` missed it by name): `FaultyStore`, a `#[cfg(test)]` store double whose `insert` always returns `Err(StoreError::Sync)`, with a genuine RED-first test proving no fabricated `Committed`, no tip/len advance | `kernel/src/event_log.rs:440-459` (double), `:694-718` (test `append_over_faulty_store_surfaces_err_not_fake_committed`). It is a single always-fail double, not a harness — the audit's "no harness" verdict stands; §2 generalizes this exact pattern rather than inventing a new one (standard §2 item 19) |
| Spool is a pure crash-safe claim/ack state machine: `append` (backpressure `None` on full), `claim_next` (strict FIFO), `ack`, `reclaim` | `kernel/src/spool.rs:70-111` |
| TokenBucket uses `Mutex<Inner>` with `.lock().unwrap()` at both entry points — a panic while the lock is held poisons the mutex and every later caller panics (predicted chaos finding, §2.4 A6) | `kernel/src/token_bucket.rs:65,77` |
| Deterministic seedable PRNG already in-kernel: SplitMix64 → PCG64, bit-identical across runs/platforms, reference-vector tested | `kernel/src/rng.rs:1-15` |
| Criterion wired; **5 bench IDs** (not 4 as the audit counted): `place_order/5_items`, `fold_transitions/5_hops`, `empirical_identify/20k_samples`, `empirical_identify/end_to_end_20k`, `token_bucket/try_acquire_permit` | `kernel/benches/criterion.rs:12,60,76,80,91` |
| Committed baseline covers only 2 of 5 IDs | `kernel/benches/baseline.json` (2 keys: `fold_transitions/5_hops`, `place_order/5_items`) |
| **Audit correction 2 — `BENCH_HISTORY.md` is NOT committed** (`git check-ignore` exit 0); committed artifacts are `baseline.json`, `BENCH_RESULTS.md`, `bench_track.py`, `criterion.rs`, `.gitignore` | `git ls-files kernel/benches/` |
| Native baseline-diff tracker: `native-trackers bench <crate-dir> [--threshold N]` — runs cargo bench, parses text, compares to `benches/baseline.json`, auto-seeds missing IDs, exit 1 on regression | `tools/telemetry/native-trackers/src/main.rs:14,21,143-214` |
| Portable fallback delegates to the native binary | `kernel/benches/bench_track.py:1-54` |
| No `verify_chain`-style read-back integrity walk exists in the event log (grep `verify` = 0 hits) — §2.3 F2 adds one **(proposal)** | `kernel/src/event_log.rs` |
| Kernel features today: `default = ["std"]`, `std`, `wasm` — no `chaos` feature yet | `kernel/Cargo.toml:11-17` |
| Regression ledger: 25 physical rows, **21 distinct IDs** (IDs 7, 9, 10, 11 each used twice — a numbering defect the migration fixes); 3 rows reference living tooling (18, 20, 21), 22 reference deleted JS-era paths | `docs/regressions/REGRESSION-LEDGER.md:27-95` |
| Ratchet rule + reversal log stand and are kept verbatim | `REGRESSION-LEDGER.md:7-22,97-100` |
| CI `cargo-test` job runs kernel + engine suites offline, unconditional | `.github/workflows/ci.yml:106-120` |
| CI push triggers are narrow: `main` + `feat/kernel-fsm-graph-analysis` only; feature branches gate via PR only | `.github/workflows/ci.yml:8-9` |

---

## 2. THE CHAOS / FAULT-INJECTION HARNESS (headline build — designed fresh, seeded by `FaultyStore`)

### 2.1 Design rules (derived, each traceable)

1. **Zero external dependencies.** No `fail`/`failpoints` crate (crates.io 403 is live-documented,
   P24 §6 row 1; zero-dep is standing law). Everything below is `core`/`std` only.
2. **Deterministic and seeded.** Every injection schedule is a pure function of a named `u64` seed
   through the existing `kernel/src/rng.rs` SplitMix64→PCG64 stream (`rng.rs:1-15`). A chaos
   failure reproduces bit-identically from `(seed, plan)`. No wall-clock, no real sleep, no real
   network — CI flakes structurally cannot masquerade as chaos findings (Hermetic P6,
   Cause-and-Effect: no effect escapes its declared cause).
3. **Compiled out of production.** The whole module is gated
   `#[cfg(any(test, feature = "chaos"))]`; the injection macro compiles to `()` otherwise. The
   unsafe state "chaos machinery reachable in a release artifact" is unrepresentable at the
   compilation boundary, not policed by a runtime flag (standard §2 item 6 — reachability argued
   from structure; same class of argument as P24's no-`compare_exchange` grep guard).
4. **One mechanism, two seams — never per-module ad-hoc doubles** (Hermetic P2, Correspondence:
   one concept, one primitive). Seam A: trait-boundary decorators (`ChaosStore<S: EventStore>` —
   the `FaultyStore` generalization). Seam B: inline `chaos_point!(ChaosSite::…)` for code with no
   trait seam. `FaultyStore` itself is migrated to `ChaosStore` with an always-fail plan, so the
   repo ends with one injection authority, not two (`event_log.rs:440-459` absorbed, tests kept).
5. **Every fault test asserts the return swing, not only the break** (Hermetic P5, Rhythm): after
   the injected fault, the same suite proves recovery (re-commit succeeds, reclaim returns the
   record, the gate re-opens). A harness that only proves breakage is half-wired.

### 2.2 Predefined types (standard §2 item 4 — spec precedes test precedes code) **(proposal)**

New module `kernel/src/chaos.rs`, declared in `lib.rs` as
`#[cfg(any(test, feature = "chaos"))] pub mod chaos;` and one line in `kernel/Cargo.toml`
`[features]`: `chaos = []`.

```rust
/// Closed set of injection points. Adding a variant is a spec change reviewed
/// against this blueprint (F32 closed-set discipline, same as P24's site table —
/// distinct table: these are INJECTION points, P24's are MEASUREMENT sites).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChaosSite {
    StoreInsert,            // inside ChaosStore::insert (seam A)
    BetweenDecideAndInsert, // event_log commit path, after decide Ok, before store.insert (seam B)
    SpoolConsumerWork,      // between claim_next and ack (seam B, test-driver level)
    TokenBucketCritical,    // inside the Mutex critical section (seam B)
}

/// Closed enum of injectable faults. THE deliverable type of this phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FaultInjection {
    /// F1 — durability barrier fails: insert returns Err(StoreError::Sync).
    StoreSyncFail,
    /// F2 — corrupted state: persist a copy with payload byte `byte_index`
    /// XOR'd by `xor_mask` (deterministic single-bit/byte flip).
    CorruptPayload { xor_mask: u8, byte_index: usize },
    /// F3 — delayed response: consumer holds a claim for `virtual_ms` of
    /// MOCK time (no real sleep) before ack/crash — drives reclaim paths.
    DelayResponse { virtual_ms: u64 },
    /// F4 — forced panic mid-transaction at the armed site.
    PanicMidTransaction,
}

/// When a scheduled fault fires. Deterministic; Probability draws from the
/// seeded PCG64 stream, never from OS entropy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Trigger { OnCall(u32), EveryNth(u32), Always, Probability(f64) }

/// A deterministic injection schedule. `seed` fully determines Probability
/// draws; `arms` is consulted per (site, call-count).
pub struct FaultPlan {
    pub seed: u64,
    pub arms: Vec<(ChaosSite, FaultInjection, Trigger)>,
}
impl FaultPlan {
    pub fn none() -> Self;                       // empty plan — chaos_point! is inert
    pub fn fire(&mut self, site: ChaosSite) -> Option<FaultInjection>; // pure, counts calls
}

/// Seam A: the FaultyStore generalization. Wraps ANY EventStore; consults the
/// plan at ChaosSite::StoreInsert. Records `insert_calls: u32` so ORDERING
/// properties are falsifiable (see A1: drift-reject ⇒ insert_calls == 0).
pub struct ChaosStore<S: EventStore> { pub inner: S, pub plan: FaultPlan, pub insert_calls: u32 }
impl<S: EventStore> EventStore for ChaosStore<S> { /* delegates; injects per plan */ }

/// Seam B: inline injection point. Compiles to `()` unless cfg(any(test, feature="chaos")).
/// In chaos builds, consults a thread-local Option<FaultPlan> (thread-local ⇒
/// parallel `cargo test` threads cannot cross-inject — the bulkhead, §2 item 11).
macro_rules! chaos_point { ($site:expr) => { /* … */ } }
```

Scaling axis (standard §2 item 8): `ChaosSite` is a closed enum (change point: if injection points
ever exceed what one enum sensibly holds, migrate to a `u16` id table mirroring P24's site table —
same discipline, stated now). `FaultPlan::arms` is O(sites × variants), trivially small forever.

### 2.3 The four concrete injection types — exact mechanism each

**F1 — `StoreSyncFail` (dropped write / failed durability barrier).**
Mechanism: `ChaosStore::insert` consults the plan at `ChaosSite::StoreInsert`; on fire it returns
`Err(StoreError::Sync("chaos: injected".into()))` WITHOUT calling `inner.insert`. Trigger
`OnCall(n)` covers the case `FaultyStore` cannot express: **partial-success sequences** (k commits
succeed, the (k+1)th write is lost — the real disk-full/reordered-fsync shape), then recovery.
Attacks: `EventLog::append` and `commit_after_decide` (`event_log.rs:339`). Existing test
`:694-718` becomes plan `(StoreInsert, StoreSyncFail, Always)` — kept green, one authority.

**F2 — `CorruptPayload` (corrupted state at rest).**
Mechanism: `ChaosStore::insert` stores a copy of the event whose payload byte `byte_index` is
XOR'd with `xor_mask` (deterministic bit-flip), while the content-id passed in stays the one
computed from the UNcorrupted payload — modeling corruption between hash and persist (torn write,
bad sector). Detection requires the read-back walk that does not exist yet (ground truth §1):
**(proposal)** `EventLog::verify_chain(&self) -> Result<(), ChainDefect>` — walk the store from
tip via `prev`, recompute sha3 per event (`event_log.rs:30` `sha3_256`), return the first
id/content mismatch as typed `ChainDefect { at: [u8;32], kind: HashMismatch | BrokenPrev }`.
RED→GREEN is exact: the corrupted-store fixture makes `verify_chain` the only observer that can
go RED; without F2 the defect class is invisible (that invisibility is the RED, same argument
shape as `:694-698`'s "inexpressible on the pre-fix signature").

**F3 — `DelayResponse` (delayed response / stalled consumer).**
Mechanism: virtual time only — no `sleep`, no `Instant` manipulation. The chaos test driver claims
a record (`spool.rs:88`), then per plan holds it for `virtual_ms` of mock time (a plain `u64`
clock the driver advances), during which the producer keeps appending and a timeout policy
declares the claim stuck; the driver then either crashes the consumer (drop without ack) or acks
late. Attacks: `Spool::reclaim` (`spool.rs:109`), backpressure `append -> None` under a stalled
consumer (`spool.rs:70-73`), and — once P24 W2a exists — the drainer-lag observable
(`drainer_lag_ms` in the capsule, P24 §4.2). Asserted invariants: strict-FIFO preserved across
reclaim; no record lost; a late ack after reclaim is a clean `false` (no double-delivery
accounting); backpressure engages at exactly `capacity` and releases after drain.

**F4 — `PanicMidTransaction` (forced panic mid-commit).**
Mechanism: one `chaos_point!(ChaosSite::BetweenDecideAndInsert)` inserted in
`commit_after_decide` between the `decide` closure returning `Ok` and `store.insert` — in normal
builds the macro is `()` (zero cost, zero symbols); in chaos builds, plan-armed
`PanicMidTransaction` executes `panic!("chaos: F4 {site:?}")`. The test wraps the commit in
`std::panic::catch_unwind` (std, zero-dep) and asserts: tip unchanged, len unchanged, and — the
return swing — re-committing the SAME event afterward succeeds and yields the identical
content-id (idempotency by content-addressing, `event_log.rs:5-7`). A second armed site,
`ChaosSite::TokenBucketCritical` inside the `Mutex` critical section
(`token_bucket.rs:65`), drives scenario A6 below.

**Seam extensions (mechanism here, assertions owned by the sibling — NOT re-derived):**
- **F5 — gauge saturation/absence**: a `Gauges` fixture constructor
  (`chaos::saturated_gauges(psi_cpu, psi_mem)` / `chaos::unreadable_gauges()`) feeding P25's
  `admission.rs`. The admission assertions (C-defer under `psi_cpu_some_avg10 = 20`, D-admit under
  the same, gauge-unreadable ⇒ C-defer/D-floor) are **P25 W3/W4's own done-checks** — this phase
  only standardizes the fixture type they inject.
- **F6 — drainer death / ring flood**: once P24 W1a lands, a chaos scenario kills the drainer
  thread and floods the producer; the assertions (drop counter exact, seq gap == recorded drops,
  aggregates keep counting) are **P24 §3.2's own property tests** plus its §3.3 degradation
  contract — this phase adds only the kill/flood driver.

### 2.4 The adversarial suite — worst-case scenarios the system must survive (standard §2 item 5)

These tests ARE the harness deliverable. Each is a permanent named regression test
(`kernel/src/chaos.rs` `#[cfg(test)] mod adversarial`), event-sequence-asserted (§2 item 3), each
with a stated RED arm. Per the operator's directive these are designed to break the invariant, and
the suite passes only when the invariant holds anyway.

| # | Scenario | Injection | Invariant proven (falsifiable assertion) | RED arm |
|---|---|---|---|---|
| A1 | **Fault mid-decide-fold under the drift-gate** (cross-ref P-C, math-based safety) | Unstable adjacency (ρ>1) fixture + `(StoreInsert, StoreSyncFail, Always)` simultaneously | `commit_after_decide_drift_gate` (`event_log.rs:389`) rejects with `CommitError::Rejected` AND `ChaosStore.insert_calls == 0` — the spectral gate fires BEFORE decide and BEFORE any store touch; the ordering is observed, not assumed. With `intervention=true` the gate lifts and the injected sync-fail surfaces as `Err(StoreError::Sync)` — never a fabricated commit in either regime | reorder the gate after `decide` (or stub `insert_calls` tracking away) → assertion inexpressible/fails |
| A2 | **Panic mid-commit, then recovery** (Snapshot Re-entry, §2 item 13: cheap regenerative recovery from the last valid tip — a math property of content-addressing, not a supervisor) | F4 at `BetweenDecideAndInsert`, `OnCall(1)` | post-panic: tip/len unchanged; re-commit of the same event succeeds with the identical content-id; a replay of an ALREADY-committed event is `AppendOutcome`-idempotent (structural no-op, `event_log.rs:6-7`) | remove the short-circuit ordering (insert before decide) → tip advances on a panicked transaction |
| A3 | **Silent corruption detection** | F2, `byte_index=0, xor_mask=0x01`, `OnCall(2)` of 3 commits | `verify_chain` returns `Err(ChainDefect::HashMismatch { at })` naming exactly the corrupted event; on the uncorrupted twin store it returns `Ok(())` | run the same fixture without `verify_chain` — no observer goes red (the pre-fix blindness IS the RED, documented in the test comment) |
| A4 | **Crash-storm on the spool** | F3 driver: seeded plan interleaves claim/crash/reclaim/late-ack/append across 1 000 records, `Probability(0.3)` crash per claim, seed named in the test | zero loss (every id eventually acked exactly once), strict FIFO among un-acked, late-ack-after-reclaim returns `false`, `append` returns `None` at exactly `capacity` and recovers after drain | weaken `reclaim` to drop instead of re-queue → loss counter ≠ 0 |
| A5 | **Sustained disk-full (degrade-closed)** | F1 `Always` for 10 000 appends | every append is `Err(StoreError::Sync)`; NEVER `Ok(Committed)`; `len() == 0` throughout (no unbounded in-memory buffering of failed writes) — Hermetic P4 (Polarity): the collapse is typed and safe-directed, the log refuses rather than lies | the pre-`FaultyStore`-fix infallible signature (historical RED, `:694-700`) |
| A6 | **Poisoned-lock cascade** (predicted REAL finding) | F4 at `TokenBucketCritical`, `OnCall(1)`, then further `try_acquire` calls from the test thread | **Predicted RED against current code**: `token_bucket.rs:65,77` `.lock().unwrap()` — after one panic inside the critical section every subsequent `try_acquire` panics (denial-of-service by poison-cascade). GREEN requires the fix this test forces: recover the mutex (`unwrap_or_else(|p| p.into_inner())` — safe here because `Inner` is two POD fields with no invariant spanning the panic point) or degrade-closed deny. Fix choice recorded in the ledger row on landing | current code IS the red arm |

Bulkhead (standard §2 item 11): all injection state is thread-local + per-instance
(`ChaosStore.plan`); the blast radius of any chaos test is its own test thread — concurrent
`cargo test` lanes cannot cross-contaminate, by construction. Mesh awareness (item 12): this
harness is node-local, in-process only; transport-layer chaos (iroh partition/latency) is
explicitly OUT of P-H scope and named as P-E territory. Living memory (item 15): chaos findings
land as ledger rows + capsule ledger entries (append-only JSONL convention, P24 §1.2), never a
new storage shape.

---

## 3. CI FIX — `ci.yml:23` (exact invocation)

Replace the broken step (`.github/workflows/ci.yml:19-23`) verbatim:

```yaml
      - name: Run telemetry self-test (local-only, no Telegram secret)
        run: |
          cd tools/telemetry
          chmod +x telemetry *.sh
          TELEMETRY_NO_TG=1 ./telemetry selftest
```

- `./telemetry selftest` is the real entry point (`tools/telemetry/telemetry:472-474`); it already
  self-scopes to local JSONL (`TELEMETRY_NO_TG=1` on its own `log_event`); the outer env var
  matches the sibling health step's existing pattern (`ci.yml:28`).
- `chmod +x telemetry` is belt-and-braces (the execute bit is committed, verified `-rwx--x--x`).
- RED→GREEN (§2 item 2): RED = the current step — `bash selftest-telemetry.sh` exits 127
  (`bash: selftest-telemetry.sh: No such file or directory`), reproducible locally in one command.
  GREEN = the replacement exits 0 printing `selftest: local JSONL write OK (…/selftest.jsonl)`.

---

## 4. CI-GATING THE BENCHMARKS — with one honesty correction to the Wave-1 audit

**The audit's implied plan ("run `native-trackers bench kernel --threshold N` in CI") has a flaw
this blueprint corrects:** `kernel/benches/baseline.json`'s numbers were measured on THIS Hetzner
host (e.g. `place_order/5_items` 90.4 ns). A GitHub shared runner has a different (and
run-to-run variable) constant factor; gating an absolute host-measured baseline on a foreign
runner produces false REDs at any threshold, or requires a threshold so wide it gates nothing.
The gate must be **same-runner-relative**; the absolute baseline remains the *local/cron*
tracker's job, which is exactly what it was built for.

### 4.1 New job `bench-regression` (same-runner A/B via criterion's built-in baselines) **(proposal)**

Add to `.github/workflows/ci.yml` (runs on `pull_request` only — on a push to `main` there is no
merge-base delta to measure). Named constant: `BENCH_NOISE_CI = 0.10` (10 % — generous for shared
runners; catches the order-of-magnitude class: accidental O(n²), debug-build leakage, alloc storms).

```yaml
  bench-regression:
    name: bench regression gate (same-runner A/B vs merge-base)
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Pre-fetch crates offline (no network in CI — BLUEPRINT-P01 §2.1 discipline)
        run: cargo fetch --manifest-path kernel/Cargo.toml
      - name: Bench merge-base (criterion saved baseline "base")
        run: |
          git checkout $(git merge-base origin/${{ github.base_ref }} HEAD)
          cargo bench --offline --manifest-path kernel/Cargo.toml -- --save-baseline base --noise-threshold 0.10
      - name: Bench HEAD vs "base" — fail on regression beyond noise threshold
        run: |
          git checkout -
          cargo bench --offline --manifest-path kernel/Cargo.toml -- --baseline base --noise-threshold 0.10 | tee bench-out.txt
          ! grep -q "Performance has regressed" bench-out.txt
```

Both runs share one runner, one thermal/CPU context — the comparison is honest. Cost: ~2× bench
wall time, PR-only. The job carries the standard SCOPE RULE banner every ci.yml job carries
(`ci.yml:40-48`), verbatim.

### 4.2 The absolute tracker stays local/scheduled — and gets its baseline refreshed

- **Baseline refresh (required, small):** run `cargo bench` once on this host, then commit
  `kernel/benches/baseline.json` covering **all 5** bench IDs (§1 — currently 2 of 5; the
  auto-seed at `native-trackers/src/main.rs:200-214` writes locally but an uncommitted seed means
  3 IDs are never actually guarded anywhere).
- **Scheduled full run:** a cron/scheduled invocation of
  `native-trackers bench kernel --threshold 10` on this host (exit 1 on >10 % regression vs the
  committed host baseline, `main.rs:21`), with its output logged through the existing
  `bench_run`/ledger convention (`tools/telemetry/lib.sh:168-194`). This is the tight gate; CI's
  10 %-noise A/B is the coarse one. `BENCH_HISTORY.md` stays the gitignored local trend
  (ground-truth correction §1).
- **Smoke in CI (cheap):** `python3 kernel/benches/bench_track.py --no-run` as a step in the
  existing `cargo-test` job — validates that `baseline.json` parses and self-compares clean
  (`bench_track.py:27`), so a malformed/truncated baseline fails fast without running benches.
- **Engine benches:** `engine/` has no criterion harness (audit Area 3). Optional follow-up, in
  scope only when an engine hot path gets a named budget: one `engine/benches/criterion.rs` with
  `field_frame::step` (the P11 §3 allocation-free path) — flagged, not mandated (YAGNI unless the
  perf-priority directive names it).
- RED→GREEN for the gate itself (§2 item 2): RED = plant `std::thread::sleep(Duration::from_millis(1))`
  inside `bench_place_order`'s iter closure on a scratch branch → A/B job fails on "Performance has
  regressed", and locally `native-trackers bench kernel` exits 1. GREEN = remove the plant; both pass.
  (The plant is the standard §2 item 5 "intentionally-failing" arm for this section.)

---

## 5. REGRESSION-LEDGER PRUNE/MIGRATE (not a rewrite — the schema and ratchet rule are kept verbatim)

Per the Wave-1 Area-2 verdict and the living-memory safe-apply rule (move-not-delete): the ledger
file keeps its header, ratchet rule (`REGRESSION-LEDGER.md:7-22`), and reversal log; the table is
split into a live section and an archived legacy section. Exact dispositions, all 25 physical rows
(21 distinct IDs; duplicated IDs 7/9/10/11 get `a`/`b` suffixes during the move — fixing the
numbering defect is part of the migration):

**LIVE (3 rows, stay in the main table):**
- **18** (Markov attractor) — `tools/loop-signals/` exists and is wired; unchanged.
- **20** (`tools/verify-scope.sh`) — file exists (verified); action: prune its dead eslint/pnpm
  routing branches (they reference scopes deleted by row 21) so the guardrail matches reality.
  One edit, noted in the row.
- **21** (legacy thin-layer removal, structural CI-gate) — the kernel-era anchor row; unchanged.

**ARCHIVE — "Legacy (pre-kernel) — guarded code deleted 2026-07-13 (row 21)" section (19 rows,
moved verbatim, no content edits):** 1 (dev-login backdoor), 2 (insecure-random eslint),
3 (fly release_command), 4 (pgboss grant), 5 (no-direct-websocket), 6 (contrast),
7a (cart reconcile), 8 (i18n hardcoded), 9a (i18n parity), 10a (arbitrary tailwind),
11a (visual net), 11b (permissive assertions), 12 (safeStorage chaos-monkey), 13 (CSS comment),
14 (behavioural invariants), 15 (menu-load pool), 16 (owner revocation), 17 (fee parity),
19 (pre-commit pnpm routing — superseded in place by row 21's rewrite; archived with a pointer).

**ARCHIVE with a named kernel-era heir (3 rows — the bug CLASS outlives the dead guardrail;
each heir is a stated obligation on the owning phase, so nothing silently evaporates):**
- **7b** (money float drift) → class structurally absorbed: kernel money is integer-typed
  (`kernel/src/money.rs`, `engine/src/money_guard.rs`); heir row lands with P-G's money
  dual-authority flip (explicitly operator-gated, standard §3 P-G).
- **9b** (raw-SQL interpolation) → no SQL layer exists at HEAD; heir obligation attaches to the
  pgrust store work (P-B `PgEventStore` seam, `event_log.rs:9-20`).
- **10b** (cross-tenant IDOR) → red-line authz class; heir obligation attaches to P-G's product
  rebuild DoD.
- **12**'s heir is THIS blueprint: the JS chaos-monkey's kernel-native successor is §2's harness —
  the archive row gets a forward pointer to the new chaos rows.

**NEW rows added by P-H itself (each with its red→green proof, per the standing rule at `:9`):**
(i) `ci.yml:23` selftest fix (§3, guardrail type `CI-gate`); (ii) bench regression gate (§4,
new guardrail type `bench-gate` added to the taxonomy line at `:19-22`); (iii) one row per landed
chaos scenario A1–A6 (new guardrail type `chaos`), A6 additionally recording the TokenBucket
poison fix. Also append to the taxonomy line: `cargo-test`, `grep-CI-gate` (the live kernel-era
analogues the audit named).

**Named residual (Ananke — honest, not fake-enforced):** the ratchet rule remains convention;
no machine check forces a ledger row per fix. A commit-message/paths heuristic gate would
false-positive constantly; this blueprint declines to build enforcement theater and records the
gap instead (same posture as P24 §9's adoption residual).

---

## 6. Build plan — DoD, RED→GREEN, wave-classed per P25 (standard §2 items 2, 3, 10)

All units below are **C-class** under P25 §3.3 (local `cargo test`/`cargo bench` done-checks —
Σ threads ≤ 4 strict-core slots, `nice 10`); the doc/ledger edits are D-class trivia. W-H1/2/3
are mutually independent lanes (different files, no shared mutable state); W-H4 depends on W-H1.

| # | Unit | Files | Falsifiable done-check |
|---|---|---|---|
| W-H1 | `chaos.rs` module: types (§2.2), `ChaosStore`, `chaos_point!`, `FaultPlan` + migrate `FaultyStore` | `kernel/src/chaos.rs` (new), `kernel/src/lib.rs` (+1 cfg-gated line), `kernel/Cargo.toml` (+`chaos = []`), `kernel/src/event_log.rs` (FaultyStore → ChaosStore, tests kept green) | `cargo test -p dowiz-kernel chaos::` green; the migrated `:694-718` test still green; `cargo build --release -p dowiz-kernel` (no features) contains no chaos symbols — CI grep guard: `grep -n 'cfg(any(test, feature = "chaos"))' kernel/src/lib.rs` non-empty (structural, P24-grep-guard style) |
| W-H2 | `ci.yml:23` fix (§3) | `.github/workflows/ci.yml` | local RED/GREEN commands from §3; CI `telemetry-selftest` job green on the next PR |
| W-H3 | Bench gate: `bench-regression` job + baseline refresh to 5 IDs + `--no-run` smoke step (§4) | `.github/workflows/ci.yml`, `kernel/benches/baseline.json` | the §4.2 sleep-plant RED arm demonstrated once on a scratch branch (both the A/B job and the local tracker go red), then removed; `baseline.json` has 5 keys; `bench_track.py --no-run` exit 0 |
| W-H4 | Adversarial suite A1–A6 (§2.4) incl. `verify_chain` **(proposal)** and the A6 TokenBucket poison fix | `kernel/src/chaos.rs`, `kernel/src/event_log.rs` (+`verify_chain`), `kernel/src/token_bucket.rs` (A6 fix), `kernel/src/spool.rs` (test-only driver) | each A-row's stated assertion green AND its stated RED arm demonstrated (A6's RED is current HEAD — run it pre-fix, record the panic, then fix); all seeds named constants in the test file |
| W-H5 | Ledger migration (§5) + new rows for W-H1..4 | `docs/regressions/REGRESSION-LEDGER.md`, `tools/verify-scope.sh` (row-20 prune) | live table = 3 + new rows only; archive section headed as §5; no row deleted (`git diff` shows moves + suffix renames, zero content loss); every W-H unit that changed behavior has its row before "done" (`:9`) |

Telemetry hook for the phase itself (standard §2 item 10): W-H3's scheduled tracker run reports
through the existing `bench_run` ledger convention (`lib.sh:168-194`) — regressions surface
automatically, not at review time. Once P24 W2a lands, chaos-run outcomes emit through the same
capsule ledger; nothing new is built for that (P24 owns it).

---

## 7. Standard-compliance map (all 20 points, where each is satisfied)

1 ground truth → §1 (2 audit corrections found live) · 2 DoD → §6 · 3 spec/event-driven TDD →
§2.2 types precede §2.4 tests precede code; A-rows assert event sequences/orderings ·
4 predefined types → §2.2 · 5 adversarial incl. intentionally-failing → §2.4 (every row has a RED
arm; A6's RED is live HEAD) + §4.2's sleep-plant · 6 hazard-safety from structure → §2.1 rules
2–3 (cfg-compilation boundary; determinism) + A1's observed gate ordering · 7 links → §0, §1, §5
(P24, P25, P-C via A1, P-B/P-G heirs, Wave-1 audit, HERMETIC-ARCHITECTURE-PRINCIPLES) · 8 scaling
axes → §2.2 (ChaosSite change point), §5 (live/archive split caps the live table) · 9 Linux
discipline → inherited via P24 §2 (this phase adds no new OS-facing mechanism; the
BLUEPRINT-LINUX-ENGINEERING verdict framework applies through P24, reused not re-derived) ·
10 bench+telemetry → §4, §6 hook · 11 bulkhead → §2.4 (thread-local plans) · 12 mesh awareness →
§2.4 (node-local; transport chaos = P-E) · 13 rollback/self-healing as math → A2 (Snapshot
Re-entry via content-addressed idempotent re-commit), A5 (self-termination as typed refusal) ·
14 smart index → grep guards (W-H1 cfg-gate, P24's compare_exchange guard referenced) + the bench
gate turning perf bugs into CI-time failures · 15 living memory → §5 archive tiering, append-only
ledgers · 16 tensor/spectral → A1 exercises `classify_drift` (`spectral.rs` via
`event_log.rs:402`); no new math invented · 17 regression tracking → §5 new rows, permanent named
tests · 18 agent-executable → every unit names exact files, commands, and observable outcomes ·
19 reuse-first → §0 (P24/P25 by reference), §2.1 rule 4 (FaultyStore generalized not duplicated),
§4 (criterion/native-trackers extended not rebuilt) · 20 Hermetic citations → §2.1 (P2, P5, P6),
§2.4 A5 (P4), plus: the RED-arm requirement on every test is P7 (Gender — no self-certified
green: a test must be seen to fail before its pass counts); rates asserted against P24/P25's
named constants, never new ones, is P3 (Vibration — single-authority rates).

## 8. 2-question doubt audit

**Q1 — least confident (concrete):** (1) criterion's `--noise-threshold` behavior on shared
runners is designed-from-docs, not yet measured on an actual GitHub runner pair-run — if the A/B
job flakes at 0.10, the named tunable moves, the mechanism stands. (2) `verify_chain` assumes the
store can iterate/fetch by id walking `prev`; if the `EventStore` trait needs a `get(&id)` method
added for it, that is a trait extension the sibling P-B (store/consistency) should sign off —
flagged as a one-method seam, not silently added. (3) The A6 fix choice (`into_inner` recovery vs
degrade-closed deny) is left to the RED evidence at fix time; both are safe for this Inner (two
POD fields), but the ledger row must record which and why.

**Q2 — biggest thing possibly missed:** cross-process chaos. Everything here is in-process
(`cargo test`); the real crash-reclaim story of `rust-spool`'s file-backed queue (the A1-CRITICAL
head-of-line wedge, P24 audit addendum §2) involves kill -9 of a real process — a class this
harness models only at the state-machine level. Named as the follow-up seam (a `chaos` subcommand
in `native-trackers` driving real process kills), deliberately out of v1 to keep the harness
deterministic; the state-machine-level A4 covers the invariant logic, not the OS-level durability.

---

---

## 9. Session fold-in (2026-07-18) — GitHub hygiene + versioning land in Layer H

Added after the Wave-2 writing pass; §0–§8 stand. This layer's §6 already lists ledger rows and CI
jobs as its deliverables; the 2026-07-18 session added a **repo-hygiene** dimension that belongs
here (ops), routed from `docs/repo-maintenance-2026-07-17/GITHUB-MAINTENANCE-AUDIT-dowiz.md`. These
are ops/CI items, not code — an operator-gated docket (the tag/branch actions need a human and a
broadened token) plus the versioning scheme this layer owns. Landing status of the four §6 build
items (W-H1..W-H5) is also refreshed here from the session synthesis.

### 9.1 Landing status refresh (from `ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md` §0)

The P-H build largely **landed**: the chaos harness + A1–A6 adversarial suite + the bench-regression
CI gate are on `main` (`f4802927e` / `a952af354`, per the synthesis's "+41 commits" wave list; this
worktree's own log shows `a952af354 feat(kernel,P-H): W-H3 bench-regression CI gate + W-H5 ledger
migration`). One consequence worth surfacing for this layer specifically: **the bench baseline and
the 452-test count in `GROUND-TRUTH-2026-07-17.md` are now stale** — GROUND-TRUTH anchors
`origin/main = 9f78b91d5`, but live `main` is `87da9ccd4` (+41 commits), and because so many
test-adding waves landed, the **452 figure must be re-run, not trusted** (this is exactly the
regression-surface hygiene this layer is responsible for; the DoD's "re-verify the live count against
git, do not trust the remembered number" rule, §6, applies to GROUND-TRUTH itself now).

### 9.2 GitHub hygiene docket (Layer H ops · Layer I docs) — GH-tag, operator-gated

The audit found the remote in a state no release-engineering layer should tolerate; the concrete
target state and its blockers:

| Item | Finding (audit) | Layer-H action |
|---|---|---|
| **Zero tags / zero releases on the remote** | the 5 local tags are all defensive backup markers, none pushed; GitHub sees no version history at all | cut the first annotated tag **`2026.07.0`** on `main`; one GitHub Release per tag thereafter |
| **Versioning scheme** | no `CHANGELOG`/`VERSION`; the project versions by date everywhere; SemVer `0.y.z` would carry no compat promise (no external consumer) | **CalVer `YYYY.MM.PATCH` for the repo/product tag** + an **independent in-code `KERNEL_PROTO_VERSION` / `MESH_WIRE_VERSION`** for the event-log/wire format (the two artifacts that DO need real compat semantics — independent nodes must interoperate). Seed `CHANGELOG.md` (Keep a Changelog) from wave history |
| **PAT scope** | the fine-grained PAT cannot see the private `dowiz` repo (SSH works, API is blind) — so topics/description/Releases are unreadable via `gh` | **not fixable from inside CI** — operator broadens the PAT scope or uses a token that can see the repo. Flagged, not worked around |
| **61 remote branches, ~39% scratch/bot/backup/snapshot** | `recover/*` (13), `plane-maintainer/*` bot (6), `docs/plane-status-*` (3), `backup*` (2) — none meant to be permanent | archive-then-delete (bundle/tag first — move-not-delete rule); enable `deleteBranchOnMerge`; fix the `plane-maintainer` bot to stop leaving permanent branches. Per-branch deletion needs a human given concurrent work |
| **6 build-output dirs tracked in git** | `temp/`, `dogfood-output/`, `graphify-out/`, `qa-shots/`, `qa-onboarding-shots/`, `playwright-report/` — two already in `.gitignore` (committed before the ignore rule) | `git rm -r --cached` these paths; low risk, non-destructive to the working tree |
| **Stale Repowise index in `.claude/CLAUDE.md`** | indexed 2026-06-14; still lists the deleted `apps/`+`packages/` tree — any agent trusting it navigates a repo that no longer exists | re-index or annotate as superseded (Layer I docs overlap; recorded here because it is a CI/tooling-hygiene item) |

**Why this is Layer H and not just chores:** versioning the *wire/event-log format independently
of the product tag* is a correctness concern (a format-breaking change is a real breaking change
even with zero app consumers), and it is the same discipline as this layer's bench-baseline and
regression-ledger ownership — the repo's compatibility surface is an ops invariant. The tag/branch
*actions* are operator-gated (need a human + a broadened token, docket item **GH-tag** in the
session synthesis §4); the *scheme decision* (CalVer + in-code proto version) is Layer H's to
specify, which it does above.

### 9.3 Net effect

No change to §2–§6's four build items or their landing status beyond the refresh in 9.1. Layer H
gains a **repo-hygiene / versioning** sub-scope (GH-tag docket, operator-gated) and the standing
note that GROUND-TRUTH's test/commit anchors are stale and must be re-run — the latter being this
layer's own regression-surface responsibility applied reflexively to the roadmap's own numbers.

---

*Registered under P-H in `CORE-ROADMAP-STANDARD-2026-07-17.md` §3. Supersedes nothing; extends
the Wave-1 audit (two corrections, §1) and consumes P24/P25 by reference. Ledger rows land with
W-H5, not with this document. §9 folded in 2026-07-18 from the session GitHub-maintenance audit +
verification synthesis.*
