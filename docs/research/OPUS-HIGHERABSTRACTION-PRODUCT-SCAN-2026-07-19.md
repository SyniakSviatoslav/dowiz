# Opus — Higher-Abstraction Product-Layer Scan (2026-07-19)

> RESEARCH-ONLY. Zero code written, no branches touched. Checks whether the
> spectral / retrieval / physics-wave / tensor toolkit has any *genuinely new*
> purchase on four **product-layer** surfaces the operator named:
> **(1) WebGPU design logic · (2) geomapping · (3) ETA prediction · (4) marketing.**
> Discipline of the day: most checks come back **already-covered** or
> **honest-negative**; novelty is not manufactured. Every claim is cited to
> `file:line` from a live read of the working tree (HEAD `1d2e3d279`), not memory.

---

## 0. Provenance caveat — the two named prior docs do not exist on disk

The task pointed me to read `docs/research/OPUS-SPECTRAL-EVERYWHERE-SWEEP-2026-07-18.md`
and `docs/research/OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md`
"first, don't duplicate." **Neither file exists** — verified by a filesystem-wide
`find /` (only node_modules-free paths), by `ls docs/research/`, and by a name-glob
across all worktrees and the scratchpad. They were never committed (or were named
differently / discarded). I therefore reconstructed "what today's spectral/wave sweep
already covered" from the code those docs would have touched and from the durable
prior synthesis that *does* exist on disk:
`docs/design/GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md`. Where I say
"already covered," it is grounded in code or that doc, not in the missing files.

---

## 1. WebGPU design logic — **ALREADY COVERED (implementation gap, not a research gap)**

**Verdict: already covered; nothing new for spectral/retrieval to find in the
*pipeline design logic itself*.** This is the most-covered of the four, exactly as
predicted.

What actually exists on the GPU surface today:

- **The architectural rule is settled and load-bearing:** `engine/src/lib.rs:4` —
  "GPU/wasm is a **display surface**; authoritative compute is CPU-side." Every GPU
  path in the repo is additive and degrade-not-crash.
- **Real headless wgpu bring-up already landed**, feature-gated: `kernel/src/render/gpu.rs:1`
  (P38 O18a) constructs a live `wgpu::Instance`/`Adapter`/`Device`, returns a typed
  `GpuContext`, and models GPU absence as a *typed value* `GpuError::NoAdapter`
  (`render/gpu.rs:19`), never a panic. Async bridged by `pollster::block_on` under
  the `gpu` feature.
- **The compute the GPU would run is fully specified and CPU-proven:** the physics
  operator `M·U̇ = −ΓU̇ − c²·L·U + S` (`engine/src/field_frame.rs:11-15`) on the
  row-major SDF field buffer (`engine/src/scene.rs:1`, `zerocopy.rs`), with `L` the
  5-point Laplacian. `docs/research/BLUEPRINT-W15-wgpu-shell.md` defines the
  `GpuField` trait whose GPU output is **byte-identical** to `FieldSim::frame()` —
  the pipeline is a backend swap, not a redesign.
- **The one genuinely GPU-pipeline-specific concern — depth/alpha compositing sort
  order for translucent primitives — is already answered** in the Gaussian-Splatting
  synthesis §2.5: Tier-A WebGPU on-GPU radix sort, Tier-B WebGL2 CPU counting-sort at
  ~4 Hz on quantized 16-bit depth keys. That is the correct, budget-first answer and
  needs no spectral method.

**Why there is no new spectral/retrieval angle *on the pipeline itself*:** the
spectral/eigenmode work (Laplacian modes for modal motion) is the *math that runs on*
the surface, and that is precisely what the (missing) spectral-everywhere sweep and
`spectral_laplacian.rs` already own. The pipeline plumbing — bind-group layout, render
state, sort, upload boundary — is engineering with settled answers (W15 trait,
`zerocopy.rs` single-`writeBuffer` upload contract). The only open item is that
**`wgpu` is not in the offline crate cache**, so the feature cannot compile here today
(W15 "Honest ceiling", `BLUEPRINT-W15-wgpu-shell.md:53`). That is a build/dependency
gap, not a research question. **No new target. Pointer: W15 + GS §2.5.**

---

## 2. Geomapping — **HONEST-NEGATIVE for spectral partitioning; the one routing
problem that exists is already optimally solved by non-spectral code**

**Verdict: no new spectral opportunity with a real product need.** Spectral graph
partitioning (Fiedler bisection / spectral clustering) is a real, established
technique, and — notably — **the machinery is already in the kernel**: `spectral.rs`
computes the Fiedler value λ₂ (`spectral.rs:591`, `:632`, algebraic connectivity) and
`spectral_laplacian.rs:83` produces the Laplacian eigenmodes. The honest problem is
that **dowiz has no geomapping surface that consumes a partition.** Three independent
walls, any one sufficient:

1. **The routing problem dowiz actually has is single-pair shortest path, and it is
   already solved optimally without spectral methods.** `kernel/src/router.rs` is a
   CSR-native Dijkstra / A* road router with OSM road-graph ingestion and an
   admissible haversine heuristic (`router.rs:1-15`, `:90`). Spectral methods do not
   improve exact single-pair shortest path — Dijkstra/A* is already optimal. There is
   **no multi-stop batching / VRP surface**: a full grep for `batch.?order` /
   `multi.?courier` in the kernel returns nothing.

2. **Delivery zones are deliberately operator-drawn polygons, not algorithmically
   partitioned.** Zone membership is ray-cast `point_in_polygon` ported from
   `delivery-zone.ts` (`geo.rs:200`, `geo.rs:198` comment). A merchant chooses *where
   it delivers*; that is a business decision, not a balanced-districting optimization.
   Spectral k-way districting solves a problem (balanced territory partition across a
   fleet) that this product does not pose.

3. **The obvious consumer of a load-balanced partition — courier↔zone assignment — is
   a hard red-line.** `kernel/src/decision/mod.rs:27` — "NO ordering, NO numeric rank:
   routing is `match`, never comparison (**NO-COURIER-SCORING**)"; `:30` makes the
   absence of an order relation a *type-level* enforcement of the no-courier-scoring
   law (mirrored by `blocklist.rs:189`). A spectral load-balancer that ranks couriers
   or optimizes their allocation would brush directly against this invariant.
   Partitioning *the map* is technically distinct from ranking *couriers*, but with no
   dispatch optimizer to feed and a red-line guarding the natural one, the technique
   has no home.

**Distinct from the address-picker.** The Gaussian-Splatting synthesis
(`GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md`) owns the *3D-scene /
last-100m facade* problem (courier-photo bootstrap, WebGL2-primary, no satellite,
CuPy/trained-reranker/TimesFM all rejected). Spectral zone-partitioning is a genuinely
different question — I did not conflate them — it simply has no product surface here.
**If a fleet-districting need ever materializes, the parts are in place** (`spectral.rs`
λ₂ + `spectral_laplacian.rs` eigenmodes + `router.rs` road-graph CSR), so this is a
"defer, machinery ready" not a "build now." **Flag: unmotivated + red-line-adjacent.**

---

## 3. ETA prediction — **REAL, SMALL, NON-ML TARGET: the estimator the prior doc
called "already optimal" exists but is NOT actually wired to the speed term**

**Verdict: a genuine, deterministic, no-new-dependency improvement — and it is
explicitly *not* re-proposing the rejected TimesFM.**

First, the two facts the task asked me to confirm:

- **dowiz DOES have ETA today** — it is not absent. `kernel/src/geo.rs:153`
  `eta_seconds(remaining_m, total_m, baseline_s)` is surfaced through
  `ports/customer.rs:203` (`TrackingView.eta_seconds`, `:180`) and rendered on the
  courier app as "ETA {}s" (`apps/courier/src/render.rs:101-109`, `types.rs:80`).
- **The TimesFM rejection is real and current.** GS synthesis §1 / §3 / §5:
  *"TimesFM — NO for per-order ETA (`geo::eta_seconds` + `kalman.rs` are already the
  optimal linear estimator …)"* and *"TimesFM for per-order ETA — rejected. Kalman/EMA
  are already the optimal estimator … a 200M-parameter transformer is worse on every
  axis that matters on the hot path."* (`…SYNTHESIS-2026-07-16.md:44-46`, `:401-403`).
  This scan does **not** disturb that ruling.

**The genuine finding — the "already optimal" claim is aspirational, not wired:**

- `eta_seconds` computes speed as a **static** `total_m / baseline_s`, with a hardcoded
  `5.0 m/s` urban fallback (`geo.rs:157-161`). The `baseline_s` is **passed in from
  outside** and is not derived from the courier's live motion (`ports/customer.rs:198`,
  `:224`, `:315`).
- The live signal exists but is thrown away for ETA: the courier `TrackFrame` already
  carries `v_mps` ground speed (`apps/courier/src/types.rs:79`) and the kernel has
  `geo::ema_next` (`geo.rs:39`, scalar steady-state filter) plus a full n-D Kalman
  filter (`kalman.rs:149`).
- **Crucially, the Kalman filter is already being stepped on courier observations —
  but only its *surprise scalar* is surfaced, never a speed/velocity estimate that
  feeds the ETA.** `ports/customer.rs:191-192` — "`kalman_surprise` is the
  dimensionless novelty scalar from a `KalmanFilter` already stepped on courier
  observations"; `from_kalman` pulls only `kalman.last_surprise()` (`:213`). And
  `kalman.rs` itself is wired into a **courier/trust state estimate** in
  `domain.rs:296-323` (a trust filter), *not* into the ETA path.

So the prior doc's "Kalman/EMA are already the optimal estimator" is true about the
*primitives* but false about the *wiring*: the ETA speed term is a static baseline, and
the adaptive estimator the doc credits is not connected to it.

**Sketch (blueprint-ready, non-ML, zero new deps):** feed the courier's smoothed
observed ground speed into the ETA speed term instead of the static baseline. Pure
kernel, RED→GREEN, in the existing deterministic style:

- Smooth `v_mps` with the existing `geo::ema_next` (`geo.rs:39`) — a documented 1-D
  steady-state Kalman — over the courier ping stream, producing `v̂_mps`.
- Add an overload, e.g. `eta_seconds_from_speed(remaining_m, speed_mps)`, or thread an
  optional `observed_speed_mps` into `from_positions` that overrides the baseline when
  present and falls back to `baseline_s` (then `5.0`) when the courier hasn't moved
  yet. Guard against `speed ≤ 0` exactly as the current `f64::INFINITY` arm does
  (`geo.rs:162-164`).
- Optionally graduate to the full `KalmanFilter` (`kalman.rs`) with a constant-velocity
  `F`/`H` state so ETA consumes the *velocity* component that is currently computed and
  discarded — matching the doc's own "Kalman is the optimal estimator" framing by
  actually using it.

**Why this respects the standing policy:** it is deterministic, bit-reproducible,
`no_std`-friendly, adds no weight file and no dependency, and reuses `ema_next` /
`kalman.rs` that already exist. It is the *opposite* of TimesFM — it closes the exact
gap the TimesFM rejection assumed was already closed. **Falsifiable acceptance:** on a
replayed ping track where the courier's real speed differs from `baseline_s`, the
adaptive ETA's mean absolute error against actual arrival beats the static-baseline ETA;
if it does not, keep the baseline. Small, honest, real.

---

## 4. Marketing — **HONEST-NEGATIVE (the cleanest of the four), as expected**

**Verdict: no spectral / retrieval / physics target; and there is barely a marketing
code surface to begin with — by deliberate product design.**

What "marketing" resolves to in this repo, in full:

- **A drafted-post pane, not a growth engine.** P22 `MasterPost` (`ports/owner_surface.rs:203`)
  is a single owner-drafted post with a one→many public blast radius, deliberately
  *not* merged with the transactional per-recipient `Notification` lane (`:204-205`).
  The G5 "Marketing auto-posting pane" (`owner_surface.rs:648`) is a Wave-0 template
  drafter: `draft_master_post` fills a template body (`"New on the menu: …"`), drafts
  at `AiMode::Off` and `status: PendingReview`, and **publish is an explicit owner tap,
  never automated** (`owner_surface.rs:650-665`, `AiMode::Off` at `:199`).
- **SEO / schema bot-pack.** The landing "bot-pack" emits JSON-LD / OG / sitemap /
  robots / manifest (`kernel/src/landing/mod.rs:20`, `json_api.rs:323`) — a
  marketing-*schema* sibling, static metadata, not an algorithmic surface.
- **Everything a spectral/retrieval method could bite is explicitly rejected by
  ruling.** `ports/customer.rs:7` (operator ruling 2026-07-18): "**NO loyalty / CRM /
  marketing identity** … NO email/SMS, NO contact channel." A repo-wide grep for
  `promo|coupon|discount|referral|loyalty|voucher` finds **no promo-code, coupon,
  referral, or loyalty engine** — the only `discount` hit is a *tax* comment noting the
  subtotal has "no discount" (`domain.rs:135`); the only `voucher` is a crypto-cert
  metaphor (`capability_cert.rs:310`); `PromoAnnounced` is just a free-text template
  variant (`owner_surface.rs`), not a discount system.

There is no audience graph to cluster, no engagement time-series to forecast, no
content corpus to rank — because the product deliberately holds no marketing identity
and runs no campaigns. The faint theoretical adjacency (embedding-dedup of drafted post
bodies) is speculative, unneeded, and would be manufacturing novelty against the day's
discipline. **No target. This is a clean, honest negative — and that is the correct
outcome, not a miss.**

---

## 5. Scorecard

| Surface | Verdict | One-line reason | Pointer |
|---|---|---|---|
| **WebGPU design logic** | **Already covered** | Pipeline plumbing is settled (W15 trait, `render/gpu.rs`, `zerocopy.rs`); the math on it is the spectral work already owned; sort/compositing answered in GS §2.5. Only gap = `wgpu` not in offline cache (build, not research). | W15, GS §2.5 |
| **Geomapping** | **Honest-negative** (defer, machinery ready) | Spectral partitioning is real, and λ₂/eigenmodes already exist (`spectral.rs:591`, `spectral_laplacian.rs:83`) — but no consumer: single-pair routing already optimal via Dijkstra/A* (`router.rs`), zones are hand-drawn (`geo.rs:200`), and the natural consumer (courier assignment) is a NO-COURIER-SCORING red-line (`decision/mod.rs:27`). | §2 |
| **ETA prediction** | **REAL small non-ML target** | ETA exists (`geo.rs:153` → `customer.rs:203`) but speed is a *static* baseline; the adaptive estimator the prior doc credits (`ema_next`/`kalman.rs`) is stepped yet only its surprise scalar is surfaced (`customer.rs:191-213`) — live `v_mps` (`courier/types.rs:79`) is discarded. Wire observed smoothed speed into the speed term. NOT TimesFM (correctly rejected, GS §5). | §3 |
| **Marketing** | **Honest-negative** (cleanest) | Marketing = an owner draft-post pane (`owner_surface.rs:203/648`) + SEO bot-pack (`landing/mod.rs:20`); loyalty/CRM/promo/referral all explicitly absent by ruling (`customer.rs:7`). No graph, series, or corpus to apply anything to. | §4 |

**Net:** three of four are already-covered or honest-negative (as the day's discipline
predicts); the single real find (ETA speed-term wiring) is small, deterministic,
dependency-free, and does the *opposite* of re-proposing a rejected trained model — it
connects the estimator the codebase already claims to use.

---

*Method: live reads of `kernel/src/{geo,router,spectral,spectral_laplacian,kalman,decision/mod,ports/customer,ports/owner_surface,render/gpu,landing/mod}.rs`,
`engine/src/{lib,scene,field_frame,zerocopy}.rs`, `apps/courier/src/{types,render}.rs`,
and `docs/research/BLUEPRINT-W15-wgpu-shell.md`. Prior synthesis:
`docs/design/GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md`. The two
07-18 spectral/wave docs named in the brief do not exist on disk (§0).*
