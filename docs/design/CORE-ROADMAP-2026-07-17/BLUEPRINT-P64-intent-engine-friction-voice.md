# BLUEPRINT P64 — Intent engine, friction spec & voice (2026-07-18)

> **Wave W1 foundation blueprint.** Written against the 20-point quality contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2, format precedent
> `BLUEPRINT-P51-open-map-routing.md`, dependency assignment
> `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (W1 row **P64**) + §2 (X12) + §4-E
> (engineering-unknown E: friction numeric mapping). This blueprint owns three genuinely hard,
> coupled problems and one shared substrate: **(1)** the intent-driven navigation runtime v1,
> **(2)** the §16.44 friction→field-state numeric mapping with an objective accessibility gate,
> **(3)** local voice behind `VoiceSource: InputSource`, and **(X12)** the shared `LocalInference`
> model-runtime port. It is the concrete implementation of P38-rev §12.2 constraint 3 ("all input
> routes through the intent-abstracted `InputSource`"). Deeper generative-UI research (Track-R,
> §16.35/§16.39) continues **beyond** this blueprint's scope; everything specified here is
> Wave-0-v1 — buildable and testable, not aspirational prose (anti-scope is explicit in §8).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

| Claim about existing code / canon | Cite (verified this pass) | Verdict |
|---|---|---|
| `Intent` / `FieldPos` / `InputSource` are **declared but not built** — a types-only slice, 0 grep hits in code | repo-wide `grep -rn "enum Intent\|struct FieldPos\|trait InputSource" --include=*.rs` → **0 hits**; declared in `BLUEPRINT-P38-webgpu-render-engine.md:606-614` (§11.2) | VERIFIED — P38b DoD-1 baseline is RED; P64 is where the runtime lands |
| P38 §11.2 declares the intent grammar: `enum Intent { Point(FieldPos), Impulse(FieldPos,f32), Select(WidgetId), Navigate(NavTarget), Scrub(f32), Command(CommandId) }` + `trait InputSource { fn poll(&mut self)->Option<Intent> }` | `BLUEPRINT-P38-webgpu-render-engine.md:608-611` | VERIFIED — P64 **extends** this, does not redefine it (single-owner: intent grammar → P64 per SYNTHESIS §5) |
| P38 §12.2 constraint 3 makes `InputSource` a **hard requirement**: "PointerSource ships in P38b; **VoiceSource/gesture are P64**; no event-type-specific handler outside `InputSource` adapters" | `BLUEPRINT-P38-webgpu-render-engine.md:718-725` | VERIFIED — P64 delivers the VoiceSource + gesture-source half of that contract |
| `LlmBackend` port exists in the **kernel** (not the adapter crate) with `id/caps/chat/embed/rerank/health`; ZERO net/HTTP/JSON | `kernel/src/ports/llm.rs:368-382` (trait), `:1-11` (compile-firewall doc) | VERIFIED — the chat modality of X12's runtime **reuses** this port; no duplication |
| `AiMode { Off, LocalOffline, Connected }` + `BackendConfig::from_env` are **already landed in code** (kernel-side mirror), the single non-test constructor of `Connected` | `kernel/src/ports/llm.rs:194-202` (enum), `:275-328` (`from_env`/`from_env_get`), `:204-208` (Default=Off) | VERIFIED — differs from P41's blueprint prose (which placed it in `llm-adapters`); code note at `:187-192` says kernel-side first, `llm-adapters` imports it. P64 integrates with the **landed** location |
| `Caps { chat, embed, rerank, tool_calling }`, `TaskClass { Code, General, Embedding }`, `LlmError { Unavailable, Unsupported, BadRequest, Timeout }` | `kernel/src/ports/llm.rs:17-23, 30-36, 169-179` | VERIFIED — the ASR/translate modalities add sibling `Caps` bits + error variants, same fail-closed shape |
| Scene graph vocabulary: `enum SdfShape { Circle, Box, RoundedBox, LineSegment }`, `struct Scene { shapes: Vec<SdfShape> }`, `Scene::add/render_frame/render_to_bridge(->VertexBridge)/sample` | `engine/src/scene.rs:29,71,88,122,168,103` | VERIFIED — "composing UI functions" = building/merging `SdfShape`s into a `Scene` (§3.2) |
| Field render oracle: `compose(scene,eq,w,h,steps)->Vec<u8>`, `FieldFrame`, `FieldEquilibrium`, `LaplacianField`/`laplacian` | `engine/src/field_frame.rs:255,159,40,106,129` | VERIFIED — the friction field-state variables ride `FieldEquilibrium` + the Laplacian substrate (§4) |
| Money integrity: `struct Money(pub i64)` (integer minor units) does **NOT** impl `FieldValue`; `interpolate<T:FieldValue>` excludes it at compile time; `TweenGuard::present_money`/`jump` | `engine/src/money_guard.rs:18,21,45,60,72` + compile-proof test `:117` | VERIFIED — load-bearing for §4: money never tweens, but the **field around** a money action does; friction reads `money.0` as a scalar parameter, never animates the amount |
| `WidgetStore` / `ParticlePool` exported from engine | `engine/src/lib.rs:41` | VERIFIED — hit-testing for pointer-intent classification (§3.1) uses `WidgetStore` |
| No voice / ASR / wake-word / whisper / Moonshine code exists yet | `grep -rniE "voicesource\|moonshine\|whisper\|wake.word" --include=*.rs .` → **0 hits** | VERIFIED — greenfield behind the port; P64 M5 |
| No local translation code exists yet | `grep -rniE "translat\|localiz" ./kernel ./llm-adapters ./engine` → only unrelated math comments | VERIFIED — §16.38 translate modality is greenfield behind X12's runtime |
| R1 voice research: Moonshine ~107 ms streaming (English-strong); whisper.cpp multilingual, batch (~11 s large-v3) fallback; run native not WASM for NEON/battery; wake-gate is the top battery lever | `OPUS-R1-INTERFACE-RENDERING-2026-07-18.md:240-278` (§5) | VERIFIED — M5 is a direct implementation of R1 §5's Wave-0 recommendation |
| §16.44 friction-as-field-state is a **closed decision**, mapping is Tier-3 blueprint work | `MASTER-ROADMAP-…-2026-07-16.md:2273-2289` (§16.44); SYNTHESIS §4-E names P64 as owner | VERIFIED — §4 of this blueprint is that owned mapping |
| §16.50: friction multi-modal incl. **full audio**; voice one of several **equal** intent channels; onboarding implicit in the field | `MASTER-ROADMAP-…-2026-07-16.md:2347-2362` (§16.50) | VERIFIED — §4 audio channel + §3.7 onboarding + equal-channel routing (§3.1) |
| §16.53: courier-in-motion **voice-primary** — a narrow safety exception to equal channels | `MASTER-ROADMAP-…-2026-07-16.md:2396-2401` (§16.53) | VERIFIED — the `InputProfile::CourierInMotion` bias (§3.1) |
| §16.34: voice runs **locally**, and is "a different category from the AI-decision-authority boundary §16.4 draws" | `MASTER-ROADMAP-…-2026-07-16.md:2162-2164` | VERIFIED — **load-bearing** for X12: ASR/MT run regardless of `AiMode`; only the assistant chat model is `AiMode`-gated (§3.6) |

**One ground-truth correction recorded (not reopened):** P41's blueprint prose places `AiMode`/
`BackendConfig` in the `llm-adapters` crate; the **landed code** puts the mirror in
`kernel/src/ports/llm.rs:194-328` with an explicit note (`:187-192`) that `llm-adapters` re-imports
it. P64 binds to the **landed** location. This is the canonical `AiMode` for the X12 integration.

---

## 1. Research direction vs Wave-0-v1 scope (the honesty line, restated per SYNTHESIS §4-E)

The operator's §16.35/§16.40 framing is a **research program**, and §16.44/§16.50/§16.51 name the
*mechanism* as closed but the *mapping* as open. This blueprint draws the line the synthesis draws:

- **Wave-0-v1 (this blueprint builds it, testably):** intent as a data type + a **deterministic**
  classifier over pointer/gesture/voice; UI composition as **selection of pre-built fragment
  functions** into the existing `Scene`/`compose()` pipeline (operator's own first clause:
  *"рендерить і показує заготовлені речі через функції"*); the friction numeric mapping with its
  gesture grammar, audio channel, and objective a11y gate; voice (wake + Moonshine + whisper
  fallback); the shared `LocalInference` runtime; implicit onboarding via idle-field hints.
- **Track-R (named, NOT built here — §8):** generating UI *"з нуля"* (from scratch, procedurally)
  via the local model (§16.35 second clause); physics/wave-generated glyphs (§16.39); the
  AI-*ranked* intent disambiguation beyond the deterministic classifier (§3.1's optional layer is
  spec'd but its Wave-0 use is bounded to non-consequential navigation only); deepening the
  friction perceptual model beyond the log-amplitude scheme.

The v1 classifier is deterministic **on purpose**: a money/destructive action's intent must never
be inferred by a probabilistic model (§16.4 — the assistant is not a decision-maker; P6
Cause-and-Effect). The AI layer only *reorders exploratory navigation candidates* and is
structurally forbidden from constructing a `CommitToken` (§4.4).

---

## 2. Predefined types & constants (standard §2 item 4 — named BEFORE implementation)

New crate module layout (native-side engine + a native inference sidecar; **zero** new
UI-library dependency, *ad fontes* §16.42-43):

```
engine/src/intent.rs        — Intent runtime (extends P38 §11.2 types); classifier; router
engine/src/compose_ui.rs    — FragmentRegistry + Composer (intent → Scene directive)
engine/src/friction.rs      — the §16.44 mapping, FrictionFsm, CommitToken (§4)
engine/src/onboarding.rs     — HintPolicy (§3.7)
inference/src/lib.rs        — LocalInference: ModelRuntime + AsrModel + TranslateModel (X12)
inference/src/voice.rs      — VoiceSource: InputSource; WakeWordSpotter; Moonshine/Whisper ASR
```

`inference/` is a **native-only** crate (Tauri side); it never compiles to WASM (R1 §5 — NEON/
battery). The wgpu UI receives `Intent`s across the `InputSource` seam and **never** touches audio
buffers or model handles directly (bulkhead — item 11).

### 2.1 Intent runtime types — extend P38 §11.2, do not redefine

```rust
// engine/src/intent.rs — P38 §11.2 declares Intent/FieldPos/InputSource (0 grep hits today);
// P64 lands the runtime. FieldPos/Intent/InputSource are IMPORTED from the P38b types-only slice.

/// Where an input came from and the active bias. Equal-channel by default (§16.50);
/// CourierInMotion biases toward voice (§16.53) WITHOUT disabling other channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputProfile { #[default] Balanced, CourierInMotion, HandsFree }

/// The classifier's verdict. Resolved is the ONLY variant that a consequential action
/// may consume (see Classification::require_resolved, §3.1). Ambiguous is the ONLY
/// place the optional AI ranker may run — and never for a consequential candidate.
#[derive(Debug, Clone, PartialEq)]
pub enum Classification {
    Resolved(Intent),
    Ambiguous(Vec<Intent>), // exploratory navigation only; ranked by the optional AI layer
    Rejected(RejectReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason { NoTarget, BelowThreshold, UnknownCommand, OutOfContext }

/// Raw, pre-classification input. One variant per InputSource kind. The classifier is the
/// ONLY consumer; nothing downstream ever sees a concrete event type (P38-rev §12.2 c3).
#[derive(Debug, Clone, PartialEq)]
pub enum RawInput {
    Pointer { pos: FieldPos, phase: PointerPhase, vel: (f32, f32) },
    Key(KeyCode),
    VoicePhrase { transcript: String, confidence: f32, is_final: bool },
    Gesture { kind: GestureKind, origin: FieldPos, vector: (f32, f32), held_ms: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerPhase { Down, Move, Up }

/// The deterministic classifier. Pure fn of (input, context) — reproducible (P6). No I/O.
pub struct IntentClassifier { /* fragment/command lexicon handle, hit-tester over WidgetStore */ }
impl IntentClassifier {
    pub fn classify(&self, input: &RawInput, ctx: &IntentContext) -> Classification;
}

/// Context the classifier hit-tests / disambiguates against. Borrowed, never owned.
pub struct IntentContext<'a> {
    pub widgets: &'a WidgetStore,      // engine/src/lib.rs:41 — pointer hit-test target
    pub surface: SurfaceId,            // which surface's command lexicon is live
    pub profile: InputProfile,
}

/// ONE code path (P38 §11.2 invariant): sources → classify → apply.
pub struct InputRouter { sources: Vec<Box<dyn InputSource>>, classifier: IntentClassifier }
impl InputRouter {
    /// Poll every source, classify, emit resolved intents. Ambiguous/Rejected are logged
    /// (telemetry item 10) and dropped from the apply stream unless the AI ranker resolves them.
    pub fn tick(&mut self, ctx: &IntentContext) -> Vec<Intent>;
}
```

### 2.2 UI composition types — "composing UI functions" made concrete

```rust
// engine/src/compose_ui.rs
/// A pre-built UI fragment: a pure function producing SDF geometry from app state.
/// This IS the operator's "заготовлені речі через функції" (§16.35 clause 1).
pub type FragmentFn = fn(&AppState) -> Vec<SdfShape>;   // SdfShape: engine/src/scene.rs:29

/// The registry the Composer selects from. v1 = a fixed dispatch table (deterministic).
/// Track-R may add a generative producer behind the SAME ComposedResponse contract (§8).
pub struct FragmentRegistry { table: BTreeMap<FragmentId, FragmentFn> }

/// The Composer's output: a Scene delta + a field-parameter delta. The renderer consumes
/// exactly this; there is no other way for an intent to reach the screen.
#[derive(Debug, Clone)]
pub struct ComposedResponse {
    pub scene: Scene,               // engine/src/scene.rs:71 — merged fragment geometry
    pub field: FieldParams,         // amplitude/intensity/color/rhythm deltas → FieldEquilibrium
    pub friction: Option<FrictionSpec>, // Some(..) iff this response gates a consequential action
    pub mirror: MirrorPatch,        // P58 a11y-mirror reconciliation (role/name/state/live)
}

pub struct Composer { registry: FragmentRegistry }
impl Composer {
    /// intent → ComposedResponse. Pure fn of (intent, state) for non-consequential intents;
    /// a consequential intent yields a response carrying friction=Some(..) (§4), NOT an
    /// immediate commit (the commit needs a CommitToken from the FrictionFsm).
    pub fn compose(&self, intent: &Intent, state: &AppState) -> ComposedResponse;
}
```

### 2.3 Friction field-state mapping types (§16.44 — the owned piece; full scheme in §4)

```rust
// engine/src/friction.rs
/// The two independent stake axes. money is read from Money (money_guard.rs:18) as a
/// SCALAR PARAMETER — it is never animated (Money is not a FieldValue, compile-proof at
/// money_guard.rs:117). reversibility is a closed enum, not a probability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stake { pub money_minor: i64, pub reversibility: Reversibility }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reversibility { Reversible, ReversibleWithCost, Irreversible }

/// The field-state a stake maps to. Every field is a FieldValue (tween-safe); NONE carries
/// the money amount as a value — they carry a DERIVED intensity. Parity-pinned to AudioParams
/// (§4.3) so visual and audio cannot drift (P2 Correspondence).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrictionField {
    pub amplitude: f32,   // = friction_amplitude(stake)  (§4.1)
    pub intensity: f32,   // environmental / particle density, secondary amplifier
    pub hue_shift: f32,   // OKLCH grade delta — REDUNDANT cue, never sole (§16.50)
    pub hold_ms: u32,     // required sustained-completion duration = friction_hold(stake) (§4.2)
}

/// Non-visual equivalent, derived from the SAME stake by the SAME functions (§4.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioParams { pub pitch_hz: f32, pub tremolo_hz: f32, pub hold_ms: u32 }

/// The completion FSM. The ONLY producer of a CommitToken. Fail-safe default: release
/// before threshold = Cancelled (the safe pole — P4 Polarity, safe-directed).
#[derive(Debug, Clone, PartialEq)]
pub enum FrictionState { Idle, Building { progress_ms: u32 }, Committed, Cancelled }

pub struct FrictionFsm { spec: FrictionSpec, state: FrictionState }
impl FrictionFsm {
    /// Feed one directed-hold sample (from Intent::Impulse held toward the commit well).
    /// Advances progress by dt only while the sustained gesture is active AND aimed.
    pub fn advance(&mut self, aimed: bool, dt_ms: u32) -> FrictionState;
    /// Some(CommitToken) IFF state == Committed. None otherwise. The ONLY way to mint it.
    pub fn commit_token(&self) -> Option<CommitToken>;
}

/// Unforgeable proof that the friction threshold was met for THIS stake. Non-Clone,
/// non-Default, private field — constructible only inside friction.rs by the FSM (§4.4).
/// P60's payment call site REQUIRES one; no code path moves money without it (item 6).
pub struct CommitToken { stake: Stake, _seal: Seal }  // Seal is a private ZST
```

**Constants (named, not magic — item 4). Values are the v1 defaults; §7 benchmarks tune them:**

```rust
pub const FRICTION_UNIT_MINOR: i64 = 100;          // 1 currency unit = 100 minor
pub const FRICTION_AMP_BASE: f32 = 0.08;           // idle friction amplitude
pub const FRICTION_AMP_LOG_GAIN: f32 = 0.11;       // amplitude per decade of money
pub const FRICTION_AMP_MAX: f32 = 0.85;            // clamp — never a fully opaque/unusable field
pub const HOLD_BASE_MS: u32 = 350;                 // minimum deliberate hold
pub const HOLD_LOG_GAIN_MS: u32 = 180;             // extra hold per decade of money
pub const HOLD_IRREVERSIBLE_MS: u32 = 500;         // added when Reversibility::Irreversible
pub const HOLD_REVERSIBLE_COST_MS: u32 = 200;      // added when ReversibleWithCost
pub const AIM_TOLERANCE: f32 = 0.35;               // cos-angle: gesture must aim at commit well
pub const AUDIO_PITCH_BASE_HZ: f32 = 220.0;        // A3 at zero stake
pub const AUDIO_PITCH_DECADE_RATIO: f32 = 1.5;     // pitch × per decade of money
pub const HINT_MASTERY_THRESHOLD: u8 = 3;          // successes before a hint retires (§3.7)
```

### 2.4 Voice + LocalInference (X12) types

```rust
// inference/src/voice.rs — native only
/// The InputSource impl P38-rev §12.2 c3 names ("VoiceSource are P64"). Emits Intent via the
/// classifier's VoicePhrase path. Behind it: wake-gate → streaming ASR → transcript.
pub struct VoiceSource {
    wake: WakeWordSpotter,          // tiny always-on keyword-spot — the battery lever (R1 §5)
    asr: Box<dyn AsrModel>,         // Moonshine (default) or Whisper (multilingual fallback)
    ring: AudioRing,                // fixed-size mic ring buffer, no allocation on hot path
}
impl InputSource for VoiceSource {
    fn poll(&mut self) -> Option<Intent>; // wake? → stream → classify → Intent::Command/Navigate
}

// inference/src/lib.rs — X12 shared substrate
/// Fail-closed capability discovery, mirrors Caps (kernel/src/ports/llm.rs:17). A modality the
/// runtime did not load is false; the caller must not assume presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InferenceCaps { pub chat: bool, pub asr: bool, pub translate: bool }

/// Audio → text. Sibling of LlmBackend for a different modality. Same typed-error shape.
pub trait AsrModel {
    fn id(&self) -> &str;
    fn caps(&self) -> AsrCaps;                       // streaming?, languages
    /// Feed a PCM chunk; returns partial/final transcript deltas. Streaming (Moonshine) or
    /// buffered (Whisper — accumulates to a 30 s window, R1 §5).
    fn feed(&mut self, pcm: &[i16]) -> Result<Vec<AsrDelta>, InferError>;
    fn reset(&mut self);
}

/// Text → text localization (§16.38). MAY be a dedicated small MT model OR expressed via a
/// chat model with a translate prompt — the runtime decides; consumers see only this trait.
pub trait TranslateModel {
    fn id(&self) -> &str;
    fn translate(&self, text: &str, from: Lang, to: Lang) -> Result<String, InferError>;
}

/// THE shared runtime (X12). ONE substrate: model load/unload/mmap, a single memory budget,
/// BYO-model registration (§16.52). The three modalities are OBTAINED from it — they are not
/// three runtimes (P2 Correspondence). Native only.
pub trait ModelRuntime {
    fn caps(&self) -> InferenceCaps;
    /// Chat modality REUSES the existing kernel port verbatim — no duplication. None when the
    /// assistant is AiMode::Off (§3.6). ASR/translate are independent of AiMode (§16.34).
    fn chat(&self) -> Option<&dyn LlmBackend>;       // kernel/src/ports/llm.rs:368
    fn asr(&self) -> Option<&dyn AsrModel>;
    fn translate(&self) -> Option<&dyn TranslateModel>;
    /// BYO-model: register an operator-supplied model file/endpoint for any modality (§16.52).
    fn register_byo(&mut self, m: ByoModel) -> Result<(), InferError>;
    /// Loaded-model memory in bytes — the single budget the bulkhead enforces (item 11).
    fn budget_used(&self) -> usize;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferError { NotLoaded, Unsupported, BadInput(String), Timeout, BudgetExceeded }
```

**Rejected alternatives (DECART, one line each):** (a) *three separate runtimes for ASR / MT /
chat* — rejected as X12's explicit anti-pattern (triple model-lifecycle code, triple memory
budgets, no shared BYO path); one `ModelRuntime` with modality accessors is the reuse-first shape
(item 19). (b) *duplicate `LlmBackend` into the inference crate* — rejected; the chat modality
imports the kernel trait (`kernel/src/ports/llm.rs:368`) verbatim (P2). (c) *WASM ASR* — rejected
per R1 §5 (no NEON, battery cost). (d) *AI classifier for consequential intents* — rejected per
§16.4/P6 (the assistant is not a decision-maker); the v1 classifier is deterministic. (e) *money
amount as a `FieldValue` so the field can animate it directly* — rejected; `money_guard.rs:117`
makes it a compile error, and §4 reads `money.0` as a scalar parameter instead.

---

## 3. Build items — spec → RED test → code, adversarial cases (standard §2 items 3, 5)

Each item: the spec (types above) precedes a RED test that fails today, then the code. Tests
assert on **event/intent sequences** (item 3), not just end state.

### 3.1 M1 — Intent runtime v1 (deterministic classifier + router)

**Spec.** `IntentClassifier::classify` is a pure fn (§2.1). Mapping rules (v1, deterministic):
- `Pointer{Down/Up}` over a widget → hit-test `WidgetStore` → `Resolved(Intent::Select(id))`;
  over empty field → `Resolved(Intent::Point(pos))`.
- `Pointer{Move}` / `Gesture` with velocity above `AIM_TOLERANCE` → `Resolved(Intent::Impulse(pos, mag))`
  or `Intent::Scrub(dx)` per axis lock.
- `VoicePhrase{is_final,transcript}` → match against the surface's fixed command lexicon
  (Aho-Corasick over `CommandId` keywords, deterministic) → `Resolved(Intent::Command(id))` or
  `Resolved(Intent::Navigate(target))`; no lexicon match → `Rejected(UnknownCommand)`.
- Genuinely ambiguous **exploratory-navigation** phrase (≥2 lexicon candidates, none
  consequential) → `Ambiguous(candidates)`; the optional AI ranker (§3.6, AiMode≠Off) may reorder.
  A consequential candidate in the set forces `Rejected(OutOfContext)` — **the AI never touches a
  money/destructive intent** (P6).
- `InputProfile::CourierInMotion` biases voice lexicon sensitivity up and pointer precision
  requirements down (§16.53) but disables **no** channel (§16.50).

**RED test** (`engine/tests/intent_v1.rs`): `intent_types_exist_and_exercised` — the P38b DoD-1
test, currently RED (0 grep hits). Round-trip: `RawInput::Pointer{Down over widget-7}` →
`classify` → `Resolved(Intent::Select(WidgetId(7)))` → `InputRouter::tick` emits it → field
responds. Plus a voice round-trip: `VoicePhrase{"open menu", final}` → `Intent::Navigate(Menu)`.

**Adversarial (item 5):** `ambiguous_never_auto_commits` — a `VoicePhrase` matching both "confirm"
(consequential) and a nav command must `Rejected(OutOfContext)`, never `Ambiguous`, never auto-pick
the consequential branch. `classifier_is_pure` — same `(input,ctx)` twice ⇒ byte-identical
`Classification` (P6 reproducibility). `no_raw_event_leak` — grep gate: no `mousedown`/`touchstart`/
`keydown` handler outside `inference/` + `engine/src/intent.rs` `InputSource` impls (P38-rev §12.2
c3 falsifier, imported).

### 3.2 M2 — UI composition (intent → scene-graph directive)

**Spec.** `Composer::compose(intent, state)` (§2.2). "Composing UI functions" = look up the
`FragmentId`s the intent maps to, call each `FragmentFn(state) -> Vec<SdfShape>`, merge into one
`Scene` (`Scene::add`, `engine/src/scene.rs:88`), attach `FieldParams`, and — if the intent is
consequential — attach `friction: Some(FrictionSpec)` (§4). The renderer calls
`compose(scene, eq, w, h, steps)` (`engine/src/field_frame.rs:255`) exactly as today; M2 adds no
new render path (reuse-first, item 19).

**RED test** (`engine/tests/compose_ui.rs`): `intent_composes_registered_fragment` — `Intent::Navigate(Menu)`
→ `ComposedResponse.scene` contains the menu fragment's `SdfShape`s (assert shape count + a known
circle centre); `ComposedResponse.mirror` names the menu region (P58 hook). RED today (no
`compose_ui.rs`).

**Adversarial:** `unknown_fragment_is_empty_not_panic` — an intent with no registered fragment
yields an empty `Scene` + a `Rejected`-logged telemetry row, never a panic (P4 safe-directed
collapse). `consequential_intent_never_bare_commits` — `Intent::Command(ConfirmOrder)` →
`ComposedResponse.friction.is_some()`; a `ComposedResponse` for a money action with `friction:None`
is a test failure (the type + this test are the smart index, item 14).

### 3.3 M3 — Friction numeric mapping + gesture grammar (owns engineering-unknown E)

Full scheme in §4. Build items: `friction_amplitude(stake)`, `friction_hold(stake)`,
`audio_params(stake)`, `FrictionFsm::{advance, commit_token}`, `CommitToken`.

**RED test** (`engine/tests/friction_map.rs`): `amplitude_is_log_monotone` — `friction_amplitude`
strictly increases with `money_minor` and is sub-linear (a 100× money increase yields <100×
amplitude; assert against the log formula) and clamps at `FRICTION_AMP_MAX`. `hold_scales_with_stake`
— `friction_hold(Irreversible, big)` > `friction_hold(Reversible, small)` by the exact
constant deltas. RED today (no `friction.rs`).

**Adversarial (the safety core):** `accidental_input_never_commits` — a fuzz of 10⁴ random
short/single input sequences (taps, sub-threshold impulses, jitter) fed to `FrictionFsm::advance`
yields `commit_token() == None` in **every** case (accidental money-commit is unrepresentable —
item 6, §4.4). `release_before_threshold_cancels` — a hold released at `progress_ms < hold_ms`
transitions to `Cancelled`, `commit_token() == None`. `misaimed_hold_does_not_progress` — a
sustained gesture with `aimed=false` (cos-angle < `AIM_TOLERANCE`) never advances `progress_ms`.

### 3.4 M4 — Objective accessibility gate (HARD requirement, falsifiable)

The a11y gate is the paired independent checker (P7 Gender): the friction generator does not get
to self-certify "it's accessible." Three falsifiable gates, each a headless test that drives the
**same** `FrictionFsm` with a sensory channel masked and asserts a scripted user can both
**complete** and **cancel** a high-stakes action, plus a safety invariant across all channels.

- **`a11y_deaf_path`** (no audio): with `AudioParams` suppressed, a scripted agent using only the
  visual `FrictionField` (amplitude/hold progress) + the P58 ARIA-live status announcements
  completes a confirm (full `hold_ms` sustained → `commit_token().is_some()`) AND cancels (release
  → `None`). Assert the P58 mirror emits a `role=status`/`aria-live=polite` node announcing the
  stake tier and hold progress (P58 owns the mirror; M4 asserts the friction data reaches it).
- **`a11y_blind_path`** (no visual): with the field buffer masked, a scripted agent using only the
  `AudioParams` stream (rising pitch = stake, sustained tremolo tone = hold, resolved chord =
  commit / detune = cancel) + P58 announcements completes AND cancels; **and the money amount is
  read back before commit** (assert an `AmountReadback` announcement precedes any
  `commit_token().is_some()`).
- **`a11y_safety_invariant`** (both channels + none): reuses M3's 10⁴ fuzz but additionally asserts
  parity — the deaf agent and the blind agent reach commit at the **same** `hold_ms` for the same
  stake (visual and audio are parity-pinned to one `Stake`, P2), and neither channel offers a
  shortcut the other lacks.

**Pass/fail bar (the objective a11y gate):** all three tests GREEN, run in CI on every P64 change,
**and** a Playwright accessibility-tree assertion (P58's shared harness) that on the live checkout
surface the friction status node is present, named, and updates its `aria-valuenow` as hold
progresses — with the field buffer programmatically hidden (`visibility` masked) the confirm still
completes via keyboard-hold + announcements. A build where any of the three unit gates is RED, or
the Playwright hidden-field confirm cannot complete, is **NOT done** regardless of other green
totals. This is the falsifiable proof that a blind or deaf user completes/cancels a high-stakes
action reliably.

### 3.5 M5 — Voice: wake + Moonshine streaming + Whisper multilingual fallback

**Spec.** `VoiceSource: InputSource` (§2.4). Pipeline: mic → `AudioRing` → `WakeWordSpotter`
(tiny always-on keyword-spot; full ASR runs only after a wake — the battery lever, R1 §5) →
`AsrModel::feed` streaming → `AsrDelta`s → on `is_final`, hand the transcript to
`IntentClassifier` via `RawInput::VoicePhrase`. Two `AsrModel` impls: `MoonshineAsr` (Wave-0
default, English-strong, ~107 ms streaming) and `WhisperCppAsr` (multilingual fallback, buffered
30 s window). Selection: `VoiceProfile { locale, prefer }` picks Moonshine when the locale is
Moonshine-covered, else Whisper (R1 §5's "Moonshine multilingual is unproven"). Native only.

**RED test** (`inference/tests/voice.rs`): `voice_wake_then_transcribe` — a fixture PCM clip of
"[wake] accept order" drives `WakeWordSpotter` true then `MoonshineAsr` (stubbed with a fixture
transcript in unit tests; real model behind an `#[ignore]`-until-model-present flag, P41's pattern
at `llm.rs` tests) → `VoiceSource::poll` yields `Intent::Command(AcceptOrder)`. RED today (no
`voice.rs`). `wake_gate_blocks_asr` — with no wake, `feed` is never called (assert call-count 0);
this is the battery-lever falsifier.

**Adversarial:** `whisper_fallback_on_uncovered_locale` — `VoiceProfile{locale: uk}` selects
`WhisperCppAsr`, not Moonshine. `voice_never_bypasses_friction` — a voice `Intent::Command(ConfirmOrder)`
still routes through the `FrictionFsm` (voice completion = spoken read-back + affirmation, §4.3);
a voice command cannot mint a `CommitToken` without the readback-affirm sequence (assert). Battery
+ latency are measured in §7, not asserted (R1 §5 Risk #2 — measured, not assumed).

### 3.6 M6 — Shared `LocalInference` runtime (X12) + AiMode integration

**Spec.** `ModelRuntime` (§2.4) is the single substrate. Integration invariants:
- **Chat modality = P41's port, reused.** `runtime.chat()` returns `Some(&dyn LlmBackend)` iff the
  assistant is enabled, i.e. `AiMode != Off` per `BackendConfig::from_env`
  (`kernel/src/ports/llm.rs:275`). `AiMode::Off` ⇒ `chat() == None` (assistant absent — P41 C-a).
  No new mode enum; no second `from_env` (P2, item 19).
- **ASR + translate are AiMode-independent.** They are local *perception* and *i18n* primitives,
  "a different category from the AI-decision-authority boundary" (§16.34, `MASTER…:2162-2164`).
  A no-AI venue (`AiMode::Off`) **still has voice and translation**. This is load-bearing: gating
  voice on `AiMode::LocalOffline` would silently break voice for no-AI venues — explicitly wrong.
- **BYO-model spans all three** (§16.52): `register_byo` accepts an operator ASR model, MT model,
  or chat endpoint (the last composing P41's `ManagedApiAdapter` / C-g path — same code, no new
  transport).
- **One memory budget** (bulkhead, item 11): `budget_used()` is the single accounting; loading a
  fourth model when the budget is full returns `InferError::BudgetExceeded`, never OOMs the hub.

**RED test** (`inference/tests/runtime.rs`): `chat_gated_by_aimode` — `AiMode::Off` ⇒
`runtime.chat().is_none()`; `AiMode::LocalOffline` ⇒ `Some`. `asr_independent_of_aimode` — with
`AiMode::Off`, `runtime.asr().is_some()` when a voice model is loaded (the load-bearing test).
`byo_registers_across_modalities` — a BYO ASR model then appears in `caps().asr`. RED today (no
`inference/`).

**Adversarial:** `budget_exceeded_is_typed_not_oom` — loading past the budget returns
`BudgetExceeded`. `no_second_from_env` — grep gate: `BackendConfig::from_env` has exactly one
non-test call site (P41's invariant, extended to the runtime — item 14).

### 3.7 M7 — Implicit onboarding v1 (idle-field hints, no tutorial modal)

**Spec.** `HintPolicy` tracks per-intent `familiarity: u8` (persisted in the P66 wallet — living
memory, item 15). When the field is **settled** (FE-14 settle gate, P38 §3.5) and the user has not
yet demonstrated the surface's primary intent, the `Composer` overlays low-amplitude **affordance
hint** directives — gentle field perturbations at the location / using the gesture that *would*
express the next available intent, in the **same field language** as friction (§16.50: "teaches
its own use through the same state-communication mechanism §16.44 established"). A hint retires
once `familiarity[intent] >= HINT_MASTERY_THRESHOLD`. No modal, no text how-to.

**RED test** (`engine/tests/onboarding.rs`): `onboarding_hint_present_then_decays` — a fresh
profile's settled-idle `Scene` contains ≥1 hint directive for the surface's primary intent; after
`HINT_MASTERY_THRESHOLD` successful performances of that intent, the settled-idle `Scene` contains
**zero** hint directives for it. RED today (no `onboarding.rs`).

**Adversarial:** `hint_never_intercepts_real_input` — a hint directive is visual/audio only; it
never consumes or reorders a real `Intent` (assert a user input during a hint resolves to the
user's intent, not the hinted one). `hint_respects_settle` — hints appear only when settled; a
busy field shows none (no distraction mid-action).

---

## 4. The §16.44 friction numeric mapping in full (engineering-unknown E, owned here)

This section is the concrete, falsifiable answer the synthesis (§4-E) assigns to P64. The
*mechanism* (friction = field dynamics, not a modal) is closed (§16.44); the *mapping* is here.

### 4.1 Stake → amplitude (money magnitude), math-grounded

Human magnitude perception is logarithmic (Weber-Fechner); a linear money→amplitude map would make
a $5 and a $500 order either indistinguishable or the large one unusable. So:

```
friction_amplitude(stake) = clamp(
    FRICTION_AMP_BASE + FRICTION_AMP_LOG_GAIN · log10(1 + money_minor / FRICTION_UNIT_MINOR),
    FRICTION_AMP_BASE, FRICTION_AMP_MAX)
```

A $5 confirm (500 minor) → base + 0.11·log10(6) ≈ 0.08 + 0.086 ≈ 0.17. A $500 confirm (50 000
minor) → 0.08 + 0.11·log10(501) ≈ 0.08 + 0.297 ≈ 0.38. Visibly different, never runaway, clamped
so the field is never fully opaque/unusable. `intensity` (particle density from
`engine/src/field_energy.rs`) rides the same signal as a secondary amplifier. **The money amount
itself is never a `FieldValue`** — `friction_amplitude` reads `money.0` as an `i64` scalar
(`money_guard.rs:18`); the compile-proof at `money_guard.rs:117` guarantees no path animates the
amount.

### 4.2 Stake → rhythm (the actual friction: a longer, sustained commitment)

The friction *is* time. The completing gesture must be **sustained** longer as stakes rise:

```
friction_hold(stake) = HOLD_BASE_MS
    + HOLD_LOG_GAIN_MS · log10(1 + money_minor / FRICTION_UNIT_MINOR)
    + match reversibility {
        Reversible        => 0,
        ReversibleWithCost => HOLD_REVERSIBLE_COST_MS,
        Irreversible      => HOLD_IRREVERSIBLE_MS }
```

Rhythm (P3 Vibration — a named, single-authority, tested rate) is this hold window: a low-stake
reversible action commits in ~350 ms of hold; a large irreversible one demands ~1 s+ of deliberate
sustained gesture. The field's pulse/tempo visibly slows and deepens as the required hold grows, so
the *feel* of "this is heavier" is the literal rhythm of the required commitment.

### 4.3 Gesture grammar: complete vs cancel (P4 Polarity — two poles, one mechanism, safe-directed)

- **Complete** = a **sustained directed impulse**: hold a gesture aimed at the "commit well" (a
  field attractor the Composer places for a consequential response) for the full `hold_ms`. Aim is
  checked by cos-angle ≥ `AIM_TOLERANCE`; progress advances only while held *and* aimed
  (`FrictionFsm::advance(aimed, dt)`).
- **Cancel** = the **fail-safe default**: release before threshold, or a distinct pull-away
  (opposite pole). Doing nothing never commits. Releasing never commits. This makes cancel the
  zero-effort action and commit the effortful one — the correct asymmetry for a money action
  (degrade-closed; the safe pole is the default, P4).
- **Voice completion** (eyes-free): a consequential voice `Intent::Command` triggers a **spoken
  read-back of the amount** ("confirm twelve fifty?") and requires an explicit affirmation token
  ("yes"/"confirm") within a timeout; silence or "cancel" = the safe pole. The read-back +
  affirmation is voice's equivalent of the sustained hold (§3.5 `voice_never_bypasses_friction`).

**Audio channel equivalence** (the §16.50 hard requirement — derived from the *same* `Stake` by
the *same* functions, so it cannot drift from the visual, P2):

```
audio_params(stake) = AudioParams {
    pitch_hz:   AUDIO_PITCH_BASE_HZ · AUDIO_PITCH_DECADE_RATIO ^ log10(1 + money_minor/UNIT),
    tremolo_hz: derived from friction_amplitude (same log signal → a faster tremor at higher stake),
    hold_ms:    friction_hold(stake)   // IDENTICAL to the visual hold — parity-pinned
}
```

A blind user hears **pitch rise with stake**, must **sustain the tone** (via held gesture or a
held voice tone / repeated affirm) for the **same** `hold_ms`, and hears a **resolved consonant
chord** on commit / a **detune** on cancel. Same parameter, same threshold, different sense.

### 4.4 Hazard-safety as math (standard §2 item 6): accidental commit is unrepresentable

`CommitToken` has a **private** field sealed by a private ZST `Seal`; it is `!Clone`, `!Default`,
and constructible **only** inside `friction.rs` by `FrictionFsm::commit_token`, which returns
`Some` **iff** `state == Committed`, which is reachable **only** after `progress_ms >= hold_ms` of
sustained aimed input. P60's payment call site's signature **requires** a `CommitToken` (owned, by
value — consumed on use). Therefore:

> No code path — including the AI assistant, a replayed event, a fuzzed input, or a
> misclassified voice phrase — can move money without a `CommitToken`, and a `CommitToken` cannot
> exist unless a human sustained the stake-scaled commitment gesture. This is the money-safety
> invariant expressed in the type system (P1 Mentalism + P6 Cause-and-Effect), not a runtime
> check. The 10⁴-input fuzz (§3.3 `accidental_input_never_commits`) is the falsifier.

This composes with `money_guard.rs`'s existing compile-proof (money is not a `FieldValue`): the
*amount* cannot be tweened, and the *act* cannot be committed without the token. Two independent
compile-time guarantees around every money action.

---

## 5. Cross-cutting design obligations (standard §2 items 6, 8, 9, 11-16)

**Hazard-safety as math (item 6):** §4.4 (`CommitToken` unrepresentable-without-hold) + §3.1
(consequential intents never reach the AI ranker). The unsafe state "money moved without deliberate
consent" is not reachable by construction.

**Schemas & scaling axes (item 8):** the intent stream scales on **inputs/sec per client** (one
user, bounded — pointer ≤ ~120 Hz, voice ≤ a few phrases/min); it is **node-local**, never
gossiped (§12). The `FragmentRegistry` scales on **fragment count per surface** (tens, static);
it needs re-sharding only if a surface exceeds ~10³ fragments (far past Wave-0). `ModelRuntime`'s
scaling axis is **loaded-model bytes vs the memory budget** (`budget_used()`), the point at which
BYO models must be unloaded LRU. `HintPolicy` scales on **distinct intents per user** (tens).

**Linux discipline (item 9), verdict framework:** REUSES the `InputSource`/`Intent` grammar
(EXTENDS P38), the `LlmBackend` port (ALREADY-EQUIVALENT for chat), the `Scene`/`compose()` render
path (REINFORCES — no new render path). GAP filled: the classifier, friction FSM, voice pipeline,
runtime. DOES-NOT-TRANSFER: nothing — no prior UI-framework abstraction is imported (*ad fontes*).

**Isolation / bulkhead (item 11):** the native `inference/` crate is a **separate process/thread**
from the wgpu render loop; an ASR hang or a model OOM cannot stall rendering — the `InputSource`
seam is the bulkhead (a dead `VoiceSource::poll` returns `None`, other channels keep working,
§16.50 equal-channel resilience). The assistant chat model failing (`AiMode` path) never affects
ASR/translate (separate modality handles) nor the deterministic classifier — mirrors P41 C-e
("assistant down, orders still flow", `kernel/src/ports/llm.rs` degradation tests).

**Mesh awareness (item 12):** intent, friction, voice, onboarding, and inference are **entirely
node-local** — zero gossip, zero transport-layer payload, no `iroh_transport.rs` dependency. A
`CommitToken` is consumed locally by the local payment call; it never crosses the wire (no protocol
message carries `AiMode` or a `CommitToken`). Stated explicitly so no future change accidentally
gossips a consent token.

**Rollback / self-healing as math (item 13):** **Self-Termination** — the `CommitToken` boundary is
a hard invariant (unrepresentable un-consented money-move), not a supervisor decision (§4.4).
**Self-Healing** — a dropped voice model degrades to other equal channels (§16.50) with no state
loss (redundant input paths); the intent stream is idempotent (re-classifying the same `RawInput`
is a pure fn, P6). **Snapshot re-entry** — n/a (no persistent friction/intent state beyond the
per-action FSM, which resets to `Idle`); onboarding familiarity is the only persisted state and
LWW-restores from the wallet.

**Smart index / error-propagation isolation (item 14):** three grep/type gates turn P64's bug
classes into CI/compile failures: (a) `no_raw_event_leak` grep (event handlers only in
`InputSource` impls); (b) `consequential_intent_never_bare_commits` type+test (money response ⇒
`friction: Some`); (c) `no_second_from_env` grep (one AiMode constructor). Each is a build-time or
CI-time catch, not a runtime surprise.

**Living-memory awareness (item 15):** onboarding `familiarity` and `VoiceProfile` are
**temporal/personalized** state stored in the on-device wallet (P66), cross-referenced to
`internal-retrieval-living-memory-arc-2026-07-14` (demote-never-delete; a retired hint is dormant,
not erased — it re-wakes if the user later struggles, i.e. familiarity can decay).

**Tensor/spectral (item 16):** the friction field is the **existing Laplacian substrate** —
`friction_amplitude` sets the source term into `FieldFrame::step`/`compose`
(`engine/src/field_frame.rs:198,255`); no new field math is invented. The friction "commit well" is
a field attractor expressed in the same `LaplacianField` (`engine/src/field_frame.rs:106`) already
driving the UI. Reuse-first, not a parallel physics.

---

## 6. DoD — falsifiable, RED→GREEN, per item (standard §2 item 2)

Every row is a named test that is RED today (verified: the target files do not exist) and GREEN
after the build. CI runs all on every P64 change.

| # | Item | RED today because | GREEN when | Test |
|---|------|-------------------|-----------|------|
| D1 | Intent runtime | `Intent`/`InputSource` = 0 grep hits | pointer + voice round-trip to a field response | `engine/tests/intent_v1.rs::{intent_types_exist_and_exercised, voice_round_trip}` |
| D2 | Classifier purity + safety | no `intent.rs` | same input ⇒ identical class; consequential never `Ambiguous` | `intent_v1.rs::{classifier_is_pure, ambiguous_never_auto_commits}` |
| D3 | No raw-event leak | grep not wired | grep gate green (handlers only in `InputSource` impls) | `engine/tests/firewall.rs::no_raw_event_leak` |
| D4 | UI composition | no `compose_ui.rs` | intent → registered fragment `SdfShape`s in `Scene` | `engine/tests/compose_ui.rs::intent_composes_registered_fragment` |
| D5 | Consequential ⇒ friction | type unenforced | money intent ⇒ `friction: Some`; `None` fails | `compose_ui.rs::consequential_intent_never_bare_commits` |
| D6 | Friction mapping | no `friction.rs` | amplitude log-monotone+clamped; hold scales | `engine/tests/friction_map.rs::{amplitude_is_log_monotone, hold_scales_with_stake}` |
| D7 | Money-safety invariant (item 6) | no FSM | 10⁴-input fuzz ⇒ zero `CommitToken`; release cancels; misaim no-progress | `friction_map.rs::{accidental_input_never_commits, release_before_threshold_cancels, misaimed_hold_does_not_progress}` |
| **D8** | **Objective a11y gate (HARD)** | no gate | deaf-path + blind-path both complete **and** cancel; parity; amount read-back before commit; Playwright hidden-field confirm completes | `engine/tests/a11y_friction.rs::{a11y_deaf_path, a11y_blind_path, a11y_safety_invariant}` + P58 Playwright hidden-field assertion |
| D9 | Voice pipeline | no `voice.rs` | wake→ASR→intent; wake-gate blocks ASR; whisper fallback | `inference/tests/voice.rs::{voice_wake_then_transcribe, wake_gate_blocks_asr, whisper_fallback_on_uncovered_locale}` |
| D10 | Voice honors friction | unenforced | voice confirm needs read-back+affirm; no bare token | `voice.rs::voice_never_bypasses_friction` |
| D11 | Shared runtime + AiMode (X12) | no `inference/` | chat gated by AiMode; ASR/translate AiMode-independent; BYO across modalities; typed budget | `inference/tests/runtime.rs::{chat_gated_by_aimode, asr_independent_of_aimode, byo_registers_across_modalities, budget_exceeded_is_typed_not_oom}` |
| D12 | One AiMode constructor | grep not wired | `from_env` single non-test call site | `inference/tests/firewall.rs::no_second_from_env` |
| D13 | Implicit onboarding | no `onboarding.rs` | fresh profile hints present, decay after mastery; hints never intercept input | `engine/tests/onboarding.rs::{onboarding_hint_present_then_decays, hint_never_intercepts_real_input, hint_respects_settle}` |

**Not-done clauses:** any DOM `<input>` for voice/text (P38-rev §12.1 — struck); voice or intent
gated on `AiMode` for ASR/translate (breaks no-AI venues, §16.34); a money response without
`friction: Some`; a `CommitToken` constructible outside `friction.rs`; a raw event handler outside
an `InputSource` impl; the a11y gate D8 RED — any of these = **NOT done** regardless of other green.

---

## 7. Benchmark plan (standard §2 item 10 — measured, not asserted)

Reuse the existing `cargo bench` harness (P51 §7 precedent); four benches, telemetry hooks emit
each metric so regressions surface automatically (item 10).

1. **Intent classification latency** — `classify` p99 must be < **2 ms** (well under P38's 2 ms
   keystroke-to-pixel budget, R1 §1); it is a pure fn with a hit-test + Aho-Corasick match.
2. **ASR streaming latency** — `MoonshineAsr::feed` partial-emit p50 target ~**107 ms** (R1 §5);
   `WhisperCppAsr` measured (expected ~seconds — used only where real-time isn't required, §3.5).
   Behind an `#[ignore]`-until-model-present flag; the number is measured on real hardware, not
   assumed (R1 §5 Risk #2).
3. **Voice battery draw (the named unknown, R1 §5/§9 Risk #2)** — full-shift wake-gated ASR draw on
   a real mid/low-end phone, measured by **P63's battery harness** (this blueprint declares the
   metric + threshold; P63 owns the rig). Falsifier: wake-gated ASR must draw materially less than
   continuous ASR (the `wake_gate_blocks_asr` test proves the gate; the bench proves the saving).
4. **Friction FSM tick** — `advance` is O(1); assert < **50 µs** so it never perturbs the render
   loop.

---

## 8. Anti-scope — Track-R vs Wave-0-v1, explicit (standard §2 items 3, 18)

**Wave-0-v1 (built here):** deterministic intent classifier; fragment-**selection** UI composition;
friction numeric mapping + gesture grammar + audio channel + a11y gate; wake + Moonshine + whisper
voice; shared `LocalInference` runtime; idle-field onboarding hints.

**Track-R (named, NOT built here — off the critical path per SYNTHESIS §5 Track-R):**
- **Generative UI "from scratch"** (§16.35 clause 2 — the local model *drawing* UI procedurally,
  not selecting a registered fragment). v1 ships fragment-selection only; a generative producer may
  later plug in behind the **unchanged** `ComposedResponse` contract (§2.2) — the seam is named now.
- **Physics/wave-generated glyphs** (§16.39) — MSDF Tier-1 (P38 FE-06) is the text path; procedural
  glyphs gate on an objective legibility bar, never on an order screen (R1 §4).
- **AI-ranked disambiguation beyond exploratory navigation** — the `IntentClassifier`'s
  `Ambiguous` → AI-rank path is spec'd (§3.1) but Wave-0-bounded to non-consequential nav; broader
  learned intent inference is Track-R.
- **Deeper friction perceptual model** — the log-amplitude/hold scheme (§4) is v1; user-study-tuned
  perceptual curves and per-user adaptation are Track-R (the constants in §2.3 are the tuning seam).
- **Multilingual streaming ASR quality** — proving Moonshine-class multilingual (vs whisper
  fallback) is a Track-R measurement (R1 §5 Risk #2); v1 ships whisper as the honest fallback.

**Explicitly out of P64 (owned elsewhere):** text editing/IME (P57); the a11y **mirror** itself and
its Playwright harness (P58 — P64 asserts friction data *reaches* the mirror, does not build it);
the payment call site that consumes `CommitToken` (P60); the assistant loop and MCP tools
(P40/P42); the shell/battery-rig spike (P63); wallet persistence of familiarity (P66).

---

## 9. Dependencies (standard §2 items 7, 18)

**Consumes (inputs):**
- **P38-rev §11.2 / §12.2** — `Intent`/`FieldPos`/`InputSource`/`WidgetId`/`NavTarget`/`CommandId`
  grammar (P64 extends, does not redefine — single-owner: intent grammar → P64); the four AR/VR
  insurance constraints (P64's classifier satisfies constraint 3 concretely); `Scene`/`SdfShape`/
  `compose()`/`FieldEquilibrium`/`money_guard` render + money substrate.
- **P41** — `AiMode`/`BackendConfig::from_env`/`LlmBackend` (`kernel/src/ports/llm.rs`); the chat
  modality reuses this verbatim; the runtime honors P41's single-constructor + fail-closed-Off
  invariants.
- **P58** (a11y mirror + Playwright harness) — the a11y gate D8's mirror assertions target P58's
  `role=status`/live-region convention; P64 supplies the friction status data, P58 renders/tests
  the mirror. Coupled: D8 cannot go fully green until P58's harness exists (noted; the unit gates
  a11y_deaf/blind/safety are P64-standalone).
- **P63** (shell/battery spike) — supplies the battery rig for §7 bench 3 and the native-shell
  verdict the `inference/` crate targets.

**Feeds (consumers):**
- **P69** (customer storefront & checkout) — every customer interaction is an `Intent`; the
  checkout confirm/cancel is the friction FSM + `CommitToken`; voice is an equal channel.
- **P70** (owner surface) — owner actions are `Intent`s; destructive owner actions (delete,
  refund) use the friction mapping; the assistant (AiMode) is obtained from the runtime.
- **P71** (courier surface) — `InputProfile::CourierInMotion` voice-primary bias (§16.53); accept/
  decline are voice `Intent::Command`s; battery gates from P63.
- **P60** (payment) — consumes `CommitToken` at the card/charge call site (the money-safety seam).
- **P66** (wallet) — persists onboarding `familiarity` + `VoiceProfile`.

---

## 10. Links to docs & memory (standard §2 item 7)

- Standard: `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2.
- Synthesis (owner assignment): `docs/design/CORE-ROADMAP-2026-07-17/SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md`
  §5 (W1 P64), §2 (X12), §4-E (friction mapping unknown), §3.2 (sequencing).
- Research: `docs/research/OPUS-R1-INTERFACE-RENDERING-2026-07-18.md` §5 (voice), §2 (a11y), §9
  (Risks #2/#5/#6).
- Canon: `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.4/§16.31/§16.34/§16.35/§16.38/
  §16.40/§16.44/§16.50/§16.51/§16.52/§16.53; §16.42-43 (*ad fontes*).
- Existing blueprints: `BLUEPRINT-P38-webgpu-render-engine.md` (§11.2 types, §12.2 AR/VR + P57
  canon-diff), `BLUEPRINT-P41-three-mode-ai-operation.md` (AiMode/LlmBackend), `BLUEPRINT-P51-open-map-routing.md`
  (format precedent).
- Code cited: `kernel/src/ports/llm.rs`, `engine/src/{scene,field_frame,money_guard,lib,field_energy}.rs`.
- Hermetic: `docs/design/hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md`.
- Regression: register D7/D8 in `docs/regressions/REGRESSION-LEDGER.md` (item 17) — the
  money-safety fuzz and the a11y gate are permanent.

---

## 11. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **P4 Polarity** (`…PRINCIPLES.md` P4) — complete vs cancel are two named poles of **one** gesture
  mechanism (sustained-aimed-hold vs release), **safe-directed**: release = cancel = the zero-effort
  default (§4.3). No two separate code paths for the two outcomes.
- **P6 Cause-and-Effect** (P6) — intent → composed response is a pure reproducible function; the AI
  assistant is structurally forbidden from being a cause of a money effect (`CommitToken`, §4.4).
- **P3 Vibration** (P3) — the friction **rhythm** (`friction_hold`, the tempo of the field pulse) is
  a named, single-authority, tested rate (§4.2), not an incidental animation.
- **P2 Correspondence** (P2) — the audio friction channel is **parity-pinned** to the visual channel
  (same `Stake`, same functions, §4.3), so they cannot drift; the `LocalInference` runtime is **one**
  substrate for three modalities, not three drifting runtimes (X12).
- **P7 Gender** (P7) — the objective a11y gate (§3.4/D8) is the **independent checker** paired with
  the friction generator; the friction code does not self-certify "accessible."
- **P1 Mentalism** (P1) — the intent grammar + friction mapping spec (§2/§4) precedes the classifier/
  FSM code; the money-safety invariant lives in the type system, the derived artifact.

---

## 12. Standard-compliance map (all 20 points, checkable)

| # | Standard item | Where satisfied |
|---|---------------|-----------------|
| 1 | Ground truth (`file:line`, this pass) | §0 (all cites re-verified; the AiMode-location correction recorded) |
| 2 | Falsifiable DoD (RED→GREEN) | §6 (13 rows, each a named test RED today) |
| 3 | Spec-driven + event-driven TDD | §2 (types first) → §3 (RED test → code; tests assert on intent sequences) |
| 4 | Predefined types & constants | §2 (full type surface + named constants; no magic numbers) |
| 5 | Adversarial / intentionally-failing tests | §3 (each M-item has adversarial cases; §3.3/§3.4 fuzz + break-the-invariant) |
| 6 | Hazard-safety as math | §4.4 (`CommitToken` unrepresentable-without-hold) + §3.1 (AI never touches money intent) |
| 7 | Links to docs & memory | §9, §10 |
| 8 | Schemas designed for scaling | §5 (inputs/sec, fragment count, model-bytes vs budget, intents/user) |
| 9 | Linux engineering discipline | §5 (REUSE/EXTEND/GAP verdict framework) |
| 10 | Benchmarks + telemetry | §7 (four benches, telemetry hooks; ASR/battery measured not assumed) |
| 11 | Isolation / bulkhead | §5 (native `inference/` process seam; assistant-down ≠ intent-down) |
| 12 | Mesh awareness | §5 (all node-local; `CommitToken`/`AiMode` never gossiped — stated) |
| 13 | Rollback/self-healing as math | §5 (Self-Termination = CommitToken boundary; Self-Healing = redundant channels) |
| 14 | Smart index / error-propagation | §5 (three grep/type gates) |
| 15 | Living-memory awareness | §5, §3.7 (familiarity/VoiceProfile; demote-never-delete) |
| 16 | Tensor/spectral reuse | §5, §4.1 (friction rides the existing Laplacian/`FieldFrame` substrate) |
| 17 | Regression tracking | §10 (D7/D8 permanent in the ledger) |
| 18 | Clear instructions for agents | §13 |
| 19 | Reuse-first, upgrade-if-needed | §2 (reject-list), §5 (item 9), §8 (extends P38/P41, no new UI lib) |
| 20 | Hermetic principles | §11 (P1-P4, P6, P7 — load-bearing) |

---

## 13. Clear instructions for other agentic workers (standard §2 item 18 — zero session context assumed)

Build order (each item RED→GREEN before the next; one crate module per step):

1. **`engine/src/intent.rs`** — import P38 §11.2's `Intent`/`FieldPos`/`InputSource`/`WidgetId`/
   `NavTarget`/`CommandId` (land them if P38b has not — they are the 0-grep-hit baseline); add
   `Classification`/`RawInput`/`IntentClassifier`/`IntentContext`/`InputRouter`/`InputProfile`
   (§2.1). Make `engine/tests/intent_v1.rs` + `firewall.rs::no_raw_event_leak` green (D1-D3).
2. **`engine/src/compose_ui.rs`** — `FragmentFn`/`FragmentRegistry`/`Composer`/`ComposedResponse`
   (§2.2) over `engine/src/scene.rs`'s `Scene`/`SdfShape`. Green `compose_ui.rs` (D4-D5).
3. **`engine/src/friction.rs`** — `Stake`/`Reversibility`/`FrictionField`/`AudioParams`/
   `FrictionFsm`/`CommitToken` + the §2.3 constants + the §4 functions. Green `friction_map.rs`
   (D6-D7). **`CommitToken` MUST have a private field + private `Seal` ZST — do not make it
   `pub`-constructible.**
4. **`engine/tests/a11y_friction.rs`** — the three a11y gates (D8). Coordinate the mirror-assertion
   half with P58's harness owner; the deaf/blind/safety unit gates are standalone.
5. **`inference/` crate** (native only, never WASM) — `ModelRuntime`/`AsrModel`/`TranslateModel`/
   `InferenceCaps` (§2.4); `VoiceSource`/`WakeWordSpotter`/`MoonshineAsr`/`WhisperCppAsr`
   (`inference/src/voice.rs`). Chat modality imports `kernel/src/ports/llm.rs`'s `LlmBackend`
   verbatim. Real ASR models behind `#[ignore]`-until-present (P41's pattern). Green
   `inference/tests/{voice,runtime,firewall}.rs` (D9-D12). **ASR/translate MUST NOT be gated on
   `AiMode`** — only `chat()` is (§3.6).
6. **`engine/src/onboarding.rs`** — `HintPolicy`; wire into `Composer` idle-state path; persist
   `familiarity` via P66's wallet (stub the store if P66 is not ready). Green `onboarding.rs` (D13).

Acceptance = all 13 DoD rows green in CI + the four §7 benches recording real numbers (ASR/battery
behind the model/hardware flags). Do **not** mark done with D8 (a11y gate) RED. Do **not** add a
UI-library dependency (*ad fontes*, §16.42-43). Do **not** let the AI assistant construct a
`CommitToken` or classify a consequential intent (§16.4/§4.4).
