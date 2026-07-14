# Physics-GPU UI Engine + Capture-Redraw — Blueprint

> Status: **v1 (2026-07-14)**. Grounded in a five-lane research pass (quantum math language · GPU
> physics UI · capture→equations · codebase inventory) + a live read of both repos. Tags:
> **PROVEN** (measured / production-real) · **RESEARCHED** (sourced) · **SPECULATIVE / NOVEL**
> (design hypothesis). Companion: `internal-retrieval-living-memory-blueprint.md` (shares the same
> Laplacian operator + resolvent spine, §12) and `math-first-architecture-blueprint.md` (§1.5
> invariants). Directs a change to the existing `field-ui-engine` corpus's DOM decision (§4).

## 0. Thesis (one sentence)

**The UI is not a DOM tree — it is a FIELD:** one graph-Laplacian operator `L`, drawn entirely on
the GPU from equations — layout & motion via `MÜ+ΓU̇+c²LU=S`, blur & shadow via `e^{−tL}` (the
*same* heat kernel), shapes via signed-distance fields, text via public-domain outline math — with
`wgpu` as the *only* graphics dependency and no DOM; and any captured screen or video can be
redrawn from a fitted equation representation, where the redraw half is already solved and only the
fitting half is honestly heavy.

## 1. The grand unification (the extraordinary math, made honest)

**ONE Laplacian `L`, one spectral calculus `f(L)`, across FIVE subsystems** — the strongest form of
the math-first §1.5.5 spectral-waves invariant, and (per lane 2's search) a composition with **no
prior art**:

| subsystem | operator | filter |
|---|---|---|
| memory recall (retrieval §12) | resolvent `(I−αW)⁻¹` / heat `e^{−tL}` | `1/(1−αλ)` · `e^{−λt}` |
| salience decay (memory TTL) | heat kernel (degenerate) | `e^{−λt}` |
| **UI layout & motion** | damped field `MÜ+ΓU̇+c²LU=S` | roots `−γ/2±i√(ω²−γ²/4)` |
| **UI blur & shadow** | heat kernel `e^{−tL}` — Gaussian blur ≡ solving `∂u/∂t=α∇²u` (PROVEN same operator) | `e^{−λt}` |
| **UI ripple / interference** | quantum walk `e^{−iHt}` · `coherence \|ψ₁±ψ₂\|²` | `e^{−iλt}` (Tier-2, gated) |

So a **single GPU Laplacian-SpMV kernel** (CSR + semi-implicit Euler / Chebyshev, deterministic —
retrieval §12.3) serves memory recall, UI layout, UI motion, and UI blur simultaneously. The
resolvent/Green's-function is the honest unifying language (§12.1); quantum-walk interference is a
gated Tier-2 upgrade where ripple semantics genuinely add.

## 2. Ground truth (codebase inventory)

- **The field operator already exists in code** — `crates/bebop/src/field_physics.rs:334`
  `step_wave` is the discretized `MÜ+ΓU̇+c²LU=S` (`v̇=(c²L u+src−damp·v)/m`), with the Lyapunov
  energy certificate `wave_energy` (`dH/dt≤0`). `bebop2/core/src/field.rs` has the eigenmode
  spectrum + heat-kernel `propagate_spectral`; `chebyshev.rs` the matrix-free large-graph heat
  kernel; `coherence.rs` the `|ψ₁±ψ₂|²` interference. The plan spec is verbatim in
  `docs/design/field-ui-engine/FIELD-UI-ENGINE-PLAN.md:20-24`. **Gap:** it runs on abstract
  Platonic/graph "Body" objects, not on UI widgets.
- **The engine crate is pure-CPU, zero-dep** (`engine/Cargo.toml` marks wgpu explicitly out of
  scope). `motion.rs` ζ=1 critically-damped integrator (semi-implicit Euler, RED→GREEN
  `zeta_one_no_overshoot`); `loop_.rs` `DT_STABLE=0.02` (three-way mirror-pinned to `field.rs:26`
  + `kernel/lib.rs:53`); `widget_store.rs` SoA + `ParticlePool` ring; **`bridge.rs` `VertexBridge`
  zero-copy staging is built + tested ("0 JSON, 1 writeBuffer") but `upload_once()` only *counts* a
  hypothetical call — it never reaches a real GPU buffer.** That's the one "wire it" gap (FE-01).
- **`webgl/particle-cloud/particle-cloud.js`** — a working WebGL2/GLSL particle system (SoA ring,
  event→{color,energy,burst} VOCAB, one live blue-hardwire bug) = the exact `wgpu` port target
  (FE-04 / RW-04). **`web/`** today = a ~12-DOM-node + one 2D-`<canvas>` debug shell (not GPU).
- **ZERO wgsl / wgpu / WebGPU code exists** in either repo today (verified). `ffmpeg` present.
- **`bebop-repo/render-cast.py`** = a working **capture → reconstruct-state → redraw-from-state**
  pipeline (terminal `.cast` → `pyte` emulator → redraw each frame from *state*, not pixels → GIF)
  — the template for "redraw with equations" (§5).
- **Design corpus**: `field-ui-engine` (FE-01..17, the operator + SDF/MSDF specs), `dowiz-interfaces`
  (Sea & Sheet: Sea = the field operator, Sheet = SDF+MSDF brand), `rust-engine-rewrite` (RW-01
  `field-math` vendoring crate; RW-04 particle→wgpu; RW-05 single wasm-bindgen shell).

## 3. The zero-dep, GPU-native rendering stack (drawn from equations)

- **`wgpu` = the sole graphics dependency** (PROVEN): one pure-Rust crate that *is* the WebGPU
  implementation in Firefox/Servo/Deno, backs Bevy; native Vulkan/Metal/DX12 + browser WebGPU
  (shipped in all major browsers by late 2025). No C/JS UI toolkit, no widget library.
- **SDF shapes** (Inigo Quilez): a shape *is* an implicit `f(p)=`signed distance; boolean = min/max;
  organic blend = smooth-min; **analytic anti-aliasing** via `smoothstep(-w,w,-d)` with `w=fwidth(d)`
  (unit-gradient SDF → ~1px AA, zero MSAA). Rounded UI, icons, brand marks + *free* glow/blur/outline
  from distance-thresholding. PROVEN since Valve 2007.
- **Text without a font library**: TrueType (quadratic) / CFF (cubic) Bézier outline parse, then
  **Slug** (per-pixel exact winding from raw outlines — **patent went public-domain March 2026**) or
  **MSDF** (`median(r,g,b)` preserves corners) or Loop-Blinn. Latin/Cyrillic is well-trodden; **full
  Unicode shaping (Arabic/Indic) + bidi = HarfBuzz-scale → scope to Latin/Cyrillic first**, add
  shaping later.
- **Painting math** (all closed-form): Porter-Duff `C_out=C_src+C_dst(1−α)` (hardware-native);
  gradients (linear=`dot`, radial=`length`, conic=`atan2` — one-line WGSL); gamma-correct blending;
  **blur/shadow = the heat kernel** (§1 unification) or Kawase dual-filter for large radii; bloom.
- **The field operator as a compute pass**: `c²L U` = a **CSR-SpMV** over node buffers (one thread
  per node, gather over its contiguous neighbour run — no atomics); time integration = **semi-implicit
  (symplectic) Euler**, double-buffered pos/vel, fixed `DT_STABLE`; **ζ=1 critical damping =
  deterministic, monotone, no overshoot**. Reuse `field_physics.rs::step_wave` (vendored via RW-01's
  `field-math` crate) + `engine/` `bridge`/`loop_`/`motion`.

## 4. The honest "no DOM" cost + reconciliation (a directed change to the corpus)

**Conflict, stated plainly (not buried):** the existing corpus *explicitly* chose **hybrid** —
`BLUEPRINTS-FIELD-UI.md:17`: "Гібрид, не чистий GPU. Форми/a11y/SSR лишаються DOM — це архітектура,
не тимчасовість," and `FIELD-UI-ENGINE-PLAN.md:223` "Pure GPU app without DOM. Impossible with full
a11y in 2026." The operator now directs **pure, no-DOM**. Reconciliation:

- **Accessibility → AccessKit** (PROVEN path): a Rust-native library that bridges a **hand-maintained
  semantic tree** to the OS accessibility APIs (UIA / AT-SPI / NSAccessibility) **without a DOM** —
  exactly how egui and Bevy do a11y. This **supersedes** the corpus's DOM-a11y-mirror decision. Be
  honest: it is a real, ongoing engineering line-item (Figma's canvas "Mirror DOM" took ~3 years) —
  a11y is a first-class subsystem here, not an afterthought.
- **Text input / IME** → one **hidden native `<input>`** receiving OS focus + IME composition,
  proxied into the field (well-precedented for canvas editors). (This is the *only* residual native
  element — a single invisible input, not a DOM UI.)
- **Find-in-page / SEO** → accepted losses for the app surface; SSR a static text summary for SEO
  where it matters.

**Honest bottom line:** pure-no-DOM is *achievable*; the cost is the AccessKit bridge + Unicode-shaping
scope, **not** the rendering math (which is proven). 🔴 `money_guard` invariant preserved — `Money`
never becomes a field value / never tweens (`engine/src/money_guard.rs`).

## 5. Capture → equations → redraw

- **Capture = zero-dep native** everywhere (PROVEN): Windows WGC/DXGI, macOS ScreenCaptureKit,
  Linux Wayland-PipeWire portal / X11, browser `getDisplayMedia`. **YouTube/file ingestion needs
  ffmpeg/yt-dlp** (external) — not zero-dep; be honest.
- **Pixels → equations (the honest catch — fitting is offline + GPU-heavy):**
  - **DCT/FFT = the zero-dep, real-time FLOOR** — JPEG/MPEG *already* express any frame as a "sum of
    cosines"; deterministic, cheap, lossy. The honest baseline, not a strawman.
  - **2D Gaussian Splatting** (GaussianImage: 8 params/Gaussian, **1500–2000 fps *render***) and
    **SIREN** (an image literally = a `sin()` function `f(x,y)→RGB`) = the strongest "primitives not
    pixels" for a *still frame* — but **fitting is a seconds-to-minutes GPU autodiff loop** (PyTorch/
    CUDA-class), which is the **one genuine break from this project's zero-dep philosophy** → quarantine
    it as an **offline/optional** job.
  - **No feed-forward encoder exists for LIVE video** → per-incoming-frame fitting at 30 fps is
    **RESEARCH, not promised** (a research contribution, not an integration task).
- **Redraw = SOLVED, zero-dep GPU** (PROVEN): `web-splat` (Rust + wgpu/WGSL) renders pre-fit Gaussian
  splats >200 fps — directly our stack.
- **The field's own "video codec" (NOVEL):** follow `render-cast.py`'s "redraw from reconstructed
  *state*, not pixels" pattern — for our UI, **store the forcing `S(t)` (the event/impulse timeline),
  not frames**, and replay the `MÜ+ΓU̇+c²LU=S` integrator to regenerate the visual deterministically.
  The UI *is* its own equation-codec.
- **MVP staging (honest):** capture (zero-dep) → DCT redraw (zero-dep floor) → Gaussian-splat
  still-frame fit (offline, optional heavy dep) → [research] live-video fit.

## 6. Roadmap

**UI engine (U0…U8):**
- **U0** — `field-math` crate (RW-01): vendor bebop `field.rs`/`chebyshev.rs`/`fft.rs`/`algebra.rs`
  as `#![no_std]`+alloc, zero-dep, wasm32-clean. The operator gets a home in `engine/`.
- **U1** — Wire `VertexBridge` to a **real `wgpu::Device`/`Surface`/`Buffer` + one real `writeBuffer`**
  (closes the FE-01 gap — the contract & tests already exist).
- **U2** — Port `particle-cloud.js` → WGSL/wgpu (FE-04/RW-04); fix the blue-hardwire bug.
- **U3** — SDF shape pipeline (FE-05): `sdRoundBox` + smooth-min + `fwidth` AA + a design-token table.
- **U4** — Text: Slug/MSDF from parsed outlines (FE-06), Latin/Cyrillic scope.
- **U5** — The **field operator as a GPU compute pass**: CSR-SpMV `c²LU` + semi-implicit Euler,
  ζ=1, deterministic. Reuse `step_wave` math.
- **U6** — **Blur/shadow via the SAME `L` kernel** (`e^{−tL}`) — the unification realized in one shader.
- **U7** — AccessKit a11y bridge + hidden-input IME (§4) — the honest cost line-item.
- **U8** — Coherence `|ψ₁±ψ₂|²` interference visuals + optional CTQW ripple (Tier-2, gated).

**Capture (C0…C3):**
- **C0** native screen capture → **C1** DCT redraw floor (zero-dep, real-time) → **C2** Gaussian-splat
  still-frame fit (offline job, optional heavy dep) → **C3** field-state replay codec (store forces,
  not frames — the NOVEL path).

Every step: deterministic ζ=1 + fixed timestep; benchmark vs baseline; `money_guard` 🔴 preserved.

## 7. Guardrails

1. 🔴 `money_guard` — `Money` never a field value, never tweens (integer, jump-only).
2. Deterministic ζ=1 + fixed `DT_STABLE` + semi-implicit Euler (bitwise-reproducible motion).
3. A11y is a first-class subsystem (AccessKit), not an afterthought — the honest cost of "no DOM."
4. The capture-**fitting** GPU/ML dependency is the ONE sanctioned break from zero-dep — quarantine
   it as an offline/optional job; the **render**, **capture**, and **DCT** paths stay zero-dep.
5. **Live-video fit is NOT promised** — flagged research, not integration.
6. Reconciles with the corpus (FE-*/DZ-*/RW-*); the *only* superseded decision is DOM-a11y →
   AccessKit, and that change is operator-directed and cost-stated.

---

### Key sources
wgpu / WebGPU (gfx-rs, Bevy, Firefox/Servo) · Inigo Quilez 2D SDF + smin · Valve SDF text (Green
2007) · Chlumský MSDF · Lengyel Slug (public domain 2026-03) · Loop-Blinn 2005 · Linebender Vello /
font-rs · Porter-Duff 1984 · heat-equation ≡ Gaussian blur · Kawase/ARM dual-filter · Fruchterman-
Reingold force layout · symplectic Euler (Gaffer) · critical damping ζ=1 · AccessKit (egui/Bevy) ·
HarfBuzz/ICU + UAX#9 (shaping scope) · Kerbl 3DGS (SIGGRAPH 2023) · GaussianImage (ECCV 2024) · SIREN
(NeurIPS 2020) · NeRV · DiffVG · web-splat (Rust+wgpu) · WGC/DXGI/ScreenCaptureKit/PipeWire/getDisplayMedia
· Chung PNAS 2007 (resolvent) · Paparo-Martín-Delgado Quantum PageRank (2012). Codebase:
`engine/src/{bridge,loop_,motion,widget_store,money_guard}.rs`, `crates/bebop/src/{field_physics,
coherence,wavefield,stabilizer,geometry_field}.rs`, `bebop2/core/src/{field,chebyshev,fft}.rs`,
`webgl/particle-cloud/particle-cloud.js`, `bebop-repo/render-cast.py`, `docs/design/field-ui-engine/`,
`docs/design/dowiz-interfaces/`, `docs/design/rust-engine-rewrite/`.
