# R-SON — Sonification Architecture (living-interface arc)

> Status: **v1 (2026-07-16)**. Grounded architecture research (design, NOT implementation). Every
> visual signal/event/state-change in the living interface (the Sea/order wave UI **and** the
> living-memory 3D viz) also emits its **own pleasant sound** (piano/strings-like, not alarms),
> where sound propagation follows the signal's movement and its interaction with other signals.
> Companions consumed: `../ARCHITECTURE.md` (V2 canon), `../rust-engine-rewrite/BLUEPRINTS-RUST-ENGINE-REWRITE.md`
> (**RW-09** thin-shell boundary, **RW-05** numeric zero-JSON shell), `../dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md`
> (**DZ-05** Green's feedback vocab, **DZ-04** OrderStatus→Море, **DZ-10** input modalities), `../physics-ui-capture-blueprint.md`
> (§1 Laplacian unification — the coherence/interference operator), `EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`
> (§5, operator-supplied third-party research — DSP architecture reused, JS-default reconciled here).
> Tags: **GROUNDED** (verified in code/corpus) · **REUSE** (existing anchor extended) · **NOVEL** (design hypothesis).
> Companion R-LM (living-memory viz) not yet written at time of writing — this design is deliberately
> general: audio is a renderer of the field's forcing `S(t)`, so **any** producer of `S(t)` impulses
> (order/wave UI events *or* living-memory viz signals) drives it identically.

## 0. Thesis (one sentence)

**Sound is the THIRD renderer of the ONE field** — DZ-05 already says "ripples + particles = 2
renderers of 1 field"; this design adds *sound = the 3rd renderer of that same field's forcing
`S(t)`* — synthesized by **Rust/wasm DSP running inside an `AudioWorkletProcessor`** (Web Audio is a
browser host API bound the same way `wgpu` binds WebGPU), driven by the **same** event-source
impulses and the **same** coherence/interference operator that already move the visuals — so audio
adds no new physics, no new event bus, and (like reduced-motion) no load-bearing information.

---

## 1. The Rust/wasm-vs-browser-API boundary (resolved, with RW-09 citation)

### 1.1 The structural claim — audio = GPU, exactly

Web Audio (`AudioContext`, `AudioWorklet`, `AudioWorkletProcessor`, `PannerNode`, `GainNode`) is a
**browser host API with no Rust-native equivalent** — structurally identical to how WebGPU is a
browser host API that `wgpu` binds into from Rust (`physics-ui-capture-blueprint.md` §3: "`wgpu` = the
sole graphics dependency … *is* the WebGPU implementation in Firefox/Servo/Deno"). The house rule
"zero-JS-math" is about **math/logic reimplementation in JS**, not about browser host-API surfaces
that have no Rust equivalent (`EXTERNAL-RESEARCH` header note; ARCHITECTURE V2). Therefore audio must
be built with the **same two-layer split the GPU/render path already uses**:

| layer | GPU/render path (existing) | Audio path (this design) | rule |
|---|---|---|---|
| **thin JS host membrane** (irreducible) | wgpu bootloader, `requestAdapter`, canvas wiring, rAF arm/cancel (RW-09) | `new AudioContext()`, `audioWorklet.addModule(url)`, `PannerNode`/`GainNode` graph wiring, user-gesture unlock, `MediaStream` (reverse mode only) | **RW-09 membrane** — "cannot be Rust" |
| **the MATH, in Rust→wasm** (never JS) | field operator `c²LU`, ζ=1 integrator, SDF, particle physics — compiled to wasm, run in the render loop | oscillators, ADSR envelopes, Karplus-Strong (pluck/piano), bowed additive/FM (strings), biquad filters, granular synth, scale-quantizer, coherence mixer — compiled to wasm, run **inside** `process()` | **RW-09 invariant 10** — "domain that thinks → Rust; JS = membrane + data" |

The `AudioWorkletProcessor` JS is a **~30-line trampoline** (the audio analogue of RW-09's wasm
bootloader): instantiate the DSP wasm module, and on every 128-frame render quantum call
`wasm.process(outPtr, 128)` then hand the wasm output view straight to the output `Float32Array`.
This is the **same zero-copy numeric boundary RW-05 already codifies** for the render path — RW-05:
"JS reads NO copy NO parse (`Float32Array(memory.buffer, ptr, len)`) → writeBuffer." Audio is the
same pattern in the render-*out* direction: `Float32Array(wasmMemory.buffer, outPtr, 128)` → audio
output bus. **Never JSON in the audio render quantum** (RW-05 / RW-09 invariant 2).

### 1.2 What is rejected, and why (DECART)

- **Faust / any JS synthesis library (Tone.js, etc.) — REJECTED.** The external research's default is
  Faust→WASM or a JS DSP lib (`EXTERNAL-RESEARCH` §5/§6). Adopting Faust would put synthesis math in a
  **second DSL + toolchain outside Rust**, duplicating the math authority and violating
  Rust-native-default + single-authority (ARCHITECTURE §3 patterns; `rust-native-bare-metal-decision`).
  A Rust `#![no_std]` DSP crate keeps **ONE math authority** alongside `field-math`. Falsifiable DECART
  ground: wasm-in-AudioWorklet is proven; no second synthesis language is needed; Karplus-Strong /
  short additive-FM are ~tens of lines each and cheap enough for real-time wasm at 48 kHz.
- **Legacy `use-sound.ts` — DELETE (it is the JS duplicate).** GROUNDED: the legacy UI ships
  `apps/web/…/.ignored_ui/src/hooks/use-sound.ts` — pre-recorded MP3s (`/sounds/notification.mp3`,
  `success.mp3`, `error.mp3`, `order-update.mp3`) played via `new Audio()` per component. That is
  sample-based, per-component, no propagation/interaction/pitch/pan — the audio equivalent of the
  per-component visual feedback DZ-05 unifies and of the `channel.js`/`money.ts` JS mirrors RW-02/RW-03
  delete. This design **supersedes** it (same delete-the-duplicate story). Its 4 event names
  (notification/success/error/order_update) map 1:1 onto DZ-05's event classes — clean migration.

### 1.3 Flag to RW-09 (a boundary it does not yet cover)

**RW-09 currently under-specifies "Audio."** RW-09's CURRENT enumerates 15 Web-API categories
"…WebSpeech/Vibration/**Audio**/NetworkInfo…" and treats each as a single "shim." But audio needs the
**same split the GPU path got**, and RW-09 does not yet draw it. Two additions RW-09 must absorb:

1. **Split "Audio" into (a) host membrane and (b) DSP crate.** (a) `AudioContext`/`addModule`/`PannerNode`
   wiring = thin-shell membrane (belongs in RW-09's boundary module, next to Push/WebGL). (b) a new
   Rust **`audio` DSP crate** (sibling of `field-math`, `#![no_std]`+alloc, wasm32-clean, zero-dep)
   compiled to a **THIRD wasm artifact** — after `dowiz_kernel` (JSON, transactional) and
   `dowiz_engine` (numeric, per-frame). Reason it must be its own artifact: **the `AudioWorkletProcessor`
   runs in a separate realm (`AudioWorkletGlobalScope`) with no DOM and no access to the main-thread
   wasm instance's memory** — it must load and instantiate its own wasm. This is a boundary RW-09's
   "two wasm artifacts" model (RW-01) does not cover; sonification makes it three.
2. **The `money_guard` invariant extends into the audio crate.** `physics-ui-capture-blueprint.md`
   guardrail 1 / `engine/src/money_guard.rs`: `Money` is never a field value, never tweens. The audio
   crate reads money as a **discrete neutral event-tick only, never as a continuous pitch/gain
   parameter** — a price change is one tick, never a glide that could imply a value (no sonified
   digits), exactly as DZ-02 forbids the visual money-tween. 🔴 red-line, listed again in §5(d).

---

## 2. Event → sound mapping (extends DZ-05, does not replace it)

### 2.1 The core rule (REUSE)

Audio consumes the **same `S(t)` forcing impulses** the visual field already consumes — DZ-05:
"кожна дія = field source … ripples + particles = 2 renderers 1 field." The impulse stream is the
one RW-05's shell already emits: **`on_event(kind:u32, count:u32)`** (+ FieldPos from the pointer/
marker path). There is **no separate audio event bus** — audio is a third read-out of the one field.
Three orthogonal mapping axes keep it musical, not cacophonous:

- **Timbre = event CLASS** (what *kind* of thing happened) — struck/plucked vs bowed vs granular.
- **Pitch = event IDENTITY/POSITION** (which node/order/region), **quantized to a bounded scale**
  (pentatonic — external research §5: "pentatonic/diatonic avoids dissonance"). Pitch is never free.
- **Volume/density = magnitude/count**; **Pan/movement = live FieldPos via `PannerNode`.**

Timbre recipes (both cheap enough for real-time wasm, per external research §5):
**piano-like = struck/plucked** — Karplus-Strong or short additive with **fast attack + exponential
decay**; **string-like = bowed** — short additive/FM with **slower attack + sustain**.

**Venue key (extends the "one accent → 4 placements" coherence).** DZ-02/RESEARCH-CONSPECT: one brand
accent already drives 4 visual placements (Sheet / Sea-tint / spectral / backdrop). Add a 5th: the
**musical key/mode** derived from the same brand seed (default Cosmo-Noir → warm pentatonic-minor,
non-alarming). One brand → coherent look *and* sound, zero manual tuning.

### 2.2 DZ-05 / DZ-04 event → sound table

| event (DZ-05 / DZ-04 anchor) | field source | timbre (synth) | pitch (scale-locked) | vol / density | pan / motion |
|---|---|---|---|---|---|
| **tap → δ ripple** (ζ0.6) | δ impulse | struck pluck (Karplus-Strong, fast attack, exp decay) — piano-like | degree by tap FieldPos.x | soft, single note | pan = tap.x |
| **add-cart → ingest pulse** toward cart-attractor | directed pulse (advected U̇) | plucked note that **glissandos** toward the cart tonic | item degree → cart tonic | medium | pans item→cart position |
| **success / delivered → HEAT bloom** | Gaussian source | bowed strings (slow attack, sustain), **resolves to a consonant chord** (major / added-6th) | root+3rd+5th of key | swells then decays | centred, wide stereo bloom |
| **error / reject → high-λ shake** | high-eigenvalue source | muted / detuned **short low pluck** — a damped "thud," a muted dissonant interval (♭2), **NOT a harsh alarm** | lowered degree, low octave, fast-damped | short, low | at the erroring element |
| **loading → sustained source** | sustained source | bowed drone, single tone, no attack transient | tonic pedal | quiet bed, fades in | centred |
| **order-placed → amber burst** | impulse burst | 3-note **rising arpeggio** (pluck) | ascending triad | medium burst | centred |
| **anomaly → agitation** (DZ-05 / DZ-09 dashboard) | agitation field | **granular** texture + tremolo, unresolved (beating/detune) | cluster around tonic | rises with anomaly magnitude | diffuse |

Continuous OrderStatus → Море (DZ-04) → continuous **audio bed** (the "tide" is a slow pad):

| OrderStatus (DZ-04) | Море param | audio bed |
|---|---|---|
| Pending/Confirmed/Preparing/Ready | ember drift, energy 0.3 | low ambient drone, dark timbre, slow LFO |
| InDelivery / PickedUp | teal swirl 1.6 | drone brightens + gentle tremolo (courier motion → tremolo rate) |
| **Delivered** | gold bloom, burst 1.8 | **consonant resolving chord** (the success cadence); terracotta→gold color-travel = **key brightening** (minor→major / pitch rises) |
| Rejected / Cancelled | blood swirl 3.4 | **dissonant muted cadence**, drone detunes then damps to silence |
| illegal transition | red recoil | the muted error-thud (never schedules a "next" that can't happen — see §5b) |

### 2.3 "Propagation follows movement & interaction" = the EXISTING coherence operator (REUSE, Tier-2 gated)

The task hypothesis holds: the sound-interaction rule **reuses the operator already designed for
visual ripple/interference** — `physics-ui-capture-blueprint.md` §1: *"UI ripple / interference:
quantum walk `e^{−iHt}` · coherence `|ψ₁±ψ₂|²` (Tier-2, gated)."* No separate physics.

- **Propagation = advection (already in the field).** Each active signal is a wavepacket ψᵢ with a
  carrier pitch + phase, **advected along the same U̇** that moves its particle tracers (DZ-05:
  "particles = tracers advected U̇"). Its grains/notes are triggered along that path and **panned to
  its live FieldPos via `PannerNode`** — so a signal's sound literally *moves through the stereo/3D
  field following its visual motion* (a courier's note pans with the marker; a memory-node's tone
  moves as the node drifts). "Propagation follows the signal's movement," grounded in existing advection.
- **Interaction = coherence.** When two signals' supports **cross/overlap** (the same |∇U| intersection
  the visual field already computes), fire the coherence term instead of two unrelated notes:
  - **Constructive `|ψ₁+ψ₂|²`** (compatible events) → the two carriers **snap to a consonant interval**
    (3rd/5th) locked to the scale, and their combined amplitude briefly blooms.
  - **Destructive `|ψ₁−ψ₂|²`** (opposed events, e.g. a success crossing an error) → both **mute to a
    beat/null**.
- **Full-cycle resolution.** A completed cycle (order delivered) → the wavepacket train **cadences to
  the tonic consonant chord**; error/anomaly → **resolves to the muted/dissonant null**. This is the
  auditory twin of "terracotta→gold bloom" vs "blood swirl."

**Gating:** this interaction layer is **Tier-2, gated on the SAME flag as the visual coherence layer**
(`physics-ui` §1 "Tier-2, gated") — Phase-0 ships the per-event table (§2.2) with pan-following
(propagation) but **without** the crossing/coherence mixer, exactly as the visual field ships ripples
before the gated interference visuals.

### 2.4 DZ-10 note (voice INPUT ≠ audio OUTPUT)

GROUNDED: DZ-10 defines `Intent`/`FieldPos`/`InputSource` with a **VoiceSource** — but it is
**input only** (wake-word → command grammar → `Command` → resolver → Navigate/Select). DZ-10 says
**nothing about audio output**. There is an `InputSource` trait but **no `OutputSink` equivalent**.
Sonification is that missing output counterpart. Flag: the input/output abstractions are asymmetric
today; this design fills the output side and should eventually be named alongside DZ-10 (e.g. a
`FieldSink`/`AudioSink` sibling that consumes `S(t)` the way `InputRouter` produces Intents).

---

## 3. Zoom-tier consistency (mesh / hub / node) — the anti-cacophony design

The 3-tier data model (mesh/hub/node — ARCHITECTURE M-series; operator-decided from day one, Phase-0
= hub only) maps to an **"orchestral distance" model**. The invariant: **the same event keeps the same
pitch-class + timbre at every tier — only its PRESENCE (audible-note vs summed-into-pad) and its
SPATIAL treatment (near/dry vs far/reverberant) change with zoom.**

| tier | what you hear | mechanism |
|---|---|---|
| **MESH** (zoomed out, aggregate) | a slow **ambient chord BED** — individual events are NOT audible, they are summed into the pad's texture | pad root = mesh health; voicing density = # active hubs; brightness/register = aggregate throughput; dissonance = aggregate anomaly rate. Distant = reverberant + low-pass (`PannerNode` distance model + send reverb). This is the external research's **firing-rate → drone-density / population-rate** mapping at mesh scale. |
| **HUB** (Phase-0 target) | individual events become audible **NOTES** per §2.2 — the "melody" tier (one hub's order lifecycle) | reverb dries up, low-pass opens ("moved closer"). |
| **NODE** (zoomed all the way in) | a single **sustained detailed tone** — a "readout" of one order / one kernel `decide()` / one courier | continuous timbre: e.g. ETA-confidence → filter cutoff, progress → pitch glide (the external research's *membrane-potential → timbre* mapping). Near-field, no reverb, full presence. |

Zoom is a **continuous "camera distance"** that crossfades pad-bed (mesh) → note-stream (hub) →
single-voice readout (node). The anti-cacophony guarantee is structural: **at scale, events do not
each get a note — they are FOLDED into the pad** (information preserved as density/brightness, not
lost — parity with §4). A hard **voice budget** (≤ ~8–12 simultaneous audible notes) caps polyphony;
overflow is summed into the tier's pad, never dropped-silent.

---

## 4. Reduced-audio parity (mirrors the existing standing law)

The existing law (RESEARCH-CONSPECT "STANDING LAWS" + coherence rule 9): *"reduced-motion first-class
never load-bearing (Sea → static gradient, state legible via pills/color/text)"* / *"reduced-motion
never loses meaning."* Audio inherits it **verbatim**, and it is **true by construction** because audio
is a *third redundant renderer of the same field* — every sound already has a visual/text twin (DZ-05
ripple + DZ-02 `<Money>` + status pills + OrderProgress stepper). **Audio off ⇒ zero information lost.**

- **Audio is NEVER the sole carrier of any state.** Same argument as reduced-motion; enforced by the
  same reliability-gate visual trace (L0–L11).
- **Independent toggle, three levels, not a binary.** (a) **Off** — an explicit `prefers-reduced-audio`-
  style setting *and* an honoring of OS `prefers-reduced-motion` as a *default hint* (calm-visual users
  often want calm audio) but with its **own** switch (a user may want ambient sound with reduced motion,
  or vice-versa). (b) **Calm/reduced-density** — keep the ambient bed, sonify only high-salience events
  (delivered, error), fold the rest into the pad (this reuses the §3 mesh summarization and doubles as
  the §5c scale rate-limiter). (c) **Full.**
- **Autoplay law is the accessible default.** Browsers block `AudioContext` until a user gesture, so the
  default is **silent-until-you-opt-in-by-interacting**. The DZ-01 Shell's first tap (arrive act) is the
  natural unlock point. Passive/kiosk/SSR views therefore start silent — the design must not rely on
  audio being present on load (flag, §5d).
- **Mobile power parity.** Suspend the `AudioContext` on `visibilitychange` (tab hidden) and after N
  seconds idle (suspend the drone), resume on activity — the same "tab-hidden pause" the codebase's
  `use-courier-marker` already does.
- **Falsifiable gate (RED→GREEN):** with audio disabled, an automated pass reads every order state from
  the visual/text channel; audio adds **no new required assertion**. (RED: a test asserts state only via
  sound → design-illegal. GREEN: state fully legible with `AudioContext` never resumed.)

---

## 5. Friction / joint map  ⚠️ (the honest risks)

### (a) 🔴 MOST IMPORTANT — two clocks, and the COOP/COEP deployment wall

**The problem.** The `AudioWorklet` runs on a **dedicated real-time audio thread** clocked by the audio
hardware (128-frame quanta, ~2.7 ms @ 48 kHz, sample-accurate); the field/render loop runs on the
**main thread's rAF** (~16.6 ms, vsync) driving wgpu. They are **different clocks** — a visual pulse
and its sound can drift and de-sync.

**Resolution.** Do **not** trigger sound from the main thread's rAF (jittery + a postMessage hop).
Share the `S(t)` impulse stream into the worklet and let the **audio thread schedule sound at sample
accuracy against `currentTime`**; the visual field reads the same `S(t)`. Both are driven by one event
stream, each on its own clock, converging to the same perceived instant without either thread blocking.

**The wall (GROUNDED).** The low-jitter transport is **`SharedArrayBuffer` + `Atomics`** (external
research §5/§6) — but **`SharedArrayBuffer` requires cross-origin isolation (COOP/COEP headers)**, and
**dowiz sets NONE today**: verified — no `Cross-Origin-Opener-Policy` / `Cross-Origin-Embedder-Policy` /
`crossOriginIsolated` / `SharedArrayBuffer` reference anywhere in `apps/`, `packages/`, `web/`, the
`fly*.toml`, or the hot `apps/api/src/routes/spa-proxy.ts` (only MapLibre's own bundle matches). So
`crossOriginIsolated` is **false** and SAB is **unavailable**. Enabling it is a **new deployment
requirement AND not free**: `COEP: require-corp` forces **every cross-origin subresource** to send
CORP/CORS or it **breaks** — dowiz loads **MapLibre tiles** and **R2 entrance photos** cross-origin, so
turning COEP on could break the map and photos until each origin is proxied or CORP-tagged (touches the
99.4th-percentile-churn `spa-proxy.ts`).
- **Recommendation:** **Phase-0 uses `port.postMessage` (structured-clone) transport — needs NO
  cross-origin isolation, zero deploy change** — accepting ~one-frame extra jitter (fine for ambient/
  musical events, which are not sample-critical like pro-audio). **SAB is a later optimization gated on
  a deliberate COEP migration** (proxy/CORP-tag MapLibre + R2 first). This is the single most important
  friction point because it is a **cross-cutting deployment + security change, not a local code choice.**

### (b) Event ordering / causal inversion under network jitter

**The problem.** The server streams events (order lifecycle, mesh gossip) over WS; network jitter can
reorder delivery so an **"error" arrives before the "attempt"** it belongs to → the ear hears the
error-thud *before its cause*, which sounds wrong. Audio makes causal violations **more** perceptible
than visuals, because sound is inherently temporal/sequential.

**Resolution (reuse existing authority).** Schedule sound off the **kernel-validated monotonic state
sequence**, not raw wire-arrival order. The field already has the authority: DZ-06's **local event-log +
`fold_transitions` replay** + the **kernel `order_machine`** (10 states, validated locally,
illegal→red recoil) + event-sourcing sequence (S9). The audio scheduler buffers a small **jitter window**
(~80–150 ms, one musical "tick"), emits in **causal order**, and **folds** an out-of-order predecessor
that arrives after its successor already sounded (a late "attempt" after its "error" is folded, not
replayed). The kernel's illegal-transition guard means an acausal pair **never schedules the second
sound**. No new ordering system — `order_machine` + `fold_transitions` already exist.

### (c) Volume / density explosion at scale

**The problem.** At real order volume (owner dashboard; mesh tier) there are far more events than can
each get an audible note → cacophony.

**Resolution.** The §3 tier model + a hard **voice budget** (≤ ~12 voices) + a per-tier **salience
filter**; **overflow events are SUMMED into the tier's ambient pad** (density/brightness), never
each-sonified — the external research's population-rate→drone-density mapping, which also *is* the
reduced-audio "calm mode" (§4b). Rate-limit **per event class**: coalesce N taps within a ~50 ms window
into ONE grain-cluster whose density encodes N (mirrors DZ-05's reduced-motion "burst × 0.25").

### (d) Other joint risks found

- **🔴 money-never-sonified (red-line).** The field drives audio and the field carries money values; a
  naive "pitch = field value" would sonify money. Guard: audio reads money as a **discrete neutral
  event-tick, never a continuous pitch/gain parameter** — the audio analogue of `money_guard` / DZ-02's
  no-money-tween. Extends `engine/src/money_guard.rs` into the `audio` crate (§1.3).
- **Autoplay-unlock gesture.** `AudioContext` is blocked until a user gesture; a passively-viewed
  living-memory viz (kiosk/SSR/no-click) stays silent. Needs an explicit "enable sound" affordance; the
  DZ-01 Shell arrive-tap is the natural unlock. **Do not rely on audio on load.**
- **Second wasm instance / small audio model.** The worklet realm cannot share the main-thread engine's
  wasm memory — the DSP wasm is a **separate instance/artifact** (§1.3) receiving only the compact `S(t)`
  impulse stream + a few aggregates, **not** the full field (external research: "reduce first, ship a
  small buffer"). Keep the audio-side model small; do not mirror the field into the audio thread.
- **Battery/CPU (mobile).** Continuous synthesis + a live `AudioContext` drains battery — the §4 suspend-
  on-hidden/idle rule is a requirement, not a nicety.

---

## 6. Phase-0 recommendation (smallest real, falsifiable slice)

**Sonify ONLY the existing single-hub order-lifecycle (DZ-04 OrderStatus→Море), reusing the already-
shipped particle VOCAB event stream, on the HUB tier only, via `postMessage` transport (no COEP change),
with a Rust `audio` DSP crate compiled to a 3rd wasm artifact loaded by ONE `AudioWorklet`.** This
serves the general order/wave UI now; the living-memory viz plugs into the same `S(t)`→audio renderer later.

**In scope**
- **Delete** legacy `use-sound.ts` (4 canned MP3s) → replaced by the synthesized field renderer (same
  delete-the-JS-duplicate story as RW-02/RW-03).
- The 5 particle-VOCAB events + the 6 OrderStatus states → the §2.2 tables. **One venue key** from the
  brand accent. Struck pluck (Karplus-Strong) + bowed additive/FM + one granular texture (anomaly).
- `PannerNode` follows the CourierTrack marker FieldPos (propagation). Delivered → consonant cadence;
  Rejected → muted dissonance.

**Explicitly OUT of Phase-0** (deferred, each gated exactly like its visual twin)
- Mesh + node tiers (§3) — hub only. · The coherence/crossing interaction (§2.3) — Tier-2, **same gate
  as the visual `|ψ₁±ψ₂|²`**. · Living-memory viz signals (kernel `decide()` / gossip). ·
  `SharedArrayBuffer` / COEP migration (§5a) — stays on `postMessage`. · Audio-reactive FFT reverse mode.

**Falsifiable RED→GREEN gate (corpus style; offline-testable, no "sounds nice" human loop)**
- **RED:** legacy `use-sound.ts` plays a canned MP3 — no propagation, no pan, no interaction; event
  order = wire order; delivered and rejected sound unrelated.
- **GREEN:** ONE `AudioWorklet`+wasm renders the order lifecycle from the **same `S(t)` stream** as the
  visual field, with assertions —
  1. **0 pre-recorded audio files loaded** (grep: no `.mp3`/`new Audio(`; `use-sound.ts` deleted).
  2. Render the worklet output buffer offline to a WAV; **FFT asserts** the **delivered** event resolves
     to a **consonant interval** (3rd/5th present) and **rejected** to a **muted dissonant cadence**
     (♭2 / damped) — the mapping is measured, not vibes.
  3. The courier note's **stereo pan tracks the marker FieldPos.x** (assert L/R balance follows position).
  4. Under **injected 200 ms WS jitter**, events schedule in **kernel-validated causal order** (assert:
     the error sound never precedes its attempt — §5b).
  5. With `AudioContext` never resumed, **every state stays legible** via the visual/text channel (reuse
     the reliability-gate L0–L11 trace — §4).

This is honest and small: it changes **no deployment headers**, reuses shipped machinery (particle
VOCAB, `order_machine`, `fold_transitions`, DZ-04, the marker FieldPos, `money_guard`), and both the
order/wave UI and (later) the living-memory viz consume the identical `S(t)`→audio renderer.

---

### Key anchors cited
RW-09 (thin-shell membrane; "Audio" under-specified — split flagged) · RW-05 (numeric zero-JSON shell,
`on_event`) · RW-01 (two→**three** wasm artifacts) · RW-02/RW-03 (delete-the-JS-duplicate precedent) ·
DZ-05 (Green's feedback vocab; "2 renderers 1 field" → **3**) · DZ-04 (OrderStatus→Море continuous) ·
DZ-02 / `money_guard` (money never tweens → never sonified 🔴) · DZ-01 (Shell arrive-tap = audio unlock) ·
DZ-10 (voice = INPUT only; **no OutputSink** — asymmetry flagged) · `physics-ui-capture-blueprint.md` §1
(coherence `|ψ₁±ψ₂|²` / quantum-walk `e^{−iHt}`, Tier-2 gated — **the reused interaction operator**) ·
EXTERNAL-RESEARCH §5 (AudioWorklet+WASM DSP, pentatonic, PannerNode, population-rate drone, SAB/COOP-COEP)
· RESEARCH-CONSPECT (reduced-motion standing law → reduced-audio parity). GROUNDED facts: `use-sound.ts`
sample-based legacy; **no COOP/COEP anywhere** in app/deploy config (SAB unavailable today).
