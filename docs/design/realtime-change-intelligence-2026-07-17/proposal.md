# Real-Time Change Intelligence (RCI) — Design Proposal (FINAL, post RESOLVE loop 2)

> Status: RESOLVED-FINAL (both Council loops closed; hard exit met — 0 unresolved
> CRITICAL/HIGH, 0 unresolved ETHICAL-STOP; awaits operator sign-off).
> Author: system-architect subagent, 2026-07-17.
> Round history: round-1 chose Option C (event-sourced graph projection); the first RESOLVE
> (`resolution.md`) accepted breaker findings H1–H4 + Counsel's steel-man and **changed the
> decision to Option D′** (git-as-single-authority derived ranker). The round-2 re-attack
> (`breaker-findings-round2.md`) found one HIGH over-claim D′ itself introduced (F-1:
> transcript stream is machine-local, not git-durable) + F-2..F-6; RESOLVE loop 2
> (`resolution-round2.md`) fixed F-1..F-5 and accepted F-6 with a named trigger. All
> changes are recorded, not silent.
> ADR: `docs/adr/ADR-realtime-change-intelligence.md` (revised in both loops).
> Style contract: no metaphor; every load-bearing statement carries a `file:line` cite or is
> explicitly tagged **(proposal)**.

## 0. Scope banner + corrections (read first)

> **SCOPE RULE — RCI is a canonical-repo dev-time fence, NOT a runtime/global control;
> at runtime every hub (M5/M9) MAY ignore it.**
> Any future blocking coupling is subordinate to the drift-gate's intervention-lift +
> kill-switch (`event_log.rs:380-383` — "ALL safeties are LIFTED"): even a
> hypothetically-enabled RCI gate is never a permanent block; the conscious operator always
> lifts all safeties.
> **Exit plan (pinned here on purpose, per Counsel N4):** removal = delete the hook line +
> the `rci` binary + `.rci/`. Nothing else couples to it.

- The task brief named `kernel/src/incidence.rs`; **it does not exist** (glob-verified
  2026-07-17). The structural primitives actually mirrored are `csr.rs` (CSR + PPR +
  Laplacian SpMV), `spectral.rs`, `cgraph.rs`.
- **v1 contains no blocking path of any kind.** `RCI_BLOCKING` does not exist in v1; the
  name is reserved with hard preconditions (§10, resolution §F) so a future proposal
  cannot introduce it more weakly than this round demands.

---

## 1. Problem + non-goals

### Problem

The repo has chronology and topology as separate, un-joined authorities:

- **Chronology**: git history (itself an append-only, content-addressed hash-DAG of file
  changes); agent tool-outcome streams consumed by the Markov attractor detector
  (`kernel/src/markov.rs:1-19`); CI/test results.
- **Topology**: import/dependency structure of ~1.2k source files, currently analyzed only
  ad hoc; the diffusion/ranking machinery that could read it
  (`kernel/src/csr.rs:228-264` PPR, `csr.rs:387-427` recall/precision scorers) has no
  live code-graph input.

Consequences:

1. **No pre-landing consequence prediction.** A change to file X gives no ranked estimate
   of which files/tests break next.
2. **No error detection on the change stream.** Per-module failure-rate math exists
   (`geo.rs:39` `ema_next` — the scalar steady-state Kalman per `kalman.rs:3-6`) but is
   wired only to courier kinematics.
3. **No principled revert candidate.** When something is broken, finding the culprit
   commit is manual archaeology.
4. **Residual Python.** `tools/loop-signals/` still contains `transcript_events.py` +
   `test_transcript_e2e.py` (verified: exactly these 2 files). The brief requires zero
   Python; this design closes the remnant including the test.

### Non-goals (v1)

- **Not a production/tenant-facing feature.** Operator dev-plane only; never reads
  `apps/*` runtime tenant data, orders, money, PII (§8).
- **No blocking, no gate feed.** Advisory fail-open only (mirrors
  `tools/loop-signals/check.sh:22-24,35-39`). No `RCI_BLOCKING` in v1 (resolution H4).
- **No event chain.** Git is durable and git-versioned; transcript JSONL + CI streams
  are append-only and replayable from their bytes but **machine-local, session-scoped,
  outside git** (`check.sh:17-21` — `$HOME/.claude/projects/*/*.jsonl`): a co-authority
  of lesser durability, named honestly (round-2 F-1; the blanket "all durable/replayable"
  claim is withdrawn). A second content-addressed chain over them would copy durability
  where it exists and cannot create it where it doesn't — the dual-authority hazard
  (`markov.rs:1-8`) reborn (resolution H1/§0). Deferred with a named trigger (§10).
- **No spectral drift organ, no full n-D Kalman in v1.** Each further analyzer must be
  pulled in by a measured gap against the backtest oracle, not by budget headroom
  (Counsel N2; resolution §E). Capacity is not need.
- **No compensator rollback.** Rollback of derived state = re-derivation (§6).
- **Not symbol-level extraction** (deferred, `RCI_SYMBOL_LEVEL`, §10 R1).
- **No new heavyweight deps** (no tree-sitter, no libgit2, no GPU; kernel zero-dep rule
  `csr.rs:36-37`).
- **Not a replacement for the bebop rule layer** (`LOGIC-LAWS.md` + `logic-gate.mjs`
  stay the sole law authority; RCI emits into their contract, flag-gated, §4.2).

---

## 2. Back-of-envelope (mandatory; revised after H3/M5)

**Graph size.**

| Quantity | Value | Ground |
|---|---|---|
| Module-level nodes | ≈ **1.2k** | Repowise layer table + kernel glob 2026-07-17 |
| Import edges | **~6–12k** (5–10 imports/file) | estimate |
| Widest commits in real history | **896 files** (max), 507 (5th); several >100 | breaker measurement 2026-07-17 |
| Naive co-change from the 896-file commit alone | C(896,2) = **400,960 edges** — forbids naive extraction | arithmetic |
| Worst included commit at F_max=30 | C(30,2) = **435 pairs** | arithmetic; F_max = literature default (ROSE / code-maat), §4.1 |
| Co-change after F_max=30 exclusion + per-node top-32 pruning | ≤ 1.2k×32 ≈ **38.4k → cap 40k** — bound is set by the pruning, **independent of F_max** | design controls (§4.1) |
| Total nnz budget | ≈ **52k** | imports + co-change |
| CSR memory at nnz=52k | 52k×16 B + 1.2k×8 B ≈ **0.85 MB** | `csr.rs:46-54` layout |
| `.rci/` total on disk | **< 5 MB** | derived cache, snapshots included |

**Latency.** v1 is per-commit / on-demand, not streaming — the round-1 100 ms streaming
budget justified organs no requirement demanded (Counsel N2) and mis-costed the fold
(breaker M5). Honest budget:

| Operation | Cost | Ground |
|---|---|---|
| Incremental derive per commit (git subprocess + changed-file rescan) | **10–50 ms** | subprocess-dominated; estimate |
| Full CSR rebuild, nnz=52k (`Csr::from_edges` = full sort+merge rebuild — NOT an O(row) insert) | **~5–10 ms** | `csr.rs:79-115`; corrects round-1 "µs" line (M5) |
| PPR blast radius, K=60, nnz=52k | 60×52k ≈ 3.1M mult-adds ≈ **3–6 ms** | `csr.rs:228-264` (fixed-K Jacobi, deterministic) |
| Stream state fold — **full recompute each derive**, canonical order (no incremental path for the non-commutative half, round-2 F-2) | O(events) **< 10 ms** at ≤ ~10⁵ session-scoped events | `geo.rs:39`; resolution-round2 §B |
| `rci frame <path>` query | **p95 < 100 ms** | sum of the above on a warm snapshot |
| Cold full derive (git log ~5k commits + scan 1.2k files + build) | **< 10 s** (847 commits today ⇒ ~5× margin; growth trigger K=30 s pinned, R10) | estimate; restore path §7; round-2 F-6 |

**Forbidden path stays named:** dense eigensolve at n=1.2k is O(n⁴)
(`spectral.rs:206-213`), and the kernel `eigenvalues()` **auto-selects it with no
refusal** — so "unreachable" must be enforced by an explicit RCI-side n≤32 check in any
future spectral organ (resolution M5), not by prose. In v1 RCI calls no spectral function
at all.

**Connection budget:** zero Postgres connections. File-backed derived cache only (§5).
A future pgrust move adds exactly 1 dev-plane connection, outside the
API+worker+analytics+migrations production budget (§10 R7).

---

## 3. Options (each named by its concept)

### Option A — CQRS dual-index + join key
Two authorities (incremental CSR index + chronology chain) joined at query time.
**Concept:** CQRS read models. **Rejected:** reintroduces the dual-authority hazard
`markov.rs:1-8` documents; the join can silently desynchronize.

### Option B — Bitemporal unified node (topo_id, chrono_id)
Persistent versioned graph, Datomic-style. **Concept:** bitemporal persistent graph.
**Rejected:** new core data structure mirroring nothing here; over-engineered for §2 scale.

### Option C — Event-sourced graph projection with snapshots (round-1 choice, SUPERSEDED)
Own `GraphDelta` hash chain as "single authority"; CSR = fold; rollback = compensating
deltas. **Concept:** event sourcing + Saga. **Superseded in RESOLVE, for cause:**
- Its substrate is single-writer with no CAS (`event_log.rs:18-19,204,310`); four
  concurrent producers fork the chain and silently lose deltas (breaker H1).
- Its compensator rollback is not idempotent under head-advance, and the digest invariant
  is unsatisfiable under concurrent writes (breaker H2).
- Decisively: round-1 §7 itself declared "git is ground truth; the chain is a derived
  cache" — a derived cache is not a single authority. Option C rebuilt git's own
  content-addressed DAG one level up: the dual-authority hazard it claimed to kill.

**Ledger note (Counsel round-2 Q3):** Option C was not waste — it was the necessary
probe: H1/H2 are findable only in a design specified well enough to break, and the small
v1 below is the **earned dividend of that research**, not its refutation. The architect's
self-reversal under evidence is recorded as integrity (anti-sunk-cost), not a corrected
blunder. Symmetrically (the inverse drift Counsel watches): removal is not a trophy —
every future cut must earn its case per-instance exactly as every addition must.

### Option D — Pure git-log co-change ranker (Counsel steel-man)
No graph machinery at all: count co-occurrence, rank top-k. **Concept:** frequency
baseline. Carries ~80% of the headline value and near-zero ethical surface — but is blind
to new files with no co-change history, has no error organ, and has no unified view
contract. Extended, not rejected:

### Option D′ — "Git-as-single-authority derived ranker" ← CHOSEN
Git is the **single git-durable authority** for the structural half; transcript JSONL +
CI are a **machine-local co-authority** for the stream half (append-only and replayable
from their bytes, but session-scoped and outside git — claim narrowed in loop 2, F-1).
RCI is a **pure, deterministic derivation, dual-keyed** (§4.1/§6): one single-writer
`derive` pass rebuilds a CSR projection + error state; one `frame` query returns the
unified view. No chain, no fold-over-chain, no compensators, no blocking.

- **Concept:** derived read-model projection (CQRS read side with the write authority =
  git); idempotency via pure functions; claim-check; backtested baselines; YAGNI /
  earn-the-power per organ.
- **Pros:** H1/H2/L2 dissolve structurally (nothing to fork, nothing to compensate,
  determinism keyed to source state); keeps everything Option C measurably delivered
  (PPR ranking, β-combined operator, EMA error organ, AnalysisFrame, zero-Python port);
  smallest mechanism that carries the value.
- **Cons (honest):** no sub-commit latency; no ingestion of non-replayable sources; no
  chain-level tamper evidence. No measured need requires any of the three today; each is
  a named deferred trigger (§10).
- **Verdict: chosen.**

---

## 4. Decision + architecture (Option D′)

Full ADR: `docs/adr/ADR-realtime-change-intelligence.md`.

### 4.1 Components (all proposal; module names indicative)

```
authority (existing, durable, replayable)      derived (RCI, all under .rci/)
─────────────────────────────────────────      ─────────────────────────────────
git history (commits, --name-only)      ──►    rci derive  ──►  graph.csr (CSR, β-combined)
transcript JSONL (4-token alphabet)     ──►    (single writer,   state.json (EMA per node)
CI / test results (closed schema)       ──►     flock on .rci/)  meta.json (derived-at HEAD)
                                                      │
                                               rci frame <path> ──► one AnalysisFrame JSON
                                                      │
                                               advisory hooks (fail-open, mirrors check.sh)
                                               ESC- emitter (RCI_ESCALATE, default OFF)
```

- **`rci derive`** *(proposal)*: pull-based, single-writer.
  - **Lock primitive, named (round-2 F-5):** OS advisory lock via
    `std::fs::File::try_lock()` (stable Rust ≥ 1.89; `flock(2)` on Linux) held on
    `.rci/lock` for the whole derive. Kernel-associated ⇒ the OS releases it on process
    death **including SIGKILL/OOM/power loss** — a crashed derive cannot wedge RCI; no
    stale-lockfile class exists, so no heartbeat/takeover machinery (rejected as
    duplicating an OS guarantee). PID + started-at go into the lockfile as observability
    metadata only (`rci status` shows holder + age); liveness never depends on them.
  - **Write protocol (crash atomicity, F-5):** every artifact is written tmp-file →
    fsync → atomic same-directory `rename(2)`; `meta.json` renames **last** (the commit
    point). A crash at any instant leaves the complete old snapshot or the complete new
    one, never a torn mix; the loader validates version + digests and falls to cold
    re-derive (§7) on mismatch.
  - **Determinism, dual-keyed (round-2 F-1):** `graph.csr` (+ structural meta) is a pure
    function of **(git HEAD, tree)** — machine-invariant, reproducible at any historical
    sha via `derive --at`. `state.json` (error EMA + loop) is a pure function of the
    **machine-local transcript/CI byte set** under the canonical fold order —
    deterministic for a FIXED input set, but the inputs are session-scoped and outside
    git (`check.sh:17-21`): **not** reconstructible at a historical sha, **not**
    machine-invariant. Frames carry both keys (`derived_at`, `state_input_digest`) so a
    cross-machine verdict difference is attributable in one glance.
  - **Canonical fold order + single fold path (round-2 F-2):** all stream events are
    merged and sorted by **(timestamp, stream_id = source file path, line_index)** — a
    total order — and `state.json` is **always fully re-folded** from that stream
    (O(events) < 10 ms, §2). The incremental cache (last-seen HEAD + per-file scan
    hashes) applies **only to the commutative CSR half**; with one fold path for the
    non-commutative half, incremental-vs-cold divergence cannot exist. Cold path
    re-derives everything (<10 s, §2).
- **Extractors** (native Rust, tools layer, zero new deps):
  - **import graph**: line-regex over `import ... from '...'` / `use crate::...`;
    deterministic; NOT tree-sitter (R2/R1).
  - **co-change graph**: `git log --name-only` via `std::process::Command` (no libgit2),
    with four explicit controls (resolution H3; round-2 F-3):
    1. commits touching > **F_max = 30** files contribute **zero** pairs — the
       established default, not a bespoke number: ROSE "ignor[es] all changes that
       affect more than 30 entities" (Zimmermann et al., ICSE 2004) and code-maat's
       `--max-changeset-size` defaults to 30 (Tornhill). Structural harmony with
       control 2: 30 ⇒ ≤ 29 same-commit neighbors < 32, so **no single commit can
       saturate a node's pruned neighbor list** (at 64 one bundle-commit could — H3
       saturation in miniature). Excluded commits are **logged and counted in
       `rci status`**, never silently pruned. (Revised 64 → 30 post-resolve;
       resolution "H3 reconciled against prior-art".)
    2. per-node **top-32** co-change neighbors by decayed weight (`ema_next`, `geo.rs:39`);
    3. resulting cap ≤ 40k edges, re-derived in §2 (not asserted);
    4. **neighbor-churn alarm (round-2 F-3):** `meta.json` keeps each node's top-32
       neighbor-set fingerprint per derive; when a node's current set has **Jaccard
       similarity < 0.5** vs its own set **W = 32 commits earlier** (≥ 16/32 replaced),
       `rci status` surfaces `churn_alerts=<n>` and the node's frame carries
       `churn_alert: true` + the plain-language why ("28 of 32 neighbors replaced within
       the last 32 commits — treat this ranking as suspect"). Closes the counter blind
       spot: sub-F_max burst commits are *included* and never touch `excluded_commits`;
       churn is the signal that sees them. Residue for below-threshold pacing = R9.
  - **transcript stage-1**: port of `transcript_events.py` into the same binary — and of
    `test_transcript_e2e.py` into an equivalent-coverage Rust e2e test, green **before**
    the `.py` files are deleted (resolution L1; test-integrity red-line). Completes the
    `markov_attractor.py` ascension precedent (`markov.rs:3-8`).
  - **Actor fields are dropped at parse** (git author, any agent id): the derived schema
    has **no actor dimension** — nodes are paths, edges are coupling. NO-AGENT-SCORING
    is structural, plus a guard test (§8, resolution N1).
- **Combined operator**: `W = β·Â_imports + (1−β)·Â_cochange`, both row-normalized
  (`csr.rs:125-152`), β=0.5 stated tunable (R4), tuned by the backtest oracle.
- **Analyzers (v1 — the earned minimum):**
  1. **Blast radius / consequence prediction**: PPR seeded at changed nodes
     (`csr.rs:228-264`), top-k = predicted impact. Scored by
     `recall_at_k`/`precision_at_k` (`csr.rs:387-427`) via the **retrospective backtest**
     (predict at commit *t*, score against fix-commits in a window; floor = Wilson lower
     bound) — measurable from day one, no cold-start flood or months-inert gap
     (resolution M4). Scoring is **stratified**: red-line-glob incidents reported
     separately, never blended into the headline number (resolution H4).
  2. **Error anomaly**: per-node failure rate from `run_fail`/test outcomes smoothed by
     `ema_next` (`geo.rs:39` — the scalar steady-state Kalman, `kalman.rs:3-6`);
     innovation beyond threshold ⇒ anomaly. Full n-D Kalman deferred (§10).
  3. **Revert-candidate suggestion (human-confirmed)**: rank recent commits by overlap of
     their touched files with anomalous nodes; output = suggested candidates for a
     **human-executed** `git revert`. Negative guarantee in §8 (resolution STOP-2).
  4. **Session loop health**: consume the existing native markov verdict
     (`kernel/src/bin/markov_attractor.rs`, contract `markov.rs:80-94`) as an input
     signal — no re-implementation.
  - **Not in v1** (deferred with named triggers, §10): spectral drift/cascade organ,
    full n-D Kalman, event chain, symbol-level extraction.
- **One simultaneous view**: `rci frame <path>` → one `AnalysisFrame` JSON *(proposal)*:
  `{chrono: recent commits touching path (from git), topo: import neighbors + co-change
  top-k, blast: PPR top-k, error: EMA + innovation, loop: markov verdict,
  churn_alert: bool, red_line: bool, derived_at: HEAD sha (structural key),
  state_input_digest (stream key — machine-local, round-2 F-1), stale: bool}` — contract
  shape mirrors `markov.rs:84-94` (the same shape `check.sh` already parses). On
  `red_line: true` frames rendering is **one-directional**: ranking numbers are
  suppressed entirely (§8, round-2 F-4). A frame produced by `derive --at <sha>` labels
  its `error`/`loop` fields as computed from the *current* machine stream, never
  presented as historical (F-1). **Every surfaced item carries a plain-language why**
  ("changed together N times in M commits, last at <sha>"; "imports X"; "failure EMA 0.4
  over last 12 runs") — a verdict that cannot explain itself may not surface
  (Counsel N3, standing rule).

### 4.2 Rule-layer integration (plug in, don't duplicate; OFF by default)

- `ESC-` emission is gated by **`RCI_ESCALATE` (default OFF)**. A cold RCI emits zero
  escalation records; findings live only in the local frame (resolution M4). Enabling
  requires the backtest floor measured first. Mapping when enabled: contradiction-type
  finding → LNC §2; unbacked prediction → PSR §4 escalate (exit 2,
  `logic-gate.mjs:17-21`). RCI never writes RESOLVED — human-arbitrated per
  `LOGIC-LAWS.md` §7.
- In dowiz, RCI output is advisory JSON consumed by hooks exactly as `check.sh` consumes
  the markov JSON today (`check.sh:33-39`).

### 4.3 Applied concepts (named, per mandate)

Derived read-model projection with a single external write authority (git); pure-function
idempotency (re-derivation, not compensation); claim-check (closed schema, no contents,
no error text); circuit-breaker-style degradation ladder (§7); retrospective backtesting
with Wilson lower bound (baseline before power); YAGNI / earn-the-power per organ;
fail-open advisory defaults. Loop-2 additions: dual-keyed determinism (per-artifact
input keys instead of a blanket reproducibility claim); OS-guaranteed lock liveness
(flock released on process death — liveness from kernel semantics, not heartbeat
protocol); write-ahead-by-rename crash atomicity (tmp+fsync+rename, commit-point file
last); one-directional information rendering on red-line surfaces (suppression, not
disclaimer).

---

## 5. Data / migrations

- **v1: no database.** `.rci/` *(proposal)*: `graph.csr` (canonical CSR serialization),
  `state.json` (EMA state), `meta.json` (derived-at HEAD, scan hashes, excluded-commit
  count, `state_input_digest`, per-node top-32 neighbor fingerprints for the churn
  alarm), `lock` (flock target; PID + started-at as metadata only — F-5). All
  **derived**, all disposable; restore = re-derivation (§7). All writes tmp+rename
  atomic; `meta.json` last (§4.1).
- **Closed schema — no free-form text anywhere** (resolution M2): test results persist as
  `{test_id, status, duration_ms}`; tool outcomes as the anonymized 4-token alphabet
  (`markov.rs:30-39`); assertion diffs / stack traces / stderr / env values **never enter
  `.rci/`**. Paths are stored **plain** — `.rci/` lives in the same trust domain as the
  git working tree; round-1's path-hash "confidentiality" claim is withdrawn as hollow
  (resolution L3). Accepted exposure: repo structure + closed-schema counters, nothing
  else.
- **Forward-only:** sources are append-only (git, JSONL); `.rci/` is regenerated, never
  migrated in place — a schema change bumps a version field and triggers cold re-derive.
- **RLS:** no tenant tables touched in v1 ⇒ not applicable — stated, not skipped.
  **Rejected construction (resolution M3):** cross-tenant folding into one graph is
  forbidden — a merged CSR has no tenant discriminant to re-partition on, and RLS-at-rest
  cannot fix an in-RAM merge. Any per-tenant variant = per-tenant derivation
  (scope-per-hub), tenant discriminant in the schema from birth, new ADR, and
  RLS ENABLE+FORCE on any stored table (R7).
- **Integer/money rule:** no money data anywhere in RCI; floats are graph-operator
  structure only — the documented float exemption (`spectral.rs:25-27`).
- **Backups:** `.rci/` is a derived cache of git + transcripts (both have their own
  backup stories); explicitly **excluded** from DB backup scope; restore = re-derive.

## 6. Consistency + rollback (claims re-scoped after H2/L2; re-keyed after round-2 F-1/F-2)

- **Determinism, dual-keyed (narrowed in loop 2 — F-1):** the blanket "same repo state ⇒
  same analysis" is **withdrawn** (round-2 breaker proved it false for the stream half)
  and replaced by two honest keys:
  - **Structural half (`graph.csr` + structural meta):** pure function of
    **(git HEAD, tree)** ⇒ same repo state ⇒ same graph bytes, on any machine, at any
    historical sha. Downstream math is fixed-order deterministic
    (`csr.rs:8-10,24,162-165`; byte-identity PPR test `csr.rs:495-508`).
  - **Stream half (`state.json` — error EMA, loop):** pure function of the
    **machine-local transcript/CI byte set** under the canonical fold order (§4.1) ⇒
    same input bytes ⇒ same state bytes. The inputs are session-scoped and outside git
    (`check.sh:17-21`): two clones of the same sha legitimately show different error
    verdicts. Declared, keyed (`state_input_digest`), and DoD-tested at the boundary
    (§9 i) — not hidden behind the structural guarantee.
- **Idempotency (per half):** re-running `derive` with unchanged (HEAD, tree) AND
  unchanged stream bytes is a byte-identical no-op; each half's digest is keyed to its
  own inputs. The digest RED+GREEN tests (§9 a for the structural half, §9 i for the
  stream half) prove **derivation determinism**, and that is the only claim attached to
  them.
- **Projection reset (was "rollback"):** `rci derive --at <sha>` re-derives the
  projection at any historical commit. Pure function ⇒ idempotent trivially; no
  compensators, no head races. Round-1's claim "rollback is idempotent and safe under
  concurrent writes" is **withdrawn** (resolution H2); the concurrent case cannot arise
  because writes are lock-serialized and the operation is a recomputation, not an append.
  **Historical scope (F-1):** `--at <sha>` reconstructs the *structural* view
  bit-identically; the stream half is not a function of sha — its fields in a `--at`
  frame are labeled current-machine-stream, never presented as historical.
- **Code revert stays human:** RCI only suggests revert candidates (§4.1 analyzer 3);
  execution is always a human-run `git revert` (§8 negative guarantee).
- **Failure poles stay distinct** (mirrors `event_log.rs:255-268`): malformed source
  input (skip + log, never retry) vs store fault writing `.rci/` (typed
  `StoreError::{Open,Write,Flush,Sync}` discipline, `event_log.rs:166-176` — never
  silent loss; on fault, keep the previous snapshot and mark STALE).

## 7. Failure modes + degradation

Every stage: explicit timeout + fallback; zero cascade into the commit path.

| Failure | Detection | Behavior |
|---|---|---|
| git subprocess hangs | 5 s timeout on `Command` | derive aborts; previous snapshot kept, marked `STALE`; hooks pass (fail-open) |
| derive slower than commit rate / lock contention | lock wait > 5 s | skip this derive; next invocation catches up (pull model has no queue to back up) |
| `.rci/` corrupt / schema-version mismatch | load-time validation | delete `.rci/`, cold re-derive from git + transcripts (< 10 s, §2). Git is ground truth — stated as the design's spine, not a buried fallback |
| binary missing / crash | hook can't exec | hook no-ops with one logged line — commit path unaffected (identical to `check.sh:35-38`) |
| derive killed mid-write (SIGKILL / OOM / power loss) | next invocation | flock is OS-released on process death — **no wedge class exists**; tmp+rename atomicity ⇒ previous snapshot intact; loads clean or falls to cold re-derive (round-2 F-5) |
| snapshot older than HEAD | `derived_at != HEAD` | every frame carries `stale: true`; hooks treat STALE as pass (fail-open, `check.sh:22-24,39`); **hook prints one alarm line when lag > 50 commits** — wave-time starvation is visible to automation, not only to a human running `status` (round-2 F-5/F-6) |
| wide-commit flood (mass sweep lands) | F_max exclusion counter jumps | excluded from co-change by design (§4.1); surfaced in `rci status`, never silent |
| narrow-commit burst reshapes a node's neighbors (sub-F_max — invisible to `excluded_commits`) | churn alarm: Jaccard < 0.5 vs the node's set W=32 commits earlier | frame carries `churn_alert: true` + why; `rci status` counts it; ranking treated as suspect (round-2 F-3; residue R9) |
| cold derive slows as history grows (it is the recovery path) | measured cold derive > 30 s, or git-log stage > 15 s | R10 trigger fires: window co-change at the decay horizon (ε-preserving — decayed weights already send older commits to ε) + snapshot base; pre-planned, not improvised (round-2 F-6) |
| wrong predictions | backtest precision below floor | analyzer stays advisory; `RCI_ESCALATE` stays/roles back OFF; there is no blocking path to mis-fire (§0) |
| spectral O(n⁴) path | — | **unreachable in v1: RCI calls no spectral function.** Future organ must carry an explicit n≤32 code check before `eigenvalues()` — the kernel auto-selects dense with no refusal (`spectral.rs:206-213`; resolution M5) |

**Advisory vs blocking — decided:** v1 is advisory / fail-open, period. There is no
RCI_BLOCKING flag, no drift-gate feed. The only blocking point in the org remains the
existing flag-gated spectral drift gate (`event_log.rs:389-419`), untouched by RCI.
Preconditions for ever proposing a blocking coupling are pinned in §10 / resolution §F.

## 8. Security + tenant boundaries + negative guarantees

- **Input allowlist (hard boundary):** git object metadata, source-file paths, import
  statements, tool-outcome tokens (anonymized alphabet, `markov.rs:30-39`), closed-schema
  test results. **Denied:** `apps/*` runtime data, orders, customer records, any DB read
  against tenant tables. v1 has no DB connection at all (§5). No network egress; all
  sources local.
- **Red-line asymmetry (LOCK; resolution H4):** RCI **never** has blocking or blessing
  authority over red-line surfaces (money / auth / RLS / migrations globs). On those
  surfaces RCI output is one-directional: it may add friction (flag a concern), it may
  **never remove it** — a low blast-radius score is never readable as "safe."
  Every frame for a red-line path carries `red_line: true` + the fixed disclaimer:
  *structural ranking cannot clear this surface; run the red-line checklist.* This LOCK
  survives any future flag and any precision@k result — the aggregate baseline is
  survivorship-biased exactly on this class (breaker H4, accepted).
  **Rendering is one-directional too (round-2 F-4):** on `red_line: true` frames the
  blast-radius number and the co-change top-k are **suppressed entirely** — the frame
  prints `blast: suppressed — ranking withheld on red-line surfaces; run the red-line
  checklist`. Only concern-raising signals may render there (error innovation above
  threshold, `churn_alert`); reassurance-capable numbers never do — a low EMA is
  omitted, not printed. The automation-bias channel (a low number beside a disclaimer
  teaching the eye to skip the checklist) is removed structurally: there is no number to
  habituate on; the red-line frame is constant-shape across 40 benign changes and the
  41st dangerous one. Residual (human may still skip the checklist unprompted) = R11.
- **Negative guarantee (verbatim; resolution STOP-2):**
  > **«RCI НІКОЛИ авто-не-виконує revert реального коду й НІКОЛИ авто-не-емітить
  > компенсуючу подію в продакшн event_log; correction — завжди human-confirmed
  > suggestion, тим паче money/auth-суміжне.»**
  Enforced by a fence guard test (mirrors the bebop kernel-fence guards): the RCI binary
  has no write path to the production event log — no import of the kernel commit API;
  RCI writes only under `.rci/`.
- **NO-AGENT-SCORING (structural + guard; resolution N1):** the derived schema has no
  actor dimension — git author and any agent identity are dropped at parse; nodes are
  paths, edges are coupling. Guard test asserts no per-actor/per-author aggregation
  exists in any RCI output schema — the mirror of NO-COURIER-SCORING
  (`event_log.rs:22-23`), with the same visible pulse. RCI ranks *changes*, never
  *actors*.
- **No PII, no secrets, no error text in `.rci/`** (closed schema, §5 / resolution M2).
- **Cross-tenant:** rejected construction, §5 / resolution M3.

## 9. Operability

- **Observe in <1 min:** `rci status` *(proposal)* — one line per subsystem in the
  established style (`graph_energy_report`, `spectral.rs:362-375`):
  `derived_at=<sha> lag=0 | edges: import=9.1k cochange=37.2k excluded_commits=14
  churn_alerts=0 | err innov=0.1 (state_input=<digest8>) | lock=free | backtest p@10=0.71
  (red-line stratum: n/a — see LOCK) | ADVISORY`. Lock holder + age shown when held
  (F-5). Full detail: one `AnalysisFrame` JSON per path. Push-visibility companion: the
  advisory hook alarms once at lag > 50 commits (§7).
- **Health taxonomy:** `fresh` (derived_at == HEAD) / `degraded` (STALE — hooks
  pass-through, one alarm line) / `down` (binary or `.rci/` unusable — hooks no-op).
  Degraded ≠ down, reported distinctly.
- **Debug = re-derivation (scope keyed, F-1):** every frame carries `derived_at` +
  `state_input_digest`; `rci derive --at <sha>` reproduces any historical **structural**
  view bit-identically (§6); stream-half fields are current-machine and labeled so. No
  log archaeology for the structural half; for the stream half the digest names exactly
  which input set produced the verdict.
- **Rollback of RCI itself:** the §0 exit plan — delete hook line + binary + `.rci/`.
- **Flags:** `RCI_ENABLED` (default on, advisory), `RCI_ESCALATE` (default OFF; requires
  the measured backtest floor + operator flip), `RCI_SYMBOL_LEVEL` (default OFF, R1).
  **`RCI_BLOCKING` does not exist in v1** — reserved name; preconditions pinned in §10.
- **Verification plan (DoD):** *(Stage honesty, Counsel round 2: these guards are
  committed structure at design time; they become running structure only when green —
  "structurally resolved" must not be read stronger than that before implementation.)*
  - (a) structural determinism: same (HEAD, tree) ⇒ byte-identical `graph.csr` digest
    **regardless of transcript contents**; RED = mutate one edge weight ⇒ digest
    mismatch;
  - (b) projection-reset idempotence: `derive --at A` after `derive` at HEAD equals a
    fresh `derive --at A` byte-for-byte (pure-function property; structural half);
  - (c) backtest with **stratified scoring** (general vs red-line globs; strata never
    blended) + the E1 sign-split case (`engine/src/field_frame.rs:92` vs `csr.rs:307`,
    no import edge) kept as a **documented negative control** — the import channel is
    expected to miss it and the test asserts + records the miss (the tool declares its
    blind spot instead of hiding it; resolution H4);
  - (d) latency: cold derive < 10 s; incremental derive p95 < 1 s; `rci frame` p95
    < 100 ms on the §2 corpus;
  - (e) zero `.py` under `tools/loop-signals/` **and** an equivalent-coverage Rust e2e
    test replacing `test_transcript_e2e.py`, green **before** the `.py` deletion lands
    (deleting a test to go green is forbidden — test-integrity red-line);
  - (f) fence guard: RCI has no write path to the kernel production event log (no import
    of the commit API; writes only under `.rci/`);
  - (g) NO-AGENT-SCORING guard: no actor field in any output schema; no per-author
    aggregation **in any RCI output including the backtest report** (Counsel round-2
    physician-seam: scoring commits vs later fix-commits is one join away from scoring
    authors — fenced at the output);
  - (h) red-line frames: a path matching red-line globs yields `red_line: true` + the
    disclaimer **and the test asserts blast/ranking values are ABSENT** (suppression,
    round-2 F-4) (fixture: `apps/api/src/routes/orders.ts`);
  - (i) stream determinism + boundary (round-2 F-1/F-2): a FIXED transcript/CI fixture ⇒
    byte-identical `state.json` digest, invariant under input-file arrival order
    (canonical sort); RED = mutate one event ⇒ digest mismatch; **and the boundary
    test**: same (HEAD, tree) with two different transcript sets ⇒ identical `graph.csr`
    digest AND differing `state.json` — the declared limit asserted, not hidden;
  - (j) crash safety (round-2 F-5): SIGKILL a derive mid-write ⇒ next invocation
    acquires the lock immediately (OS-released flock) and loads an intact previous
    snapshot (tmp+rename atomicity);
  - (k) churn alarm (round-2 F-3): a synthetic burst of 32 two-file commits against one
    node trips `churn_alert: true` + the `rci status` counter.

## 10. Open / accepted risks + deferred organs (owner: operator, SyniakSviatoslav)

| # | Item | Status | Rationale / trigger |
|---|---|---|---|
| R1 | Module-level graph misses intra-file dynamics; symbol-level needs a parser (tree-sitter = new dep) | **DEFER + flag** (`RCI_SYMBOL_LEVEL`) | trigger: module-level **stratified** precision measured < 0.6 useful floor; dep decision = DECART report |
| R2 | Regex import extraction has false edges | **ACCEPT** | advisory-ranking noise bounded by the backtest oracle; a wrong edge cannot block anything (no blocking path exists) |
| R3 | Co-change = correlation, not causation | **ACCEPT v1, note v2** | PPR-over-co-change proven at scale for this ranking job; `cgraph.rs` d-separation is the named research lane |
| R4 | β=0.5, F_max=30, top-32 are stated tunables at birth | **ACCEPT** | tuned by the same backtest oracle; stated, not hidden. First backtest run MUST include an F_max sensitivity check {30, 64} + census of excluded 31–64-file commits — confirms or moves the literature default with THIS repo's data (resolution "H3 reconciled") |
| R5 | Wide-commit exclusion drops pairs from a genuinely-coupled 31-file commit | **ACCEPT** | such commits still produce touch/error signal; import channel still covers them; exclusion is logged + counted (never silent); genuine coupling recurs in focused commits and is re-learned there |
| R6 | Escalation flood / inertia | **FIXED** | `RCI_ESCALATE` default OFF + backtest-bootstrapped Wilson floor (resolution M4) |
| R7 | Future pgrust move | **OPEN — precondition pinned** | forward-only atomic migration; RLS ENABLE+FORCE if multi-scope |
| R8 | Brief's `incidence.rs` does not exist | **RESOLVED** | mirrors `csr.rs`/`spectral.rs`/`cgraph.rs`; §0 |
| R9 | Sub-F_max narrow-commit bursts can shape a node's neighbor list while pacing below the churn threshold | **ACCEPT (residue, alarmed)** | churn alarm (W=32, Jaccard<0.5) makes the fast path non-silent (DoD k); a repo-write-capable adversary already holds strictly stronger powers (edits the code the operator runs); payoff bounded to misdirecting advisory attention on non-red-line paths (rankings suppressed on red-line, F-4; no blocking path exists). Measured natural rate: 192/300 recent commits sit organically in the 2–30 band (round-2 F-3) |
| R10 | Cold re-derive is O(total history), unwindowed — and it is the recovery path; wave-time lock starvation leaves RCI STALE during bursts | **ACCEPT + DEFER-FLAG** | 847 commits today ⇒ <10 s with ~5× margin. Trigger: measured cold derive > 30 s (or git-log stage > 15 s) ⇒ window co-change at the decay horizon (ε-preserving by construction: decayed weights already send older commits to ε) + snapshot base. Starvation: frames truthfully `stale: true` (late, never wrong-and-fresh-looking); hook alarm at lag > 50 commits; `rci status` shows lock holder + age (round-2 F-6) |
| R11 | No test can observe whether the human actually runs the red-line checklist | **ACCEPT** | the named habituation channel (number beside disclaimer) is deleted structurally (F-4 suppression — red-line frames are constant-shape); measuring operator checklist compliance = instrumenting operator behavior, disproportionate — declined by name, not silently skipped (round-2 F-4 residual) |
| D1 | GraphDelta event chain (Option C machinery) | **DEFER — named trigger** | trigger: measured need for sub-commit granularity OR a non-replayable source (none today). Preconditions: CAS/lock-proven single-writer (H1); convergent rollback + concurrent-load DoD (H2) |
| D2 | Spectral drift / cascade organ | **DEFER — named trigger** | trigger: backtest shows an incident class PPR ranks poorly that quotient-spectrum ranks well. Preconditions: curated ≤32 partition map, never auto-dirs (M1); explicit n≤32 code check (M5); plain-language why (N3); **declare code-vs-swarm claim AND record the operator's explicit want/don't-want decision on reifying swarm mood as a repo-health number at all** — deliberation, not checkbox: a passing backtest (efficacy) can never swallow the values question (purpose) (Counsel §5, sharpened round 2) |
| D3 | Full n-D Kalman | **DEFER — named trigger** | trigger: measured EMA false-positive/negative rate insufficient |
| D4 | `RCI_BLOCKING` (any blocking coupling) | **DOES NOT EXIST in v1; reserved** | preconditions: recorded human decision (STOP-1); §0 SCOPE-RULE banner + intervention-lift subordination; stratified precision incl. red-line strata; **red-line LOCK — never authority over money/auth/RLS/migrations, no threshold overrides this** (H4) |
| D5 | Per-tenant / runtime-stream variant | **DEFER — named trigger** | trigger: any proposal to point RCI at runtime error streams. Rule: no cross-tenant fold (rejected construction); per-tenant derivation + tenant discriminant + new ADR + RLS FORCE (M3) |
