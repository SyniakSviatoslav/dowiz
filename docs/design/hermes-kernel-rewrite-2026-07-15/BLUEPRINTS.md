# Hermes kernel rewrite — Blueprints

> Format matches this session's established convention. Each item traces to a specific AUDIT.md
> finding — nothing here is speculative scope, every blueprint fixes a cited, evidenced problem.

## Wave 0 — kernel scaffold

### HK-00 · `hermes-kernel` Rust crate scaffold
- **Мета:** a compilation target for pure decision logic, structurally incapable of I/O.
- **Межа:** ЧІПАЄМО — new crate only. НЕ ЧІПАЄМО — the existing Python codebase, until each
  individual blueprint below is ready to call into it via FFI/subprocess-boundary/local HTTP (the
  exact bridge mechanism is an implementation choice per-blueprint, not fixed here).
- **Форма:** `Cargo.toml` with `default-features = false`, no `tokio`/`reqwest`/`sqlx`/anything
  network- or filesystem-capable in the kernel crate's own dependency tree — mirrors dowiz's
  `kernel/Cargo.toml` posture (`default = ["std"]`, zero non-essential deps).
- **RED:** a `cargo tree` on the kernel crate must show zero I/O-capable dependencies — this is the
  crate-level equivalent of openbebop's R0 compilation firewall, enforced by dependency graph rather
  than a custom lint. **Хвиля:** W0, first, blocks nothing else structurally but should land before
  any of HK-01..06 to give them a home.

## Wave 1 — pure bookkeeping fixes (lowest risk, cleanest evidence)

### HK-01 · Search pagination/truncation kernel function
- **Мета:** fix AUDIT.md's cleanest bug — `truncated`/`total_count` currently cannot mathematically
  report correctly in the primary search path (`file_operations.py` content/files-only/count modes
  and the grep fallback all independently compute this, one is provably wrong).
- **Межа:** ЧІПАЄМО — one new kernel function; the four Python call sites become thin callers.
  НЕ ЧІПАЄМО — the actual `rg`/`grep`/`find` shellout logic, which the code review confirmed is
  already fast and correct (ripgrep-backed, `.gitignore`-respecting, proper timeout).
- **Форма:** `fn paginate(raw_line_count: usize, offset: usize, limit: usize, limit_reason: Option<LimitReason>) -> Page { total_count: usize, truncated: bool, shown: Range }`. Callers must fetch a
  sentinel row (`limit + offset + 1`, not `limit + offset`) so `raw_line_count` can actually exceed
  the shown window — the current bug is that the fetch itself clips before the count happens.
- **Reuse:** none needed — self-contained arithmetic.
  **RED:** property test — for any `raw_line_count > offset + limit`, `truncated` must be `true`.
  This is the literal fix for AUDIT.md's cited bug: today this property fails 100% of the time
  because it's structurally unreachable. **Хвиля:** W1, immediate, highest-confidence fix in this
  plan.

### HK-02 · Checkpoint dedup key kernel function
- **Мета:** fix the content-only dedup key that conflates duplicate prompts in batch-resume.
- **Межа:** ЧІПАЄМО — the dedup key function only. НЕ ЧІПАЄМО — `batch_runner.py`'s checkpoint
  file format (already atomic, already crash-safe per the code review — the bug is purely in what
  key is used to check "already done", not in the persistence mechanism).
- **Форма:** `fn dedup_key(prompt_text: &str, prompt_index: Option<usize>) -> Key` — when
  `prompt_index` is present (it already is in the batch-file schema), the key includes it; falls back
  to text-only for legacy files without an index, matching the backward-compat need the code review
  flagged.
- **Reuse:** the existing atomic-write/content-scan-resume mechanism stays untouched — this narrowly
  replaces one function.
  **RED:** a fixture batch with two identical prompts at different indices must resume with BOTH
  still pending after one completes — today's test suite doesn't actually catch this (per the code
  review's B5b finding that the checkpoint tests simulate rather than call the real code), so this
  blueprint's test must drive `BatchRunner.run(resume=True)` for real, not a mirror. **Хвиля:** W1.

## Wave 2 — verification as a hard invariant

### HK-03 · Verification FSM: "session complete" as a guarded transition
- **Мета:** turn the existing (good) verification-evidence data into an authoritative gate instead
  of `verification_stop.py`'s self-documented "policy-only... never blocks" nudge.
- **Межа:** ЧІПАЄМО — a new kernel state machine consuming `verification_evidence.db`'s existing
  schema read-only. НЕ ЧІПАЄМО — the ledger-writing side (`terminal_tool.py`/`file_tools.py`'s
  existing writes stay exactly as-is; they're already correct per the code review).
- **Форма:** mirrors `kernel/src/order_machine.rs::assert_transition` directly: a
  `session_status` FSM with states `{InProgress, AwaitingVerification, Verified, Complete}`; the
  `Complete` transition is only legal from `Verified`; `Verified` requires every code-file touched
  this session (per `verification_evidence.db`'s existing `root`/`session_id` columns) to have a
  `status == passed` row newer than its last edit. Non-code-only turns (matching
  `verification_stop.py`'s existing `_NON_CODE_VERIFY_EXTENSIONS` allowlist — reuse that list
  unchanged, it's already correct) skip straight to `Verified`.
- **Reuse:** the ledger schema, the non-code-extension allowlist, and the `project_facts_for`
  workspace-detection logic — all confirmed working, all reused as-is. Only the *consequence* of an
  unverified state changes: today it's a chat nudge, this makes it a hard transition refusal (the
  agent loop cannot mark the turn/session as done while in `AwaitingVerification`).
  **RED:** a session that edits a code file and attempts `Complete` without a passing verification
  event for that file must be refused — this is the literal, direct fix for AUDIT.md's #1 empirical
  finding (the ~30-correction false-"done" cluster). Test against the exact false-green scenario
  quoted in AUDIT.md (lines 401-410 in the history log: claimed-done, still-failing tests).
  **Хвиля:** W2, the single highest-leverage fix in this plan given it's AUDIT.md's most-evidenced
  problem.

## Wave 3 — dispatch timeout unification

### HK-04 · Unified tool-dispatch deadline function
- **Мета:** close the sequential-dispatch no-timeout gap without duplicating the concurrent path's
  (already-correct) budget logic a second time.
- **Межа:** ЧІПАЄМО — extract the concurrent path's deadline computation into a kernel function;
  both `execute_tool_calls_concurrent` and `execute_tool_calls_sequential` call it.
  НЕ ЧІПАЄМО — the actual subprocess-kill mechanics (`_wait_for_process`, process-group signaling)
  — confirmed already correct, untouched.
- **Форма:** `fn resolve_tool_timeout(config: &TimeoutConfig, tool_count: usize, elapsed: Duration) -> Deadline` — pure function of config + call context, no I/O. The sequential path wraps its
  synchronous dispatch in a worker-thread + `future.result(timeout=deadline)` using this same
  computed value, synthesizing a timeout tool-result on expiry (the machinery for this already
  exists in the concurrent path per the code review — B1a's fix is "reuse it," not "invent it").
- **Reuse:** the concurrent path's existing abandon-semantics/timeout-result-synthesis code.
  **RED:** a tool call with no internal timeout, dispatched via the sequential path, must be forcibly
  timed out at the computed deadline — today it hangs forever; this is directly falsifiable by
  constructing exactly that fixture. **Хвиля:** W3.

## Wave 4 — quality-aware model routing

### HK-05 · Task-complexity-aware routing with harmonic-centrality model ranking
- **Мета:** replace availability-first-only fallback selection with a decision that also accounts
  for task difficulty and historical model performance on similar tasks — without touching the
  (already well-engineered) error taxonomy/cooldown/retry machinery that handles *availability*.
- **Межа:** ЧІПАЄМО — a new kernel decision function inserted *before* the existing fallback
  selection, not replacing it — availability-based fallback still applies once a model is chosen
  as the target. НЕ ЧІПАЄМО — `agent/error_classifier.py`'s `FailoverReason` taxonomy,
  `chat_completion_helpers.py`'s cooldown logic, and the per-provider `reasoning_effort` translation
  (all confirmed correct/well-engineered per the source-architecture review).
- **Форма:** two parts.
  1. **Complexity classification** (cheap, heuristic, kernel-pure): signals already available
     pre-dispatch — predicted multi-step tool chains, presence of math/proof language, multi-file
     edit scope, message length/structure — feed a `TaskComplexity` score. No new model call needed
     for this; it's feature extraction over the already-composed prompt.
  2. **Model ranking via harmonic centrality**: build a graph where nodes are (task-complexity-bucket,
     model) pairs and edges are weighted by historical outcome (drawn from the same session logs
     already captured — `hermes_state.db`'s message history plus `verification_evidence.db`'s pass/
     fail data double as the training signal, no new instrumentation needed). Harmonic centrality
     over this graph ranks each model's actual track record *for this complexity bucket specifically*
     — not a global leaderboard, a per-task-shape one. This is the concrete "vectorless spectral
     indexed graph" application: no embeddings anywhere in this pipeline, just a graph over
     (task-type, model, outcome) triples and a centrality measure already proven (TS-01 in the
     tech-synthesis plan) on dowiz's own kernel.
  Route classified-complex tasks to whichever model ranks highest by this measure among currently
  *available* (per existing health/cooldown state) options — falling back to today's pure-availability
  order only when no outcome history exists yet (cold start).
- **Reuse:** `verification_evidence.db` and `hermes_state.db` as the outcome-signal source (already
  populated, no new logging needed); the existing fallback/cooldown machinery as the availability
  layer this sits on top of; harmonic centrality's implementation directly from TS-01.
  **RED:** given a synthetic history where model B outperforms model A specifically on
  high-complexity tasks (but A wins on simple ones), the router must prefer B for a new
  high-complexity task and A for a new simple one — a direct test of whether ranking is actually
  per-bucket and not a flattened global average. **Хвиля:** W4, gated on W1-W3 landing first (highest
  complexity item in this plan, benefits from the kernel scaffold being proven on simpler cases
  first).

## Wave 5 — additive spectral memory ranking

### HK-06 · Spectral/harmonic memory-ranking plugin
- **Мета:** rank retrieved facts from the existing `holographic` SQLite store by graph-connectedness
  to the current task, not just by the flat `trust_score` column.
- **Межа:** ЧІПАЄМО — a new memory-provider plugin (`plugins/memory/spectral/`, following the exact
  structure `holographic`/`retaindb` already establish). НЕ ЧІПАЄМО — the `holographic` store's
  schema, its HRR-vector binding, or `retaindb`'s cloud integration — both stay exactly as they are;
  this is a third, optional provider, not a replacement.
- **Форма:** build a graph from the existing `facts`/`entities`/`fact_entities` tables already in
  `holographic/store.py`'s schema (no migration needed — this plugin reads that schema, doesn't own
  new tables) — facts and entities as nodes, `fact_entities` join rows as edges. Harmonic centrality
  over this graph, combined with (not replacing) the existing `trust_score`, ranks retrieval results.
  Vectorless-RAG-style structure-navigation (TS-02) applies to the MEMORY.md/USER.md frozen-snapshot
  files specifically — their heading/entry structure is already navigable without embeddings.
- **Reuse:** 100% of the existing schema and entity-resolution logic; this plugin only adds a ranking
  pass, following the plugin architecture Hermes already documents and supports.
  **RED:** a query where the highest-`trust_score` fact is graph-isolated (no entity connections to
  the current context) and a lower-`trust_score` fact is highly connected must surface the connected
  one higher — proving the ranking genuinely uses graph structure, not just re-deriving the trust
  score. **Хвиля:** W5, fully independent of W1-W4, can land whenever convenient.
