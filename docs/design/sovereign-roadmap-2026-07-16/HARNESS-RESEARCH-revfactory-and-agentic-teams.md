# Harness Research — revfactory/harness + the agentic-team genre vs dowiz's own harness

> Research report, 2026-07-16. Requested gap-analysis on four axes: **logging · metrics ·
> organization · planning**. Sources: live WebFetch of github.com/revfactory/harness,
> revfactory.github.io/harness, github.com/revfactory/harness-100; two research passes over
> AutoGen / LangGraph / CrewAI / MetaGPT / claude-flow official docs; and dowiz's own files
> (cited per line below). **No adoption is decided here** — every candidate is tagged
> `PATTERN` (reimplement Rust-native/zero-dep, house style) or `DECART` (new dependency →
> requires a decart comparison report per `docs/operating-model/integration-decart-rule.md`,
> never silent adoption).
>
> Self-watch (per BRAIN-TOPOLOGY 2026-07-16 lesson): external quality claims below are
> **self-reported by their authors** and marked as such; dowiz-side statements carry file
> citations so they are checkable, not authority.

---

## 1. What revfactory/harness actually is

A **Claude Code plugin acting as a meta-skill**: prompt it with a domain description
("build a harness for website development") and it *generates* an agent team — writing
`.claude/agents/*.md` (roles, principles, I/O protocols, team-communication contracts) and
`.claude/skills/*/SKILL.md` (YAML frontmatter, aggressive trigger descriptions, Progressive
Disclosure: metadata → body → references) into the target repo.

**Six-phase generation workflow:** domain analysis → team-architecture design → agent
definition generation → skill generation → integration/orchestration → validation
(trigger verification, dry-run, with-skill vs without-skill comparison).

**The six team-architecture patterns** (selection keyed to task dependency structure):

| Pattern | When |
|---|---|
| Pipeline | strictly sequential dependent tasks (gen → review → test → deploy) |
| Fan-out/Fan-in | parallel independent tasks + aggregation |
| Expert Pool | context-dependent selective invocation of specialists |
| Producer-Reviewer | generate-then-verify quality cycles |
| Supervisor | central coordinator, dynamic task distribution |
| Hierarchical Delegation | top-down recursive decomposition |

**Two runtime modes:** Agent Teams (TeamCreate + SendMessage + TaskCreate, for 2+
collaborating agents) vs plain Subagents (one-off Agent-tool dispatch).

**Quality claims (self-reported, not verified):** "+60% average quality (49.5→79.3),
15/15 win rate, −32% variance" from an **author-measured A/B, n=15**, on a sister repo;
their own FAQ pairs every citation with "third-party replications pending." They also claim
the effect grows with difficulty (+23.8 basic / +29.6 advanced / +36.2 expert). Treat as a
directional hypothesis, not a fact.

**harness-100:** 100 prebuilt harnesses × EN+KO across 10 domains (content, dev/devops,
data/ML, business, education, legal, health, comms, ops, specialized). Each: 4–5 specialist
agents + 1 QA/reviewer agent, 1 orchestrator skill + 2–3 agent-extending skills; dependency
DAGs, error strategies (retry/skip/fallback), scale modes (full/reduced/single-agent),
explicit trigger boundaries (should-trigger AND not-trigger). Install = `cp -r
en/NN-name/.claude/ yourproject/.claude/`.

**Critical negative finding:** the harness docs contain **no logging, metrics, cost-tracking,
or telemetry mechanism at all**. It is a team-*generation* library, not a runtime. Its
transferable value to dowiz is exactly two things: (a) the named six-pattern taxonomy with a
selection rubric, (b) the "meta-skill that writes the team files" generation move.

---

## 2. The wider genre on the four axes

### (a) Logging
- **AutoGen**: runtime emits **OpenTelemetry spans** per agent message (GenAI semantic
  conventions), exportable to any OTel backend (their example: Jaeger); plus two stdlib
  loggers — human-readable trace and structured event stream (`ToolCallRequestEvent`,
  `ToolCallExecutionEvent`).
- **LangGraph/LangSmith**: every node execution = a **run in a trace tree** (inputs, outputs,
  run type, timing, `usage_metadata`), stored in LangSmith, queryable/replayable per project.
- **CrewAI**: typed event bus (`AgentExecutionStarted/Completed/Error`,
  `TaskStarted/Completed/Failed`) dual-piped to OTel; storage delegated to the backend.
- **MetaGPT**: every turn is a structured `Message {content, cause_by, sent_from, send_to}`
  in a shared pool; the whole team state serializes to **one `team.json` checkpoint**, and
  `--recover_path` **resumes an interrupted run exactly where it stopped** — a checkpoint,
  not just an audit log.
- **claude-flow**: a **SQLite store** (`.swarm/memory.db`, 12 tables: events, workflow_state,
  performance_metrics, consensus_state…) — per-agent activity as SQL-queryable rows, fed
  deterministically by Claude Code PreToolUse/PostToolUse hooks.

Genre consensus: **per-agent-turn structured records with a run-correlation ID, captured
automatically (never by agent discipline), queryable after the fact.**

### (b) Metrics
- **LangSmith**: the most granular — `usage_metadata` with input/output/total tokens *and*
  sub-breakdowns (cache-read, reasoning tokens), auto-priced per model, rolled up per node →
  per trace → per project, with P50/P99 latency + error-rate + cost dashboards; plus a full
  eval harness (datasets, LLM-as-judge, pairwise, trajectory evals over threads).
- **AutoGen**: `RequestUsage(prompt_tokens, completion_tokens)` per model call,
  `gather_usage_summary(agents)` across a team; known gap: streaming doesn't populate usage.
- **MetaGPT**: built-in `CostManager` with a **hard `max_budget`** — prints running cost per
  call and **raises/aborts when the budget is exceeded** (`--investment` flag).
- **CrewAI**: delegates to AgentOps (`agentops.init()` auto-instruments every LLM call →
  per-agent spend dashboard).
- **claude-flow**: performance_metrics tables + a benchmark system that evaluates
  coordination strategies/topologies systematically.

Genre consensus: **token/cost auto-capture per call, per-agent and per-team rollup, and (in
MetaGPT's case) budget as a hard runtime constraint, not a memory rule.**

### (c) Organization
- **AutoGen**: role = `name` + `description` (used programmatically for routing) + system
  message; overlap prevented **structurally** by pub/sub — an agent only receives message
  types/topics it subscribed to.
- **LangGraph**: role = a graph node + its slice of the shared state schema; exactly one node
  holds control at any instant (`Command(goto=…)` handoffs).
- **CrewAI**: declarative `agents.yaml` (role/goal/backstory) + `tasks.yaml`; each task has
  exactly **one owner agent**; `allow_delegation=False` prevents delegation loops; documented
  failure mode: hierarchical manager can still misroute on fuzzy role text.
- **MetaGPT**: the cleanest primitive — `RoleContext.watch()` subscribes a role to specific
  **action types** (`cause_by`); the environment only delivers matching messages, so a role
  *cannot* act outside its declared subscriptions. Anti-overlap by type system, not convention.
- **claude-flow**: markdown agent definitions (like dowiz/revfactory) + topology choice
  itself as the collision-avoidance mechanism (mesh/hierarchical/ring/star).

### (d) Planning (task → team topology)
- **AutoGen**: first-class team classes — `RoundRobinGroupChat` (fixed order),
  `SelectorGroupChat` (LLM picks next speaker per turn), `Swarm` (decentralized handoff via
  tool call). Topology static at construction; sequencing dynamic per turn.
- **LangGraph**: named patterns — **network** (peer-to-peer), **supervisor**,
  **hierarchical** (supervisors-of-supervisors); graph shape static, edge firing dynamic.
  Current docs recommend tool-based handoffs over the packaged supervisor for context control.
- **CrewAI**: `Process.sequential` / `Process.hierarchical` (manager LLM assigns at runtime)
  + `Flow` event-driven DAG (`@start/@listen/@router`) for branching/fan-in.
- **MetaGPT**: fully **static SOP assembly line** (PRD → design → code → QA) defined by role
  subscriptions; no meta-orchestrator reshapes the team.
- **claude-flow**: the outlier — **topology chosen and reshaped dynamically at runtime** by
  an "adaptive queen" (mesh ↔ hierarchical switching, agent count scaled up/down by feedback).
- **revfactory**: the six-pattern taxonomy above, chosen once at generation time.

So the genre's planning spectrum is: static-SOP (MetaGPT) → static-topology/dynamic-routing
(AutoGen, LangGraph, CrewAI) → generation-time pattern selection (revfactory) →
runtime topology reshaping (claude-flow).

---

## 3. dowiz's current harness on the same four axes (file-cited)

### (a) Logging — partially ahead in kind, behind in structure
**Has:**
- Zero-dep JSONL ledgers, one per event kind — 13 live ledgers (`alert, bench, benchmark,
  blueprint, health, metric, note, phase, plan, plan_step, session, task, trajectory`) in
  `tools/telemetry/logs/*.jsonl`, written by `log_event` (`tools/telemetry/lib.sh:69`), each
  row `{ts, kind, host, …}` — plus a Telegram bridge with an async spool + 429-aware rate
  control (`tg_deliver`/`tg_send`, `lib.sh:57–141`).
- **Transcript interoception the genre does not have**: `tools/loop-signals/` runs a Markov
  attractor detector over the agent's *own session transcript* (first-order chain, real
  eigenvalues via Faddeev-LeVerrier + Durand-Kerner, SLEM, entropy rate, progress-vs-probe
  state alphabet) — `check.sh` answers "am I stuck in a loop right now?" with a deterministic
  verdict. Nothing in AutoGen/LangGraph/CrewAI/MetaGPT/claude-flow analyzes its own event
  stream spectrally.

**Lacks:**
- **No run-correlation ID.** Ledgers are per-*kind*, not per-*run*: there is no field tying a
  `bench` row, a `task` row, and a `plan_step` row to the same swarm wave or subagent
  dispatch. LangSmith's trace tree / claude-flow's SQL rows are exactly this join.
- **No per-agent-turn record.** A subagent dispatch (SWARM waves W1-1…W22) leaves only its
  final prose report + whatever the MAIN agent re-verifies; there is no
  `{run_id, unit, model, tokens, ms, verdict, files_touched}` row per dispatch.
- **No query layer.** Reading the ledgers today = grep + inline python heredocs
  (`lib.sh:200–248`). No replay/checkpoint analog to MetaGPT `--recover_path`.

### (b) Metrics — mechanism ahead of the genre, **feeding starved**
**Has (genuinely ahead):**
- **EV-driven model routing**: `gov_record` → `track_record.jsonl {ts,model,task,success,
  value,cost}`; route = max-EV survivor under a ruin cap (`EV = p·v − (1−p)·c`,
  `ruin=(q/p)^budget`), ½-Kelly lane width, served by the native `hermes-kernel` binary in
  4–5ms (`tools/telemetry/governance.sh:38–72,177–183`; design:
  `docs/design/SWARM-GOVERNANCE-DESIGN.md §A`). No framework in the genre routes model
  selection by measured EV with a ruin constraint — they statically config a model per agent.
- **False-claim meter** — claimed-vs-verified integrity ledger (`gov_falseclaim`,
  `governance.sh:239–267`): `false-estimation%` and `false-positive-of-done%`. The genre
  measures cost/latency; **nobody measures honesty**. Unique.
- `bench_run` wall-ms + peak-RSS per op (`lib.sh:168–194`), learned ETA/latency models
  (`gov_learn`/`gov_anu`, `governance.sh:270–323`), host resource sampling, real benchmark
  reports (`tools/telemetry/BENCHMARKS-FRICTION-2026-07-15.md`).

**Lacks:**
- **Auto-capture.** `track_record.jsonl` has **9 rows**; `precedents.jsonl` has **3**
  (counted 2026-07-16). `gov_record` is called by hand. Meanwhile every Claude Code session
  transcript (`~/.claude/projects/*/*.jsonl` — the same files loop-signals already parses)
  contains per-call token usage that is never harvested. The genre's core lesson is that
  metrics only work when captured automatically per call (LangSmith, AgentOps, CostManager).
- **No hard budget abort.** Token thresholds exist as memory rules
  (`token-lifecycle-thresholds`), and the kernel computes Chernoff spend-guards, but nothing
  aborts a run at a cap the way MetaGPT's `max_budget` raises.
- **No per-team/per-wave rollup** (follows directly from the missing run ID).

### (c) Organization — stronger separation-of-powers, weaker enforcement
**Has:**
- A 3-plane, 7-role architecture (`docs/agents/README.md`): design plane (system-architect /
  system-breaker / counsel — build/break/weigh as *independent stimuli*), loop plane
  (loop-architect certifies structure / loop-orchestrator runs usage — an explicit
  quality↔usage split), object plane (worker / error-fix). Live definitions in
  `.claude/agents/*.md` with **tool allowlists as structural constraints** (breaker/counsel/
  critics are read-only; "Breaker proposes no fixes"; "Architect never self-marks resolved").
- **Adversarial anti-collusion the genre lacks**: judges drawn from a `JUDGE_POOL` disjoint
  from the author, citation gate (`gov_judge_gate` RED-rejects verdicts without
  `CITES:/DISTINGUISHES:/NO-BINDING-PRECEDENT`), precedent registry with stare-decisis bind
  gate τ=0.82 (`SWARM-GOVERNANCE-DESIGN.md §C–D`). revfactory's QA-reviewer agent is a toy
  next to this.
- **Disjoint file lanes**: every swarm manifest has an "Owns (disjoint)" column per unit
  (`docs/design/SWARM-MANIFEST-2026-07-15.md` W1 table; `SWARM-MANIFEST-2026-07-16.md` §1) —
  dowiz's file-ownership analog of MetaGPT's pub/sub.

**Lacks:**
- **Machine-checkable lanes.** "Owns" is prose in a markdown table; nothing verifies a
  subagent's diff stayed inside its globs. MetaGPT's `watch()` makes out-of-scope action
  *undeliverable*; dowiz relies on brief-writing discipline + MAIN re-verification.
- **No typed inter-agent messages.** Communication is file-mediated
  (`proposal.md → breaker-findings.md → resolution.md`, `/council` steps 1–5) — auditable
  and durable (a genuine plus) but schema-free; no `cause_by`-style routing if agent count grows.

### (d) Planning — richer process taxonomy, missing topology taxonomy
**Has:**
- A **certified loop library**: 11 loops in 3 families with statuses
  (`docs/agents/loops/REGISTRY.md`), each loop a structured YAML card (id, intent,
  problem_signature, preconditions, iron_principles, loop_body, exit_conditions, gates,
  proof_artifacts, out_of_scope, escalation, memory_file — see
  `loops/error-fix-convergence.md:9–33`). **M1–M11 certification + anti-cheat dry-run on a
  real broken fixture before dispatch** — the genre has *nothing* like loop certification.
- A dispatch discipline: 4-condition classification ("is this even a loop?"), REUSE /
  ADAPT-PARAMS / DELEGATE-to-architect, health checks, HARVEST of run memory
  (`.claude/commands/loop-orchestrator.md`; role: `docs/agents/roles/loop-orchestrator.md`).
- **4 of revfactory's 6 patterns already practiced, unnamed**: SWARM manifests = fan-out/
  fan-in with waves + dependency order + per-unit RED→GREEN gates + "distrust subagent green,
  MAIN re-verifies" (`SWARM-MANIFEST-2026-07-16.md §4`); `/council` = producer-reviewer
  (attack/examine/resolve rounds, `.claude/commands/council.md`); build-stage → audit-gate →
  error-fix → exit-audit = pipeline (`REGISTRY.md:25`); loop-orchestrator = supervisor.
  A quantitative fan-out cost model even exists (`tools/telemetry/swarm_proof.py`).

**Lacks:**
- **A named topology taxonomy + selection rubric.** Loops are *process* patterns (what
  discipline governs the work); nothing declares *team shape* (how many agents, in what
  communication structure, chosen by what task property). Every swarm manifest is bespoke
  hand-writing; the choice "fan-out vs pipeline vs single agent" is made by feel each time.
- **Expert Pool and Hierarchical Delegation unformalized** — the specialist bench exists
  (`.claude/agents/`: sentinel, guardian, critics, test-scout) but there's no routing rubric
  for selective invocation; waves never recursively decompose.
- **No manifest generator.** revfactory's one real trick — a meta-skill that emits the team
  files from a domain prompt — has no dowiz analog: loop-architect certifies loops but no
  role drafts a swarm manifest from a blueprint mechanically.
- **Honest status caveat:** per `docs/design/organism-status-2026-07-15/ORGANISM-STATUS.md`,
  the executive layer is 100% reactive, all 10 governance hooks are operator-suspended
  no-ops (2026-07-15 directive, conscious), and 9/11 registry loops are still DRAFT
  (uncertified). Much of the machinery above is built and correct but **not continuously
  exercised** — the same "stranded organ" pattern the organism audit documents.

---

## 4. Gap table

| # | Axis | dowiz HAS (better/equal) | dowiz LACKS | Adoption route |
|---|---|---|---|---|
| 1 | Logging | 13 JSONL ledgers + TG spool (`lib.sh`); Markov transcript interoception (`tools/loop-signals/`) — genre has no analog | run-correlation ID joining ledger rows to a wave/dispatch | **PATTERN** — add `run_id`/`unit` kv to `log_event` call sites; zero dep |
| 2 | Logging | final-report + MAIN re-verify per subagent | per-agent-turn record `{run_id, unit, model, tokens, ms, verdict, files}` (LangSmith run-tree / claude-flow rows) | **PATTERN** — new `agent_turn.jsonl` via existing `log_event`; harvest from session JSONL already on disk |
| 3 | Logging | grep+python ledger reads | SQL-queryable store (claude-flow `.swarm/memory.db`) | **PATTERN** — `rusqlite` already in-tree (`tools/deep-clean`); no *new* dep. OTel export instead = **DECART** (collector dep; likely rejected on sovereignty) |
| 4 | Logging | — | checkpoint/resume of a multi-agent run (MetaGPT `team.json` + `--recover_path`) | **PATTERN** — swarm manifest + per-unit status row is 80% of it; formalize resume-from-ledger |
| 5 | Metrics | EV/ruin/½-Kelly routing on measured track-record (`governance.sh`, kernel) — ahead of entire genre | **automatic** token/cost capture feeding `track_record.jsonl` (now 9 rows) | **PATTERN** — parse `~/.claude/projects/*/*.jsonl` usage into `gov_record`; same source loop-signals already parses; zero dep |
| 6 | Metrics | false-claim meter (claimed vs verified) — unique | hard budget abort mid-run (MetaGPT `max_budget` raises) | **PATTERN** — kernel already computes spend-guard; add cap check to dispatch path |
| 7 | Metrics | `bench_run` ms+RSS, learned ETA/latency models | per-wave/per-team cost rollup + P50/P99 views (LangSmith dashboards) | **PATTERN** once #1/#5 exist (one aggregation script). LangSmith/AgentOps SaaS = **DECART**, near-certain reject (external telemetry sink violates sovereign stance) |
| 8 | Organization | 3-plane roles, read-only tool allowlists, disjoint judges, citation gate, precedent registry — stronger than genre | machine-checkable Owns lanes (diff ⊆ declared globs) | **PATTERN** — small check script vs manifest table; mirrors existing `tools/verify-scope.sh` idea; zero dep |
| 9 | Organization | file-mediated artifacts (auditable) | typed message routing / subscriptions (MetaGPT `cause_by`) | **PATTERN** if ever >2-hop agent chains appear; not urgent at current scale |
| 10 | Planning | certified loop library (M1–M11 + anti-cheat) — genre has nothing comparable | named **topology taxonomy + selection rubric** (revfactory's 6 patterns; LangGraph's network/supervisor/hierarchical) | **PATTERN** — one table in the swarm-manifest template + a CLASSIFY extension in loop-orchestrator |
| 11 | Planning | bespoke swarm manifests (high quality, hand-written) | manifest **generator** (revfactory meta-skill move: blueprint → team files) | **PATTERN** — a `swarm-architect` skill mirroring the existing loop-architect/orchestrator split. Installing revfactory/harness or harness-100 as a literal plugin = **DECART required**; expected verdict: reject as dep (content library, self-reported n=15 evidence, zero telemetry — its gap is exactly dowiz's strength), harvest as pattern |
| 12 | Planning | static waves + EV lane-width (quantitative scale-mode) | runtime topology reshaping (claude-flow adaptive queen) | **PATTERN**, low priority — dowiz's EV routing is the sounder quantitative version; revisit only if long autonomous runs return |

---

## 5. Recommendations (ranked by leverage)

**R1 — Auto-feed the governance ledgers from session transcripts (metrics; gap #5).**
Highest leverage by far. dowiz's most differentiated asset — EV routing + false-claim meter —
is starving on 9 hand-written rows. The genre's single deepest lesson is that metrics exist
only when captured automatically per call. The per-call token usage is *already on disk* in
the session JSONL that `tools/loop-signals/transcript_events.py` parses; a harvest step
(loop-orchestrator step 7, or a small Rust addition to hermes-kernel) folding each dispatch
into `gov_record {model, task_type, success, value, cost}` makes `gov_route` real instead of
aspirational. Zero new deps.

**R2 — Run-correlation ID + `agent_turn` ledger (logging; gaps #1–2).**
Add `run_id`/`unit` to `log_event` call sites and write one row per subagent dispatch
(`{run_id, unit, model, tokens, ms, verdict, files_touched}`). This is the LangSmith
trace-tree / claude-flow SQL idea at JSONL cost, and it is the prerequisite for per-wave
cost rollups (#7). Pairs naturally with R1 (same harvest pass).

**R3 — Name the topology taxonomy; add it to the swarm template + orchestrator CLASSIFY
(planning; gap #10).** Write the six patterns (pipeline, fan-out/fan-in, expert pool,
producer-reviewer, supervisor, hierarchical delegation) with dowiz-native selection criteria
into the swarm-manifest template and the loop-orchestrator's step-1 test, mapping each to the
existing practice (council = producer-reviewer, waves = fan-out/fan-in, build chain =
pipeline, orchestrator = supervisor) and formalizing the two missing ones (expert pool =
selective `.claude/agents` invocation rubric; hierarchical = wave-of-waves). This captures
~all of revfactory's transferable value in one doc page, no dependency.

**R4 — Machine-checkable Owns lanes (organization; gap #8).**
A deterministic check comparing each dispatched unit's `git diff --name-only` against the
manifest's Owns globs; violation = RED. Converts dowiz's strongest existing convention into a
ratchet — exactly the house philosophy (deterministic gates over prose discipline), and a
counterweight to the append-only/self-certification bias the BRAIN-TOPOLOGY research flagged.

**R5 — Hard budget guard in dispatch (metrics; gap #6).**
MetaGPT-style `max_budget` abort wired to the kernel's existing ruin/Chernoff math: a cap the
dispatch path checks, not a memory rule the agent is trusted to remember. Small; depends on
R1 for live spend numbers.

**Explicitly NOT recommended without DECART:** installing revfactory/harness, harness-100,
claude-flow, or any LangSmith/AgentOps/OTel sink as a dependency. Each is a new integration
under `integration-decart-rule.md` and needs the comparison table + probe first. On current
evidence the likely verdicts are: revfactory → harvest pattern, reject dep (no runtime, no
telemetry, self-reported n=15 evidence); external telemetry SaaS → reject (sovereignty);
OTel/SQLite → only if JSONL provably fails (rusqlite is already in-tree, so even SQL needs no
new crate).
