# Session Index — 2026-07-16 (sovereign architecture + living interface + harness)

> One index tying together everything produced this session. Each linked document is the
> single source of truth for its own scope — this page is navigation, not a summary that can
> drift out of sync. Read the linked doc, not a paraphrase of it.

---

## 1. Sovereign Architecture Roadmap — the whole 147-anchor target

**Entry:** [`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`](MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md)
**Detail:** [`sovereign-roadmap-2026-07-16/`](sovereign-roadmap-2026-07-16/) (30 files: 5 gap-analyses R1-A..E,
1 merge R2, 19 phase blueprints P01-P19, self-critique + credits + harness-research follow-ups)

19 dependency-ordered phases covering every locked anchor in `ARCHITECTURE.md` (M1-12, V1-6, D1-8,
S1-9, E1-62, F1-50) — mesh-foundation PQ crypto, hub autonomy, kernel correctness, delivery-on-protocol,
dispute/escrow, living-organism self-mod, product UI rebuild, public-flip readiness, ecosystem growth.
Critical path: **P3 → P9 → P10 → P13 → {P14,P16} → P17**.

**Self-critique already applied** (`sovereign-roadmap-2026-07-16/SELF-CRITIQUE-2Q-DOUBT-AUDIT.md`):
triple-confirmed the llama.cpp/GPU-gating error (corrected in P05/P15/R2) and flagged the G11-vs-crypto
priority tension — the same tension resolved for the living-interface arc below (§3).

**Companion research** (same directory): `HARNESS-RESEARCH-revfactory-and-agentic-teams.md`,
`LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md`, `HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md`,
`OPEN-SOURCE-CREDITS-LIST.md` — these seeded §4 below.

---

## 2. Living Interface Roadmap — the GPU wave/neural-field UI

**Entry:** [`living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md`](living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md)
**Detail:** [`living-interface-2026-07-16/`](living-interface-2026-07-16/) (13 files: 4 research passes
R-LM/R-SON/R-DEV/R-VENDOR, external research, the roadmap itself, 7 phase blueprints)

12 phases (0–8, 9a, 9b, 10) extending the pre-existing `field-ui-engine`/`rust-engine-rewrite`/
`dowiz-interfaces` blueprints with three new surfaces: a 3-tier (mesh/hub/node) living-memory 3D
visualization, sonification as a third renderer of the one field, and a vendor brand-token pipeline
ported to GPU. Self-critique corrected two synthesis-level findings (J2 payload mismatch, J4 brand
contradiction) and — **per operator ruling 2026-07-16** — resequenced so the customer order path
never waits on the audio/memory-viz enhancements it doesn't need.

### 🎯 G11 fast-path (confirmed shortest path to the first real order)
```
0 (dev/CI+CSP fix) → 1 (brand token source) → 2 (GPU engine foundation)
  → 3 (render primitives+brand-on-GPU) → 4 (field dynamics+guards) → 5 (spectral/Green's feedback)
  → 6 (Sea&Sheet backbone + event-ordering authority) → 9a (order-critical product surface = G11)
```
Skips Phase 7 (sonification), Phase 8 (memory-viz), Phase 10 entirely — none is required for a customer
to browse, cart, checkout, and track an order.

**All 6 fast-path phases now have execution-ready blueprints:**

| Phase | Blueprint | Headline finding |
|---|---|---|
| 0 | `BLUEPRINT-P00-dev-ci-deploy-enablement.md` | CSP missing `'wasm-unsafe-eval'` blocks wasm in prod *today* |
| 1/3/4 | `BLUEPRINT-P01-brand-token-pipeline.md` | One canonical `resolve(T1)` — zero brand-token drift by construction |
| 2 | `BLUEPRINT-P02-gpu-engine-foundation.md` | No path reaches a real GPU device yet; `upload_once()` already does real staging (W20) |
| 6 | `BLUEPRINT-P06-sea-sheet-backbone-event-stream.md` | Two-layer ordering authority: `event_log` (common) + `order_machine` (order-path only) |
| 8 (primitive only) | `BLUEPRINT-P08-living-memory-viz-phase0.md` | The one net-new kernel primitive (spectral coords) — memory-viz-only, off the G11 path |
| **9a** | **`BLUEPRINT-P09A-order-critical-product-surface.md`** | **⚠ `place_order_js` trusts client-supplied `unit_price`; server `PriceCatalog` sits unused — now a named blocking prerequisite** |

Off-path (still fully blueprinted, for the growth-substrate track): `BLUEPRINT-P07-sonification-phase0.md`
(sound as third renderer), `BLUEPRINT-P08` (full — living-memory 3D viz).

---

## 3. Operator rulings this session (what's decided, not just proposed)

- **GPU-gating corrected**: llama.cpp needs no GPU (triple-confirmed); split into `E13-cpu`
  (DECART+operator-go only) vs `E13-gpu` (real GPU-unlock). Applied to both roadmaps.
- **Living-interface access**: living-memory viz is per-role configurable (operator-set).
- **Render split**: hybrid — server (Rust) computes graph-layout/state, client (wgpu) does final
  compositing. Backend generates, frontend renders — never the reverse.
- **3-tier hierarchy**: mesh → hub → node architected into the wire protocol from day one, even
  though Phase-0 build targets hub-level only.
- **GPU-less Hetzner VPS**: confirmed data-only server; Mesa Lavapipe for GPU-less CI/dev.
- **G11 priority (2026-07-16, the load-bearing ruling)**: *"It DOES lead to a real order — once the
  interface is built, that resolves the question."* Commercial-delivery-first. Verified the
  consequence (Phase 9's audio/viz dependency was artificial) and resequenced Phase 9 → 9a/9b so the
  order path is the shortest chain in the roadmap.

---

## 4. Harness completion (vLLM + local Ollama) — next, tracked separately

See [`harness-2026-07-16/HARNESS-LLM-BACKEND-PLAN.md`](harness-2026-07-16/HARNESS-LLM-BACKEND-PLAN.md)
once written (this session, immediately following this index) — extends
`sovereign-roadmap-2026-07-16/HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md`'s H1-H3 with the actual local
inference state on this host (Ollama already running: `llama3.1:8b`, `qwen2.5-coder:7b`,
`nomic-embed-text`, `qwen3-embedding:0.6b`), caching (content-addressed exact-match + embedding-based
near-duplicate via the already-pulled embedding models), and parallel-processing design. Implementation
begins on a dedicated branch (see that plan for the branch name), separate from the planning work
above — code changes are NOT part of this index's scope.

---

## 5. What none of this touches

- `kernel/src/hydra.rs` and related G9 breach-witness work — a parallel, independent thread this
  session did not author (visible in `git log` as `feat(hydra): ...` commits interleaved with the
  docs commits above). Not part of any roadmap in this index.
- `ARCHITECTURE.md` / `STRATEGIC-VECTORS-LOCKED.md` themselves — every roadmap above proposes canon
  corrections (see each roadmap's own preamble) but none edits the canon files directly, per their own
  "merge, never append" rule. That merge is the operator's action, not automated here.
