# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

**dowiz** is a decentralized mesh-hub delivery platform whose authority lives in a
deterministic **Rust/WASM kernel**. **DeliveryOS** is the reference app on top of it,
not the other way around. The codebase is **Rust-first** (~270 `.rs` files); the former
TypeScript/JS frontend and its pnpm/turbo stack were **removed on 2026-07-15** ("drop js").
Only generated `wasm-bindgen` `.d.ts` glue remains — there is no `apps/api`, `apps/web`, or
`packages/` anymore. (Note: the Repowise index embedded in `.claude/CLAUDE.md` was last indexed
2026-06-14, *before* the drop, so its TS file citations are stale — trust the live tree.)

Read `README.md`, `MANIFESTO.md`, and `DECISIONS.md` for the "why". `DECISIONS.md` D0 lists the
six non-negotiable invariants that outrank all roadmap/feature pressure:
**decentralized · local-first · post-quantum · crypto · mesh · reliability-over-latency.**

## Build model — CRITICAL

**There is NO root `Cargo.toml` / cargo workspace.** Each crate is standalone. You **must `cd`
into the crate directory** and run cargo there. Do **not** use `cargo -p <crate>` or
`cargo --manifest-path …/Cargo.toml` from the repo root — with no workspace, that resolves the
manifest but pulls the wrong target graph and can mask failures as exit-0 (a documented
**false-green trap**, see `.github/workflows/ci.yml` cargo-test job header).

## Common commands

```sh
# Kernel tests (the source-of-truth surface)
cd kernel && cargo test              # full; use --lib for lib-only, --offline in CI
cd kernel && cargo test <filter>     # single test / module by name substring

# Engine tests (physics render engine; path-depends on kernel)
cd engine && cargo test

# One-shot local gate before pushing (kernel+engine tests, wasm32 build, fmt --check)
bash scripts/verify-kernel-engine.sh

# Format / lint (run inside the crate dir — no workspace)
cd kernel && cargo fmt --check
cd kernel && cargo clippy

# Build the kernel WASM surface consumed by web/ (emits gitignored kernel/pkg + kernel/pkg-web)
bash scripts/build-kernel-wasm.sh

# Zero-dependency kernel-driven web demo (renders only; all math in the wasm)
cd web && npm run serve              # → http://localhost:8099/web/index.html
cd web && npm test                   # node kernel test harness

# Dependency policy gate (bans yanked crates, wildcards, disallowed licenses)
cd kernel && cargo-deny check        # config: deny.toml
```

Feature-gated builds matter (see "Feature discipline" below): the **default kernel build is
pure-`std` and serde-free**. Opt-in features: `wasm`, `json-api`, `pq`, `gpu`, `pgrust`,
`slot-arena`, `telemetry`, `chaos`, `count-allocs`. Verify a feature stays out of the default
graph with e.g. `cd kernel && cargo tree -e no-dev | grep -c serde` (expect `0`).

## Crate map (the big picture)

Standalone crates, wired by **path dependencies** — not a workspace:

- **`kernel/`** (`dowiz-kernel`) — the sole math authority. Order lifecycle, money, decisions,
  crypto, spectral/graph math, retrieval. Compiles to WASM. This is what almost everything else
  depends on.
- **`engine/`** (`dowiz-engine`) — physics-based field-UI render engine (no DOM). **Zero external
  crates by default** ("offline-clean"); path-depends on `kernel` to drive the graph-Laplacian
  field. GPU/WebGL/WebGPU are declared-but-empty feature seams.
- **Agent lane (P40), with a deliberate compile firewall:** `agent-facade` is the *only* agent
  crate that imports `dowiz-kernel`, and it does **not** re-export mutation symbols
  (`decide`/`fold`/stores). `agent-loop` (the bounded plan→act→observe executor) imports **only**
  `agent-facade`, so it structurally *cannot name* kernel mutation. `agent-adapters` (MCP bridge,
  JSON-RPC) and `llm-adapters` (Ollama/vLLM/managed-API, `ureq`, no tokio) sit at the edges.
- **`mesh-adapter/`** — wires the kernel as the consumer/driver of the `bebop2/` delivery protocol
  (the PQ, capability-authenticated mesh; its crypto core lives in the companion **OpenBebop** repo
  and is injected at a seam).
- **`wasm/`, `agent-governance-wasm/`** — `wasm-bindgen` bridges exposing engine/kernel surfaces to JS.
- **`apps/courier/`** — a Rust courier app crate.
- **`web/`** — a zero-dependency Node shell that *only renders*; it consumes the kernel wasm glue and
  never re-implements math.
- **`tools/`** — supporting Rust/py crates: `eqc-rs`/`eqc` (equation→Rust compiler),
  `ci-truth` (CI re-execution/ledger binary), `native-spa-server` (native HTTP adapter over the
  kernel's `json-api`), `telemetry` (bash telemetry bridge — the always-green CI job).

## Kernel authority model (why edits here are load-bearing)

- **Order state is a decide/fold FSM.** `kernel/src/order_machine.rs`: `decide → Event`, then
  `state = fold(events)`. Forbidden transitions are **errors, not silent no-ops**. The FSM is
  self-checked by five graph lenses (cycle/cyclomatic/topo/reachability/spectral-radius) pinned to
  a golden signature — introduce a cycle and the self-check goes red.
- **Money is exact integer arithmetic — zero floats, ever.** `kernel/src/money.rs`. Amounts are
  currency-typed, overflow-checked `i64`/`i128`; refunds net to exactly zero through a double-entry
  ledger. The core decision path has no clock/RNG/network/float (MANIFESTO C2) so every node
  replays identically offline.
- **Trust is a signed capability, never a score.** No rating/ranking/reputation of any participant.
  Enforced two ways: a CI job (`no-courier-scoring`) fails the build if a
  `courier_score/rating/reputation` identifier appears in kernel/engine, and routing enums omit
  `Ord`/`PartialOrd` so a "quality router" is unrepresentable in the type system
  (`kernel/src/decision/mod.rs`, `kernel/src/domain.rs`).
- **Red-line capabilities deny by default** (`kernel/src/ports/agent/scope.rs`,
  `RedLinePolicy::DenyByDefault`) — ledger/money, auth, migrations are denied unless explicitly granted.
- **Generated code is parity-pinned.** `kernel/src/eqc_gen.rs` is emitted by `tools/eqc-rs`
  ("GENERATED — do not hand-edit") and a test asserts *exact integer equality* against the
  hand-written money law, so the law and its compiled organ cannot silently diverge.
- **PQ crypto is real and KAT-gated** (`kernel/src/pq/`): byte-exact ML-DSA-65 vs NIST ACVP,
  X25519+ML-KEM-768 hybrid with no classical-only fallback. Never fake/stub a crypto primitive.

## Feature discipline (repo-specific rule)

New heavy or external-dep functionality goes **behind an off-by-default Cargo feature** with a
header comment stating what it pulls in and how to verify the default build stays clean. This keeps
the canonical order/money core pure-`std`, serde-free, and WASM-lean. Read the extensive feature
docs at the top of `kernel/Cargo.toml` before adding a dependency; new deps/swaps are expected to
carry a rationale (the "DECART" convention).

## Verification & CI gates

- **Pre-commit** (`.husky/pre-commit`, scope-aware): gitleaks on staged files → `cd kernel &&
  cargo test` if `kernel/` touched → `cargo-deny check` if any `Cargo.*` staged.
- **CI** (`.github/workflows/ci.yml`): telemetry self-test; `eqc` math proofs; **unconditional**
  kernel+engine `cargo test --offline` (per-crate `cd`); bench-regression gate; and `v5c-reexec`,
  which independently re-executes the diff range in a clean worktree when a red-line path
  (`money.rs`/`order_machine.rs`/`event_log.rs`/auth) is touched.
- **Bench regression:** `kernel/benches/bench_track.py` runs criterion twice on the same runner
  (merge-base vs HEAD) and gates on criterion's own statistical A/B verdict. Bench ids are
  `<group>/<n>`; `kernel/benches/baseline.json` mirrors committed means. Only deterministic kernel
  benches are baseline-gated (host-noisy harness/LLM benches stay pass/fail probes).

Engineering culture is **"verified, not claimed"**: land fixes with a RED→GREEN test proving the
bug existed and is closed; back performance claims with a measured benchmark number.

## Related guidance in this repo

- **`.claude/CLAUDE.md`** — agent operating *discipline* (tool-use, planning, safety rules,
  governance status). Complementary to this file, which covers *build + architecture*.
- **`AGENTS.md`** — "innovating senior dev" mode + `/innovate-*` review commands.
- **`DECISIONS.md`** / **`MANIFESTO.md`** — the authoritative red-line decisions and product thesis.
- **`docs/design/`** — active roadmap/blueprints. **Start at `docs/design/ROADMAP.md`** — the
  chronological master roadmap and single canonical "what's the status" entry point (2026-07-20).
  It sits above `CORE-ROADMAP-INDEX.md` (the detailed P-number/blueprint cross-reference table)
  and the `MASTER-ROADMAP-*`/`GROUND-TRUTH-*` docs, several of which are now historical — see
  `ROADMAP.md` §8 before trusting any doc whose name starts with `ROADMAP`/`MASTER`/`GROUND-TRUTH`.
  Design corpus may describe speculative unifications flagged as research
  directions; trust the tree over the vision docs when they disagree.

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- For codebase questions use `graphify explain "<name>"` and `graphify path "<A>" "<B>" [--undirected]`
  against `bebop-lang/graphify-out/graph.json`. **There is NO `graphify query` command in this version** --
  the CLAUDE.md line that told every agent to run it was wrong from 2026-09-04 to 2026-09-09.
  The index covers `.bp` only because `bebop-lang/tools/bp_graph.py` supplies the extractor graphify lacks;
  after editing `.bp`, refresh with `tools/bp_graph.py bebop-lang -o /tmp/bp.json && graphify merge-graphs
  bebop-lang/graphify-out/graph.json /tmp/bp.json --out bebop-lang/graphify-out/graph.json`.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update bebop-lang --no-cluster` for the harness, then the
  `bp_graph.py` + `merge-graphs` pair above for `.bp` -- `graphify update` ALONE silently drops every
  `.bp` node, which is how the index sat at 0 of 63,257 lines of Bebop for five days.

## Token-economy toolstack (RE-VERIFIED 2026-09-09 — the table said "always on, verified 2026-09-04"
## and five of its six entries were not installed on this box; the absent rows are struck rather than
## deleted, because a tool that was expected and is missing is a fact worth carrying)

| Tool | How it is active | What you must do |
|---|---|---|
| **rtk** 0.42 — ABSENT 2026-09-09 (not on PATH); the `PreToolUse` rewrite this row claims is not in effect | global `PreToolUse` hook rewrites Bash to `rtk …` (63.8% measured savings) | prefer `rtk read/grep/git/ls/find/diff` explicitly for large outputs; never `cat` a big file |
| **graphify** 0.9 — PRESENT, but it had ZERO `.bp` nodes until 2026-09-09: no extractor exists for the project's own language, and `graphify query` does not exist in this version (use `explain`/`path`). Fixed by `bebop-lang/tools/bp_graph.py` + `graphify merge-graphs` — 2,615 -> 5,740 nodes, 3,366 -> 24,105 edges | project hooks (`hook-guard`) + `## graphify` rule above + `/graphify` skill | `graphify query/path/explain` before grep; `graphify update .` after code edits (AST-only) |
| **mempalace** 3.x — plugin INSTALLED (`memory-palace@mempalace`) but its MCP server failed to connect 2026-09-09 and no `mempalace` CLI is on PATH | global plugin; PreCompact/SessionEnd hooks mine the session | `mempalace search <words>` before re-reading history; re-mine journals after commits |
| **ponytail** 4.9 — ABSENT 2026-09-09 (not on PATH) | global plugin (lazy-senior mode: simplest working solution) | do not add unrequested abstractions |
| **tb** — WRITTEN 2026-09-09 as `bebop-lang/tools/tb.py` because it was absent; `h` content-address, `s`/`n` hits, `d` changed-or-not (prints NOTHING when unchanged) | `tb h <path>` crc32 content-address, `tb s <needle> <path>` hit lines | re-read a file only when its hash changed |

**FREE-MODEL ROUTING for mechanical worker text (2026-09-09): `bebop-lang/tools/freellm.py`.** The
per-worker model selector accepts four Anthropic models and nothing else, so a subagent CANNOT be pointed
at a free provider; `ANTHROPIC_BASE_URL` routes the WHOLE session including merge decisions and is the
operator's to set. What a worker CAN do is shell out. `freellm.py` routes one prompt to the first
configured free provider with fallback across six (groq, cerebras, google, openrouter, mistral, together),
all six TLS-reachable from this box as measured 2026-09-09. It REFUSES rather than degrades: no key exits 3
naming the providers, every provider failing exits 4 with each error, and a truncated or empty completion
exits rather than being passed off as an answer. **The only missing input is an API key** -- the
environment has none. A LOCAL model is refuted by measurement, not preference: 1,619 MB available RAM
against a 3B-Q4's ~2 GB, with procs at 33 against a 32-process ceiling. Send mechanical text work there
(summarise a log, extract a table, reformat); judgement and merges stay where they are.

Rules: compile/test output to `/dev/null` and read `tail -1`; one deterministic run is proof;
**re-read a file only when its hash moved: `tools/tb.py d <path> <crc>` prints nothing and exits 0 when it
has not (measured 2026-09-09 against reading bebop.bp whole: `tb s` 3,028x fewer bytes, `tb n` 22,709x,
`tb h` 12,977x, `tb d` on the unchanged path infinite);**
scratch lives in the session scratchpad, never `/tmp` root. Details: `bebop-lang/docs/TOKEN-ECONOMY.md`.
