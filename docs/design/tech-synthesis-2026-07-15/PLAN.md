# Tech Synthesis 2026-07-15 — engineering-merit-only adoption plan

> Scope: deep research across rendering, scientific computing, LLM infra, dev tooling, a batch of
> unclear/niche names, one defensive security-testing tool, and an AI/architecture concept dump —
> for openbebop (Rust/WASM sovereign core) and dowiz (TS/React SPA + Rust kernel). Ranking is by
> speed/optimization/current-technical-fit ONLY — industry adoption/popularity is explicitly not a
> factor, per operator instruction. No opinion injected; every ranking traces to a cited metric or a
> concrete existing-code hook (kernel files read directly, not inferred). See RESEARCH-CONSPECT.md
> for full sourcing, BLUEPRINTS.md for the buildable items.
>
> **Excluded from this research entirely, on request:** njRat, AsyncRAT, SpyNote, 888 RAT, GhostShell
> Framework, hexsec-rat, KittySploit — remote-access/surveillance malware and breach-associated
> tooling with no coherent "integrate into a production delivery platform" story, defensive framing
> or not. Hydra (THC-Hydra) IS included, scoped narrowly to authorized credential-testing of our own
> staging endpoints — a standard, legitimate pentest tool, categorically different from the above
> (see RESEARCH-CONSPECT.md for the distinction).

## Tier 1 — concrete technical fit, current gap, low marginal cost (buildable now)

| # | Item | Why it ranks here | Blueprint |
|---|------|--------------------|-----------|
| TS-01 | **Harmonic centrality / information centrality** | Extends `kernel/src/spectral.rs`'s existing Laplacian eigenmode computation and the shipped reachability check directly — same BFS/graph-distance data, no new subsystem. Strongest technical hook of every math primitive researched. | BLUEPRINTS.md §TS-01 |
| TS-02 | **Vectorless RAG for `docs/design/`\*\*/`docs/adr/`\*\*/`docs/governance/`\*\*** | This repo's own doc corpus is already hierarchically structured (PLAN→BLUEPRINTS→RESEARCH-CONSPECT, consistently) — the exact case where structure-navigation beats embedding-similarity: zero embedding-generation cost, zero vector-DB round-trip, for a corpus with a reliable nav structure. | BLUEPRINTS.md §TS-02 |
| TS-03 | **Authorized Hydra credential-testing against staging** | Proves (rather than assumes) that the already-planned OPS-12 rate-limiting + Cloudflare Turnstile/Bot-Fight actually holds under the documented ~110x parallelism speedup Hydra itself benchmarks. Ties directly into existing, already-decided infra. | BLUEPRINTS.md §TS-03 |

## Tier 2 — real technical hook, conditional on a future/different state (not a current gap)

| # | Item | Condition that would activate it |
|---|------|-----------------------------------|
| TS-04 | Gamma function (Beta-distribution credible intervals on `causal.rs::empirical_identify`) | Only if the causal-inference layer needs uncertainty quantification beyond its current point estimate. |
| TS-05 | Gaussian Splatting via `web-splat` or `brush` (Rust/wgpu, Apache-2.0, real fps benchmarks: web-splat &gt;200fps RTX3090/~130fps low-end AMD) | Only if 3D/photogrammetric content enters openbebop's scope — no existing content-authoring pipeline for UI/text via splatting exists yet, and WebGPU-in-browser is currently Chrome-134+-only (Firefox/Safari not reliable), a real cross-browser gap today. |
| TS-06 | Mesh-LLM (`Mesh-LLM/mesh-llm`, Rust-native, OpenAI-compatible `/v1` HTTP API already shipped) | Only if LLM-agent infra is actually being built out — lowest integration cost of the 4 LLM tools researched (HTTP sidecar, Rust-native), but it's pre-1.0/self-described "experimental," and there's no capability-model of its own — the port still has to enforce the full attenuation boundary. |
| TS-07 | Netclode-style microVM agent sandboxing (`angristan/netclode`) | Architecturally aligned with the already-scoped zero-OCI/no-Docker direction (docker-swap-arc). Single-maintainer project, license unconfirmed — needs a license check + hardening review before any trust-boundary-adjacent use, consistent with the DECART-report standing rule. |
| TS-08 | Skylos (TS/JS dead-code + secret scanner, Apache-2.0, own benchmark: 29/29 vs. Vulture's 24/29 recall on seeded bugs) | Needs a bake-off against dowiz's existing ESLint/`ts-prune`-class tooling before adding as a new dependency — may be redundant with what's already gating `pnpm lint`. |

## Tier 3 — no current technical hook, or a documented conflict; not recommended on present evidence

Every item here has a one-line, cited reason it doesn't clear the bar — none are dismissed on
popularity/trend grounds, all on a specific technical fact found during research:

- **Born rule** — explicitly flagged by research as a stretch: the kernel's causal-inference code is
  entirely classical Pearl-style (no complex amplitudes, no non-commuting operators anywhere in
  `causal.rs`). No genuine hook.
- **Wronskian** — real math (Abel's identity ties it to the trace/spectrum), but redundant: the
  kernel already checks solution/eigenvector independence via direct rank/determinant computation.
  No unmet need.
- **CuPy** — not integrable into a Rust/WASM kernel at all (Python-only, no FFI, no CPU fallback).
  Its underlying CUDA libs are reachable from Rust via `candle`+`cudarc`, but the kernel's actual
  eigensolve workload (`householder.rs`, stack-only, N≤32, ≤8KB) is 2-3 orders of magnitude below
  the ~1MB threshold where CuPy's own benchmarks show GPU transfer overhead makes acceleration a net
  loss. Premature at current problem sizes.
- **AirLLM** — Python-only, no native serving mode (would need a hand-built sidecar), and the
  memory-for-latency tradeoff is severe: third-party benchmarks cite 10-40 minutes for a ~200-token
  response unquantized on a 70B model — a bad fit for anything latency-sensitive.
- **Omni-route** — large developer-tool surface (Electron GUI, 24+ coding-agent integrations,
  42-language i18n) that's mostly irrelevant overhead versus what a minimal custom router would cost
  to build directly; no published latency/throughput data despite the surface area.
- **OpenInterpreter** — the name now points to a different, Rust-based *agent* (not library), a
  fork of Codex — architecturally a whole autonomous agent with its own approval loop, not something
  to embed; would need sandboxed-subprocess isolation either way. The classic Python tool people
  usually picture is now an unofficial, single-maintainer fork, re-licensed AGPL-3.0 (copyleft) —
  not viable for embedding as-is.
- **OpenAlice** (TraderAlice/OpenAlice) — a trading-agent workspace; no domain overlap with either
  codebase beyond one abstract, non-code pattern (git-backed agent task workspace).
- **VeRa** (best-match candidate: VeRA parameter-efficient fine-tuning, via `huggingface/peft`) —
  only relevant if either codebase fine-tunes an LLM locally, which neither currently does.
- **Webscope** (Aditya060806/WebScope) — dev/agent tooling (browser-state-to-LLM-text bridge), not a
  runtime dependency for either product; at most an alternative to evaluate for internal agent
  tooling, not a codebase integration.
- **Octorender** — does not exist as a findable project (confirmed via GitHub, crates.io, npm
  registry searches — zero matches). Dropped.
- **Afaan/mc** — could not be identified as any real project after genuine search effort. Dropped;
  provide the original source context if this needs re-attempting.
- **IaC/wip** — not a named tool, resolves only to the generic phrase. Moot: dowiz's ops-reliability
  blueprint already specifies OpenTofu (OPS-18).
- **Terraform** — dowiz's own ops-reliability blueprint (OPS-18) already chose OpenTofu, the
  functionally-identical open-source fork, for governance reasons after Terraform's BUSL license
  change. Zero speed/optimization delta between them — re-adopting Terraform would be a pure
  license/governance regression with no engineering upside. Not proposed.
- **AWS service list** (EC2/Lambda/RDS/DynamoDB/etc.) — directly conflicts with the ops-reliability
  plan's already-recorded decision to consolidate OFF managed clouds onto a single Hetzner box
  ("Дроп Fly+Supabase," Cloudflare scoped to edge-only). Adding AWS services would reintroduce
  exactly the managed-cloud dependency/cost surface that decision rejected.

## Already implemented — not proposals, noted so this plan doesn't re-suggest existing work

Event Sourcing (openbebop's core), read/write separation at the port boundary, the
reflection/verification loop (doubt-escalation + Verified-by-Math + reflection pipeline — the
working implementation of the XAI/prompt-engineering "self-verification" pattern), and the
bidirectional CRDT/event-bus mesh direction (already the scoped bebop2-mesh / Living-Memory-as-pgrust
design — the pasted mesh architecture sketch validates rather than proposes an alternative to it).

## Database patterns with concrete payoff for the in-flight pgrust migration

Of the 18 database patterns in the concept material, four have a direct, non-generic payoff for the
already-in-progress pgrust migration specifically: **Outbox Pattern** (atomicity between a DB write
and an event publish — directly on the event-sourced core's write path), **Write-Ahead Logging**
(pgrust's WAL compatibility is explicitly UNVERIFIED per the existing ops-reliability blueprint — a
real open risk to verify, not a pattern to newly adopt), **Change Data Capture** (a candidate
mechanism for the mesh's reactive Mesh→Agent propagation), **Connection Pooling** (PgBouncer is
already in the ops-reliability latency stack — this confirms rather than proposes). Detail in
CONCEPTS-APPLIED.md.
