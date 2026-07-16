# SWARM-MANIFEST — finish-all-6 (2026-07-16)

> Spec-driven swarm to close the 6 open tracks from ORGANISM-STATUS 07-15 + KU03 spec.
> Research/structure done (git/grep/cargo + .specify/KU03 + ORGANISM-STATUS). Ground truth
> outranks this doc — every unit re-verified against disk before dispatch.
> Workflow: spec-driven-swarm + sdd-spec-driven. Mirror-dialogue injected into every subagent brief.
> Push plans FIRST (this commit) before Wave-1 dispatch.

## 0. GROUND TRUTH (re-verified 2026-07-16)
- dowiz `origin/main` = `6c7212b5` (325 kernel lib tests green). bebop `openbebop/main` = `b87b7e2`.
- `wgpu` crate NOT in cargo cache (~/.cargo/registry/src: no wgpu/gpu/ash/naga); absent from every Cargo.lock. → GPU path unbuildable air-gapped (decart W15: reject offline).
- `web/src` EMPTY (0 .mjs/.js) — spectral/order_machine/geo wasm bridges regressed by JS-purge `f9ab28ff`.
- governance hooks: 10 scripts present but ALL no-op (CLAUDE.md: "SUSPENDED operator directive 2026-07-15"). Deliberate, not a bug.
- `kernel/src/{kalman,trigram,living_knowledge}.rs` exist; ORGANISM-STATUS: "11/11 organs stranded" (math present, not consumed by loop/decide).

## 1. THE 6 TRACKS → 6 WAVES
| # | Track (unit) | Repo/crate | RED→GREEN gate | Owns (disjoint) | Red-line |
|---|---|---|---|---|---|
| W17 | web-UI wasm-bridge restore (Rust-native, NO TS) | dowiz / web+engine | browser smoke renders ρ/drift/FSM from kernel wasm (0 JS re-impl) | web/src/* (new), web/index.html, web/serve.mjs | no (UI) |
| W18 | living-knowledge Rust wiring | dowiz / kernel | recall@k via Rust adapter consumed by self-improve loop | kernel/src/living_knowledge.rs, retrieval/mod.rs | no |
| W19 | Kalman/trigram integration into decide/loop | dowiz / kernel+engine | loop consumes kalman.predict + trigram pattern (RED: stranded) | kernel/src/lib.rs wiring, telemetry/*.rs | no |
| W20 | VertexBridge CPU-complete + gpu feature-gate (honest stub) | dowiz / engine | `cargo build --features gpu` OK + headless 0 uploads, 1 mock | engine/src/bridge.rs, engine/Cargo.toml | no |
| W21 | Field-UI GPU render loop (FE-04/05/08–17) | dowiz / engine | **BLOCKED offline** — wgpu uncached; feature-gated shell per W15 decart | engine/src/{field_frame,gpu}.rs | no (but network gate) |
| W22 | Governance/doc finalize + DOD retro | dowiz / docs | governance documented as operator-suspended; DOD retro 0-failed proof | docs/design/*, .specify/* | docs only |

## 2. WAVE ORDER + DEPENDENCIES
- W17 → W18 → W19 layer on existing kernel math (each consumes prior). Parallel-safe within a wave (disjoint files).
- W20/W21 are engine-side (separate crate) → can run parallel to W17-19 BUT W21 blocked on wgpu.
- W22 is terminal (retro after W17-21 verify).
- **W21 is the only hard blocker.** Decision recorded in decart (§3): wgpu cannot build offline → either (a) operator grants network `cargo add wgpu`, or (b) accept feature-gated ceiling (documented, no GPU raster yet). Until then W21 = documented ceiling, NOT fake-green.

## 3. DECART — wgpu (re-verified 2026-07-16)
| criterion | wgpu (uncached) | CPU field-frame (shipped W10) | hand-roll GPU on cached deps |
|---|---|---|---|
| offline build | ✗ NOT in cache | ✓ built+served | ✗ no GPU binding crate cached |
| browser-observable | identical raster | identical raster | n/a (impossible) |
| supply-chain | ✗ large GPU tree | ✓ zero | ✗ impossible |
| falsifiable | ✓ but unbuildable | ✓ green | ✗ |
DECISION: **reject wgpu offline** (cache miss = air-gap fail). Keep `feature="gpu"` gate EMPTY (no dep) so the flag exists; ship `HeadlessGpu` mock so headless GREEN holds. PROBE: a real GPU raster is the product's headline visual — but it cannot be built/tested without network. TRIGGER: "when `cargo add wgpu` succeeds (network), implement real `wgpu::Device`/`Surface` + run GPU-raster smoke in CI." OLDER-AS-ADAPTER: CPU `FieldSim`/`compose_field` is the carrier; wgpu is additive behind the flag.

## 4. DISPATCH RULES (from spec-driven-swarm)
- Each leaf brief self-contained: inline blueprint unit + RED→GREEN + Owns + "do NOT commit/push".
- Mirror-dialogue mandate injected: "State design ≤5 lines, critique opposite stance, resolve ≤2 laps, else least-friction."
- MAIN re-verifies every lane with literal `cargo test`/`cargo build` before commit. Distrust subagent "green".
- Feature-flag trap: grep changed module for `#[cfg(feature=…)]`; confirm same gate as imported symbols; re-verify under dependent crate's feature set.
- Red-line (money/auth/RLS/migrations) — none in W17-22 except W20/W21 are engine-only, no red-line crossed.

## 5. PLANS PUSHED FIRST
This manifest + BLUEPRINT-W17..W20 + decart committed+ushed BEFORE Wave-1 dispatch (operator process rule 2026-07-16).

> Generated 2026-07-16. Re-verify each unit against disk before dispatching — stale manifest lines lie.
