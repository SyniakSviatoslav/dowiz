# BLUEPRINT — Unified Living Memory + Kernel-Native Extraction + Prediction + Swarms

- **Date:** 2026-07-21 · **Status:** PLAN MODE — needs operator approval before implementation
- **Scope:** Cross-cutting: memory unification, kernel-native tooling, prediction engine, swarm coordination

---

## 0. Problem Statement

Today:
- Each agent (Claude, Hermes, OpenCode) has its own memory store — no shared context
- Scripts use `grep`/`awk`/`node -e JSON.parse` for extraction/parsing — slow, fragile, not kernel-native
- No chronological-topological prediction — system reacts, doesn't predict
- No sub-agency/swarm coordination — single-agent only

Goal: **ONE living memory**, kernel-native primitives for all extraction/search, prediction engine using existing spectral/Markov/causal primitives, and dynamic swarm coordination.

---

## 1. Single Living Memory Architecture

### 1.1 File Layout
```
/root/dowiz/MEMORY.md          ← THE single source of truth (agent-agnostic)
/root/dowiz/MEMORY.md.hier     ← hierarchical index (auto-generated from MEMORY.md)
/root/dowiz/MEMORY.idx         ← binary index (BM25 + trigram + PPR adjacency)
```

### 1.2 How All Agents Use It
- **Claude**: reads `MEMORY.md` instead of `.claude/projects/.../MEMORY.md`
- **Hermes**: reads `MEMORY.md` instead of `~/.hermes/memories/MEMORY.md`
- **OpenCode**: reads `MEMORY.md` (already does via AGENTS.md)
- All agents **write back** to `MEMORY.md` after meaningful changes
- No agent reads its own isolated memory store

### 1.3 Search Protocol (Agent-Agnostic)
```
Agent needs to find something in memory:
  1. Lexical: BM25 rank over MEMORY.md sections → top-k candidates
  2. Graph: PPR from seed section → related sections
  3. Fuzzy: trigram index → approximate matches
  4. Fusion: combine scores → final ranked list
  5. Return: section + context + confidence score
```

---

## 2. Kernel-Native Extraction & Parsing

### 2.1 New Modules Needed

| Module | Replaces | Pattern |
|--------|----------|---------|
| `kernel/src/parse/tsv.rs` | `awk -F'\t'` in 5+ scripts | `parse_rows(src, n_cols) -> Vec<Vec<&str>>` |
| `kernel/src/parse/env.rs` | `split('=')` in TS scripts | `parse_env(src) -> HashMap<String, String>` |
| `kernel/src/parse/json_query.rs` | `node -e JSON.parse` | stdin JSON → dot-path query → stdout |
| `kernel/src/parse/mod.rs` | — | re-exports |

### 2.2 Existing Primitives Already Usable
- `json::parse()` + `Value::get()` — replace all `node -e JSON.parse`
- `readability::extract()` — replace HTML parsing
- `spine::parse_frontmatter()` — replace YAML frontmatter parsing
- `bm25::tokenize()` — replace `tr`/`awk` tokenization
- `retrieval::pattern::Pattern` — replace `grep -E` (restricted subset)
- `metrics::LogEvent::from_line()` — replace pipe-delimited parsing

### 2.3 CLI Exposure
New `kernel/src/bin/json_query.rs` binary:
```bash
echo '{"name":"test","nested":{"key":"val"}}' | json_query "nested.key"
# → "val"
```
Replaces all `node -e` JSON extraction in scripts.

---

## 3. Kernel-Native Search (Replace grep)

### 3.1 Architecture
```
MemorySearchEngine
├── Lexical Layer
│   ├── BM25 (retrieval/bm25.rs) — Okapi scoring
│   ├── TrigramIndex (retrieval/index.rs) — inverted index + exact verify
│   └── Pattern (retrieval/pattern.rs) — wildcard matching
├── Graph Layer
│   ├── CSR + PPR (csr.rs) — personalized PageRank
│   ├── CGraph (cgraph.rs) — causal navigation
│   └── Harmonic (harmonic.rs) — centrality ranking
├── Semantic Layer
│   └── LeakGate cosine (leak_gate.rs) — embedding similarity
└── Fusion Layer
    └── FusedRanker (retrieval/recall.rs) — multi-signal combination
```

### 3.2 Usage Pattern
Instead of: `grep -rn "pattern" docs/`
Use: `memory_search("pattern")` → ranked results with context

Instead of: `awk -F'\t' '{print $3}' file.tsv`
Use: `parse_tsv(file, 3)` → typed column extraction

Instead of: `node -e "process.stdin.on('data',...)"` 
Use: `json_query(json_string, "path.to.field")` → value

### 3.3 Grep Replacement Map
| Current grep usage | Kernel replacement |
|--------------------|--------------------|
| `grep -rn "pattern" docs/` | BM25 + trigram index search |
| `grep -oP 'class="[^"]*"'` | Pattern matching (restricted wildcard) |
| `grep -F "literal" file` | TrigramIndex literal query |
| `grep -E "regex" file` | Pattern (limited subset) or BM25 |
| `grep -c "pattern" file` | BM25 score threshold |
| `grep -l "pattern" files` | TrigramIndex candidate filter |

---

## 4. Chronological-Topological Prediction

### 4.1 Existing Primitives for Prediction

| Primitive | Current State | Prediction Use |
|-----------|---------------|----------------|
| `markov::analyze()` | Returns Verdict + stationary dist + spectral gap | Predicts next-state distribution, convergence speed |
| `spectral::classify_drift()` | Damped/Resonant/Unstable | Predicts system trajectory class |
| `spectral::graph_energy()` | Sum of |eigenvalues| | Predicts structural change |
| `spectral_laplacian::laplacian_eigenmodes()` | k smallest eigenpairs | Predicts graph partition changes |
| `cgraph::descendants(X)` | All affected nodes | Predicts intervention impact |
| `causal::backdoor_adjust()` | P(Y\|do(X)) | Predicts causal outcome |
| `absorbing::expected_steps()` | Steps to terminal | Predicts funnel completion time |
| `absorbing::absorption_probs()` | P(terminal j \| start i) | Predicts outcome probabilities |
| `noether::invariant_drift()` | Accumulated drift | Predicts time-to-violation |
| `online::LinearGaussNatural` | Scale-robust predictor | Predicts next observation |
| `diffusion::related(seed)` | PPR ranking | Predicts information spread |

### 4.2 Prediction Engine Design
```
TemporalPredictor
├── State Classification (point-in-time)
│   ├── markov::analyze() → Verdict
│   └── spectral::classify_drift() → DriftClass
├── Convergence Forecasting (closed-form)
│   ├── markov::gap/mixing_time → time-to-stable
│   └── absorbing::expected_steps → time-to-terminal
├── Causal Intervention (do-calculus)
│   ├── cgraph::descendants(X) → affected set
│   └── causal::backdoor_adjust() → P(Y|do(X))
├── Structural Monitoring (temporal)
│   ├── spectral trajectory: rho(t), gamma(t), fiedler(t)
│   └── energy trajectory: E(t) → structural change prediction
├── Invariant Guard (trajectory-based)
│   ├── noether::invariant_drift() → drift rate
│   └── threshold prediction: time-to-violation
└── Online Adaptation (streaming)
    ├── LinearGaussNatural → next-observation prediction
    └── NaturalLogistic → regime change detection
```

### 4.3 Prediction Use Cases
1. **Action prediction**: "Given the last N actions, what is the next likely action?"
   - Markov chain + stationary distribution
2. **Change prediction**: "If file X is modified, what else needs updating?"
   - CGraph descendants + BM25 semantic similarity
3. **Failure prediction**: "Is this subsystem approaching failure?"
   - Spectral drift class + Noether invariant drift
4. **Performance prediction**: "How long will this operation take?"
   - Absorbing chain expected steps + online regression
5. **Intervention prediction**: "What happens if we change parameter P?"
   - Causal backdoor adjustment + counterfactual

---

## 5. Sub-Agency & Dynamic Swarms

### 5.1 Existing Swarm Substrate

| Primitive | Swarm Role | Status |
|-----------|-----------|--------|
| `AgentLoop` | Per-executor loop | Exists, needs FanOut step |
| `spool.rs` | Task queue | Exists, needs multi-consumer |
| `token_bucket::child_bucket()` | Budget slicing | Exists |
| `breaker::Breaker` | Fault containment per executor | Exists |
| `breaker::BreakerChain` | Swarm-wide kill switch | Exists |
| `ports::AgentBridge` | Security scoping | Exists, needs swarm manifest |
| `mesh.rs` | Distributed coordination | Exists |
| `fdr::ring` | Swarm telemetry | Exists |
| `dsu.rs` | Task dependency grouping | Exists |
| `harmonic.rs` | Candidate ranking | Exists |
| `router.rs` | Task→executor dispatch | Exists |

### 5.2 Swarm Coordinator Architecture
```
SwarmCoordinator
├── Task Decomposition
│   ├── DSU.components(task_deps) → independent groups
│   └── Router.dispatch(group, executors) → assignment
├── Execution
│   ├── Spool.append(TaskSpec[]) → durable queue
│   ├── TokenBucket.child_bucket() → budget slice per executor
│   ├── Admitter.admit(ExecutorManifest) → security scope
│   └── AgentLoop.run(executor) → parallel execution
├── Monitoring
│   ├── Breaker per executor → fault isolation
│   ├── FDR.event!() → telemetry stream
│   └── Harmonic.centrality(swarm_graph) → health assessment
├── Aggregation
│   ├── MeshLog.append(results) → distributed memory
│   └── spine.verify_chain() → integrity check
└── Dynamic Adaptation
    ├── markov::analyze() → swarm health verdict
    ├── spectral::classify_drift() → swarm trajectory
    └── autonomic::schedule() → preemptive adjustment
```

### 5.3 What Needs Implementation

| Gap | Effort | Impact |
|-----|--------|--------|
| `FanOut` step in AgentStep | Small | Enables parallel dispatch |
| Multi-consumer Spool claim | Small | Enables parallel task pickup |
| Swarm manifest variant | Small | Enables role-based security |
| Swarm aggregate breaker | Medium | Enables N-of-M health threshold |
| Temporal prediction engine | Medium | Enables proactive adaptation |
| Task-level mesh addressing | Small | Enables efficient completion detection |

---

## 6. Implementation Order (Dependency Graph)

```
Phase 1: Foundation (parallel-safe, no cross-dependencies)
├── 1a. MEMORY.md rewrite (unified, agent-agnostic)
├── 1b. Kernel parse module (tsv.rs, env.rs, json_query binary)
├── 1c. MemorySearchEngine (BM25 + trigram + PPR wrapper)
└── 1d. Update MEMORY.md with Hermes skill catalog

Phase 2: Prediction (depends on Phase 1 for memory search)
├── 2a. TemporalPredictor (compose existing primitives)
├── 2b. Prediction tests (red-green)
└── 2c. Wire prediction into autonomic layer

Phase 3: Swarms (depends on Phase 1 for memory, Phase 2 for prediction)
├── 3a. FanOut step + multi-consumer Spool
├── 3b. SwarmCoordinator module
├── 3c. Swarm manifest + aggregate breaker
└── 3d. Dynamic swarm tests

Phase 4: Integration
├── 4a. All agents read/write MEMORY.md
├── 4b. Scripts use kernel-native extraction
├── 4c. Prediction feeds into autonomic scheduling
└── 4d. Swarms use prediction for task decomposition
```

---

## 7. Falsifiable Acceptance Criteria

| Criterion | Test |
|-----------|------|
| Single memory file | All 3 agents read MEMORY.md, no agent reads own store |
| Kernel extraction | `json_query` binary replaces all `node -e` in scripts |
| BM25 search | `memory_search("spectral")` returns relevant sections |
| PPR navigation | `memory_search("related to item 58")` finds item 61 |
| Prediction | `TemporalPredictor::predict_next()` returns probability distribution |
| Swarm FanOut | AgentStep::FanOut dispatches N parallel tasks |
| Budget slicing | child_bucket correctly limits per-executor spend |
| Fault isolation | Breaker trips kill one executor, not the swarm |

---

## 8. DECART Analysis (New Integrations)

No new external dependencies introduced. All work uses existing kernel primitives.

| Criterion | Assessment |
|-----------|------------|
| Bare-metal fit | Pure Rust, zero deps, compiles to wasm32 |
| Falsifiable correctness | All primitives already tested (1157 tests) |
| Measured performance | BM25/PPR/CSR already benchmarked |
| Supply-chain/license | No new deps = no supply chain risk |
| Maintainability | All in kernel, same conventions |
| Reversibility | Can disable prediction/swarms without breaking core |

**DECISION:** No new integrations — all work extends existing kernel primitives.
