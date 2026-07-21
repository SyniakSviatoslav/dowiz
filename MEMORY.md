# dowiz / DeliveryOS — Living Memory (Agent-Agnostic)

> **Read this file first** before any code change. This is the single source of truth
> for project context, conventions, and status — not a specific agent's memory store.
> All agents (Claude, Hermes, OpenCode, any future model) read and write ONLY this file.

## Project Overview
- dowiz is a sovereign kernel (Rust, zero external deps) for a delivery-order system
- Architecture: kernel (decision engine) + agents (LLM-driven) + tools + intake adapters
- Primary language: Rust; secondary: TypeScript (SPA), shell scripts (CI)
- Repo root: `/root/dowiz/`; no workspace-level Cargo.toml (kernel is the root)
- Deploy targets: Hetzner VPS (prod), Tauri desktop (card-capture), Fly.io (staging)

## Architecture Quick-Map
| Layer | Path | Notes |
|-------|------|-------|
| Kernel | `kernel/` | Rust, zero deps, decision engine. Compiles to wasm32 |
| FDR | `kernel/src/fdr/` | Flight-Data Recorder — hand-rolled logger + ring. NOT tracing |
| Agent facade | `kernel/src/agent/` | LLM interface, model routing, dual-witness |
| Intake | `intake-adapters/` | Telegram adapter crate |
| Tools | `tools/` | native-spa-server, CLI helpers |
| Frontend | `apps/` | Tauri desktop, SPA |
| Blueprints | `docs/design/BLUEPRINT-*.md` | Every change starts with a blueprint |
| Audit | `docs/audits/hardening/` | HOT-PATHS.tsv, CHECKLIST.md |
| Lessons | `docs/lessons/` | Permanent learning records |

## Conventions (HARD)
1. **Zero external deps** — kernel compiles with no crates.io deps. `cargo tree -e no-dev` must be empty.
2. **Named absence, not silent omission** — every counter/stamp uses `Reading::Value(u64)` or `Reading::Unavailable(Absence::Variant)`. Never fabricate a 0.
3. **Optional-field discipline** — new fields on FdrEvent are `Option<T>`, present ONLY on their record class. Non-carrier records serialize byte-identical to before.
4. **Closed enums** — `Absence`, `Kind`, `WorkloadKind` are closed. New variants = conscious edit + `as_str`.
5. **P3 firewall** — span_id, parent_span_id, PMU, and work are forensic-plane. They NEVER feed hash, signature, idempotency, or replay surfaces.
6. **No ratio fields** — work/cost are raw u64 pairs. Efficiency is a consumer concern, not a schema field.

## Mesh Swarm Architecture (Agent Self-Organization)

### Core Principle
**No hierarchical orchestration.** Any agent, on its own discretion, selects necessary skills from this living memory based on context. Agents self-organize as a decentralized mesh swarm — like a murmuration of starlings, not a military hierarchy.

### Workflow Gates (MANDATORY for every task)
Every task follows this sequence, regardless of which agent handles it:

```
1. RESEARCH     — explore codebase, read docs, understand ground truth
2. SYNTHESIS    — combine findings into coherent understanding
3. CRITIQUE     — challenge assumptions, find gaps, stress-test logic
4. PLAN         — produce blueprint with explicit dependencies + falsifiable checks
5. CRITIQUE     — verify plan against live repo, check dependency graph
6. WORK         — implement per blueprint, TDD (RED→GREEN)
7. VERIFY       — DIFFERENT MODEL/AGENT reviews (never self-verification)
8. CRITIQUE     — reviewer challenges implementation, finds edge cases
9. COMMIT       — evidence in commit message, save to living memory
```

### Self-Verification Ban
**The model and agent NEVER checks its own work.** Only a different model or different agent may verify. This is structural, not aspirational — the verification step is a hard gate, not a recommendation.

### Living Memory as Coordination Layer
All positive and negative consequences are stored in this MEMORY.md. The memory is the swarm's shared nervous system — what one agent learns, all agents know. No agent operates in isolation.

### Skill Selection Protocol
1. Agent reads MEMORY.md to understand project state
2. Agent identifies required skills from the skill catalog below
3. Agent selects only necessary skills — no bloat, no unnecessary dependencies
4. Agent executes with selected skills
5. Agent writes results back to MEMORY.md

## Testing Rules
- `cargo test -p dowiz-kernel` — all kernel tests (currently ~1152)
- `cargo test -p intake-adapters` — adapter tests (~17)
- Golden-string tests in `fdr/schema.rs` pin exact JSON output
- TDD: write RED test first, then GREEN implementation
- Run `cargo clippy --all-targets` and `cargo test` before every commit
- No external test frameworks (no proptest, no quickcheck)

## Security Invariants
- Hydra: closure=NEVER, kill-switch only, command-filter (SHA3-256), breach-alarm (G9)
- P103 supervisor: dual-witness 2-of-2, drift-gated
- P97/P101: locked pair + CPU-only
- Intake firewall: `intake-adapters` produces `InboundMessage`, structurally cannot call `place_order`
- No recovery keys on wallet self-custody
- No `push --force` (worktree exception: force-with-lease allowed after fetch+ls-remote)

## Current Status (2026-07-21)
### Done — Verified (cargo test 1208 pass, 0 fail, clippy clean)
- Items 1-33, 36, 43, 45-46, 48, 52-54, 57, 58, 61, 62 DONE-VERIFIED
- P106 (AiMode→compose.rs), P48-INTAKE, R16-R19 (DoS hardening)
- FDR relational linkage (item 62): span_id, parent_span_id, SpanGuard threading
- WorkloadKind/Work schema (item 58): closed enum + optional field on FdrEvent
- Runtime counter closure (item 61): SpanGuard carries work, emit_subprocess_record updated
- Living-memory wave propagation finishing layer: committed
- **Blueprint-Unified: kernel parse module** (tsv.rs, env.rs, json_query binary) — 30 tests
- **Blueprint-Unified: MemorySearchEngine** (BM25 + trigram + PPR fusion) — 11 tests
- **Blueprint-Unified: TemporalPredictor** (Markov + spectral + absorbing + Noether + causal) — 9 tests
- **Blueprint-Unified: SwarmCoordinator** (DSU decomposition + executor selection + health) — 9 tests
- **Blueprint-Unified: AgentStep FanOut/Merge** variants added to agent loop
- **Mesh swarm architecture** documented in MEMORY.md — workflow gates, self-verification ban
- Total kernel tests: **1208 passed, 0 failed**

### Structural Gaps Found (2-question doubt check)
1. **Workflow gates are cultural, not structural** — no kernel code enforces the
   research→synthesis→critique→plan→critique→work→verify→critique→commit sequence.
   An agent can skip any phase and nothing in the kernel catches it. FIX NEEDED.
2. **MEMORY.md not universally read** — Claude reads its own ~/.claude/ memory store,
   Hermes reads ~/.hermes/memories/MEMORY.md, OpenCode reads AGENTS.md which points
   to a Claude-specific path. No hook, CI gate, or config ensures any agent reads
   /root/dowiz/MEMORY.md before acting. FIX NEEDED.
3. **json_query binary has no e2e test** — 7 unit tests for resolve_path() exist, but
   no integration test spawns the binary as a subprocess to verify stdin/args/exit codes.
   FIX NEEDED.

### Blocked / Gated
- Items 59/60: now unblocked (item 58 schema landed), ready to wire
- Item 66: gated on item 64 (composition root)
- Items 34-44: in worktree `exec/toy-pilot-arc`, cannot touch from main
- eval-layer retirement: operator ruling required
- Items 4-12: many gated on operator decisions (D1-D6)

### Next Steps (in priority order)
1. **FIX structural gaps** from doubt check (workflow gate, MEMORY.md accessibility, json_query e2e)
2. **PLL-inspired clock stabilizer** — stabilize irregular kernel ticks into consistent output
3. **ASCII knowledge index** — fast skill/capability lookup for all agents
4. **Skill patching** — adapt skills based on execution results
5. Items 59/60: wire per-kind effective-effort tracking (now unblocked)
6. Item 66: composition root
7. Item 64: operator-gated decisions
8. Items 4-12: await operator decisions (D1-D6)

## File Reference
| File | Purpose |
|------|---------|
| `kernel/src/fdr/schema.rs` | FDR event envelope, WorkloadKind, Work |
| `kernel/src/fdr/mod.rs` | SpanHandle/SpanGuard, SPAN_SEQ, emit functions |
| `kernel/src/fdr/pmu.rs` | PMU stamps, bracket, delta |
| `kernel/src/hydra.rs` | Hydra organism (691 lines) |
| `kernel/src/agent/model_pair.rs` | P103 supervisor |
| `kernel/src/agent/model_registry.rs` | P97/P101 registry |
| `kernel/src/agent/loop.rs` | AgentLoop + FanOut/Merge step types |
| `kernel/src/parse/tsv.rs` | Kernel-native TSV parser (replaces awk) |
| `kernel/src/parse/env.rs` | Kernel-native .env parser (replaces split) |
| `kernel/src/bin/json_query.rs` | JSON field extraction CLI (replaces node -e) |
| `kernel/src/memory_search.rs` | MemorySearchEngine (BM25 + trigram + PPR) |
| `kernel/src/predict.rs` | TemporalPredictor (Markov + spectral + causal) |
| `kernel/src/swarm.rs` | SwarmCoordinator (DSU + executor selection) |
| `kernel/src/ports/hub_intake.rs` | P48-INTAKE kernel port |
| `docs/audits/hardening/HOT-PATHS.tsv` | Hot-zone accounting (eff column) |
| `docs/audits/hardening/CHECKLIST.md` | Standing hardening law |
| `docs/design/ROADMAP.md` | Master roadmap |
| `docs/design/BLUEPRINT-UNIFIED-*.md` | Unified memory + extraction + prediction + swarms |

## Kernel-Native Search & Extraction (replacing grep/python)

### Available Primitives
| Primitive | Module | Replaces |
|-----------|--------|----------|
| BM25 ranker | `retrieval::bm25` | `grep -rn` lexical search |
| Trigram index | `retrieval::index` | `grep -F` literal search |
| Pattern matcher | `retrieval::pattern` | `grep -E` regex (restricted subset) |
| Recall fuser | `retrieval::recall` | Multi-signal search combination |
| PPR navigation | `csr::Csr` + `markov` | Graph-based relatedness |
| JSON parser | `json::parse` + `Value::get` | `node -e JSON.parse` |
| Readability | `readability::extract` | HTML content extraction |
| Frontmatter | `spine::parse_frontmatter` | YAML parsing |
| Log parser | `metrics::LogEvent::from_line` | Pipe-delimited parsing |

### Search Protocol
```
Agent needs to find something:
  1. Lexical: BM25 rank → top-k candidates
  2. Graph: PPR from seed → related sections
  3. Fuzzy: trigram index → approximate matches
  4. Fusion: combine scores → final ranked list
  5. Return: section + context + confidence score
```

### Extraction Protocol
```
Agent needs to extract structured data:
  1. JSON: json::parse() + Value::get("path") — replaces node -e
  2. TSV: parse_tsv(file, column) — replaces awk
  3. Env: parse_env(file) — replaces split('=') in TS
  4. HTML: readability::extract() — replaces browser parsing
  5. YAML: spine::parse_frontmatter() — replaces yaml.load
```

## Prediction Engine (Chronological-Topological)

### Available Prediction Primitives
| Primitive | Module | Predicts |
|-----------|--------|----------|
| Markov attractor | `markov::analyze()` | Next-state distribution, convergence |
| Spectral drift | `spectral::classify_drift()` | System trajectory class |
| Causal inference | `causal::backdoor_adjust()` | P(Y\|do(X)) interventional outcome |
| Absorption chain | `absorbing::expected_steps()` | Time to terminal state |
| Noether invariant | `noether::invariant_drift()` | Time to violation |
| Online learner | `online::LinearGaussNatural` | Next observation |
| Diffusion | `diffusion::related()` | Information spread |

### Prediction Use Cases
1. **Action prediction**: "Given last N actions, what's next?" → Markov + stationary
2. **Change prediction**: "If file X changes, what else updates?" → CGraph descendants + BM25
3. **Failure prediction**: "Is subsystem approaching failure?" → Spectral drift + Noether
4. **Performance prediction**: "How long will this take?" → Absorbing + online regression
5. **Intervention prediction**: "What if we change parameter P?" → Causal backdoor

## Sub-Agent & Swarm Coordination

### Swarm Substrate (existing primitives)
| Primitive | Swarm Role |
|-----------|-----------|
| `AgentLoop` | Per-executor loop |
| `spool::Spool` | Task queue |
| `token_bucket::child_bucket()` | Budget slicing |
| `breaker::Breaker` | Fault isolation per executor |
| `breaker::BreakerChain` | Swarm-wide kill switch |
| `ports::AgentBridge` | Security scoping |
| `mesh.rs` | Distributed coordination |
| `fdr::ring` | Swarm telemetry |
| `dsu.rs` | Task dependency grouping |
| `harmonic.rs` | Candidate ranking |
| `router.rs` | Task→executor dispatch |

### Swarm Coordinator Pattern
```
SwarmCoordinator
├── Task Decomposition (DSU + Router)
├── Execution (Spool + TokenBucket + AgentLoop)
├── Monitoring (Breaker + FDR + Harmonic)
├── Aggregation (MeshLog + spine.verify_chain)
└── Dynamic Adaptation (Markov + Spectral + Autonomic)
```

## Skills/Hooks/Memory Mechanism

### Cross-Agent Rules
1. **This MEMORY.md** is the agent-agnostic source of truth
2. **Never trust memory past its timestamp** — re-verify with `grep`/`git`/`cargo test`
3. **Ground truth outranks plans** — the live codebase is what IS, not what a plan says
4. **Update memory BEFORE coding** — record new facts to this file
5. **No push without explicit operator approval** — commit locally, operator decides push timing
6. **Scope-lock: only what the task explicitly says** — no "while I'm here" drift
7. **Self-verification ban** — different model/agent must verify, never the same one
8. **Mesh swarm self-organization** — agents select skills from this memory, no hierarchy
9. **Workflow gates mandatory** — research→synthesis→critique→plan→critique→work→verify→critique→commit
10. **All consequences → living memory** — positive and negative, nothing lost

## PLL-Inspired Clock Stabilizer (Kernel Oscillator)

### Concept
The kernel's oscillator (tick/timestamp/event stream) is like a PLL's VCO — it produces
irregular output. A PLL stabilizes an unstable frequency by comparing it to a reference
and feeding back an error signal. We apply the same pattern to kernel timing:

```
IRREGULAR INPUT          PLL STABILIZER              STABLE OUTPUT
+-----------+     +------------------------+     +---------------+
| raw ticks | --> | Phase Detector         | --> | aligned ticks |
| timestamps|     |   (compare to ref)     |     | timestamps    |
| events    |     | Loop Filter            |     | events        |
| agent     |     |   (smooth jitter)      |     | latencies     |
| actions   |     | VCO-equivalent         |     | predictions   |
+-----------+     |   (adaptive rate)      |     +---------------+
                  +------------------------+
```

### PLL Components → Kernel Mapping
| PLL Component | Kernel Mapping | Purpose |
|---------------|----------------|---------|
| Phase Detector | TickDiffer | Detect phase error between expected and actual tick |
| Loop Filter | EMA smoother | Dampen high-frequency jitter, preserve trend |
| VCO | Adaptive tick generator | Produce next tick based on filtered error |
| Reference Clock | Target tick rate (configurable) | Desired stability baseline |
| Lock Detector | Stability evaluator | Detect when system is locked vs free-running |
| Loop Bandwidth | Adaptation rate | How fast the stabilizer reacts to drift |

### Why This Matters
- Agent actions have variable latency (LLM calls, tool execution, I/O)
- Without stabilization, metrics, predictions, and scheduling are unreliable
- The PLL analogy gives us a proven feedback-control model
- Stabilized ticks → consistent FDR timestamps → reliable spectral analysis

## ASCII Knowledge Index (Fast Agent Lookup)

### Why ASCII, Not Graphs
All agents (Claude, Hermes, OpenCode) operate in terminal/CLI environments.
ASCII art is:
- Searchable with grep/ripgrep
- Renderable in any terminal
- Updatable without tooling
- Faster to navigate than visual graphs

### Index Structure (ASCII tree)
```
SKILL-CATALOG/
├── SEARCH/
│   ├── BM25 ............ retrieval::bm25 (lexical rank)
│   ├── Trigram ......... retrieval::index (fuzzy match)
│   ├── PPR ............. csr + markov (graph nav)
│   └── Fusion .......... retrieval::recall (multi-signal)
├── EXTRACTION/
│   ├── JSON ............ json::parse + Value::get
│   ├── TSV ............. parse::tsv (awk replacement)
│   ├── Env ............. parse::env (split replacement)
│   ├── HTML ............ readability::extract
│   └── YAML ............ spine::parse_frontmatter
├── PREDICTION/
│   ├── Markov .......... markov::analyze (next-state)
│   ├── Spectral ........ spectral::classify_drift (trajectory)
│   ├── Causal .......... causal::backdoor_adjust (P(Y|do(X)))
│   ├── Absorbing ....... absorbing::expected_steps (time-to-end)
│   └── Noether ......... noether::invariant_drift (symmetry break)
├── SWARM/
│   ├── Decompose ....... swarm::SwarmCoordinator::decompose (DSU)
│   ├── Select .......... swarm::SwarmCoordinator::select_executor
│   ├── Dispatch ........ swarm::SwarmCoordinator::dispatch
│   ├── Health .......... swarm::SwarmCoordinator::health
│   └── FanOut/Merge .... agent::AgentStep variants
├── STABILITY/
│   ├── PLL ............. clock_stabilizer (tick alignment)
│   ├── Breaker ......... breaker::Breaker (fault isolation)
│   ├── TokenBucket ..... token_bucket (budget control)
│   └── Circuit ......... breaker::BreakerChain (swarm kill)
├── SECURITY/
│   ├── Hydra ........... hydra (closure=NEVER)
│   ├── P103 ............ agent::model_pair (dual-witness)
│   ├── P97/P101 ........ agent::model_registry (locked pair)
│   └── Intake .......... intake-adapters (InboundMessage)
└── LIFECYCLE/
    ├── FDR ............. fdr::schema + fdr::ring
    ├── Span ............ fdr::SpanGuard
    ├── Mesh ............ mesh (cross-repo gossip)
    └── Spine ........... spine::verify_chain
```

### How Agents Use This Index
1. Agent reads MEMORY.md on startup
2. Agent identifies task type (search, predict, swarm, etc.)
3. Agent looks up the ASCII tree for the relevant primitive
4. Agent calls the kernel-native function (no external tools)
5. Agent records results back to MEMORY.md
