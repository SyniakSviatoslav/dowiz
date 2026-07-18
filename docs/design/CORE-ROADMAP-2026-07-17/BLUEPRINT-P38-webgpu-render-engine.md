# BLUEPRINT P38 — WebGPU render engine completion (P38a) + Sea & Sheet surfaces (P38b) (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9). This is DELIVERY's
> **P38** as scoped by `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.3: P38a
> absorbs FE-04(=RW-04)/05/06/07/10/11/12/13/14/15/16 + RW-01/05/10/11; P38b absorbs DZ-01..12.
> The FE/RW/DZ arcs (`docs/design/field-ui-engine/`, `docs/design/rust-engine-rewrite/`,
> `docs/design/dowiz-interfaces/`) are **reused, not re-derived** — this blueprint binds them to
> live code and to the standard; per-unit design detail stays in the arcs. P38b is deliberately
> the lighter §11: its 12 DZ units are already implementation-ready blueprints
> (`BLUEPRINTS-DOWIZ-INTERFACES.md:46-320`); re-deriving them here would violate standard
> item 19. Structural template: `BLUEPRINT-P-A-kernel-primitives.md` (numbering mirrored).

> **⚠ CANON-DIFF appended 2026-07-18 (see §12) — append-only, originals preserved.** This
> blueprint carries a dated correction block from `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md`
> (§0.2-2, §2 X2, §5 canon-diffs row **P38-rev**): (1) G6's transparent-`<input>` overlay is
> **struck** — text input is now P57's fully-custom in-canvas `cosmic-text` path (§12.1);
> (2) four AR/VR insurance constraints are added as **hard requirements** (§12.2); (3) FE-15
> (mirror) + FE-16 (fallback ladder) are reaffirmed as the shared base every new W1/W2/W3
> blueprint imports (§12.3). The original G6/overlay text below is preserved unaltered, with an
> inline SUPERSEDED marker at the point of the struck claim (§3.6).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads; drifts from the roadmap's claims noted.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| `compose()` — physics-state→RGBA renderer, real + tested | `engine/src/field_frame.rs:218` (`compose(scene, eq, w, h, steps) -> Vec<u8>`) | **VERIFIED — this is the oracle** |
| compose is **bit-deterministic**, proven by test | `engine/src/field_frame.rs:346` (`compose_returns_deterministic_frame`, asserts byte-equality `:366`), allocation-free variant pinned `:375-396`; "guaranteed bit-identical" doc `:102` | VERIFIED |
| Stability gate: `FieldEquilibrium::assert_stable` fail-closed CFL bound `dt ≤ M/(Γ+2c²)` | `engine/src/field_frame.rs:24-29`, `:59` | VERIFIED |
| `VertexBridge` — real CPU staging copy; `upload_to<S: GpuUploadSink>` = exactly ONE `write_buffer` per frame; `FrameProfiler` counters | `engine/src/bridge.rs:69` (struct), `:157` (`upload_once` staging), `:174-180` (`upload_to`), `:183-187` (JSON red-path counter) | VERIFIED |
| `new_gpu()` is an HONEST Err stub, gated `gpu = []` EMPTY feature, ONLY because wgpu is uncached | `engine/src/bridge.rs:248-260` (`Err("gpu adapter not built — wgpu uncached")`); `engine/Cargo.toml:23-32` (`gpu = []`), `:36` (`webgpu = []` also empty) | VERIFIED |
| Guard test pins the fail-closed boundary (default build must have NO real GPU adapter) | `engine/src/bridge.rs:214-218` (`e21_default_build_has_no_real_gpu_adapter`) | VERIFIED — stays green forever; the gpu feature is additive |
| O18a `graphics-unlock` gate status | `docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md:3` — "BLOCKED OFFLINE (wgpu uncached — verified 2026-07-16)"; trigger `:26` "operator grants network `cargo add wgpu`" | VERIFIED — hard, environment-gated, ONE-TIME, shared with P17 |
| SoA store + particle ring | `engine/src/widget_store.rs:14` (`WidgetStore`, `integrate` `:54`), `:68` (`ParticlePool`, `spawn` `:98`) | VERIFIED |
| Fixed-timestep loop | `engine/src/loop_.rs:29` (`FixedTimestep`, `frame` `:52`) | VERIFIED |
| Critically-damped motion | `engine/src/motion.rs:14` (`Spring`; presets `snappy/fluid/playful` `:66-76`; `zeta` `:80`; `heat_kernel_delay` `:87`) | VERIFIED |
| Money-never-tween guard LANDED and binding | `engine/src/money_guard.rs:50` (`TweenGuard`, `present_money` `:55`, `jump` `:65`) | VERIFIED — P38 must not weaken |
| SDF primitives + scene exist (CPU) | `engine/src/sdf.rs:19-152` (`sdf_circle/box/rounded_box/line_segment`, ops, `SdfField::rasterize`), `engine/src/scene.rs:29` (`SdfShape`), `:71` (`Scene`, `render_to_bridge` `:168`) | VERIFIED — FE-05's WGSL mirrors these, no redesign |
| Zero-copy substrate | `engine/src/zerocopy.rs:31` (`ParticleBuffer`), `:85` (`write_into_linear`), `:113` (`view_as_f32`), `:143` (`GpuSink` CPU mock) | VERIFIED |
| Kernel↔engine energy gate (Lyapunov leg) exists TEST-ONLY | `engine/src/field_energy.rs:1-16` (E1 seam gate; kernel `noether::lyapunov_nonincreasing` named `:4`; strict energy decrease measured `:161`) | VERIFIED — FE-14's watchdog reuses this, doesn't invent one |
| FE-07 spectral contract already documented at the bridge | `engine/src/bridge.rs:657-676` (kernel `spectral_flat_js` flat-array layout; fail-closed `None` keeps last good state) | VERIFIED |
| `wasm/src/lib.rs` is the WRONG shape for zero-copy: returns copied `Vec<u8>`/`Vec<f32>`; retrieval exports mixed in | `wasm/src/lib.rs:57` (`compose_field -> Vec<u8>`), `:96` (`frame() -> Vec<u8>`), `:112` (`vertex_field -> Vec<f32>`), `:153-168` (`knowledge_map`/`lookup_tag`/`related_docs` mixed in) | VERIFIED — RW-05/FE-01-caveat closure target |
| `web/src/app.mjs` — 204 lines, console-only, binds 24/24 kernel `_js` exports; own header defers DOM/FieldSim to "G3 … separate work unit" | `web/src/app.mjs:1-12`; `wc -l` = 204 | VERIFIED — deliberate first step, NOT throwaway |
| `web/package.json` — bare Node script runner (serve + one test), no toolchain | `web/package.json:7-10` | VERIFIED — RW-10 target |
| `particle-cloud.js` source is GONE — only a README survives | `webgl/particle-cloud/` contains ONLY `README.md` (listed this pass) | **VERIFIED — FE-04/RW-04 is reimplement-from-spec, not a port** |
| Zero glyph/MSDF code anywhere | `grep -rl "glyph\|msdf"` over `engine/ wasm/ web/` → 0 files | VERIFIED — FE-06 starts from nothing |
| FE-15/DZ-11's own ground truth: "**AccessKit no web backend 2026**"; mechanism = hidden DOM semantic mirror + input overlay | `docs/design/field-ui-engine/BLUEPRINTS-FIELD-UI.md:383-403`; `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md:304-318` | VERIFIED — reconciles §10.5.3's "AccessKit mirror" shorthand (§3.6) |
| DZ-01..12 all present as implementation-ready units; DZ-10 = Intent/FieldPos/InputSource + voice/gesture | `BLUEPRINTS-DOWIZ-INTERFACES.md:46,69,92,117,140,161,189,225,247,286,304,320` | VERIFIED — §11 links, doesn't restate |
| `Intent`/`FieldPos`/`InputSource` structs: 0 grep hits in code | repo-wide grep this pass | VERIFIED — P38b DoD-1 baseline |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope — what P38 owns vs deliberately does NOT (§10.5.3 anti-scope, sharpened)

**P38a owns (build items §3):**

| Item | Arc unit(s) | Content |
|---|---|---|
| G1 | O18a + FE-01 close | `wgpu` unlock; real `WgpuSink`/`new_gpu()`; headless blit pipeline whose pixel readback matches `compose()` **bit-for-bit** |
| G2 | FE-04/RW-04 | Particle renderer reimplemented-from-spec against `widget_store`/`loop_`/`motion` (source is gone — §0) |
| G3 | FE-05 + FE-06 | SDF instanced-quad pipeline + GPU design-token table; MSDF glyph atlas + text draw |
| G4 | FE-07/10/11/12/13 | Field-dynamics quartet+1, each with a deterministic test against kernel math |
| G5 | FE-14 | Lazy-render-on-settle (the battery lever) with hysteresis + Lyapunov watchdog |
| G6 | FE-15 + FE-16 | A11y semantic mirror + input overlay (invisible DOM); functional WebGL2/CPU fallback flags |
| G7 | RW-05/01/10/11 | wasm ptr/len boundary (closes FE-01's caveat), retrieval-export separation, `dowiz-engine` workspace, web toolchain |

**P38b owns (§11, light):** DZ-01..12 binding — Sea (ambient-field client surface) + Sheet
(brand-SDF) on P38a's pipelines, wired to real order data via P37's wire or the F12 local path.

**P38 explicitly does NOT own:**

- **NOT a DOM-first redesign** — the UI is a WebGPU/WASM render of kernel physics-field state
  (canon). DOM survives ONLY as the invisible a11y mirror + input overlay (FE-15/DZ-11). Any
  visible DOM widget is a scope violation, falsified by §3.6's zero-visible-DOM assertion.
- **NOT touching the landed math substrate** — `compose`/`zerocopy`/`widget_store`/`loop_`/
  `motion`/`money_guard` are done and tested (§0). P38 consumes them; a diff that edits their
  arithmetic is out of scope (their existing bit-determinism tests are the tripwire).
- **NOT money tweening** — FE-09's guard (`money_guard.rs:50`) is landed and binding; money
  values render via `TweenGuard::present_money`/`jump` only. DZ-02's no-tween rule extends this
  to P38b. No interpolation path may accept a money value (type-level: `Money` implements no
  `FieldValue`).
- **NOT the physics authority on GPU** — the kernel/engine CPU path REMAINS the bit-deterministic
  state authority; the GPU is presentation (§3.1's honesty split). Migrating the field *step*
  to compute shaders is a named future decision (§4.2), not this phase.
- **NOT resurrecting Whisper voice / gesture ahead of the order path** — DZ-10 stays at its
  arc's own Phase-9b deferral (§10.5.3: "pulling them forward is a scope violation, not
  initiative").
- **NOT a second design-token system** — FE-05's GPU token table is the single source (P38b
  binding; DZ-02 maps onto it).
- **NOT deployment/ops** — P45's job, unchanged.

**Hard gate, stated honestly:** every GPU-executing item (G1-G4 GPU legs, G6 fallback's GPU
side) is blocked on O18a (`cargo add wgpu` + companion crates — network, operator-granted,
verified RED 2026-07-16, §0). What is NOT blocked and lands first: all CPU-side specs, types,
RED tests marked `#[ignore = "O18a"]`, G5 (pure CPU), G6's mirror (DOM-side), G7 (structural).
The unlock's exact crate list is pinned in §2 so the one network grant is a single event, not
a drip.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── O18a unlock manifest (ONE network grant, all together) ──────────────────
// wgpu            — the GPU API (feature `gpu` stops being empty, Cargo.toml:23-32)
// cosmic-text     — shaping for FE-06 (engine/Cargo.toml:8 names it out-of-scope-until-now)
// (naga ships inside wgpu; NO other graphics dep is authorized by this manifest)

// ── engine/src/gpu.rs — NEW module, #[cfg(feature = "gpu")] (G1) ────────────
/// Real GPU sink behind the EXISTING trait (bridge.rs:174 consumes it unchanged).
pub struct WgpuSink { device: wgpu::Device, queue: wgpu::Queue, vbuf: wgpu::Buffer }
impl GpuUploadSink for WgpuSink { fn write_buffer(&mut self, offset: u64, data: &[f32]); }
/// Headless bring-up: instance→adapter→device with NO surface (CI-runnable).
pub fn headless() -> Result<WgpuSink, GpuUnavailable>;
/// Typed absence — the degrade-not-crash arm (§4.1). NEVER panics.
pub struct GpuUnavailable(pub &'static str);

/// G1 oracle pipeline: upload an RGBA frame (compose()'s exact bytes) as a
/// texture, draw one fullscreen quad with NEAREST sampling, read pixels back.
/// Integer texel transport end-to-end — bit-identity is achievable and REQUIRED.
pub fn blit_roundtrip(frame_rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, GpuUnavailable>;

// ── G2: particle SoA → GPU buffer mapping (the layout, fixed BEFORE code) ───
/// One particle instance = 4 f32 (x, y, vx, vy) — EXACTLY ParticleBuffer's
/// stride (zerocopy.rs:31-73; STRIDE_F32 = 4). vx/vy drive motion-blur length
/// in the shader; life/color ride a parallel u32 buffer from ParticlePool.
pub const PARTICLE_STRIDE_F32: usize = 4;          // must equal zerocopy layout — asserted in test
pub const PARTICLE_BUDGET: usize = 10_000;          // §4.2 scaling axis; §6 frame budget assumes this

// ── G3: FE-05 token table + rect instance (GPU-side shapes of the arc's spec) ──
/// RectInstance — flat, 64-byte aligned; matches BLUEPRINTS-FIELD-UI.md:186-188.
#[repr(C)] pub struct RectInstance {
    pub rect: [f32; 4],          // x, y, half_w, half_h
    pub corner_radii: [f32; 4],  // per-corner (quadrant select in WGSL)
    pub fill: u32, pub border: u32,      // RGBA8 packed — color-mix PRE-RESOLVED on CPU
    pub border_w: f32, pub gradient: u32,
    pub shadow: [f32; 2], pub flags: u32, pub _pad: u32,
}
/// Bind group 0 UBO — theme switch = ONE uniform write (arc gate, FIELD-UI:187).
#[repr(C)] pub struct FrameUniforms {
    pub screen: [f32; 2], pub dpr: f32, pub time: f32,
    pub theme_tokens: [[f32; 4]; TOKEN_SLOTS],
}
pub const TOKEN_SLOTS: usize = 64;   // 11 brand + 40 status + 8 semantic + headroom (arc counts, FIELD-UI:172)
pub const SDF_PIXEL_DIFF_MAX: f64 = 0.02;  // ≤2% of pixels may differ CPU-raster vs GPU-AA (G3 gate; §3.3 why not bit)

// ── G5: FE-14 settle predicate (pure, CPU) ──────────────────────────────────
pub struct SettleGate { pub epsilon: f32, pub hysteresis_k: u8 }   // K = 3 (arc, FIELD-UI:371)
pub const SETTLE_EPSILON: f32 = 1e-4;      // max SoA step delta; same ε family as field active_diffuse
/// should_render = input_pending || !settled(K) || animation_active || external_change.
/// Watchdog: energy increase (field_energy.rs law) ⇒ force render, NEVER dormant.
pub fn should_render(g: &SettleGate, s: &FrameSignals) -> bool;
pub struct FrameSignals { pub input_pending: bool, pub max_step_delta: f32,
                          pub animation_active: bool, pub external_change: bool,
                          pub energy_delta: f32 }

// ── G6: a11y mirror node (DOM-side shape, mirrored per frame) ───────────────
// web/src/lib/a11y_mirror.mjs — reconciles from the widget list each frame:
//   { role: 'button'|'status'|'listitem'|…, label: string, rect: [x,y,w,h],
//     focusable: bool, state: string }        // order state rides role='status'
// Mirror root style: position:fixed; clip-path:inset(50%); (screen-reader-only,
// zero painted pixels) — the standard SR-only technique, asserted in test.

// ── G7: wasm ptr/len boundary (RW-05 — replaces the copied-Vec returns) ─────
#[wasm_bindgen] pub fn frame_ptr(sim: &FieldSim) -> *const u8;   // view into wasm linear memory
#[wasm_bindgen] pub fn frame_len(sim: &FieldSim) -> usize;
#[wasm_bindgen] pub fn vertex_ptr(bridge: &SimBridge) -> *const f32;
#[wasm_bindgen] pub fn vertex_len(bridge: &SimBridge) -> usize;
// JS side: new Uint8Array(wasm.memory.buffer, frame_ptr(s), frame_len(s)) — ZERO copy.
// Invalidation rule (the hazard, §4.1): views are frame-scoped; any wasm call may
// grow memory and detach the buffer — JS re-derives the view EVERY frame, never caches.
```

Rejected alternatives (DECART one-liners): **GPU compute for the field step now** — rejected:
breaks the bit-deterministic CPU authority (WGSL float contraction is implementation-permitted;
the oracle would die) — GPU is presentation until a separately-gated decision (§4.2).
**egui/iced as the widget layer** — rejected: DOM-free canon is satisfiable directly on wgpu;
a retained-widget framework re-imports the abstraction the field UI replaces. **accesskit
crate** — rejected for web: no web backend (arc ground truth, §0); the DOM mirror IS the
AccessKit-role-tree pattern by hand, revisit only if accesskit ships a web adapter.

---

## 3. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 3.1 G1 — wgpu + real `new_gpu()` + the bit-oracle, with the honesty split

Two claims, separated because they have different proof classes:

1. **Transport bit-identity (REQUIRED, bit-for-bit):** `blit_roundtrip` (§2) feeds
   `compose()`'s RGBA bytes through texture-upload → fullscreen-quad draw (nearest sampling,
   no blending, no sRGB conversion — raw `Rgba8Unorm`) → readback. Integer texels end-to-end:
   the GPU never does float math on the payload, so byte-equality is a fair demand.
   RED test `gpu_blit_roundtrip_matches_compose_oracle` (`#[ignore = "O18a"]` until unlock):
   `assert_eq!(readback, compose(...))` on the P₃-scene fixture from
   `field_frame.rs`'s own test corpus. This is §10.5.3 DoD-1, made precise: the oracle checks
   the GPU **path carries frames losslessly**, not that GPUs do IEEE math like CPUs do.
2. **`VertexBridge::new_gpu()` real:** flip the stub (`bridge.rs:253-260`) to build a
   `WgpuSink`; `upload_to` (`:174`) needs ZERO changes — the trait was built for this.
   Test: `write_buffer_calls == 1` per frame against the real sink (the FrameProfiler
   discipline, already asserted CPU-side, now on GPU).

**Adversarial:** (i) adapter absent (`headless()` Err) → typed `GpuUnavailable`, caller falls
back per §3.6's ladder, NO panic — test forces the Err arm; (ii) `e21_default_build_has_no_
real_gpu_adapter` (`bridge.rs:214`) stays green — the default build still has no GPU;
(iii) a deliberately-wrong texture format (`Rgba8UnormSrgb`) must FAIL the oracle (teeth
proof: sRGB conversion corrupts bytes — run once, assert ≠, restore).

### 3.2 G2 — particle renderer, reimplemented from spec (FE-04/RW-04)

The source `particle-cloud.js` no longer exists (§0) — the spec is FE-04's blueprint
(`FIELD-UI:150-167`) + the engine's own SoA machinery. Pipeline: `ParticlePool::spawn` →
`WidgetStore::integrate(dt, friction)` inside `FixedTimestep::frame` → pack into
`ParticleBuffer` (stride pinned by `PARTICLE_STRIDE_F32` — a `const _: () = assert!` ties it
to the zerocopy layout at compile time) → `VertexBridge::upload_to(WgpuSink)` → ONE instanced
draw (point-quad, vx/vy as motion-blur axis per the arc).

RED→GREEN: (i) `particles_cpu_determinism` — two runs, same seed/steps ⇒ vertex buffer
bit-identical (CPU side, lands pre-unlock); (ii) `particles_single_upload_per_frame` —
profiler counters (works against `GpuSink` mock today, `WgpuSink` after unlock);
(iii) `particles_draw_smoke` (`#[ignore = "O18a"]`) — headless render N=100 particles,
readback: ≥ N non-background pixels within the particles' bounding boxes.
**Adversarial:** spawn at pool capacity + 1 (`ParticlePool` ring must overwrite oldest, never
grow/panic — asserts `capacity()` invariant); dt spike 10× (FixedTimestep clamps —
`max_seen_dt` bounded); NaN velocity injected → integrate must not propagate NaN into the
vertex buffer (assert `is_finite` post-integrate; today's behavior pinned RED-first).

### 3.3 G3 — SDF pipeline + GPU token table (FE-05) and MSDF text (FE-06)

FE-05's WGSL (`sdRoundBox` + analytic `fwidth` AA + erf7 shadow, `FIELD-UI:178-184`) mirrors
the CPU `sdf.rs::sdf_rounded_box` (`sdf.rs:41`) — the CPU rasterizer (`SdfField::rasterize`,
`:124`) is the reference. **Why NOT bit-for-bit here (honesty):** GPU analytic AA (`fwidth`)
is mathematically different from CPU feathering; the arc's own gate is "pixel-diff <
threshold" (`FIELD-UI:189`). Pinned: `SDF_PIXEL_DIFF_MAX` (§2), plus a **zero-diff interior
band** requirement — pixels farther than 1px from any edge must be byte-identical (only the
AA fringe may differ; this stops threshold-hiding of real defects).

- Token table: `color-mix` pre-resolved CPU-side into `FrameUniforms.theme_tokens`;
  theme/tenant switch = ONE `queue.write_buffer` of the UBO — test asserts exactly one buffer
  write and zero re-tessellation on switch (arc gate, `FIELD-UI:190`).
- FE-06: cosmic-text shaping → MSDF atlas (generated offline into a committed asset — atlas
  determinism test: same font+charset ⇒ same atlas bytes) → instanced glyph quads through the
  SAME RectInstance path (one pipeline family, no bespoke text renderer).

RED tests (`#[ignore = "O18a"]` until unlock): `sdf_storefront_card_pixel_gate`,
`sdf_theme_switch_single_ubo_write`, `msdf_atlas_deterministic`, `msdf_draw_smoke`.
**Adversarial:** corner radius > half-extent (WGSL must clamp exactly as `sdf.rs:41` does —
divergence shows in the interior band); glyph absent from atlas → tofu box from a reserved
atlas slot, NEVER a panic/skip (fail-visible, not fail-silent); token index out of
`TOKEN_SLOTS` range → compile-time const assert on the CPU side, clamp in WGSL.

### 3.4 G4 — field-dynamics quartet+1 (FE-07/10/11/12/13), each pinned to kernel math

Per unit, one deterministic test against the kernel-side truth (the engine never re-derives
math — `bridge.rs:657-676` contract):

| Unit | Lands as | Deterministic test (vs kernel) |
|---|---|---|
| FE-07 layout field | stress-majorization step over `WidgetStore` positions | monotone stress decrease per iteration on a fixed 6-widget fixture; jitter = 0 at convergence (positions bit-stable across 2 extra iterations) |
| FE-10 feedback | Green's-function impulse response driving `ParticlePool` bursts | impulse at node i ⇒ response decays with graph distance per `heat_kernel_delay` (`motion.rs:87`) — ordering asserted, not approximated |
| FE-11 focus wells | potential-well field summed into the layout force | well at focus ⇒ neighbors displace toward it, non-focus unaffected beyond ε; removing the well restores baseline bit-exactly |
| FE-12 spectral embed | φ₂/φ₃ from kernel `spectral_eigenvalues_js`/flat contract | embedding matches kernel eigenvector output to 1e-12; kernel `None` ⇒ engine keeps last good state (fail-closed arm of `bridge.rs:675` TESTED, not just documented) |
| FE-13 constraint solver | exact align/equal-width post-pass (measure-zero, not field) | constrained pairs EXACTLY equal (bit), unconstrained widgets unmoved |

All five are CPU-side (land pre-unlock); their render legs ride G2/G3 pipelines.
**Adversarial (one per unit, arc-sourced):** FE-07 disconnected component (must not collapse
to origin); FE-10 simultaneous impulses (superposition, no double-free of pool slots); FE-11
two equal wells (symmetric equilibrium, no oscillation — damped by `Spring` ζ); FE-12
degenerate spectrum (λ₂ ≈ λ₃ — embedding must be stable-or-explicitly-refused, not flickering);
FE-13 contradictory constraints (typed refusal, layout keeps field solution).

### 3.5 G5 — lazy-render-on-settle (FE-14, the battery lever; pure CPU, no gate)

`should_render` (§2) with hysteresis K=3 and the Lyapunov watchdog: `energy_delta > 0`
(energy increase = divergence, per the E1 energy law `field_energy.rs:8-10`) forces rendering
regardless of settle — divergence is never invisible. RED→GREEN (arc gate, `FIELD-UI:377`):
`settle_stops_frame_callbacks` — drive a scene to equilibrium, assert frame callbacks cease
within K ticks of `max_step_delta < SETTLE_EPSILON` (RED today: no gate exists, callbacks are
unconditional); `touch_wakes_instantly` — input during dormancy ⇒ render on the SAME tick.
**Adversarial:** `divergence_never_dormant` — artificially pump energy (the broken-integrator
trick `field_energy.rs:10` already uses) and assert the gate NEVER sleeps while energy rises;
oscillating-at-threshold signal (delta alternating ±ε) must not flap (hysteresis proof:
≤ 1 state change per K ticks).

### 3.6 G6 — a11y mirror + input overlay (FE-15) and functional fallbacks (FE-16)

**Reconciliation, stated once:** §10.5.3 says "invisible AccessKit mirror"; the arc's own
verified ground truth is "AccessKit no web backend 2026" (§0). Resolution: the *mechanism* is
FE-15/DZ-11's hidden DOM semantic mirror (the AccessKit-style role tree, hand-reconciled),
which SATISFIES the roadmap's intent (screen-reader tree exposes order state, zero visible
DOM); the accesskit crate is a named future swap-in if a web backend ships. No contradiction
propagated.

- Mirror: `web/src/lib/a11y_mirror.mjs` reconciles the §2 node shape from the widget list
  per frame (only on list *change* — piggybacks G5's settle gate, dormant UI = dormant mirror).
- Overlay: transparent `<input>` (type=email/tel preserved) for IME/autofill/mobile keyboard
  (arc spec `FIELD-UI:391-395`).
  > **SUPERSEDED 2026-07-18 — see §12.1 (CANON-DIFF).** This transparent-`<input>` overlay is
  > **STRUCK** by operator ruling (SYNTHESIS §0.2-2): Wave-0 text input is Latin+Cyrillic ONLY,
  > entered fully custom in-canvas via `cosmic-text` — **no DOM `<input>` exists on any
  > platform** (enforces MASTER-ROADMAP §16.34 in canon). IME-composition scripts
  > (Arabic/CJK/Thai/Indic) are deferred to v2 alongside §16.58's RTL-deferred-to-v2 ruling. The
  > bullet text above is preserved for the record; the replacement is blueprint **P57** (canvas
  > text input & editing). §3.6's remaining Mirror/FE-16 legs and the a11y-tree proof are
  > unaffected — only the input-overlay leg is removed (web live-edit a11y becomes P58's
  > synthetic ARIA-textbox, per §12.1).
- FE-16: the empty `webgpu = []`/fallback flags (`engine/Cargo.toml:36`) become functional:
  ladder = WebGPU → WebGL2 → CPU `compose_field` + canvas2d `putImageData` (the wasm export
  exists TODAY, `wasm/src/lib.rs:57` — the floor is already built, the ladder just has to
  reach it).

**Verified how (the §10.5.3 "verified how" answered with a real assertion, not "should
work"):** Playwright spec `web/tests/a11y-mirror.spec.mjs` — (1) drive an order to DELIVERED
via the local wasm path; (2) take the **accessibility-tree snapshot** (Playwright's ARIA
snapshot API) and assert a `role=status` node whose accessible name contains the order id and
"DELIVERED"; (3) assert zero visible UI DOM: every element except `<canvas>`, the clip-path
mirror root, and the overlay input has no painted box (`getBoundingClientRect` empty or
`visibility:hidden`), and the mirror root itself paints 0 pixels (clip-path inset check);
(4) keyboard: Tab reaches the mirror's focusable nodes in widget order. **Adversarial:**
remove one widget from the field list mid-test ⇒ its mirror node is gone by next reconcile
(stale-node leak check); force `navigator.gpu = undefined` ⇒ CPU-floor renderer draws AND the
a11y assertions still pass unchanged (a11y is renderer-independent by construction).

### 3.7 G7 — structural closure: RW-05 ptr/len, RW-01 workspace, RW-10 toolchain, RW-11

- **RW-05 + FE-01 caveat:** replace the copied-`Vec` exports (§0) with the §2 ptr/len pairs +
  frame-scoped JS views; retrieval exports (`knowledge_map`/`lookup_tag`/`related_docs`,
  `wasm/src/lib.rs:153-168`) move to a separate `wasm-retrieval` module/crate so the render
  wasm stays render-only. RED: `zero_copy_frame_no_alloc` — JS-side test asserts the frame
  view's buffer IS `wasm.memory.buffer` (no copy) and that byte content equals the old
  `frame()` output (parity pin before the Vec exports are removed). **Adversarial (the
  detached-buffer hazard, §2):** grow wasm memory between view creation and read ⇒ test
  asserts the STALE view throws/is-detached and the re-derived view is correct — proving the
  "re-derive every frame" rule is load-bearing, not superstition.
- **RW-01:** root `Cargo.toml` workspace unifying kernel/engine/wasm/tools (path-deps today,
  §0 P-A precedent); acceptance = one `cargo test --workspace` runs all three.
- **RW-10:** `web/package.json` graduates from bare scripts (§0) to a real toolchain (build +
  test + wasm-pack invocation pinned); no framework import — toolchain ≠ framework.
- **RW-11:** satisfied by construction — the view layer G2/G3 build IS wgpu-native from day
  one; the DoD line is "zero interim DOM view exists to migrate" (grep gate: no
  `createElement` outside `a11y_mirror.mjs` + overlay module).

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

- **GPU-absent devices (the §10.5.3 hazard question):** absence is a typed value
  (`GpuUnavailable`), not an exception path — the fallback ladder (§3.6) terminates in the
  CPU floor that exists today (`compose_field`), so "no adapter ⇒ no UI" is unreachable: the
  floor is reached by construction, and the forced-absence test falsifies it. Degrade, never
  crash; order flow is renderer-independent (kernel decide/fold has no GPU dependency at all).
- **Frame corruption:** the transport is bit-audited (G1 oracle); the only lossy layer is
  SDF AA, bounded by the interior-band rule (§3.3) — silent whole-frame drift is
  unrepresentable under the two tests together.
- **Detached wasm views:** the re-derive-per-frame rule is enforced by an adversarial test
  (§3.7), turning a heisenbug class into a deterministic RED.
- **Money on screen:** `Money` implements no `FieldValue` ⇒ a money tween does not compile
  (`money_guard.rs` landed law) — P38 adds the render-side assertion that money glyphs update
  only via `jump`.
- **Divergent physics burning battery or hiding:** the Lyapunov watchdog makes divergence
  force-visible (§3.5) — the failure mode "field exploded while renderer slept" is unreachable
  while the energy law holds, and the energy law itself is the tested E1 seam
  (`field_energy.rs`).

### 4.2 Schemas for scaling (item 8)

Stated axes: **particles/frame** (`PARTICLE_BUDGET` = 10⁴; §6 budget assumes it; break point
~10⁵ where CPU integrate exceeds frame budget → the named-not-taken decision: field/particle
step to GPU compute, which would REVISIT the §3.1 honesty split — an operator-visible
architecture decision, not a silent migration), **rect instances/frame** (10³ typical screen;
instanced single-draw holds to ~10⁵), **glyph atlas** (one page 2048², ~4k glyphs; second page
= the named growth step), **token slots** (`TOKEN_SLOTS` 64; >64 = UBO→storage-buffer switch,
one-line WGSL change, named). Mirror reconcile is O(widgets), widgets ≤ 10² — no axis.

### 4.3 Isolation / bulkhead (item 11) + error-propagation gates (item 14)

The render stack is a **consumer** of kernel state, never a mutator — a renderer panic cannot
corrupt an order (no shared mutable state across that boundary; wasm views are read-only).
`gpu` stays a cargo feature: the default build compiles zero GPU code (`e21` guard, §0), so a
wgpu regression cannot break kernel/engine CI lanes. Named CI gates per bug class: bit-oracle
(transport), interior-band (SDF), stride const-assert (layout drift), no-visible-DOM grep +
Playwright assertion (canon drift), detached-view test (memory hazard), `e21` (feature-bleed).

### 4.4 Mesh awareness (item 12)

Entirely **node-local** — P38 renders local kernel state; order data arrives via P37's wire or
the F12 local path (P38b DoD-2). Zero transport payloads originate here. The one mesh-adjacent
budget: FE-12 consumes kernel spectral output in-process (flat f64 array, `bridge.rs:662`),
not over any wire.

### 4.5 Rollback / self-healing vocabulary (item 13, used precisely)

- **Self-Termination leg claimed:** typed `GpuUnavailable`, compile-time stride/token asserts,
  un-compilable money tween, CFL `assert_stable` fail-closed (`field_frame.rs:24-29`, already
  landed).
- **Self-Healing leg claimed narrowly:** the fallback ladder is genuine redundancy (three
  independent render paths to the same frame truth; the CPU floor is the error-correcting
  term) — claimed ONLY for rendering, not for state.
- **Snapshot-Re-entry: NOT claimed.** Frames are derived, never stored (memory arc: store
  forces S(t), not frames); recovery = recompute from kernel state, which is re-derivation,
  not snapshot restore.
- Mechanical rollback: the `gpu` feature OFF restores today's exact build (guard-tested);
  every G-item is feature- or module-additive.

### 4.6 Living memory (item 15) + tensor/spectral (item 16) + Linux discipline (item 9)

Item 15: the settle gate gives UI state a temporal access pattern (hot while moving, dormant
at rest — demote-never-delete applied to compute); frames are recall-by-recompute from S(t),
per `physics-ui-capture-quantum-math-arc-2026-07-14`. Item 16: FE-12 REUSES kernel
`spectral.rs` output (no engine-side eigen-math — the bridge contract forbids it); closed-form
pieces (erf7 shadow, sdRoundBox) mirror already-tested CPU forms rather than inventing new
math; eqc-rs applies if/when any NEW closed-form organ appears (none does in this phase —
stated, not decorative). Item 9 verdicts: **ALREADY-EQUIVALENT** — one pipeline family for
rects+glyphs (one concept, one primitive); **REINFORCES** — feature-gated hardware access with
a software floor (the kernel-module discipline); **EXTENDS** — the interior-band pixel gate is
a new discipline this repo adds for GPU-vs-CPU comparisons; **GAP** honestly named — no GPU CI
runner exists; gpu-feature tests run on developer/operator hardware until P45 provides one
(each `#[ignore = "O18a"]` doubles as the marker).

---

## 5. DoD — falsifiable, RED→GREEN, extending §10.5.3's P38a items (item 2)

| §10.5.3 DoD | Named test(s) | Permanent regression (item 17) |
|---|---|---|
| 1. wgpu added; `new_gpu()` real; headless readback matches compose oracle | `gpu_blit_roundtrip_matches_compose_oracle` (bit), sRGB teeth-proof (§3.1) | oracle test + `e21` guard |
| 2. Particle renderer from widget_store/loop_/motion | `particles_cpu_determinism`, `particles_single_upload_per_frame`, `particles_draw_smoke` (§3.2) | determinism + profiler tests |
| 3. FE-05 SDF+tokens; FE-06 MSDF | `sdf_storefront_card_pixel_gate` (+ interior band), `sdf_theme_switch_single_ubo_write`, `msdf_atlas_deterministic` (§3.3) | all three |
| 4. FE-07/10/11/12/13 each with a deterministic kernel-math test | the five §3.4 table tests + five adversarial arms | all ten |
| 5. FE-14 settle | `settle_stops_frame_callbacks`, `touch_wakes_instantly`, `divergence_never_dormant` (§3.5) | all three |
| 6. FE-15 a11y (zero visible DOM) + FE-16 functional fallbacks | `web/tests/a11y-mirror.spec.mjs` 4-assertion set + forced-`GpuUnavailable` degrade test (§3.6) | Playwright spec + degrade test |
| 7. RW-05 ptr/len (FE-01 caveat closed); RW-01 workspace; RW-10 toolchain; RW-11 no-DOM-view | `zero_copy_frame_no_alloc` + detached-view adversarial; `cargo test --workspace`; createElement grep gate (§3.7) | all |

Ledger rows (`docs/regressions/REGRESSION-LEDGER.md`, ratchet rule applies): G1 oracle, G3
interior-band, G5 divergence-watchdog, G7 detached-view. **Not-done clauses:** an `#[ignore =
"O18a"]` test silently deleted instead of un-ignored at unlock = NOT done; raising
`SDF_PIXEL_DIFF_MAX` or shrinking the interior band to pass = NOT done; any visible DOM widget
= NOT done regardless of green totals.

---

## 6. Benchmark plan (item 10) — frame budget + the FE-14 target, measured

Budgets (mid-tier device class, the WebGL2-primary budget-device canon from the
splatting arc): **16.6 ms/frame @60fps** split: CPU integrate+pack ≤ 2 ms at
`PARTICLE_BUDGET`, encode+submit ≤ 2 ms, SDF+text draw ≤ 4 ms GPU-time, mirror reconcile
≤ 0.5 ms (change-frames only); WebGL2 floor: 33 ms @30fps acceptable. **FE-14 target (the
headline): 0 rAF wake-ups per second on a settled screen** — measured, not "reduced".

Mechanism: criterion benches `engine/benches/frame.rs` (`particles_integrate_10k`,
`bridge_upload_10k`, `sdf_pack_1k_rects`) — added RED-commit-first so the bench_track baseline
auto-seeds (P-A §6 discipline, same `BENCH_HISTORY.md` append rule); GPU-time via wgpu
timestamp queries recorded when the gpu feature runs (operator hardware until a GPU runner
exists — §4.6 gap). The settle target is asserted by `settle_stops_frame_callbacks` with a
counted 1-second window (0 callbacks), which doubles as the CI tripwire — the benchmark IS a
test, not a report.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.3 (scope
authority) · `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`docs/design/field-ui-engine/BLUEPRINTS-FIELD-UI.md` + `FIELD-UI-ENGINE-PLAN.md` (FE-04..16
unit specs — the design detail this doc deliberately does not restate) ·
`docs/design/rust-engine-rewrite/BLUEPRINTS-RUST-ENGINE-REWRITE.md` (RW-01/04/05/10/11) ·
`docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md` (DZ-01..12, §11) ·
`docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md` (O18a gate record) ·
`docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P16-product-ui-rebuild.md` (P16 = the
home this phase fills) · `BLUEPRINT-P37-order-http-surface.md` (sibling; independent, §10.5.3
— the two proceed in parallel) · `docs/regressions/REGRESSION-LEDGER.md` ·
`BLUEPRINT-P-A-kernel-primitives.md` (template). Memory:
`field-ui-engine-arc-2026-07-13` · `rust-engine-rewrite-arc-2026-07-13` ·
`physics-ui-capture-quantum-math-arc-2026-07-14` (ONE Laplacian; store S(t) not frames) ·
`dowiz-interfaces-design-arc-2026-07-13` · `dowiz-brand-voice-canon-2026-07-07` (Sheet skin) ·
`gaussian-splatting-address-picker-arc-2026-07-16` (WebGL2-primary budget-device canon) ·
`test-integrity-rules-2026-06-27` (money red-line). Supersedes: nothing — binds the arcs to
live code under the standard.

---

## 8. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source, code derived): §2's layouts/constants and the arc specs
  precede every implementation; the WGSL mirrors the CPU reference forms, never free-hands.
- **P2 CORRESPONDENCE** (one concept, one primitive): one instanced pipeline family for rects
  AND glyphs (§3.3); one settle predicate serving render, mirror, and battery (§3.5/§3.6);
  one frame truth (`compose`) refereeing every render path.
- **P6 CAUSE-AND-EFFECT** (determinism as law): bit-oracle (G1), CPU determinism (G2), atlas
  determinism (G3), 1e-12 spectral pin (G4) — every determinism claim has a falsifier.
- **P7 GENDER** (paired verification): the GPU path is refereed by the independent CPU path at
  three layers (blit oracle, SDF interior band, fallback-parity in the degrade test); the
  a11y mirror is refereed by the accessibility tree, not by its own reconcile code.

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; roadmap-claim drifts noted, incl. AccessKit reconciliation) |
| 2 DoD | §5 (extends §10.5.3 P38a items 1-7 with named tests) + §11.3 (P38b) |
| 3 spec/event-driven TDD | §2 spec-first; §3 RED-first per item; §3.5/§3.6 assert on event/callback sequences |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §3.1–§3.7 (≥1 per item: sRGB teeth, NaN velocity, capacity+1, contradictory constraints, energy pump, stale mirror node, detached view) |
| 6 hazard-safety as math | §4.1 (typed absence, unreachable states, un-compilable money tween) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 (each with named break point) |
| 9 Linux discipline | §4.6 (all four verdict categories, incl. an honest GAP) |
| 10 benchmarks+telemetry | §6 (budgets, bench_track seeding, benchmark-as-test tripwire) |
| 11 isolation/bulkhead | §4.3 |
| 12 mesh awareness | §4.4 (node-local, stated with the one in-process budget) |
| 13 rollback/self-heal vocabulary | §4.5 (all three legs addressed, two claimed, one refused) |
| 14 error-propagation gates | §4.3 (six named CI gates) |
| 15 living memory | §4.6 |
| 16 tensor/spectral + eqc reuse | §4.6 (kernel spectral reuse mandatory; eqc N/A stated) |
| 17 regression ledger | §5 (four rows named) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | header + §1 (arcs reused), §2 (rejected alternatives), §3.6 (existing CPU floor as fallback) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Two lanes. **Lane A (buildable TODAY, no network):** T1-T5. **Lane B (blocked on O18a — do NOT
attempt `cargo add wgpu` without the operator's network grant; check
`docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md` for gate status first):** T6-T8.

1. **T1 (G5, pure CPU).** Create the settle gate in `engine/src/settle.rs` per §2
   (`SettleGate`/`FrameSignals`/`should_render`, K=3, `SETTLE_EPSILON`). Write RED first:
   `settle_stops_frame_callbacks`, `touch_wakes_instantly`, `divergence_never_dormant`
   (energy-pump trick from `engine/src/field_energy.rs`'s broken-integrator pattern).
   Acceptance: `cargo test -p engine settle` green; ledger row added.
2. **T2 (G4, CPU legs).** Implement FE-07/10/11/12/13 per the §3.4 table, one module each or
   grouped in `engine/src/layout_field.rs`/`feedback.rs` — read each unit's spec in
   `docs/design/field-ui-engine/BLUEPRINTS-FIELD-UI.md` first (FE-07 `:221`, FE-10 `:283`,
   FE-11 `:303`, FE-12 `:322`, FE-13 `:342`). FE-12 consumes kernel spectral output ONLY
   (contract comment `engine/src/bridge.rs:657-676`) — write zero eigen-math in the engine.
   Acceptance: ten §3.4 tests green.
3. **T3 (G2 CPU half).** `engine/src/particles.rs`: pool→integrate→pack pipeline per §3.2
   with the `PARTICLE_STRIDE_F32` compile-time assert against `zerocopy.rs`'s layout; tests
   `particles_cpu_determinism`, `particles_single_upload_per_frame` (vs the `GpuSink` mock,
   `zerocopy.rs:143`), NaN/capacity/dt adversarial arms. Acceptance: green; bench
   `particles_integrate_10k` added RED-commit-first per §6.
4. **T4 (G6 DOM half).** `web/src/lib/a11y_mirror.mjs` + overlay per §2/§3.6;
   `web/tests/a11y-mirror.spec.mjs` with the 4-assertion set (accessibility-tree snapshot,
   zero-visible-DOM, keyboard order, stale-node). Drive order state via the local wasm path
   (`web/src/app.mjs` bindings — extend, do NOT rewrite; its header's G3 note is the charter).
   Acceptance: Playwright spec green headless.
5. **T5 (G7).** RW-05: add ptr/len exports per §2 to `wasm/src/lib.rs`, JS view helpers +
   `zero_copy_frame_no_alloc` + detached-view adversarial; keep the old Vec exports until the
   parity pin is green, then remove them in the SAME commit that flips consumers. Move
   retrieval exports (`wasm/src/lib.rs:153-168`) to a separate module. RW-01: root workspace
   `Cargo.toml`. RW-10: real `web/` toolchain scripts. RW-11 grep gate (no `createElement`
   outside mirror/overlay). Acceptance: `cargo test --workspace` green; web tests green.
6. **T6 (G1 — FIRST task after O18a unlocks).** `cargo add wgpu` under `feature = "gpu"`
   (+ cosmic-text for T8; the §2 manifest is the complete authorized list). Implement
   `engine/src/gpu.rs` per §2; flip `bridge.rs::gpu::new_gpu` (`:253-260`) to build `WgpuSink`;
   un-ignore + green `gpu_blit_roundtrip_matches_compose_oracle`; run the sRGB teeth-proof
   once. `e21_default_build_has_no_real_gpu_adapter` must STAY green (default build
   unchanged). Acceptance: `cargo test -p engine --features gpu` green on GPU hardware.
7. **T7 (G2/G4 GPU halves).** Instanced particle draw + field-unit render legs; un-ignore
   `particles_draw_smoke`. Acceptance: smoke green; profiler shows 1 write_buffer/frame
   against the REAL sink.
8. **T8 (G3).** SDF pipeline + token UBO + MSDF atlas per §3.3 with the interior-band gate.
   Acceptance: the three §3.3 tests green; `SDF_PIXEL_DIFF_MAX` untouched (raising it = not
   done, §5).

---

## 11. P38b — Sea & Sheet product surfaces (deliberately light: the DZ arc is the blueprint)

**Why light:** DZ-01..12 are already implementation-ready unit blueprints with their own
current-state audits, gates, and acceptance boxes (`BLUEPRINTS-DOWIZ-INTERFACES.md:46-320`).
This section binds them to P38a and to §10.5.3's P38b DoD; the unit detail lives there.
Re-deriving it here would violate standard item 19.

### 11.1 Unit map (all twelve, none dropped)

| Unit (arc line) | Binds to | P38b note |
|---|---|---|
| DZ-01 Shell grammar (`:46`) | G3 pipelines | the two-layer Sea/Sheet act structure |
| DZ-02 tokens + `<Money>` + no-tween (`:69`) | FE-05 token table + `money_guard` | NO second token system — maps onto `FrameUniforms.theme_tokens` |
| DZ-03 spectral edge + dive transitions (`:92`) | FE-12 + G5 | |
| DZ-04 OrderStatus → Sea (`:117`) | P37 wire / F12 local path | order state drives the field |
| DZ-05 feedback vocab (`:140`) | FE-10 | |
| DZ-06 local-first event-log + replay (`:161`) | kernel event spine | F12-canon |
| DZ-07 CLIENT flows (`:189`) | all P38a | the storefront + checkout + track |
| DZ-08 COURIER flows (`:225`) | all P38a | |
| DZ-09 OWNER/ADMIN (`:247`) | all P38a + P37 caps | admin surface ≠ P37 scope creep — renders via caps P23-P3 wires later |
| DZ-10 Intent/FieldPos/InputSource + voice + gesture (`:286`) | — | **types + PointerSource ONLY now; voice/gesture stay Phase-9b deferred** (§1) |
| DZ-11 a11y mirror hybrid (`:304`) | G6 (same mechanism, one implementation — DZ-11 must cite §3.6, not re-build) | |
| DZ-12 cross-platform fallback (`:320`) | FE-16 ladder | |

### 11.2 Predefined types (P38b DoD-1's baseline: 0 grep hits today, §0)

```rust
// engine/src/intent.rs (or web-side mirror) — DZ-10's spec, types-only slice:
pub struct FieldPos { pub u: f32, pub v: f32, pub w: f32 }
pub enum Intent { Point(FieldPos), Impulse(FieldPos, f32), Select(WidgetId),
                  Navigate(NavTarget), Scrub(f32), Command(CommandId) }
pub trait InputSource { fn poll(&mut self) -> Option<Intent>; }
// ONE code path: InputRouter::tick → field.apply (the arc's own invariant) —
// PointerSource ships in P38b; VoiceSource/HandCameraSource are Phase-9b, NOT here.
```

### 11.3 DoD (from §10.5.3 P38b, named)

1. `intent_types_exist_and_exercised` — the §11.2 types + a PointerSource round-trip test
   (tap → `Intent::Select` → field response). RED today (0 grep hits).
2. `sea_order_end_to_end` — an order placed through the Sea surface reaches delivery-domain
   fold state via P37's wire OR the F12 local WASM path; asserts the same byte-identity oracle
   as P37's §3.4/§3.5 tests (reuse the fixture, don't fork it).
3. DZ traceability table — each of DZ-01..09/11/12 traceable to landed code or an explicit
   dated deferral note; DZ-10 voice/gesture carry the Phase-9b deferral note from day one.

### 11.4 Anti-scope + dependencies (restated from §10.5.3, binding)

No DOM-first screens (G6's assertions apply to Sea/Sheet unchanged). No Whisper resurrection
ahead of the order path. No second token system. Hard-depends on P38a's pipelines (G1-G3) and,
for real order data, P37 + PROTOCOL P34; blocks nothing downstream except demo polish
(P17/P20 benefit, not gated).

### 11.5 Experiential/craft layer (cross-ref, 2026-07-18)

P38a's build items are mechanism; the QUALITY those mechanisms must produce is specified in
`docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md` **Додаток C** — the
narrative-cinematic reading of the Sea: the order-lifecycle pacing arc with named beats and an
amplitude budget (C.2), the OKLCH grade riding existing tokens (C.3), camera language mapped
onto existing primitives with 3D/DoF honestly costed-out (C.4), and the restraint discipline
(C.5 — "cinematic" is a deletion criterion as much as an addition one). Read G4 (field
dynamics, focus wells) and G5 (settle) alongside it: the settle gate IS the held-stillness
instrument, TIDE/attending speeds ARE the tempo instrument, and its verdict is binding here —
narrative pacing is Sea/T2 dowiz-fixed; NO brand pacing token exists (the 5-token Sheet limit
of DZ-02 is unchanged).

---

## 12. CANON-DIFF (2026-07-18) — append-only corrections, originals preserved above

> Source of authority: `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §0.2-2, §2 (X2), and §5's
> canon-diffs table (row **P38-rev**). This block corrects P38 **without deleting a prior
> claim** — every superseded statement above stays in place, with an inline SUPERSEDED marker at
> the point of the struck claim (§3.6); this section states what changed and why. Convention:
> this repo never silently reinterprets canon.

### 12.1 G6's transparent-`<input>` overlay is STRUCK — text input moves fully in-canvas (P57)

**What was written (preserved above, unaltered):** the G6 "input overlay (invisible DOM)"
leg — §1 scope table (G6 row), §1 anti-scope ("DOM survives ONLY as the invisible a11y mirror +
input overlay"), §2's G6 comment context, §3.6's Overlay bullet ("transparent `<input>`
(type=email/tel preserved) for IME/autofill/mobile keyboard"), §3.6 verified-how ("the overlay
input has no painted box"), §3.7 RW-11 grep gate ("no `createElement` outside `a11y_mirror.mjs`
+ overlay module"), §5 DoD row 6, and §10 T4 ("`a11y_mirror.mjs` + overlay per §2/§3.6").

**What changed, and why:** the operator ruled (SYNTHESIS §0.2-2) that **Wave-0 text input is
Latin + Cyrillic ONLY, entered fully custom inside the canvas via `cosmic-text`** (buffer,
cursor, selection, clipboard) — **no DOM `<input>` element exists on any platform.** This
enforces MASTER-ROADMAP §16.34 ("text input inside canvas is fully custom — no HTML `<input>`
overlay hybrid") in **canon**, not just in prose, and resolves the R1 §0 contradiction that
SYNTHESIS X2 flags. Non-Latin scripts that require **IME composition** (Arabic, CJK, Thai,
Indic) are **deferred to v2**, consistent with §16.58's existing RTL-deferred-to-v2 scope
boundary — a scope cut, not a new exception. Because Latin+Cyrillic need **no composition
events**, the entire keydown→buffer→GPU-glyph path is self-contained (SYNTHESIS X2).

**Where it is now built:** the struck overlay is replaced by blueprint **P57 — Canvas text
input & editing (Latin+Cyrillic)** (`cosmic-text` buffer/cursor/selection/clipboard over the
FE-06 MSDF glyph render; keydown wiring native + web; explicit v2 boundary for IME scripts).
P57 is the owner of every typed field across P69/P70/P71/P73.

**Residual engineering consequences (named in SYNTHESIS §0.2-2 / X2 — recorded, not reopened):**
(a) the **web** a11y mirror for *live text editing* uses R1's **synthetic ARIA-textbox** variant
(the hidden-editable-element variant is now ruled out) — owned by P58's a11y convention, cited
by P57's a11y half; (b) raising the **mobile-web soft keyboard** with no editable DOM element has
no standard mechanism → a **named spike** (P63); the installed Tauri client has native
soft-keyboard control (`show_soft_input`) and is unaffected, and voice (§16.31) is the honest
interim input on that one web-mobile combination if the spike comes back empty.

**DoD delta (not-done clauses this adds):** a build that ships any DOM `<input>`/editable element
for text entry on any platform = **NOT done** (grep gate: no `<input>` / `contenteditable` in the
render/web tree); text entry that is not the P57 in-canvas path = NOT done. The §3.7 RW-11 grep
gate is tightened accordingly — `createElement` is permitted **only** inside `a11y_mirror.mjs`
(the semantic mirror); the "overlay module" exemption is **removed**.

### 12.2 AR/VR insurance constraints — four HARD requirements (not deferred, not Track-R)

Per MASTER-ROADMAP §17.5 ("build in AR/VR / new-form-factor readiness **now**, not deferred")
and SYNTHESIS §2/§5 (R1 §8), P38's render core carries four **structural** constraints as of this
pass. These are cheap architectural insurance **paid now**; the actual OpenXR (native) / WebXR
(web) **backend implementation stays deferred** to Track-R. This is a shape requirement on the
render core, **not** a feature build — no XR device support ships in Wave-0.

1. **View/projection-matrix-driven pipeline, end-to-end.** The render pipeline MUST be driven by
   an explicit `view` × `projection` matrix pair from vertex transform onward — **no baked
   2D-only assumption** anywhere. Consequence for §2's types (stated here, §2 left intact per
   append-only): `FrameUniforms` (currently `screen`/`dpr` only, §2) gains a
   `view_proj: [[f32;4];4]`, and the flat 2D screen case becomes the **orthographic identity**
   configuration of that matrix — never a separate hardcoded path. RectInstance/SDF quads
   position through the matrix: a 2D screen is `ortho()`, a spatial surface is a real
   perspective, one code path. Falsifier: a render of the standard screen scene under the
   ortho-identity matrix must match the current 2D output bit-for-bit (the G1 oracle extended).
2. **`FieldPos` is 3D end-to-end — never 2D-with-a-3D-facade.** The scene-position type is
   already declared 3D in §11.2 (`pub struct FieldPos { pub u: f32, pub v: f32, pub w: f32 }`).
   This constraint makes that load-bearing: `w` must survive **unchanged** through layout
   (FE-07/11), scene assembly, the `VertexBridge` upload, and the GPU vertex path — no stage may
   collapse to `(u, v)` and reconstruct `w`, and no `FieldPos` may be silently truncated to 2D.
   Falsifier: a round-trip test asserting a non-zero `w` is preserved bit-identically from intent
   through to the vertex buffer.
3. **All input routes through the intent-abstracted `InputSource`.** No render/interaction code
   may consume raw mouse/touch events directly. Every input channel — pointer, keyboard,
   **voice, gesture, and any future XR controller** — normalizes to `Intent` via an `InputSource`
   implementation (§11.2 already declares `pub trait InputSource { fn poll(&mut self) ->
   Option<Intent>; }` and the `Intent` enum). PointerSource ships in P38b; VoiceSource/gesture
   are P64; an XR-controller source is a later `InputSource` impl needing **zero** change to the
   intent surface. Falsifier: grep gate — no event-type-specific handler
   (`mousedown`/`touchstart`/…) outside the `InputSource` adapters.
4. **Exactly one XR seam.** There is **one** extension point where an XR backend (OpenXR native,
   WebXR web) supplies the per-eye `view`/`projection` matrices (constraint 1) and pose-derived
   `Intent`s (constraint 3); the render core consumes them unchanged. Adding the XR backend later
   must require **no restructuring** of the render core — it is a new provider behind that one
   seam, exactly as `TunnelProvider`/`VpsProvider` are named-but-unbuilt ports elsewhere in the
   roadmap (SYNTHESIS §3.2 rationale 6). The seam is named now; the backend is Track-R.

**Not-done clauses:** a pipeline with a 2D-only transform path, a `FieldPos` truncated to 2D at
any stage, an input handler bound to a concrete event type outside an `InputSource`, or more than
one (or zero) XR seam = **NOT done** — regardless of Wave-0 green totals. Building an actual
OpenXR/WebXR backend in Wave-0 remains out of scope (Track-R); only the four structural
properties are required now.

### 12.3 FE-15 (mirror base) + FE-16 (fallback ladder) reaffirmed as the shared imported base

FE-15 (the a11y **semantic-mirror base** — §2's mirror-node shape + §3.6's per-frame reconcile,
clip-path SR-only root, zero painted pixels) and FE-16 (the **WebGL2 → CPU `compose_field`
fallback ladder** — §3.6) are the **canonical shared foundations** that every new W1/W2/W3 surface
blueprint **imports**, not re-derives. Specifically: **P57, P58, P69, P70, P71, and P73** cite
FE-15/FE-16 from this file — P58 generalizes the FE-15 mirror to every screen (plus the shared
Playwright a11y-tree gate), and P57's a11y half rides P58's ARIA-textbox convention on top of it;
every surface blueprint carries the FE-16 "renders correctly on the WebGL2 and CPU floors" DoD
line (SYNTHESIS §3.2's WebGL2-floor standing gate). There is exactly **one** mirror implementation
and **one** fallback ladder in the product; a blueprint that stands up a second of either = scope
violation, not initiative.
