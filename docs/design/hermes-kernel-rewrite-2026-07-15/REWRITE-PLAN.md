# Hermes → kernel/Rust rewrite plan — 2026-07-15

> Grounded in AUDIT.md's findings. Principles and component mapping reuse patterns already
> proven — not proposed, *proven*, with working code — in this same operator's own openbebop and
> dowiz codebases. Detailed per-item blueprints in BLUEPRINTS.md.

## Principle: kernel = pure and deterministic, Rust = adapters/bridges

This is not a new idea being introduced to Hermes — it's the exact architecture already load-bearing
in dowiz's own kernel and openbebop's own core, being applied to a third tool built by the same
operator. Concretely:

- **Kernel** = zero I/O, zero network, zero subprocess calls, zero non-determinism. Pure functions
  and state machines over explicit inputs, same input always produces same output, provable with
  unit/property tests. This is what dowiz's `kernel/src/order_machine.rs::assert_transition` already
  is (a hard 409 on an illegal transition, not a suggestion) and what `kernel/src/spectral.rs`
  already is (Laplacian eigenmodes computed from a graph, no side effects).
- **Adapters/bridges** = everything that touches the outside world — subprocess execution, file I/O,
  network calls to model providers, terminal interaction. These call *into* the kernel for decisions
  but never contain the decision logic themselves. This is openbebop's own port pattern: an adapter
  never imports the kernel's internals directly except through a defined interface (the R0
  compilation-firewall pattern — import-kernel-directly literally fails to compile in that
  codebase), and a capability-boundary check (R3) sits between every adapter and anything
  sensitive.
- **Why this maps cleanly onto AUDIT.md's two failure modes**: "soft nudge instead of hard invariant"
  is precisely what happens when decision logic lives inside an I/O-adjacent Python function instead
  of being a kernel state-machine transition that can categorically refuse an illegal move. "Pure
  logic duplicated slightly-wrong four times" is precisely what happens when there's no single
  kernel function that owns that arithmetic — each call site re-derives it.

## The proven-technique toolkit (from openbebop/dowiz, with real benchmarks behind each one)

These aren't proposals to research — they're already built, already grounded against real kernel
source in this session's tech-synthesis work, and now being pointed at a second application:

- **Harmonic / information centrality** — a continuous relevance ranking over a graph's existing
  distance data, reduces cleanly to a binary bit at the extremes, handles disconnected graphs
  without blowing up (`∞⁻¹ = 0`, unlike arithmetic-mean centrality). Already scoped for dowiz's own
  `spectral.rs` (see tech-synthesis-2026-07-15/BLUEPRINTS.md TS-01). Applied to Hermes: rank
  retrieved memory facts / skills / search results by graph-connectedness to the current task
  context, not by a scalar `trust_score` or raw keyword match.
- **Vectorless RAG / structure-navigation** — route a query by document/data *structure* before
  falling back to embedding-similarity or keyword search; zero embedding cost, zero vector-DB
  round-trip for anything with a reliable structure. Already scoped for dowiz's own doc corpus (TS-02
  in the same blueprint). Applied to Hermes: MEMORY.md/USER.md, the skills library, and the session
  archive all have real structure (entries, categories, timestamps, entity links in the holographic
  store's schema) that a structure-first index can exploit before reaching for anything heavier.
- **Event-sourced state** — the append-only, replay-derivable state model already load-bearing in
  openbebop's own core. Applied to Hermes: the verification-evidence ledger and the conversation
  archive are already *shaped* like event logs (append-only, timestamped); the fix is making the
  kernel's "is this session actually done" decision a genuine fold over that event log, not a
  best-effort read.
- **Capability/port boundary (R0/R3)** — a compilation-enforced firewall between decision logic and
  anything that touches the outside world. Applied to Hermes: the kernel crate should be structurally
  incapable of importing `subprocess`, `httpx`, or a DB driver — if a "decision" function needs one of
  those, that's a sign it isn't actually pure and belongs in the adapter layer instead.

## Component mapping — grounded in AUDIT.md's actual findings, not generic

| Hermes component (current) | Verdict | Target |
|---|---|---|
| Verification "done" decision (`verification_stop.py`) | Soft nudge over real ledger data | **Kernel FSM transition guard** — `session_complete` transition requires `evidence.status == Passed` for every touched code path; illegal transition is a hard refusal, matching `order_machine.rs::assert_transition` exactly |
| Search pagination/truncation math (4 duplicated Python implementations, one provably broken) | Pure logic, wrong | **Kernel function**, single implementation: `fn paginate(total: usize, offset: usize, limit: usize) -> Page` with a property test asserting `truncated == (total > offset + limit)` — the exact invariant that's currently unreachable |
| Model routing / fallback selection | Availability-first, quality-blind | **Kernel decision function** taking (task-complexity signal, provider health, historical success-by-task-type) → chosen model. Historical success ranked via **harmonic centrality** over a task-type↔model-outcome graph built from the same session data already logged — this is the concrete "vectorless spectral indexed graph" application: no embeddings, a graph of (task-type, model, outcome) edges, centrality tells you which model is actually central to success for this task shape |
| Sequential tool-dispatch timeout | Absent — can hang forever | **Kernel-computed deadline** derived once, deterministically, from the same budget logic the concurrent path already has correct — one function, both dispatch paths call it |
| Checkpoint/resume dedup key | Text-only, conflates duplicates | **Kernel function**: `fn dedup_key(prompt: &str, index: Option<usize>) -> Key` — pure, testable, the `(text, index)` fix from AUDIT.md expressed as a single owned function instead of inline logic |
| Memory retrieval ranking (`trust_score` scalar in the holographic SQLite store) | Works, but ranks on one flat number | **Additive kernel-backed plugin** alongside the existing `holographic`/`retaindb` providers — spectral/harmonic ranking over the *same* SQLite fact store (reuses `plugins/memory/holographic/store.py`'s schema, doesn't replace it), following the plugin pattern that already exists |
| Subprocess execution, process-group kill, UTF-8-safe draining, Windows compat | Already correct, already hardened | **Unchanged.** Kept exactly as-is as the adapter layer. This is the concrete case for "adapters, not a full rewrite" — AUDIT.md found this subsystem genuinely mature; touching it would be regression risk for zero benefit |
| Tool-output spill-to-disk tiering | Already correct, standout design | **Unchanged**, same reasoning |

## What this plan deliberately does NOT propose

- **Not a full Python→Rust port.** AUDIT.md is explicit that the I/O/subprocess/compat layer is
  already well-engineered. Porting working, hardened code for its own sake is exactly the kind of
  unforced rewrite risk a careful engineer avoids — matches this operator's own DECART standing rule
  (new-dep/swap decisions need a falsifiable comparison, not a rewrite-for-its-own-sake).
- **Not a replacement of the existing memory backends.** `holographic` (HRR+FTS5) and `retaindb`
  (paid cloud) stay; a spectral/harmonic backend is proposed as a third, additive plugin option,
  following the plugin architecture Hermes already has.
- **Not a claim that free-tier models are the root problem.** The routing *mechanism* being
  quality-blind is the bottleneck; a smarter router still has to work within whatever models are
  actually configured. Fix the decision function, not the model budget.

## Migration waves (additive, matching the "big-bang forbidden" convention used throughout this session's other blueprints)

- **W0 — kernel scaffold**: a new Rust crate (`hermes-kernel`) with zero external dependencies beyond
  `std`, mirroring dowiz's own kernel's `Cargo.toml` posture. No integration yet — this wave is purely
  "does the pure logic compile and pass its own property tests."
- **W1 — pagination/dedup kernel functions** (lowest risk, highest confidence fix): HK-01, HK-02 in
  BLUEPRINTS.md. Pure arithmetic, directly fixes AUDIT.md's cleanest, most provable bug (A1's
  unreachable truncation flag).
- **W2 — verification FSM**: HK-03. Requires the existing `verification_evidence.db` schema as the
  kernel's input — no schema change needed, just a new consumer that's authoritative instead of
  advisory.
- **W3 — dispatch timeout unification**: HK-04. Single deadline function shared by both dispatch
  paths, closing the sequential-path hang risk.
- **W4 — model-routing kernel + harmonic-centrality ranking**: HK-05. Highest complexity of the five,
  gated on W0-W3 landing first since it's the one genuinely new subsystem (the others are extracting
  existing-but-scattered logic into one place).
- **W5 — spectral memory-ranking plugin**: HK-06. Fully additive, zero risk to the existing memory
  backends, can land independently of the W1-W4 sequence.

Each wave's RED-proof criteria are in BLUEPRINTS.md, following the same red→green discipline used for
every other blueprint produced this session.
