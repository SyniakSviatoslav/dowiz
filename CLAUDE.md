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
- **`docs/design/`** — active roadmap/blueprints, anchored by `CORE-ROADMAP-INDEX.md` and the
  `MASTER-ROADMAP-*` docs. Design corpus may describe speculative unifications flagged as research
  directions; trust the tree over the vision docs when they disagree.
