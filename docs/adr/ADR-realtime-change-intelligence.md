# ADR: Real-Time Change Intelligence — git-as-single-authority derived ranker

- **Status:** PROPOSED, revised in Triadic Council RESOLVE loop 1 and finalized in
  RESOLVE loop 2 (2026-07-17); hard exit met (0 unresolved CRITICAL/HIGH, 0 unresolved
  ETHICAL-STOP); awaits operator sign-off. Round-1 decision (Option C, event-sourced
  graph projection) is **superseded** — recorded in
  `docs/design/realtime-change-intelligence-2026-07-17/resolution.md`, not made silently.
  Loop-2 re-attack findings F-1..F-6 resolved in `resolution-round2.md` (F-1 HIGH: the
  "single durable authority" claim was over-broad — narrowed to dual-keyed determinism).
- **Date:** 2026-07-17
- **Design doc:** `docs/design/realtime-change-intelligence-2026-07-17/proposal.md` (final)
- **Council record:** `breaker-findings.md` (4 HIGH accepted), `counsel-opinion.md`
  (2 ETHICAL-STOPs resolved, steel-man Option D accepted in extended form),
  `resolution.md`; loop 2: `breaker-findings-round2.md` (1 HIGH, 4 MED, 1 LOW),
  `counsel-opinion-round2.md` (no new STOPs), `resolution-round2.md` (5 FIX, 1
  ACCEPT+DEFER).
- **Related:** ADR 0001 (queue-in-postgres — boring/proven precedent);
  `kernel/src/event_log.rs` (decide/fold law; drift gate; NO-COURIER-SCORING);
  `kernel/src/markov.rs` (Python→Rust ascension precedent, dual-authority hazard);
  bebop-repo `LOGIC-LAWS.md` + `logic-gate.mjs` (rule layer, live).

> **SCOPE RULE — RCI is a canonical-repo dev-time fence, NOT a runtime/global control;
> at runtime every hub (M5/M9) MAY ignore it.** Any future blocking coupling is
> subordinate to the drift-gate's intervention-lift + kill-switch
> (`event_log.rs:380-383`): even enabled, RCI is never a permanent block.
> Exit plan: delete the hook line + the binary + `.rci/`.

## Context

Chronology (git; tool-outcome transcripts; CI results) and topology (import/dependency
structure of ~1.2k module-level files) exist as separate, un-joined authorities: no ranked
pre-landing impact prediction, no error detection on the change stream, no principled
revert candidate, and two residual Python files on the loop-signals path
(`tools/loop-signals/transcript_events.py`, `test_transcript_e2e.py`).

The kernel owns the required math: deterministic CSR + fixed-K personalized PageRank +
recall/precision scorers (`csr.rs`), scalar steady-state Kalman (`geo.rs:39`,
`kalman.rs:3-6`), the native Markov attractor detector (`markov.rs`). The correct move is
to re-point these, not build new math. The brief's `kernel/src/incidence.rs` does not
exist; the primitives mirrored are `csr.rs`/`spectral.rs`/`cgraph.rs`.

**Why round 1 was overturned.** The breaker pass proved Option C's three load-bearing
claims false: (H1) its chain substrate is single-writer with no compare-and-swap
(`event_log.rs:18-19,204,310`) — four concurrent producers fork the chain and silently
lose deltas; (H2) compensator rollback is not idempotent under head-advance and its digest
invariant is unsatisfiable under concurrent writes; and decisively, round-1's own §7
declared "git is ground truth; the chain is a derived cache" — a derived cache is not a
single authority. Option C rebuilt git's own content-addressed hash-DAG one level up:
the dual-authority hazard (`markov.rs:1-8`) it claimed to kill by construction.

## Decision

Build RCI as **Option D′ — git-as-single-authority derived ranker** (Counsel's steel-man
Option D extended with two organs whose need the brief itself establishes):

1. **Authorities, named honestly (narrowed in loop 2, F-1):** **git is the single
   git-durable authority for the structural half** (`graph.csr`); **transcript JSONL +
   CI are a machine-local co-authority for the stream half** (`state.json`: error EMA +
   loop) — session-scoped, outside git (`tools/loop-signals/check.sh:17-21`), not
   reconstructible at a historical sha, not machine-invariant. The blanket "all durable,
   append-only, replayable" claim is withdrawn. RCI keeps **no chain of its own**; all
   RCI state is a derived, disposable cache under `.rci/`.
2. **`rci derive` = pure, deterministic, single-writer derivation, dual-keyed:**
   `graph.csr` keyed to **(git HEAD, tree)** — machine-invariant, historical via
   `--at <sha>`; `state.json` keyed to the **fixed machine-local stream byte set**
   folded in canonical order **(timestamp, stream_id, line_index)** and **always fully
   re-folded** (no incremental path for the non-commutative half — F-2; incremental
   caching applies only to the commutative CSR half; cold path < 10 s). Frames carry
   both keys (`derived_at`, `state_input_digest`). **Lock primitive (F-5):** OS advisory
   lock, `std::fs::File::try_lock()` (Rust ≥ 1.89; `flock(2)`) on `.rci/lock` —
   OS-released on process death incl. SIGKILL, so no stale-lock class exists; PID +
   started-at in the lockfile are observability metadata only. **Write protocol:**
   tmp-file → fsync → atomic same-dir rename; `meta.json` renames last (commit point) —
   crash leaves a complete old or complete new snapshot, never a torn mix.
3. **Graph = CSR of `W = β·Â_imports + (1−β)·Â_cochange`.** Co-change extraction carries
   four explicit controls (breaker H3; loop-2 F-3): commits > 30 files contribute zero
   pairs (logged + counted, never silent — Zimmermann/ROSE ICSE 2004 + code-maat's own
   default, reconciled against this repo's live commit history, not asserted; the repo's
   real 896-file commit would alone emit 400,960 edges, 20× any budget); per-node top-32
   neighbor pruning; resulting cap ≤ 40k co-change edges, nnz ≈ 52k total, CSR < 1 MB,
   PPR 3–6 ms; **neighbor-churn alarm** — Jaccard < 0.5 of a node's top-32 set vs W=32
   commits earlier ⇒ `churn_alert: true` on the frame + a `rci status` counter (closes
   the `excluded_commits` blind spot: sub-F_max burst commits are included and never
   increment that counter).
4. **Analyzers (v1 minimum, each with a plain-language "why"):** PPR blast radius scored
   by a **retrospective backtest** (predict at commit *t*, score vs later fix-commits;
   floor = Wilson lower bound; **stratified** — red-line-glob incidents reported
   separately, never blended); per-node failure EMA with innovation-threshold anomaly;
   **revert-candidate suggestion (human-confirmed)**; markov loop verdict consumed as-is.
   One query → one `AnalysisFrame` JSON (mirrors the `markov.rs:84-94` contract shape).
5. **Rollback = re-derivation.** `rci derive --at <sha>` recomputes the projection at any
   commit — idempotent because pure. The compensating-delta mechanism is deleted; round-1's
   "idempotent + safe under concurrent writes" claim is **withdrawn**. Historical scope
   (F-1): `--at` reconstructs the *structural* view bit-identically; stream-half fields
   are current-machine and labeled so, never presented as historical. Code reverts remain
   human-executed `git revert`; RCI only suggests candidates.
6. **Advisory / fail-open only. No blocking path exists in v1.** `RCI_BLOCKING` does not
   exist; the name is reserved with pinned preconditions (below). Degradation: derive
   failure/staleness ⇒ frames marked STALE, hooks pass; missing binary ⇒ hook no-ops
   (mirrors `check.sh:22-24,35-39`). `RCI_ESCALATE` (default OFF) gates any `ESC-`
   emission into the bebop `logic-gate.mjs` contract; enabling requires the measured
   backtest floor first.
7. **Negative guarantees (LOCK-grade):**
   - **Red-line asymmetry:** RCI never has blocking or blessing authority over
     money/auth/RLS/migrations surfaces; on those it may only add friction, never remove
     it; frames carry `red_line: true` + a fixed disclaimer, **and rendering is
     one-directional (loop-2 F-4): blast/ranking numbers are suppressed entirely on
     red-line frames — only concern-raising signals (innovation over threshold,
     churn_alert) may render; reassurance-capable numbers never do.** The
     automation-bias channel (a low number beside a disclaimer) is removed structurally.
     **No precision@k threshold overrides this** — the aggregate baseline is
     survivorship-biased exactly on the invariant-coupled class the import/co-change
     graph is structurally blind to (breaker H4; the E1 sign-split case is kept in the
     DoD as a documented negative control, an expected miss).
   - **«RCI НІКОЛИ авто-не-виконує revert реального коду й НІКОЛИ авто-не-емітить
     компенсуючу подію в продакшн event_log; correction — завжди human-confirmed
     suggestion, тим паче money/auth-суміжне.»** Enforced by a fence guard test: no write
     path to the kernel production event log; RCI writes only under `.rci/`.
   - **NO-AGENT-SCORING** (mirror of NO-COURIER-SCORING, `event_log.rs:22-23`): the
     derived schema has no actor dimension (author/agent identity dropped at parse);
     guard test asserts no per-actor aggregation in any output.
8. **Data discipline:** closed schema everywhere — no free-form text, no error text, no
   contents in `.rci/` (test results = `{test_id, status, duration_ms}`; tool outcomes =
   the 4-token anonymized alphabet). Plain paths (same trust domain as the working tree;
   the round-1 path-hash confidentiality claim is withdrawn as hollow). No DB, zero
   Postgres connections; `.rci/` excluded from backup scope (restore = re-derive).
   Cross-tenant folding into one graph is a **rejected construction**; any per-tenant
   variant = per-tenant derivation + tenant discriminant + new ADR + RLS ENABLE+FORCE.
9. **Native Rust only, zero Python:** port `transcript_events.py` **and** an
   equivalent-coverage Rust replacement for `test_transcript_e2e.py`, green before any
   `.py` deletion (test-integrity red-line).

## Options considered

- **A — CQRS dual-index + join key:** rejected; dual-authority hazard with a silently
  desyncable join.
- **B — bitemporal unified node:** rejected; new core structure mirroring nothing here;
  over-engineered for the measured scale.
- **C — event-sourced graph projection (round-1 choice):** superseded for cause — see
  Context ("Why round 1 was overturned") and `resolution.md` §0/A.
- **D — pure git-log co-change ranker (Counsel steel-man):** carries ~80% of headline
  value with near-zero ethical surface, but blind to no-history files and lacks the error
  organ + unified view. Extended rather than rejected.
- **D′ — git-as-single-authority derived ranker:** **chosen.** Smallest mechanism that
  carries the value; H1/H2/L2 dissolve structurally; every removed organ returns only via
  a named measured trigger (earn-the-power applied to analyzers, not just flags —
  Counsel N2).

## Deferred organs (named triggers; MISSING until fired)

| Deferred | Trigger | Pinned preconditions |
|---|---|---|
| GraphDelta event chain | measured need for sub-commit granularity OR a non-replayable source | CAS/lock-proven single-writer; convergent rollback + concurrent-load DoD |
| Spectral drift / cascade organ | backtest shows an incident class PPR ranks poorly that quotient-spectrum ranks well | curated ≤32 partition map (40 top-level dirs today > 32 — auto-dirs forbidden); explicit n≤32 code check before `eigenvalues()` (kernel auto-selects the O(n⁴) dense path with no refusal, `spectral.rs:206-213`); plain-language why; **declared code-vs-swarm claim AND a recorded operator want/don't-want decision on reifying swarm mood as a repo-health number — deliberation, not checkbox; efficacy can never swallow the purpose question** (Counsel §5, sharpened loop 2) |
| Co-change history windowing | measured cold derive > 30 s, or the git-log stage alone > 15 s (cold derive is the recovery path — its cost grows O(total history)) | window at the decay horizon (ε-preserving: decayed weights already send older commits to ε) + snapshot base — pre-planned so the trigger cannot force an improvised patch (loop-2 F-6) |
| Full n-D Kalman | measured EMA FP/FN rate insufficient | — |
| `RCI_BLOCKING` | operator explicitly proposes it | recorded human decision (ETHICAL-STOP-1); SCOPE-RULE banner + intervention-lift subordination; stratified precision incl. red-line strata; red-line LOCK (never overridable) |
| `RCI_SYMBOL_LEVEL` | stratified module-level precision < 0.6 floor | tree-sitter dep = DECART report |
| pgrust storage move | Living-Memory arc lands + state outgrows files | forward-only atomic migration; RLS ENABLE+FORCE if multi-scope |
| Per-tenant / runtime-stream variant | any proposal to read runtime error streams | no cross-tenant fold; per-tenant derivation + tenant discriminant + new ADR + RLS FORCE |

## Consequences

- (+) One git-durable authority for the structural half; **dual-keyed** reproducibility
  (structural: (HEAD, tree), machine-invariant; stream: fixed machine-local byte set,
  canonical fold order — declared, keyed, boundary-tested); idempotent re-derivation
  replaces compensator machinery; no fork class, no rollback race, no stale-lock class
  (flock is OS-released on death), no incremental-vs-cold divergence for the stream half
  (the second fold path is deleted) — by removal of mechanism, not by protocol
  discipline.
- (−) The error/loop organ is a **machine-local instrument**: not reconstructible at a
  historical sha, not machine-invariant — declared and keyed (`state_input_digest`)
  instead of guaranteed falsely (loop-2 F-1). Transcript git-versioning was considered
  and rejected (over-engineering + a new PII/noise surface for zero measured need).
- (+) Honest budget: incremental derive 10–50 ms, cold < 10 s, frame query p95 < 100 ms,
  CSR < 1 MB, zero Postgres connections.
- (+) Zero Python on the loop-signals path, including the e2e test (ported, not deleted).
- (+) Both ETHICAL-STOPs resolved structurally (banner + no blocking mechanism;
  negative revert guarantee + fence test); NO-AGENT-SCORING structural + guarded.
- (−) No sub-commit latency, no non-replayable sources, no chain tamper-evidence —
  accepted; no measured need today; named triggers above.
- (−) Wide-commit exclusion drops pairs from genuinely-coupled >30-file commits —
  accepted, logged + counted; import channel still covers them; first backtest run must
  include a {30, 64} sensitivity check per the reconciliation's named residue.
- (−) The graph is structurally blind to invariant-coupling (money/auth/RLS by runtime
  contract, not import) — **declared, measured as a separate stratum, and locked out of
  ever earning authority there**, rather than hidden in an aggregate.
- Risk register with owners: proposal §10 (R1–R8, D1–D5).

## Verification (DoD)

(a) structural-determinism digest test — same (HEAD, tree) ⇒ same `graph.csr`
**regardless of transcript contents** (RED = one mutated edge ⇒ mismatch);
(b) projection-reset idempotence (pure-function byte-equality, structural half);
(c) stratified backtest + E1 negative control (expected-miss asserted + recorded);
(d) cold derive < 10 s, incremental p95 < 1 s, frame p95 < 100 ms;
(e) zero `.py` under `tools/loop-signals/` with the Rust e2e replacement green first;
(f) fence guard — no write path to the production event log;
(g) NO-AGENT-SCORING guard — no actor field, no per-author aggregation **in any output
including the backtest report**;
(h) red-line frame test — flag + disclaimer present **and blast/ranking values ABSENT**
(suppression, loop-2 F-4; fixture: `apps/api/src/routes/orders.ts`);
(i) stream determinism + boundary (loop-2 F-1/F-2) — fixed transcript fixture ⇒
byte-identical `state.json`, invariant under input arrival order; and same (HEAD, tree)
with different transcript sets ⇒ identical `graph.csr` AND differing `state.json` (the
declared limit, asserted);
(j) crash safety (loop-2 F-5) — SIGKILL mid-derive ⇒ lock immediately reacquirable,
previous snapshot intact;
(k) churn alarm (loop-2 F-3) — synthetic 32×2-file commit burst trips `churn_alert` +
the status counter.
