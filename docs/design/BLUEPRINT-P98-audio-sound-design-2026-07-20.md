# BLUEPRINT P98 — Audio & sound design layer (2026-07-20)

> **Greenfield blueprint, grounded in existing code and prior rulings, not invented in a vacuum.**
> Before this document, dowiz's audio design consisted of exactly one accessibility ruling-bullet
> (§16.50, `ROADMAP.md:2867-2869`: "friction is multi-modal, with full audio support — not a
> color/visual-only signal") and one already-built implementation of it scoped to a single gesture
> (P64's friction `AudioParams`). This blueprint is the missing counterpart: it owns non-speech
> sound cues across the rest of the product surface (order-status/event notification sounds,
> the general accessibility-parity requirement) and states, with evidence rather than assertion,
> what does and does not belong on the engine's Laplacian field substrate. Format follows the
> `BLUEPRINT-P64-intent-engine-friction-voice.md` precedent.

---

## 0. Ground truth — every claim re-verified live this pass

| Claim | Cite (verified this pass) | Verdict |
|---|---|---|
| §16.50: "Friction is multi-modal, with full audio support — not a color/visual-only signal (which would fail colorblind users)" | `docs/design/ROADMAP.md:2869-2870` | VERIFIED — this is the entire prior audio spec; scoped to friction only in its own text |
| P64 already builds a full, tested, parity-pinned audio channel **for the friction/consequential-action gesture only**: `AudioParams { pitch_hz, tremolo_hz, hold_ms }`, derived by the *same* functions as the visual `FrictionField` from the *same* `Stake`, plus an objective `a11y_blind_path` test (deaf/blind parity, amount read-back before commit) | `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P64-intent-engine-friction-voice.md:203-205` (types), `:546-559` (§4.3 audio_params formula), `:393-406` (§3.4 M4 a11y gates) | VERIFIED — P98 must not redesign this; it extends the *pattern* to the rest of the visual surface |
| P64 §5 item 16: "the friction field is the **existing Laplacian substrate** — `friction_amplitude` sets the source term into `FieldFrame::step`/`compose`… no new field math is invented" | `BLUEPRINT-P64-intent-engine-friction-voice.md:631-635` | VERIFIED — the one place in this corpus where field-driven audio is *proven*, not speculative |
| The a11y mirror already defines `role=Status` (web: `aria-live="polite"`) and `role=Alert` (`aria-live="assertive"`) — the mechanism through which an OS screen reader (VoiceOver/TalkBack/NVDA/Orca) **already speaks dowiz's status/error text aloud**, with zero dowiz-authored TTS code | `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P58-a11y-mirror-everywhere.md:96-97` | VERIFIED — load-bearing for §4's TTS decision |
| `kernel/src/order_machine.rs`'s `OrderStatus` enum: `Pending, Confirmed, Preparing, Ready, InDelivery, Delivered, Rejected, Cancelled, Scheduled, PickedUp, Refunding, CompensatedRefund` — the exhaustive transition source this blueprint's sound catalog maps against | `kernel/src/order_machine.rs:8-25` | VERIFIED — read live this pass |
| DZ-04 already maps `OrderStatus` → color/energy/swirl: Pending/Confirmed/Preparing/Ready = ember drift (energy 0.3); InDelivery/PickedUp = teal swirl (energy 1.6, "courier motion"); Delivered = gold bloom burst (energy 1.8); Rejected/Cancelled = blood swirl (energy 3.4); illegal transition = red recoil | `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md:117-138` (DZ-04) | VERIFIED — the exact visual vocabulary §5 below derives audio from |
| DZ-05 already maps event→field-source: tap→δ ripple, add-cart→ingest pulse+money snap, success/delivered→Gaussian heat bloom, error/reject→high-λ shake, loading→sustained source, order-placed→amber burst, anomaly→agitation | `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md:140-159` (DZ-05) | VERIFIED — the second half of the exhaustive visual-signal surface |
| The intent-interface synthesis's own honest sonification verdict: "genuinely interesting, currently absent, honestly speculative… Never load-bearing… Owner-toggleable, default off… Deferred behind the same network-grant unlock as the GPU work" | `docs/design/BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20.md:190-203` (§6) | VERIFIED — the prior triage of an operator-supplied external report; its architecture (not neuroscience) was kept as directionally correct, explicitly non-load-bearing |
| That triage traces to a real in-repo research artifact: `docs/design/living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md` (+ `BLUEPRINT-P07-sonification-phase0.md`, `R-SON-sonification-architecture.md`) — an OLDER, differently-numbered ("P07" in that arc's own local scheme, not this roadmap's P-series) prior pass at the same idea | `docs/design/living-interface-2026-07-16/` (file listing, this pass) | VERIFIED — exists; superseded by the 2026-07-20 triage above for anything this blueprint decides |
| Conversational-agent voice **output** (TTS) is an already-RULED BUILD decision, engine = Kokoro-82M via `ort`, in a new `voice-adapters` edge crate, Phase 4 (command loop) / Phase 6 (proactive alerts) — **not this blueprint's to design** | `docs/design/BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md:58` (D-V4), §8 Phase 4/6 | VERIFIED — cross-referenced, not duplicated |
| Proactive spoken alerts (Phase 6) are explicitly ruled to **reuse the existing classical warning/alert set**, not invent a new taxonomy: "Whatever already exists as a visual/text notification… becomes optionally also spoken, per-class toggleable… off by default" | `BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md:288-294` (O6) | VERIFIED — load-bearing: this blueprint's Tier-1/2 catalog (§5) is exactly the taxonomy that ruling refers to; building it well is a direct prerequisite for Phase 6, not a parallel effort |
| `engine/Cargo.toml`: default build is zero-external-crate (only `dowiz-kernel` path-dep); `gpu`/`webgl`/`webgpu`/`telemetry`/`a11y_native` are the existing off-by-default feature pattern this blueprint's `audio` feature must match | `engine/Cargo.toml:1-60` | VERIFIED — read live this pass |
| No audio code exists anywhere in the tree today | `grep -rniI "AudioContext\|audiocontext\|web_sys::Audio" wasm/ web/ engine/` → 0 hits | VERIFIED — greenfield |
| P64's `InputProfile::CourierInMotion` names the voice-primary-in-motion exception (§16.53) — the nearest existing concept to a "hands/eyes-busy" audio-primary context this blueprint's Tier-2 cues also serve | `BLUEPRINT-P64-intent-engine-friction-voice.md:97-98` | VERIFIED — reused, not redefined |

---

## 1. Scope — what P98 owns, and its exact boundary against P64 / the voice-hub arc

**P98 owns:**
1. The **general accessibility-parity principle** of §16.50 — extended from P64's one proven
   instance (friction) to every other visual signal already designed in this corpus (DZ-04 order-
   status, DZ-05 event vocab) — an exhaustive checklist, not a partial gesture.
2. **Non-speech notification/status sound cues** — dings, tones, earcons for order-status
   transitions and event feedback, playable in-app/in-hub.
3. The **architectural verdict** on whether/how audio should ride the existing Laplacian field
   substrate (§3) — the specific question this blueprint's task named.

**P98 explicitly does NOT own** (cross-referenced, never re-decided):
- **Voice input** (wake-word, streaming ASR, `VoiceSource`) — P64, unchanged.
- **Friction's own audio channel** (`AudioParams` for the consequential-action hold gesture) — P64
  §4.3, unchanged; P98 reuses its *pattern*, never its code path.
- **Conversational-agent spoken output** (Kokoro-82M TTS for answering questions, reading order
  status aloud on request) — `voice-adapters`, Phase 4 of the spatial-storefront-voice-hub arc,
  unchanged.
- **Proactive spoken alerts** (Phase 6 of the same arc) — P98 supplies the alert taxonomy that
  phase consumes (§5), but does not build the speech synthesis or the allowlist/rate-limiter
  machinery.
- **Push/SMS/email transactional notifications** (P61, `notify-adapters`) — a different channel
  entirely (out-of-app delivery to a device the customer isn't currently looking at); P98 is the
  in-app/in-hub *sound* layer. The two may fire for the same `OrderStatus` fold event without
  overlapping in mechanism, exactly as P61 itself already draws a boundary against P43/P49
  (`BLUEPRINT-P61-notification-fabric.md` §2.1).
- **Ambient generative sonification of the whole field** (spike-rate mapping of every FSM event
  into a continuous soundscape) — see §3.2; deliberately left where the operator already ruled it,
  not re-opened here.

---

## 2. Why one new item, and why P98

Audio currently has no first-class home: it is a single accessibility bullet with a partial
implementation (P64's friction channel) and a separately-triaged, explicitly-speculative research
idea (the intent-interface synthesis's §6). Both are real but neither is a design of "what dowiz's
sound layer *is*." P98 is that design. Per `ROADMAP.md`'s own confirmed "every P01–P96 either has
a file or is P84" coverage state (verified this pass — `grep` across the whole `docs/design/` tree
finds no P97 or P98 usage anywhere as a roadmap item number; P99/P100 appear only as latency-
percentile notation, never as item numbers), **P98 is genuinely free** and is assigned here.

---

## 3. The architectural question: does audio belong on the Laplacian field?

This is the question the task instructing this blueprint asked to be answered honestly, not
by elegance. Two separate pieces of evidence exist in this corpus, and they point in different
directions for different scopes — the honest answer is "yes, narrowly; no, broadly," not a single
verdict.

### 3.1 The narrow case: proven, load-bearing, already shipping — extend it

P64's friction audio channel is not a proposal; it is a built (blueprinted-and-DoD'd) system where
`audio_params(stake)` and `friction_amplitude(stake)`/`friction_hold(stake)` are **the same
function family reading the same `Stake`**, and the friction amplitude itself is documented as
riding `engine/src/field_frame.rs`'s existing Laplacian source-term mechanism (`FieldFrame::step`).
This is real: the visual and audio channels *cannot* drift from each other because they are
mathematically the same derivation, and the a11y gate (P64 M4) proves a blind user can complete a
consequential action using only the audio channel, at the identical threshold a sighted user meets
visually.

**This blueprint extends that exact pattern — not a new one — to the rest of the visual-signal
surface (DZ-04/DZ-05).** The justification is concrete, not aesthetic: DZ-04 already computes a
`FieldParams`-shaped delta (color/energy/swirl) for every `OrderStatus` transition to drive the
Sea's visual response, and DZ-05 already computes an impulse/source-term event for every
tap/add-cart/success/error/loading/anomaly action. **An audio cue for each of these is a second,
cheap consumer of a quantity the renderer already computes — not a new physics system, a new event
bus, or a new source of truth.** Deriving `EarconParams` (§6) from the same delta the Sea already
animates from is the P64-proven pattern, applied where the data already exists.

### 3.2 The broad case: real prior art, explicitly triaged as speculative — do not adopt it here

The intent-interface synthesis (`BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20.md` §5-§6)
already reviewed an operator-supplied external research report proposing a much larger idea:
turning the *entire* field simulation into a generative ambient soundscape — every `order_machine`
transition as a "spike" triggering a grain/note, queue throughput mapped to grain density, a
GPU-computed reduction buffer feeding an `AudioWorklet`+WASM DSP synth. That prior pass's own
verdict, verbatim: **"genuinely interesting, currently absent, honestly speculative… Never
load-bearing… no information exists only as sound… Owner-toggleable, default off… deferred behind
the same network-grant unlock as the GPU work."** It also explicitly discarded that external
report's neuroscience content (spiking-neuron/Izhikevich/LIF simulation — dowiz doesn't need a
neuron model, it already has real discrete FSM events with real meaning) while keeping only the
*mapping pattern* (discrete event → audio trigger; rate → density) as directionally correct.

**This blueprint does not re-adopt that broader idea as required scope, and does not expand it.**
The reasons are the same ones the prior triage already gave, re-verified as still true: it needs
the wgpu/GPU compute path this repo has not yet unlocked (an operator network-grant gate, unrelated
to audio), it is explicitly non-load-bearing by the operator's own standing framing (the Sea
restates, it never nags — same rule this blueprint's Tier-1 cues must also respect for *sighted*
users, §7), and — the decisive point — **nothing evaluated for this blueprint provides new evidence
that the ambient/generative layer is needed.** The accessibility requirement (§16.50) is fully
satisfiable by the narrow, proven pattern of §3.1; the ambient layer would be aesthetic enrichment
on top, not a load-bearing accessibility mechanism. Building it here, now, because the underlying
math is available would be exactly the "forcing an analogy that doesn't hold" failure mode this
blueprint was asked to guard against.

**Verdict, stated once:** P98 designs and requires **Tier 1/2** (§5) — discrete, parity-pinned,
per-event earcons derived from the same field data the Sea already computes, following the P64
pattern exactly. P98 explicitly does **not** design **Tier 3** (whole-field ambient sonification) —
that stays exactly where the operator already put it: named, optional, deferred, cross-referenced
here (§8) and nowhere re-decided.

---

## 4. TTS — the honest scope call

Three separate questions, three separate honest answers, none of them "build new TTS here":

1. **Does P98 need to synthesize speech for accessibility text announcements?** No. P58's
   `role=Status`/`role=Alert` mirror already routes dowiz's status/error text through the platform
   screen reader (VoiceOver/TalkBack/NVDA/Orca), which speaks it aloud using the OS's own TTS —
   zero dowiz-authored speech synthesis required. This is the single biggest reason P98 does not
   need a general-purpose TTS engine: the accessibility-critical *text* channel already has one,
   for free, at the OS layer.
2. **Does P98 need to synthesize speech for conversational-agent answers?** No — already ruled
   BUILD elsewhere (Kokoro-82M, `voice-adapters`, Phase 4/6 of the spatial-storefront-voice-hub
   arc). Re-specifying it here would fork a decision that already has an owner.
3. **Does P98 itself need any new spoken-word capability for its own notification/earcon layer?**
   No, for Wave-0. Tier-1/2 cues (§5) are non-speech earcons by design — a rising-pitch tone
   communicates "stake is high," a resolved chord communicates "committed," exactly as P64 already
   proved for friction. If a *specific* notification class later needs to become spoken (e.g. "your
   order is ready" announced hands-free in a kitchen), that is precisely what Phase 6's proactive-
   alert allowlist (O6, §0) already covers — "whatever already exists as a visual/text notification
   becomes optionally also spoken." **P98's job is to make sure that existing set (the Tier 1/2
   catalog, §5) is well-defined enough for Phase 6 to point at** — not to build the speech path
   itself.

---

## 5. Predefined types (standard-precedent §2 item 4 — named before implementation)

```rust
// engine/src/sound.rs — pure math, zero I/O, mirrors friction.rs's shape exactly (P64 precedent).
// Behind an off-by-default `audio` feature (matches gpu/webgl/telemetry — engine/Cargo.toml).

/// Every discrete visual signal this corpus has already designed, in one exhaustive enum.
/// Adding a new OrderStatus variant or DZ-05 event WITHOUT adding its SoundEvent arm here is a
/// compile error (non-exhaustive match) — the "unrepresentable-if-incomplete" pattern (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEvent {
    // OrderStatus transitions (kernel/src/order_machine.rs:8-25, DZ-04 vocab)
    OrderPending, OrderConfirmed, OrderPreparing, OrderReady,
    OrderInDelivery, OrderPickedUp, OrderDelivered,
    OrderRejected, OrderCancelled, OrderRefunding, OrderCompensatedRefund,
    IllegalTransition,                       // DZ-04's "red recoil"
    // DZ-05 event vocab (BLUEPRINTS-DOWIZ-INTERFACES.md:140-159)
    Tap, AddToCart, Success, ErrorReject, Loading, OrderPlaced, Anomaly,
    // Derived courier signal (§5.3 — gated, not yet code-grounded)
    CourierNearby,
}

/// Non-visual sound parameters. Deliberately the SAME shape as P64's AudioParams (pitch/rhythm),
/// extended with a waveform/envelope so a synthesizer has enough to render a tone, not just a gate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarconParams {
    pub freq_hz: f32,        // fundamental pitch — derived from the SAME delta driving the Sea
    pub duration_ms: u32,    // envelope length; short for taps, longer for terminal states
    pub waveform: Waveform,  // timbre family, not a sample — see §6 (procedural synthesis)
    pub tremolo_hz: f32,     // 0.0 = none; >0 = urgency signal, same axis as friction's tremolo
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform { Sine, Triangle, Square }   // 3 timbre families = 3 semantic classes, §5.1

/// Registry: SoundEvent -> EarconParams. A `match` over SoundEvent, not a HashMap — the
/// exhaustiveness check IS the accessibility-parity proof (§7's DoD).
pub fn earcon_for(event: SoundEvent) -> EarconParams { /* §5.1/§5.2 tables, below */ }
```

**Rejected alternative (DECART, one line):** sample-based playback (pre-recorded `.wav`/`.mp3`
assets) — rejected for Tier 1/2. Procedural synthesis needs zero asset pipeline, zero binary size
cost, zero licensing surface, and matches the *ad fontes* principle (§16.42) exactly as the visual
field does: math primitives over library/asset dependencies. Sample playback remains available as
a Tier-3/ambient-layer option if that speculative tier is ever built (§3.2) — not decided here.

### 5.1 Tier 1 — accessibility-mandated parity cues (order status, §16.50 REQUIRED)

Derived from the *same* DZ-04 color/energy/swirl delta already computed for the Sea. Three timbre
families map to DZ-04's three semantic clusters so a cue is recognizable by *class* even before a
user has learned the specific mapping (the same "learn through use, no tutorial" principle §16.50
already applies to onboarding, P64 §3.7):

| `SoundEvent` | DZ-04 visual (cite) | `EarconParams` derivation | Semantic class |
|---|---|---|---|
| `OrderPending`/`Confirmed`/`Preparing`/`Ready` | ember drift, energy 0.3 | `Waveform::Sine`, low `freq_hz` (~220 Hz, matches P64's `AUDIO_PITCH_BASE_HZ`), short duration, `tremolo_hz: 0.0` | Calm/progress |
| `OrderInDelivery`/`PickedUp` | teal swirl, energy 1.6 ("courier motion") | `Waveform::Triangle`, mid `freq_hz`, `tremolo_hz` > 0 scaled to swirl energy (motion = audible motion) | Active/in-motion |
| `OrderDelivered` | gold bloom burst, energy 1.8 | `Waveform::Sine`, resolved-consonant `freq_hz` step (same "resolved chord on commit" idiom P64 §4.3 already uses), longest duration (terminal, positive) | Terminal/positive |
| `OrderRejected`/`Cancelled` | blood swirl, energy 3.4 | `Waveform::Square`, detuned `freq_hz` (the same "detune on cancel" idiom P64 already defines), medium duration | Terminal/negative |
| `OrderRefunding`/`CompensatedRefund` | (not yet in DZ-04's table — a real gap DZ-04 itself doesn't cover; flagged, not silently invented) | `Waveform::Sine`, distinct from Delivered's resolved chord (compensated ≠ celebratory) — **exact `freq_hz` is a tuning constant, not derived from an unmapped visual delta; named as an open sub-item, §9** | Terminal/neutral |
| `IllegalTransition` | red recoil | `Waveform::Square`, sharp/short, `tremolo_hz` high (matches "high-λ shake" urgency register) | Rejection/error |

### 5.2 Tier 2 — event/notification cues (DZ-05 vocab)

| `SoundEvent` | DZ-05 visual (cite) | `EarconParams` derivation |
|---|---|---|
| `Tap` | δ ripple, ζ0.6 | `Sine`, very short (<80ms), quiet — a UI acknowledgment, not a notification |
| `AddToCart` | ingest pulse + `<Money>` snap | `Triangle`, short, distinct 2-note interval (add = arrival, not alarm) |
| `Success` | Gaussian heat bloom | shares `OrderDelivered`'s resolved-chord family when it co-occurs with a terminal state; a lighter version otherwise |
| `ErrorReject` | high-λ shake | shares `IllegalTransition`'s square/sharp family |
| `Loading` | sustained source | `tremolo_hz` only, no discrete onset — a continuous low hum, ceases on resolution (never a spinner-equivalent nag; matches DZ-05's own reduced-motion-legible discipline) |
| `OrderPlaced` | amber burst | `Sine`, mid `freq_hz`, distinct from `AddToCart` (a bigger commitment) |
| `Anomaly` | agitation | `Square`, irregular `tremolo_hz` (agitation = audibly irregular, not just loud) |

### 5.3 `CourierNearby` — honestly not yet code-grounded

No proximity signal exists anywhere in the codebase today (`grep -rniI "proximity"` across
`kernel/`, `engine/`, and `docs/design/` finds no prior design). This blueprint does **not**
pretend otherwise. `CourierNearby` is named here as the natural extension of DZ-04's existing
"teal swirl = courier motion" semantic (§5.1) — a geo-distance threshold derived from the courier
tracking math that already exists (`kernel`'s router + the P51/P96 Kalman/EMA speed work) — but
**it is explicitly gated on that geo signal being wired to a threshold event, which is not this
blueprint's scope to build.** Its `EarconParams` (a rising-urgency `tremolo_hz` as distance
decreases, reusing the InDelivery timbre family) is specified so the sound layer is ready the
moment the geo event exists; the event itself is out of scope here.

---

## 6. Implementation shape

**Pure math (engine, off-by-default `audio` feature):** `engine/src/sound.rs` computes
`EarconParams` from the same `FieldParams`/`Stake`-shaped deltas DZ-04/DZ-05 already produce for
the visual channel. Zero I/O, zero external crate — matches `friction.rs`'s shape exactly and
keeps the default engine build at zero external dependencies (verified §0).

**Playback, at the edge, per platform (never inside the deterministic core — the same bulkhead
discipline P64 §5 already applies to `inference/`):**
- **Web:** the browser's native `AudioContext` + `OscillatorNode`/`GainNode` — a **zero-dependency**
  path (no npm package, no vendored asset, built into every browser), consistent with `web/`'s
  render-only/zero-dependency charter even more strictly than Track 1's AR `<model-viewer>`
  precedent (P97 §2.1/O3), since this needs no external file at all.
- **Native (Tauri apps, `apps/courier`):** `cpal` (RustAudio), the same crate the spatial-
  storefront-voice-hub arc already scoped for voice/TTS playback (`voice-adapters`). A new,
  minimal sibling crate (working name `audio-adapters`) or a shared output stream inside
  `voice-adapters` if that crate lands first — this blueprint does not hard-block on
  `voice-adapters` existing, since Tier 1/2 earcons have zero dependency on the voice pipeline.
  DECART rationale: `cpal` is already an accepted dependency elsewhere in this exact arc: adding a
  second consumer of it is not new dependency-surface, just a second call site.

Both playback paths consume the *same* `EarconParams` computed by the *same* pure function — the
parity discipline P64 established for friction (§3.1) extended to every Tier-1/2 event.

---

## 7. Acceptance criteria — falsifiable, "verified not claimed"

- **A1 — Exhaustive parity, proven by the type system, not a checklist someone forgets to update.**
  `earcon_for(SoundEvent) -> EarconParams` is a non-exhaustive-forbidding `match`. Adding a new
  `OrderStatus` variant to `kernel/src/order_machine.rs` or a new DZ-05 event without a
  corresponding `SoundEvent` arm is a **compile error** in `engine/src/sound.rs`. This is the
  concrete mechanism behind "every visual friction signal has a corresponding audio cue, verified
  by an exhaustive checklist against the friction-spec enum" — except realized as a Rust
  exhaustiveness check rather than a manually-maintained checklist, which cannot silently drift.
- **A2 — Blind-path completion, reusing P64's proven harness shape.** A scripted test drives the
  Sea through every `OrderStatus` transition with the visual buffer masked; using only
  `EarconParams` output (+ P58's `role=Status` mirror for the text content), a listener can
  identify which of the 6 DZ-04 semantic classes (§5.1) occurred, for all 12 `OrderStatus`
  variants. Modeled directly on P64's `a11y_blind_path` (`BLUEPRINT-P64-intent-engine-friction-voice.md:398-402`).
- **A3 — Deaf-path unaffected (the never-load-bearing-for-sighted-users half).** With audio fully
  muted, every Tier-1/2 event's information is independently recoverable from the Sheet/mirror text
  alone — a regression test asserting no `SoundEvent` carries information absent from the visual/
  text channel (the DZ-05 GATE discipline — "money snap never tween in feedback" — extended to
  "sound never carries exclusive information for a sighted, unmuted-but-not-listening user").
- **A4 — Mute/quiet-mode round-trip.** A per-hub audio-off toggle exists; toggling it changes zero
  order-lifecycle behavior (sound is observational, never a gate — consistent with §3.2's
  never-load-bearing framing for the *ambient* tier, applied here to confirm Tier 1/2 is not
  secretly load-bearing for sighted users either, only for the blind-path scenario A2 covers).
- **A5 — Zero new default-build dependency.** `cd engine && cargo tree -e no-dev` is byte-identical
  before/after (the `audio` feature stays off by default, matching `gpu`/`telemetry`).
- **A6 — Parity with P64 unbroken.** P64's own `friction_map.rs`/`a11y_friction.rs` test suite
  stays green, unmodified — P98 extends the *pattern*, never edits `friction.rs`'s existing
  `CommitToken`/`FrictionFsm` machinery.
- **Not-done clauses:** a `SoundEvent` added without a `match` arm (compile error, so this is
  structurally impossible, not just a lint); a Tier-1/2 cue implemented as a bundled audio sample
  instead of procedural synthesis (violates §6/§5's DECART); any new dependency entering the
  default engine build graph; Tier 3 (ambient sonification) code landing under this blueprint's
  name — any of these is **NOT done** regardless of other green totals.

---

## 8. Dependencies

**Consumes:**
- **P64** — the `AudioParams`/`Stake`-derivation pattern this blueprint extends; friction's own
  audio channel is reused unmodified, never re-implemented.
- **P58** — the `role=Status`/`role=Alert` a11y mirror, the mechanism that already gives dowiz's
  text content OS-level TTS for free (§4).
- **DZ-04/DZ-05** (`BLUEPRINTS-DOWIZ-INTERFACES.md`) — the visual-signal vocabulary this blueprint's
  entire catalog (§5) is derived from; P98 adds no new visual semantics, only an audio projection
  of the existing ones.
- **`kernel/src/order_machine.rs`** — the `OrderStatus` enum whose exhaustiveness A1 is checked
  against.

**Feeds:**
- **Phase 6 of the spatial-storefront-voice-hub arc** (proactive spoken alerts) — consumes this
  blueprint's Tier-1/2 catalog as the "existing classical warning/alert set" its own O6 ruling
  already named as the thing that becomes optionally spoken.
- **P61** (notification fabric) — no mechanism overlap (§1), but both fire from the same
  `OrderStatus` fold events; a future cross-check that the two stay behaviorally consistent (e.g. a
  courier-nearby push and an in-app `CourierNearby` earcon don't contradict each other) is named
  as a possible future regression, not built here.

**Does not feed / explicitly declines to unblock:** Tier 3 ambient sonification (§3.2) — this
blueprint adds no new evidence toward building it, and its own gating (operator wgpu network grant)
is unrelated to anything P98 builds.

---

## 9. Anti-scope, restated plainly

**Built here (Wave-0):** the `SoundEvent`/`EarconParams` types and exhaustive mapping (§5), the
pure-math derivation reusing P64's proven parity pattern (§3.1), procedural synthesis playback at
the web/native edges (§6), the accessibility-parity acceptance suite (§7).

**Not built here, cross-referenced only:** voice input (P64), friction's own audio channel (P64,
unmodified), conversational TTS (`voice-adapters` Phase 4), proactive spoken alerts
(`voice-adapters` Phase 6 — P98 supplies its input taxonomy, not its speech path), push/SMS/email
notifications (P61), whole-field ambient/generative sonification (Tier 3, `BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20.md`
§6 — named, deliberately not expanded).

**Named gap, not silently resolved:** the exact `EarconParams` tuning for `OrderRefunding`/
`CompensatedRefund` (§5.1) has no prior visual-delta mapping to derive from — DZ-04 itself doesn't
cover these two states. Recorded as an open sub-item for whoever implements §5.1, not invented here
as if it were already grounded.

---

## 10. Links & registration

- Standard precedent: `BLUEPRINT-P64-intent-engine-friction-voice.md` (format, `AudioParams`
  pattern this blueprint extends).
- Canon: `docs/design/ROADMAP.md` §16.50 (`:2867-2882`), §16.44 (friction field-state framing).
- Code cited: `kernel/src/order_machine.rs`, `engine/src/field_frame.rs`, `engine/Cargo.toml`,
  `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P64-intent-engine-friction-voice.md`,
  `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P58-a11y-mirror-everywhere.md`,
  `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md` (DZ-04/DZ-05),
  `docs/design/BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20.md` §5-§6,
  `docs/design/BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md` (D-V1/D-V4, Phase
  4/6, O6), `docs/design/living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`
  (superseded prior pass, cited for provenance only).
- Registered in `docs/design/ROADMAP.md` (Part I, new §16) and `docs/design/CORE-ROADMAP-INDEX.md`
  (new §11).
