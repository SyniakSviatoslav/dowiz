# P-H Wave-1 Audit — Ops / Telemetry / Benchmarks / Regression (2026-07-17)

> "P-H" here = CORE-ROADMAP **Layer H** (ops / telemetry / benchmarks / regression), an altitude
> lens — NOT execution phase P-anything. Naming ruling per `CORE-ROADMAP-STANDARD-2026-07-17.md` §3
> and `P-I-audit-cross-repo-consolidation.md` §4.
>
> **Wave 1, Opus, read-only.** Ground-truth audit for CORE-ROADMAP layer **P-H**: inventory what
> ops/telemetry/benchmark/regression machinery already exists so the Wave-2 blueprint extends it
> rather than rebuilding, and confirm where the operator's "designed to literally break everything"
> directive (STANDARD §2 item 5) has nothing to reuse. Companion Wave-1 audits in this dir:
> `P-D-audit-root-delegation-policy.md`, `P-G-audit-product-ui-post-decommission.md`,
> `P-I-audit-cross-repo-consolidation.md`.

> **RECONSTRUCTION NOTE (2026-07-17).** The original of this audit was written on disk but lost
> before commit — a separate consolidation session merged ~20 `feat/*` branches onto `main` while
> this file existed only uncommitted (root cause confirmed; `BLUEPRINT-P-I-consolidation.md` §32 G6
> records it: "the dir is untracked — no git recovery path"). Its load-bearing content survived only
> as embedded quotes inside the Wave-2 blueprint (`BLUEPRINT-P-H-ops-telemetry.md`, which cites this
> audit's Areas 3–4). This document is a **full reconstruction with every citation re-verified fresh
> on current `main`** (`git rev-parse HEAD` → `caba2203c`), NOT inherited. Because `main` changed
> heavily since the Wave-2 blueprint was written (it pinned `feat/harness-llm-backend @ cc3d5c916`),
> **line numbers here differ from the blueprint's embedded cites** — this reconstruction supersedes
> those numbers where they diverge; §6 tabulates the deltas.

---

## Bottom line up front

1. **Headline gap holds: there is ZERO chaos / fault-injection *harness* anywhere in
   kernel/engine/tools.** `grep -rniE "chaos|fault_inject|failpoint|inject_(failure|fault|panic)"`
   over `kernel/ engine/ tools/` returns **three hits, all in `kernel/src/pq/fractal.rs`**, where
   "chaos" means the logistic-map dynamical-systems sense (`fractal.rs:1,9,46`) — a *derived-diversity*
   artifact, not fault injection. No `fault_inject`, no `failpoint`, no `inject_*`. This layer's
   headline deliverable is genuine greenfield.
2. **Correction to the greenfield framing (a precedent the grep missed by name):** `FaultyStore`
   at `kernel/src/event_log.rs:452-489` is a `#[cfg(test)]` `EventStore` double whose durability
   barrier ALWAYS fails (`insert` → `Err(StoreError::Sync("simulated fsync failure"))`), with a
   genuine RED-first test. It does not match the grep (it is `Faulty`, not `fault_inject`), which is
   exactly why the term-based search missed it. So the honest framing is **"one existing single-fault
   pattern to GENERALIZE into a harness, not build from absolute scratch"** — the Wave-2 blueprint
   must seed its harness from this shape (STANDARD §2 item 19, reuse-first), not invent a parallel one.
3. **One real bug found and re-confirmed live: `.github/workflows/ci.yml:23` calls
   `bash selftest-telemetry.sh`, which does not exist.** The real self-test is a dispatcher
   subcommand (`tools/telemetry/telemetry:472-474`, `./telemetry selftest`), not a standalone
   script. `ls tools/telemetry/*.sh` = `governance.sh lib.sh report.sh` only. This CI step exits 127
   on every `main` push / PR today. Still broken on current `main`.
4. **Mature substrate to EXTEND, not rebuild:** `tools/telemetry/lib.sh` (spool / `log_event` /
   `bench_run`), the native-ser / native-trackers / rust-spool Rust crates, and **criterion already
   wired** (`kernel/Cargo.toml`, 5 bench IDs) with a real baseline-diff tracker
   (`native-trackers bench` + `bench_track.py` + committed `baseline.json`).
5. **Systemic gap: nothing is CI-gated for benchmarks or anomalies — the whole regression-detection
   surface is manual / local / scheduled only.** The criterion harness runs by hand; the anomaly
   detector that exists (`ci-truth claim-latency`) appends a ledger but its *consumer* is deferred to
   Phase 8; no CI job fails on a perf regression.
6. **Regression ledger needs a prune/migrate pass, not a rewrite:** 25 physical rows / 21 distinct
   IDs, of which **all but 3 IDs (18, 20, 21) reference JS-era paths deleted by row 21** — the
   `apps/web` / `packages/ui` / `apps/api` / `e2e/` thin-layer decommissioned 2026-07-13. The header,
   ratchet rule, and reversal log are sound and stay verbatim.
7. **P24 and P25 are already-decided and own half this layer — do NOT re-derive them** (Area 4).

---

## 0. Verified current state (run fresh this pass)

| Fact | Verified value | Method |
|---|---|---|
| Branch / HEAD | `main` @ `caba2203c` ("docs: land CORE-ROADMAP Layer A-I execution structure") | `git branch --show-current`; `git rev-parse HEAD` |
| Kernel tests | 452 default (serde-free) + 107 `--features pq` (NIST-ACVP KAT) | `GROUND-TRUTH-2026-07-17.md:9` (this pass, not re-run) |
| Chaos/fault-injection **harness** | **none** — grep returns only `pq/fractal.rs` (dynamical-systems "chaos") | `grep -rniE "chaos\|fault_inject\|failpoint\|inject_(failure\|fault\|panic)" kernel/ engine/ tools/` |
| Fault-injection **precedent** (grep-invisible) | `FaultyStore` always-fail `EventStore` double + RED-first test | `kernel/src/event_log.rs:452-489`, test `:815` |
| CI telemetry bug | `bash selftest-telemetry.sh` invoked; file absent | `.github/workflows/ci.yml:23`; `find . -name selftest-telemetry.sh` → 0 |
| Real self-test entry point | `selftest)` → `TELEMETRY_NO_TG=1 log_event selftest … && echo "selftest: local JSONL write OK"` | `tools/telemetry/telemetry:472-474` (executable bit set, `-rwx--x--x`) |
| criterion wired | 5 bench IDs | `kernel/benches/criterion.rs:12,60,76,80,91` |
| committed baseline | 2 of 5 IDs | `kernel/benches/baseline.json` (`fold_transitions/5_hops`=5.59, `place_order/5_items`=90.4) |
| regression ledger | 25 physical rows / 21 distinct IDs; 3 live (18/20/21), 22 JS-era | `docs/regressions/REGRESSION-LEDGER.md:27-50,95` |

---

## 1. Method

Each Area below: (a) enumerate what exists with a live `file:line` cite verified this pass
(STANDARD §2 item 1, "ground truth is non-discussible"); (b) state the GAP against the operator's
directive; (c) name the existing pattern the Wave-2 blueprint should extend (item 19). Read-only —
no code written, no test run beyond the greps and reads cited. Nothing here is trusted from the
lost original or from the Wave-2 blueprint's embedded quotes; every number was re-derived.

---

## Area 1 — Chaos / fault-injection (the headline greenfield, seeded by one precedent)

### 1.1 The gap is real

The operator's directive — test cases "designed to literally break everything" (STANDARD §2 item
5) — has **no harness to reuse**. The term grep is empty of fault-injection machinery:

```
grep -rniE "chaos|fault_inject|failpoint|inject_(failure|fault|panic)" kernel/ engine/ tools/
  → kernel/src/pq/fractal.rs:1   //! Fractal / chaos artifacts over a shared secret — DERIVED …
  → kernel/src/pq/fractal.rs:9   //! … the chaos …
  → kernel/src/pq/fractal.rs:46  // r in (3.57, 4.0) for full chaos; derive from secret …
```

All three are the logistic map (`x → r·x·(1−x)`) used for anti-traffic-analysis diversity — a
number source, not a fault injector. There is no `fail`/`failpoints` crate (crates.io egress is
403-blocked, standing zero-dep law), no failpoint macro, no injection scheduler, no adversarial
crash suite. **Verdict: genuine greenfield for the harness itself.**

### 1.2 The correction: one precedent the grep missed by name

`FaultyStore` (`kernel/src/event_log.rs:452-489`) is the shape a harness generalizes:

- A `#[cfg(test)]` `EventStore` double (`struct FaultyStore { tip, count }`, `:463-466`) whose
  `insert` **always** returns `Err(StoreError::Sync("simulated fsync failure".into()))` (`:473`) —
  modelling a full disk / read-only mount / failed `fsync`.
- Its doc comment (`:452-461`) states the RED explicitly: this shape is *inexpressible against the
  pre-fix infallible `insert -> ()` trait*, so the impossibility itself is the red arm.
- `set_tip`/`len` track real state (`:485-488`) so the "no in-memory advance on a failed durability
  barrier" assertions are falsifiable, not tautological — a correct `append` short-circuits on the
  failed `insert?` before `set_tip`, keeping tip/count at their empty values.
- Consumed by a genuine RED-first test: `append_over_faulty_store_surfaces_err_not_fake_committed`
  (`:815`) plus the follow-on faulty-store tests (`:816`, `:839`, `:849`) — the exact
  "no fabricated `Committed`" invariant.

**This does NOT weaken the "no harness" verdict** (it is a single always-fail double, not a
seeded/scheduled injector across sites), but it corrects the framing from *absolute zero* to
*"one existing pattern to generalize."* The Wave-2 blueprint must **absorb `FaultyStore` into the
harness's trait-decorator seam** (an always-fail plan), ending with one injection authority, not
two — the grep-invisibility is itself the lesson: a term-based audit will keep missing it.

### 1.3 Building blocks a harness can reuse (all present, all zero-dep)

| Block | Cite | Why it matters to the harness |
|---|---|---|
| `EventStore` trait seam (`insert -> Result<(), StoreError>`, `set_tip`, `tip`, `contains`) | `kernel/src/event_log.rs:182-204` | the decorator boundary (`ChaosStore<S: EventStore>`) — `FaultyStore` generalized |
| Drift-gate commit path (`commit_after_decide_drift_gate` runs `classify_drift` BEFORE decide; `Unstable ⇒ CommitError::Rejected` pre-persist) | `kernel/src/event_log.rs:410-431` (gate), `:357` (plain path), `spectral.rs:325` (`classify_drift`) | the ordering an adversarial test can prove: gate fires before any store touch |
| Deterministic seedable PRNG (SplitMix64 → PCG64, reference-vector tested, bit-identical cross-platform) | `kernel/src/rng.rs:31-43` | seeded injection schedules → chaos findings reproduce from `(seed, plan)`, no wall-clock flake |
| Spool state machine (`append` w/ backpressure, `claim_next` strict-FIFO, `ack`, `reclaim`) | `kernel/src/spool.rs:70,88,98,109` | the crash-storm / stalled-consumer target |
| `TokenBucket` `Mutex<Inner>` with `.lock().unwrap()` at both entry points | `kernel/src/token_bucket.rs:29,65,77` | **a predicted real finding:** a panic while the lock is held poisons the mutex → every later `try_acquire` panics (poison-cascade DoS). A `PanicMidTransaction` injection at the critical section forces the fix (`into_inner` recovery or degrade-closed deny). Current code IS the red arm |
| Feature-gate precedent (`default=["std"]`, `std`, `wasm`) — no `chaos` feature yet | `kernel/Cargo.toml` `[features]` | the harness gates `#[cfg(any(test, feature = "chaos"))]` → unreachable in a release artifact by construction, not by runtime flag |

**Gap for Wave-2:** no read-back integrity walk exists (`grep verify_chain kernel/src/event_log.rs`
= 0) — a `CorruptPayload` injection needs a `verify_chain` observer added to make silent corruption
detectable; without it the corruption class has no red arm. Flag as a one-method seam for P-B
(store/consistency) sign-off, not a silent trait extension.

---

## Area 2 — Regression ledger (prune/migrate, keep the schema)

`docs/regressions/REGRESSION-LEDGER.md` is a **Tier-1 regression ratchet** (harness infra, not
product). Header, ratchet-process rule (`:7-22`), guardrail taxonomy, and reversal log (`:97-100`)
are sound and stay **verbatim**.

**Count (re-derived this pass):** 25 physical table rows (`:27-50` = 24 rows, `:95` = row 21) with
**21 distinct IDs** — IDs 7, 9, 10, 11 each appear twice (a numbering defect: 25 = 21 + 4 dups). The
Wave-2 migration should suffix the collisions `a`/`b` during the move, fixing the defect as part of
the prune.

**Disposition (living vs. legacy):**

- **LIVE — 3 IDs point at tooling that still exists on `main`:**
  - **18** (Markov attractor loop-detector) — `tools/loop-signals/markov_attractor.py` present + wired.
  - **20** (`tools/verify-scope.sh`) — file present; but its row describes eslint/pnpm routing
    branches for scopes deleted by row 21, so the *guardrail text* is now partly stale (prune, don't
    delete).
  - **21** (legacy thin-layer removal, structural CI-gate) — the kernel-era anchor row; unchanged.
- **LEGACY — the other 18 distinct IDs (22 physical rows) reference JS-era paths deleted 2026-07-13
  by row 21** (`apps/web`, `packages/ui`, `apps/api/tests`, `e2e/`, `tools/eslint-plugin-local`,
  `.husky/pre-commit`'s eslint/pnpm branches, `CheckoutPage.tsx`, `useWebSocket.ts`, i18n catalog,
  Astro build, etc.). Their bug CLASS may still matter but their *guardrail* is gone with the code.
- **Three legacy rows have a live kernel-era heir** (the class outlives the dead guardrail; each
  heir becomes a stated obligation so nothing silently evaporates):
  - **7 (money float drift)** → kernel money is integer-typed (`kernel/src/money.rs`,
    `engine/src/money_guard.rs`); heir attaches to P-G's operator-gated money dual-authority flip.
  - **9 (raw-SQL interpolation)** → no SQL layer at HEAD; heir attaches to the pgrust store work
    (P-B `PgEventStore` seam).
  - **10 (cross-tenant IDOR)** → red-line authz class; heir attaches to P-G's product-rebuild DoD.
  - **12 (JS `safeStorage` chaos-monkey)** → its kernel-native successor is **Area 1's harness** — the
    archive row gets a forward pointer.

**Verdict:** move-not-delete (living-memory safe-apply rule). Split the table into a LIVE section
(3 rows) and a headed "Legacy (pre-kernel) — guarded code deleted 2026-07-13 (row 21)" archive
section (moved verbatim, zero content edits, `git diff` shows moves + suffix renames only). This is
a prune/migrate, **not a rewrite**.

---

## Area 3 — Benchmarks (criterion wired; nothing CI-gated)

### 3.1 What exists

- **criterion is wired** in `kernel/Cargo.toml` (`criterion = "0.5"` dev-dep; `[[bench]] name =
  "criterion"`). The harness defines **5 bench IDs** (the Wave-2 blueprint counted 4; re-verified
  here as 5): `place_order/5_items` (`criterion.rs:12`), `fold_transitions/5_hops` (`:60`),
  `empirical_identify/20k_samples` (`:76`), `empirical_identify/end_to_end_20k` (`:80`),
  `token_bucket/try_acquire_permit` (`:91`).
- **Real baseline-diff tracker:** `native-trackers bench <crate-dir> [--threshold N]` runs
  `cargo bench`, parses the text output, compares to `benches/baseline.json`, auto-seeds missing
  IDs, and exits 1 on regression beyond threshold (`tools/telemetry/native-trackers/src/main.rs:14,
  21,143-214`). Portable fallback: `kernel/benches/bench_track.py` delegates to the native binary.
- **Committed baseline is partial:** `kernel/benches/baseline.json` holds only **2 of the 5** IDs
  (`fold_transitions/5_hops`=5.59, `place_order/5_items`=90.4) — the other 3 are auto-seeded
  *locally* but the seed is never committed, so 3 IDs are guarded nowhere.
- **`BENCH_HISTORY.md` is gitignored, not committed** (`git check-ignore kernel/benches/BENCH_HISTORY.md`
  → exit 0; the Wave-2 blueprint's "committed" claim is wrong — correcting it here). Git-tracked
  bench files are exactly: `.gitignore`, `BENCH_RESULTS.md`, `baseline.json`, `bench_track.py`,
  `criterion.rs`.
- **`engine/` has no criterion harness at all** (`ls engine/benches` → absent; no `criterion` in
  `engine/Cargo.toml`).

### 3.2 The gap and the honesty correction

- **Nothing runs benches in CI.** The tracker is a hand/cron tool; no workflow job fails on a perf
  regression. This is the "manual-only" systemic gap.
- **Correction the Wave-2 blueprint must carry:** the committed `baseline.json` numbers were measured
  on THIS Hetzner host (`place_order/5_items` = 90.4 ns). A GitHub shared runner has a different,
  run-to-run-variable constant factor — gating an absolute host baseline on a foreign runner produces
  false REDs or a threshold so wide it gates nothing. The CI gate must be **same-runner-relative**
  (criterion's built-in `--save-baseline base` / `--baseline base` A/B across merge-base vs HEAD, both
  on one runner); the absolute host baseline stays the local/scheduled tracker's job. Refresh the
  committed baseline to all 5 IDs on this host as part of the fix.

---

## Area 4 — Native telemetry substrate + the CI bug + what NOT to re-derive

### 4.1 The mature substrate (extend, don't rebuild — STANDARD §2 item 19)

- `tools/telemetry/lib.sh` — the spool / `log_event` / `bench_run` bridge conventions
  (append-only JSONL ledgers, greppable, one typed object per line).
- `tools/telemetry/telemetry` — the dispatcher (bash, executable) with the real `selftest`
  subcommand (`:472-474`).
- Rust crates: `native-ser` (canonical raw-LE-f64 wire, no serde), `native-trackers` (bench + ledger
  folds, `hetzner-serve` gauges via `statvfs64` FFI), `rust-spool` (crash-safe drainer),
  `hetzner-exporter` (`/proc` gauges).
- The anomaly-detection *pattern* already proven in miniature: `tools/ci-truth`'s `claim-latency`
  appender wired into CI (`ci.yml:50-73`), with the anomaly *consumer* explicitly deferred to Phase 8
  — named constants + pure predicate + explained JSONL record + advisory exit-0.

### 4.2 The CI bug (re-confirmed live on `main`)

`.github/workflows/ci.yml:19-23`:

```yaml
      - name: Run telemetry self-test (local-only, no Telegram secret)
        run: |
          cd tools/telemetry
          chmod +x *.sh
          bash selftest-telemetry.sh      # ← line 23: no such file
```

`ls tools/telemetry/*.sh` = `governance.sh lib.sh report.sh` — **no `selftest-telemetry.sh`.** The
step exits 127 (`bash: selftest-telemetry.sh: No such file or directory`) on every push to `main` or
`feat/kernel-fsm-graph-analysis` and every PR to `main` (`ci.yml:8-11`). The one-line fix is the
dispatcher subcommand: `TELEMETRY_NO_TG=1 ./telemetry selftest` (RED = current step exits 127;
GREEN = replacement prints `selftest: local JSONL write OK (…/selftest.jsonl)`). Reproducible
locally in one command. The adjacent Markov health step already uses the correct `./telemetry`
dispatcher form (`ci.yml:24-29`), so line 23 is an isolated stale reference.

The unconditional `cargo-test` job (kernel + engine, offline, `ci.yml:106-120`) is the only test
gate and is sound — but it runs no benches and no anomaly consumer.

### 4.3 What NOT to re-derive (fold in by reference — the Area-4 verdict)

Two blueprints are **already-decided** and own the runtime halves of this layer. The Wave-2 P-H
blueprint must consume them by reference and re-open no decision inside them:

- **P24** `docs/design/BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md` — owns ALL
  runtime telemetry: the SPSC `kernel/src/ring.rs` flight recorder (single-writer-by-construction per
  RCI H1, no CAS), two-tier emission (`SiteAgg` aggregates + anomaly/1-in-N events), the
  `ExplainedAnomaly` PSI-cause-attribution capsule, RRD bounded history (fixed ~4.5 MB, max-preserved
  tiers), and gauge consolidation onto `statvfs64` FFI. Build plan W1a–W3 stands.
- **P25** `docs/design/BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md` — owns ALL
  concurrency/scheduling: the corrected host truth (4 physical cores × 2 SMT, not 8), the C/D/L
  work-class split, `kernel/src/admission.rs` (pure local admission over PSI/procfs), the
  LOCAL-DECISION + CORE-BOUND rules, and D_max=16. Build plan W1–W4 stands.

**Net P-H scope after this fold** = exactly four things Wave-2 must build: (1) the chaos harness
(Area 1, seeded by `FaultyStore`); (2) the `ci.yml:23` fix (Area 4.2); (3) same-runner-relative
benchmark CI gate + baseline refresh to 5 IDs (Area 3); (4) the regression-ledger prune/migrate
(Area 2). Where a chaos scenario touches P24/P25 territory (gauge-saturation fixtures feeding
`admission.rs`; drainer-death/ring-flood feeding `ring.rs`), P-H supplies only the *injection
mechanism* — the assertions stay owned by the sibling's own build plan.

---

## 5. Standard-compliance notes for the Wave-2 blueprint (what this audit obligates)

- **Item 5 (adversarial / intentionally-failing):** the harness IS the deliverable; each scenario
  needs a stated RED arm. The `TokenBucket` poison-cascade (Area 1.3) is a scenario whose RED arm is
  *current HEAD* — run it pre-fix, record the panic, then fix.
- **Item 6 (hazard-safety from structure):** "chaos machinery reachable in a release artifact" must
  be unrepresentable at the compilation boundary (`#[cfg(any(test, feature = "chaos"))]` + a CI grep
  guard), not policed by a runtime flag — same class as P24's no-`compare_exchange` grep guard.
- **Item 10 (bench + telemetry):** the bench gate turns perf regressions into CI-time failures; the
  scheduled tracker keeps reporting through the existing `bench_run` ledger convention (`lib.sh`).
- **Item 19 (reuse-first):** `FaultyStore` generalized (not duplicated); criterion / native-trackers
  extended (not rebuilt); P24/P25 consumed (not re-derived). This is the load-bearing obligation.
- **Named residual (honest, not fake-enforced):** the ratchet rule ("every fix adds a ledger row")
  stays convention — a commit-message heuristic gate would false-positive constantly. Record the gap;
  do not build enforcement theater.

---

## 6. Citation deltas — reconstruction vs. the Wave-2 blueprint's embedded numbers

The Wave-2 `BLUEPRINT-P-H-ops-telemetry.md` pinned `feat/harness-llm-backend @ cc3d5c916`; this
reconstruction is on `main @ caba2203c`. Where they diverge, **these fresh numbers win:**

| Fact | Blueprint's cite (stale) | Current `main` (this pass) |
|---|---|---|
| `FaultyStore` double | `event_log.rs:440-459` | `event_log.rs:452-489` (struct `:463-466`, always-fail `insert` `:473`) |
| `FaultyStore` test | `event_log.rs:694-718` | `event_log.rs:815` (`append_over_faulty_store_surfaces_err_not_fake_committed`) |
| Drift gate | `event_log.rs:389-419` | `event_log.rs:410-431` (plain path `:357`) |
| `EventStore` trait | `event_log.rs:182-204` | `event_log.rs:182-204` (unchanged) |
| `TokenBucket` `.lock().unwrap()` | `token_bucket.rs:65,77` | `token_bucket.rs:65,77` (unchanged; `Mutex<Inner>` `:29`) |
| criterion bench IDs | 5 (`criterion.rs:12,60,76,80,91`) | 5 (unchanged) — supersedes the *earlier* "4 benches" summary |
| `baseline.json` coverage | 2 of 5 | 2 of 5 (unchanged) |
| `BENCH_HISTORY.md` | (blueprint elsewhere implies committed) | **gitignored** — corrected |
| Regression ledger | 25 rows / 21 IDs | 25 rows / 21 IDs (unchanged) |
| `ci.yml` selftest bug | `ci.yml:23` | `ci.yml:23` (unchanged — still broken) |

---

*Registered under CORE-ROADMAP Layer P-H (`CORE-ROADMAP-STANDARD-2026-07-17.md` §3). Read-only
Wave-1 audit; the Wave-2 blueprint is `BLUEPRINT-P-H-ops-telemetry.md` (same dir). Indexed by
`CORE-ROADMAP-INDEX.md` — this file closes the "MISSING ON DISK" dead link recorded there.*
