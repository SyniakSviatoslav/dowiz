# BLUEPRINT — Intent Interface: One-Screen Owner and Customer Surfaces

**Doc id:** BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20
**Status:** RESEARCH SYNTHESIS / PLAN — no code written, nothing built in this pass. One exception to the usual risk framing: Stage A below is a wiring task between two endpoints that *both already exist and are tested* (`engine/src/intent.rs` + `engine/src/compose_ui.rs` on one side, the working CPU+WASM render path in `wasm/` + `web/src/app.mjs` on the other). Its technical risk is essentially zero; its risk is scope discipline only.
**Inputs synthesized:** (1) operator-provided Report 1 — browser GPU neural-field rendering + sonification; (2) operator-provided Report 2 — native AR compositing stack; (3) live code audit of `main`; (4) prior rulings this session: `BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md` (AR Phase 1 = `<model-viewer>` Quick Look), the media/comms synthesis (granular human-or-agent dual-mode per capability; red-line actions never agent-operable), and the concurrency ruling (agent/UI lane stays synchronous, no tokio).
**Out of scope for this doc:** full roadmap wiring — that is a separate, later step per the operator's request. This document is the design synthesis it will wire.

---

## 1. Correction first: there is no "one operator" — and this design does not pretend there is

The design corpus claims "ONE physical operator MÜ + ΓU̇ + c²LU = S draws the whole UI" (recall, decay, layout, motion, blur unified under one wave equation). **This claim is false.** It was falsified by internal audit `docs/research/AUDIT-2026-07-18-FEYNMAN-meta-patterns-math.md`, finding FEYNMAN-01 (CRITICAL): the corpus "canon" is in fact four different equations, and two of the five claimed unifications have no code at all. Verified against the tree:

| Claimed unified subsystem | What actually exists | Where |
|---|---|---|
| Grid field ("the wave") | First-order **semi-implicit screened diffusion** (not a second-order wave equation), Lyapunov-monotone energy decrease proven, stability bound fixed | `engine/src/field_frame.rs::FieldFrame::step` |
| Motion | Genuine second-order damped oscillator, **per-property, uncoupled — no Laplacian in it** | `engine/src/motion.rs::Spring` |
| Modal rendering | A third dynamical system: damped eigenmode advance over the kernel's spectral basis | `engine/src/field_modal.rs` |
| Blur ≡ heat equation | **No corresponding code.** Prose analogy only | — |
| Unified "recall" | **No corresponding code** in the engine; retrieval is a separate personalized-PageRank system | — |

The true, verifiable story — and the framing this document uses throughout — is:

> **dowiz has ONE shared, well-tested Laplacian primitive — `kernel/src/csr.rs::laplacian_spmv` — reused honestly across several genuinely different dynamical systems** (a grid PDE, a graph-particle bridge, a modal eigenbasis renderer, and a separate retrieval system). One shared primitive, several honest dynamics.

This is not a weaker story. A shared primitive with distinct, individually-proven dynamics is *more* defensible than a claimed grand unification: each system carries its own stability proof, and a bug in one does not silently imply a bug in the others. Every subsequent section that says "the Sea" means "the screened-diffusion `FieldFrame` substrate plus its impulse-source vocabulary," never "the one wave equation."

---

## 2. What the intent interface already is

The operator's requested pattern — one screen, dynamically composed from classified intents, near-zero chrome — is not a proposal to be designed from scratch. It is built and tested on `main`:

- **Input unification.** `engine/src/intent.rs` (P64 M1): `RawInput` (pointer / key / voice / gesture) is the only event type anything downstream ever sees — enforced by a grep-gate test, not convention.
- **Classification.** `IntentClassifier::classify` is a pure function (proven pure by test) producing `Intent` — `Point`, `Impulse`, `Select`, `Navigate`, `Scrub`, `Command`.
- **The money firewall.** `Intent::is_consequential()` (`intent.rs`) hard-separates money/destructive intents (confirm/accept/cancel/decline order, checkout). The `ambiguous_never_auto_commits` test proves a consequential intent can never come out of ambiguity resolution or an optional ranker.
- **Composition.** `engine/src/compose_ui.rs` (P64 M2): `Composer::compose(&Intent, &AppState) -> ComposedResponse` maps the intent to a `FragmentRegistry` lookup (Menu / Cart / Catalog / Checkout / OwnerDashboard / CourierBoard / ConfirmWell), each fragment a pure `fn(&AppState) -> Vec<SdfShape>`, merged into one `Scene` plus a `FieldParams` delta fed to the diffusion substrate. The `consequential_intent_never_bare_commits` test proves consequential intents always attach friction (a confirmation gate) instead of committing.
- **Production wiring.** `engine/src/engine_loop.rs::EngineLoop::frame` drives this in the real run loop, proven by a RED→GREEN integration test. 122/122 engine tests green.
- **Waiting kernel counterparts.** Customer side: `kernel/src/storefront.rs` `JourneyStep` FSM (Storefront → Menu → Detail → Cart → Fulfillment → Payment → Placed), whose module doc already names "the full-wgpu intent UI" as its target front end. Owner side: `kernel/src/ports/owner_surface.rs` (P70 — zero I/O, fold + capability-cert-signed), currently a port with no front end.
- **Voice.** `engine/src/voice.rs` (feature `voice`, ASR body stubbed pending network grant): a voice input can only ever emit an unresolved `RawInput::VoicePhrase` and cannot mint a commit token — it passes the same friction gate as any input. Voice therefore needs *no new safety design* in this blueprint; it is already just another `RawInput` variant.

What does **not** exist: any Sea & Sheet code. `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md` (DZ-01..12) fully specifies the visual grammar — Sea (ambient field backdrop, order status drives color/energy/swirl, terracotta→gold progression, red recoil on illegal transitions), Sheet (fluid-damped brand content surface, 5-token brand model: accent/ink/paper/type-pair/radius), the three-act arrive→choose→receive shell, `<Money>` never tweens, DZ-05's unified field-source impulse feedback, and DZ-10 already naming `intent.rs` as its input layer. Zero of it is implemented in `engine/` or `web/`. **The gap this blueprint closes is exactly that seam: classified-intent composition (built) × Sea&Sheet grammar (designed) × the live CPU+WASM render path (working).**

There is also no incumbent to migrate: the React/Vite app was deleted, and `web/index.html` is already a single page with no router — a dev console showing the real `field_frame` canvas and real kernel FSM buttons. The one-screen constraint is not a refactor; it is styling and composing a surface that is already structurally one screen.

---

## 3. Owner surface: one screen, recomposed by intent

### 3.1 The zero-chrome rule, stated falsifiably

The owner surface has **no nav bar, no sidebar, no tab strip, no modal stack, no route changes**. This is achievable — not aspirational — because every function those elements normally serve is already covered by an existing mechanism:

| Chrome element it replaces | Replaced by | Mechanism |
|---|---|---|
| Nav bar / tabs / routes | Intent → recomposition | `Composer::compose` swaps the active fragment in the same `Scene`; DZ URL-state rule keeps back-button/state-restore without page loads |
| Modal / dialog stack | ConfirmWell fragment + Sea impulse | Confirmation is a *composed Sheet state* (§3.5), never a DOM overlay |
| Status widgets / badges / toasts | Sea parameters | Ambient state lives in the field, not in components (§3.2) |
| Sidebar entity lists | The single active fragment | One fragment visible at a time; the previous fragment's state is preserved in `AppState`, not in hidden DOM |
| Spinners / progress bars | Field maturation | DZ tracking-as-field-maturation; consistent with the established "never a spinner, never fake urgency" ethos |

Falsifiable form: the rendered document contains exactly one composition root; no `history` pushes that trigger loads; no element with modal/overlay semantics; grep-gate on the web shell for router/modal identifiers (same enforcement style as the existing `RawInput` grep-gate).

Input surface: a single omnipresent intent line (type or, post-unlock, speak) plus direct manipulation on rendered shapes. Both paths produce `RawInput`; nothing downstream can tell them apart — this is already enforced.

### 3.2 Owner Sea parameters — what exists, what drives it

The Sea is the `FieldFrame` screened-diffusion substrate. Its owner-surface parameter contract (all delivered through the existing `FieldParams` delta channel from `ComposedResponse` — no new channel):

| Sea parameter | Driven by | Source of truth | Notes |
|---|---|---|---|
| Base hue / temperature | Aggregate order-status distribution of the active queue | `order_machine.rs` fold state via `owner_surface.rs` port | DZ terracotta→gold ramp, applied per-order as localized warm regions, not one global tint |
| Energy (source amplitude) | Order throughput: decide/fold events per recent window | Kernel event stream | Quiet shop = near-still field; rush = visibly energetic. Ambient information, never load-bearing |
| Impulse (Green's-function kick) | Every discrete event: order arrival, status advance, staff message, courier hand-off | DZ-05 unified feedback vocabulary | One vocabulary for all feedback; no per-component animation code |
| Red recoil | Illegal FSM transition attempts | `order_machine.rs` errors (errors, not silent no-ops) | The existing DZ spec; kernel error text also surfaces on the Sheet — the field alone never carries required information |
| Swirl / flow bias | Staff activity (active staff sessions, comms volume) | Comms surface from the media/comms synthesis | Directional, low-amplitude; distinguishable from order energy by motion character rather than color |
| Damping shift ("held breath") | One or more `PendingIntent`s awaiting confirmation | PendingIntent flow (media/comms synthesis) | The field visibly stills while a decision is pending — the inverse of urgency theater (§3.5) |
| Agent-activity shimmer | Agent-mode capability currently executing | Autonomy config + agent lane (`agent-facade` boundary) | Subtle periodic texture in the region of the affected fragment; honest "something is acting on your behalf," not an alert |

Hard boundary, restated: **the Sea is never load-bearing.** Every fact an owner needs to act on appears as Sheet content (text, integer money, explicit state labels). The Sea encodes only redundant ambient texture. This keeps the surface accessible (reduced-motion users lose nothing) and keeps `<Money>`-never-tweens trivially satisfied — money never enters the field at all.

### 3.3 Fragment vocabulary and intent mapping

Minimal owner vocabulary — seven fragments, each a pure `fn(&AppState) -> Vec<SdfShape>` registered in the existing `FragmentRegistry` (the current `OwnerDashboard` fragment becomes the resting composition; the rest are new registrations in the same table, not a new mechanism):

1. **OwnerPulse** (resting state) — today's queue summary, takings (integer cents, direct render), anything pending. What exists when no intent is active.
2. **OrderQueue** — live orders with per-order FSM state and legal next transitions (only legal ones render — the FSM decides, the fragment reads).
3. **MenuEditor** — item/price/availability editing against the catalog surface.
4. **Comms** — staff/customer messages, per the media/comms synthesis surface.
5. **LedgerView** — read-only double-entry view from `kernel/src/money.rs`. Read-only by construction: red-line capabilities deny by default (`kernel/src/ports/agent/scope.rs`, `RedLinePolicy::DenyByDefault`); mutation is not a fragment capability at all.
6. **BrandTokens** — the DZ 5-token brand editor (accent/ink/paper/type-pair/radius), one change propagating everywhere.
7. **ConfirmWell** — already exists in `compose_ui.rs`; unchanged; the only path a consequential intent can take.

Intent → composition (illustrative; all rows are the same screen recomposing):

| Owner input (typed / spoken / tapped) | Classified `Intent` | Composition result |
|---|---|---|
| "orders", tap on an order region | `Navigate` / `Select` | OrderQueue fragment; Sea impulse at selection point |
| "menu", "change the price of X" | `Navigate` / `Command` | MenuEditor, focused on X where the phrase resolves it |
| "messages", "reply to the last customer" | `Navigate` / `Command` | Comms fragment |
| "accept order 14" | `Command`, `is_consequential() == true` | ConfirmWell attached — friction, never a bare commit (existing D5 guarantee) |
| "money", "today's takings" | `Navigate` | LedgerView (read-only) |
| Ambiguous phrase | `Ambiguous` candidate set | Non-consequential candidates rendered for one-tap disambiguation; the existing `ambiguous_never_auto_commits` proof means no money action can hide in this set |
| Scroll/drag inside a fragment | `Scrub` / `Point` | Intra-fragment state change, no recomposition |

### 3.4 The human-or-agent signal — without a mode-switcher

The media/comms synthesis established that every capability is owner-configurably human- or agent-operated, with red-line/money actions structurally excluded from agent operation. The interface consequence is one rule:

> **Every rendered actionable element carries a two-state operator mark** — a small glyph in the element's own SDF geometry (part of the fragment's pure output, not a separate widget): one form for "you operate this," one for "your agent operates this." Nothing else. No toggle on the surface, no settings panel embedded in the screen.

Properties that keep this honest and chrome-free:

- It is **read-only on this surface**. Changing an assignment is itself an intent ("let the agent handle menu replies") that routes through classification — and reassignment of any capability is consequential, so it takes the ConfirmWell path. Configuration state lives in the media/comms synthesis's autonomy config; this surface renders it, never owns it.
- **Red-line actions render no mark at all.** Absence is the signal: an unmarked action is human-only *by structure* (the agent lane cannot name kernel mutation — `agent-loop` imports only `agent-facade`, which does not re-export `decide`/`fold`/stores). Rendering a "human/agent" choice on an action the agent can never touch would be a false affordance.
- When an agent-assigned capability is *currently acting*, the Sea shimmer (§3.2) localizes near its fragment. The owner sees delegation happening without a log window; the actual log remains available as Comms/LedgerView content on request.

### 3.5 `PendingIntent` as a first-class field state — not a popup

When an action awaits confirmation (an agent proposing something above its autonomy grant, or the owner's own consequential intent), the surface enters a composed state that the DZ three-act structure already anticipates:

- **Arrive:** the triggering event lands as a Sea impulse at the source fragment's location — same DZ-05 vocabulary as every other event, so pending confirmations are not a special visual dialect.
- **Choose:** the Sheet recomposes to ConfirmWell — full statement of the action, exact integer amounts, who initiated (human/agent mark), explicit confirm and decline shapes. This is a fragment composition, not an overlay: the screen *is* the confirmation, with the prior fragment's state held in `AppState` for restoration.
- **Receive:** on decision, `kernel::decide` runs, the fold advances, the outcome lands as a Sheet state change plus a closing Sea impulse. On decline, restoration plus a damped null impulse.

While pending, the Sea's global damping rises (§3.2 "held breath") — the field goes calm, not loud. A pending decision is the one moment the surface should *reduce* stimulation; this is the direct inverse of dark-pattern urgency, consistent with the session's no-fake-urgency ethos. Multiple simultaneous `PendingIntent`s queue in ConfirmWell as an explicit ordered list — never stacked layers, preserving the one-layer invariant.

---

## 4. Customer surface: same architecture, plus scoped atmosphere

### 4.1 The Journey on one screen

The customer surface is the same stack — `RawInput` → `IntentClassifier` → `Composer` → Sea&Sheet — with `kernel/src/storefront.rs`'s `JourneyStep` FSM as the authority for what may compose. Each step maps to a fragment (Storefront hero / Menu / Detail / Cart / Fulfillment / Checkout / Placed-tracking); the Sheet recomposes per classified intent; DZ URL states preserve back-button behavior without page loads. The kernel FSM — not the UI — decides which advances are legal, so the interface cannot render an illegal shortcut.

Two steps get special treatment:

- **Checkout is deliberately the *least* atmospheric moment.** Sea energy drops to near-still, no particles, no impulses beyond the outcome itself. All salience concentrates on the Sheet: items, integer prices, the confirm gate. `Intent::Command(Checkout)` is consequential, so `consequential_intent_never_bare_commits` applies verbatim — the confirmation gate *renders*; nothing auto-commits. Atmosphere that increases arousal at the payment moment would be a dark pattern; this design does the opposite.
- **Placed / tracking is the *most* atmospheric moment** — the DZ "tracking as field maturation" idea, made concrete in §4.3.

### 4.2 Report 2 triage: what applies, what is deferred, what is reused sideways

The operator's criterion — "where it is needed and actually increases experience atmosphere" — resolves Report 2 into three disjoint buckets. Stated plainly up front: **Report 2's full stack is a native-app product decision that has not been made, and this document does not make it.**

**Bucket 1 — already decided, ships without Report 2's stack.** The customer's first AR moment is the sibling blueprint's Phase 1: `<model-viewer>` auto-routing to AR Quick Look (iOS) / Scene Viewer (Android) — "see this dish on your table" from the Detail fragment. No app install, no camera pipeline of ours, production-viable today. That ruling was made *because* Report 2's own evidence closes the browser path for true camera compositing: WebXR `immersive-ar` absent from iPhone Safari, 8th Wall's hosted SLAM shut down (core SLAM binary closed and unmaintained), and the WebXR↔WebGPU binding still an unstable Editor's Draft. Nothing in this synthesis reopens that.

**Bucket 2 — Report 2's actual AR stack: deferred behind an explicit, unmade native-app decision.** Zero-copy camera import (CVPixelBuffer / AHardwareBuffer), 6DoF planar tracking (homography → IPPE → Levenberg-Marquardt → One-Euro filter) or ARKit/ARCore VIO, depth-only proxy-geometry occlusion, the stencil-buffer portal with per-object `escape`, and the degrade-to-match-camera compositing pass all require a native companion app (Rust + wgpu behind thin Swift/Kotlin FFI shells). That is a separate product with its own distribution, maintenance, and support cost — and Report 2's own closing caveat cuts against rushing it: real-time is only worth the investment if live interaction is genuinely the point. For a food storefront, whether pointing a phone at a table beats a 10-second pre-rendered dish video is an open question, not a premise. **Flagged as open decision (ii), §8.** Until decided, no code, no blueprint-writing, no dependency on it anywhere in Stages A/B.

**Bucket 3 — Report 2 techniques that are not AR-specific and are cheaply reusable in the browser.** These are pure math/shader/discipline ideas that enhance the *already-loaded storefront page*, independent of any camera:

- **Curl noise** (divergence-free procedural flow) for ambient particles over the Sea — organic drift without clumping or dispersal, exactly the quality wanted for tracking-view atmosphere. Implementation is Report 1's WebGPU compute path (§5), not Report 2's native stack.
- **LUT-based palette unification.** Report 2 uses a shared color-grade LUT to pull disparate CG into one palette. Mapped here: derive a small grading LUT *from the DZ 5-token brand model*, applied to the Sea's final composite — so every hub's ambient field is automatically graded into that hub's brand palette. One brand-token change re-grades the whole atmosphere. This gives per-hub visual identity with zero per-hub art direction.
- **Degrade-to-match as a design discipline, not a feature.** Report 2's strongest claim: matching the imperfection of the surrounding context buys more perceived quality than adding rendering techniques. Translated: the Sea sits *behind photographic food imagery* on the Sheet, so it gets subtle grain, restrained bloom, and no clean-vector plastic look — tuned to sit with photographs, not compete with them. This is a shader-parameter discipline written into DZ acceptance criteria, costing nothing.
- **2.5D layered planes over full 3D.** Report 2's preference for layered alpha-plane "diorama" content validates the Sea/Sheet architecture itself — Sheet planes rising fluid-damped over the field backdrop *is* a 2.5D composition. Noted as confirmation that no full-3D scene graph is needed; no new work.
- **Explicitly not reused:** the boiling-line / low-frame-rate NPR outline technique. It sells "hand-drawn character" in Report 2's context; against photographic food content it reads as glitch, and it fails the operator's "actually increases atmosphere" test here. Cut, with the reason stated.

### 4.3 Where atmosphere concretely appears in the customer journey

| Journey moment | Atmosphere | Driven by | Path |
|---|---|---|---|
| Storefront arrival | Calm brand-graded Sea (LUT from brand tokens), very low curl-noise particle density | Static hub identity | Stage A (CPU field) → Stage B (particles) |
| Menu browsing | Faint Sea impulse under touch/scroll (DZ-05 Green's-function feedback) | `Intent::Point`/`Scrub` | Stage A |
| Dish Detail | Sheet-dominant; `<model-viewer>` AR affordance on items with a 3D asset | Sibling blueprint Phase 1 | Independent of Sea work |
| Add to cart | Single localized impulse from the item's position toward the cart region | `Intent::Command(AddToCart)` composition | Stage A |
| Checkout | Deliberate stillness (§4.1) | Consequential-intent gate | Stage A |
| **Placed / tracking** | The centerpiece: Sea as maturation field. Particle density and field energy = f(order progress through the kernel FSM); hue walks the terracotta→gold DZ ramp per status; each status advance (Placed→Accepted→Preparing→PickedUp→Delivered) lands one distinct bloom-pulse impulse | Real `order_machine.rs` fold events — the customer watches the actual FSM, ambiently | Stage A (hue/energy on CPU field) → Stage B (particles + bloom) |
| Delivery arrival | Field reaches its gold resting state; one final warm impulse; no confetti, no gamification | Terminal FSM state | Stage A/B |

The load-bearing channel is always the Sheet (explicit status text, times, courier info). The Sea restates it ambiently. A customer with reduced-motion or a WebGL2-only device loses zero information.

---

## 5. Report 1 reuse: a GPU upgrade path for the substrate that already renders

Report 1's stack maps onto dowiz with one substitution: dowiz is Rust-first, so the implementation lane is Report 1's own "Rust + wgpu → wasm32" option, not Three.js/TSL (which would drag a JS framework into a `web/` shell whose whole point is zero dependencies). What is adopted is Report 1's *architecture*: compute pass → render pass → post (bloom/tone map) → small reduction buffer → audio (§6). WebGPU-primary with WebGL2 fallback matches Report 1's January-2026 baseline assessment; the CPU+WASM canvas blit that works today (`wasm/` wrapping `field_frame::compose`/`FieldSim`, `web/src/app.mjs`) remains the universal fallback below even WebGL2.

The decisive code-grounded fact: **none of this is blocked on design.** `engine/Cargo.toml` declares `gpu`/`webgl`/`webgpu`/`splat` as deliberate empty feature seams; `engine/src/bridge.rs::gpu::new_gpu` returns an honest `Err("gpu adapter not built — wgpu uncached")`; a pinned regression test asserts the feature stays off *by default* — off by default, not impossible. The blocking doc names the single trigger: operator grants network for `cargo add wgpu`. That one action (open decision (i), §8) unblocks, at once:

- GPU execution of the *same* `FieldFrame` screened-diffusion step as a WGSL compute kernel (same math, same tests as oracle — the CPU implementation becomes the parity reference, in the same spirit as the `eqc_gen.rs` parity pin);
- curl-noise particle simulation in storage buffers (Report 1's proven scale: 100k–1M particles at 60 fps is far beyond the few thousand this design needs, so headroom is not a concern — the constraint will be aesthetic restraint, not throughput);
- the emissive-HDR + selective-bloom + tone-mapping post chain for the status-advance pulses;
- the GPU→small-summary-buffer reduction that sonification consumes.

Report 1's Izhikevich/LIF spiking-neuron content is **not** adopted as simulation. dowiz does not need a neuron model; it already has real discrete events with real meaning — FSM transitions. What is adopted from that section is only the *mapping pattern* (discrete spike → audio trigger; rate → density), which is §6. Honesty note: this synthesis reuses roughly half of Report 1 (pipeline, compute particles, bloom, sonification architecture, fallback strategy) and deliberately discards the neuroscience payload (morphology, connectomes, membrane dynamics) as content without a purpose here.

Feature discipline (repo rule) applies unchanged: everything in this section lands behind the existing off-by-default `gpu`/`webgpu` features with the standard header comment, and the default engine build stays zero-external-dep. The pinned default-off test already enforces this shape.

---

## 6. Sonification: genuinely interesting, currently absent, honestly speculative

Report 1's distinctive direction — discrete simulation events driving an `AudioWorklet` + WASM DSP synth — happens to fit dowiz's event structure exactly, because dowiz's core *is* a discrete-event system:

- **Spike → grain/note trigger:** each `order_machine.rs` decide/fold transition is a natural spike. Order placed, accepted, picked up, delivered — each a distinct timbre; a day of operation becomes a sparse, legible sound texture.
- **Rate → density/pitch:** queue throughput maps to grain density — the owner *hears* a rush building without looking.
- **Distinct alarm class:** Hydra's breach-alert signaling is exactly a high-salience discrete event. Sonification would be a strictly read-only *consumer* of Hydra's existing alert output — nothing about Hydra's chartered design is touched or reconciled (standing operator ruling; restated here so no future reader mistakes this for a Hydra change).

Scope, stated bluntly:

- **Speculative, nice-to-have, not core.** Named because it is a real, cheap-once-GPU-lands idea that nothing in the current corpus covers — not because any journey depends on it.
- **Owner-toggleable, default off, per hub.** An ambient audio layer some owners will love in a kitchen and others will kill in an hour.
- **Never load-bearing.** No information exists only as sound — the same rule as the Sea (§3.2). Consistent with never-a-spinner/never-fake-urgency: sound restates, it never nags.
- **Deferred behind the same network-grant unlock as the GPU work** (decision (i)): the DSP path (Faust→WASM or Rust DSP) needs uncached dependencies, and the clean architecture consumes the GPU reduction buffer from §5. One unlock gates the whole (b) tier; no separate audio decision needed unless the operator cuts sonification entirely (decision (iv)).

---

## 7. Build sequence

### Stage A — buildable now, zero new dependencies (highest leverage, lowest risk)

Wire what exists: `intent.rs` + `compose_ui.rs` → `wasm/` bridge → `web/`, replacing the dev console with the Sea&Sheet-composed surface on the existing CPU `field_frame` path. Implement the fragment vocabulary (§3.3, §4.1) as pure functions in the existing `FragmentRegistry`; implement the DZ Sea/Sheet visual grammar (brand tokens, terracotta→gold ramp, impulse vocabulary, checkout stillness) against `FieldParams` and the existing RGBA compose path; connect `storefront.rs::JourneyStep` (customer) and `ports/owner_surface.rs` (owner). Both endpoints of every wire already exist and are tested; the new code is fragments, styling, and glue.

**Falsifiable acceptance (RED→GREEN, each lands with a failing test first):**

- **A1 — full journey, one screen.** A Node-harness (`web/ npm test` lane) run completes a real order through every `JourneyStep` (Storefront→…→Placed) driving *only* `RawInput` events into the composed interface. Assert: zero document loads, zero route changes (URL state per DZ rules permitted), one composition root throughout.
- **A2 — the money gate renders.** The checkout step asserts the classified intent `is_consequential()`, that `ComposedResponse` contains the ConfirmWell fragment *before* any `kernel::decide` call, and that no fold event exists until the explicit confirm input. This is `consequential_intent_never_bare_commits` promoted from unit level to the end-to-end surface.
- **A3 — the Sea tells the truth.** For each order-status fold transition, assert a `FieldParams` delta (hue/energy per the DZ ramp) distinct from the previous state; for an attempted illegal transition, assert the kernel error is returned (not swallowed), the recoil impulse is present, *and* the error text renders on the Sheet.
- **A4 — no leakage, no deps.** Existing `RawInput` grep-gate stays green; new grep-gate on `web/` for router/modal identifiers passes; `cd engine && cargo tree -e no-dev` shows zero new external crates; default-build feature checks unchanged.
- **A5 — PendingIntent is a state, not a popup.** Injecting a pending confirmation composes ConfirmWell within the single Scene; assert no overlay/modal construct in the web shell output, and that declining restores the prior fragment's `AppState` intact.
- **A6 — the operator mark is structural.** For a capability configured agent-mode, its rendered fragment output contains the agent mark; for a red-line action, assert *no* mark shape is emitted (absence-as-signal, §3.4).

### Stage B — gated on one operator action: the wgpu network grant (decision i)

WGSL port of the `FieldFrame` step (CPU implementation retained as parity oracle), curl-noise particle layer, bloom/tone-map post chain, brand-token LUT grading, WebGL2 fallback, and — if not cut by decision (iv) — the sonification reduction→AudioWorklet path. All behind the existing off-by-default features; Stage A's surface keeps working unmodified on machines without WebGPU. Acceptance criteria to be written RED→GREEN at stage start, headlined by CPU/GPU field-step parity within a stated tolerance and a frame-time budget on a named reference device.

### Stage C — gated on a separate, larger, unmade decision: the native companion app (decision ii)

Report 2's full compositing stack (VIO/planar tracking, proxy-geometry occlusion, stencil portal, degrade-to-match pass). No blueprint work proceeds until the decision is made; nothing in Stages A/B depends on it in any direction. If the answer is "web-first indefinitely," the customer AR story remains Bucket 1 (`<model-viewer>`), which is already shipped-quality by design.

*(Roadmap registration of A/B/C is the separate later wiring step named in the operator's request — deliberately not performed in this document.)*

---

## 8. Open decisions for the operator

1. **wgpu network grant — now or later?** One action (`cargo add wgpu` under a network grant) unblocks all of Stage B: GPU field rendering, curl-noise particles, bloom, LUT grading, and sonification. Cost: one heavyweight dependency tree entering the cargo cache, behind off-by-default features with the default build provably unchanged (pinned test already exists). Stage A neither needs nor waits on this. Recommendation implicit in sequencing: grant is not urgent until Stage A's surface exists to put pixels on.
2. **Native companion app — ever?** The precondition for Report 2's full AR stack (Bucket 2, §4.2). This is a product decision — distribution, maintenance, support — not an engineering one, and Report 2's own caveat (real-time only pays if live interaction is truly the point) plus the Lightform precedent already noted in the sibling blueprint both counsel skepticism. Until decided: web-first stands, `<model-viewer>` is the AR story, zero Stage C work.
3. **Owner-configurable vs. fixed Sea behavior.** The DZ 5-token brand model already gives owners color/type identity, and §4.2's LUT derivation extends those same five tokens to grade the atmosphere — no new surface needed for that. Proposed default, pending ruling: exactly two additional per-hub toggles — particle density (off/low/standard) and sonification (off/on, default off) — carried as plain hub config, *not* a new capability surface (they are cosmetic, so the media/comms autonomy machinery would be overkill). Everything else (impulse vocabulary, checkout stillness, recoil, maturation ramp) stays fixed dowiz behavior: it encodes safety and honesty semantics, and making honesty owner-configurable is how dark patterns get in.
4. **Sonification — prototype or cut?** The most speculative element here. Case for keeping as a flagged idea: it is architecturally nearly free once Stage B lands (the reduction buffer exists; the event mapping is a page of code), and it is a genuine differentiator no competitor surface has. Case for cutting now: zero user demand evidence, and it is the only item in this document with no falsifiable value criterion yet. Either ruling is cheap to honor; this document carries it as optional-deferred until ruled.

---

## 9. Consistency ledger

Invariants this design inherits and must not violate, with the enforcing mechanism named:

| Invariant | Enforced by | This design's exposure |
|---|---|---|
| Consequential intents never bare-commit | `consequential_intent_never_bare_commits`, `ambiguous_never_auto_commits` (existing tests) | Extended to surface level (A2); ConfirmWell is the only consequential path |
| Money is exact integers, never floats, never tweened | `kernel/src/money.rs`; DZ `<Money>` rule | Money never enters the Sea; LedgerView renders integers directly (A3/A5 adjacent) |
| Trust = signed capability, never a score | `no-courier-scoring` CI gate; routing enums omit `Ord` | No fragment ranks or rates any participant; nothing in the Sea encodes participant "quality" |
| Red-line capabilities deny by default | `RedLinePolicy::DenyByDefault` | LedgerView read-only; unmarked actions are human-only by structure (§3.4) |
| Agent lane cannot name kernel mutation | `agent-loop` → `agent-facade` compile firewall | Operator marks render config, never grant capability (§3.4) |
| Synchronous agent/UI lane, no tokio | Session concurrency ruling | Nothing here introduces async; AudioWorklet (Stage B) is a browser thread, not a Rust runtime |
| Default builds stay zero-external-dep / serde-free | Feature discipline + pinned default-off GPU test | A4 gate; all Stage B work behind existing features |
| No false unification claims | FEYNMAN-01 audit | §1 is this document's own compliance: one shared `laplacian_spmv` primitive, several honest dynamics — nowhere does this design require the systems to be one equation, and it works precisely because they are not |
