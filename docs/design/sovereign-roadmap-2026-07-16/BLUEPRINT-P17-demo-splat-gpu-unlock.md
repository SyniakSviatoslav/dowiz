# BLUEPRINT P17 — DEMO, SPLAT TIERS & GPU-UNLOCK CLOSURE (2026-07-16)

> **Phase 17 of 19** (R2-MERGED-PHASE-ROADMAP §2). Anchors: **E4** (wasm-demo→video-after-GPU),
> **E45** (product physics/math wasm demo), **F14** (feature-gated webgl topology viz),
> **F47** (demo = wasm physics/math render of a delivery).
> Depends on: **Phase 11** (compute budget/cache — including the empty `webgl`/`webgpu`/`splat`
> feature scaffolds) and **Phase 16** (product UI rebuild — the demo scenario runs inside the
> rebuilt UI). Parallel-safe with Phases 18, 19.
> **Split phase.** The PRE-unlock half is buildable now, no trigger. The POST-unlock half is gated
> on an external **GPU-unlock trigger** (§3). `P17 ← P11, P16 [+GPU-unlock, operator]` (R2 §3, O18).
>
> Planning document only — **no code is written or edited by this blueprint.** The splat-tier work
> ports `GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md` (henceforth **GS**) §4 P1.1/P1.2/
> P1.3 and §5 **verbatim**; nothing there is re-derived. E4's LOCK — *demo = wasm now, video AFTER
> GPU-unlock* — is honoured absolutely: this phase does **not** fake or work around the video block.

---

## 1. Current-state evidence (file:line)

**The demo half of E4/F47 is genuinely BUILT — but it renders math, not a delivery scene.**
`web/src/app.mjs:1-33` boots the kernel wasm and prints ρ (`spectral_radius_js`), the FSM graph
report (`fsm_graph_report_js`), and a route snap (`geo_progress_flat_js`) — kernel math only, no
JS re-implementation (AGENTS invariant asserted in the file header). `web/src/lib/kernel/
kernel.test.mjs` holds the 20 green VbM assertions (W17). This is a *numeric* demo. There is **no
delivery scenario**: no courier moving, no order ticking, no field around it.

**`compose` exists and is deterministic; the residual gap is scenario + browser reach.**
`engine/src/field_frame.rs:189` `compose(scene, eq, w, h, steps) -> Vec<u8>` is "the single call a
future wgpu blit would consume", proven bit-identical by `compose_returns_deterministic_frame`
(`:302`), and already exposed through the `dowiz-wasm` crate: `compose_field` (`wasm/src/lib.rs:57`)
and stateful `FieldSim::new/step/frame` (`:78`), each with a GREEN host determinism test. The honest
ceiling at `wasm/src/lib.rs:172` — "verified by wasm-bindgen-cli + a browser smoke (`canvas.
putImageData`) once CI has a display" — means the physics *can* reach an RGBA8 buffer today; missing
is only (a) a scripted scenario to compose and (b) a browser blit smoke, **not** a new compute path.

**The courier-marker bridge is wired but unscripted.** `engine/src/bridge.rs:556` `CourierMarker`
ingests `dowiz-kernel::geo::progress_along_route` output decoded from the flat payload
`[remaining_m, snapped_lat, snapped_lng, segment_index]` (`ROUTE_PROGRESS_SLOTS = 4`), fail-closed on
malformed frames. Nothing drives it along a route; the FSM entry `apply_event_js`
(`kernel/src/wasm.rs:288`) exists but no tick script advances it.

**Video is CORRECTLY blocked — this is a LOCK, not an oversight.** `engine/Cargo.toml` declares
`default = []` and `gpu = []` (empty by design); the honest adapter at `engine/src/bridge.rs:200-243`
(`pub mod gpu`, `#[cfg(feature = "gpu")]`) has `new_gpu(...) -> Result<VertexBridge, &str>` returning
`Err("gpu adapter not built — wgpu uncached")`. ARCHITECTURE §8: "GPU: wgpu offline-ceiling (W21);
GPU-unlock pending network." Faking a video earlier than the trigger violates E4/F47's LOCK
("Demo = wasm physics/math render … FUT: GPU after unlock. PRO: honest. CON: no video yet. LOCK").

**F14 topology viz — absent.** Grep for a topology renderer returns zero. Canon F14: "Hub uses
webgl render of its own topology. SIT: possible (E23 feature-gated) … LOCK + feature-gated." The
`webgl`/`webgpu`/`splat` cargo features do **not exist yet** in `engine/Cargo.toml` (only `default`
and `gpu`); they land as **empty scaffolds in Phase 11** (P11 target: "webgl/webgpu empty features").

**Splat P1 — nothing built, fully speced.** `SplatReconstructionJob` (GS §2.3), the tiered client
renderer (GS §2.5), and the Tier-C pre-render (GS §2.5 tier C) are unbuilt. Their upstream
dependency — the courier photo capture flow (GS P0.3) that bootstraps imagery — is a **Phase 16**
courier-page deliverable ("one capture flow feeding PoD AND the splat bootstrap", R1-D F42), so by
the time P17 runs, imagery is already accreting into the content-addressed BlockStore
(`kernel/src/backup.rs`, sha3 + Buzhash-CDC) keyed by address geohash.

---

## 2. PRE-unlock scripted-demo design (buildable now, no trigger)

Goal: a **scripted, bit-deterministic** wasm delivery demo — a courier icon moving along a
Phase-4-computed route, the order FSM ticking through its real states, the physics field rendering
around it — running offline in **node** and **in-browser** with **IDENTICAL frame output both
times**. This is determinism, not "looks right."

**The scenario is data, versioned and frozen.** A single fixed file, `web/demo/scenario/
delivery-01.json` (also mirrored as an engine test fixture so the determinism test is in-tree),
declares everything the run consumes: the road-graph (`nodes: [(lat,lng)]`, `ways: [(u,v,cost)]`),
`src`/`dst`, a fixed **arc-length schedule** (an integer metre-increment per tick, N ticks), an
ordered **FSM event list keyed to tick indices** (`Placed@0 → Confirmed@k1 → InDelivery@k2 →
Delivered@N`), the field-source `scene` parameters (circles), and the frozen render constants
(`w`, `h`, `dt`, `steps`). No wall-clock, no RNG, no ambient input enters the run.

**Per-tick frame construction — every dynamic pixel is composited in wasm.** For tick `t`:

1. **Route** — `router::route` (Phase 4) computes the polyline **once** at t=0; the courier's
   position is `geo::progress_along_route` fed the *fixed* cumulative arc-length `s(t)` from the
   schedule, emitted as the flat `geo_progress_flat_js` payload and decoded by `CourierMarker`
   (`bridge.rs:556`). Deterministic because `s(t)` is a fixed integer sequence and the kernel geo
   math is bit-reproducible.
2. **FSM** — `apply_event_js` (`wasm.rs:288`) applies the scenario's event at tick `t` (or none),
   yielding the state + golden signature string. The order machine is deterministic (golden-signature
   drift gate, `order_machine.rs`).
3. **Field** — `FieldSim::step` / `compose_field` (`wasm/src/lib.rs`) evolves the field with a
   **moving source at the courier's snapped position** (the marker is injected as a `scene` circle),
   returning an RGBA8 buffer.

**The load-bearing determinism decision: the frame is composited entirely in wasm; Canvas is a dumb
blit.** The courier icon and any status glyph are drawn **into the RGBA8 buffer by the engine's own
integer rasterizer** (a `scene` element at the snapped pixel), *not* by Canvas-2D `fillRect`/`arc`
(which would introduce per-browser anti-aliasing and break byte-identity). The browser path is then
exactly `ctx.putImageData(new ImageData(frame, w, h), 0, 0)` — a pure, lossless blit of bytes the
wasm produced. Node produces the same bytes because it runs the **same wasm module** over the same
scenario. This is what makes node ≡ browser byte-identity *provable*, not aspirational.

**The frame stream that gets diffed.** Each tick contributes an ordered record:
`(fsm_state_string, [remaining_m, snapped_lat, snapped_lng, segment_index], sha3(rgba_frame))`.
The full-run artifact is the concatenation across all N ticks (and, for the acceptance test, the raw
RGBA frames too). Determinism proof = run the scenario **twice** and byte-diff the stream (`==`);
node-vs-browser proof = diff the node stream against the browser stream. Both must be empty.

**Two harnesses, one scenario, one wasm.** A node runner (`web/demo/run-node.mjs`, reading the
kernel/engine wasm from disk exactly as `app.mjs:9-13` does) and a browser runner (`web/demo/index.
html` + module, served like the existing shell) both import the **same** scenario data and the
**same** compiled wasm bytes. The runner is JS glue (display + scheduling, *not* math) plus a data
file; it adds **no crate** and touches **no feature flag**. The default `cargo build`/`cargo test`
dependency graph therefore stays byte-identical to today throughout this half (`gpu` and the Phase-11
`webgl`/`webgpu`/`splat` all stay empty). No video is produced here; `bridge.rs::gpu::new_gpu` still
returns its honest `Err`.

---

## 3. The GPU-unlock trigger (precise fire-condition for the POST-unlock half)

Per R2 §4 O18 and ARCHITECTURE §8, the trigger is **network access that allows `cargo add wgpu` to
succeed, plus operator go**. Made falsifiable:

- **Necessary condition (environmental):** `cargo add wgpu` (and, for the splat tiers, `bytemuck`
  and `mosure/bevy_gaussian_splatting`) resolves over the network and the resulting crates + their
  transitive deps **vendor into the offline cargo cache and commit into a `Cargo.lock`** that builds
  air-gapped afterward. The operational check is `cargo build --features gpu` (and `--features
  webgl,webgpu,splat`) succeeding against the vendored cache — not merely a one-time online build.
  Until this holds, `wgpu` is absent from every `Cargo.lock` (verified 2026-07-16, `bridge.rs:203`)
  and the honest `Err` stub is the *correct* state.
- **Necessary condition (governance):** each new crate (`wgpu`, `bytemuck`, `mosure/bevy_gaussian_
  splatting`) is a new dependency at a display surface → a **DECART report** is required per the
  rust-native rule and Phase 1's DECART-new-dep lint. No silent `cargo add`.
- **Necessary condition (operator):** O18 is an explicit operator/external trigger. The half does
  not self-start; the operator confirms the unlock (mirrors the O17 public-flip discipline: never
  autonomous for one-way/irreversible ceilings).

The trigger gates the **POST-unlock half only** (§4-§6). The PRE-unlock half (§2) needs none of it.
The same network unlock simultaneously makes the Modal control-plane reachable for the splat job
(§6 P1.1) and `wgpu` vendorable for the renderers (§6 P1.2/P1.3) and the video capture (§4).

---

## 4. POST-unlock wgpu-adapter + video-capture design

**wgpu adapter — flip the existing honest stub, don't add a path.** The `gpu` feature and its
boundary already exist (`engine/Cargo.toml` `gpu = []`; `bridge.rs:200-243`). Post-unlock work is to
(a) make `gpu` pull the vendored `wgpu` (`gpu = ["dep:wgpu"]`), (b) implement `GpuUploadSink` for a
real `wgpu::Queue` (the trait at `bridge.rs:35` already defines `write_buffer(offset, data)`), and
(c) flip `new_gpu` from `Err("gpu adapter not built — wgpu uncached")` to a real `wgpu::Device`/
`Queue` construction. The `HeadlessSink` mock (`bridge.rs:57`) stays as the DEFAULT-build path so the
offline "1 writeBuffer / 0 json" gate remains falsifiable without a GPU. **CPU compute is untouched**:
the bridge contract is "GPU is a display surface; authoritative compute is CPU-side (scalar == SIMD
bit-identical)" (`engine/Cargo.toml:6-7`). The GPU only *uploads and displays* the RGBA frames the
CPU `compose` already produced.

**Video capture of the SAME scenario — continuity is the whole point.** The video captures the
**identical `delivery-01` scenario** used pre-unlock, not a fresh one; "demo → video" is a literal
continuation. Preferred route: **native headless** on the GPU box — run the frozen scenario, upload
each pre-computed RGBA frame to an offscreen `wgpu` texture, present/read-back, pipe the sequence to
an encoder (ffmpeg over exported PNG/RGBA, or a native encoder). A browser route (WebGPU/WebGL2 +
`MediaRecorder`) is an acceptable secondary artifact; native is deterministic and display-independent.

**Continuity is falsifiable, not asserted.** The video's **pre-encode** RGBA frame sequence must be
**bit-identical** to the pre-unlock demo's frame stream (§2) — same scenario, same wasm `compose`
bytes, `wgpu` merely uploading them; `assert_eq!` pre-encode frame N against demo frame N. The lossy
encoder step is the *only* difference the artifact may carry. The MP4/WebM exists **only after** the
trigger — never before.

---

## 5. F14 webgl-topology-viz design (feature-gated, never in the default path)

F14 is a display-only WebGL2 render of the hub's **own** mesh topology — nodes = peer hubs, edges =
live links — sourced from Phase 9's peer graph and coloured by Phase 4's kernel graph-math
(`dsu::components` for partition islands, `kruskal_mst` for the overlay spanning tree). It is
one-directional like the splat renderer: it **consumes** engine display output and **never feeds the
kernel**.

- **Layout is kernel-side and deterministic.** The 2D node placement is a **spectral embedding**
  (Laplacian eigenvectors via the existing kernel spectral organs), computed CPU-side; the WebGL2
  layer only rasterizes points/lines from those coordinates. This keeps F14 inside the "GPU is
  display, compute is CPU" mandate — no graph algorithm runs on the GPU.
- **Feature-gated behind `webgl`/`webgpu`** (the Phase-11 empty scaffolds), `default = []` unchanged.
  It reuses the same capability cascade as the splat renderer (§6): `navigator.gpu` → WebGL2 probe →
  a **static fallback** (an SVG/text adjacency + component dump) so a no-GPU hub still sees its
  topology.
- **F14 must be absent from every DEFAULT build** — the falsifiable done-test. A CI grep proves no
  `wgpu`/`webgl` symbol is reachable without the feature, and the default dependency graph stays
  byte-identical. F14 never leaks into the default path.

---

## 6. Splat-tier P1 design (GS §4 P1.1/P1.2/P1.3 — cited verbatim, not re-derived)

The mechanism lives in GS §2 (data = courier photos not satellite §2.1; trigger = delivery volume
§2.2; rented GPU behind a port §2.3; compression seam §2.4; tiered rendering §2.5; UX geometry as
geo.rs extension §2.6). This phase **executes** GS §4's P1 items to their **verbatim acceptance
criteria**. Reproduced below with GS as the authority.

**P1.1 — `SplatReconstructionJob` port + Modal adapter (GS §4 P1.1).** The port trait (GS §2.3):
`trait SplatReconstructionJob { fn submit(&self, req) -> JobId; fn poll(&self, id) -> JobStatus;
fn fetch_result(&self, id) -> SplatBlob; }` with `struct RentedGpuAdapter { backend: Backend }`,
`Backend = Modal | VastApi | RunPodApi | HetznerGex44(future)`. **Modal is the DEFAULT adapter**
(~$1.25–1.67/job on A100, per-second billing, **true scale-to-zero**, arbitrary Docker image,
near-zero ops). Content-addressed result cache keyed by **`sha3(geohash(address) ⊕ hash(imagery_set)
⊕ params)`** into the existing BlockStore (`backup.rs`) — "reconstruction runs exactly once per
(locale, imagery) tuple; re-submissions are $0 cache hits; imagery change → new hash → new job only
then" (GS §2.4). Controls (GS §2.2): **mandatory-teardown watchdog** (per-job wall-clock so a hung
optimizer cannot leave a meter running), **monthly GPU-spend ceiling that trips the port into
queue-only (degrade-closed)**, per-tenant **TokenBucket submission bulkhead** (reuses the transport
TokenBucket from Phase 11), **7k-iteration preview default** (~10 min, ~⅓ cost; 30k reserved for
high-value). Container (GS §2.3): **COLMAP** (SfM/pose) → **PyTorch + gsplat** (Apache-2.0); the
Rust-native **brush bake-off is deferred to P2**. The Rust port itself compiles offline (honest
`Err`/queue-only until the Modal control-plane is reachable), mirroring Phase 11's Modal stub.
*Acceptance (GS P1.1, verbatim):* "**one real address reconstructed end-to-end for ≤$2; artifact
lands in the BlockStore; re-submission of identical inputs is a $0 cache hit; a killed job provably
tears down its rental.**"

**P1.2 — tiered client renderer (GS §4 P1.2).** **`mosure/bevy_gaussian_splatting`** (Apache-2.0)
integrated behind the `webgl`/`webgpu`/`splat` cargo features — `webgl`/`webgpu` gate `wgpu`, `splat`
gates `bytemuck`, **`default = []` unchanged (zero external crates)** (GS §2.5). Runtime capability
cascade **`navigator.gpu` → WebGL2 probe → static fallback** (GS §2.5, verbatim). Tiers: **A
(WebGPU enhancement)** — GPU compute radix-sort, modern devices, zero server; **B (WebGL2 PRIMARY)**
— CPU Web-Worker counting-sort at ~4 Hz on quantized 16-bit depth keys, instanced-quad raster,
budget-device majority; **C (server pre-render floor)** — §P1.3. Asset: **mosure `gcloud`** (f16/f32),
**frontage-cropped** scene (tens of thousands of splats) + **Niedermayr quantization** (~31× compress)
→ **single-digit-MB CDN-cacheable per-address asset** (GS §2.4). CI: **golden-image shader tests on
the software rasterizer** (llvmpipe/SwiftShader/wgpu software adapter, PSNR-thresholded golden output;
GS §2.3). *Acceptance (GS P1.2, verbatim):* "**default `cargo build/test` dependency graph
byte-identical to today; frontage scene renders interactively on a WebGL2-only budget-Android device;
WebGPU absence falls back silently with the same asset.**"

**P1.3 — Tier-C pre-render batch job (GS §4 P1.3).** The **same splat crate + wgpu on the
self-hosted box, offline**, emitting panorama / fixed-viewpoint images to the CDN path; the P0.1
vector overlay (the six geo functions from Phase 4) draws on top **unchanged** (GS §2.6 — the overlay
is splat-independent). This is deliberately the **degenerate cheapest form** of server-side rendering
(content is per-address and static → render once, offline, on our own box; no live GPU per session,
no SaaS rendering API — rejected §7). It **completes the A/B/C floor so nobody is locked out** (GS
§2.5). *Acceptance:* Tier-C static assets exist for a reconstructed address; a no-usable-GPU device
gets the swipeable panorama with the same overlay.

**Standing budget-lens test (GS §6, carried as a gate):** the A/B/C cascade is honest **only while
Tier B remains fully functional, not a degraded apology.** Any future splat-adjacent feature that
*requires* Tier A to be useful (rather than merely prettier) breaks the budget mandate and is
rejected. Every proposal in this lane must pass that test.

---

## 7. Preserved rejections (carried forward verbatim from GS §5 — none resurrected)

This phase resurrects **nothing** on the GS reject list. Each stands as written:

1. **Full satellite/aerial reconstruction pipeline — rejected.** Three independent walls (legal ToS
   prohibitions on derivative datasets; nadir imagery is the wrong input shape for multi-view GS;
   upfront-global-coverage economics). **The legal wall is absolute** — even with unlimited budget.
2. **Apartment-level indoor identification — rejected from the roadmap.** A data-**existence**
   problem (no open global indoor floor-plan data at apartment resolution), not a funding problem;
   must not appear as if money unlocks it.
3. **Persistent / standing GPU rental as the starting point — rejected.** €184–889/mo idle vs
   ~$0.10–1.67 per actual job; a future adapter swap behind the trait, nothing more.
4. **CuPy as trainer, preprocessor-by-default, or kernel dependency — rejected.** Kernel: Tier 3,
   no hook, unchanged. Pipeline: not the trainer (no autodiff / differentiable rasterizer).
5. **Trained ML reranker (cross-encoder / ColBERT / LLM-listwise) now — rejected.** No labelled
   corpus; the hand-tuned fusion is right-sized; LLM-listwise is additionally non-deterministic.
6. **TimesFM for per-order ETA — rejected.** `geo::eta_seconds` + Kalman/EMA are already the optimal
   linear estimator for the actual problem.
7. **A manufactured TimesFM ↔ Gaussian-Splatting bridge — rejected.** No shared math surface; the
   only adjacency is a queue boundary already served by the delivery-volume heuristic.
8. **Software GPU emulation for training — rejected** (2–3+ orders of magnitude too slow); kept
   strictly as a CI shader-correctness and smoke-test tool.
9. **SaaS rendering APIs for Tier C — rejected** (violates self-hosted-first; pre-render on our own
   box needs no always-on GPU).
10. **graphdeco-inria reference implementation as a shipped dependency — permanently rejected**
    (non-commercial licence); Apache-2.0 alternatives (mosure, gsplat, brush, web-splat) cover every
    role.
11. **Faking video before the trigger — rejected (E4/F47 LOCK).** The W21 ceiling is deliberate; no
    workaround, no fabricated GPU-green path. The honest `Err` stub stands until §3 fires.

---

## 8. Acceptance criteria (numbered checklist, split pre/post-unlock)

### PRE-unlock (buildable now, no trigger)

1. **Node determinism.** The `delivery-01` scenario runs offline in node and, **run twice**, the
   frame streams byte-diff to **empty** (courier along a Phase-4-computed route + FSM ticking through
   its real states + field physics compose).
2. **Node ≡ browser.** The same scenario runs in-browser (`putImageData` pure blit) and its frame
   stream is **bit-identical** to the node stream — all dynamic pixels composited in wasm, zero
   Canvas-2D drawing of dynamic content.
3. **Kernel-only math.** A CI grep proves the demo re-implements **no** geo/FSM/field math in JS
   (route from `router::route`, position from `progress_along_route`, state from `apply_event_js`,
   pixels from `compose_field`/`FieldSim`).
4. **Zero-dep discipline held throughout.** Default `cargo build`/`cargo test` dependency graph is
   **byte-identical to today**; `gpu` and the Phase-11 `webgl`/`webgpu`/`splat` features stay
   **empty**; no new crate enters any default `Cargo.lock`.
5. **No premature video.** No video artifact is produced pre-unlock; `bridge.rs::gpu::new_gpu` still
   returns `Err("gpu adapter not built — wgpu uncached")`.

### POST-unlock (gated on the §3 trigger firing)

6. **wgpu adapter behind the existing `gpu` feature.** `new_gpu` builds a real `wgpu::Device`/`Queue`;
   `GpuUploadSink` is implemented for the real queue; the DEFAULT `HeadlessSink` path and the
   scalar==SIMD bit-identical CPU compute are unchanged (GPU is display-only).
7. **Video from the SAME scenario (continuity).** A real video artifact is rendered from the
   identical `delivery-01` scenario; its **pre-encode** RGBA frame sequence is **bit-identical** to
   the pre-unlock demo stream (`assert_eq!` frame-by-frame).
8. **F14 feature-gated correctly.** The webgl topology viz builds behind `webgl`/`webgpu` and is
   **absent from every DEFAULT build** — CI grep finds no `wgpu`/`webgl` symbol reachable in the
   default path; F14 never leaks in.
9. **Splat P1.1 (GS verbatim).** One real address reconstructed end-to-end for **≤$2**; artifact
   lands in the BlockStore; re-submission of identical inputs is a **$0 cache hit** (content-addressed
   `sha3(geohash ⊕ imagery_set ⊕ params)`); a killed job **provably tears down its rental**.
10. **Splat P1.2 (GS verbatim).** Default `cargo build/test` dependency graph byte-identical to today;
    frontage scene renders interactively on a **WebGL2-only budget-Android** device; WebGPU absence
    falls back **silently with the same asset** (`navigator.gpu` → WebGL2 → static cascade).
11. **Splat P1.3.** Tier-C pre-rendered static assets exist for the reconstructed address; a
    no-usable-GPU device receives the swipeable panorama with the same P0.1 overlay drawn on top.
12. **Rejections intact.** A grep proves **no** CuPy, trained reranker, TimesFM-for-ETA, satellite/
    aerial reconstruction, or standing/always-on GPU rental crept into this phase's build (§7).
13. **DECART on every new crate.** `wgpu`, `bytemuck`, and `mosure/bevy_gaussian_splatting` each
    carry a written DECART report; the budget-lens test (§6, GS §6) is recorded as a standing gate.

---

## 9. What this closes and what it does not

P17 closes E4/E45/F14/F47: the demo becomes a *delivery* not a numeric readout, the video-after-GPU
narrative gets its literal continuation, F14 lands correctly feature-gated, and the GS splat P1 tier
ships to its own acceptance lists — every GS §5 rejection and the E4/F47 LOCK standing. It advances no
earlier-phase gap: router/geo/`compose` are Phase 4's (consumed, not rebuilt), the UI shell is Phase
16's, the compute cache and the empty `webgl`/`webgpu`/`splat` scaffolds are Phase 11's, and the
courier photo bootstrap is Phase 16's courier surface. P17 sits at the tail of the critical path
(`P3 → P9 → P10 → P13 → {P14, P16} → P17`) and is parallel-safe with 18/19. Its PRE-unlock half adds
no dependency; its POST-unlock half is the **single place in the whole roadmap** where `wgpu` and the
splat crates legitimately enter the tree — and only after the §3 trigger and their DECART reports.
