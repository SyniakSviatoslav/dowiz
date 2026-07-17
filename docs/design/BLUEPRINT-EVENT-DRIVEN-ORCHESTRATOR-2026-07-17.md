# BLUEPRINT — Event-Driven Gap Pipeline + Native Orchestrator ("GapWire") — 2026-07-17

> Status: PROPOSED (design artifact only; no production code in this change).
> Author: system-architect subagent, 2026-07-17, branch `feat/harness-llm-backend`.
> Style contract: plain prose, no metaphor; every load-bearing claim carries a `file:line`
> cite, a live-command ground, or an explicit **(proposal)** / **(training-knowledge)** tag.
> Protocol: Detailed Planning Protocol (AGENTS.md:160-219) — ground-truth-first, explicit
> dependencies, inline DECART, blueprint-grade signatures, falsifiable done-checks,
> 2-question doubt audit, Anu/Ananke check (AGENTS.md:223-261).

---

## 0. Ground truth — what exists, what does not (verified this session)

The operator's ask presumes an "orchestrator agent" and an "event-driven gap pipeline."
Per the standing methodological rule (design the missing piece against what partially
exists), here is the honest inventory:

**Exists (real, verified):**

| Piece | Where | What it actually is |
|---|---|---|
| Loop dispatcher concept | `.claude/commands/loop-orchestrator.md:1-29` | A *prompt*, human-triggered (`$ARGUMENTS`), that classifies a request (4-condition test), matches `loops/registry.md`, dispatches a worker, supervises, harvests memory. **Not event-driven; runs only when a human/agent invokes the command.** |
| Loop builder/certifier | `.claude/agents/loop-architect.md:10-37` | BUILD/VERIFY/IMPROVE with M1–M11 rubric; the sanctioned structural-change path for loops. |
| Programmatic loop gate | `kernel/src/loops.rs:1-31` (BP-20) | Machine-checkable DRAFT→CERTIFIED status; a DRAFT loop cannot be dispatched by an ungated file edit. |
| Deterministic intake gate | `kernel/src/intake.rs:1-19` (BP-08 `admit()`) | UNSAT ladder + under-determined + non-reproducible-verify checks, std-only, fail-closed. |
| Crash-safe async queue (pure) | `kernel/src/spool.rs:1-42` | Append/claim/ack/reclaim-stuck state machine with backpressure; explicitly designed as "the kernel-native async channel reused by every subsystem" (`spool.rs:19-20`). |
| Proven JSONL spool adapters | `tools/telemetry/rust-spool/src/main.rs:1-21`, `tools/async-spool/src/main.rs:1-29` | Producer appends one JSON line (microseconds, never blocks); single long-lived drainer sends; drop-on-ok, deadletter-on-parse-failure, retry-on-transient. Multi-process producers → one queue file is already the production pattern here. |
| Content-addressed event log | `kernel/src/event_log.rs:1-23,275-419` | SHA3-256 content-id chain, idempotent `Duplicate` no-op, `commit_after_decide` (Law before persist), drift gate. **Single-writer only** (`event_log.rs:18-19`), which is exactly what RCI breaker H1 proved breaks under concurrent producers. |
| Budget primitive | `kernel/src/token_bucket.rs` (used by `llm-adapters/src/dispatch.rs:1-16`) | Degrade-closed spend ceiling; `Dispatcher` already emits `track_record.jsonl` EV rows (`dispatch.rs:30-54`). |
| Loop registry | `loops/registry.md:6-23` | 3 CERTIFIED loops (error-fix-convergence, design-convergence, mobile-polish), 9 DRAFT. **No gap-research loop exists.** |
| Self-mod discipline (design + partial code) | `BLUEPRINT-P15-living-organism-unbounded.md` §2-3 | `self_mod_loop.rs` propose → `deliberate()` mirror → apply-iff-converged, fail-closed; red-line floor `HumanGated`; P15 §3 generalizes it to `HubPolicy` revisions, hard-gated on P10 kill-switch (P15 §1). |
| Living-memory blueprint | `docs/design/internal-retrieval-living-memory-blueprint.md` §0-3 | Memory = MD notes + 462-wikilink graph today; pgrust store planned; L0-L3 retrieval stack; TTL = demote-never-delete. |

**Does not exist (the gap this blueprint fills):**

- No mechanism by which a finding made *inside* a worker/research agent ("`incidence.rs`
  doesn't exist", "P06 key_V blocks 3 arcs", "only the untrusted-price order path is wired")
  becomes anything other than prose in that agent's report. Grep for any gap-event emitter:
  none. Findings propagate only if the parent agent or the operator happens to read and act.
- No standing orchestrator process. `loop-orchestrator` is invoked per-request; nothing
  watches a queue, triages, or dispatches autonomously.
- No single registry of open gaps. They are scattered across blueprints' "open risks"
  tables, memory arc files, and `⚠` markers — each with a different owner and no shared
  lifecycle.
- No automatic plan/roadmap reconciliation. `ROADMAP-GROUND-TRUTH-2026-07-14.md:6` says
  "Ground truth always outranks this plan" but the outranking is performed by whoever
  notices.

---

## 1. Problem + non-goals

**Problem.** Ground-truth verification is the highest-yield activity in this repo's sessions
(the Detailed Planning Protocol's step 1 exists because of it, AGENTS.md:170-175), and it
constantly produces *findings that are not the current task*: missing components, broken
assumptions, blocked dependencies, doc-vs-code contradictions. Today each finding's survival
depends on human/parent-agent diligence — a direct Ananke failure (AGENTS.md:249-250:
"a plan fails Ananke when its quality depends on a future reader's diligence"). The fix is a
pipeline where emitting a finding is cheaper than not emitting it, propagation is structural,
and triage-to-task is deterministic.

**Non-goals.**
- Not a production/tenant feature. Dev-plane, operator-owned, zero tenant data (same hard
  boundary as RCI proposal §8).
- Not a general workflow engine and not a replacement for the loop-architect's M1–M11
  certification authority. Structural loop changes still route through loop-architect only
  (`.claude/commands/loop-orchestrator.md:19`).
- Not an auto-editor of canon/ADR/roadmap. The pipeline *proposes* plan revisions; the
  operator merges (mirrors the canon-diffs-proposed-not-applied pattern of the mesh arc).
- Not agent evaluation. Gap events carry `found_by` identity for provenance, never for
  scoring — NO-AGENT-SCORING, mirroring NO-COURIER-SCORING (`event_log.rs:22-23`) and the
  RCI counsel's N1 recommendation (`realtime-change-intelligence-2026-07-17/counsel-opinion.md` §3 N1).
- Not a second memory system. The gap registry is markdown-in-repo with wikilinks — the
  exact substrate the living-memory blueprint already indexes (§2: "174 flat .md files …
  462 [[wikilinks]] (a real graph)"); it becomes one more note type, not a parallel store.

---

## 2. Back-of-envelope (mandatory)

| Quantity | Value | Ground |
|---|---|---|
| Gap findings per heavy session | ~10–40 | estimate from this session's own output (incidence.rs, key_V, order-path, E1 sign split, eqc-drift precedent…) |
| Peak day, parallel fan-out | ≤ 200 events | estimate (8-agent swarms are the recorded max) |
| Event size (claim-check discipline, §4.2) | ≤ 1 KB target, hard cap 4 KB | design cap (proposal) |
| Queue growth | 200 KB/day worst case | derived |
| Producer cost per emission | one `write()` of one line — microseconds | proven contract, `rust-spool/src/main.rs:9-11` |
| Drainer triage per event | pure-fn table lookup + hash — < 1 ms | derived from `intake.rs`/`spool.rs` scale |
| Auto-dispatched research task | minutes of LLM time, dominated by the model | — |
| Auto-dispatch budget | default ≤ 5/day (TokenBucket), degrade-closed | design cap (proposal) |

Conclusion: the pipeline itself is never the bottleneck; the LLM work it dispatches is.
Therefore **one** single-threaded drainer is sufficient at 10× growth; anything more
(priority heaps, sharded queues, a workflow server) fails ponytail/YAGNI on these numbers.
Zero Postgres connections (file-backed; pgrust upgrade is a later seam, §4.5).

---

## 3. PART 1 — the event-driven gap pipeline

### 3.1 Event schema — `GapEvent` (proposal)

New kernel module `kernel/src/gap.rs` (pure, std-only, mirroring `spool.rs`/`intake.rs`
conventions):

```rust
pub enum GapKind {
    MissingComponent,      // referenced thing does not exist (incidence.rs case)
    BrokenAssumption,      // doc/plan claim contradicted by live code or command output
    BlockedDependency,     // X blocks arcs/phases (P06 key_V case)
    DocContradiction,      // two docs assert incompatible things
    RepeatedLoopFailure,   // loop memory shows accumulating failures (routes to loop-architect)
    StaleDoc,              // plan superseded by landed code, not yet marked
}

pub enum GapSeverity { Blocker, High, Med, Low }

pub struct GapEvent {
    pub kind: GapKind,
    pub severity: GapSeverity,
    pub repo: String,            // "dowiz" | "bebop-repo" | worktree name
    pub subject: String,         // path or arc id the gap is about
    pub claim: String,           // one-sentence finding, ≤ 280 chars
    pub evidence: String,        // file:line cite OR sha3 digest of a command output
                                 // stored beside the queue — NEVER raw output inline (§4.2)
    pub affected: Vec<String>,   // doc/arc paths whose plans this invalidates
    pub found_by: String,        // agent/session id — provenance only, never scored
    pub suggested: String,       // optional one-line suggested action
}

impl GapEvent {
    pub fn canonical_bytes(&self) -> Vec<u8>;          // fixed field order, length-prefixed
    pub fn content_id(&self) -> [u8; 32];              // event_log::sha3_256(canonical_bytes)
}
```

`content_id` deliberately excludes timestamp and `found_by`-session-suffix noise so the
*same* gap found twice (by two agents, or re-emitted after a crash) dedups to a structural
`Duplicate` no-op — the exact idempotency law of `event_log.rs:5-7`.

### 3.2 Transport — "fastest path to the top", concretely

The fastest path is the one already proven twice in this repo: **fire-and-forget JSONL
append + single background drainer** (`rust-spool`, `async-spool`). Not a priority queue
service, not a direct escalation RPC — at §2 volumes both are over-engineering, and a
blocking escalation call would put network/LLM latency back on the finder's critical path,
which is the precise disease `spool.rs:3-8` was written to cure.

- **Emit:** a finder (any agent, any worktree) runs `gap emit --kind broken-assumption
  --sev high --subject kernel/src/incidence.rs --claim "..." --evidence "csr.rs:46" ...`
  (thin subcommand of the `tools/gap-wire` binary, or the shell one-liner in
  `tools/telemetry/lib.sh`). It appends **one line** to
  `~/.cache/gap-wire/queue.jsonl` and returns. Host-level path (not repo-local) is
  deliberate: the three active worktrees (`/root/dowiz`, `/root/dowiz-agentic-mesh`,
  `/root/dowiz-spectral-evolution`) share one queue, so a gap found in any lane reaches the
  same orchestrator; the `repo` field disambiguates.
- **Escalate (Blocker only):** the drainer additionally appends a `dest: "telegram"` line
  to the existing async-spool queue (`async-spool/src/main.rs:10-14`) — the operator sees a
  Blocker within one pacing gap (3.5 s contract, `async-spool/src/main.rs:36-43`). Zero new
  network code.
- **Priority:** no priority *queue*. The drainer claims all pending records (they are tiny),
  sorts by `(severity, id)` in memory, and processes Blockers first. Priority is a drain-order
  policy, not a data structure.

### 3.3 Concurrency — the H1 lesson, applied instead of repeated

RCI breaker H1 (`realtime-change-intelligence-2026-07-17/breaker-findings.md:11-24`) proved
that pointing multiple concurrent producers at the hash-chained `EventLog` forks the chain
silently: `append` reads `tip()`, computes the id, `set_tip()` with no compare-and-swap
(`event_log.rs:293-311`), and `MemEventStore` documents itself as "single-node … not shared
across processes" (`event_log.rs:18-19`). GapWire's topology makes that fork **impossible by
construction, not by locking**:

1. **Producers never touch the chain.** They append unordered, idempotent lines to the
   queue file. Order between producers is irrelevant because gap events are independent
   facts, not a causal chain — this is the structural difference from RCI's `GraphDelta`
   stream, where order changes the fold.
2. **Exactly one writer folds.** The drainer (the orchestrator, §5) is the *sole* process
   that appends drained events to the hash-chained ledger
   (`~/.cache/gap-wire/ledger.jsonl`, each line carrying `prev` = SHA3 of the previous
   line, reusing `event_log::sha3_256` — the `spine.rs:1-23` pattern) and the sole writer
   of `docs/gaps/GAP-REGISTRY.md`. Single-writer funnel ⇒ no fork, no CAS needed.
3. **Torn-line honesty instead of an atomicity assumption.** POSIX guarantees `O_APPEND`
   offset atomicity; whether one multi-byte `write()` on a regular file can interleave with
   another is platform behavior, not a guarantee we build on. So the design does not *rely*
   on append atomicity: every producer writes one complete line per `write()` (≤ 4 KB cap),
   and the drainer classifies any non-parseable line as deadletter
   (`<queue>.deadletter`, never silently lost — the exact contract of
   `async-spool/src/main.rs:22-26`), while content-id idempotency makes producer re-emit
   safe. A torn line is detected and quarantined; it cannot corrupt state.

### 3.4 Triage — what auto-creates a task vs. what needs a human

Deterministic, pure function in `kernel/src/gap.rs` (testable without I/O):

```rust
pub enum Route {
    RegisterOnly,                          // registry row + GAP file; no dispatch
    AutoResearch { loop_id: String },      // dispatch certified research loop
    ImproveQueue,                          // RepeatedLoopFailure → loop-architect queue
    OperatorGated { reason: String },      // registry row + Telegram; human decides
}
pub fn triage(ev: &GapEvent, policy: &TriagePolicy) -> Route;
```

| Condition | Route | Rationale |
|---|---|---|
| `subject`/`claim` matches red-line categories (money/auth/RLS/migrations/`.claude/`/secrets) | `OperatorGated` | never-bypass-human-gates standing rule; same categories `self_mod.rs` hard-refuses (P15 §3.3) |
| `DocContradiction` touching canon/ADR/roadmap | `OperatorGated` | canon changes are operator merges (mesh-arc precedent: "canon-diffs proposed, not applied") |
| Cross-repo action implied (`repo != "dowiz"`) | `RegisterOnly` | cross-branch rule: bebop files → bebop-repo, never fixed from here |
| `RepeatedLoopFailure` | `ImproveQueue` | the loop-architect IMPROVE path already exists for exactly this (`loop-orchestrator.md:27`) |
| `MissingComponent` / `BrokenAssumption` / `BlockedDependency`, severity ≤ High, target loop CERTIFIED, budget available | `AutoResearch` | research/planning is doc-only output — the safe automation class |
| Budget exhausted, or gap-research loop not yet CERTIFIED | `RegisterOnly` | degrade-closed (`dispatch.rs:4-5` discipline): queued as a row, never dropped, never silently downgraded |
| Everything else | `RegisterOnly` | default-conservative |

Three structural guards on `AutoResearch`:

- **BP-20 gate:** dispatch requires `LoopStatus::Certified` compiled from the registry
  (`loops.rs:14-31`) — a DRAFT loop cannot be auto-dispatched, programmatically, not by
  prose. Today **no gap-research loop exists**, so v1 launches with `AutoResearch`
  structurally unreachable until loop-architect BUILDs and certifies
  `gap-research-convergence` (Wave 2, §7). Honest consequence: the pipeline is
  register-and-notify-only on day one, and that is correct sequencing, not a defect.
- **TokenBucket budget:** `try_acquire` per auto-dispatch (default 5/day, operator-tunable);
  refusal ⇒ `RegisterOnly`. Prevents a gap-storm (e.g. a doc sweep emitting 100
  `StaleDoc` events) from consuming the session token budget.
- **Auto-dispatched output is doc-only:** the research loop's contract writes
  `docs/gaps/GAP-<id>-research.md` + a proposed plan diff. It never edits code, canon, or
  other plans. Escalation beyond docs goes back through triage as a new event or to the
  operator.

### 3.5 "Update the roadmap and affected plans" — without two processes corrupting a doc

The failure to avoid is documented twice in this repo: RCI H1 (concurrent chain-forking)
and the AGENTS.md shared-working-tree incident (a TOCTOU race where a concurrent process's
staged files leaked into another agent's commit, AGENTS.md:263-299). The mechanism:

1. **Single authority, single writer, append-mostly.** `docs/gaps/GAP-REGISTRY.md` (one
   table row per gap: id, kind, severity, subject, status
   `OPEN|RESEARCHING|OPERATOR-GATED|RESOLVED|ACCEPTED-RISK`, links) plus one
   `docs/gaps/GAP-<id>.md` per gap. Only the orchestrator writes these files. No other
   process ever does — enforced the same way loop cards are (convention + the orchestrator
   is the only thing wired to do it), and verifiable (`git log --format=%an -- docs/gaps/`).
2. **Plans are not patched in place by the hot path.** Affected plans are *linked from* the
   gap (`affected` field → registry row → wikilink), and each plan gains at most a one-line
   standing pointer ("open gaps against this doc: see GAP-REGISTRY") added once at plan
   creation, not per event. The registry outranks the plan the same way ground truth already
   outranks the roadmap by declared rule (`ROADMAP-GROUND-TRUTH-2026-07-14.md:6`).
3. **Reconciliation is a batch task, isolated, operator-merged.** When accumulated gaps
   genuinely invalidate a roadmap/blueprint (e.g. 3+ OPEN gaps against one doc, or any
   Blocker), the orchestrator dispatches a *consolidation task*: a doc agent running with
   `isolation: "worktree"` (mandatory per AGENTS.md:276-284) that drafts the plan revision
   (`⚠ CORRECTED` markers, superseded sections) as a commit for operator merge. Two
   processes can therefore never hold the same doc open: the hot path only appends to files
   it exclusively owns; the batch path runs in its own worktree and index.

---

## 4. PART 2 — the orchestrator: research + DECART

### 4.1 The field, honestly surveyed **(training-knowledge — no network egress from this
host: `crates.io` → 403, live probe recorded in P15 §9; treat version-specific claims as
to-be-reverified when egress opens)**

Current (2024–2026) orchestration technique clusters and their load-bearing ideas:

1. **Durable execution / workflow-as-code** — Temporal/Cadence (event-sourced workflow
   histories + deterministic replay; Temporal's core SDK is notably Rust), Restate
   (Rust-implemented server, journaled RPC handlers), Inngest, DBOS (durable execution
   inside Postgres). Load-bearing idea: *every workflow state transition is an event in a
   log; crash recovery = replay; side effects made idempotent*.
2. **DAG/batch engines** — Airflow, Dagster, Prefect. Load-bearing idea: static dependency
   graphs, scheduled; wrong shape for reactive event-driven dispatch.
3. **Job queues** — pg-boss (Postgres, this repo's own product-plane canon), BullMQ, Celery;
   Rust-native: `apalis`, `underway` (both tokio + Redis/Postgres backends). Load-bearing
   idea: *claim/ack with visibility timeout and dead-lettering* — which `spool.rs` already
   implements as a pure state machine (append/claim/ack/`reclaim_stuck`).
4. **Agentic-framework orchestration** — LangGraph (checkpointed graph state machines),
   AutoGen (conversation-driven), CrewAI (role crews), OpenAI Agents SDK handoffs,
   Claude-Code-style supervisor→subagent fan-out (this repo's actual working mechanism,
   used all session). Load-bearing ideas: *supervisor/worker hierarchy; explicit handoff
   contracts; checkpoint state outside the model context*.
5. **Classical planning** — hierarchical task networks (HTN: decompose task → method →
   primitive). Load-bearing idea: a library of *vetted decompositions* rather than free-form
   planning — which is exactly what `loops/registry.md` + loop cards already are: the HTN
   method library, with M1–M11 as the vetting.
6. **Supervision discipline** — Erlang/OTP supervision trees: restart strategies,
   let-it-crash, heartbeats. Load-bearing idea: *liveness is monitored by a supervisor that
   holds no business state*.

### 4.2 DECART — candidates × criteria

| Candidate | Bare-metal / native fit | Falsifiable correctness | Runtime cost | Supply chain | Reversibility | Verdict |
|---|---|---|---|---|---|---|
| **Temporal / Restate (server + SDK)** | ✗ standing server + DB; Restate server is Rust but is a *service dependency*; Temporal server Go+Cassandra/PG | replay model is proven, but unverifiable here without the server | heavyweight for ≤200 events/day | new external service; `cargo add` currently 403 (P15 §9) | poor — orchestration logic becomes SDK-shaped | REJECT (adopt the *journal+replay concept* only) |
| **apalis / underway (Rust job-queue crates)** | ✗ tokio — contradicts the repo's recorded DECART that chose `ureq`/no-tokio (AGENTS.md:184; `dispatch.rs:1-8` "WITHOUT tokio") + Redis/PG backing (pgrust not landed) | crate tests exist | moderate | blocked today (crates.io 403) | moderate | REJECT |
| **LangGraph / CrewAI / AutoGen** | ✗ Python runtime — violates native-Rust / adapters-only standing rule | — | new interpreter + dep tree | pip supply chain | poor | REJECT |
| **pg-boss** | ✗ Node runtime; product-plane pattern (canon for the *product's* queues), not dev-plane | — | needs Postgres | — | — | REJECT for this plane |
| **Full HTN planner (build)** | native, but new | hard to falsify | — | none | — | REJECT — `loops/` registry already is the method library; a second planner = dual authority (`markov.rs:1-8` lesson) |
| **Minimal native drainer on existing kernel primitives** ← | ✓ std-only kernel fn + one `tools/` binary (serde/ureq-class deps only in tools, per the compile-firewall pattern `event_log.rs:12-14`) | every piece pure/testable: `triage()` pure fn, `Spool` proven, ledger hash-chain RED-testable | one long-lived process, same footprint as `telemetry-spool` (already runs) | zero new deps in kernel; tools reuse vendored serde/ureq already in tree | trivial — delete binary + hook lines; registry stays as plain docs | **ADOPT (build)** |

**DECISION:** build the minimal native primitive — `tools/gap-wire` drainer +
`kernel/src/gap.rs` pure triage — because (falsifiable reason) every capability the external
candidates would add (claim/ack crash safety, idempotent replay, budget, certified-method
dispatch) **already exists in-tree as tested primitives** (`spool.rs`, `event_log.rs`
content-id, `token_bucket.rs`, `loops.rs`), and the only currently-executable integration
path is in-tree anyway (crates.io egress 403). Adopted *concepts*, credited: durable
execution's journal-everything (ledger = event-sourced state; orchestrator holds no
in-memory-only state), OTP supervision (`reclaim_stuck` + heartbeat line in the ledger),
HTN (loops registry as the method library).

**Older-as-adapter note:** the `.claude/commands/loop-orchestrator.md` prompt is **kept,
not purged** — it remains the LLM-facing dispatch surface (INTAKE→CLASSIFY→MATCH→…→HARVEST).
The native orchestrator does the machine half (watch, triage, budget, ledger, registry) and
*invokes* the prompt-half as its dispatch mechanism for LLM work. Division of labor, not
replacement.

**Mandatory probe (strongest honest argument against building):** hand-rolled orchestrators
rot into unprincipled state machines; Temporal exists because resumable multi-step workflows
are genuinely hard (partial completion, at-least-once side effects). Mitigations, and the
honest ceiling: v1 tasks are single-step (dispatch one research loop, doc-only output) with
idempotent effects keyed by content-id, and crash recovery is `reclaim_stuck` + re-dispatch
(safe because idempotent). **Upgrade trigger, named:** the day GapWire needs multi-step
sagas with compensation (e.g. dispatch → wait → merge → notify chains), stop extending the
drainer and re-run this DECART with durable-execution replay as the default candidate.

### 4.3 The orchestrator, concretely (proposal)

`tools/gap-wire/src/main.rs` — one long-lived process (systemd unit beside
`telemetry-spool`'s), single-threaded loop:

```
loop:
  1. read queue.jsonl → parse each line (unparseable → .deadletter, preserved)
  2. for each new event: content_id dedup against ledger → Duplicate = skip
  3. sort pending by (severity, id)
  4. route = gap::triage(ev, policy)          # pure, deterministic
  5. effect (idempotent, keyed by content_id):
       RegisterOnly   → append ledger line + write docs/gaps/GAP-<id>.md + registry row
       OperatorGated  → same + async-spool Telegram line
       ImproveQueue   → same + append to loops/memory improve-queue (loop-architect input)
       AutoResearch   → same + bucket.try_acquire → dispatch (claude -p / Task fan-out
                        invoking the loop-orchestrator command with the loop contract)
                        → record dispatch row in ledger (claim); ack on harvest
  6. remove processed lines from queue (drop-on-ok, the rust-spool contract)
  7. heartbeat ledger line every N min (liveness observable in <1 min)
```

`TriagePolicy` is data (`~/.config/gap-wire/policy.toml`): severity thresholds, auto-dispatch
allowlist by kind, daily budget, Blocker notification switches. Policy-as-data is what makes
§6's P15 integration possible.

Living-memory wiring (the "right hand knows the memory" requirement): every `GAP-<id>.md`
carries `[[wikilinks]]` to its affected arcs/docs, so gaps enter the exact graph the
living-memory blueprint indexes and diffusion-ranks (blueprint §2-3, L3 relatedness layer);
when the pgrust `memory_notes` store lands, gap files ingest like any note — no parallel
memory system, per non-goal. The orchestrator's MATCH step reads the registry + loop memory
(`loops/memory/<id>.md`) before dispatch, which is precisely the loop-orchestrator prompt's
step 2 health check made mechanical.

---

## 5. Relationship to RCI (shared vs. distinct — asked explicitly)

**Genuinely distinct systems.** RCI folds *code-change* deltas into a topology projection
(what breaks when file X changes); GapWire ingests *agent-reported research/planning*
findings (what the plan wrongly assumes). Different event kinds, different chains, different
consumers; merging them would recreate the free-form-`meta` leak surface RCI's breaker M2
flagged, on a wider input.

**Shared infrastructure and shared lessons:**
- Same substrate primitives (`sha3_256` content-id, spool pattern, ledger-chain), and the
  single-writer-funnel topology in §3.3 is the concrete resolution RCI's own H1 needs —
  when RCI's council round resolves H1, this is the precedent to cite (many producers →
  unordered idempotent queue → one folding writer).
- Same claim-check discipline: GapWire's `evidence` field is a cite or a digest, never raw
  command output inline — the direct fix for the M2 class (free-form `meta`/`TestResult`
  carrying secrets/PII into an append-only chain).
- Same NO-AGENT-SCORING boundary (counsel N1): `found_by` is provenance; any aggregation
  into per-agent reliability is out of scope and should get the same guard-test treatment
  N1 asks of RCI.
- Future join, flagged not built: an RCI `Unstable`-drift verdict is a legitimate GapWire
  *producer* (`kind: BrokenAssumption, subject: <supernode>`), which answers the RCI
  counsel's §5 open question (swarm-health signal routed to a triage point that can name a
  task, without naming an agent).

---

## 6. PART 3 — real-time autoupgrade: plugging into P15, not duplicating it

P15's self-modification design is already specific (P15 §3): candidate revision →
`deliberate()` mirror dialogue → apply-iff-converged, fail-closed; floor gates (schema
validation, snapshot-root, test-count); red-line fields permanently `HumanGated`
(`self_mod.rs:59-61,144` per P15 cites); every propose/reject/apply an immutable audit
event. And P15 §1's ordering argument is binding: no self-modifying machinery before the
Phase-10 kill-switch exists. This blueprint builds **no second autoupgrade mechanism**;
it defines three attachment points:

1. **The orchestrator's `TriagePolicy` is a `HubPolicy`-analog** — deliberately shaped as
   config-as-data (§4.3) so that when P15 §3's generalized `HubPolicyRevision` machinery
   lands, a policy self-revision (e.g. "raise auto-dispatch budget 5→8 because the measured
   dispatch success rate cleared a floor") travels the *same* propose→mirror→apply-iff-
   converged path, serialized and audited the same way. Until P10+P15 land, policy changes
   are operator edits to the TOML — stated plainly, hard-gated, per P15 §1. Red-line rows
   of the triage table (§3.4 rows 1–2) are the policy's `HumanGated` floor: no revision may
   ever move them out of `OperatorGated`, mirroring P15 §3.3 exactly.
2. **Loop upgrades already have their autoupgrade path — GapWire feeds it.** The
   loop-architect IMPROVE mode + orchestrator HARVEST step
   (`.claude/commands/loop-orchestrator.md:27`, `.claude/agents/loop-architect.md:21`) is
   the existing, certified mechanism for upgrading the system's *methods* in flight.
   `RepeatedLoopFailure` gap events route into that queue (§3.4) — GapWire makes the
   trigger event-driven instead of harvest-time-only; the upgrade mechanism itself is
   untouched.
3. **Forward-compatibility with P15 §6 capability minting:** when per-agent minted
   capability tokens exist, the orchestrator is a natural minting point — an auto-dispatched
   research agent receives an attenuated, doc-only-scoped token
   (`Scope::is_subset_of` narrow-only attenuation, P15 §6.2). Flagged as the integration
   seam; nothing minted now. The orchestrator's own binary never self-updates outside P15
   §7's eqc-gate-or-deny path, if ever.

---

## 7. Waves, dependencies (re-derived), falsifiable done-checks

```
W0 (schema + emitter)  ──►  W1 (drainer + registry)  ──►  W2 (certified gap-research loop → AutoResearch live)
                                      │
                                      └──►  W3 (consolidation task, worktree-isolated, operator-merged)
P10 kill-switch (NOT this blueprint) ──►  W4 (TriagePolicy self-revision via P15 §3)   [HARD GATE]
```

W0⊥nothing; W1 needs W0's schema only; W2 needs W1 running + loop-architect certification
(independent of W3); W3 needs W1's registry to have content; W4 is not schedulable here at
all — it is P15 §3 work consuming this blueprint's policy-as-data shape. Re-derivation
check: W2 and W3 are mutually independent (parallel-safe); the draft order W2→W3 was
sequence-of-writing, not necessity — corrected per protocol step 2.

**Done-checks (each RED-first at implementation time):**

- **W0:** `cargo test -p dowiz-kernel gap::` — (a) `content_id` stable across timestamp/
  session variation, distinct across claim variation; (b) `triage()` table test: every
  red-line-category fixture routes `OperatorGated`; a DRAFT-loop fixture can never yield
  `AutoResearch` (BP-20 gate); budget-exhausted fixture yields `RegisterOnly`.
- **W1:** kill -9 the drainer mid-drain with 5 pending events → restart → all 5 present in
  ledger exactly once (claim/ack + content-id dedup); a hand-corrupted queue line lands in
  `.deadletter` byte-identical, and processing continues; emit the same gap from two
  worktrees concurrently → exactly one registry row; ledger hash-chain walk detects a
  mutated line (spine-pattern RED test); heartbeat visible within 60 s of start.
- **W2:** `loops/registry.md` gains `gap-research-convergence … CERTIFIED` with an M9
  anti-cheat dry-run report in `loops/reports/` (on a known-bogus gap fixture the loop must
  go RED/escalate, not "research successfully"); one end-to-end auto-dispatch produces
  `docs/gaps/GAP-<id>-research.md` and **zero** non-`docs/gaps/` file modifications
  (`git status --porcelain` assertion).
- **W3:** a consolidation run executes in its own worktree (`git worktree list` shows it),
  produces a proposed-revision commit touching only the named plan docs, and the operator
  merge remains the only path to `main`-visible roadmap text.

---

## 8. Doubt audit (2-question ritual) + Anu/Ananke check

**Q1 — what would make this design wrong?** If agents don't emit. The pipeline's value is
zero at zero adoption, and no mechanism here *forces* a finder to run `gap emit`. Honest
mitigations, in strength order: (a) make emission a one-liner cheaper than writing the
finding into prose (it becomes the *citation* for the prose); (b) a standing AGENTS.md rule
("a ground-truth contradiction found mid-task MUST be emitted as a gap event before the
task report is written") — **proposed here for the operator to add; this blueprint does not
edit AGENTS.md**; (c) loop cards' Exit+Memory block gains a `gaps_emitted` field so the
harvest step audits it (a loop-architect parametric change). Residual risk accepted and
owned by the operator: (b) is diligence-based until (c) exists — a named Ananke debt, not a
hidden one.

**Q2 — least-verified load-bearing claim?** That severity/kind triage can be decided
*deterministically* from producer-supplied fields — i.e., that producers label severity
honestly and consistently. A mislabeled `Low` on a real blocker delays escalation; a
`Blocker` habit inflates Telegram noise. Mitigation is structural, not hopeful: severity
only changes *notification speed*, never safety — the red-line/canon rows of the triage
table route `OperatorGated` on **subject/kind match, independent of severity**, so an
under-labeled red-line gap still cannot auto-dispatch. Mislabeling therefore degrades
latency, not correctness. The claim that *latency* degradation is acceptable at §2 volumes
is the remaining unverified part — measured after W1 (count of re-triaged rows), flagged.

**Anu (does it follow?):** the dependency graph re-derives cleanly (§7); the build-native
decision is derived from in-tree evidence (existing primitives + egress 403), not asserted;
the one place a decision rests on unverifiable ground is §4.1's 2026 framework survey —
labeled **(training-knowledge)** with a named re-verification trigger rather than presented
as checked fact. No sibling-doc contradiction found: this design *consumes* RCI's H1/M2
findings and P15's ordering argument rather than contradicting them; it leaves the
loop-architect's M1–M11 authority and the BP-20 gate intact.

**Ananke (is the good outcome structural?):** single-writer registry — structural (only one
process wired to write). Chain-fork impossibility — structural (topology, §3.3). DRAFT
never auto-dispatched — structural (`loops.rs` compiled gate, not prose). Red-line gaps
never auto-actioned — structural (triage table rows independent of producer labels). Budget
overrun — structural (TokenBucket, degrade-closed). Torn-line corruption — structural
(deadletter + idempotent re-emit). The two named non-structural residues: emitter adoption
(Q1, debt named) and registry-single-writer being convention-plus-audit rather than an OS
lock (accepted: a second writer is detectable via `git log -- docs/gaps/`, and the cost of
an flock-style enforcement is deferred until a violation is ever observed).

## 9. Open items for the operator

| # | Item | Why it is the operator's |
|---|---|---|
| O-A | Add the "emit gaps" standing rule to AGENTS.md (§8 Q1 mitigation b) | AGENTS.md is doctrine; agents don't self-edit it |
| O-B | Certify `gap-research-convergence` (loop-architect BUILD → M1–M11) before any AutoResearch | loop-architect is the only certification authority |
| O-C | Default budget (5/day) and Blocker-Telegram routing — confirm or retune | spend + attention policy |
| O-D | W4 stays frozen until P10 kill-switch lands | P15 §1 hard gate, restated |
| O-E | Whether bebop-repo gets its own emitter writing to the same host queue (`repo: "bebop-repo"`, `RegisterOnly`-max) | cross-repo scope decision |

---

## 10. Audit addendum (2026-07-17, appended — fault-isolation audit pass; design above unchanged)

The Phase-27 audit (`BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md` §1.2)
verified two defects in the exact adapters this blueprint cites as its proven pattern, which the
implementation waves must NOT inherit:

1. **Head-of-line wedge (finding A1, CRITICAL).** `async-spool/src/main.rs:366-381` — a
   `Sendable` entry that keeps failing past `MAX_ATTEMPTS` stays queued and is retried forever,
   starving every entry behind it; `rust-spool/src/main.rs:240-247` has no send-failure deadletter
   at all. §3.2's "the exact contract of `async-spool/src/main.rs:22-26`" therefore covers only
   the *parse-failure* deadletter; the *send/effect-failure* case is a live wedge in both cited
   adapters. Binding consequence for W1: the drainer's effect step (registry write, Telegram line,
   dispatch) gets an attempt cap, after which the event moves to `.deadletter` **and the drainer
   advances** — one undeliverable effect must never stall triage of the events behind it. Add to
   W1's done-checks: queue = [event whose effect permanently fails, ordinary event] ⇒ ordinary
   event fully processed, failed one deadlettered within the cap.
2. **Unbounded deadletter/ledger growth (finding A6-class, HIGH).** `async-spool`'s deadletter
   file is append-only with zero retention (`async-spool/src/main.rs:107-111,186-201`), and this
   design's own `ledger.jsonl` is append-only forever with §2 bounding only the *queue*. State the
   growth story explicitly at W1: ledger rotation-by-size with hash-chain carry-over (last line's
   digest seeds the next file), deadletter alarmed via `gap-wire status` above a named size —
   caps stated in policy.toml, per the Phase-27 "growth bound" rule (§6 there).
