# BLUEPRINT P07 — Sonification Phase-0 (execution-ready)

> **Status:** v1 (2026-07-16). Execution-detail blueprint for **Phase 7** of `LIVING-INTERFACE-ROADMAP.md`.
> Design only — writes/edits **no** product code, Cargo file, CSP header, or canon. Its single research
> source is `R-SON-sonification-architecture.md` (already-decided architecture); this document turns those
> recommendations into named modules, exact signatures, an exact wire format, and a precisely-threaded
> causal-ordering design. Roadmap dependencies: **Phase 6** (Sea & Sheet backbone + the one-field `S(t)`
> event stream — the shared ordering authority), **Phase 0** (the CSP `'wasm-unsafe-eval'` fix, without
> which a third wasm artifact cannot instantiate behind the production header), and the **§3 RW-09/RW-01
> amendment** — a *precondition* of this phase, **not itself a roadmap phase**. Everything below is scoped
> to R-SON's Phase-0: **HUB tier only, `postMessage` transport (no COEP), per-event table + pan-following,
> coherence/crossing mixer scaffolded but Tier-2-gated OFF.**

---

## 1. Current-state evidence (what exists, cited precisely)

**1.1 The legacy sound path to delete.** The only sound machinery in the tree is the legacy hook
`apps/web/node_modules/@deliveryos/.ignored_ui/src/hooks/use-sound.ts` (41 LOC) plus its build outputs
`.ignored_ui/dist/hooks/use-sound.js` and `use-sound.d.ts`. Verbatim, it declares
`SoundType = 'notification' | 'success' | 'error' | 'order_update'`, a `DEFAULT_SOUNDS` map to four canned
assets (`/sounds/notification.mp3`, `success.mp3`, `error.mp3`, `order-update.mp3`), and a `play()` that
lazily does `new Audio()`, sets `.src`, and calls `.play().catch(() => {})`. This is exactly what R-SON
§1.2 flags: sample-based, per-component, **no propagation, no pan, no interaction, no pitch** — the audio
twin of the per-component JS mirrors RW-02/RW-03 delete. The audit trail corroborates the pattern is
live-in-legacy: `docs/audit/accessibility-gate.md:157` and `docs/audit/execution-plan.md:75` cite
`DashboardPage.tsx` calling `useSound('/sounds/ping.mp3')` / `lib/hooks.ts:30` `new Audio().play()` on a WS
event (not a user gesture) — the autoplay-on-WS anti-pattern R-SON §4 replaces with a gesture unlock.

**Load-bearing nuance for the acceptance test:** a fresh grep of the *active* `apps/web/src` tree already
returns **zero** `.mp3` / `new Audio(` hits — the legacy hook is quarantined inside the `.ignored_ui`
package and is not imported by the shipping SPA. Therefore Phase-7's deletion is (a) remove the hook +
`.d.ts` + `.js` from `@deliveryos/.ignored_ui`, and (b) remove any `public/sounds/*.mp3` assets that
survive; and the done-test (§7.1) is a **regression lock** whose scope must include the legacy package, not
only `src/`, so a future re-introduction fails CI.

**1.2 RW-09's current two-artifact model.** `docs/design/rust-engine-rewrite/BLUEPRINTS-RUST-ENGINE-REWRITE.md`
RW-01 (§48) fixes the artifact count at line 59 — *"Два wasm artifacts (dowiz_kernel JSON + dowiz_engine
numeric)"* — with acceptance checkbox line 64 *"☐ 2 wasm artifacts."* RW-09 (§212, "Thin-shell boundary")
enumerates ~15 Web-API shims with "Audio" as a single undifferentiated entry. Neither the `field-math`
crate, the RW-05 `shell` crate, nor its `on_event(kind:u32, count:u32)` export (designed at BLUEPRINTS
line 142 / RUST-ENGINE-REWRITE-PLAN line 121) exist in code yet — they are Phase-2 deliverables. **Phase 7
consumes them and requires the §3 amendment first**: RW-01's line must read *three* artifacts
(`+dowiz_audio`) and add `audio` to the scaffold crate list; RW-09's "Audio" must split into (a) host
membrane and (b) the Rust `audio` DSP crate. This blueprint assumes that doc-only amendment has landed; it
does not re-argue it.

**1.3 The kernel authority already in code (the causal spine).** Two kernel modules — real, tested code —
supply everything the J2 fix needs, so Phase 7 invents no new ordering system:
- `kernel/src/order_machine.rs`: `OrderStatus` (10 states, lines 7-19), `assert_transition(from, to) ->
  Result<(), TransitionError>` (line 123, the illegal-transition guard returning `TransitionError::Illegal`),
  and `fold_transitions(start, steps) -> Result<OrderStatus, (TransitionError, OrderStatus)>` (line 140 —
  documented as *"the deterministic reducer the WS event bus replays against"*).
- `kernel/src/event_log.rs`: `MeshEvent { prev:[u8;32], actor_pubkey:[u8;32], actor_seq:u64, payload:Vec<u8> }`
  (line 133), `MeshEvent::event_id() -> [u8;32]` = SHA3-256(prev‖actor_pubkey‖actor_seq‖payload) (line 148,
  the content-id / idempotency key), and `AppendOutcome::{Committed, Duplicate}` (line 222 — a duplicate
  content-id is a *structural* no-op).

**1.4 The money guard to extend.** `engine/src/money_guard.rs` (FE-09 🔴): `Money(pub i64)` deliberately does
**not** implement the `FieldValue` trait, so `interpolate(money, ..)` / `Spring<Money>` is a compile error;
`TweenGuard::present_money(i64) -> Result<i64,String>` rejects fractional presentation; exported from
`engine/src/lib.rs`. The `audio` crate mirrors this construction (§6).

---

## 2. The `audio` crate design

**2.1 Crate identity & discipline.** New crate `audio/`, sibling of `engine/` and the planned `field-math/`
(RW-01 scaffold list). Package `dowiz-audio`; `[lib] name = "dowiz_audio"`, `crate-type = ["cdylib","rlib"]`
(cdylib → the **third** wasm artifact `dowiz_audio.wasm`; rlib → native offline tests). Discipline follows
`field-math`, **not** `engine`: `#![no_std]` + `extern crate alloc;`, **zero external dependencies**,
wasm32-clean. No `std`, no I/O, no float-nondeterminism knobs. Rationale (R-SON §1.1/§1.3): the
`AudioWorkletGlobalScope` is a separate realm with no DOM and no access to the main-thread wasm memory, so
this crate must instantiate standalone; keeping it `no_std`+zero-dep keeps the third artifact tiny (R-SON
§5d "keep the audio-side model small") and keeps **one math authority** alongside `field-math` (Faust /
Tone.js rejected, R-SON §1.2).

**2.2 Module structure** (`audio/src/`):

| module | responsibility |
|---|---|
| `lib.rs` | the wasm ABI surface (<10 zero-JSON exports, RW-05 discipline): `process`, `push_events`, `set_flags`, `resume`/`suspend`, `memory`. |
| `event.rs` | `AudioEvent` / `EventKind`; decode the `postMessage` wire record (§3.3) from the shared `ArrayBuffer`. |
| `sched.rs` | the causal scheduler — jitter/lookahead buffer keyed by `t_logical`, sample-accurate dispatch (§4). |
| `scale.rs` | the scale-quantizer (pentatonic/diatonic key from brand seed). |
| `karplus.rs` | Karplus-Strong plucked-string voice (piano-like struck timbre). |
| `bowed.rs` | additive/FM bowed voice (string-like sustained timbre). |
| `env.rs` | ADSR envelope generator. |
| `filter.rs` | one-pole / biquad low-pass (distance shadowing, damped-error thud). |
| `granular.rs` | grain scheduler (anomaly texture; overflow density render). |
| `voices.rs` | fixed voice pool (budget ≤12) + steal-into-pad overflow (§5). |
| `pad.rs` | ambient pad / tier bed (MESH summation + reduced-audio calm mode). |
| `coherence.rs` | the `|ψ₁±ψ₂|²` coherence mixer — **`#[cfg(feature = "coherence")]`, gated OFF in Phase-0** (§2.5). |
| `money_guard.rs` | money-never-sonified guard (§6). |
| `mixer.rs` | final voice-sum → interleaved output buffer. |

`lib.rs` dep graph: `lib → sched → {event, voices}`; `voices → {karplus, bowed, granular, env, filter,
scale, pad, money_guard}`; `coherence → scale` (gated). No module reaches upward; `money_guard` is imported
by `voices` and `event` so money can never route into a continuous param anywhere.

**2.3 Core DSP signature — Karplus-Strong pluck** (`karplus.rs`). Fast attack + exponential decay =
piano-like struck timbre (R-SON §2.1). Pure integer-indexed delay line, deterministic seed:

```rust
pub struct Pluck {
    line: alloc::vec::Vec<f32>,  // delay line, len = round(sample_rate / freq_hz)
    idx: usize,                  // read/write head
    damp: f32,                   // loop-filter coefficient in [0.5, 0.5+ε]; ↓ = faster decay
    gain: f32,                   // per-voice velocity gain (a FieldValue — never money-derived)
    ringing: bool,
}

impl Pluck {
    /// Strike a string. `seed` fills the delay line with deterministic pseudo-noise
    /// (no `rand` dep — a splitmix step), so offline renders are byte-reproducible.
    pub fn strike(sample_rate: f32, freq_hz: f32, damp: f32, gain: f32, seed: u32) -> Self;
    /// Advance one sample: y = damp * (line[idx] + line[idx+1]); write-back; wrap idx.
    pub fn next_sample(&mut self) -> f32;
    pub fn is_ringing(&self) -> bool;   // false once RMS falls below the silence floor
    pub fn release(&mut self);          // force-damp (voice steal → into pad, §5)
}
```

The bowed/string voice (`bowed.rs`) presents the parallel surface `Bowed::pluck(sample_rate, freq_hz,
attack_s, sustain_gain, partials) -> Bowed` with `next_sample()`, using a short additive stack (3-5
partials) or 2-operator FM — slow attack + sustain (R-SON §2.1). Both voices expose the identical
`next_sample() -> f32` / `is_ringing()` contract so `voices.rs` treats them polymorphically via an internal
`enum Voice { Pluck(Pluck), Bowed(Bowed), Grain(Grain) }` (no `dyn`, no alloc-per-sample).

**2.4 Core DSP signature — scale-quantizer** (`scale.rs`). Pitch is **never free** (R-SON §2.1): every
event's pitch is snapped to a bounded key derived from the brand token seed (the "venue key", the 5th
placement of the one accent, R-SON §2.1). Equal-temperament, integer degrees:

```rust
pub const PENTATONIC_MINOR: [u8; 5] = [0, 3, 5, 7, 10];   // default Cosmo-Noir, non-alarming
pub const DIATONIC_MAJOR:   [u8; 7] = [0, 2, 4, 5, 7, 9, 11];

pub struct Scale {
    root_midi: u8,               // key center, derived from the brand token_hash
    degrees: &'static [u8],      // PENTATONIC_MINOR (Phase-0 default) or DIATONIC_MAJOR
    bright: bool,                // minor→major brightening (Delivered cadence, R-SON §2.2)
}

impl Scale {
    /// Derive root + mode from the SAME brand seed the visual venue tint uses,
    /// so one brand => coherent look AND sound with zero manual tuning.
    pub fn from_brand_seed(token_hash: u32) -> Self;
    /// Scale degree (may be <0 or ≥len; wraps by octave) → frequency in Hz.
    pub fn quantize(&self, degree: i32) -> f32;
    /// Continuous FieldPos.x ∈ [-1,1] → nearest in-key frequency (pitch = position).
    pub fn quantize_pos(&self, field_x: f32) -> f32;
    /// A scale-locked interval above a base degree — the consonance vocabulary.
    pub fn interval(&self, base_degree: i32, iv: IntervalClass) -> f32;
    /// Brightened copy for the resolving cadence (minor→major / pitch up).
    pub fn brightened(&self) -> Scale;
}

pub enum IntervalClass { Unison, Minor2, Third, Fifth }   // Third/Fifth = consonant; Minor2 = ♭2 dissonant
```

`interval(.., Third)` and `interval(.., Fifth)` are what the **delivered** cadence stacks (root+3rd+5th);
`interval(.., Minor2)` + a fast filter damp is the **rejected** thud (♭2). This makes done-test §7.2
*measurable*: the FFT peaks are the exact `quantize`/`interval` outputs, not vibes.

**2.5 Core DSP signature — coherence mixer** (`coherence.rs`, Tier-2 gated). R-SON §2.3 is explicit:
Phase-0 ships the per-event table + pan-following **without** the crossing/coherence layer, which is Tier-2
gated on the **same flag** as the visual `|ψ₁±ψ₂|²` interference (physics-ui-capture-blueprint §1). The
crate **scaffolds** the surface under `#[cfg(feature = "coherence")]` (default off) so wiring it later is a
flag flip, not a re-plumb — reusing the *existing* interference operator, never a new physics:

```rust
#[derive(Clone, Copy)]
pub struct WavePacket { pub carrier_hz: f32, pub phase: f32, pub amp: f32 }

pub enum Coherence { Constructive, Destructive }  // |ψ1+ψ2|²  vs  |ψ1−ψ2|²

/// Reuses physics-ui-capture §1's |ψ1±ψ2|². When two signals' supports cross:
/// Constructive → carriers snap to a consonant interval (3rd/5th) locked to `scale`,
///   combined amp blooms; Destructive → both mute toward a beat/null.
pub fn coherence_mix(a: WavePacket, b: WavePacket, mode: Coherence, scale: &Scale)
    -> (WavePacket, WavePacket);

/// The |ψ1±ψ2|² scalar at the overlap — drives the bloom (constructive) or
/// mute (destructive) gain. Identical operator the visual field already computes.
pub fn interference_gain(a: WavePacket, b: WavePacket, mode: Coherence) -> f32;
```

---

## 3. AudioWorklet + `postMessage` wiring design

**3.1 Two-layer split (RW-09 membrane vs Rust math).** Per the §3 amendment:
- **Host membrane (thin JS, RW-09 boundary module, next to Push/WebGL):** `new AudioContext()`;
  `ctx.audioWorklet.addModule(workletUrl)`; construct the `AudioWorkletNode`; build the graph
  `worklet → PannerNode → GainNode → destination`; the user-gesture `resume()` (DZ-01 arrive-tap unlock);
  `suspend()` on `visibilitychange`/idle (R-SON §5d battery). No math.
- **Worklet processor (the ~30-line trampoline, R-SON §1.1):** a single `AudioWorkletProcessor` whose
  `constructor` instantiates `dowiz_audio.wasm` and allocates the output pointer; whose `process(_in, out)`
  calls `wasm.exports.process(outPtr, 128)` and copies the `Float32Array(wasmMemory.buffer, outPtr, 256)`
  (128 L + 128 R interleaved) into `out[0]`/`out[1]`; whose `onmessage` copies the incoming batch buffer
  into wasm memory and calls `wasm.exports.push_events(ptr, byteLen)`. **Never JSON in the render quantum**
  (RW-05 / RW-09 invariant 2) — the audio-out direction of RW-05's zero-copy numeric boundary.

**3.2 The wasm ABI** (`lib.rs`, <10 zero-JSON exports):

```rust
#[no_mangle] pub extern "C" fn audio_new(sample_rate: f32) -> *mut Engine;
#[no_mangle] pub extern "C" fn events_ptr(e: *mut Engine) -> *mut u8;   // staging buffer JS writes into
#[no_mangle] pub extern "C" fn push_events(e: *mut Engine, byte_len: u32);
#[no_mangle] pub extern "C" fn process(e: *mut Engine, out_ptr: *mut f32, frames: u32);
#[no_mangle] pub extern "C" fn set_flags(e: *mut Engine, bits: u32);    // reduced-audio level, tier
#[no_mangle] pub extern "C" fn out_ptr(e: *mut Engine) -> *mut f32;     // interleaved stereo scratch
// plus the wasm `memory` export.
```

**3.3 The `postMessage` wire format for `S(t)` events.** Transport is `port.postMessage(buf, [buf])` with a
**transferable `ArrayBuffer`** (structured-clone move, no copy, no COEP — R-SON §5a). The buffer is a packed
array of fixed **32-byte little-endian records**, one per already-validated `S(t)` event (the main thread
does the ordering, §4 — the worklet receives a *causally-ordered, deduped* batch, never raw wire order):

```
offset  size  field           meaning
  0      u16  kind            EventKind discriminant (§3.4)
  2      u16  flags           bit0 reduced-audio-hi-salience · bit1 terminal · bit2 coherence-eligible · bits3-4 tier
  4      u32  content_id_lo   low 32 bits of MeshEvent::event_id()  — worklet-side idempotency hint
  8      u32  actor_seq_lo    low 32 bits of MeshEvent.actor_seq    — causal tiebreak
 12      u32  t_logical       DERIVED monotonic schedule tick (§4)  — the primary ordering key
 16      f32  play_at_rel     seconds relative to batch epoch (currentTime + lookahead) to sound at
 20      f32  field_x         FieldPos.x ∈ [-1,1] → pan / pitch-by-position
 24      f32  magnitude       count / intensity → velocity+density (NEVER a money value)
 28      u16  order_status    OrderStatus discriminant → continuous bed selection (DZ-04)
 30      u16  count           coalesced on_event count (N taps folded to one grain-cluster)
```

Design constraints made explicit: **money is absent from the record by construction** — there is no
`amount` field; a price change arrives only as `kind = MoneyTick` with `magnitude` ignored (§6). The record
carries the *compact* `S(t)` (R-SON §5d "not the full field") — no positions array, no field grid. `t_logical`
is the authoritative ordering key; `content_id_lo`/`actor_seq_lo` are 32-bit hints sufficient for
worklet-side dedup because the main thread already guaranteed causal order and full-width uniqueness.

**3.4 `EventKind` (u16), 1:1 with DZ-05 vocab + DZ-04 states.** `Tap=0, AddCart=1, Success=2, Error=3,
Loading=4, OrderPlaced=5, Anomaly=6, MoneyTick=7, StatusBed=8`. The legacy `use-sound.ts` four names map
cleanly (R-SON §1.2): `notification→Tap`, `success→Success`, `error→Error`, `order_update→OrderPlaced`.
`StatusBed` carries the continuous OrderStatus→Море bed (R-SON §2.2 table).

---

## 4. The causal-ordering design (the J2 fix, threaded to kernel types)

**J2 is the roadmap's "most dangerous joint":** both the visual renderer and the audio renderer consume the
*same* ordered stream, and audio makes causal inversion *more* perceptible because sound is sequential
(R-SON §5b). The blueprint's resolution is **structural, not aspirational**: the ordering authority lives on
the **main thread** (where the kernel wasm, `order_machine`, and `event_log` already run), and the worklet
**only schedules within a small lookahead** — it is never handed raw wire order and thus *cannot* invert.

**4.1 Where `t_logical` and `actor_seq` come from — precisely.** There is **no field literally named
`t_logical` in the kernel** (verified: `causal.rs` is Pearl do-calculus, unrelated). `t_logical` is a
*derived* presentation-schedule key, exactly as R-LM's wire format already names it (`t_logical:u64`,
"actor_seq-derived"). Its raw material is real kernel state:
- **`actor_seq`** — `MeshEvent.actor_seq: u64` (`kernel/src/event_log.rs:140`), the per-actor monotonic
  counter, plus the `prev` hash-chain link (`event_log.rs:135`) giving a per-actor happens-before.
- **content-id** — `MeshEvent::event_id()` (`event_log.rs:148`) = SHA3-256(prev‖actor_pubkey‖actor_seq‖
  payload), the idempotency key; a re-received event is `AppendOutcome::Duplicate` (`event_log.rs:222`) — a
  structural no-op, *the* mechanism that makes jittered dupes/reorders harmless.
- **total causal order across actors** — the fold position in
  `order_machine::fold_transitions(start, steps)` (`kernel/src/order_machine.rs:140`). The main-thread
  scheduler defines **`t_logical = index of this event in the fold-validated replay`** (monotone by
  construction; ties broken by `actor_seq` then `content_id`).

**4.2 The main-thread scheduling algorithm** (host membrane, driving the shared kernel wasm):
1. WS frames arrive jittered. Each is turned into a `MeshEvent` (payload = the encoded order transition)
   and offered to the local event-log via the existing `EventLog::commit_after_decide` path.
2. **Dedup:** if `event_id()` already present → `Duplicate` → **drop** (do not emit a second sound). This is
   R-SON §5b "a late attempt after its error is folded, not replayed."
3. **Validate:** the decide-closure runs `assert_transition(prev_status, next_status)`; an
   `Err(TransitionError::Illegal)` means the acausal pair **never reaches the worklet** — the kernel's
   illegal-transition guard is what guarantees "an error sound never precedes its attempt" (§7.4).
4. **Order:** accepted events are folded into the monotonic sequence; each gets `t_logical` = its fold index
   and `play_at_rel = lookahead` where lookahead ∈ **[80ms,150ms]** (R-SON §5b one musical "tick"). The
   buffer holds events for the lookahead window, absorbing the injected 200ms jitter by *reordering to
   causal order before emission*, so out-of-order arrivals within the window sort correctly and a
   predecessor arriving after its successor already sounded is dropped by step-2 dedup.
5. Emit the sorted, deduped batch as the §3.3 transferable buffer.

**4.3 Worklet-side scheduling** (`sched.rs`). The worklet keeps a tiny min-heap (fixed capacity ~32) keyed
by `(t_logical, actor_seq_lo)`. In `process()` it pops every event whose `play_at_rel` has been reached by
the sample-accurate frame counter (derived from `currentTime` at batch epoch + frames elapsed × 1/sr) and
allocates a voice (§5). Because the batch is *already causally ordered*, the heap only smooths sub-tick
timing; it can never surface an error before its attempt. This is the single kernel-validated monotonic
sequence *implemented*, not merely described: the same authority (`fold_transitions` + `event_id` dedup)
feeds **both** renderers; Phase 8's viz plugs into the identical `t_logical`-stamped stream (roadmap J2).

---

## 5. Density / voice-budget design (anti-cacophony + calm mode)

**5.1 Orchestral-distance tiers** (R-SON §3). The invariant: an event keeps the same pitch-class + timbre at
every tier — only its **presence** (audible note vs summed-into-pad) and **spatial treatment** change with
zoom. `flags` bits 3-4 carry the tier; Phase-0 ships **HUB only** (`voices.rs` renders individual notes;
`pad.rs` idles as the low ambient bed). MESH (pad-only) and NODE (single sustained readout) are scaffolded
enums but unreachable in Phase-0 (R-SON §6 "hub only").

**5.2 Hard voice budget** (`voices.rs`): `pub const MAX_VOICES: usize = 12;` a fixed
`[Voice; MAX_VOICES]` pool. `allocate(kind, freq, gain) -> VoiceId` steals the **oldest-and-quietest** voice
when full and routes the stolen (and any budget-overflow) energy into `pad.rs` via
`Pad::absorb(freq, residual_amp)` — **overflow is summed into the ambient pad, never dropped-silent**
(R-SON §5c; information preserved as pad density/brightness). Per-class rate-limit: N `Tap` events within a
~50ms window coalesce (the wire `count` field) into **one** grain-cluster whose density encodes N (mirrors
DZ-05 reduced-motion "burst × 0.25").

**5.3 Calm mode = the same summation** (R-SON §4). `set_flags` carries a 2-bit reduced-audio level: **Off**
(silent, autoplay default before gesture), **Calm** (keep the pad, sonify only high-salience events —
`flags` bit0: Delivered, Error — fold the rest into the pad), **Full**. Calm-mode reuses §5.2's pad
summation verbatim, so reduced-audio accessibility and scale-throttling are **one mechanism**, not two.

---

## 6. Money-never-sonified guard extension (🔴 red-line)

The FE-09 guard extends into the `audio` crate by the **same compile-time-plus-runtime construction** as
`engine/src/money_guard.rs`, so a money value can never become a continuous pitch/gain/cutoff parameter
(R-SON §1.3 / §5d):

```rust
// audio/src/money_guard.rs
/// Marker for values a voice MAY vary continuously (pitch, gain, cutoff, pan).
/// Deliberately NOT implemented for any money-carrying type — the audio twin of
/// engine::money_guard::FieldValue (which is not implemented for Money).
pub trait AudioParam: Copy {}
impl AudioParam for f32 {}   // Hz, linear gain, normalized cutoff, pan — all f32

/// The ONLY legal money→audio mapping: one discrete neutral tick.
pub struct MoneyTick;
impl MoneyTick {
    /// Fixed neutral scale degree, fixed gain, NO glide, NO sonified digits.
    pub fn tick(scale: &Scale) -> Pluck;    // e.g. scale.quantize(0) at a constant velocity
}

pub struct AudioMoneyGuard;
impl AudioMoneyGuard {
    /// Runtime mirror: reject any attempt to route a money-derived scalar into a
    /// continuous param (the RED signature of "pitch = money value").
    pub fn assert_discrete(param_is_money_derived: bool) -> Result<(), &'static str>;
}
```

The type system makes `bind_pitch(money)` / `bind_gain(money)` unrepresentable (money is not `AudioParam`);
`MoneyTick::tick` is the sole path from a price change to sound, and it is one fixed neutral tick — exactly
as DZ-02 forbids the visual money-tween. The guard survives the eventual wgpu/COEP migrations unchanged
(roadmap density-friction row). A RED unit test asserts a fractional/continuous money route is rejected; a
GREEN test asserts a price change produces exactly one `Tap`-class neutral tick with zero pitch glide.

---

## 7. Acceptance criteria (numbered checklist, matching the 5 falsifiable done-tests)

**7.1 — Zero pre-recorded audio files loaded.**
- ☐ `use-sound.ts`, `use-sound.js`, `use-sound.d.ts` removed from `@deliveryos/.ignored_ui`; any
  `public/sounds/*.mp3` assets removed.
- ☐ Repo-wide grep (scope **includes** the legacy package, not only `src/`): `grep -rn '\.mp3\|new Audio('`
  over the app trees returns **0** hits. CI regression-locks this so a re-introduction fails.
- ☐ No `<audio>` element and no `Audio` constructor remain; the only audio path is the `AudioWorklet` +
  `dowiz_audio.wasm`.

**7.2 — Delivered = consonant interval, Rejected = muted dissonance (measured, not vibes).**
- ☐ A **native Rust** offline render (`rlib` test, no browser): drive `process()` with a scripted
  `EventKind::Success`/Delivered `StatusBed`, capture the mono output, run a hand-rolled radix-2 FFT (in
  test code, no dep), assert spectral peaks at `scale.quantize(root)`, `interval(root, Third)`,
  `interval(root, Fifth)` (a major/added-6th-class consonance).
- ☐ Same harness for `EventKind::Error`/Rejected: assert a `interval(root, Minor2)` (♭2) peak with a
  fast-damped envelope (energy decays below the silence floor within the specified short window) — a muted
  dissonant cadence, not a harsh alarm.
- ☐ Determinism: identical input ⇒ byte-identical output buffer (seeded `Pluck`), paralleling the kernel's
  `*_byte_identical_two_runs` gates.

**7.3 — Courier note stereo pan tracks the CourierTrack marker FieldPos.x.**
- ☐ Spatialization authority is **one** `PannerNode` per active courier track, `positionX` bound to the
  marker's live `FieldPos.x` (the geo/`use-courier-marker` path); the worklet outputs mono per-voice (no
  second pan authority).
- ☐ An `OfflineAudioContext` render (node/Playwright) of a sweep of `field_x` values asserts per-channel RMS
  balance is **monotone** in `field_x` (x=-1 → left-dominant, x=+1 → right-dominant), i.e. the pan follows
  the marker.

**7.4 — Under injected 200ms WS jitter, events schedule in kernel-validated causal order.**
- ☐ Test harness injects 200ms of reordering jitter into the WS event feed; the main-thread scheduler folds
  through `order_machine::fold_transitions` + dedups by `event_id()` (`AppendOutcome::Duplicate`).
- ☐ Assert the **error sound never precedes the sound of its attempt**: for every (attempt, error) pair, the
  emitted `t_logical(error) > t_logical(attempt)`, and an `assert_transition` `Illegal` result emits **no**
  second sound.
- ☐ Assert a late-arriving predecessor (arriving after its successor already sounded) is **folded, not
  replayed** (dropped as a content-id `Duplicate`), not re-triggered.
- ☐ Confirm the ordering authority is the **single** `t_logical`-keyed monotonic sequence (the same stream
  Phase 8's viz will consume) — the worklet heap only smooths sub-tick timing within the 80-150ms lookahead;
  it never receives raw wire order.

**7.5 — Audio is enhancement, never load-bearing.**
- ☐ With `AudioContext` never resumed (autoplay blocked pending the DZ-01 arrive-tap gesture), an automated
  pass (reliability-gate L0-L11 visual trace) reads **every** order state from the visual/text channel
  (status pills, `<Money>`, OrderProgress stepper, DZ-05 ripple).
- ☐ No test asserts any state via sound alone (design-illegal); audio adds zero required assertions.
- ☐ `AudioContext` suspends on `visibilitychange`/idle and resumes on gesture (battery parity, R-SON §5d).

**7.6 — Preconditions (gate before starting).**
- ☐ §3 amendment landed: RW-01 reads *three* artifacts (`+dowiz_audio`), `audio` in the scaffold list;
  RW-09 "Audio" split into host membrane + DSP crate.
- ☐ Phase 0 CSP `'wasm-unsafe-eval'` shipped (else `dowiz_audio.wasm` throws a CSP `CompileError` behind the
  production header).
- ☐ Phase 6 shipped: the one-field `S(t)` stream + DZ-04/05/06 (`fold_transitions` replay) exist as the
  shared ordering authority — Phase 7 consumes it, never invents its own clock.

---

*End BLUEPRINT P07. Execution detail only — no product code, Cargo file, CSP header, or canon edited.
Scoped strictly to R-SON Phase-0 (HUB tier, `postMessage`, coherence mixer scaffolded-but-gated-off). The
three preconditions in §7.6 gate the start; the five checklists in §7.1-7.5 are the falsifiable done-tests
from the roadmap Phase-7 row, expanded and bound to exact kernel types (`event_log.rs::MeshEvent`/
`event_id`/`AppendOutcome`, `order_machine.rs::assert_transition`/`fold_transitions`) and crate signatures.*
