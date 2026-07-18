# BLUEPRINT P63 — Shell & platform spike (measured verdicts) (2026-07-18)

> **Planning document — writes no product code, ships no production UI.** Written against the
> 20-point contract in `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9).
> Component: **DELIVERY / measurement spike**. Wave **W1**, per
> `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (the P63 row) and §3.2 point 1 ("P63 is first among
> equals … cheap-to-measure, expensive-to-guess"). Structural template:
> `BLUEPRINT-P39-app-shell-installability.md` (its sibling — P39 §1.2 explicitly names this file as
> the evidence source for its directional ruling); rigor precedent: `BLUEPRINT-P51-open-map-routing.md`.
>
> **This blueprint is unusual: its DoD is a measurement program, not a shipping feature.** It exists
> to produce the evidence that other blueprints depend on. Every build item (§3) is a *spike* stated
> as **Hypothesis → Measurement method → Falsifiable pass/fail bar → Minimal prototype → Adversarial
> cases** — a number that crosses a stated threshold is the RED→GREEN falsifier, not a feature
> checkbox. The prototypes are throwaway; the **verdicts** (numbers + a `Confirms`/`Refines`/
> `Contradicts`/`Blocked` ruling) are the durable deliverable. P63 does **not** re-decide the
> operator's directional shell ruling (P39-rev §1.2) — it supplies the measured evidence that
> **confirms or refines** it, and, if the numbers contradict it, the evidence governs (P39 §1.2
> verbatim: "If P63's measurements contradict the directional ruling … P63's evidence governs").

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. Ground truth is non-discussible; everything
below builds on this table only.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| A **real headless wgpu bring-up already exists**, feature-gated OFF by default: constructs a live `wgpu::Instance`, requests an `Adapter`, best-effort `Device`/`Queue`, returns a typed `GpuContext`; GPU absence is a typed value (`GpuError::NoAdapter`/`DeviceRequest(String)`), never a panic; async bridged with `pollster::block_on` | `kernel/src/render/gpu.rs:1-16` (module doc), `:19` (`use wgpu::{Adapter,Device,Instance,Queue}`), `:23-27` (`GpuError`) | **VERIFIED — this is the exact seam P63 extends.** gpu.rs's own upgrade trigger names it: "if/when the render stack grows a **real event loop / async surface pump** (P38a G2+ **live present path**), replace `block_on` with that loop's executor" — SP-1/SP-2 are that live present path |
| The `gpu` feature wires wgpu 30.0.0 + pollster 1.0.1, OFF by default so the canonical order/money graph never pulls them | `kernel/Cargo.toml:57` (`gpu = ["dep:wgpu","dep:pollster"]`), `:97` (`wgpu = { version = "30.0.0", optional = true }`), `:100` (pollster 1.0.1) | VERIFIED — the surface prototype adds `winit` beside this feature, never in the default graph |
| The **engine crate deliberately keeps wgpu OUT OF SCOPE** (offline CPU-parity): the GPU upload sink is a stub `Err("gpu adapter not built — wgpu uncached")`; the WebGL/WebGPU/splat feature boundaries are **EMPTY scaffolding** (E23), no deps pulled | `engine/Cargo.toml:8` ("wgpu/cosmic-text are OUT OF SCOPE"), `:34` (`webgl = []`), `:37` (`webgpu = []`), `:39` (`splat = []`); `engine/src/bridge.rs:241-247` (stub `gpu::new_gpu`) | VERIFIED — SP-6's parity method is exactly "make these empty boundaries functional and prove they match the CPU floor" |
| The **CPU-parity reference frame is bit-deterministic**: `compose(scene, eq, w, h, steps) -> Vec<u8>`, asserted identical across calls | `engine/src/field_frame.rs:255` (`pub fn compose`), `:430` (`compose_returns_deterministic_frame`), `:447-450` (bit-equality assert) | **VERIFIED — this determinism is what makes SP-6 a real pixel-diff gate, not a flaky screenshot compare** |
| The **CPU floor is a live wasm export**: `compose_field(circles,w,h,steps) -> Vec<u8>` (+ `frame()`, `vertex_field`) — the terminal rung of the FE-16 ladder | `wasm/src/lib.rs:57` (`compose_field`), `:96` (`frame`), `:112` (`vertex_field`) | VERIFIED — SP-6 compares WebGL2/WebGPU output against this floor |
| Zero-copy upload seam: `GpuUploadSink` trait, `HeadlessSink`, `VertexBridge`, `upload_to<S: GpuUploadSink>` — the abstraction a real windowed surface sink implements | `engine/src/bridge.rs:35` (trait), `:50` (`HeadlessSink`), `:69` (`VertexBridge`), `:174` (`upload_to`) | VERIFIED — SP-1/SP-2's present path is a new `impl GpuUploadSink` targeting a `winit` surface, not a rewrite |
| **FE-14 lazy-render-on-settle is the battery lever** — `should_render = input_pending \|\| !settled(K) \|\| animation_active \|\| external_change`, hysteresis K=3, Lyapunov watchdog (`energy_delta > 0` forces a wake so divergence is never invisible); the energy seam is wired test-only | P38 §2 (`BLUEPRINT-P38-webgpu-render-engine.md:153-156`), §3.5 (`:278-283`); `engine/src/field_energy.rs:1-16` (E1 seam), `engine/src/motion.rs:145-154` (critically-damped ζ=1 settle) | VERIFIED — SP-5 must **measure** FE-14's real-world savings (settle-on vs settle-off), the exact thing R1 §9 risk #3 says must not be asserted |
| **FE-16 fallback ladder** = WebGPU → WebGL2 → CPU `compose_field` + canvas2d `putImageData`; the empty feature flags "become functional" in P38 G6 | P38 §3.6 (`:313-314`), G6 gate row (`:70`) | VERIFIED — SP-6 defines the reusable parity method that proves each rung matches |
| **AccessKit is native-ready, web/canvas backend is planning-only in 2026** | R1 §2 (`OPUS-R1-INTERFACE-RENDERING-2026-07-18.md:89-96`); P38 §0 (`:51`, "AccessKit no web backend 2026") | VERIFIED — SP-1 measures **native** AccessKit integration friction (the desktop path); the web mirror is P58's lane, not P63's |
| **No `winit` / `accesskit` / `tauri` code exists anywhere in the tree** | `grep -rliE 'winit\|accesskit\|tauri' --include='*.rs' --include='*.toml'` → 0 files (this pass) | VERIFIED — every P63 prototype starts from zero, exactly as P39 leg A did (P39 §0: "installability 0%") |
| The **directional shell ruling P63 supplies evidence for**: desktop = `winit`+`wgpu`+AccessKit, **no embedded webview** (Path C hosted-redirect frees it); mobile keeps the Tauri webview/shell, card path = Path B native SDK sheet (Path C fallback); §4-A recorded **CLOSED** | `BLUEPRINT-P39-app-shell-installability.md` §1.2 (`:128-199`), esp. `:181-189` ("P63 supplies the *evidence*") | VERIFIED — P63 is the named evidence intake; it confirms/refines, does not re-decide |
| The **still-open engineering unknown P39 §1.2 hands to P63**: Path B "rests on a **named, still-unresolved engineering unknown** — the native-SDK-over-GPU-surface bridge … This block does **NOT** resolve it … Until P63 reports, Path B is the directional default, not a confirmed-feasible one" | `BLUEPRINT-P39-app-shell-installability.md:175-179` | VERIFIED — SP-3 is precisely this measurement |
| **X3 makes the payment path and the shell decision one coupled choice**; the mobile Path-B bridge = "the unproven Tauri-plugin-over-GPU-surface bridge (R2 risk #3) — same spike as R1's mobile-shell question; **one combined spike** (P63)" | `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §2 X3 (`:145-156`) | VERIFIED — SP-2 + SP-3 are that one combined spike |
| **X2 names the mobile-web soft-keyboard mechanism as a spike with no standard solution**; interim fallback = voice channel + installed app on that platform combo | `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §2 X2 (`:135-141`), §0.2-2 (`:50-52`), §4-E (`:381-383`) | VERIFIED — SP-4 |
| **R2 risk #3** (native-SDK↔Tauri-wgpu bridge unproven), **R1 risk #3** (full-shift GPU battery unproven on budget hardware), **R1 risk #7** (shell decision), **R1 risk #9** (WebGL2-floor parity for ~18% without WebGPU) are the four named unknowns this spike closes | `OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md:202` (risk #3); `OPUS-R1-INTERFACE-RENDERING-2026-07-18.md:437-466` (risks #3/#7/#9) | VERIFIED — one-to-one with SP-3 / SP-5 / SP-1+SP-2 / SP-6 |
| WebGPU 2026 reach ≈ 82% (WebGL2 floor covers the ~18% remainder) | R1 §6 (`OPUS-R1-INTERFACE-RENDERING-2026-07-18.md:284-291`) | VERIFIED — the population SP-6's method protects |
| Kernel `TokenBucket` exists (not central here; noted so SP-3's anti-abuse note reuses, never re-adds) | `kernel/src/token_bucket.rs` (ls this pass) | VERIFIED |

---

## 1. Scope — what P63 owns vs deliberately does NOT (§5 anti-scope, sharpened)

**P63 owns (build items §3) — six spikes, each a measured verdict:**

| Item | Spike | The measured question |
|---|---|---|
| SP-1 | Desktop shell prototype | Does `winit`+`wgpu`+AccessKit hit frame/latency budget and is AccessKit friction acceptable — **no-webview desktop confirmed or flagged?** |
| SP-2 | Mobile wgpu-surface reality check | How "rough" is raw wgpu inside a Tauri mobile shell (iOS/Android) — **real surface-conflict numbers, not R1's word "rough"** |
| SP-3 | Native-payment-SDK-over-GPU bridge | Can a native Stripe/Adyen SDK sheet be presented over/alongside a Tauri wgpu surface **without breaking the render loop?** (R2 risk #3) |
| SP-4 | Mobile-web soft-keyboard mechanism | With no editable DOM element, **how does a web-mobile user raise the OS keyboard to type into the canvas** — a real mechanism or a real "voice is the interim fallback" verdict (X2) |
| SP-5 | Full-shift battery benchmark | Actual GPU-render drain on **real budget Android** over a simulated multi-hour shift — **real numbers, and FE-14's savings measured not asserted** |
| SP-6 | WebGL2/CPU-floor parity method | The **reusable** method P69/P70/P71/P73 each import to prove "renders correctly on the FE-16 WebGL2 and CPU floors" |

**P63 explicitly does NOT own (each with its owner):**

- **NOT any production UI.** No shipped screen, no menu, no checkout, no courier surface. The
  prototypes render whatever minimal scene exercises the measurement (a `compose()` frame, a text
  field, a settled map placeholder) and are **deleted after the verdict is captured** (§4.5
  rollback). Building product UI is P69/P70/P71/P73's lane.
- **NOT the operator's directional ruling.** The desktop=winit / mobile=Tauri / Path-C-desktop /
  Path-B-mobile decisions are P39 §1.2's, already made. P63 measures; if a number contradicts the
  ruling, P63 files a `Contradicts` verdict and a dated refinement block goes into P39-rev — P63
  does not silently re-rule (P39 §1.2's own escalation path).
- **NOT the payment adapter or the card-capture flow.** The `PaymentProvider` port, Stripe server
  adapter, idempotency contract, and the client card leg are **P60's** (SYNTHESIS §5). SP-3 answers
  *only* "does the native SDK sheet coexist with the GPU surface"; it writes no adapter code and
  handles no PAN (the PCI red-line — R2 §6.3 — holds: no card-data type is constructed anywhere in
  the spike; the sheet is the provider's, tokenization is on-device).
- **NOT the text-input engine.** cosmic-text buffer/cursor/selection is **P57's**. SP-4 measures
  *only* the keyboard-raise mechanism on web-mobile, not the editing model.
- **NOT the a11y mirror or its harness.** The web hidden-DOM mirror + the shared Playwright a11y
  gate are **P58's** (SYNTHESIS X1). SP-1 measures *native* AccessKit friction only; SP-6's parity
  method is about pixels, not roles.
- **NOT the battery *gates*.** SP-5 produces the measured *baseline* and the bar; **P71** consumes
  that baseline as its own DoD (SYNTHESIS §5 P71 row: "battery gates from P63's measured baseline as
  DoD"). P63 measures, P71 gates.
- **NOT the procedural-glyph / AR-VR / intent-UI programs.** Track-R and the four AR/VR insurance
  constraints are P38-rev's; P63 touches none of them.

### 1.1 Why a spike is the right shape here (reuse-first, item 19)

The naive alternative — "just build the desktop shell in P39 / the mobile card path in P60 and find
out" — is rejected: R1 §9 ranks the shell decision (#7) and the battery question (#3) as
"expensive to reverse / cannot be discovered late," and R2 §9 ranks the native-SDK bridge (#3) as
"where §6 could still fail in practice." Every W2 surface (P69/P70) and W3 surface (P71) inherits
these verdicts; a wrong guess propagates into four blueprints before the first measurement. The
existing substrate makes the spike **cheap**: `kernel/src/render/gpu.rs` already brings up a real
device (only the *surface/present* path is new), and `compose()` is already a bit-deterministic
reference frame — so the measurement harness extends landed code rather than starting blank. That
is exactly the reuse-first bar item 19 demands: extend the `gpu`-gated bring-up and the
`GpuUploadSink` seam, do not fork a second renderer.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE any prototype)

A spike still has a spec: **the verdict schema and the falsifiable bars are the spec of "done."**
These types are the contract every SP-* lane writes into; the bars are hypotheses the spike may
itself *refine* (a `Refines` verdict may move a bar), but they are stated as numbers first so that
"pass" is machine-checkable and never a matter of taste.

```rust
// ── Home: tools/shell-spike/ (NEW throwaway workspace member, /root/dowiz) ──────
// A `[workspace]` member gated behind its own `spike` feature so nothing in the
// canonical order/money graph can depend on it. Deleted wholesale at close (§4.5).
// Verdict RECORDS are the durable output; they serialize to a committed markdown
// evidence file docs/design/CORE-ROADMAP-2026-07-17/P63-VERDICTS.md (§5).

pub enum SpikeId { Sp1Desktop, Sp2MobileSurface, Sp3PaymentBridge,
                   Sp4WebKeyboard, Sp5Battery, Sp6FloorParity }

pub enum Platform {
    Desktop { os: DesktopOs },        // Linux / macOS / Windows
    MobileAndroid { chipset: String },
    MobileIos { model: String },
    WebMobile { browser: String, os: String },   // Safari-iOS / Chrome-Android
}

pub enum HwClass { BudgetAndroid, MidAndroid, MidDesktop, HighDesktop, Emulator }

/// The four-valued verdict. `Confirms`/`Refines` keep the directional ruling;
/// `Contradicts` forces a dated refinement block into P39-rev (evidence governs);
/// `Blocked` is the honest arm when a measurement needs a resource not present
/// (a physical device / an app-store dev account) — NOT a pass, NOT a guess.
pub enum SpikeVerdict {
    Confirms,
    Refines     { delta: String },      // ruling stands + a named bound/caveat
    Contradicts { evidence: String },   // measured value crossed the bar the wrong way
    Blocked     { on: String },         // e.g. "physical budget Android device"
}

/// One row of evidence. `measured` is a REAL number or "BLOCKED: <resource>" —
/// never an estimate. Emulator rows are legal for SP-1/SP-2/SP-4/SP-6 but ILLEGAL
/// for SP-5 (battery on an emulator is meaningless; §3.5 forbids it).
pub struct VerdictRecord {
    pub spike: SpikeId,
    pub hypothesis: &'static str,
    pub method: &'static str,
    pub bar: &'static str,              // the falsifiable threshold, human-readable
    pub measured: String,              // real number, distribution summary, or BLOCKED
    pub platform: Platform,
    pub hw_class: HwClass,
    pub verdict: SpikeVerdict,
    pub captured_utc: u64,             // caller-supplied counter (no ambient clock)
}

// ── Falsifiable bars (the pass/fail thresholds; §3 states each in context) ──────
// SP-1 desktop
pub const DESKTOP_FRAME_BUDGET_MS:     f64 = 16.7;  // 60 FPS interactive target
pub const DESKTOP_FRAME_FLOOR_MS:      f64 = 33.0;  // 30 FPS hard floor (P38 §6)
pub const INPUT_LATENCY_BAR_MS_P95:    f64 = 50.0;  // tap/keystroke→pixel, p95.
                                                    // Refs: GPUI 2 ms, VS Code 12–25 ms (R1 §1)
// SP-2 mobile surface
pub const MOBILE_SOAK_MIN:             u32 = 30;    // continuous render soak
pub const MOBILE_FLICKER_FRAMES_MAX:   u32 = 0;     // webview↔wgpu contention flicker (R1 §6)
pub const MOBILE_FRAME_FLOOR_MS:       f64 = 33.0;
// SP-3 payment bridge
pub const SHEET_PRESENT_DROPPED_MAX:   u32 = 3;     // frames lost presenting the SDK sheet
pub const SHEET_RECOVERY_MS_MAX:       f64 = 250.0; // render loop back within budget after dismiss
// SP-5 battery (REAL device only)
pub const SHIFT_HOURS_SIM:             f64 = 6.0;   // simulated shift length
pub const SETTLED_DRAIN_BAR_PCT_PER_HR:f64 = 4.0;   // settled screen, foreground
pub const SETTLE_SAVINGS_MIN_PCT:      f64 = 30.0;  // settle-ON must beat settle-OFF drain by ≥30%
pub const THERMAL_SUSTAIN_MS:          f64 = 33.0;  // sustained frame time over soak ≤ this
// SP-6 parity
pub const PARITY_PERCEPTUAL_DELTA_MAX: f64 = 0.02;  // normalized per-pixel ΔE / (1−SSIM) tolerance
```

Rejected alternatives (DECART one-liners): **a second renderer / a spike that forks `compose()`** —
rejected: the CPU floor is the *reference*, not a competitor; the prototype's WebGL2/WebGPU output is
diffed *against* `compose()`, so forking it destroys the oracle (item 19 reuse-first). **Wiring
`winit` into the default kernel graph** — rejected: it would pull an event-loop dep into the
order/money crate; the surface lives behind the `spike` feature beside the existing OFF-by-default
`gpu` feature (`kernel/Cargo.toml:57`). **Asserting battery "should be fine" from the FE-14 design**
— rejected explicitly: R1 §9 risk #3 names this as the thing that must be *measured*; SP-5's `Blocked`
arm exists so an honest "no device yet" never masquerades as a pass. **An emulator battery number** —
rejected: physically meaningless; SP-5 forbids `HwClass::Emulator`. **A screenshot-similarity "looks
the same" parity check** — rejected: `compose()`'s bit-determinism (`field_frame.rs:430`) lets SP-6
be a real numeric pixel diff with a stated tolerance, not a human eyeball.

---

## 3. Build items — each spike as Hypothesis → Method → Falsifiable bar → Prototype → Adversarial (items 3, 5)

Every SP-* is RED before its verdict exists (no harness, no number) and GREEN when a
`VerdictRecord` with a **real** `measured` value and a `Confirms`/`Refines`/`Contradicts`/`Blocked`
ruling is committed to `P63-VERDICTS.md`. The prototype is the instrument; the record is the result.

### 3.1 SP-1 — Desktop shell: `winit`+`wgpu`+AccessKit (confirms/flags the no-webview ruling)

- **Hypothesis.** A `winit` window presenting a real `wgpu` swapchain, fed by the existing
  `GpuUploadSink` seam (`bridge.rs:174`) and carrying a native AccessKit tree, renders the
  `compose()` scene within frame budget, keystroke/tap→pixel latency stays below the bar, and
  AccessKit wiring for a *live text field* (caret/selection via `SetTextSelection`) is tractable
  without a webview. → the P39 §1.2 desktop ruling holds.
- **Method.** Extend `kernel/src/render/gpu.rs`'s bring-up (it already yields a `Device`/`Queue`) to
  a `winit` present loop behind the `spike` feature. Drive a scene of ~1–2k SDF instances plus one
  editable text field. Measure: (a) **frame time** as a distribution (p50/p95/p99) via the existing
  `FrameProfiler` (`bridge.rs:20`) over a 60 s interactive session; (b) **input latency**
  keystroke→pixel with a photodiode-or-frame-timestamp method (log the input event counter and the
  present-completion counter, diff them; GPUI's 2 ms and VS Code's 12–25 ms from R1 §1 are the
  reference band); (c) **AccessKit friction** as a structured checklist — does a screen reader
  (Orca/NVDA/VoiceOver) announce focus, role, value, and caret movement on the text field — recorded
  pass/partial/fail per capability, with the integration LOC + any upstream gaps noted.
- **Falsifiable bar.** p95 frame time ≤ `DESKTOP_FRAME_BUDGET_MS` (16.7 ms), p99 ≤
  `DESKTOP_FRAME_FLOOR_MS` (33 ms); p95 input latency ≤ `INPUT_LATENCY_BAR_MS_P95` (50 ms); AccessKit
  announces focus+role+value+caret on ≥ the text field (caret is the hard one — R1 §2). **Fail on
  any** → `Contradicts` or `Refines` (e.g. "frame budget met but caret announcement needs a manual
  `SetTextSelection` push — friction bounded, ruling stands with caveat").
- **Minimal prototype.** `tools/shell-spike/desktop/` — one window, one scene, one field. No product
  screens. ≤ a few hundred LOC.
- **Adversarial cases.** (i) **No adapter** — run on a headless/software-GL host: the prototype must
  degrade to the CPU `compose()` floor (`GpuError::NoAdapter` path already exists, gpu.rs:23), never
  panic — proves "no GPU ⇒ no UI" is unreachable on desktop too. (ii) **Resize storm** — rapid window
  resize must not deadlock the swapchain (a classic wgpu surface-lost bug). (iii) **AccessKit under
  rapid edit** — type fast; assert the a11y tree's caret node does not go stale (the same failure
  class P58 fears on web, measured here on native where it should be *easy*).

### 3.2 SP-2 — Mobile wgpu-surface reality check (real numbers for R1's "rough")

- **Hypothesis.** Raw `wgpu` rendering inside a Tauri mobile shell (the mobile ruling keeps the Tauri
  webview, P39 §1.2) is *usable* — the webview↔wgpu surface contention R1 §6 calls "rough"
  (flickering, macOS/iOS threading) is either absent or bounded on real devices.
- **Method.** Two variants per platform, both behind the `spike` feature: **(a) raw native wgpu
  surface** hosted in a custom view beside Tauri's webview, and **(b) WebGPU-in-webview** (Safari 26 /
  Chrome-Android WebGPU). Soak each for `MOBILE_SOAK_MIN` (30 min) rendering the settled-then-animated
  scene; capture frame time (device GPU profiler / `FrameProfiler`), and **count flicker/tear frames**
  where the webview and wgpu layers visibly compete (frame-capture diff against the deterministic
  `compose()` reference — a frame that matches neither the last nor the expected frame is a contention
  artifact). Record threading constraints hit (main-thread present requirements, surface-recreate on
  background/foreground).
- **Falsifiable bar.** Flicker/tear frames = `MOBILE_FLICKER_FRAMES_MAX` (0) over the soak; sustained
  frame time ≤ `MOBILE_FRAME_FLOOR_MS` (33 ms) p95; no surface-lost crash across ≥10
  background/foreground cycles. **Fail** → `Refines`/`Contradicts` with the winning variant named
  (e.g. "raw native surface flickers on iOS; WebGPU-in-webview is clean → mobile default = variant b").
- **Minimal prototype.** `tools/shell-spike/mobile/` (Tauri scaffold, one scene). No product screens.
- **Adversarial cases.** (i) **Background/foreground churn** — the OS reclaims the surface; the app
  must recreate it, not crash. (ii) **Low-memory pressure** — force a memory warning; assert the
  render loop survives. (iii) **Rotation** during animation — swapchain resize on a live field.
- **Honesty gate.** iOS requires a Mac + provisioning; Android requires the NDK + a device. If the
  toolchain/device is unavailable in the build environment, this spike files a **`Blocked{on:…}`**
  verdict naming the exact missing resource — it does **not** emit an emulator "pass" (emulator GPU
  paths do not reproduce the contention this spike exists to measure).

### 3.3 SP-3 — Native-payment-SDK-over-GPU-surface bridge feasibility (R2 risk #3, the combined spike)

- **Hypothesis.** A native provider SDK sheet (Stripe `PaymentSheet` / Adyen Drop-in, UIKit/Compose)
  can be presented **over/alongside** a Tauri wgpu surface and dismissed, **without breaking the wgpu
  render loop** — so Path B (P39 §1.2's mobile default) is *confirmed feasible*, not merely
  directional. This is X3's "one combined spike" (shell question ∧ payment question).
- **Method.** In the SP-2 mobile scaffold, add a Tauri plugin that invokes the provider's native
  sheet in **test mode** (test publishable key; **no real PAN, no live charge** — the sheet is the
  provider's surface, tokenization is on-device, dowiz code sees only an opaque handle: the R2 §5.2
  `ClientHandoff` shape, no card-data type constructed — PCI red-line held). Measure, across ≥20
  present→interact→dismiss cycles: (a) **frames dropped** during sheet presentation; (b) **render-loop
  recovery time** after dismiss (present-loop back within budget); (c) whether the wgpu surface must be
  torn down/recreated to show the sheet (a torn-down surface that cleanly restores is acceptable; a
  *corrupted* one is not); (d) input routing — taps go to the sheet, not the canvas underneath.
- **Falsifiable bar.** Frames dropped presenting the sheet ≤ `SHEET_PRESENT_DROPPED_MAX` (3);
  recovery ≤ `SHEET_RECOVERY_MS_MAX` (250 ms); zero surface corruption; input correctly captured by the
  sheet. **Fail** → `Contradicts`, which is load-bearing: it forces Path B → **Path C (hosted redirect)
  as the mobile card path** (P39 §1.2 names Path C the mobile fallback), and P60's client leg plans
  against Path C instead. A `Confirms` here is what upgrades Path B from "directional default" to
  "confirmed-feasible" (P39 §1.2's exact words).
- **Minimal prototype.** Extends `tools/shell-spike/mobile/` with the SDK-sheet plugin. Test keys
  only. No adapter code (P60 owns that).
- **Adversarial cases.** (i) **Dismiss mid-animation** — cancel the sheet while the field is
  animating; the loop must resume, not freeze. (ii) **Present under low memory** — the SDK sheet +
  GPU surface together; assert no OOM kill of the render context. (iii) **Rapid re-present** — open,
  cancel, reopen quickly; no leaked surface/context.
- **Honesty gate.** Same device/toolchain `Blocked` arm as SP-2. This is the highest-risk spike (R2
  §9 risk #3: "where §6 could still fail in practice") — a `Blocked` verdict here is itself
  actionable (P60's client leg cannot assume Path B until this is a real `Confirms`).

### 3.4 SP-4 — Mobile-web soft-keyboard with no editable DOM element (X2)

- **Hypothesis.** On the **web-mobile** path (a browser user, not the installed Tauri app), there is
  **no clean standard mechanism** to raise the OS soft keyboard to type into a wgpu canvas without an
  editable DOM element — so the honest Wave-0 verdict is "voice is the interim input on this platform
  combination" (X2/§0.2-2), *unless* the spike finds a real mechanism.
- **Method.** Enumerate and empirically test every candidate on real iOS-Safari and Chrome-Android:
  (a) a **visually-hidden but focusable** element (`contenteditable`/`<input>` moved off-canvas or
  `opacity:0`) — but note this is the very overlay §16.34/§0.2-2 struck, so it is tested only to
  *quantify what is being given up*, not proposed; (b) the **`VirtualKeyboard` API**
  (`navigator.virtualKeyboard`) and `overlays-content` — does it raise the keyboard without a focused
  editable? (c) `inputmode`/`enterkeyhint` on a focus target; (d) a same-origin invisible input that
  *forwards* keystrokes into the canvas buffer then is discarded (the "composition host" R1 §3 names,
  measured for keyboard-raise only). For each: does the keyboard **appear**, do **keydown/input**
  events reach the canvas, does **autofill** work, does the screen reader stay coherent. Record a
  capability matrix per browser/OS.
- **Falsifiable bar.** A candidate **passes** only if it raises the keyboard AND delivers character
  input to the canvas buffer AND requires **no visible DOM input field** AND does not regress the P58
  a11y story — on **both** iOS-Safari and Chrome-Android. If no candidate passes on both, the verdict
  is an explicit **`Refines`**: "no clean web-mobile keyboard mechanism; Wave-0 interim = voice
  channel (§16.31, equal intent channel per §16.50) + the installed Tauri app (native
  `show_soft_input`) as the daily-use path (§16.8) — web-mobile typing deferred." That is a **real
  answer**, not a gap — X2 pre-authorizes it.
- **Minimal prototype.** `tools/shell-spike/webkbd/` — a static page hosting the wasm `compose_field`
  canvas + each candidate behind a flag; driven on real devices (or BrowserStack-class real-device
  cloud). Emulator/desktop-devtools "mobile mode" is **insufficient** (soft-keyboard behavior is
  OS-native) — a `Blocked` arm applies if no real mobile browser is reachable.
- **Adversarial cases.** (i) **Screen-reader on** — does raising the keyboard via the candidate break
  the reading order (the reason the hidden-input overlay was struck)? (ii) **Autofill** — does the OS
  offer address/name autofill without a real form (the feature the overlay gave "for free")? (iii)
  **Rotation with keyboard up** — layout must not strand the caret off-screen.

### 3.5 SP-5 — Full-shift battery benchmark on real budget Android (R1 risk #3; FE-14 measured, not asserted)

- **Hypothesis.** Continuous GPU render for a multi-hour courier shift is sustainable on **budget**
  Android because FE-14 lazy-render-on-settle (`should_render`, K=3, P38 §3.5) keeps the screen
  dormant most of a ride — and the settle gate's saving is **large and real**, not assumed.
- **Method.** On a **physical budget Android device** (`HwClass::BudgetAndroid`; emulator forbidden),
  run a **scripted synthetic shift** of `SHIFT_HOURS_SIM` (6 h, or a scaled proxy with a stated
  extrapolation): a settled map/ETA screen (the courier's steady state) punctuated by periodic order
  events, short animations, and voice bursts. Measure with Android `dumpsys batterystats` / Battery
  Historian (mAh and %/hour), device thermals, and sustained frame time. **Run the A/B control:**
  identical script with FE-14 **settle-ON** vs **settle-OFF** (force `should_render = true` every
  frame). The delta *is* the FE-14 real-world saving — the exact number R1 §9 risk #3 says must be
  demonstrated.
- **Falsifiable bar.** Settled-screen drain ≤ `SETTLED_DRAIN_BAR_PCT_PER_HR` (4 %/h, so a shift fits a
  realistic battery budget alongside GPS/radio); settle-ON beats settle-OFF by ≥ `SETTLE_SAVINGS_MIN_PCT`
  (30 %) — if FE-14 saves less than that, the "0 rAF on a settled screen" claim (R1 §6) is overstated
  and P71 must plan a lower frame-rate floor; sustained frame time ≤ `THERMAL_SUSTAIN_MS` (33 ms) with
  no thermal throttle collapsing it over a ≥2 h soak. **Fail** → `Contradicts`/`Refines` feeding P71's
  battery gates (e.g. "settle saves 45% ✓ but a 6 h continuous-nav segment throttles at 3.5 h → P71
  needs a nav-mode power tier").
- **Minimal prototype.** Reuses the SP-2 Android scaffold with the FE-14 predicate wired to a
  toggle; the synthetic-shift driver is a scripted event feed (no product courier UI).
- **Adversarial cases.** (i) **Settle-gate defeat** — a divergent field (Lyapunov watchdog forces
  wakes, `field_energy.rs`): measure the *worst-case* drain when the screen never settles, so P71 knows
  the ceiling, not just the happy path. (ii) **Screen-on brightness** confound — hold brightness fixed
  and subtract a render-idle baseline so the number is *GPU* drain, not panel drain. (iii) **Background
  radio** — GPS + network active (a real shift), so the number is honest, not a lab artifact.
- **Honesty gate (the load-bearing one).** This spike **cannot be emulated** and **cannot be faked**.
  If no physical budget device is available in the environment, SP-5's deliverable is: the harness +
  the scripted-shift protocol + the stated bars + a **`Blocked{on:"physical budget Android device /
  first-client device fleet"}`** verdict — mirroring P39 §4.6's honest GAP ("real Android install
  verification waits for the first-client device fleet"). A `Blocked` SP-5 still unblocks P71's
  *design* (the bar and method are fixed); it flags the *number* as owed. Per memory
  `ground-truth-over-proxy` and `verified-by-math`: no asserted battery figure is ever written as
  measured.

### 3.6 SP-6 — WebGL2/CPU-floor parity check method (the reusable gate, R1 risk #9)

- **Hypothesis.** A single, reusable, deterministic method can prove any future surface renders
  **correctly** (not just "runs") on the WebGL2 and CPU rungs of the FE-16 ladder — protecting the
  ~18% of web users without WebGPU (R1 §6) — and it can be imported by P69/P70/P71/P73 as a DoD line,
  not re-invented per surface.
- **Method.** Define the method as a committed, importable harness (this is the one SP output that is
  **durable, not throwaway** — like P58's a11y gate). For a given scene: render the **WebGPU** frame,
  the **WebGL2** frame, and the **CPU `compose_field`** frame (`wasm/src/lib.rs:57`) at identical
  size/steps; compute a **per-pixel perceptual delta** (normalized ΔE / `1−SSIM`) of each GPU rung
  **against the bit-deterministic `compose()` reference** (`field_frame.rs:255`, the oracle whose
  determinism is proven at `:430`). The method ships as: (1) a Rust reference-frame generator
  (reuses `compose()`, zero fork), (2) a WebGL2/WebGPU capture path (makes the empty `webgl=[]`/
  `webgpu=[]` boundaries at `engine/Cargo.toml:34-37` functional — SP-6 is what "becomes functional"
  in P38 G6 means for *parity*), (3) a `floor-parity.spec.mjs` Playwright driver that forces each rung
  (WebGPU-disabled, WebGL2-only, CPU-only via the FE-16 flags) and asserts the delta.
- **Falsifiable bar.** For every rung, per-pixel perceptual delta ≤ `PARITY_PERCEPTUAL_DELTA_MAX`
  (0.02) against the CPU reference, over a fixed scene corpus (a storefront card, a text field, a
  settled map placeholder). **A WebGPU-only visual effect that has no WebGL2/CPU equivalent fails the
  gate by construction** — which is the whole point (R1 §9: "any WebGPU-only visual effect silently
  excludes ~1-in-5 web users"). The **deliverable is the method + a green baseline on the spike's own
  scene corpus**, plus the one-line DoD other blueprints paste: *"passes `floor-parity` at
  ΔE ≤ 0.02 on WebGPU, WebGL2, and CPU rungs."*
- **Minimal prototype.** Lands in a durable location (`engine/tests/floor_parity/` for the Rust
  reference + `web/tests/floor-parity.spec.mjs`), not `tools/shell-spike/` — SP-6 is a kept gate.
- **Adversarial cases.** (i) **Deliberately WebGPU-only effect** — add a compute-shader-only bloom to
  the corpus; the gate must go RED (a parity check that can't catch the exclusion it exists to catch
  is worthless). (ii) **Silent WebGL2 downgrade** — a rung that renders a *blank* frame must fail
  (blank ≠ the reference), not pass by rendering nothing. (iii) **Non-determinism smoke** — run the CPU
  reference twice; if it ever differs, the harness aborts (guards against a future `compose()`
  regression breaking the oracle — reuses the `:430` determinism assertion as a precondition).

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

Reachability arguments, not prose. **"A faked/asserted number ships as measured"** is unreachable:
`VerdictRecord.measured` is either a real reading or the string `"BLOCKED: <resource>"`, and SP-5
forbids `HwClass::Emulator` at the type level — an un-measured pass has no representation. **"No GPU ⇒
no UI"** is unreachable on every platform: the CPU `compose()` floor is the terminal rung
(`GpuError::NoAdapter` already returns a typed value, gpu.rs:23; SP-1 adversarial (i) exercises it;
SP-6 proves the floor *matches*). **"A WebGPU-only effect silently excludes 18%"** is unreachable once
SP-6 is a gate: an effect with no WebGL2/CPU equivalent fails parity by construction (§3.6 adversarial
i). **"The spike leaks into the product graph"** is unreachable: everything is behind the `spike`
feature beside the OFF-by-default `gpu` feature, and `tools/shell-spike/` is deleted at close — a
`cargo build` of the default workspace never compiles it. **"A `Contradicts` verdict is quietly
ignored"** is unreachable procedurally: §5's not-done clause makes an un-reconciled `Contradicts` a
P39-rev refinement obligation, not an optional footnote.

### 4.2 Schemas for scaling (item 8)

The measurement schema's scaling axes are stated, not timeless: **device-class matrix** (the axis that
grows — budget/mid/high × Android/iOS/desktop × browser; Wave-0 measures the load-bearing subset:
budget Android for SP-5, one desktop OS for SP-1, iOS+Android for SP-2/SP-3, iOS-Safari+Chrome-Android
for SP-4/SP-6 — the matrix expands with the first-client fleet); **soak duration** (frame-time and
battery distributions need minutes-to-hours, not single samples — `MOBILE_SOAK_MIN`, `SHIFT_HOURS_SIM`
are the stated points); **sample rate** (frame time sampled per-frame via `FrameProfiler`, battery per
`batterystats` bucket). Honest non-axis: the verdict *file* is O(6 spikes × device rows) — it does not
grow with orders or nodes; it is a fixed evidence artifact, versioned in git.

### 4.3 Isolation / bulkhead (item 11) + error-propagation gates (item 14)

The spike crate is bulkheaded by the `spike` cargo feature: its failure (a `winit` panic, a wgpu
surface loss) cannot reach the canonical order/money graph, which never compiles it. Each SP-* lane is
independent (different device/prototype, no shared mutable state) — a `Blocked` SP-3 does not block a
green SP-1. Named gates that turn this blueprint's bug classes into CI/compile failures: the `spike`
feature-gate (product graph can't depend on spike code — a `cargo build --workspace` without the
feature is the check); the `HwClass::Emulator`-forbidden-for-SP-5 arm (a review-grep gate on the
verdict file); the SP-6 determinism precondition (reuses `field_frame.rs:430`); and the not-done
clause (§5) that a `Contradicts` without a P39-rev refinement block fails review.

### 4.4 Mesh awareness (item 12)

Entirely **device-local**. Every measurement runs on one machine/phone; zero wire payloads, zero
gossip, no transport-layer dependency. SP-3 uses the provider's SDK in *test mode* over the device's
own network (the provider's call, not a dowiz mesh event). The verdicts are committed to git, not
propagated. No `iroh_transport`/`discovery` involvement of any kind.

### 4.5 Rollback / self-healing vocabulary (item 13, used precisely)

**Self-Termination leg claimed:** typed `SpikeVerdict::Blocked` and `GpuError` arms; the spike crate
is feature-gated and *fully deletable* — mechanical rollback is `rm -rf tools/shell-spike/` + drop the
`spike` feature line, and the product tree is byte-unchanged (nothing depends on it). **Self-Healing
leg claimed narrowly:** the FE-16 fallback ladder SP-6 verifies **is** genuine regenerative redundancy
(three independent render paths to the same frame truth; the CPU floor is the error-correcting terminal
rung) — claimed for the *ladder* SP-6 proves, not for the spike harness itself. **Snapshot-Re-entry:
NOT claimed.** The one durable output (SP-6's parity harness) is not rolled back — it is a kept gate
like P58's; only SP-1..SP-5's throwaway prototypes are deleted.

### 4.6 Living memory (item 15) + tensor/spectral (item 16) + Linux discipline (item 9)

Item 15: the verdict file is a temporal record — each `VerdictRecord` carries `captured_utc` and a
device row, so a re-measurement on new hardware **appends** (demote-never-delete: an old budget-device
number stays as history when a newer device supersedes it), matching the living-memory move-not-delete
discipline. Item 16: mostly honest N/A — no closed-form kernel math is introduced; the one reduction is
SP-6's perceptual-delta metric over the `compose()` reference (a scalar norm on a frame tensor), which
reuses the deterministic composer rather than an `eqc-rs` form — stated, not decorated. Item 9
verdicts: **REINFORCES** — this whole blueprint is the Linux-kernel "measure, don't assume" discipline
applied to a UI decision (the operator's directional ruling gets *benchmarked*, not trusted); **GAP**
honestly named — the environment may lack physical iOS/budget-Android hardware, so SP-2/SP-3/SP-4/SP-5
carry explicit `Blocked` arms rather than emulated pretend-passes (the same honesty as P39 §4.6's
"no real-device matrix exists" GAP); **ALREADY-EQUIVALENT** — the `gpu`-gated typed-absence pattern
(`GpuError`) is reused for the surface path, one concept one authority.

---

## 5. DoD — measurement-oriented, falsifiable, RED→GREEN (item 2)

**Not a feature checklist.** The DoD is: **each of the six spikes has a committed `VerdictRecord` with
a real `measured` value (or an honest `Blocked`) and a ruling**, and the aggregate is reconciled
against P39 §1.2. RED today: `P63-VERDICTS.md` does not exist; no harness exists; no `winit`/`tauri`
code exists (§0 grep = 0). GREEN at close: the table below is filled with real numbers.

| Spike | Hypothesis (one line) | Measurement method | Falsifiable pass/fail bar | Named artifact (RED→GREEN) |
|---|---|---|---|---|
| **SP-1** desktop | winit+wgpu+AccessKit meets budget, native a11y tractable, no webview needed | `FrameProfiler` distribution + input-event↔present-counter latency + screen-reader capability checklist | p95 frame ≤ 16.7 ms, p99 ≤ 33 ms; p95 input ≤ 50 ms; AccessKit announces focus+role+value+caret | `tools/shell-spike/desktop/` + verdict row |
| **SP-2** mobile surface | raw-wgpu-in-Tauri is usable, not "rough" | 30-min soak per variant (native surface vs WebGPU-in-webview); flicker-frame count vs `compose()` ref | 0 flicker frames; p95 ≤ 33 ms; survives ≥10 bg/fg cycles; **or** `Blocked{device}` | `tools/shell-spike/mobile/` + verdict row |
| **SP-3** payment bridge | native SDK sheet coexists with wgpu surface (Path B feasible) | ≥20 present→dismiss cycles in test mode; dropped frames, recovery time, surface integrity, input routing | ≤3 dropped frames; ≤250 ms recovery; zero corruption; input to sheet — **or** `Contradicts`→Path C | SDK-sheet plugin + verdict row |
| **SP-4** web keyboard | no clean web-mobile keyboard mechanism; voice is interim | candidate matrix on real iOS-Safari + Chrome-Android (VirtualKeyboard API, hidden-focus, forwarding host) | a candidate passes only if keyboard+input+no-visible-DOM+a11y-intact on BOTH; else `Refines`=voice interim | `tools/shell-spike/webkbd/` + verdict row |
| **SP-5** battery | FE-14 settle makes a full shift sustainable on budget Android | scripted 6 h synthetic shift, `batterystats`, **settle-ON vs settle-OFF A/B**, thermal + frame soak | settled ≤ 4 %/h; settle saves ≥30% vs off; sustained ≤ 33 ms no throttle — **or** `Blocked{device}` | scripted-shift driver + verdict row |
| **SP-6** floor parity | one reusable method proves WebGL2/CPU render correctly | perceptual ΔE / (1−SSIM) of WebGPU + WebGL2 + CPU rungs vs bit-deterministic `compose()` reference | every rung ΔE ≤ 0.02 on the scene corpus; WebGPU-only effect fails by construction | `engine/tests/floor_parity/` + `web/tests/floor-parity.spec.mjs` (durable) |

**Aggregate DoD (the reconciliation step):** a top-of-file **verdict summary** in `P63-VERDICTS.md`
maps each spike to `Confirms`/`Refines`/`Contradicts`/`Blocked` **against the four consumers**:
P39-rev §1.2 (shell ruling), P60 (client payment leg — SP-3), P71 (battery gates — SP-5), and the
now-closed §4-A operator decision (SP-1+SP-3 are its measured evidence). Every `Refines`/`Contradicts`
carries the one-line delta the consumer must apply.

**Not-done clauses (item 17 ratchet):** the phase is NOT done if any spike row's `measured` is an
estimate rather than a real reading or an honest `Blocked`; if SP-5 carries an `HwClass::Emulator`
number; if a `Contradicts` verdict lacks a matching dated refinement block filed into P39-rev; if
SP-6's parity harness passes a deliberately-WebGPU-only effect (its own adversarial case i); or if any
spike prototype leaked into the default (non-`spike`-feature) build graph. Ledger rows added to
`docs/regressions/REGRESSION-LEDGER.md`: **SP-6 floor-parity gate** (permanent — the durable one),
**spike-feature-isolation** (product graph must not compile spike code), **no-emulator-battery**
(review-grep on the verdict file).

---

## 6. Benchmark plan (item 10) — the benchmark IS the deliverable

Unlike a feature blueprint where §6 is a small honest measurement, **for P63 the benchmarks are the
entire product.** The protocols:

- **Frame time (SP-1/SP-2/SP-5):** the existing `FrameProfiler` (`engine/src/bridge.rs:20`) records
  per-frame present time; report p50/p95/p99 over the stated soak, not a mean (tail latency is what a
  courier feels). Baselines to beat are stated as constants (§2), grounded in R1 §1's GPUI/VS-Code
  reference band and P38 §6's 30 FPS floor — no invented numbers.
- **Input latency (SP-1):** log a monotonic counter at the input event and at present-completion; the
  diff distribution is the keystroke/tap→pixel latency. GPUI's 2 ms is the aspirational reference,
  50 ms p95 the honest bar (R1 §1).
- **Battery (SP-5):** `dumpsys batterystats` → Battery Historian, mAh + %/h, with a **render-idle
  baseline subtracted** so the figure is GPU drain not panel drain, and the **settle-ON/settle-OFF
  A/B** as the FE-14-saving measurement (the one R1 §9 risk #3 demands). Physical device only.
- **Parity (SP-6):** perceptual ΔE / (1−SSIM) per rung vs the deterministic `compose()` reference;
  reported per scene in the corpus, gated at 0.02.
- **Telemetry hook:** each `VerdictRecord` is appended to `P63-VERDICTS.md` (git-versioned) so a
  re-run on new hardware shows up as a diff, not a silent overwrite — the regression surface for the
  device matrix. No live CI gate on device numbers (they'd flake / need hardware); the **SP-6 parity
  gate is the one CI-runnable, permanently-gated** benchmark (headless-capable via the CPU reference).

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (P63 scope row — the authority),
§3.2 pt 1 (first-among-equals rationale), §2 X3 (payment↔shell coupling — SP-2/SP-3), §2 X2 (web
keyboard — SP-4), §4-A (the CLOSED decision P63 evidences), §4-E (named engineering unknowns) ·
`BLUEPRINT-P39-app-shell-installability.md` §1.2 (the directional ruling P63 confirms/refines; P39
names P63 as its evidence source — this is the reciprocal file) · `BLUEPRINT-P38-webgpu-render-engine.md`
§2/§3.5 (FE-14 settle predicate + Lyapunov watchdog — SP-5), §3.6 (FE-16 ladder — SP-6), §6 (frame
budget) · `OPUS-R1-INTERFACE-RENDERING-2026-07-18.md` §1 (GPUI/latency refs), §2 (AccessKit native/
web split — SP-1), §6 (Tauri surface conflict + battery — SP-2/SP-5), §9 risks #3/#7/#9 ·
`OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md` §6 (three card-capture paths), §9 risk #3 (native-SDK bridge
— SP-3) · `CORE-ROADMAP-STANDARD-2026-07-17.md` (the 20-point contract) · `docs/design/ARCHITECTURE.md`
(F12 offline canon — the CPU floor SP-6 protects) · `docs/regressions/REGRESSION-LEDGER.md`. Ground-
truth code: `kernel/src/render/gpu.rs`, `kernel/Cargo.toml:57`, `engine/src/{bridge,field_frame,
field_energy,motion}.rs`, `engine/Cargo.toml:34-39`, `wasm/src/lib.rs:57`. Memory:
`ground-truth-over-proxy-2026-07-07` + `verified-by-math-2026-07-07` (why SP-5 forbids asserted
battery numbers), `performance-priority-over-minimal-change-2026-07-17` (perf spikes justify real
measurement effort), `gaussian-splatting-address-picker-arc-2026-07-16` (budget-Android / WebGL2-
primary device canon SP-5/SP-6 target), `rust-native-bare-metal-decision-2026-07-14` (DECART
discipline §2). **Feeds:** P39-rev (confirms/refines the shell ruling), P60 (client payment leg —
SP-3 gates Path B vs C), P71 (battery gates — imports SP-5's baseline), and the §4-A evidence record.
**Supersedes:** nothing — it measures decisions already made.

---

## 8. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the `VerdictRecord` schema and the falsifiable bars (§2) precede
  every prototype — the *bar is the spec of done*, not the code that measures it.
- **P4 POLARITY** (same axis, opposite poles): the verdict is explicitly two-poled —
  `Confirms`↔`Contradicts` on one axis — and a `Contradicts` is not a failure of the spike but its
  most valuable success (it saves four downstream blueprints from a wrong guess).
- **P6 CAUSE-AND-EFFECT** (determinism as law): SP-6's oracle is the bit-deterministic `compose()`
  (`field_frame.rs:430`); SP-5's FE-14 saving is a controlled A/B (settle-ON vs settle-OFF), not a
  narrative; measured distributions, not single anecdotes.
- **P7 GENDER** (paired verification): every spike carries a negative control — SP-1's no-adapter CPU
  degrade, SP-5's settle-OFF worst case and render-idle baseline subtraction, SP-6's
  deliberately-WebGPU-only effect that MUST go red. A pass with no failing twin is not trusted.

(Other principles not load-bearing here; not claimed decoratively.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh `file:line` cites; the real `gpu.rs` bring-up + `compose()` oracle + zero-shell-code grep) |
| 2 DoD | §5 (measurement matrix — hypothesis/method/bar per spike + aggregate reconciliation, not a feature list) |
| 3 spec/event-driven TDD | §2 spec-first (schema + bars before prototypes); §3 each spike RED (no verdict) → GREEN (real number); verdicts assert on the measured *distribution*, not one sample |
| 4 predefined types/consts | §2 (`VerdictRecord`, `SpikeVerdict`, `Platform`, `HwClass` + all falsifiable bars as named consts) |
| 5 adversarial/breaking tests | §3 (each spike's adversarial trio: no-adapter degrade, resize/rotation storms, dismiss-mid-animation, settle-defeat worst case, deliberately-WebGPU-only effect) |
| 6 hazard-safety as math | §4.1 (five unreachable-state arguments incl. "faked number has no representation") |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 (device matrix + soak + sample-rate axes; honest fixed-size non-axis for the verdict file) |
| 9 Linux discipline | §4.6 (REINFORCES measure-don't-assume; honest hardware GAP → `Blocked` arms, not emulated passes) |
| 10 benchmarks+telemetry | §6 (the benchmarks ARE the deliverable; git-versioned verdict file as the regression surface; SP-6 the one CI-gated) |
| 11 isolation/bulkhead | §4.3 (`spike` feature-gate keeps failure out of the product graph; independent lanes) |
| 12 mesh awareness | §4.4 (device-local, zero wire, stated) |
| 13 rollback/self-heal | §4.5 (Self-Termination = deletable feature-gated crate; Self-Healing claimed for the FE-16 ladder SP-6 proves; Snapshot-Re-entry refused) |
| 14 error-propagation gates | §4.3 (feature-gate, no-emulator-battery grep, SP-6 determinism precondition, Contradicts-needs-refinement not-done clause) |
| 15 living memory | §4.6 (append-not-overwrite verdict rows with `captured_utc` + device; move-not-delete history) |
| 16 tensor/spectral | §4.6 (honest partial N/A — the one reduction is SP-6's perceptual-delta norm over the `compose()` frame; no `eqc-rs` form invented) |
| 17 regression ledger | §5 (three rows: SP-6 floor-parity permanent gate, spike-feature-isolation, no-emulator-battery) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §1.1 (extends `gpu.rs` bring-up + `GpuUploadSink` seam, not a second renderer), §2 (rejected alternatives) |
| 20 Hermetic citations | §8 (MENTALISM/POLARITY/CAUSE-AND-EFFECT/GENDER, load-bearing) |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Six independent lanes; **SP-6 is the only durable output (a kept gate)** — SP-1..SP-5 prototypes live
under `tools/shell-spike/` and are deleted at close (their verdicts survive in `P63-VERDICTS.md`).
Mobile lanes (SP-2/SP-3/SP-4/SP-5) require real device/toolchain access — if absent, file an honest
`Blocked{on:…}` verdict, never an emulated pass. All work is `/root/dowiz` (no bebop-repo files here).

0. **T0 (scaffold).** Create the `tools/shell-spike/` workspace member behind a `spike` cargo feature
   (beside `kernel/Cargo.toml`'s OFF-by-default `gpu` feature — never in the default graph). Create the
   empty `docs/design/CORE-ROADMAP-2026-07-17/P63-VERDICTS.md` with the §2 `VerdictRecord` schema and
   the §5 summary table skeleton. Acceptance: `cargo build --workspace` (no `spike` feature) does NOT
   compile the spike crate.
1. **T1 (SP-1, desktop).** `tools/shell-spike/desktop/` — extend `kernel/src/render/gpu.rs`'s device
   bring-up to a `winit` present loop + AccessKit tree over one `compose()` scene + one text field.
   Measure frame p50/p95/p99 (`FrameProfiler`), input latency, AccessKit capability checklist. Include
   the no-adapter CPU-degrade adversarial. Acceptance: a real verdict row with numbers vs §2 bars +
   `Confirms`/`Refines`/`Contradicts`.
2. **T2 (SP-2, mobile surface).** `tools/shell-spike/mobile/` — Tauri scaffold, two variants (native
   wgpu surface vs WebGPU-in-webview), 30-min soak, flicker-frame count, bg/fg cycles. Acceptance:
   verdict row per platform, or `Blocked{on:"iOS Mac+provisioning / Android NDK+device"}`.
3. **T3 (SP-3, payment bridge).** Extend T2's scaffold with a Tauri plugin invoking Stripe/Adyen native
   sheet in **test mode** (test key, no PAN, no charge, no adapter code). ≥20 present→dismiss cycles;
   dropped frames, recovery, surface integrity, input routing. Acceptance: verdict row (a `Contradicts`
   here reroutes P60's mobile client leg to Path C — flag it loudly), or `Blocked`.
4. **T4 (SP-4, web keyboard).** `tools/shell-spike/webkbd/` — static page, wasm `compose_field` canvas,
   each keyboard-raise candidate behind a flag, driven on **real** iOS-Safari + Chrome-Android (real-
   device cloud acceptable; devtools mobile-mode is NOT). Acceptance: a candidate passes only on BOTH
   platforms with no visible DOM input + a11y intact; else `Refines`="voice interim" (a real, X2-
   sanctioned answer), or `Blocked`.
5. **T5 (SP-5, battery).** Reuse T2's Android scaffold + FE-14 toggle + scripted-shift driver. On a
   **physical budget Android device** run the 6 h synthetic shift, `batterystats`, settle-ON/OFF A/B,
   thermal soak, render-idle-baseline subtraction. Acceptance: real %/h + saving-delta vs §2 bars, or
   `Blocked{on:"physical budget Android / first-client fleet"}` — **never an emulator number.**
6. **T6 (SP-6, floor parity — DURABLE).** Land the reusable harness in `engine/tests/floor_parity/`
   (Rust reference-frame generator reusing `compose()`) + `web/tests/floor-parity.spec.mjs` (forces
   WebGPU / WebGL2 / CPU rungs via the FE-16 flags). Make `engine/Cargo.toml`'s empty `webgl`/`webgpu`
   boundaries functional enough to capture a frame. Gate at ΔE ≤ 0.02 vs the CPU reference; include the
   deliberately-WebGPU-only-effect-must-fail adversarial. Acceptance: green on the scene corpus + the
   one-line DoD string other blueprints paste; add the permanent REGRESSION-LEDGER row.
7. **T7 (close-out).** Fill `P63-VERDICTS.md`'s summary: map every spike to a verdict **against
   P39-rev §1.2, P60, P71, and §4-A**. For each `Refines`/`Contradicts`, file the dated refinement block
   into `BLUEPRINT-P39-app-shell-installability.md` (evidence governs). Delete `tools/shell-spike/`
   (verdicts persist; prototypes do not). Add the three ledger rows. Do NOT mark P63 done if any
   `measured` is an estimate, if SP-5 used an emulator, if a `Contradicts` lacks its P39-rev
   refinement, or if any spike prototype leaked into the default build graph.

**Forbidden in this phase (for the zero-context reader):** no production UI of any kind; no real card
data / live charge (test-mode SDK only, no card-data type constructed — PCI red-line); no adapter code
(P60's); no text-editing engine (P57's); no web a11y mirror (P58's); no asserted/estimated benchmark
number written as measured; no emulator battery figure; no `winit`/`wgpu`/spike code in the default
(non-`spike`) build graph; no re-deciding the operator's directional shell ruling (measure it, and if
it's wrong, file the evidence — don't silently overrule).
