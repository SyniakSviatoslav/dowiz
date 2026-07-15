# Tech Synthesis 2026-07-15 — Blueprints

> Format matches this repo's convention (see BLUEPRINTS-OPS-RELIABILITY.md,
> BLUEPRINTS-INTEGRATION-PORTS.md): Мета · Межа · Форма · Reuse/джерело · RED · Хвиля.
> Only Tier 1 + Tier 2 items get full blueprints — Tier 3 items have no blueprint by design
> (not recommended on current evidence, see PLAN.md for the one-line reason each).

## Group A — buildable now (Tier 1)

### TS-01 · Harmonic centrality / information centrality on the spectral kernel
- **Мета:** replace the binary reachable/unreachable bit with a continuous centrality measure over
  the same graph-distance data the kernel already computes.
- **Межа:** ЧІПАЄМО — a new function in `kernel/src/spectral.rs` (or adjacent) consuming the
  existing Laplacian eigendecomposition. НЕ ЧІПАЄМО — the eigensolver itself (`householder.rs`),
  the FSM graph-analysis primitives (`has_cycle`/μ/topo/reachable/ρ stay as-is).
- **Форма:** harmonic mean of shortest-path distances from a node to all others (handles
  disconnected graphs cleanly — `∞⁻¹ = 0`, unlike arithmetic-mean closeness centrality, which
  breaks on any unreachable pair). Second form: information centrality as the harmonic mean of
  pairwise resistance distances, computed directly from the existing Laplacian eigenmodes via the
  Moore-Penrose pseudo-inverse — no new decomposition needed, rides on what's already computed.
- **Reuse:** 100% — reuses `spectral.rs`'s Laplacian eigenmode output and the existing
  reachability BFS. Reference implementations for validation: NetworkX/igraph/Neo4j-GDS
  `harmonic_centrality` (cross-check output, don't import).
  **RED:** a synthetic disconnected graph must not produce NaN/Inf where the arithmetic-mean
  analogue would; harmonic centrality output must reduce to the existing binary reachable bit at
  the extremes (0 ⟺ unreachable). **Хвиля:** immediate — smallest, lowest-risk item in this plan.

### TS-02 · Vectorless RAG over `docs/design/`, `docs/adr/`, `docs/governance/`
- **Мета:** near-zero-latency retrieval for this repo's own structured doc corpus, ahead of the
  more expensive planned retrieval layers.
- **Межа:** ЧІПАЄМО — a new, additive retrieval layer (structure-index over doc headings/hierarchy:
  PLAN→BLUEPRINTS→RESEARCH-CONSPECT convention already followed repo-wide). НЕ ЧІПАЄМО — the
  planned trigram/BM25/HNSW/diffusion 4-layer system (this becomes a 5th, cheaper-first layer, not
  a replacement).
- **Форма:** parse doc structure (headings, cross-references via the existing `[[name]]`-style
  linking convention already used in memory files) into a navigable index; route a query to the
  right document/section by structure-match before falling back to the heavier layers.
- **Reuse:** the doc corpus's existing structure/convention IS the index — no new authoring
  discipline required, just a parser over what's already there.
  **RED:** a query answerable purely by structure-navigation must resolve without ever invoking the
  embedding/vector layers (prove the cost-avoidance, not just correctness). **Хвиля:** immediate,
  additive, no dependency on the pgrust/mesh work landing first.

### TS-03 · Authorized Hydra credential-testing against staging
- **Мета:** prove — not assume — that rate-limiting/lockout/bot-challenge defenses on dowiz's login
  surface hold under realistic parallelized brute-force load.
- **Межа:** ЧІПАЄМО — a scoped, time-boxed test run against **staging only**, using Hydra's
  `HTTP-FORM-POST` module against the existing login endpoint. НЕ ЧІПАЄМО — production, any
  third-party system, any account not created specifically for this test.
- **Форма:** verify three things empirically: (1) Cloudflare rate-limit + Bot Fight + Turnstine
  (already planned per OPS-12) actually blocks scripted submission at the ~110x parallelism Hydra's
  own benchmarks demonstrate (92min→50s at 128 threads on a comparable protocol), (2) account
  lockout triggers after N failed attempts, (3) response timing doesn't leak username existence.
- **Reuse:** OPS-12 (layered rate-limiting) is already blueprinted — this is the verification step
  for it, not new infra.
  **RED:** the test run itself IS the red-proof — if Hydra's scripted attempts succeed at volume
  against staging, that's a real, confirmed gap in already-planned defenses, not a hypothetical.
  **Хвиля:** after OPS-12 lands (testing a control that doesn't exist yet proves nothing).

## Group B — conditional (Tier 2), blueprint kept minimal since these are gated on a future state

### TS-06 · Mesh-LLM as the LLM-agent port adapter (if/when LLM-agent infra is built)
- **Мета:** if LLM-agent infra is ever built, avoid a from-scratch router/gateway.
- **Межа:** ЧІПАЄМО — a new port adapter calling Mesh-LLM's local OpenAI-compatible `/v1` API as an
  untrusted external process. НЕ ЧІПАЄМО — the kernel; no direct linkage, ever (R0 compilation
  firewall already enforced elsewhere in the ports architecture applies here too).
- **Форма:** HTTP sidecar behind the existing port pattern; the port itself must implement the full
  capability/attenuation check, since Mesh-LLM's own "owner-control plane" is a bearer-style local
  API, not a macaroon/biscuit system.
- **RED:** the adapter must be provably incapable of reaching kernel state even if Mesh-LLM itself
  is fully compromised (same R0/R3 pattern as every other port). **Хвиля:** gated on LLM-agent infra
  actually entering scope — not scheduled.

### TS-05 · Gaussian Splatting via `web-splat`/`brush` (if 3D content enters scope)
- **Мета:** if openbebop's physics-UI ever needs photogrammetric/3D content, avoid the
  non-commercial-licensed reference implementation.
- **Межа:** ЧІПАЄМО — a wgpu compute+render pass (GPU radix-sort depth ordering + instanced
  ellipse rasterization, per `web-splat`'s demonstrated pattern). НЕ ЧІПАЄМО — no existing content
  pipeline changes required until this is actually scoped.
- **RED:** must render correctly on the non-Chrome browsers the product actually needs to support
  today — current WebGPU-in-browser reality (Chrome-134+-only) makes this the real gating test, not
  raw fps. **Хвиля:** not scheduled; no current content-authoring need identified.
