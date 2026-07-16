# G11 FAST-PATH — Consolidated Blueprint (Phases 0→1→2→3→4→5→6→9a)

> **One document, indexed, for the confirmed shortest path to the first real customer order.**
> Merges six execution-ready blueprints (`BLUEPRINT-P00/P01/P02/P06/P08§2/P09A`) that were each
> written by a separate agent pass this session — per operator directive, restructured around
> **waves of parallel work**, not just a linear phase list, and closed with an independent
> 2-question audit of the load-bearing technology decisions (§9). Status: planning complete,
> execution-ready, **no code written**. Branch this work belongs to: not yet created (this is the
> living-interface arc; harness work lives on a separate `feat/harness-llm-backend` branch — this
> arc has not started implementation).
>
> **Provenance:** `LIVING-INTERFACE-ROADMAP.md` §8 (the operator's 2026-07-16 ruling — "it DOES
> lead to a real order, once the interface is built" — selecting commercial-delivery-first and
> confirming this exact phase chain as the shortest path to G11). Six source blueprints, each
> already read in full and merged below with zero net loss of the falsifiable content (file paths,
> struct signatures, exact done-tests) — only narrative repetition is compressed.

---

## 0. Index

| § | Section | Phase(s) |
|---|---|---|
| 1 | Ground truth (deduplicated across all six sources) | — |
| 2 | Phase 0 — Dev/CI + Deploy Enablement | 0 |
| 3 | Phase 1/3/4 — Vendor Brand-Token Pipeline | 1, 3 (brand slice), 4 (brand slice) |
| 4 | Phase 2 — GPU Engine Foundation | 2 |
| 5 | Phase 3/4 — Render Primitives + Field Dynamics (engine slice) | 3, 4 |
| 6 | Phase 5 — Spectral Embedding Primitive (G11 slice) | 5 |
| 7 | Phase 6 — Sea & Sheet Backbone + the Ordering Authority | 6 |
| 8 | Phase 9a — Order-Critical Product Surface (the G11 deliverable) | 9a |
| 9 | 2-question audit of the load-bearing decisions | all |
| 10 | The wave map (cross-phase + within-phase parallelism) | all |
| 11 | What this does NOT cover | — |

---

## 1. Ground truth (deduplicated — verified live 2026-07-16, cited once, referenced by every phase below)

- **No Cargo workspace exists.** Four independent crates at repo root — `kernel/`, `engine/`,
  `wasm/`, `agent-governance-wasm/` — plus four under `tools/`, each with its own `Cargo.lock`, no
  root `[workspace]`. Every new crate this arc adds (`brand-resolve/`, `field-math/`, `audio/`
  placeholder) is **another standalone or workspace-joined peer**, never a physical relocation of
  existing crates (Phase 2 §2.1 explicitly rejects RW-01's literal "promote into `crates/`" — see
  §9(b) for the audit of that call).
- **`engine/Cargo.toml`**: `default = []`, `gpu = []` (empty stub, comment "verified 2026-07-16").
  Single dep `dowiz-kernel = { path = "../kernel" }`. Zero `.wgsl` files anywhere; zero `wgpu` in
  any `Cargo.lock`. `engine/src/bridge.rs:200-242` — `new_gpu()` returns an honest
  `Err("gpu adapter not built — wgpu uncached")`; **`VertexBridge::upload_once()` already does a
  real CPU staging copy (W20 upgraded it — a stale "only counts a hypothetical call" framing from
  an earlier session doc is corrected here)**, and a `GpuUploadSink` trait seam already exists at
  `bridge.rs:174` for exactly the real sink Phase 2 builds.
- **CSP is a latent production bug today.** `tools/native-spa-server/src/lib.rs:39`
  (`SECURITY_HEADERS`) and `docker/nginx-default.conf:23` both ship `script-src 'self'` with no
  `'wasm-unsafe-eval'` — Chromium blocks `WebAssembly.instantiate` behind this CSP; the *existing*
  kernel wasm only loads today because the *dev* server sends no CSP at all. RED-locked by
  `tools/native-spa-server/tests/integration.rs:213`'s exact-match assertion.
- **CI is a 2-job pipeline** (`telemetry-selftest`, `eqc-proofs`); no GPU/wgpu/shader job exists.
  A stale `visual.yml` (untracked `apps/web/**` target) is vestigial but donates two reusable CI
  mechanics: a `paths:` trigger filter and `concurrency: cancel-in-progress`.
- **`apps/web`, `packages/ui` (all i18n), `packages/domain` are fully deleted** (`79ef316f6` +
  `db766de47`, 2026-07-13); `apps/api`/`apps/worker`/`packages/db` are quarantined to `attic/`
  (`fce5738b0`). `git ls-files apps/web` / `packages/ui` = 0. **Every phase below that touches
  "the product UI" is a rebuild with feature-inventory preservation, never a port of live code.**
- **Kernel test count is 402** (an earlier "37" citation in RW-01 is stale) — the "stays GREEN
  throughout" baseline for every phase below.
- **`kernel/src/geo.rs` is fully ported and bridged** (10 `geo_*_js` wasm exports already exist,
  `wasm.rs:474-599`) — an earlier "~70% ported" note is stale. `kernel/src/cart.rs` and
  `kernel/src/messenger.rs` have complete kernel authority and tests but **zero wasm bridge**
  (`grep cart`/`grep messenger` in `wasm.rs` = 0) — this is real, current-session remaining work,
  not stale.
- **`place_order_js` (`wasm.rs:176-281`) currently trusts a client-supplied `unit_price`.** The
  server-side pricing authority (`kernel/src/catalog.rs::PriceCatalog::unit_price`) exists and is
  unused by the order path — this is the single most load-bearing correctness gap on the entire
  G11 fast-path (§8).
- **The RW-09/RW-01 "third wasm artifact" amendment (for the off-path audio work) is recorded in
  the roadmap but was never applied to the actual `BLUEPRINTS-RUST-ENGINE-REWRITE.md` file** — it
  remains a live dependency edge Phase 2 must apply as a doc-edit, not assume done.

---

## 2. Phase 0 — Dev/CI + Deploy Enablement

**Depends on nothing. Parallel-safe with Phase 1 (Wave 0).** Unblocks every wasm-shipping and
GPU-rendering phase downstream — the roadmap's widest-blast-radius joint (J6) even though the fix
is a single header token.

**Four recommendations, two of which are mutually independent within this phase (see §10):**

1. **`cargo add wgpu` (operator-gated, one-time network fetch), gated behind the existing `gpu`
   feature, never `default`.** The real W21 blocker was package availability, not GPU absence.
   `engine/Cargo.toml` gains `wgpu = { version = "22", optional = true }` under `gpu = ["dep:wgpu"]`
   — the default build stays zero-external-dep. `bridge.rs::gpu::new_gpu` stays the honest `Err`
   stub for Phase 0; the real sink is Phase 2's job.
2. **New CI workflow `.github/workflows/wgpu-smoke.yml`** running on **Mesa Lavapipe** (software
   Vulkan) — a proven pattern (wgpu's own CI does this). Job: install `mesa-vulkan-drivers` +
   `libgl1-mesa-dri`, request an adapter with `force_fallback_adapter: true` (binds Lavapipe, no
   window needed), `create_shader_module` compile-check over every `**/*.wgsl` (0 today, becomes
   load-bearing the moment Phase 2/3 add shaders), render one offscreen frame, read back, assert a
   **pixel-hash/SSIM-vs-golden** (never bit-exact across rasterizers — real GPUs round differently).
   `timeout-minutes: 10`, `paths:`-gated on `engine/**` + `**/*.wgsl`.
3. **The local visual loop** — no new infra: the GPU-less box serves, the operator's own
   GPU-having laptop/phone renders (`http://<box-ip>:4173` or the existing Cloudflare tunnel). One
   manual real-device pass per shader-touching change is the honest Phase-0 real-GPU gate;
   automated real-device CI is explicitly deferred to Phase 10.
4. **The CSP fix — exactly one token, three synchronized edit sites, one commit:**
   `tools/native-spa-server/src/lib.rs:39` and `docker/nginx-default.conf:23` both get
   `script-src 'self' 'wasm-unsafe-eval'`; `tests/integration.rs:213`'s golden literal updates in
   the same commit (RED→GREEN). **Explicitly NOT added: `Cross-Origin-Opener-Policy` /
   `Cross-Origin-Embedder-Policy`** — that's Decision B, deferred to Phase 10 (it would break
   cross-origin MapLibre tiles and R2 photos until proxied).

**Falsifiable done-checks:**
- AC-1: `cargo build -p dowiz-engine --features gpu` links wgpu and exits 0; the non-`gpu` build
  still pulls zero external crates.
- AC-2: the Lavapipe CI job is GREEN — shader compile-check passes, offscreen render matches the
  golden within SSIM tolerance, job is `paths:`-gated with a 10-minute ceiling.
- AC-3: a build served with the production header config throws no CSP `CompileError` instantiating
  the kernel wasm in Chrome; the golden-literal test passes with the new string.
- AC-4 (scope discipline): no COOP/COEP header added; no real `wgpu::Device` sink in `bridge.rs`
  yet (that's Phase 2); no GPU-runner/device-cloud automation added (Phase 10).

---

## 3. Phase 1/3/4 — Vendor Brand-Token Pipeline

**Phase 1 depends on nothing (Wave 0). The Phase-3 and Phase-4 slices below depend on Phase 2's
live device and Phase 1's crate respectively — see §10.** One continuous piece of work, three
roadmap phases, because the brand-token *source* must exist before any GPU consumer of it is built
(a second, independent reason beyond the obvious: CI goldens that include brand color must pin to
a stable `token_hash`, or every `resolve()` change silently invalidates them — §7 of the source
blueprint).

### 3.1 Phase 1 — the `resolve()` crate (new standalone crate `brand-resolve/`)

```
brand-resolve/
  Cargo.toml    crate-type ["rlib","cdylib"]; zero deps in default build; wasm feature = wasm-bindgen only
  src/lib.rs    pub fn resolve(&T1Inputs) -> ResolvedTokens
  src/input.rs  T1Inputs — the 5 owner-touched fields, 1:1 with location_themes columns
  src/resolved.rs  ResolvedTokens — canonical field order, canonical_bytes()
  src/css.rs    to_css() -> String   (resolved literals; NEVER emits color-mix())
  src/gpu.rs    to_gpu_table() -> [u8; 288]  (linear-RGBA + oklch)
  src/hash.rs   token_hash([u8;32]) — vendored SHA-256, zero-dep
  src/presets.rs   7 presets as 5-INPUT sets (not resolved sets) — a golden fixture
  src/color.rs  srgb<->linear, oklch<->srgb; f64 internal, f32 emit; native==wasm bit-identical
```

`T1Inputs` is the **only** owner-authorable surface: `accent, ink, ink_muted, paper, surface,
surface_raised: Srgb8`; `font_heading, font_body: FontId` (a `u16` index into a curated enum, never
a free string); `radius_px: u16`. `resolve()` is the **only** implementation of the T1→T2/T3
derivation — it absorbs the branding editor's live-preview path too (same wasm build, bit-identical
to the eventual server bake), and the 7 existing presets are redefined as 5-input sets re-derived
through `resolve()` to reproduce the current `presets.json` byte-for-byte (closing a second, implicit
derivation path).

**⚠ CORRECTED (§9 audit, Q1-f — necessary-not-sufficient finding) — `ResolvedTokens` must be
sealed.** The original design left `ResolvedTokens`'s fields implicitly public and never stated
`resolve(&T1Inputs)` as its *only* constructor — meaning a future route handler could hand-construct
a `ResolvedTokens` directly (bypassing the 5-input constraint) and still call `to_css()`/
`to_gpu_table()`. "Structurally incapable of emitting a per-component theme" is only true once (i)
`ResolvedTokens`'s fields are **private**, with `resolve()` as the sole public constructor, **and**
(ii) the bake job is stated as the **sole writer** of the CDN CSS path and the GPU-table blob — the
`token_hash` tripwire (below) catches a *divergent* write after the fact, but sealing the
constructor is what prevents an *unauthorized* write from being possible at all. Both are cheap
(private fields + `pub(crate)`/crate-sealed ctor) and are now part of this crate's design, not an
afterthought.

**⚠ Workspace note (see §4's correction):** this crate is created in Wave 0, before Phase 2
introduces the repo's first `[workspace]`. It is **not** a permanently-standalone peer directory —
Phase 2's workspace declares it as a member from the start (§4), so its build stays green across the
phase boundary. Design and build it as a normal crate; do not hand-roll workspace-avoidance
tooling on the assumption it will never join one.

**The `token_hash` drift tripwire (the anti-drift contract, exact wire format):** `canonical_bytes()`
is a fixed-order LE serialization in **display-space** (sRGB u8 quads, oklch stops as 4×f32, radius
as u16, font as u16) — stable across GPU-layout changes. `token_hash = SHA-256(canonical_bytes())`,
stamped into **both** artifacts: a leading `/* dowiz-token-hash: <64-hex> */` comment in the CSS, and
the first 32 bytes of the 288-byte GPU table. The bake job re-derives and cross-checks both — a
mismatch **fails the bake** (DOM≠GPU is unrepresentable by construction, caught belt-and-braces).

**"Auto-generate brand" constraint (operator hard rule, no AI theming):** the generator endpoint
must return type `T1Inputs` — structurally incapable of emitting a per-component theme, since the
type itself only carries the 5 fields.

### 3.2 Phase 3 slice — GPU wiring (consumer: FE-05's Bind0 UBO)

The 256-byte payload of `to_gpu_table()` is a fixed `array<vec4<f32>, 16>`, slot-mapped (sea_tint,
sea_backdrop, accent + 3 derived shades, ink×2, paper/surface×3, money_ink, price_red, 2 spectral
stops, and a scalar lane for radii + bloom threshold) — a shared golden byte-layout test keeps the
Rust `#[repr(C, align(16))]` struct and the WGSL `ThemeTokens` struct in lockstep. **Theme swap = one
`queue.writeBuffer` call, no re-tessellation** (geometry is color-free — the token UBO is
fragment-stage-only). The engine does **not** compile-depend on `brand-resolve` (keeps the engine's
default build zero-dep) — the 256-byte table crosses as opaque `&[u8]`. Curated fonts (~6, matching
`presets.json`) get server-baked MSDF atlases (Latin + Albanian + Cyrillic + tabular-nums); a
non-curated `FontId` is structurally unrepresentable. The Sea consumes **only scalars**, never vendor
imagery — the vendor logo is the one raster asset and lives on the Sheet only, never a field source.

**⚠ ADDED (§9 audit, Q1-c — the previously-unowned golden-pin seam).** Once this phase's Lavapipe
golden includes brand color (any golden rendered against a real theme, not the Phase-0/2 brand-free
prove-the-pipe shader), **`engine/tests/gpu_smoke.rs` is the named owner** of recording the
`token_hash` the golden was rendered against, and re-asserting it on every run. Without an explicit
owner this requirement (already correctly stated in principle) silently rots the moment a brand
color enters any CI golden — assigning it here closes that gap before it opens.

**⚠ ADDED (§9 audit, Q1-a — the naga↔Tint compiler-frontend ceiling, decided explicitly rather than
left an unstated assumption).** The single WGSL source shipped via `include_str!` is compiled by
**two independent front-ends**: naga (native, what the Lavapipe CI gate exercises) and the browser's
Tint (production). They can diverge on accepted feature subset and strictness. **Decision: accept
the manual real-device pass (Phase 0 §4) as the ceiling for this specific risk, with an `innovate:`
marker naming the upgrade trigger** — an automated browser-engine (Tint) WGSL compile gate added to
CI, revisited if a real naga-passes/Tint-rejects incident ever occurs. This is a stated, accepted gap,
not a silent one.

### 3.3 Phase 4 slice — bloom-aware contrast gate + live push

**The gap:** the existing WCAG check validates flat ink-on-paper contrast only; GPU emissive bloom
raises effective backdrop luminance and can wash out on-Sea text even when flat AA passes. **Design:**
a `contrast.rs` gate, run in the same bake job right after `resolve()`, composites the resolved
backdrop through the (brand-invariant, T2-fixed) bloom curve at max Sea energy and checks WCAG AA of
on-Sea ink against the **post-bloom** luminance — deterministically computable at save time, no GPU
needed. Scope is narrower than it looks: only text drawn directly on the Sea (Sheet text is opaque
paper, unaffected). **Live push:** owner edits accent mid-session → a small WS event carries the new
`token_hash` → client fetches the new table → one `writeBuffer` → SPREAD/heat-kernel swap over FE-08
ζ=1 critical damping, **money frozen** throughout (`money_guard`'s `Money`-is-not-`FieldValue`
compile barrier untouched — a color-hue shift on `--money-ink` is not a value tween).

**Colour-space handling (the subtle brand-wide bug this closes):** CSS is sRGB/oklch display space;
GPU bloom/blending must be linear light. `resolve()` applies the standard sRGB→linear EOTF once,
CPU-side, emitting linear-RGBA f32 for the GPU table — naive hex reuse on the GPU would ship a
consistent, brand-wide wrongness. This is a **real-device** verification item (Lavapipe f32 rounding
differs from real GPUs here), not CI-golden-only.

**Falsifiable done-checks (all three slices):** T1Inputs → bit-identical native/wasm output;
`grep 'color-mix('` on generated CSS = 0; `token_hash` cross-check passes or the bake fails; brand
generator returns only `T1Inputs`; GPU-rendered card vs CSS card pixel-diff below threshold; theme
switch = exactly one `writeBuffer`; every curated font crisp at 3 scales in sq/en/uk; a brand passing
flat AA but failing post-bloom contrast is rejected (RED→GREEN); live accent edit migrates the
customer view with money unchanged and no reload.

---

## 4. Phase 2 — GPU Engine Foundation

**Depends on Phase 0 (wgpu must be linkable + Lavapipe CI must exist to verify this phase's work).**
Originates no new design — pure integration of six already-designed items (RW-01, FE-01, FE-02,
FE-03, RW-05, RW-10) into one working, tested GPU foundation every later phase builds on.

**The corrected framing (re-verified, not assumed):** FE-01/02/03 are already built as **headless
CPU Rust** — ~51 tests, real zero-copy CPU staging (`upload_once()` at `bridge.rs:157` genuinely
stages, contradicting an earlier "only counts" claim). What's missing is a concrete `GpuUploadSink`
backed by a real `wgpu::Queue`, and the `wgpu::Device`/`Surface`/`Buffer` behind it — additive to
the existing `GpuUploadSink` trait seam, not a rewrite.

**Dependency order (each step waits only on strictly earlier ones):**

| Step | Delivers | Waits on |
|---|---|---|
| A — RW-01 | Root `[workspace]` over existing paths (kernel/ stays put, RW-01's literal "promote into crates/" is deliberately not followed — see §9(b)); vendored `field-math/` crate (`#![no_std]+alloc`, zero deps, copies bebop2's `field.rs`/`chebyshev.rs`/`fft.rs`/`algebra.rs` verbatim incl. tests + `eig_parity.rs`-style eigenvector residual checks — `field::jacobi_eigen` genuinely returns tested `(eigvals, eigvecs)`, audited §9); an *empty* `audio/` placeholder crate | Phase 0 |
| B — FE-02/03 slot-in | SoA store + fixed-timestep confirmed green under the new workspace | A |
| C — FE-01 real-device wiring | `WgpuSink` (native `wgpu::Device`+`Queue`, `force_fallback_adapter` for Lavapipe) **and** the browser path (JS-owned WebGPU, zero Rust `wgpu` in the browser wasm) — the dual-context model, see §9(a) | A, Phase 0 |
| D — RW-05 | Single `wasm-bindgen` shell, <10 zero-JSON numeric exports (`instance_ptr/len`, `on_event(kind,count)`, `resize`, …) replacing the current copy-and-JSON `dowiz-wasm` surface | A, B, C |
| E — RW-10 | Build toolchain + Astro island (`client:only`) creating the browser WebGPU device, ≤2MB gzip budget | D |

**B and C are mutually independent once A lands — a genuine within-phase parallel wave (§10).**

**⚠ CORRECTED (§9 audit, Q1-b + Q2 — a BIG DEAL finding, fixed here before this phase may land).**
Step A's `members` list as originally drafted (`["kernel","engine","wasm","field-math","audio", …]`)
was **incomplete and self-contradictory against §3 below**: it omitted `agent-governance-wasm` and
all four `tools/*` crates (`deep-clean`, `async-spool`, `native-spa-server`, `telemetry/rust-spool`)
— including the **production SPA/CSP binary Phase 0 itself edits** and the **cronjob-critical
`deep-clean`** — and it omitted `brand-resolve`, the crate §3 creates one phase earlier. Under Cargo
semantics, the instant a root `[workspace]` exists, every `Cargo.toml` beneath it must be a
`members` entry or `exclude`d, or that crate's build fails with "believes it's in a workspace when
it's not." Landing the incomplete list as originally drafted would have broken `brand-resolve`'s
build (and hence Phase 1's bake job + `token_hash` CI cross-check) **the moment Phase 2 executed** —
silently, on the G11 fast path.

**The corrected `members` list, enumerating every sibling crate found live in the repo:**
```toml
[workspace]
resolver = "2"
members = [
  "kernel", "engine", "wasm", "field-math", "audio", "brand-resolve",
  "agent-governance-wasm",
  "tools/deep-clean", "tools/async-spool", "tools/native-spa-server", "tools/telemetry/rust-spool",
]
```
**§3's "no root workspace exists" framing is time-scoped, not contradicted**: it was true when Phase
1 lands (Wave 0, before Phase 2), and Phase 2 is where the workspace is introduced with
`brand-resolve` as a declared member from the start — §3 below is corrected to say so explicitly
rather than assert workspace-absence as a durable fact.

**The dual-context device model, stated precisely:** native Rust `wgpu` (behind `feature = "gpu"`)
serves CI-headless + the operator's local visual loop; the **browser production path never links
`wgpu`-in-wasm** — it's JS-owned `navigator.gpu` calling `queue.writeBuffer` over the shell's
zero-copy `Float32Array` view. Both sides implement the identical one-`writeBuffer` `GpuUploadSink`
contract. This keeps the bundle under the 2MB budget and honors the determinism firewall (physics
stays CPU-side/Rust-authoritative; GPU is a dumb display sink on both sides) — audited in §9(a).

**Falsifiable done-checks:**
- DT1: `field-math` re-runs its bebop2 test suite GREEN unchanged; wasm32-clean build.
- DT2: `new_gpu` no longer returns `Err`; the Lavapipe smoke test renders a real offscreen frame
  end-to-end; frame-loop profiler shows `json_parse_calls == 0` and `write_buffer_calls == 1` on
  both native and default builds; the browser path does one real `writeBuffer` per frame, no `Vec`
  copy-out.
- DT3: shell wasm ≤2MB gzip; Astro island mounts, creates the device, renders, and degrades without
  crashing if WebGPU is absent; all 402 kernel tests + ~51 engine tests stay GREEN.
- Scope guard: no FE-04 particle shader, no WGSL compute integrator, no full FE-16 WebGL2 fallback,
  no COOP/COEP, no audio DSP — all explicitly later phases.

---

## 5. Phase 3/4 — Render Primitives + Field Dynamics (engine slice, non-brand items)

*(The brand-specific items for these two phases are §3.2/§3.3 above; this section is the remaining
FE-item content, each already independently blueprinted in `field-ui-engine/BLUEPRINTS-FIELD-UI.md`
— referenced here for completeness of the fast-path index, not re-derived.)*

**Phase 3** (depends on Phase 1, 2): **FE-04** particle→wgpu port (fixes a known blue-hardwire bug,
widens to full RGBA), **FE-06** MSDF text (paired with the brand font-atlas bake, §3.2). **Phase 4**
(depends on Phase 3): **FE-07** layout (SMACOF, retained only as the warm-start relaxation for
single-node topology changes, not cold layout — the cold layout is Phase 5's spectral embedding),
**FE-08** motion (ζ=1 critically-damped spring easing — the mechanism every later phase's "no
overshoot" done-test cites), **FE-09** the `money_guard.rs` red-line (`Money(i64)` deliberately not
`FieldValue` — the compile-time backstop under every money-tween-kill mechanism downstream),
**FE-14** lazy-render-on-settle (dormant rAF on a static frame), **FE-16** WebGL2 + scalar-SIMD
fallback (completes the mount-time degrade seam Phase 2 only stubs).

---

## 6. Phase 5 — Spectral Embedding Primitive (the G11-relevant slice)

**Depends on Phase 2 (the vendored `field-math` eigensolver) and Phase 4 (field-dynamics guards).**
The **one net-new kernel primitive** the whole living-interface arc needs (the rest of Phase 5 —
FE-10 Green's-function feedback, FE-11 focus wells, FE-13 constraint solver — is already fully
blueprinted pre-existing content in `field-ui-engine/BLUEPRINTS-FIELD-UI.md`, referenced not
repeated here; FE-10 specifically is what DZ-05, §7 below, is built over).

**Target: new sibling module `kernel/src/spectral_embedding.rs`** (NOT an edit to `spectral.rs`,
which stays the eigen*value*-only engine — zero regression surface on its FSM/drift-gate callers).

```rust
pub fn coords_2d(adj: &[Vec<f64>]) -> Vec<[f64; 2]>   // n rows → (x,y) = (φ₂, φ₃)
pub fn coords_3d(adj: &[Vec<f64>]) -> Vec<[f64; 3]>   // n rows → (x,y,z) = (φ₂, φ₃, φ₄)
```

Builds `L = D − A` by reusing `spectral::laplacian` (no new Laplacian code), gets low eigenvectors
from the vendored `field-math` solver (the kernel's own `householder` path returns eigenvalues only
— this module is the single consumer of the solver's eigenvector output), discards the constant φ₁
(DC component, collapses to one point), takes φ₂→x, φ₃→y, φ₄→z (the axes of maximal cluster
separation — why spectral beats force-directed on tangle-freeness).

**The non-obvious determinism discipline** (two free choices that must be pinned or byte-identity
breaks): **sign** (φ and −φ are both valid — fix: flip so the first structurally-nonzero component is
positive) and **ordering/degeneracy** (near-equal eigenvalues can return in any order — fix: sort
ascending with a deterministic tie-break on a stable index, fixed solver iteration count/seed, no
convergence-epsilon early-out).

**Falsifiable done-checks:** on a real graph with a disconnected component, the embedding separates
it cleanly along φ₂ (RED case: naive force-directed local-minimum tangle); `coords_3d(adj)` run twice
is bitwise-identical including full-precision serialization (the same structural test pattern as the
kernel's existing `diffusion::green_ppr_byte_identical_two_runs`); the embedding is one operator over
`L`, not per-component layout code.

---

## 7. Phase 6 — Sea & Sheet Backbone + the Ordering Authority

**Depends on Phase 3, 4, 5.** DZ-01 through DZ-05 are UI compositions over the engine — real work,
no new *design*. **DZ-06 is where this phase's one genuinely new deliverable lives: the
kernel-validated monotonic ordering authority (the roadmap's "most dangerous joint," J2).**

### 7.1 Shell / spectral-edge / token / vocab sequencing (DZ-01→05)

- **DZ-01 `<Shell>` first**: the two-layer grammar (Море field-backdrop under everything, per
  role/screen; Sheet rises FLUID ζ≈0.8 with brand radius, spectral rim, grip) + the 3-act
  `arrive→choose→receive` state machine bound to 3 URL states with working-back. DZ-04 needs Act
  3's tracking layer, so DZ-01 must precede it.
- **DZ-02 tokens + `<Money>`** (parallel with DZ-01's content pass): the GPU table is already wired
  (§3.2); this phase's residue is the UI surface — the `<Money>` component (mono, tabular,
  integer-from-kernel, no tween prop exists), the ESLint no-tween-price rule, and converting the 4
  legacy money-tween call sites to `<Money>` snap.
- **DZ-03 spectral edge + transitions** (after DZ-01's Sheet exists): the button-anchored ζ=1 DIVE
  (ring-wipe + refraction + chromatic), FLUID SHEET-RISE, attending-speedup. **Corrected per the
  roadmap's own J4 finding: the `--spectral` edge is T2-fixed, brand-invariant — NOT "re-derived
  per brand" as an earlier DZ-03 draft said** (cross-tenant legibility + bloom-contrast control
  require it; only the ambient Sea carries the owner tint).
- **DZ-05 the general Green's-function vocab** (buildable in parallel with DZ-01-03 — it's the
  general event→field-source machine, a prerequisite of DZ-04 but not sequenced after DZ-03): one
  event→`FieldSource` table over FE-10 (tap→ripple, add-cart→pulse+money-snap, success→heat bloom,
  error→shake, loading→sustained, order-placed→amber burst, anomaly→agitation). Particles are
  tracers of the same field ripples animate — "two renderers, one field," the arc's founding thesis.

### 7.2 DZ-04 — OrderStatus → Sea (the production "Tide over Bedrock")

The **Bedrock** is the kernel-validated `OrderStatus` from `fold_transitions` (§7.3). The **Tide**
is a continuous field target `f(OrderStatus)` — ember drift (maturing) → teal swirl (in-delivery) →
gold bloom burst (delivered) → blood-red swirl (rejected) — replacing the current one-shot particle
burst. Amplitude grows and color travels terracotta→gold as status advances; the binding is **local**
(no server round-trip for the render — the status the Tide reads is already validated). Money on the
tracking Sheet stays `<Money>` snap. **Depends on DZ-06's ordering spine (§7.3) and DZ-05's vocab.**

### 7.3 THE ORDERING-AUTHORITY DESIGN (the phase's one net-new deliverable)

Two layers of authority feeding one stream — and critically, **the roadmap's own self-critique (§7
of `LIVING-INTERFACE-ROADMAP.md`) found these two layers do NOT cover the same renderers**, so this
scope fence matters:

**Layer A — the common substrate (`kernel/src/event_log.rs`), serves every future renderer:** every
field-relevant event is a `MeshEvent { prev, actor_pubkey, actor_seq, payload }`, content-addressed
by `event_id() = sha3_256(prev‖actor_pubkey‖actor_seq‖payload)`. This layer gives, for free:
idempotency (a re-received content-id is a structural no-op — `AppendOutcome::Duplicate`, proven by
an existing kernel test), per-actor happens-before order (`actor_seq` + the `prev` hash-chain), and
a reserved `epoch:u64` field (unused by Phase 6 itself, but reserved now so Phase 8's viz can pin
deltas to a layout epoch later without a wire-format change).

**Layer B — order-path validation + total order (`kernel/src/order_machine.rs`), the G11-critical
half:** the `decide` closure passed to `commit_after_decide` is exactly `assert_transition`. An
illegal transition is **never persisted, never emitted downstream** — the red-recoil UI reaction is
driven by that local `Err`, not a server round-trip. On reload, `fold_transitions` replays the
persisted sequence into canonical current status with zero round-trip, stopping at the first
illegal step so a corrupted tail can never fabricate forward state.

**`t_logical` — the derived scheduling key this phase defines** (no such field exists in the kernel
today; it is a presentation-schedule key, not kernel state): the index of an event in the
fold-validated replay, monotone by construction, ties broken by `actor_seq` then `event_id`. Both
off-path consumers (P07's audio worklet, P08's viz) already name this field as their scheduling
authority — Phase 6 is where it's produced, not where it's consumed.

**The `S(t)` stream contract Phase 6 emits:** one `FieldSource` record per accepted event, with a
**common envelope** (`t_logical`, `event_id`, `actor_seq`, `epoch`) and a **renderer-specific
payload** (deliberately NOT unified — order path carries the `OrderStatus` transition; the viz/audio
payloads are explicit Phase 7/8 deliverables, not designed here).

**The scope fence (why this is cheap now, expensive later):** only **Layer B** is strictly required
for G11 — DZ-04 and Phase 9a consume the fold-validated `OrderStatus` stream. **Layer A is landed
anyway because it's the same code the order path already uses** — reserving the common envelope now
is what lets Phase 7/8 plug in later *without re-plumbing either scheduling loop*, which is precisely
the "cheap to design in, very expensive to retrofit" property that makes this the roadmap's most
dangerous joint. This also **replaces the legacy hand-maintained JS mirror `channel.js`** with the
canonical kernel-wasm `assert_transition` — no drift between a JS mirror and the Rust Law.

**Falsifiable done-checks:** 3 acts = 3 URL states with working-back; Sea under every screen,
reduced-motion → static+legible; a legal status advance produces the amplitude/color shift, an
illegal one produces a local (0-network-request) red recoil; `<Money>` never tweens; reload replay
reconstructs canonical state with 0 server requests; a duplicated/reordered persisted event is a
structural no-op; the `S(t)` stream emits exactly one record per accepted event, none for
illegal/duplicate events, with an envelope matching what the off-path P07/P08 blueprints already
assume.

---

## 8. Phase 9a — Order-Critical Product Surface (the G11 deliverable itself)

**Depends on Phase 6 only** (transitively 3, 4, 5) — **verified, not assumed**: no order-critical
item in this phase reads Phase 7 (sonification) or Phase 8 (memory-viz). This is the actual
deliverable the whole fast-path exists to produce: a real customer places and completes a real order
on the new interface.

**Acceptance authority: the `/reliability-gate` skill's L0-L11 order-lifecycle trace**, re-pointed at
the new interface. This blueprint invents no parallel verification scheme.

**⚠ CORRECTED (§9 audit, Q1-d — a BIG DEAL finding: this is the done-test for the entire phase, so
the original "only the file-target list changes, semantics preserved unchanged" claim could not
stand uninvestigated).** The **five threads** (exactly-once, recoverable, cross-surface consistent,
proof-by-artifact, timely signal) genuinely are architecture-agnostic. The **per-stage PASS
criteria are not** — reading `.claude/skills/reliability-gate/SKILL.md` in full shows they are
concretely Postgres/Express/pg-boss assertions (L2: one `BEGIN/COMMIT` with an `idempotency_keys`
composite-PK row; L5: `WHERE status=$current RETURNING id`, `rowCount=0 → 409`; L7:
`delivery_trace` `ON CONFLICT (order_id) DO NOTHING` + RLS FORCE; L9: ratings `UPSERT ON
CONFLICT(order_id)`; L11: `NOTIFY`-based cross-instance broadcast). Live-verified: `place_order_js`
(`kernel/src/wasm.rs:276`) is a **stateless function with zero references to idempotency,
`AppendOutcome`, or `event_log`** — it cannot satisfy L2's literal criterion as written, and §7's
earlier claim that it ships with "an idempotency guard" named an artifact that does not yet exist
and was scoped to no deliverable.

**The decision this phase cannot silently make for itself — pick one explicitly before Phase 9a is
called done:**
- **(a) Wire the kernel order path to satisfy the gate's actual criteria** — add the missing
  idempotency guard to `place_order_js` backed by `event_log`'s existing `AppendOutcome::Duplicate`
  content-id dedup (§7.3's Layer A, already reserved for exactly this), map L5's anti-race onto
  `order_machine::assert_transition`'s `Err(Illegal)`, map replay onto `fold_transitions`. This
  makes "the kernel order path is the L2/L7 backend" literally true, and adds "wire `place_order_js`
  idempotency" as a **named, scoped 9a deliverable** (§8.1's prerequisite table gains a fifth row).
- **(b) Rule explicitly that the mesh/attic Postgres backend remains the audited L2/L7 order
  backend for a 9a GO**, and that `place_order_js`/`event_log` are not yet the gate's target — in
  which case the Postgres criteria stand as originally written and the "re-point" is narrower than
  claimed (client+courier UI screens only, not the order-write path).
- **Recommendation, not a silent pick:** (a) is the more coherent long-term direction (it's what
  DZ-06's local-first design and §7.3's Layer A already point toward, and it avoids indefinitely
  keeping the *new* interface's most safety-critical path backed by the *deleted* architecture's
  database), but it is real, scoped work this document did not previously account for — do not
  treat it as already done by merely re-pointing a file list.

Either way, §8.2's Checkout screen build and §8.1's prerequisite table must reflect whichever option
is chosen — this correction does not resolve the choice, it makes clear that a choice is required.

### 8.1 The one hard sequencing gate: kernel-port prerequisites before UI screens

| Prerequisite | State | Blocks | Must land before |
|---|---|---|---|
| `geo_*_js` family | **DONE** (already green) | Track map, Delivery map | — |
| `cart_*_js` over `cart.rs` | kernel done, bridge missing | Detail add-to-cart, Checkout summary | **Checkout screen** |
| `place_order_js` priced through `catalog.rs` (stop trusting client `unit_price`) | **gap — the single most load-bearing correctness item on the whole fast-path** | Checkout place-order, the reliability-gate's L2 criterion | **Checkout screen** |
| `messenger_*_js` + `format_money_js` | kernel done, bridge missing | Checkout messenger link, Earnings display | **Checkout / Earnings** |
| `place_order_js` idempotency guard over `event_log`'s `AppendOutcome::Duplicate` | **conditional — only if reliability-gate option (a) above is chosen** | L2's `idempotency_keys` criterion | **Checkout screen, same commit as the catalog-pricing rewire (same function)** |

`geo_*_js` is off the critical path (already done). **The cart bridge and the catalog-pricing rewire
are the two things that must land before Checkout can be built** — everything else in this phase is
UI construction against already-available or already-being-built kernel surface.

### 8.2 CLIENT (4 screens) and COURIER (6 screens) — build order

**CLIENT** on the DZ-01 shell: **Menu** (`/s/:slug`, SSR real DOM — never migrates, public SEO +
native screen-reader; needs nothing new) → **Detail** (bottom-sheet, modifier groups, add-to-cart via
the new `cart_*_js`) → **Checkout** (delivery/pickup, contact+messenger, entrance photo to R2,
summary via `estimate_order_total_js`, **`place_order_js` with idempotency key + OTP + server
pricing** — the riskiest screen, carries the exactly-once and server-pricing threads) → **Track**
(DZ-04's sea-develops, live WS status + 30s watchdog, courier map as geo field flow, rating+feedback
at DELIVERED).

**COURIER**: Login/Invite (role-aware invite redeem) → Shift (live timer, on/off pulse) → Tasks
(WS `task_assigned` → one ripple+ping+dedupe — the exactly-once thread at the courier edge, 60s
auto-decline) → Delivery (12s GPS heartbeat, live map, **`SwipeToComplete` — never fakes success,
resets on failure** — the DELIVERED termination node) → Earnings (`<Money>` snap) → History.

**Feature-preservation gate**: every row of the client/courier master checklist gets a REBUILT
disposition in a reconciliation ledger — silent absence is RED. The courier-rating /
NO-COURIER-SCORING question is flagged as an operator ruling, not resolved in this blueprint.

### 8.3 The money-tween-kill mechanism — three enforcement layers, not one

1. **Structural**: `<Money>` has no `tween`/`duration`/`from` prop — count-up is unreachable by
   construction; there is no `AnimatedNumber` in the new tree (the legacy sites this used to name
   exist only in the deleted tree, so this is a never-introduce constraint, not a legacy edit).
2. **Static gate**: an ESLint/grep rule denying number-animation on money-bound identifiers, with a
   RED→GREEN falsifier (plant a violation on a probe branch, confirm RED; remove, confirm GREEN).
3. **Boundary (runtime backstop)**: `money_guard.rs` — `Money(i64)` is deliberately not a
   `FieldValue`, so the Sea physically cannot interpolate a money value even if a view tried.

### 8.4 Hybrid a11y (non-optional, kept in this phase, not deferred)

Three mechanisms: SSR menu stays real DOM (never migrated to canvas); every checkout/OTP/contact
field is a real transparent `<input>` overlay (autofill, IME, mobile keyboard all preserved, forms
never canvas-faked); a hidden semantic DOM mirror reconciled per-frame from the widget list (dishes
as real `<button role aria-label>`, `aria-live` announces status changes, keyboard nav operates the
mirror including the `SwipeToComplete` keyboard path).

### 8.5 Reconciliation with the sovereign-architecture roadmap's Phase 16 (complementary, not
redundant)

Phase 16 (`sovereign-roadmap-2026-07-16/BLUEPRINT-P16-product-ui-rebuild.md`) is the **horizontal**
rebuild — all 26 pages (client+courier+owner) + i18n + WCAG + responsive matrix, on the sovereign
roadmap's own dependency chain. Phase 9a is the **vertical G11 slice** through it — client+courier
only, sequenced for the fastest path to one completed order, owner/i18n/full-WCAG deferred to 9b.
Phase 9a inherits (does not re-author) Phase 16's reconciliation-ledger methodology, wasm-math grep
gate, and Sea&Sheet spec — scoped to its own two rows. The only real divergence is the acceptance
instrument: Phase 16 uses a broad Playwright E2E including PoD-signing and payout-over-mesh; Phase 9a
uses the narrower `/reliability-gate` L0-L11 trace (a strict subset — the sharper instrument for "one
real order completed," not "the whole product is rebuilt"). If both are executed, 9a lands first and
yields G11; Phase 16's remainder is 9b-and-later.

**Falsifiable done-check (the whole phase, one gate):** `/reliability-gate` returns **GO** — all
L0-L11 stages PASS with a code-citation artifact, exactly-once throughout, N=2 cross-instance
consistency, zero cross-tenant leak, zero partial state on rollback. Plus: money-bound `AnimatedNumber`
grep = 0; screen-reader reads the semantic mirror; the degrade path (no WebGPU → WebGL2/static) works
without crashing; SSR menu stays DOM.

---

## 9. Two-question audit of the load-bearing decisions

Independent, decorrelated audit — full detail in `G11-FAST-PATH-2Q-AUDIT.md`, applied to
decisions/technologies rather than to prose, per the AGENTS.md ritual (now mandatory at the
blueprint-organization stage, not only at closing). **Two findings hit the "big deal" bar and are
already fixed in-place above** (marked `⚠ CORRECTED`); three cheaper findings are also fixed
in-place (marked `⚠ ADDED`); the rest cleared.

**Q1 — seven load-bearing decisions investigated:**

| Decision | Verdict | Disposition |
|---|---|---|
| (a) Dual-context device model (native `wgpu` vs JS-owned WebGPU, `GpuUploadSink` seam) | CLEARED\* | The seam is real and sound; residual naga↔Tint compiler-frontend divergence has no automated gate — **fixed**: accepted as the ceiling with a named `innovate:` upgrade trigger (§5) |
| (b) No workspace relocation; add a root `[workspace]` over existing paths | CONFIRMED | Deviation itself is safe (0 tooling assumes `crates/`); the `members` list was incomplete, orphaning production/cron crates — **fixed**: full member enumeration (§4) |
| (c) `resolve()`'s "five consumers incl. CI golden" | CLEARED\* | No real P00↔P01 contradiction (temporally consistent — brand-in-golden is correctly Phase-3-only); the token_hash-golden pin was unowned — **fixed**: assigned to `gpu_smoke.rs` (§3.2) |
| (d) reliability-gate re-point "preserved unchanged" | **CONFIRMED — the big deal** | Five threads are agnostic; per-stage PASS criteria are concretely Postgres-shaped and don't map onto `place_order_js` as-is — **fixed**: the criteria-mapping decision is now explicit, not silently claimed (§8) |
| (e) event_log/order_machine "serves order+audio+viz" | CLEARED / nuance CONFIRMED | Reserving `epoch` is genuinely near-free (not an over-build); "Layer A *is* the gate's exactly-once" was overstated — same root cause as (d), fixed together |
| (f) "5 T1 tokens, no AI theming" via return type alone | CONFIRMED (partial) | Necessary but not sufficient without a sealed constructor — **fixed**: `ResolvedTokens` sealing specified (§3) |
| (g) Phase-5's eigenvector source (bebop2 `field.rs`) | CLEARED | Verified against actual bebop2 source, not just a citing document — `field::jacobi_eigen` genuinely returns tested `(eigvals, eigvecs)` |

**Q2 — the sharpest finding (the tokio/ureq-class cross-document inconsistency, on this arc):** the
brand-token blueprint (§3) stated "the repo has no root workspace" as a durable fact; the GPU-engine
blueprint (§4) then created one — without listing the crate the first blueprint had just built.
Neither document's own self-review caught it, because each was internally consistent; the
contradiction existed only *between* them, exactly the shape of the earlier harness-arc's
tokio-vs-ureq drift. **Fixed in §3/§4 above** — the workspace framing is now time-scoped and the
member list is complete.

**What stayed clean, verified not assumed:** the 402/51 kernel/engine test counts, the
`order_machine`/`event_log` line citations, the `place_order_js` client-price gap, the empty `gpu`
feature stub, and "no eigenvector→coords helper exists in-kernel today" all hold exactly as stated.

---

## 10. The wave map — cross-phase and within-phase parallelism

**Cross-phase (the fast-path itself) is mostly a single chain, honestly — no false parallelism is
claimed where none exists:**

```
WAVE 0:  Phase 0 (dev/CI+CSP)  ‖  Phase 1 (brand-resolve crate)      [mutually independent]
WAVE 1:  Phase 2 (GPU engine foundation)                              [needs Phase 0]
WAVE 2:  Phase 3 (render primitives + brand-on-GPU)                   [needs Phase 1, 2]
WAVE 3:  Phase 4 (field dynamics + contrast-gate + live-push)         [needs Phase 3]
WAVE 4:  Phase 5 (spectral embedding primitive)                       [needs Phase 2, 4]
WAVE 5:  Phase 6 (Sea & Sheet backbone + ordering authority)          [needs Phase 3, 4, 5]
WAVE 6:  Phase 9a (order-critical product surface = G11)              [needs Phase 6 ONLY]
```

**Within-phase parallel sub-waves — real ones, extracted from the source blueprints, not invented:**

- **Phase 0**: recommendations {1+2+3 (wgpu+CI+visual-loop chain)} run in parallel with
  {recommendation 4 (CSP fix)} — different files (`engine/` vs `tools/native-spa-server/` +
  `docker/`), zero shared state.
- **Phase 1**: once `ResolvedTokens` exists, `to_css()` / `to_gpu_table()` / `token_hash()` are three
  independent serialization functions off the same struct — parallelizable.
- **Phase 2**: Step B (FE-02/03 slot-in) and Step C (FE-01 real-device wiring) are mutually
  independent once Step A (workspace + vendored `field-math`) lands — a genuine two-way parallel
  wave before Step D (RW-05 shell, which needs both).
- **Phase 3/4**: FE-04 (particle rendering), the brand-GPU-wiring slice (§3.2), and FE-06 (MSDF
  text) touch disjoint engine concerns and are parallel-safe once Phase 2's device exists; likewise
  FE-07/08/09/14/16 in Phase 4 are independent field-dynamics concerns.
- **Phase 6**: DZ-06 (the ordering-authority backend work, pure kernel/event-log) can build in
  parallel with DZ-01/02/03 (frontend shell/token/spectral-edge work) — they touch disjoint files
  and DZ-04 (which needs both) is correctly sequenced last.
- **Phase 9a**: the three kernel-port bridges — `cart_*_js`, the `catalog.rs` pricing rewire, and
  `messenger_*_js`/`format_money_js` — are three independent wasm-export additions over
  already-existing, already-tested kernel logic; they can be built in parallel by up to three
  workers, with the UI screens that consume each gated on its specific bridge landing (Detail needs
  cart; Checkout needs cart+pricing+messenger; the courier screens need only the already-green
  `geo_*_js`).

**The honest limit of this wave structure**: the seven cross-phase waves are NOT parallelizable with
each other — each phase's engine/rendering foundation is a real prerequisite of the next, verified
against the actual technical dependencies in each source blueprint, not assumed from draft order.
Claiming otherwise would repeat the exact mistake the harness arc's consolidation caught and fixed
(an artificially "strict" sequencing there turned out to hide real parallelism; here, checking
honestly finds the opposite — real sequential dependencies that a swarm cannot shortcut). A swarm
executing this fast-path gets its speedup from the **within-phase** waves above, not from attempting
to collapse the seven-phase chain.

---

## 11. What this does NOT cover

- **Phase 7 (sonification) and Phase 8 (living-memory viz, full)** — both confirmed off the G11
  critical path (roadmap §8). Their blueprints (`BLUEPRINT-P07-sonification-phase0.md`,
  `BLUEPRINT-P08-living-memory-viz-phase0.md`) remain separate, unmerged documents — they are a
  distinct growth-substrate track, not part of "the same treatment" this consolidation applies to
  the G11 fast-path specifically.
- **Phase 9b** (owner/admin + multimodal + cross-platform) and **Phase 10** (deferred
  optimizations: COOP/COEP, SAB, MESH/NODE viz tiers, RevocationSet wiring, real-device CI) — both
  outside this document's scope by the same fast-path boundary.
- **Implementation.** Every phase above is planning/blueprint detail; no product code, CI config,
  or canon file has been written or edited by this consolidation or any of the six source documents
  it merges.
- **The sovereign-architecture roadmap's own 19 phases** — a parallel, larger arc; §8.5 above notes
  the one real overlap point (Phase 16) and states the reconciliation explicitly rather than
  duplicating either document's content.

---

*Consolidates (superseding narrative duplication, preserving all falsifiable content): `BLUEPRINT-P00-dev-ci-deploy-enablement.md`,
`BLUEPRINT-P01-brand-token-pipeline.md`, `BLUEPRINT-P02-gpu-engine-foundation.md`,
`BLUEPRINT-P06-sea-sheet-backbone-event-stream.md`, `BLUEPRINT-P08-living-memory-viz-phase0.md` §2
(the Phase-5 slice only — the rest of that file remains the authoritative off-path Phase-8 blueprint
and is NOT deleted), `BLUEPRINT-P09A-order-critical-product-surface.md`. Independent 2-question
audit: `G11-FAST-PATH-2Q-AUDIT.md` (§9). Planning only — no code written; the branch for this arc's
implementation has not yet been created.*
