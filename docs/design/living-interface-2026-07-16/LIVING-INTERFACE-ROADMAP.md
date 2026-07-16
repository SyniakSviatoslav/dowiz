# LIVING-INTERFACE ROADMAP — one phased sequence for the wgpu living interface, 2026-07-16

> **Scope:** the *living-interface* arc — the GPU-rendered (`wgpu`/WebGPU) wave/neural-field UI,
> extended this session with (a) a real-time 3-D living-memory visualization, (b) sonification as a
> third renderer of the one field, (c) a GPU-less dev/CI strategy, and (d) a vendor brand pipeline
> ported to the GPU. It sequences the FOUR new research passes against the already-blueprinted engine
> work into a single dependency-ordered phase order.
>
> **Single source of truth for the four NEW designs:**
> - `R-LM-living-memory-visualization-architecture.md` (3-tier viz, one-Laplacian pipeline, dual
>   epoch-versioned streams, Phase-0 = HUB tier over the 20-node fixture)
> - `R-SON-sonification-architecture.md` (Rust/wasm DSP in an AudioWorklet, sound = 3rd renderer,
>   Phase-0 = order-lifecycle audio via `postMessage`)
> - `R-DEV-gpu-less-dev-ci-strategy.md` (Mesa Lavapipe software-Vulkan CI, prod needs no server GPU,
>   the COOP/COEP + CSP gap)
> - `R-VENDOR-brand-pipeline-wgpu-extension.md` (one canonical Rust `resolve(T1)`, zero token drift)
>
> **Single source of truth for the EXISTING engine designs this extends (NOT re-litigated here):**
> - `field-ui-engine/BLUEPRINTS-FIELD-UI.md` (FE-01..17)
> - `rust-engine-rewrite/BLUEPRINTS-RUST-ENGINE-REWRITE.md` (RW-01..12)
> - `dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md` (DZ-01..12)
> - `physics-ui-capture-blueprint.md` (the ONE-Laplacian-`L` unification thesis; roadmap U0..U8)
>
> This document is **planning only** — like the four research passes it merges, it wrote/edited no
> product code, CI config, or canon. It does **not** re-blueprint FE/RW/DZ items; it references them by
> ID and slots them into the sequence at the point their dependency graph requires.

---

## 1. Preamble — what this arc is, and what it is NOT

**What it is.** The living interface is the realization of the physics-ui-capture thesis (§0: "the UI is
not a DOM tree — it is a FIELD; one graph-Laplacian operator `L`, drawn on the GPU from equations") for
three concrete surfaces: the customer/owner order UI (Sea & Sheet), a diagnostic 3-D visualization of a
hub's own living-memory substrate, and a sonification layer that voices the same field. All three are
consumers of the same operator and the same event stream — the strongest form of "the interface is a
literal rendering of the backend."

**What it does NOT cover / does NOT re-litigate.** The already-decided *content* of FE-01..17,
RW-01..12, and DZ-01..12 is fixed. This roadmap does not re-argue the hybrid-vs-pure-GPU question (the
DOM→AccessKit supersession is owned by physics-ui-capture §4 / FE-15 / DZ-11), the token 3-tier model
(locked by DZ-02 / RESEARCH-CONSPECT), the money-never-tween red-line (FE-09 / FE-17 / `money_guard.rs`),
the wgpu-sole-graphics-dep ruling, or the Sea-vs-Sheet assignment rule. Those are prerequisites cited by
ID, not open questions. This roadmap owns exactly one thing the four component blueprints could not: **the
order in which their pieces + the four new designs must land, and the joints between them where friction
lives.**

**Prerequisite map (which existing item gates which new work), stated up front:**

| New design | Hard prerequisites (existing) | Why |
|---|---|---|
| **R-VENDOR** (brand→GPU) | **FE-05** (design-token GPU table), FE-06 (MSDF fonts), FE-09/`money_guard`, DZ-02 (token 3 tiers) | R-VENDOR P0-3 writes the resolved table into FE-05's Bind0 `theme_tokens` UBO. But R-VENDOR **P0-1/P0-2 (the `resolve()` crate + versioned artifacts) have NO GPU dependency and MUST land before FE-05's GPU token work** — see §4 sequencing rule. |
| **R-LM** (living-memory viz) | **FE-12** (spectral structure layout — the Laplacian eigenvector→coords helper is the ONE net-new kernel primitive R-LM needs), **RW-01** (`field-math` vendoring of the bebop2 eigensolver), RW-04 (ParticlePool, extended to `pos_z`), RW-05 (zero-copy shell), FE-01/02/03 (compute foundation), FE-08 (ζ=1 easing), FE-14 (lazy-render), FE-16 (WebGL2 fallback), proto-cap capability (F-4 access) | Positions come from `L=D−A`'s low eigenvectors (φ₂,φ₃,φ₄→x,y,z), which is exactly FE-12 over RW-01's vendored `field.rs` eigensolver. `spectral.rs` today computes eigen*values only* — the eigen*vector*/coords helper is the single net-new kernel primitive. |
| **R-SON** (sonification) | **RW-05** (`on_event` zero-JSON shell), **RW-09** (thin-shell membrane — **requires amendment, §3**), **RW-01** (→ a THIRD wasm artifact), DZ-04 (OrderStatus→Море), DZ-05 (Green's feedback vocab), DZ-06 (`fold_transitions`/`order_machine` for causal ordering), FE-09/`money_guard` | Audio is the 3rd renderer of the one field's forcing `S(t)`; it rides RW-05's existing `on_event(kind,count)` impulse stream and DZ-04/05's event vocab — no new event bus. |
| **R-DEV** (dev/CI + deploy) | FE-01 (`VertexBridge`/real wgpu wiring it unblocks), BLUEPRINT-W20/W21 (the offline ceiling it corrects) | R-DEV's one-time `cargo add wgpu` is the *actual* W21 blocker (uncached crate, not "software-raster impossible"). Its Lavapipe CI is the correctness harness for every WGSL-touching phase. |

---

## 2. The cross-cutting deployment-fix decision — COOP/COEP + CSP `'wasm-unsafe-eval'`

**Both R-DEV (§5.3) and R-SON (§5a) found the same production-header gap independently.** dowiz's
production surface today (the `native-spa-server` axum binary / `docker/nginx-default.conf`) sends **no
`Cross-Origin-Opener-Policy`, no `Cross-Origin-Embedder-Policy`, and a `script-src 'self'` with no
`'wasm-unsafe-eval'`.** The task asks: does fixing this belong early or deferred? **The honest answer is
that the "gap" is actually two independent header changes with opposite verdicts — and the sharpest
decision is to split them, not treat them as one fix.**

### Decision A — CSP `'wasm-unsafe-eval'`: **EARLY (Phase 0). Mandatory, not optional.**

This is **not a living-interface feature at all — it is a latent production bug today.** R-DEV verified
(§5.3.2): Chromium gates `WebAssembly.compile`/`instantiate` behind `'wasm-unsafe-eval'` in `script-src`;
the existing kernel wasm loads only because the *dev* `serve.mjs` sends no CSP. **Behind the production
CSP, the current kernel wasm — and every wasm artifact this arc ships (wgpu-wasm engine, the R-SON audio
DSP, RW-05's shell) — would throw a CSP `CompileError`.** The fix is one string added to a header that is
already RED-test-locked (`tests/integration.rs`), so the golden updates in the same commit. It has no
downside for a `default-src 'self'` all-same-origin app. **Every wasm-shipping phase downstream depends on
it. It goes in the first phase regardless of everything else in this arc.**

### Decision B — COOP/COEP (`same-origin` + `require-corp`): **DEFERRED (Phase 10). Gated on a deliberate migration.**

COOP/COEP exist only to unlock `SharedArrayBuffer`, whose only consumer in this arc is the low-jitter
audio-sync path (R-SON §5a). **R-SON's Phase-0 explicitly designs around it** — it uses `port.postMessage`
(structured-clone) transport, which needs no cross-origin isolation and accepts ~one frame of extra
jitter (fine for musical/ambient events that are not sample-critical). And turning on `COEP:
require-corp` is **not free**: it forces every cross-origin subresource to send CORP/CORS or it breaks —
and dowiz loads **MapLibre tiles and R2 entrance photos cross-origin**, so enabling it prematurely could
break the map and photos until each origin is proxied or CORP-tagged, touching the 99.4th-percentile-churn
`spa-proxy.ts`. The better long-term audio-sync path (SAB + `Atomics`) is real, but it is a strict
*optimization* over a working `postMessage` baseline, and it carries a real regression cost. **Deferring it
loses nothing in Phase-0 through Phase-8 and avoids a header change with production blast radius.**

**Net:** the two headers are not one decision. `'wasm-unsafe-eval'` is a mandatory, near-free, latent-bug
fix that goes in Phase 0. COOP/COEP is a gated optimization that goes in Phase 10 behind a MapLibre/R2
CORP-proxy migration. Anyone who bundles them and asks "early or late?" is forced into a false binary; the
correct engineering call is early for one, deferred for the other.

---

## 3. Required amendment to an existing blueprint — the RW-09 third-wasm-artifact gap

R-SON (§1.3) found that **RW-09 under-specifies "Audio."** This is a correction to an existing blueprint,
not a new one, and it must be recorded as an amendment rather than silently built around. **Exact target:**

- **File:** `docs/design/rust-engine-rewrite/BLUEPRINTS-RUST-ENGINE-REWRITE.md`
- **Section RW-09** ("Thin-shell boundary (codify irreducible JS)"), **TARGET** paragraph (the browser-API
  shims list). RW-09 today enumerates 15 Web-API categories with "…WebSpeech/Vibration/**Audio**/
  NetworkInfo…" and treats each as a single "shim." **Amend to split "Audio" into two:** (a) the
  **host membrane** — `new AudioContext()`, `audioWorklet.addModule(url)`, `PannerNode`/`GainNode` graph
  wiring, user-gesture unlock — which stays in RW-09's boundary module next to Push/WebGL; and (b) a
  **new Rust `audio` DSP crate** (sibling of `field-math`, `#![no_std]`+alloc, wasm32-clean, zero-dep)
  that carries the synthesis math (Karplus-Strong, additive/FM, biquad, granular, scale-quantizer,
  coherence mixer).
- **Section RW-01** ("`dowiz-engine` Cargo workspace + field-math vendor"), **TARGET** line, which reads
  *"Два wasm artifacts (dowiz_kernel JSON + dowiz_engine numeric)."* **Amend to THREE:** add
  **`dowiz_audio`** as a third wasm artifact, and add an `audio` crate to the scaffold list. The reason it
  must be its own artifact (not a feature of `dowiz_engine`): the `AudioWorkletProcessor` runs in a
  **separate realm (`AudioWorkletGlobalScope`) with no DOM and no access to the main-thread wasm
  instance's memory** — it must load and instantiate its own wasm. RW-01's "two wasm artifacts" model does
  not cover this; sonification makes it three.

Until this amendment lands, R-SON's Phase-0 (roadmap Phase 7) has no home for its DSP crate. The amendment
is a doc edit with no code and is a precondition of Phase 7, not of the whole arc.

*(Secondary, non-blocking flag from R-SON §2.4: DZ-10 defines an `InputSource` trait but no `OutputSink`
sibling — the input/output abstractions are asymmetric. Sonification is the missing output side; it should
eventually be named alongside DZ-10 as a `FieldSink`/`AudioSink`. Recorded here so it is not lost; not a
Phase gate.)*

---

## 4. THE MASTER PHASE TABLE

**Sequencing rule that governs the whole table (task point 4):** the brand **token source of truth
(R-VENDOR's canonical `resolve()`) lands BEFORE any wgpu rendering that consumes brand tokens.** Building
FE-05's GPU token table against a token source that might later change shape is wasted work — so Phase 1
(the pure-Rust `resolve()` crate + versioned artifacts) precedes Phase 3 (FE-05 GPU token table wiring),
even though Phase 1 has no GPU dependency and could otherwise float anywhere.

Phases 0 and 1 are mutually independent and start immediately (Wave 0). The rest is a dependency chain.

| # | Phase | New work covered | Existing FE/RW/DZ slotted in | Depends on | Falsifiable done-test |
|---|---|---|---|---|---|
| **0** | **Dev/CI + Deploy Enablement** | R-DEV Phase-0 recs 1–4: one-time `cargo add wgpu` (the real W21 blocker); native **Mesa Lavapipe** software-Vulkan GPU-smoke CI; local visual loop on operator's own GPU device; **CSP `'wasm-unsafe-eval'` header edit** (Decision A, §2). Corrects W21's "software-raster impossible" non-goal. | Unblocks FE-01/U1 (real `wgpu::Device` wiring); consumes BLUEPRINT-W20/W21 offline ceiling | — | (1) `cargo build -p dowiz-engine --features gpu` **links wgpu** (RED today: uncached). (2) Lavapipe headless renders one offscreen field-frame → readback → **deterministic pixel-hash / SSIM-vs-golden**; `create_shader_module` compile-check over every `.wgsl`. (3) Loading the **production-header** build in Chrome throws **no** CSP wasm `CompileError` (RED before the header edit: kernel wasm blocked). Job gated on `engine/**`+`**/*.wgsl` paths, `timeout-minutes: 10`. |
| **1** | **Brand Token Source of Truth** | R-VENDOR **P0-1** (extract single canonical `resolve(T1)->ResolvedTokens` zero-dep Rust crate, native+wasm) + **P0-2** (serve GPU token table beside CSS under one `token_hash`; the `token_hash` drift tripwire). Absorbs `presets.json` into `resolve()`; constrains "auto-generate brand" to *propose the 5 T1 inputs only*. | Realizes the CPU-side pre-resolution FE-05 already specified; feeds DZ-02 | — | (1) Same T1 ⇒ **bit-identical** outputs native == wasm. (2) Re-deriving a preset's 5 inputs reproduces `presets.json` exactly. (3) `grep 'color-mix('` over generated T1/T2 CSS output = **0**. (4) `token_hash` cross-check: served CSS literals and the GPU table agree **bit-for-bit** or the bake fails. |
| **2** | **GPU Engine Foundation** | (Enables all downstream GPU work; no new-design item originates here) | **RW-01** (workspace + `field-math` vendor, U0 — **now amended to scaffold the `audio` crate**), **FE-01** wired to a real `wgpu::Device`/`Surface`/`Buffer`+one `writeBuffer` (U1), **FE-02** (SoA store), **FE-03** (fixed-timestep), **RW-05** (`shell` crate, <10 zero-JSON exports incl. `on_event`), **RW-10** (build toolchain + Astro island + ≤2MB budget) | 0 | (1) Vendored `field-math` re-runs bebop2 tests GREEN unchanged. (2) Frame-loop profile: **0 `JSON.parse`**, one `writeBuffer` from a `Float32Array` view. (3) `dowiz-engine` wasm ≤2MB gzip; Astro island mounts + degrades; kernel's existing tests still GREEN. |
| **3** | **Render Primitives + Brand-on-GPU** | R-VENDOR **P0-3** (wire the resolved table into FE-05's Bind0 `theme_tokens` UBO; theme swap = 1 uniform write) + **P0-5** (curate T1 fonts + pre-bake MSDF atlases). **Net-new selective bloom pass** (R-LM §4.2 — shared machinery, a screen-space `e^{−tL}`-family blur; also the surface R-VENDOR §5c must later gate). | **FE-04** (particle→wgpu, **blue-hardwire fix**, widen→RGBA), **RW-04** (ParticlePool port, **extended to `pos_z`** for R-LM), **FE-05** (SDF + design-token GPU table), **FE-06** (MSDF text) | 1, 2 | (1) Storefront card rendered vs CSS version: **pixel-diff < threshold**; theme switch = **1 uniform write**, not re-tessellation. (2) `delivered` renders **gold** not pink (RED: hardwired blue). (3) Every curated font crisp at 3 scales in sq/en/uk (FE-06 gate). (4) Bloom pass composites (threshold→multi-mip blur→tonemap) with a locked golden. |
| **4** | **Field Dynamics + Money/Brand Guards** | R-VENDOR **P0-4** (bloom-aware contrast gate on brand-save: rejects a brand that passes flat AA but fails post-bloom on-Sea contrast) + **P0-6** (live brand-update push → refetch table → SPREAD/heat-kernel swap, no reload). | **FE-07** (layout SMACOF), **FE-08** (motion ζ=1), **FE-09** (money-guard red-line 🔴), **FE-14** (lazy-render-on-settle), **FE-16** (WebGL2 + scalar fallbacks) | 3 | (1) ζ=1 monotone, **no overshoot** (RED: overshoot). (2) A brand passing flat AA but failing post-bloom on-Sea contrast is **rejected** (RED→GREEN). (3) Owner edits accent mid-session → live customer Sea migrates over ζ-motion, **money unchanged**, no reload. (4) Static screen → rAF goes dormant (0 wake-ups). |
| **5** | **Field Semantics + Spectral Embedding** | The **ONE net-new kernel primitive R-LM needs**: a Laplacian eigenvector→3-D-coords helper (`coords_2d`/`coords_3d` over the vendored eigensolver). | **FE-10** (feedback Green's), **FE-11** (focus wells), **FE-12** (spectral structure layout φ₂,φ₃,φ₄), **FE-13** (constraint solver) | 2, 4 | (1) Spectral layout **tangle-free** on a real graph (RED: FR local-minimum tangle); clusters separate along φ₂. (2) Same graph ⇒ **byte-identical** positions (fixed spectral seed), paralleling `diffusion.rs` `green_ppr_byte_identical_two_runs`. (3) One action = one field impulse (RED: per-component feedback code). |
| **6** | **Sea & Sheet Backbone + One-Field Event Stream** | (The `S(t)` forcing stream both viz and audio consume originates here — the J2 joint, §5) | **DZ-01** (Shell 3-act two-layer), **DZ-02** (token 3-tiers + `<Money>`, on the canonical `resolve()`), **DZ-03** (spectral edge + transitions), **DZ-04** (OrderStatus→Море), **DZ-05** (Green's feedback vocab), **DZ-06** (local-first event-log + `fold_transitions` replay) | 3, 4, 5 | (1) 3 acts = 3 URL states, working-back intact; Море under every screen; reduced-motion → static gradient, state legible. (2) Status advance → amplitude jump + terracotta→gold shift; illegal transition → red recoil (local `channel.js` validate, **no server call**). (3) Reload → `fold_transitions` replay reconstructs canonical state with no round-trip. |
| **7** | **Sonification Phase-0 (R-SON)** | R-SON full Phase-0: Rust `audio` DSP crate → **THIRD wasm artifact** + **ONE `AudioWorklet`**, `postMessage` transport (**no COEP**); sonify the DZ-04 order lifecycle from the same `S(t)`; **delete legacy `use-sound.ts`** (4 canned MP3s); `PannerNode` follows CourierTrack FieldPos; money-never-sonified guard 🔴. Scheduled off the kernel-validated monotonic sequence (§5, J2). | Consumes **DZ-04/05/06**, **RW-05** (`on_event`), **FE-09**/`money_guard`; **requires the RW-09+RW-01 amendment (§3)** | 6 (+ §3 amendment) + 0 (`wasm-unsafe-eval` for the 3rd artifact) | (1) **0** pre-recorded audio files loaded (`use-sound.ts` deleted; grep no `.mp3`/`new Audio(`). (2) Render worklet output offline → **FFT asserts** `delivered` resolves to a consonant interval (3rd/5th) and `rejected` to a muted dissonant cadence (♭2/damped). (3) Courier note stereo pan **tracks marker FieldPos.x**. (4) Under injected **200 ms WS jitter**, events schedule in kernel-validated **causal order** (error never precedes its attempt). (5) `AudioContext` never resumed ⇒ **every state legible** via visual/text channel. |
| **8** | **Living-Memory Viz Phase-0 — HUB tier (R-LM)** | R-LM full Phase-0: 3-tier data model in the *protocol* from day one (`tier`+`epoch`+`graph_id`), but **only the HUB builder ships**; current/local hub, read-only, **owner-scope fail-closed**; over the in-kernel **20-node/41-edge `retrieval/diffusion.rs` fixture**; server streams **`LayoutKeyframe`** (spectral positions via the Phase-5 helper + `graph_spectrum` + PPR) and **`ActivityDelta`** (pinned to epoch, dedup by `event_id`); client renders a 3-D SoA cluster, bloom, ONE signal type (`Recall` wave) + **one audio grain per activation** (via Phase-7's renderer). proto-cap access gate. | Consumes **FE-12** (positions), **RW-01** field-math (eigensolver), **RW-04**+`pos_z`, **RW-05**, **FE-08/14/16**, the Phase-3 bloom pass | 5, 7 | (1) Same graph ⇒ **byte-identical** positions. (2) Activity ⟺ kernel truth: the 4 unreachable nodes **`{5,6,12,16}` render DARK** (PPR score exactly 0, bound to `diffusion.rs` test). (3) Seeding a query lights the top-5 containing the relevant node (ties to `recall_at_k == 1.0`). (4) **grain count == lit-node count** (A/V sync). (5) No owner capability ⇒ **explicit stream reject (Err), not empty**. |
| **9** | **Product-Surface Migration + Hybrid a11y + Multimodal** | (No new-design item originates here; this is where the existing per-role product surfaces ride everything built above) | **DZ-07/08/09** (CLIENT/COURIER/OWNER full checklists), **RW-11** (view→wgpu-field per-island), **FE-15/DZ-11** (hybrid DOM a11y + input overlay), **RW-06/07/08** (port geo/cart/messenger), **RW-02/03/12 + FE-17** (delete JS dups, kill money-tweens 🔴), **DZ-10** (multimodal input), **DZ-12** (cross-platform WebGL2/native/AR) | 6, 7, 8 | (1) Every migrated island: full master-checklist feature preserved (RED: enumerate missing); money `<Money>` snap; degrade path works; SSR menu stays DOM. (2) Screen-reader reads the semantic mirror (RED: canvas invisible to AT). (3) `grep` money-bound `AnimatedNumber` = 0. |
| **10** | **Deferred Optimizations** | **COOP/COEP header migration** (Decision B, §2 — proxy/CORP-tag MapLibre + R2 first) → **`SharedArrayBuffer`** low-latency audio sync; **Tier-2 coherence `|ψ₁±ψ₂|²`** interference — visuals AND audio, gated on the SAME flag; **MESH + NODE viz tiers** (new server graph-builders, no wire-format change); **large-graph Lanczos** eigensolver (R-LM F-9); **RevocationSet** wiring (R-LM F-4 gap); **real-device CI** (R-DEV deferred: GPU-runner Playwright/xvfb, BrowserStack/LambdaTest) | (Extends FE-12 Lanczos path; adds MESH transport = SignedFrame; no FE/RW/DZ item is *first* built here) | 8, 9 (+ operator gate for COEP) | (1) `crossOriginIsolated === true` and MapLibre map + R2 photos still load (RED: COEP breaks cross-origin subresources). (2) SAB audio path: measured jitter < the `postMessage` baseline. (3) MESH tier renders N hubs; a **partition tints/splits** the cluster when streamed `fiedler λ₂(L)` crosses ~0 (no special partition protocol). (4) Revoked viz capability stops streaming within one session boundary. |

**Coverage accounting (zero new-design item unslotted):**
- **R-DEV:** Phase-0 recs 1–4 → Phase 0; deferred real-device automation → Phase 10. `wasm-unsafe-eval` → Phase 0; COOP/COEP → Phase 10 (Decision split, §2).
- **R-VENDOR:** P0-1/P0-2 → Phase 1; P0-3/P0-5 → Phase 3; P0-4/P0-6 → Phase 4; the living-memory-viz **T3 (unbranded)** verdict → constrains Phase 8 (viz marks read only T2/T3; ambient Sea tint inherited from the owner Shell, no special rule). All 6 P0 items + the tier verdict covered.
- **R-SON:** full Phase-0 → Phase 7; RW-09/RW-01 amendment → §3 (precondition of Phase 7); SAB/COEP + Tier-2 coherence audio + mesh/node audio tiers + FFT reverse mode → Phase 10.
- **R-LM:** full Phase-0 HUB tier → Phase 8; the one net-new kernel primitive → Phase 5; MESH/NODE tiers + Tier-2 coherence visuals + Lanczos + RevocationSet → Phase 10.

---

## 5. THE INTEGRATION RISK MAP — organized by component-pair joint

This is the single most valuable deliverable (operator directive: focus on "joints between components
where errors/friction will occur"). Every friction point named across all four research docs is
consolidated here, organized by **which two components meet at the joint** — not by which doc found it —
and **cross-referenced** where one doc's friction interacts with another's.

### J1 — Server layout-computer ↔ Client renderer (the wire)
- **R-LM F-1 (staleness, THE load-bearing one):** layout (spectral eigensolve, O(n³)–O(n⁴), slow) and
  activity (event feed + PPR, cheap, fast) ride wildly different cost curves; one stream at one cadence
  makes layout latency throttle activity, or a fast mutation outrun the layout the client draws.
  **Resolution:** dual epoch-versioned streams — slow `LayoutKeyframe` (epoch-stamped, `graph_id`
  content-addressed) + fast `ActivityDelta` pinned to an epoch; a delta whose epoch ≠ current layout is
  buffered, never applied to the wrong layout; **idempotent dedup by `event_id`** (the `event_log`
  content-id) makes reorders/dupes structural no-ops.
- **R-LM F-7 (presentation vs state divergence):** server = state, client = presentation; the client only
  eases old→new *streamed* positions via FE-08 ζ=1 critical damping (monotone, no overshoot) — it can
  never invent a position.
- **R-LM F-9 (eigensolver ceiling):** dense eigen-engine is O(n³)–O(n⁴); Phase-0 targets small graphs so
  the dense path suffices; beyond that needs the Lanczos/iterative low-eigenvector path (Phase 10).

### J2 — Ordered event stream ↔ its multiple renderers (video + audio) — ★ THE MOST DANGEROUS JOINT
**This connects THREE findings across two docs plus the substrate:** R-LM F-1 (staleness) + R-LM F-5
(A/V sync) ↔ R-SON §5a (two clocks) + R-SON §5b (causal inversion under jitter). Both the neuron-viz
and the sonification are **renderers of the SAME ordered event stream** — R-SON is explicit that "sound
is the 3rd renderer of the one field's `S(t)`," and R-LM drives audio from the *same* `ActivityDelta`
stream as the visuals.
- **The failure if the two subsystems bind to DIFFERENT ordering authorities:** a signal lights a neuron
  (video, main-thread rAF, ~16.6 ms clock) at one logical instant but sounds (audio, dedicated audio
  thread, 128-frame/~2.7 ms quanta) at another; under network jitter an "error" can arrive before its
  "attempt," and **audio makes causal violation more perceptible than visuals because sound is inherently
  sequential** (R-SON §5b) — the "boom precedes the flash" (R-LM F-5) at scale.
- **The unified resolution (must be architected once, up front — Phase 6/7/8):** BOTH renderers schedule
  off the **one kernel-validated monotonic sequence** keyed by the **same `event_log` content-id /
  `t_logical` / `actor_seq`**, not raw wire-arrival order. The authority already exists — DZ-06's local
  event-log + `fold_transitions` replay + the kernel `order_machine`'s illegal-transition guard — so an
  acausal pair *never schedules the second sound*, and a late predecessor is *folded, not replayed*. One
  small jitter/presentation-lookahead buffer (~80–150 ms) feeds both, so the visual pulse peak and the
  audio grain land on the same perceived instant.
- **⚠ CORRECTED 2026-07-16 (self-critique §7, item Q1-a/J2 — the sharpest confirmed finding).** The
  resolution above conflates TWO guarantees that do not both cover both renderers, and it papers over a
  **wire-shape** mismatch the "same ordered stream" phrasing hides:
  1. **The `order_machine` illegal-transition guard is order-path-only.** It validates `OrderStatus`
     transitions (`kernel/src/order_machine.rs:123` `assert_transition`, `:140` `fold_transitions`) — the
     Phase-7 sonification path. The living-memory viz signals (`Recall`/`Gossip`/`Decide`, R-LM §2.2) are
     **not** order transitions and never pass through `order_machine`. For the viz path the shared ordering
     authority is the **`event_log` layer only** — `actor_seq` (`event_log.rs:140`) + `event_id` (`:148`)
     dedup (`AppendOutcome::Duplicate`, `:222`) + `epoch` pinning. So "the authority already exists — … the
     kernel `order_machine`'s illegal-transition guard" is true for Phase 7 and **false for Phase 8**; only
     the `event_log` content-id ordering is common to both.
  2. **The two renderers do NOT consume the same payload shape.** R-LM emits `ActivityDelta.Signal {
     signal_type:u8, kind:u8, energy:f32, node:u32 }` where `node` is an **index into the epoch's
     `LayoutKeyframe.nodes[].pos:[f32;3]`** (position by reference). R-SON's Phase-0 renderer consumes
     `on_event(kind:u32, count:u32) + inline FieldPos` (R-SON §2.1) and explicitly **defers "living-memory
     viz signals" out of its Phase-0**. "Same ordered stream" is accurate for **WHEN** to fire (ordering)
     but false for **WHAT** to fire from (payload): `kind` widths differ (u8 vs u32), R-SON never reads
     `signal_type` or `energy`, and it has no `node → LayoutKeyframe.pos` epoch-join for `PannerNode`.
  **Consequence:** Phase 8's done-test #4 ("grain count == lit-node count … via Phase-7's renderer")
  requires an **adapter that Phase 7's own scope does not build** — a `Signal → audio-event` map
  (signal_type/kind → timbre, energy → amplitude) plus a `node → epoch layout position` resolve for pan.
  **This adapter is now an explicit Phase-8 deliverable** (not a free consequence of "both consume one
  field"). The *ordering* half of J2 stands; the *payload/impedance* half was missed and is added here.
- **Why it is the most dangerous:** it is a **latent architectural coupling** that only manifests at
  scale / under jitter (so it passes every small-fixture test), it spans two independently-built
  subsystems (R-SON's audio crate and R-LM's viz), and it is **cheap to design in but very expensive to
  retrofit** — retrofitting a shared ordering authority across two shipped renderers means re-plumbing
  both scheduling loops. The mitigation is a sequencing constraint on this roadmap: the ordering authority
  is fixed in Phase 6 (event stream) and *both* Phase 7 (audio) and Phase 8 (viz) consume it — neither is
  allowed to invent its own clock.

### J3 — AudioWorklet thread ↔ Main render thread
- **R-SON §5a (two clocks):** audio-hardware clock vs main-thread rAF; do not trigger sound from rAF —
  share `S(t)` into the worklet and let the audio thread schedule at sample accuracy against
  `currentTime`.
- **R-SON §1.3 / §5d (separate realm):** the `AudioWorkletGlobalScope` cannot share the main-thread
  engine's wasm memory → the DSP is a **separate wasm instance/artifact** (the third artifact, §3)
  receiving only the compact `S(t)` stream, **not** the full field. Keep the audio-side model small.
- **R-SON §5d (battery):** suspend `AudioContext` on `visibilitychange`/idle; autoplay is blocked until a
  user gesture (the DZ-01 arrive-tap is the natural unlock) — do not rely on audio on load.
- **Cross-ref to J6:** the *low-jitter* transport (`SharedArrayBuffer`+`Atomics`) across this thread
  boundary is gated on COOP/COEP (Phase 10); Phase-0 uses `postMessage`.

### J4 — CSS/DOM Sheet ↔ GPU/WGSL Sea (brand rendering)
- **R-VENDOR §5a (token drift, THE central brand friction):** two implementations of "derive T2 from T1"
  (browser `color-mix()` vs a shader/Rust mix) drift → DOM Sheet and GPU Sea show different colors for the
  same brand. **Resolution (drift-impossible by construction):** one canonical Rust `resolve()`; served
  CSS carries **resolved literals, never live `color-mix()`**; DOM and GPU read the same values under one
  `token_hash`; the `token_hash` tripwire fails the bake if they disagree.
- **R-VENDOR §5d (colour-space):** CSS is sRGB/oklch *display* space; GPU bloom/additive blend must be in
  **linear light** — `resolve()` emits the GPU table in linear RGBA once (a naive hex reuse ships a
  subtle brand-wide wrongness).
- **R-VENDOR §5c (bloom washes out contrast):** the flat WCAG check never sees the GPU's emissive bloom;
  an arbitrary brand can pass flat AA yet be inaccessible once bloomed → the bloom-aware contrast gate
  (Phase 4), scoped to on-Sea text only (the opaque Sheet is unaffected).
- **Cross-ref to J1 (convergent, risk-reducing):** **R-VENDOR §4 (viz = T3, unbranded) and R-LM F-3
  (STATE crosses the wire, STYLE stays client-local) reach the SAME verdict independently** — the
  living-memory viz streams brand-neutral state and its marks never read the 5 T1 tokens; only the ambient
  Sea it floats over inherits the owner Shell's tint. Two docs converging on one rule *lowers* the risk at
  this joint, but it is load-bearing: a MESH view of N hubs must render peers in the viewing hub's own
  neutral/spectral palette, never each peer's brand (F48 + privacy — injecting brand leaks which tenant is
  active).
- **⚠ CORRECTED 2026-07-16 (self-critique §7, item Q1-a/J4).** "Reach the SAME verdict independently" is
  **overstated** — the two docs converge on the *state* being brand-neutral but **diverge on the marks**.
  R-VENDOR §4 (line 261) rules the viz **T3, unbranded — "the visualization's marks never read the 5 T1
  tokens,"** nodes/edges/glow read only T2/T3 (contrast-controlled by dowiz). R-LM F-3 (line 400) says the
  opposite for one mark: **"the `--spectral` edge re-derived *per brand*."** A per-brand edge IS a branded
  mark, so this is a real contradiction the synthesis smoothed over by quoting only R-LM's brand-neutral-
  *state* half. **Ruling (adopting R-VENDOR, the substantive case): the `--spectral` edge is DOWIZ-fixed
  T2/T3, NOT re-derived per brand** — cross-tenant legibility + bloom-contrast control require the marks be
  brand-invariant; only the ambient Sea the cluster floats over carries the owner tint. R-LM's "re-derived
  per brand" language is superseded (R-VENDOR was written before R-LM existed and never saw it; R-LM never
  saw R-VENDOR's T3 ruling). This is a *resolved divergence*, not a risk-lowering convergence.
- **⚠ Related, flagged (self-critique §7, item Q1-a/J4-colour):** R-VENDOR §5d mandates the GPU token table
  be **linear-RGBA** (bloom must blend in linear light); R-LM's viz bloom (§4.2, HDR emissive >1.0) is
  exactly the case that rule protects but **R-LM never states the linear-space discipline**. Since Phase 8
  reuses the Phase-3 bloom pass and (per the ruling above) reads the T2/T3 palette, its palette **must be
  sourced from the same linear-RGBA `resolve()` output**, or it ships the "subtle brand-wide wrongness"
  R-VENDOR §5d warns of. Recorded as a Phase-8 constraint, not a new phase.

### J5 — CI software-rasterizer ↔ real client GPU
- **R-DEV §5.1a (looks correct in the emulator, breaks on a real GPU):** Lavapipe is non-conformant and
  CPU-only; real GPUs differ in f32 rounding, `fwidth`/derivative accuracy (load-bearing for FE-05's SDF
  analytic AA), filtering. **Resolution:** the determinism firewall (all authoritative math CPU-side, GPU
  = dumb display) shrinks the shader-logic surface to near-zero; golden-image **SSIM/tolerance** compare
  (never a bit-exact hash across rasterizers); stay in the WebGPU/WGSL core feature set; **one manual
  real-device pass per shader-touching change**.
- **R-DEV §5.2b (CI slow):** software raster is 1–2 orders slower and Lavapipe can hang the runner — scope
  CI to compile + a handful of small still frames, pin Mesa, per-test timeout.
- **Cross-ref to J4/J8 (the task's explicit cross-reference):** **a visual-correctness CI test needs a
  stable token source to render against.** R-DEV's Lavapipe golden asserts a pixel-hash/SSIM — but if the
  brand-token source isn't canonical, **every `resolve()`-logic change silently invalidates every golden**,
  and the golden is chasing a moving target. Worse, R-VENDOR's `token_hash` tripwire (DOM literals == GPU
  table) *is itself a CI check* that runs in the same harness R-DEV designs. **Consequence for sequencing:**
  R-VENDOR P0-1/P0-2 (Phase 1) must land before any Lavapipe golden that includes brand color is locked
  (Phase 3+), and goldens must be pinned to a `token_hash`. This is a second, independent reason the
  token-source phase precedes GPU rendering (§4 rule). Cross-ref also to J4 §5d: linear-vs-sRGB wrongness
  is exactly the class of bug that can *pass on Lavapipe and look wrong on a real GPU* — so the colour-space
  correctness item needs the real-device pass, not just CI.

### J6 — Production HTTP headers ↔ browser runtime (deployment) — the independently-corroborated joint
- **R-DEV §5.3c and R-SON §5a found this SAME gap independently.** Two sub-joints:
  (1) `script-src 'self'` (no `'wasm-unsafe-eval'`) **blocks WebAssembly instantiation in Chromium** — a
  latent bug **today** for the existing kernel wasm behind the prod CSP, and a hard blocker for the
  wgpu-wasm engine, the R-SON third artifact, and RW-05's shell. → **fixed early, Phase 0** (Decision A).
  (2) No COOP/COEP ⇒ `crossOriginIsolated === false` ⇒ **`SharedArrayBuffer` unavailable** (the low-jitter
  audio path). `COEP: require-corp` breaks cross-origin MapLibre tiles + R2 photos until proxied/CORP-tagged.
  → **deferred, Phase 10** (Decision B), design-around = `postMessage`.
- **Cross-ref to J3 + J1 + J4:** this joint gates the audio-thread transport (J3), and the `wasm-unsafe-eval`
  half gates *every* wasm-shipping phase — R-LM's wgpu-wasm (J1) and the whole GPU brand path (J4) both die
  behind the current CSP. It is the widest-blast-radius joint even though the fix is one header string.

### J7 — Server authz ↔ Client subscriber (access control) — the most security-relevant single finding
- **R-LM F-4:** the viz stream must be gated **server-side, fail-closed** behind a proto-cap
  `Capability{Resource::LivingMemoryView, scope}` (ML-DSA-signed, scope+expiry verified on subscribe and
  re-verified at session boundary); fail-closed = **explicit reject, not an empty stream**. NODE-tier
  drill-down must redact PII/money payloads server-side (the spine is PII-free — only `payload_hash` — but
  `retrieval/memory_store.rs` holds real content).
- **🔴 The real existing gap (R-LM flags, does not solve):** MESH-REAL-PLAN records *"revocation НЕ існує
  (лише expiry)"* — `RevocationSet` is specced (M12) but **not built**; a granted-then-revoked viz
  capability **keeps streaming until it expires.** Mitigation until it lands: issue viz capabilities
  short-lived + re-verify every session boundary; wire `RevocationSet` the moment it exists (Phase 10).
  This is the single most security-relevant finding in the arc.

### J8 — Brand token source ↔ all consumers (DOM · GPU · live-preview · presets · CI golden)
- **R-VENDOR §5a/§2.3/§5d:** one canonical `resolve()` is the *only* implementation, compiled native +
  wasm, so the branding editor's **live preview before save** runs the *same* code and is bit-identical to
  the eventual server bake (drift-free preview). **presets.json must be defined as 5 T1 inputs run through
  `resolve()`**, or presets become a third resolution path that drifts from both DOM and GPU. **"Auto-
  generate brand" must terminate in the 5 T1 inputs** (operator constraint: no AI theming) — an input-
  assist to the 5 tokens, never a per-component theming engine.
- **Cross-ref to J5:** same joint as the CI cross-reference above — the CI golden is *also* a consumer of
  this source and must pin to `token_hash`.

### J9 — Kernel graph mutation ↔ layout stability
- **R-LM F-2 (topology change mid-render):** the spectral embedding is global — adding one record
  perturbs all eigenvectors → naïve re-layout makes every neuron jump. **Resolution:** spectral for
  cold-start / large change; a single node add/remove pins existing nodes and runs one warm-started FE-07
  SMACOF relaxation (monotone, Lyapunov). The **drift gate already rejects `Unstable` (ρ>1) mutations
  pre-persist**, so the client never has to render a divergent graph. A MESH partition renders directly:
  streamed `fiedler λ₂(L) → 0 ⟺ disconnected`.
- **R-LM F-6 (LOD decimation flicker):** top-K-by-activity makes the visible set flicker at the K
  boundary → **hysteresis** (reuse FE-14 K=3, enter at θ_high, leave at θ_low); positions computed once on
  the full/coarsened graph and projected, so LOD level never moves a node.
- **R-LM F-8 (over-signing the local stream):** per-frame ML-DSA on the 60 fps HUB/NODE stream is
  infeasible and wrong-layer → sign only cross-hub MESH frames (low-frequency, opportunistic); HUB/NODE is
  intra-hub, trust established once at subscribe-time (J7).

### Density / scale frictions (single-component, recorded for completeness)
- **R-SON §5c (volume/density explosion at scale):** at real order volume there are more events than
  audible notes → the §3 orchestral-distance tier model + a hard voice budget (≤~12) + **overflow summed
  into the tier's ambient pad** (never dropped-silent), which doubles as the reduced-audio "calm mode."
- **🔴 Money red-line, at TWO joints (R-SON §5d + R-VENDOR §2/§5b, both extending FE-09/`money_guard`):**
  money is never a field value / never tweens *and* never a continuous pitch or gain parameter — audio
  reads money as a discrete neutral event-tick only; a brand edit can shift `--money-ink` hue but never
  interpolates a money value. The guard extends into the new `audio` crate (§3) and survives the wgpu port
  unchanged.

---

## 6. What ships in the first coherent slice across the whole arc

Each research doc wrote its own "Phase-0 recommendation." They do **not** all land in this roadmap's
Phase 0 — they form a short dependency chain (Phases 0→1→…→8) whose union is the first genuinely
shippable, end-to-end-provable slice of the living interface. Pulled together:

1. **Enablement first (R-DEV Phase-0 → roadmap Phase 0).** One-time `cargo add wgpu`; the Mesa Lavapipe
   software-Vulkan GPU-smoke CI (shader compile + still-frame pixel-hash, correctness not fps); the local
   visual loop on the operator's own GPU device; **and the `'wasm-unsafe-eval'` header edit** without which
   *nothing wasm renders behind the production CSP*. This is what makes every later slice shippable at all.

2. **Token source second (R-VENDOR Phase-0 P0-1/P0-2 → roadmap Phase 1).** The single canonical
   `resolve(T1)->ResolvedTokens` crate emitting CSS literals + a linear-RGBA GPU table + `token_hash`, so
   the GPU renderer built next consumes a stable, drift-proof brand source (and the CI goldens have
   something fixed to render against).

3. **Two visible deliverables, in dependency order, sharing one event stream:**
   - **Order-lifecycle audio (R-SON Phase-0 → roadmap Phase 7):** one `AudioWorklet` + the Rust DSP third
     artifact, `postMessage` transport (no deploy-header change), sonifying the existing DZ-04 order
     lifecycle from the same `S(t)`, legacy `use-sound.ts` deleted. Ships the general order/wave UI now;
     the viz plugs into the identical `S(t)`→audio renderer next.
   - **HUB-tier living-memory viz (R-LM Phase-0 → roadmap Phase 8):** the 3-tier model present in the
     protocol from day one but only the HUB builder shipping, read-only/owner-scope/fail-closed, over the
     in-kernel 20-node fixture, with the falsifiable visual assertion that the 4 unreachable nodes
     `{5,6,12,16}` render **dark** and grain-count equals lit-node-count — every claim bound to an existing
     kernel test.

**The through-line that makes this a coherent slice, not four parallel demos:** all four Phase-0 slices
are consumers of one operator and one event stream. R-DEV proves the GPU path compiles and the wasm loads;
R-VENDOR guarantees one brand number reaches both DOM and GPU; R-SON and R-LM are the audio and video
renderers of the same kernel-validated `S(t)` — bound to the **single ordering authority** whose absence is
the most dangerous joint in §5 (J2). Ship them in this order and the arc's first slice is provable
end-to-end against tests that already exist in the kernel today; ship them out of order (viz before its
FE-12 spectral primitive, GPU renderer before the token source, either renderer before the shared ordering
authority) and the retrofit cost is exactly the friction §5 maps.

---

## 7. Self-critique pass (2026-07-16)

Per the operator's standing session-closing ritual (`AGENTS.md` — the 2-question doubt check), this
roadmap was immediately subjected to an independent, decorrelated adversarial review — a fresh pass that
re-verified the pipeline's own chain (research doc → synthesis → blueprint → roadmap) against the **live
repo HEAD**, not against the prior link's paraphrase. Findings below; the two load-bearing ones are already
corrected in-place above with `⚠ CORRECTED 2026-07-16` markers.

### What was checked against live source (not trusted from the chain)
- **All the most load-bearing blueprint file:line citations — re-grepped, and they hold.** The CSP header
  (`tools/native-spa-server/src/lib.rs:39` — `script-src 'self'`, no `'wasm-unsafe-eval'`, no COOP/COEP;
  golden-locked at `tests/integration.rs:213`), `engine/Cargo.toml` (`default = []`, `gpu = []` empty stub,
  comment "verified 2026-07-16"), `diffusion.rs` (`UNRELATED: [5,6,12,16]` at `:166`,
  `green_relatedness_ranking_correct` at `:220`, `WIKI_EDGES.len()==41`), `spectral.rs` (eigen*values* only;
  a kernel-wide grep for `eigenvector|coords_2d|coords_3d|spectral_embedding` returns **only** the Perron
  comment in `order_machine.rs` — the "no eigenvector→coords helper exists" GAP is **true**), `ParticlePool`
  (`widget_store.rs:68`, `pos_x/pos_y`, no `pos_z`), and `money_guard.rs` (`Money(pub i64)` deliberately not
  `FieldValue`) **all verify exactly.** The blueprints' current-state evidence is accurate; nothing needed
  correcting *in* them.
- **The 11-phase dependency graph is acyclic — traced.** Every phase's declared deps point to a strictly
  lower phase number (8→{5,7}; 7→{6,0}; 6→{3,4,5}; 5→{2,4}; 4→3; 3→{1,2}; 2→0). It is a clean topological
  order; no cycle, no back-edge. (The Phase 8→7 edge is *under-specified*, not circular — see Confirmed #1.)
- **The COOP/COEP-vs-`wasm-unsafe-eval` split (§2) actually improves on its own research input.** R-DEV
  §6.4 bundled COOP+COEP+`wasm-unsafe-eval` as one Phase-0 header edit and claimed sonification "depends on"
  COOP/COEP; R-SON §5a explicitly designs around it (`postMessage`, no COEP). The synthesis **caught this
  and split them correctly** (Decision A early / B deferred), and P00 correctly implements "Decision A only."
  This is the pipeline re-verifying and fixing a research-doc overstatement — cleared, no action.

### Confirmed, load-bearing (corrected in-place)
1. **J2 was only half-resolved (Q1-a).** The roadmap declared its "most dangerous joint" solved by a single
   shared ordering authority, but (a) the `order_machine` FSM guard covers only order-status transitions,
   **not** the viz's memory-graph signals (for those the shared authority is the `event_log` layer only),
   and (b) the two renderers consume **different wire shapes** — R-LM's `ActivityDelta.Signal{signal_type,
   kind:u8, energy:f32, node:u32}` vs R-SON's `on_event(kind:u32, count:u32)+FieldPos`. "Same ordered
   stream" is true for *when* to fire, false for *what* to fire from. Phase 8's "grain per activation via
   Phase-7's renderer" therefore needs an **adapter Phase 7 does not build** — now an explicit Phase-8
   deliverable. Corrected at J2.
2. **J4's "convergence" was a smoothed-over contradiction (Q1-a).** R-VENDOR §4 rules the viz marks
   T3/unbranded ("never read the 5 T1 tokens"); R-LM F-3 says the `--spectral` edge is "re-derived *per
   brand*." Those disagree on the marks. Ruled in R-VENDOR's favour (marks are brand-invariant T2/T3; only
   the ambient Sea is tinted) and flagged the linked colour-space gap (R-LM's bloom must read `resolve()`'s
   linear-RGBA palette). Corrected at J4.

### Flagged, not resolved (for the operator / an implementation pass — not silently decided)
- **The `wasm-unsafe-eval` necessity is inferred, not browser-verified.** R-DEV rates it HIGH but never ran
  it; P00's done-test 3b correctly makes it a *manual Chrome-console confirm*. It is consistent with current
  Chromium CSP behaviour (WASM compile needs `'wasm-unsafe-eval'` or `'unsafe-eval'` when a `script-src` is
  set), so shipping the one-token header edit is low-risk regardless — but the arc should treat "the current
  kernel wasm is blocked in prod today" as *probable*, not *proven*, until the one-line browser check runs.
- **The browser wasm-artifact ledger is ambiguous (C6).** §3 amends "two → three" (adds `dowiz_audio` for
  the AudioWorklet realm), but R-VENDOR P0-1 independently introduces a `brand-resolve` crate compiled to
  wasm for the live brand preview — a potential **fourth** browser artifact. The realm argument that forces
  `dowiz_audio` to be separate does **not** apply to `brand-resolve` (fold-vs-standalone is unspecified).
  Needs a definitive artifact ledger before Phase 1/7 land.
- **R-SON's "delete legacy `use-sound.ts`" headline is largely a no-op (C8).** Verified: `apps/web` is
  **untracked** (`git ls-files apps/web` = 0), `use-sound.ts` survives only under
  `apps/web/node_modules/@deliveryos/.ignored_ui/`, and the active tree already greps **zero** `.mp3` /
  `new Audio(`. Deleting it is a git no-op on quarantined dead code (P07 itself half-acknowledges this).
  Phase 7's done-test #1 is a documentation/regression-lock, not a live migration — the roadmap slightly
  over-frames it as a real deliverable. Non-blocking.

### Q2 — the biggest thing the arc is missing (flagged for the operator, not decided)
The same G11 ("first real order") priority tension the sovereign-architecture roadmap flagged applies here
**and is sharper.** That roadmap left it as an open charter question; for this arc the honest reading is
harder still: the living-interface arc's *visible* Phase-0 slice (§6) is **not the customer order path.**
Its two headline deliverables are (Phase 7) an order-lifecycle **audio enhancement** — R-SON's own
done-test 7.5 certifies audio is "never load-bearing" — and (Phase 8) an **owner-only 3-D diagnostic of the
hub's own memory graph** (the MEMORY.md wikilink fixture): pure agent self-introspection with **zero
customer touch**. The genuinely order-gating customer UI (Sea & Sheet, DZ-01..09) sits at Phase 6/9, deep
behind the GPU engine foundation, spectral eigensolver, and DSP crate — and even then it *re-renders an
order flow the legacy TS/JS stack already ships* into a wave-field. So the "UI is customer-facing therefore
more justified than crypto" argument does **not** rescue this arc as scoped: a product with zero completed
orders does not need its working order UI re-rendered on the GPU, and needs the memory-graph visualizer
least of all. **The one framing under which this arc is on-mission is the operator's PRIMARY directive
(MEMORY.md: "Main job = growth — reflection, metacognition, … bare-metal kernel as growth substrate"):**
under *that* charter the living-memory viz is core self-development, not a distraction, and G11 is a
secondary commercial goal. Which charter governs — commercial-delivery-first (→ this arc is premature
infrastructure; do the reliability-gate order path first) or growth-substrate-first (→ Phase 8 is
on-mission) — is an **operator-level decision this roadmap does not prejudge.** It is flagged here, not
resolved. Nothing in the arc should be built past Phase 1 until that charter question is answered, because
the answer changes the whole ordering.

---

*End roadmap. 11 phases (0–10), each with a name, covered new work, slotted existing FE/RW/DZ items,
explicit dependencies, and a falsifiable done-test. Planning only — no product code, CI config, or canon
edited. Supersedes nothing; extends FE-01..17 / RW-01..12 / DZ-01..12 / physics-ui-capture without
re-litigating their decided content. The one required amendment to existing canon (RW-09/RW-01 third wasm
artifact) is noted in §3 as a precondition of Phase 7, left for an implementation pass to apply. A
self-critique pass (§7, 2026-07-16) re-verified the chain against live HEAD: all blueprint file:line
citations hold and the phase graph is acyclic; two synthesis-level findings (J2 payload/impedance,
J4 viz-brand convergence) were corrected in-place; the G11 priority tension is flagged for the operator.*
