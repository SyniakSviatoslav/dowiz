# Gaussian-Splatting Address Picker + Reranker/TimesFM — Reasoning Synthesis (2026-07-16)

> Method: one reasoning pass over six independent Opus research reports (reranker,
> TimesFM, CuPy-reconsidered, GS rendering/budget-devices, satellite/building-data/UX,
> GPU rental). The reports are cross-referenced, not concatenated; where two reports
> independently reach the same conclusion this is flagged as cross-confirmation.
> Consistent with the prior verdicts in `docs/design/tech-synthesis-2026-07-15/`
> (CuPy Tier-3/no-hook; TS-05 conditional) and with
> `docs/design/SYSTEMS-GPU-ML-KERNEL-SYNTHESIS-2026-07-16.md` (Trait-as-Port;
> content-addressing as universal cache key). Explicit operator lens for the whole
> effort: **бюджетність та бюджетні пристрої** (budget-friendliness, budget devices) —
> §6 audits every recommendation against that lens honestly.
>
> Raw inputs: scratchpad `splat-research/01..06-*.md` (session 75493fed). Kernel
> ground truth: `/root/dowiz/kernel/src/geo.rs` (haversine_meters, bearing_deg,
> point_in_polygon, progress_along_route, ema_next, eta_seconds), `kalman.rs`,
> `bm25.rs`, `ppr.rs`, `attention.rs`, `backup.rs` (sha3 content-addressed BlockStore).

---

## 1. Executive summary

**What was asked.** Six threads: (1) do we need a trained ML reranker for internal
retrieval; (2) is TimesFM useful, and does it connect to Gaussian Splatting; (3) does
the splat workload change the CuPy verdict; (4) how do splats render on budget
devices; (5) where does the 3D address-picker's data and UX geometry come from;
(6) how is the one-time GPU reconstruction paid for.

**One-sentence verdicts:**

- **CuPy — NO, twice over.** Kernel verdict unchanged (Tier 3, no hook — N≤32
  eigensolve is 2–3 orders below GPU break-even); for the offline splat pipeline the
  old "not Rust-integrable" objection becomes *moot* (out-of-process job), but CuPy
  is still not the tool — real 3DGS trainers are PyTorch + hand-written CUDA
  (graphdeco/gsplat) or Rust-native (brush); CuPy is at most optional Python-side
  preprocessing glue we do not need.
- **Reranker — NO trained model.** A deterministic hand-tuned fusion of the signals
  the kernel already emits (BM25 + PPR + exact-match + recency/status) is
  right-sized and sufficient; a cross-encoder/ColBERT solves web-scale ambiguity
  dowiz does not have, at a training-corpus cost dowiz cannot honestly pay.
- **TimesFM — NO for per-order ETA** (geo::eta_seconds + kalman.rs are already the
  optimal linear estimator for that problem); **conditionally MAYBE later** for
  aggregate multi-merchant demand forecasting, only behind an out-of-kernel adapter
  and only after beating a classical seasonal baseline (ETS/STL/Theta) on a backtest
  of our own history. TimesFM and Gaussian Splatting are **unrelated threads** —
  no connection is manufactured (§3).
- **Address picker — BUILDABLE, in two decoupled layers.** A no-splat v1 (pin-drop +
  OSM floor-slice + orientation-arrow/FOV geometry as pure geo.rs extensions) ships
  now with zero new dependencies; the splat layer (courier-photo bootstrap → per-job
  rented GPU behind a port → compressed content-addressed asset → WebGL2-primary
  tiered client rendering) is a real, costed, budget-first architecture that layers
  on top without touching the kernel or the default build.

---

## 2. The address-picker — one coherent architecture

Reports 03, 04, 05, 06 compose into a single pipeline. Each stage below names which
report owns it and where the seams are.

### 2.0 The scene the whole pipeline serves (report 05's scoping win)

The user has already dropped a 2D pin (Stage 1 — geo.rs haversine/point_in_polygon
localize it). Stage 2 needs only the **immediate building facade + street-frontage
strip** — not a neighborhood. That single scoping decision is load-bearing for
everything downstream: a tightly cropped frontage scene is 1–2 orders of magnitude
fewer Gaussians than a block-scale reconstruction, which drops "millions of splats,
needs aggressive LOD" to "tens of thousands, comfortably inside the WebGL2 budget
tier with light quantization." Reports 04 and 05 reach this independently from
opposite ends — 04 from the render budget, 05 from the content scope — and the
match is exact. **Cross-confirmation #1.**

### 2.1 Data sourcing — courier photos, not satellite (report 05)

Satellite/aerial-first is rejected on three *independent* walls:

1. **Legal** — Google/Mapbox tile ToS explicitly prohibit mass download, caching
   beyond render, and derivative datasets; Bing oblique likewise and the platform is
   sunsetting. This is a hard no regardless of feasibility or budget.
2. **Technical shape** — GS needs many overlapping *posed* views of a facade;
   a nadir satellite tile is one viewpoint of a rooftop. Wrong input shape even if
   the licence were clean (Sentinel-2 at 10 m/px is one pixel per apartment).
3. **Economics** — satellite-first pays upfront for global coverage mostly unused.

The correct source is **crowd-sourced ground photos captured by couriers at delivery
time** (with consent; we own the licence). This is exactly the multi-view,
multi-angle, entrance/doorway-detail input GS wants, and coverage accretes precisely
where demand is: after N deliveries an address has N photo sets from slightly
different positions — a free operational byproduct. **Coverage follows demand;
we never need coverage we don't use.**

Footprints and floors come from **OSM `building=*` + `building:levels`/`height`
(primary) and Overture buildings theme (conflated fallback)** — both free/ODbL.
Where tags are absent, degrade gracefully to a "ground-floor/single-level" mode:
show the footprint, let the user place position + arrow, offer no floor selector.
Never fabricate a floor count. Floor is **user-selected, never auto-derived** —
a 2D pin carries zero vertical information, and the honest design admits it.

### 2.2 Trigger and cost gate (report 06 §5, reusing report 05's demand signal)

Addresses do not get reconstructed speculatively. The queue is gated on the same
signal that produces the imagery: **delivery volume**. An address that has accrued
enough real deliveries has (a) the photos and (b) the justification, in one motion.
**Cross-confirmation #2** — 05's coverage-follows-demand bootstrap and 06's
"prioritize by delivery volume, not speculation" are the same policy discovered
from the data side and the cost side respectively.

Cost controls (all report 06): per-tenant submission rate limit reusing the
transport TokenBucket bulkhead; global max-concurrent-GPU-jobs semaphore; a hard
monthly GPU-spend ceiling that trips the port into queue-only (degrade-closed);
a per-job wall-clock teardown watchdog so a hung optimizer or flaky marketplace
host cannot leave a meter running; 7k-iteration previews (~10 min, ~⅓ cost) as the
default tier, full 30k iterations reserved for high-value addresses.

### 2.3 Offline reconstruction — rented GPU behind a port (reports 03 + 06)

**Cross-confirmation #3 (the strongest in the set).** Reports 03 and 06 arrive
independently — one from library-fit analysis, one from cost analysis — at the
identical architectural shape: **GPU work is offline, batch, behind a port; never
in-kernel, never in the request path.** Report 03: the old R1 objection ("CuPy not
Rust-integrable") is moot for a job that is not in-process — the kernel never links
it, never calls it; this matches Trait-as-Port. Report 06: a standing GPU bills for
idle; a one-time-per-address cached-forever workload wants per-job rental with
mandatory teardown. Same conclusion, two roads.

The port (report 06):

```rust
trait SplatReconstructionJob {
    fn submit(&self, req: ReconRequest) -> JobId;
    fn poll(&self, id: JobId) -> JobStatus;   // Queued|Running|Done(SplatRef)|Failed
    fn fetch_result(&self, id: JobId) -> SplatBlob;
}
struct RentedGpuAdapter { backend: Backend }  // Modal | VastApi | RunPodApi | HetznerGex44(future)
```

**Backend choice:** Modal as default adapter (~$1.25–1.67/job on A100, per-second
billing, true scale-to-zero, arbitrary Docker image, near-zero ops) with a
Vast.ai/RunPod marketplace adapter (~$0.06–0.25/job on 3090/4090-class) as the
cost-optimized swap once volume makes the babysitting worth it. Hetzner's own GPUs
(GEX44 €184/mo, GEX131 €889/mo, monthly dedicated only — Hetzner Cloud has no GPU
flavor) are **not the starting point**: at one-time-per-address volume a standing
box is pure idle cost; it becomes the right adapter only if reconstruction volume
ever saturates it, and the trait makes that a backend swap with zero kernel change.

**Trainer choice inside the container (report 03):** the decision is
**PyTorch+gsplat (mature reference) vs Rust-native brush (Burn+CubeCL — real,
trains end-to-end without Python, but trails on maturity with no published
benchmarks)**. CuPy sits outside this decision entirely. Recommendation: start with
the mature PyTorch+gsplat container — it is disposable, out-of-process, and behind
the port, so its sovereignty cost is contained; keep brush as the candidate second
backend and decide by a falsifiable bake-off (same imagery set → compare PSNR,
wall-clock, $/job) per the rust-native-default + DECART rule. SfM/pose recovery is
COLMAP (C++/CUDA) in the same container — no mature Rust SfM exists, and a
standalone binary behind the port drags no Python into *our* stack.
graphdeco-inria's non-commercial licence remains permanently rejected; everything
named here is Apache-2.0.

Software GPU emulation is **not** a fallback for training — 2–3+ orders of
magnitude too slow (a 30–40 min job becomes days-to-weeks; operationally infinite).
Its legitimate roles: CI shader-correctness tests on the GPU-less Hetzner box
(llvmpipe/SwiftShader/wgpu software adapter asserting PSNR-thresholded golden
output) and tiny 50-Gaussian smoke tests proving the harness is wired before
shipping the real job to rented silicon.

### 2.4 The asset — compression is the seam between reports 04 and 06

**This is the one explicit handoff in the set.** Report 04's compression choice
determines the object that report 06 caches and ships:

- **Format:** mosure's `gcloud` (f16/f32, ply→gcloud converter exists) — chosen
  because *one* asset format drives both the WebGL2 and WebGPU client tiers and the
  server pre-render (§2.5). Picking mosure's format is what collapses three render
  paths into one asset pipeline.
- **Size:** frontage-cropped scene (tens of thousands of splats) + quantization
  (Niedermayr et al. CVPR 2024: up to 31× compression, ~4× faster rendering on
  lightweight GPUs, explicitly built for streaming to low-power devices) →
  **single-digit-MB per-address asset**, CDN-cacheable.
- **Cache key (report 06):** `sha3(geohash(address) ⊕ hash(imagery_set) ⊕ params)`
  into the existing content-addressed BlockStore (`backup.rs`, sha3 + Buzhash-CDC
  dedup — already the port). Reconstruction runs exactly once per (locale, imagery)
  tuple; re-submissions are $0 cache hits; imagery change → new hash → new job only
  then. Stale scenes are never silently served; unchanged ones never recomputed.
  This is the "content-addressing as universal cache key" pattern from the prior
  synthesis, applied verbatim.
- Smaller asset ⇒ cheaper egress from the rented GPU, cheaper CDN storage, faster
  first-render on hostile networks — the compression lever pays three times.

### 2.5 Tiered client rendering — WebGL2 primary (report 04)

The TS-05 gate ("must render on non-Chrome browsers") is now answered with data:
WebGPU is ~84.7% global (Safari 26 ships it by default on iOS/iPadOS — the
2026-07-15 snapshot predates this; the old "Chrome-134+-only" fear was brush's
subgroup-feature floor, not the WebGPU baseline). But the missing ~15% is
disproportionately **the budget-device population this product serves** (old
Android WebView, Firefox-Android, sub-WebGPU GPUs). Hence:

| Tier | Path | Population | Cost |
|---|---|---|---|
| A (enhancement) | WebGPU — mosure webgpu backend, GPU compute radix-sort | modern iPhone/iPad, modern Android Chrome, desktop | client GPU; zero server |
| **B (primary)** | **WebGL2** — mosure wgl2 backend; CPU Web-Worker counting-sort at ~4 Hz on quantized 16-bit depth keys, instanced-quad raster; ≤ few-hundred-k splats | budget-device majority, Firefox-Android, old WebView | client CPU/GPU; zero server |
| C (floor) | Server **pre-rendered** panorama / fixed viewpoints + the same vector overlay | no-usable-GPU devices, hostile networks | self-hosted batch render → static CDN assets |

Runtime capability detection cascades A → B → C (`navigator.gpu` → WebGL2 probe →
static images). WebGL2 is the floor treated as primary — it runs on essentially
every GPU of the last decade; WebGPU's win is *scaling* (on-GPU sort past ~1M
splats), not *access*, and the frontage crop means the budget tier doesn't need
that scaling. Tier C is deliberately the **degenerate cheapest form** of
server-side rendering: content is per-address and static, so render once, offline,
on our own box — no live GPU per session, no interactivity latency budget, no SaaS
rendering API (which would violate self-hosted-first and is rejected).

**Dependency discipline (report 04 §5):** the renderer is the pre-declared opt-in
in `engine/Cargo.toml` — `default = []` unchanged (zero external crates),
`webgl`/`webgpu` features gate `wgpu`, `splat` gates `bytemuck`. Canonical CI job
never pulls wgpu; a separate non-blocking lane builds the featured WASM target.
The renderer consumes engine display output and per-address gcloud assets,
one-directional, display-only — it never feeds the kernel. This satisfies the "GPU
is a display surface; authoritative compute is CPU-side" mandate by construction.

**Primary dependency: mosure/bevy_gaussian_splatting** (Apache-2.0) — chosen over
the higher-fps web-splat (WebGPU-only) and brush (subgroup floor) precisely because
its dual wgl2+webgpu backend serves Tiers A and B from one asset format and one
render plugin. The decisive criterion was budget-tier reach, not peak fps.

### 2.6 UX geometry — pure geo.rs extension, splat-independent (report 05)

Six new functions extend `/root/dowiz/kernel/src/geo.rs` in its existing
deterministic, unit-tested, RED→GREEN style — small pure functions with no new
dependencies:

- `storey_height_m(height, levels)` — measured height/levels when both present,
  else generic 3 m storey; never fabricates a level *count*.
- `floor_slice_height_m(floor, storey_h)` — mid-storey (+0.5) slice plane so the
  cut passes through windows/doors.
- `arrow_screen_rotation_deg(facing, view_rotation)` — on a **north-up** slice this
  is the compass bearing unchanged (bearing_deg is already 0=N clockwise; screen
  rotation is clockwise-positive). v1 resists rotating the view to "straighten"
  facades: one missed θ-subtraction and the arrow lies. North-up = one global frame.
- `angular_diff_deg(a, b)` — seam-correct (350° vs 10° → 20°).
- `in_field_of_view(facing, target, fov)` — ±60° "in view" default, optional ±30°
  "direct" tier; parameterized.
- `los_clear(self, other, footprints)` — coarse 2D segment-vs-footprint-edge test.

Composition with existing primitives: `point_in_polygon` decides
building-branch vs open-space-branch; `bearing_deg(prev, cur)` smoothed with
`ema_next` gives a moving courier's facing; `bearing_deg(self, other)` feeds the
FOV cone. **Stated limits stay loud:** the LOS test is a 2D approximation ignoring
height entirely (false positives over low walls, false negatives across
courtyards) and reads empty footprint data as "all clear" — so it drives a **soft
advisory hint** ("courier facing your way"), never a hard visibility claim.

Crucially, this whole layer is **independent of the splat pipeline**: the same
arrow/slice/FOV overlay draws over a Tier-C static panorama, over a bare OSM
footprint, or over nothing at all. That independence is what makes the no-splat v1
shippable now (§4 P0).

---

## 3. Reranker and TimesFM — separate verdicts, no forced connection

Report 02's structural finding is preserved as-is: **TimesFM and Gaussian Splatting
share no mathematical surface** (causal attention over normalized numeric patches
vs covariance-weighted alpha-compositing over 3D primitives). The only true
adjacencies are (a) forecasting demand *at* an address — an ordinary time-series
task where "address" is just a label, and (b) using a forecast to schedule when a
splat job runs — a queue-boundary handoff, not a method bridge, and one that
report 06's plain delivery-volume threshold already covers better (the "plain
heuristic" report 02 itself predicted would win). The 4D/dynamic-GS literature's
"time" is a per-scene deformation field fit to multi-view video — not forecasting.
No published work drives Gaussians with a TS foundation model, and none should be
invented here. Two independent asks landed in one sentence; they stay independent.

**Reranker verdict.** The kernel already emits three orthogonal deterministic
relevance signals per candidate — BM25 (`bm25.rs`, deterministic tie-break), PPR
graph mass (`ppr.rs`, byte-identical power iteration), exact/trigram membership
(`index.rs`). The right-sized reranker is the **hand-tuned linear fusion** of those
plus recency and status/tier weight — fixed weights, per-signal normalization,
fixed summation order, ascending-id tie-break: `no_std`-compatible,
bit-reproducible, zero training corpus, exactly the "status-aware rerank" the
knowledge-spine blueprint already specifies. A trained cross-encoder/ColBERT/LLM
reranker is scope-creep: its advantage exists *because* of ~500k labelled MS-MARCO
judgments dowiz does not have; trained on nothing it would be strictly worse than
the fusion while adding a weight file, training loop, and permanent liability.
**Falsifiable stopping rule:** extend the retrieval oracle with queries where BM25
and graph proximity disagree (paraphrase/synonym-only); if fusion nDCG@10 is within
~2–3 points of an offline ms-marco-MiniLM used purely as a measuring stick, the ML
reranker is confirmed unnecessary. Reconsider only if that gap is large **and** a
genuine labelled corpus materializes — and then ColBERT-style late interaction
(offline-precomputable doc-token matrices) is the fit, never a per-query
cross-encoder pass.

**TimesFM verdict.** For per-order ETA, courier tracking, and any short-horizon
single series: **no** — `geo::eta_seconds` + `kalman.rs` (with `ema_next` as its
documented 1D steady-state special case) are the optimal linear estimator for this
Gaussian tracking problem, and TimesFM loses on latency, determinism, footprint,
and explainability simultaneously. The one profile where a foundation forecaster
could earn its adapter-boundary cost: **aggregate multi-merchant/region demand
forecasting** — hours-to-days horizon, many parallel series (zero-shot has real
value), rich multi-scale seasonality, batch/offline/non-safety-critical. Even
there, adoption is gated: out-of-kernel ops adapter only, and only after a
falsifiable backtest **beats a classical seasonal baseline (Holt-Winters/ETS, STL,
Theta)** on dowiz's own held-out demand history — classical methods are frequently
competitive on well-behaved seasonal business series at a fraction of the cost.
Per the DECART rule that comparison is mandatory before the dependency lands.
Until both gates pass, skip it.

---

## 4. Prioritized roadmap

### P0 — buildable now, low-risk, high-confidence

**P0.1 — geo.rs geometry extension + no-splat address-picker v1.**
Six functions (§2.6) + OSM/Overture footprint-and-levels ingestion with graceful
ground-floor degrade. Floor selection is explicit UI, bounded by `building:levels`.
Report 05's six-item falsifiable acceptance list applies verbatim:
1. Stage-1 pin-drop works everywhere, no external data dependency;
2. address with OSM footprint+levels → correct floor-selector range + north-up
   slice at `floor_slice_height_m`, verifiable against the raw tag;
3. address with no footprint → graceful open-space degrade, no crash, no
   fabricated floor;
4. arrow bearing on the north-up slice matches `bearing_deg` to <1°;
5. `in_field_of_view` correct across the 0/360 seam;
6. `los_clear` false across a known rectangular footprint, true routing around it.

**P0.2 — hand-tuned fusion reranker (knowledge-spine).**
Implement the fixed-weight fusion over BM25+PPR+exact+recency+status; extend the
29-query oracle with BM25-vs-graph-disagreement queries; record the
nDCG@10-vs-MiniLM measuring-stick gap as the standing falsifier.

**P0.3 — courier photo capture flow (consent + upload + geohash staging).**
The data bootstrap costs nothing GPU-wise and must start accreting before P1 has
anything to reconstruct. Stage into the content-addressed BlockStore keyed by
address geohash from day one so P1's dedup is free.

### P1 — real but conditional or larger

**P1.1 — SplatReconstructionJob port + Modal adapter (splat pipeline MVP).**
Port trait, Modal default backend, content-addressed result cache
(`sha3(geohash ⊕ imagery_set ⊕ params)`), mandatory-teardown watchdog, monthly
budget ceiling (degrade-closed), TokenBucket submission bulkhead, 7k-iter preview
default. Container: COLMAP → PyTorch+gsplat (brush bake-off deferred to P2).
*Acceptance:* one real address reconstructed end-to-end for ≤$2; artifact lands in
the BlockStore; re-submission of identical inputs is a $0 cache hit; a killed job
provably tears down its rental.

**P1.2 — tiered client renderer (engine opt-in features).**
mosure integration behind `webgl`/`webgpu`/`splat` cargo features; runtime
capability cascade `navigator.gpu` → WebGL2 probe → static fallback (report 04's
detection cascade, verbatim); CI golden-image shader tests on the software
rasterizer. *Acceptance:* default `cargo build/test` dependency graph byte-identical
to today; frontage scene renders interactively on a WebGL2-only budget-Android
device; WebGPU absence falls back silently with the same asset.

**P1.3 — Tier-C pre-render batch job.**
Same splat crate + wgpu on the self-hosted box, offline, emitting
panorama/fixed-viewpoint images to the CDN path; the P0.1 vector overlay draws on
top unchanged. This completes the A/B/C floor so nobody is locked out.

### P2 — deferred, correctly

- **brush (Rust-native trainer) bake-off** — same imagery, compare PSNR/time/$;
  swap the adapter backend only on a win; DECART report either way.
- **TimesFM aggregate demand forecasting** — gated on beating the classical
  seasonal baseline on backtest (§3); ops adapter only.
- **Hetzner GEX44 standing GPU** — gated on sustained volume that would saturate
  it; a backend swap behind the same trait, zero kernel change.
- **LOD machinery (CLoD-GS budget-based rendering, progressive streaming)** —
  a WebGPU-tier scaling nicety; the frontage crop makes it unnecessary for the
  budget tier that matters first.
- **Facade "straightening" (PCA/min-area-box view rotation)** — v2 nicety;
  `arrow_screen_rotation_deg` already takes `view_rotation_deg` so the seam exists.
- **ColBERT-style reranker** — gated on (labelled corpus exists) ∧ (fusion gap
  proven large).

---

## 5. Explicit rejections and deferrals

- **Full satellite/aerial reconstruction pipeline — rejected.** Three independent
  walls (legal ToS prohibitions on derivative datasets; nadir imagery is the wrong
  input shape for multi-view GS; upfront-global-coverage economics). Any one
  suffices; the legal one is absolute.
- **Apartment-level indoor identification — rejected from the roadmap.** Open
  global indoor floor-plan data at apartment resolution effectively does not exist.
  This is a data-existence problem, not a funding problem, and must not appear on a
  roadmap as if money unlocks it.
- **Persistent GPU rental as the starting point — rejected.** €184–889/mo standing
  idle vs ~$0.10–1.67 per actual job; break-even needs hundreds-to-thousands of
  reconstructions/month at saturation, which one-time-per-address-cached-forever
  will not produce. Future adapter swap, nothing more.
- **CuPy as trainer, preprocessor-by-default, or kernel dependency — rejected.**
  Kernel: Tier 3, no hook, unchanged. Pipeline: not the trainer (no autodiff, no
  differentiable rasterizer — the falsifier "does any mainstream 3DGS trainer use
  CuPy as its core" comes back negative); at most optional preprocessing glue in a
  Python pipeline we are not building for that tier.
- **Trained ML reranker (cross-encoder / ColBERT / LLM-listwise) now — rejected.**
  No labelled corpus; fusion is right-sized; LLM-listwise is additionally
  non-deterministic — the exact opposite of a bit-reproducible kernel primitive.
- **TimesFM for per-order ETA — rejected.** Kalman/EMA are already the optimal
  estimator for the actual problem; a 200M-parameter transformer is worse on every
  axis that matters on the hot path.
- **A manufactured TimesFM↔Gaussian-Splatting bridge — rejected.** No shared math
  surface; the only adjacency is a queue boundary, already better served by the
  delivery-volume heuristic.
- **Software GPU emulation for training — rejected** (2–3+ orders of magnitude too
  slow; kept strictly as a CI shader-correctness and smoke-test tool).
- **SaaS rendering APIs for Tier C — rejected** (violates no-cloud-by-default /
  self-hosted-first; pre-render on our own box needs no always-on GPU at all).
- **graphdeco-inria reference implementation as a shipped dependency — remains
  permanently rejected** (non-commercial licence); Apache-2.0 alternatives
  (mosure, web-splat, brush, gsplat) cover every role.

---

## 6. Budget-device framing check — honest audit against the operator's lens

The operator's stated lens for this entire effort was **бюджетність та бюджетні
пристрої**. Which recommendations genuinely serve it, and which are really about
something else:

**Genuinely budget-driven (the lens did real work here):**
- **WebGL2 as the primary tier, not WebGPU** — the single most budget-consequential
  decision in the set. caniuse's 84.7% WebGPU figure is skewed by wealthy-market
  device refresh; the missing ~15% is disproportionately exactly the old-WebView /
  mid-range-Android population this product serves. Treating WebGL2 as primary
  (and picking mosure *because of* its dual backend, over faster WebGPU-only
  alternatives) is the lens overriding a raw-performance ranking.
- **Per-job GPU rental with mandatory teardown** — ~$0.06–1.67 per reconstruction,
  $0 idle, hard monthly ceiling, preview-tier default. Pure cost discipline.
  (Honest caveat: Modal at ~$1.25–1.67/job is 15–25× Vast's marketplace price per
  job; at low volume the absolute spend is trivial and the saved ops dominate, and
  the port makes the cheaper adapter a later swap — but "default = Modal" is an
  ops-simplicity choice, not the cheapest-possible one.)
- **Courier-photo bootstrap** — near-zero marginal data cost, coverage follows
  demand, no licence fees. Budget-aligned *and* correctness-aligned at once.
- **Frontage crop + 31×-class compression → single-digit-MB assets** — serves
  budget devices twice: less to sort/rasterize on weak silicon, and less to
  download on metered/hostile connections.
- **Tier-C pre-render floor** — the explicit guarantee that a device with no usable
  GPU at all still gets the feature as a swipeable panorama with the same overlay.
- **Content-addressed cache-once-serve-forever** — amortizes the one real GPU cost
  to ~$0 across all subsequent deliveries to an address.

**Not budget findings — do not launder them as budget wins:**
- **The satellite rejection is first a *legal* finding** (ToS prohibitions on
  derivative datasets) and second a *technical-shape* finding (nadir ≠ multi-view).
  The economics wall is real but third; even with unlimited budget the legal wall
  stands. Saying "we rejected satellite because budget" would be false.
- **The graphdeco licence rejection is purely legal.**
- **The reranker and TimesFM verdicts are right-sizing/determinism findings** —
  they follow from the kernel's discipline and the absence of training data, not
  from device cost. They *incidentally* avoid GPU-inference spend, which is a
  pleasant side effect, not the reason.
- **The kernel-side CuPy "no hook" is a break-even/architecture finding** (N≤32,
  8 KB stack) that would hold on any budget.
- **Floor-is-user-selected is an honesty finding** (2D pins carry no vertical
  information), not a cost saving.

**One place the lens genuinely constrains future ambition, stated plainly:** the
WebGPU tier's scaling headroom (million-splat scenes, GPU radix sort, LOD
streaming) is real and cheap to keep as progressive enhancement — but any future
feature that *requires* Tier A to be useful (rather than merely prettier) would
break the budget mandate. The A/B/C cascade is only honest while Tier B remains
fully functional, not a degraded apology. That is the standing test every future
splat-adjacent proposal must pass.

---

*Inputs: scratchpad splat-research/01–06 (Opus, 2026-07-16). Prior art:
tech-synthesis-2026-07-15 (PLAN/RESEARCH-CONSPECT/BLUEPRINTS — TS-05, CuPy),
SYSTEMS-GPU-ML-KERNEL-SYNTHESIS-2026-07-16 (Trait-as-Port, content-addressing).
Kernel ground truth: kernel/src/{geo,kalman,bm25,ppr,attention,backup}.rs,
engine/Cargo.toml feature discipline.*
