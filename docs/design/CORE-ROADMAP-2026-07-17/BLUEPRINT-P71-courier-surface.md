# BLUEPRINT P71 — Courier surface: the rendered, voice-primary, dispatch-wired courier app (P52-rev) (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9). Wave **W3**,
> component **DELIVERY / courier**. Source scope: `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md`
> §5 W3 table row **P71** ("extends P52: full-wgpu courier app; voice-primary in motion (§16.53);
> P51 map/route integration; accept/decline against P65; **battery gates from P63's measured
> baseline as DoD**"), and §3 (**M2 = first delivery order**, gated by **P71 + P65** live).
> Structural template: `BLUEPRINT-P51-open-map-routing.md`; direct predecessor it revises:
> `BLUEPRINT-P52-courier-working-surface.md` (P71 **is** P52-rev — §1 states exactly what it
> preserves vs supersedes vs adds).
>
> **The one-sentence thesis:** the courier *protocol* leg is built (claim machine, HRW matcher,
> PoD, settlement — the most-built part of the stack), and P52 already designed the courier's
> *fold/law/capture* logic (K1-K8) as consumable kernel state; what remains — and what P71 owns —
> is the **realized full-wgpu surface** for a human courier, its **voice-primary-in-motion**
> interaction, and its binding to the **now-existing P65 dispatch orchestrator** — a binding that
> **supersedes P52's self-owned 60 s offer window** with P65's hub-owned 30 s deadline. P71 renders
> and wires; it re-owns none of P52's protocol-consumption logic and re-designs none of P51/P65/P64.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working trees on `main` (`/root/dowiz`) and `/root/bebop-repo`, 2026-07-18. All fresh reads. The
**three load-bearing findings**, each a supersede/extend seam this blueprint resolves in §1:

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| **P52 owns its own offer-expiry window, value 60 s, surface-computed**: `OFFER_DECISION_WINDOW_SECS: u64 = 60` ("DZ-08's 60s auto-decline, kept: expiry emits ClaimReleased"), and K2 "decline (or `OFFER_DECISION_WINDOW_SECS` expiry) relays `ClaimReleased`" | `BLUEPRINT-P52-courier-working-surface.md:139` (const), `:240` (K2 expiry) | **VERIFIED — SUPERSEDED by P65 (§1.2 finding A):** the accept-timeout is now hub-owned dispatch Law, one value for all couriers, not a surface constant |
| **P65 owns the accept-timeout as hub-side dispatch Law, value 30 s, courier-independent**: `OFFER_TIMEOUT_SECS: i64 = 30`; deadline `= now_ts + OFFER_TIMEOUT_SECS` with **no courier input**; on expiry the driver releases (`Offered→Released`) + advances | `BLUEPRINT-P65-dispatch-orchestrator.md:228` (const), `:98-110` (§1.2), `:246-249` (`LiveOffer`) | **VERIFIED — P71 renders this deadline; it defines no competing window** |
| **P65's offer/accept/decline event contract** (the exact API P71's inbox consumes/emits): `DispatchEvent::{Offered{courier,deadline_ts}, Advanced{from,reason}, Assigned{courier}, RoundExhausted, Requeued, StaleAccept{courier}}`; `DispatchInput::{Tick, Accept{courier}, Decline{courier}, OnlineSetChanged}`; `DispatchSession::tick(input, candidates, now_ts) -> Vec<DispatchEvent>` | `BLUEPRINT-P65-dispatch-orchestrator.md:273-309` | **VERIFIED — P71 consumes `DispatchEvent` inbound, emits `DispatchInput` outbound; mints no dispatch type** |
| **P65 late-accept race already resolved** by an existing gate: `Accept` after timeout ⇒ claim already `Released`, `Released→Claimed` illegal ⇒ `StaleAccept` surfaced, never a double-assign | `BLUEPRINT-P65-dispatch-orchestrator.md:367-382` (§4.3) | **VERIFIED — P71 renders `StaleAccept` as "offer passed on", honestly; adds no race logic** |
| **P65's no-scoring red line**: a decline/timeout on this order cannot penalize a courier on the next order; `assign` takes no history parameter; the two-world byte-identical test (`decline_never_penalizes_future_ranking`) enforces it | `BLUEPRINT-P65-dispatch-orchestrator.md:113-132` (§1.3), `:420-426` (Test A) | **VERIFIED — binding on P71: the surface must show NO missed-offer / accept-rate metric (§4.1); the CI gate extends over P71 files** |
| **P52's explicit "NOT voice" anti-scope**: "NOT voice/turn-by-turn — DZ-10 Phase-9b + P51 anti-scope, unchanged" | `BLUEPRINT-P52-courier-working-surface.md:101` | **VERIFIED — SUPERSEDED (§1.2 finding B):** §16.53 voice-primary-in-motion + P64's `VoiceSource` now make voice in-scope for the courier surface (command/navigation voice, still NOT turn-by-turn TTS routing) |
| **P52 deferred all GPU rendering** to `#[ignore = "O18a"]`; "CPU-compose assertions land now"; run screen `#[ignore = "O18a"]` | `BLUEPRINT-P52-courier-working-surface.md:623-627` (T7), `:78-81` ("NOT a fourth rendering technology" — P38a pipelines) | **VERIFIED — EXTENDED (§1.3):** P71 lands the *actual* full-wgpu realization + the FE-16 floor-parity + a11y-mirror DoD P52 left as O18a stubs |
| **P64's voice contract** (what P71's voice binding consumes): `VoiceSource: InputSource` (native-only `inference/` crate, never WASM); `InputProfile::{Balanced, CourierInMotion, HandsFree}` where `CourierInMotion` biases toward voice **without disabling** other channels; `RawInput::VoicePhrase{transcript,confidence,is_final}` → `IntentClassifier::classify` → `Classification::Resolved(Intent::Command/Navigate)`; consequential-voice completion = spoken read-back + affirmation (`voice_never_bypasses_friction`); ASR is **AiMode-independent** (voice works with `AiMode::Off`) | `BLUEPRINT-P64-intent-engine-friction-voice.md:250-257` (VoiceSource), `:97-98` (InputProfile), `:119` (VoicePhrase), `:435-437` (§4.3 read-back), `:448-450` (§3.6 AiMode-independent), `:736-737` (P64's own "feeds P71" note) | **VERIFIED — P71 sets `CourierInMotion`, maps offer/run intents; owns no ASR/classifier code** |
| **P64's audio-channel equivalence** (reused for the countdown cue): `AudioParams{pitch_hz,tremolo_hz,hold_ms}` derived from the same stake by the same functions as the visual field, parity-pinned (P2) so audio and visual cannot drift | `BLUEPRINT-P64-intent-engine-friction-voice.md:205` (type), `:546-559` (§4.3 equivalence) | **VERIFIED — P71's timeout-urgency cue reuses this path; invents no second audio model** |
| **P51's tracking contract** (what P71's run screen renders): `TrackFrame{est,v_mps,eta_s,remaining_m,route_version}` — "two consumers, one implementation — the **courier surface** (route + own marker + ETA — Sea grammar) and the customer live-track view"; `map_scene(pack_roads,route,marker,viewport)->Scene`; `CourierTrack::ingest(sample,route)->Vec<TrackEvent>`; `CourierPositionUpdated` **privacy invariant** (emit ONLY between assignment-accept and delivery-complete) | `BLUEPRINT-P51-open-map-routing.md:354-356` (`TrackFrame`), `:245` (`map_scene`), `:236` (`ingest`), `:359-363` (§4.6 privacy) | **VERIFIED — P71 consumes P51's outputs verbatim (P52 §3.3 already bound them); re-designing routing/tracking is a scope violation** |
| **P63's SP-5 battery baseline** (the DoD source): bars `SHIFT_HOURS_SIM=6.0`, `SETTLED_DRAIN_BAR_PCT_PER_HR=4.0`, `SETTLE_SAVINGS_MIN_PCT=30.0`, `THERMAL_SUSTAIN_MS=33.0`; `HwClass::BudgetAndroid` (emulator forbidden); SP-5 "P63 measures, **P71 gates**" | `BLUEPRINT-P63-shell-platform-spike.md:170-173` (bars), `:128` (HwClass), `:83-85` (§1 "P71 consumes"), `:316-350` (SP-5) | **VERIFIED — P71's battery DoD is SOURCED from these numbers, not invented** |
| **P63-VERDICTS.md does NOT exist this pass** (`test -f … → ABSENT`); SP-5's honest verdict until physical budget-Android hardware is present is `Blocked{on:"physical budget Android device / first-client device fleet"}` | `docs/design/CORE-ROADMAP-2026-07-17/P63-VERDICTS.md` (absent, ls this pass); `BLUEPRINT-P63…:346-350` (Blocked arm) | **VERIFIED — the battery gate's *number* is OWED, not measured; P71's battery DoD is therefore CONDITIONAL on SP-5 landing (§5, `#[ignore="P63-SP5-baseline"]`)** |
| **P63's SP-6 floor-parity method** (the reusable render-correctness DoD line other surfaces paste): "passes `floor-parity` at ΔE ≤ 0.02 on WebGPU, WebGL2, and CPU rungs"; `PARITY_PERCEPTUAL_DELTA_MAX=0.02`; durable harness `engine/tests/floor_parity/` + `web/tests/floor-parity.spec.mjs` | `BLUEPRINT-P63-shell-platform-spike.md:368-374`, `:175`, `:474` | **VERIFIED — P71 imports this as a DoD line (§5), does not re-invent parity** |
| **P39-rev/P63 shell ruling** the courier app inherits: desktop = winit+wgpu+AccessKit (no webview); **mobile keeps the Tauri webview host + full-wgpu surface**; card path = Path B native SDK sheet | `BLUEPRINT-P63-shell-platform-spike.md:40` (ruling), `:71` (grep 0 winit/tauri) | **VERIFIED — the courier is mobile-primary (a rider) ⇒ Tauri mobile shell + full-wgpu surface, §16.8 installed-app daily path** |
| **§16.34 courier-app-is-also-full-wgpu is a DELIBERATE tradeoff**, named: "chosen for rendering-architecture consistency … over the battery/simplicity tradeoff a lighter native courier UI would have offered … a courier on a bike running a GPU-rendered UI for a full shift is a genuine battery-life question the Tier-3/**P52 build needs to benchmark, not assume away**" | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:2165-2169` | **VERIFIED — P71 is where that named benchmark lands, sourced from P63 SP-5, NOT a lighter alternative UI (§1.3, §4.1)** |
| **§16.53 voice-primary-in-motion is a NARROW safety exception**, not a paradigm change: "hands-busy/eyes-on-road contexts specifically favor voice, other contexts keep all channels equal" (§16.50) | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:2398-2401` | **VERIFIED — P71 sets `CourierInMotion` only while in transit; idle / at-pickup / reviewing-the-day keep equal channels (§3.3)** |
| **§17.6 courier protocol is species-agnostic**: "a courier is any agent holding a valid cert … Drone/robot couriers use the same courier protocol, not a new one" | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:2599-2603` | **VERIFIED — P71's UI is the human-courier RENDER case; the PROTOCOL it consumes (P65 `CourierKey`, claim machine, PoD, cert) assumes no humanity (§1.4 boundary, §4.1 reachability)** |
| **P52's preserved substrate** (consumed unchanged): K1 availability (`DutyFold`/`candidates`), K4 PoD capture + §3.4b NFC-tag evidence, K5 earnings fold, K6 `EnrollInvite`/`redeem_invite`, K7 cash attestation, K8 conversation pane; phase falsifier `p52_courier_end_to_end` | `BLUEPRINT-P52-courier-working-surface.md:68-74` (K-items), `:308-342` (§3.4b NFC), `:492-496` (falsifier), `:637-724` (K8) | **VERIFIED — P71 preserves ALL of these unchanged (§1.1); it renders/wires them, re-owns none** |

Ground truth is non-discussible; everything below builds on this table only. **The through-line:**
P52 designed the courier's *logic* against a world where P65, P64's voice, and P63's battery
baseline did not yet exist; all three now exist on paper, and P71 is the reconciliation that binds
the surface to them — one supersede (the timeout), one un-defer (voice), one realization (full-wgpu
+ battery gate).

---

## 1. Scope — precisely what P71 preserves, supersedes, and adds vs P52

P71 **is** P52-rev. This section is the contract the task demands: no double-ownership, every
delta named. P52's §0-§11 stay on disk as the protocol-consumption design; a one-line pointer to
this file is added at merge time (the P43-§11 append convention P52 itself uses), not a rewrite.

### 1.1 PRESERVED UNCHANGED from P52 (P71 renders/wires these; re-owns none)

| P52 item | What it is | P71 relationship |
|---|---|---|
| **K1** availability (`DutyFold`, `candidates` bootstrap law, `POST /api/courier/duty`) `P52:68,§3.1` | the audit-M4 stopgap + node-local duty fold + cap-gated toggle | **unchanged.** P71 renders the duty screen; the duty fold feeds P65's `candidates` input (the online set), exactly as designed |
| **K4** PoD capture (geo+timestamp `DeliveryClaim`, k-of-n hub sigs) `P52:71,§3.4` | the UI for the built PoD crypto; `pod_location_bytes` 12-byte micro7 | **unchanged.** The signature-load-bearing artifact and its emit-gate-on-`is_settled` stay verbatim; P71 renders the capture flow through full-wgpu |
| **§3.4b** NFC-tag PoD (optional evidence leg — NTAG213/215/216, `tools/nfc-pod-codec` SHAKE256 MAC, server-side verify, phone reader, Flipper-as-production rejected) `P52:308-342` | an *optional* corroborating evidence source, never a gate on `is_settled()` | **unchanged.** The tag tap folds into K4's capture step as before; P71 adds no NFC logic, keeps the "never gates settlement" invariant |
| **K5** earnings (`fold_statement` over `SettlementRecorded`, `TweenGuard` money law) `P52:72,§3.5` | derive-only second reader; i64 minor units; no count-up | **unchanged.** Rendered through full-wgpu with the money-display discipline preserved |
| **K6** invite handoff (`EnrollInvite`/`redeem_invite` over `DOMAIN_DELEGATION`) `P52:73,§3.6` | the P48-DoD-4 ↔ P23-P2 bridge; single-use TTL'd cert | **unchanged.** P71 renders the first-launch enrollment scan/paste in the real shell |
| **K7** cash attestation (witness-typed, hub-derived amount) `P52:74,§3.7` | P47's input surface; the one money-adjacent action | **unchanged.** Hub-derived amount, emit-site witness — P71 renders the one-tap, adds no money arithmetic |
| **K8** run-scoped conversation pane (P48 §10 spine, access-window = P51 privacy window) `P52:637-724` | courier-side of the unified conversation; agent can never complete/deliver/attest | **unchanged.** Rendered in the run screen; the §10.6 order-authority walls + no-scoring gate hold verbatim |

**Net for §1.1:** every P52 K-item's *fold/law/capture/crypto* is frozen. P71 touches the
**presentation** and the **real-time bindings**, nothing beneath them.

### 1.2 SUPERSEDED by P71 (P52 wrote these before P65 / P64 existed; they change)

**A — Offer timeout: `OFFER_DECISION_WINDOW_SECS = 60` is STRUCK; the courier surface renders
P65's hub-owned `OFFER_TIMEOUT_SECS = 30` deadline instead** (`P52:139,:240` → `P65:228,:98`).
P52 K2 had the *surface* compute a 60 s auto-decline and emit `ClaimReleased` on expiry. P65 —
written after — makes the accept-timeout **hub-side dispatch Law**: one courier-independent value
(30 s), computed as `now_ts + OFFER_TIMEOUT_SECS` with no courier argument (its structural
anti-scoring property, `P65:107`), exposed as `DispatchEvent::Offered{courier, deadline_ts}`. The
surface no longer *owns* the expiry; it **renders the countdown to P65's `deadline_ts`** and, if
the courier does nothing, P65's own hub-side timeout fires (`Advanced{TimedOut}`) and advances to
the next-ranked courier. **P52's constant is deleted; P71 carries no offer-window constant of its
own** — the single source of truth for "how long a courier has to accept" is P65 (`§2`, `§3.2 R2`).

**B — Voice: P52's "NOT voice/turn-by-turn … unchanged" anti-scope (`P52:101`) is SUPERSEDED for
the command/navigation half.** §16.53 makes voice the *practical primary* input while a courier is
in motion, and P64 now ships `VoiceSource: InputSource`. P71 brings voice **into** the courier
surface: offers are announced and accepted/declined by voice, run-screen actions are voice
`Intent::Command`s. **What stays out** (the part of P52:101 that is *not* superseded): **turn-by-turn
spoken navigation** (DZ-10 Phase-9b + P51's own anti-scope, `P51:154-155`) — P71 speaks the *offer*
and *state*, it does not narrate the route. (§3.3, §2 anti-scope.)

**C — Rendering: P52's `#[ignore = "O18a"]` GPU deferral (`P52:623-627`) is SUPERSEDED by the
actual full-wgpu realization.** P52 correctly said "every pixel goes through P38a" but landed only
CPU-compose stubs. P71 lands the rendered app: the P39-rev/P63 mobile shell hosting P38a's
pipelines, the FE-16 WebGL2/CPU floor-parity gate (P63 SP-6), and a11y-mirror-everywhere (P58).
(§1.3, §3.2 R1.)

### 1.3 ADDED by P71 (surface work P52 did not specify — the four binding pillars)

1. **Full-wgpu courier app, deliberately not a lighter UI (§16.30/§16.34/§16.51).** The courier app
   is the **Tauri mobile shell** (P63/P39-rev mobile ruling; §16.8 installed-app daily path) with a
   **full-wgpu surface** rendering K1-K8 through P38a — the *same* rendering paradigm as customer
   (P69) and owner (P70), NOT a battery-optimized alternative UI. §16.34 accepted this tradeoff
   knowingly ("over the battery/simplicity tradeoff a lighter native courier UI would have
   offered"); §16.51 says the budget-device answer is an *efficient kernel* ("без тяжких
   бібліотек"), never a second UI. P71's rendering DoD is the P63 SP-6 floor-parity line + P58
   a11y-mirror parity. (§3.2 R1.)
2. **Voice-primary in motion (§16.53).** `InputProfile::CourierInMotion` (P64) is set while an
   `ActiveRun` is *in transit* — offers spoken, accept/decline by voice affirmation, hands/eyes
   free — while idle / at-pickup / reviewing-the-day keep all channels equal (§16.50). (§3.2 R3.)
3. **Dispatch integration against P65 (the accept/decline UX).** The courier's offer inbox and
   accept/decline are bound to P65's `DispatchSession` driver — including the exact
   timeout-approaching UX the task asks for (§3.2 R2). (§3.2 R2, R3.)
4. **Battery gates as REAL DoD, sourced from P63 SP-5.** The full-shift battery question §16.34
   named is answered by a gate whose numbers come from P63's SP-5 baseline — and, because
   `P63-VERDICTS.md` is absent this pass (SP-5 `Blocked`), the gate is *defined now, asserted when
   SP-5 lands* (§5). (§3.2 R5.)

### 1.4 Species-agnostic boundary (§17.6) — humanity lives ONLY in the render layer

P71's UI is the **human-courier** case. Everything it *talks to* is species-agnostic: P65's
dispatch keys on `CourierKey = [u8;32]` (a cert-holder, not a person), the claim machine, the PoD's
k-of-n hub signatures, and the capability cert all treat "courier" as "any agent with a valid cert"
(`P65:37,§17.6`). A drone/robot courier holding a valid cert uses the **same** P65 dispatch, claim
machine, and PoD — it simply presents a machine/headless agent in place of this human surface. **P71
must bake no human-only assumption into any protocol contract it consumes** (voice, the visual
field, and taps are render-layer only; the accept it emits is `DispatchInput::Accept{courier}`,
identical whether a human or an autonomous agent sent it). Stated as a hazard-reachability
invariant in §4.1.

### 1.5 P71 explicitly does NOT own (each with its owner)

- **NOT the dispatch driver.** `DispatchSession`/`tick`/the offer-sequence/no-scoring logic are
  **P65's** (`bebop2/proto-cap/src/dispatch.rs`). P71 consumes `DispatchEvent`, emits
  `DispatchInput` over the wire; a diff that re-implements offer-advance or forks the timeout is a
  scope violation. (P65 lives in `bebop-repo`; P71's surface mirrors the event shape, does not
  import the crate — the cross-repo boundary P52 already respects.)
- **NOT routing / maps / tracking / ETA math.** **P51's** lane entirely — `TrackFrame`, `map_scene`,
  `CourierTrack`, re-route, the position-privacy invariant. P71 renders P51's outputs. (§3.2 R4.)
- **NOT the voice pipeline / intent classifier / friction FSM.** **P64's** lane — `VoiceSource`,
  ASR, `IntentClassifier`, `FrictionFsm`, `CommitToken`. P71 sets the profile and maps offer/run
  intents; it builds no ASR and no classifier.
- **NOT the battery *measurement*.** **P63's** SP-5 — P71 imports the baseline, it does not run the
  rig ("P63 measures, P71 gates", `P63:83-85`).
- **NOT money-safety / `CommitToken`.** No money leaves the courier on this surface: K7's cash
  amount is **hub-derived** (`P52:394`), not UI-supplied. `CommitToken` (P64/P60) gates *customer
  money moves* (checkout/refund) and is out of scope here — accepting a delivery is a coordination
  commitment, not a money leg (§4.1).
- **NOT courier scoring / ratings / accept-rate / missed-offer counts — EVER.** The P52 §4.1 +
  P65 §4.6 red line, restated as a P71 anti-scope: **no P71 surface may display or store a
  per-courier decline/timeout/accept metric.** The `ci-no-courier-scoring.sh` file set extends over
  P71's modules (§4.1).
- **NOT P34's event vocabulary, the matcher, the order/claim FSMs, the PoD crypto, P47 money
  semantics, P48 owner hub, P49 customer identity** — all consumed exactly as P52 already
  established (`P52 §1`), unchanged.
- **NOT multi-order batching, NOT turn-by-turn spoken navigation** — P52's ground rules, kept.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── apps/courier/ — NEW Tauri mobile shell (the FULL-WGPU realization) + engine wiring.
//    P52 landed only #[ignore="O18a"] CPU stubs (P52:626); P71 lands the rendered app.
//    Renders P52's K1-K8 kernel folds through P38a in the P39-rev/P63 mobile shell. ──

/// Which shell the courier app runs in (P39-rev §1.2 / P63 SP-2 verdict). The rider is
/// MOBILE-PRIMARY → TauriMobile is the daily-use default (§16.8). WinitDesktop is only the
/// back-office/dispatcher variant. Both host a FULL-WGPU surface (§16.30/§16.34) — NOT a
/// lighter native UI; the budget-device answer is the efficient kernel (§16.51), not a 2nd UI.
pub enum CourierShell { TauriMobile, WinitDesktop }

/// SUPERSEDES P52 `ClaimCard` (P52:133) AND P52 `OFFER_DECISION_WINDOW_SECS` (P52:139).
/// The offer and its deadline now come from P65's `DispatchEvent::Offered` — the surface
/// computes no expiry of its own. `deadline_ts` is P65's hub-side `now_ts + OFFER_TIMEOUT_SECS`.
pub struct OfferCard {
    pub claim_id: u64,
    pub order_id: u64,
    pub deadline_ts: i64,        // P65 LiveOffer.deadline_ts (P65:249) — the ONLY expiry authority
    pub pickup: GeoRef,          // pickup venue, for the voice read-back + map marker
    pub dropoff_coarse: ZoneRef, // COARSE dropoff area pre-accept; precise pin unlocks on accept,
                                 // symmetric with P51's position-privacy window (P51:359-363)
    pub payout_i64: i64,         // the ORDER's delivery fee (hub-derived), NOT a courier metric;
                                 // rendered via TweenGuard money law (P52:165) — no count-up
}

/// The surface's view of P65's driver. P71 CONSUMES `DispatchEvent` (inbound; rides the P61/
/// P34/P37 wire per P65 §2) and EMITS `DispatchInput` (outbound). It mirrors the event shape,
/// it does NOT import the bebop2 dispatch crate (cross-repo boundary — memory cross-branch-todo-map;
/// same discipline P52 uses for proto-cap events).
pub enum SurfaceOfferState {
    Idle,                                 // no live offer — duty screen (K1)
    Live { card: OfferCard },             // P65 Offered — render countdown to card.deadline_ts (R2)
    Passed { stale: bool },               // P65 Advanced{TimedOut} | StaleAccept — offer gone,
                                         // rendered honestly with NO penalty/metric shown (§4.1)
    Accepted { run: ActiveRun },          // P65 Assigned — claim Offered→Claimed; P52's ActiveRun (K3)
}

/// Accept/decline are emitted ONLY from `Live` (type-level witness — an accept/decline in any
/// other state does not construct). Map to P65 DispatchInput (P65:288-289). NOT a CommitToken gate:
/// accepting a delivery moves no money (K7 cash amount is hub-derived, P52:394); CommitToken
/// (P64/P60) is money-only and unreachable from this surface (§4.1).
pub fn emit_accept(witness: &SurfaceOfferState) -> DispatchInputFrame;   // Live ⇒ Accept{me}
pub fn emit_decline(witness: &SurfaceOfferState) -> DispatchInputFrame;  // Live ⇒ Decline{me}

// ── R3 — voice-primary in motion (§16.53), over P64's VoiceSource (P64:250) ──────────────
/// The active input profile. CourierInMotion (P64:98) is set WHILE an ActiveRun is in transit;
/// it BIASES toward voice without disabling taps (P64 §3.1). Idle / AtPickup / Reviewing keep
/// Balanced (equal channels, §16.50). Fold-derived from the run/claim state — never a stored flag.
pub fn input_profile_for(state: &CourierSurfaceState) -> InputProfile;   // P64 InputProfile

/// The last stretch of the offer window where the audible+visual urgency cue rises, so a rider
/// with eyes on the road HEARS the window closing. Reuses P64's audio-channel equivalence
/// (AudioParams, P64:205) — same signal drives visual field intensity, parity-pinned (P2).
pub const OFFER_URGENCY_WINDOW_SECS: i64 = 10;  // final 10s of P65's 30s window
pub fn offer_urgency(now_ts: i64, deadline_ts: i64) -> AudioParams;      // reuse P64 §4.3 path
/// The eyes-free accept: spoken read-back of the offer (venue, dropoff area, distance, payout) +
/// an affirmation token ("accept"/"yes"); "skip"/"decline"/silence-to-deadline = the safe pole
/// (no accept). Same read-back+affirm shape as P64 voice_never_bypasses_friction (P64:435), reused
/// for coordination reliability — NOT because a CommitToken is required (it is not, §4.1).
pub const AFFIRM_TIMEOUT_SECS: i64 = 4;         // window to say "accept" after the read-back

// ── R5 — battery DoD, SOURCED from P63 SP-5 (P63:170-173) — NOT invented here ─────────────
// P63-VERDICTS.md is ABSENT this pass (SP-5 verdict = Blocked{physical budget Android}, P63:346).
// The gate is DEFINED now; its assertion is #[ignore="P63-SP5-baseline"] until SP-5 lands a real
// (non-Blocked) VerdictRecord — the P51/P52 ignored-not-deleted honesty convention.
pub const BATTERY_SHIFT_HOURS: f64            = 6.0;   // P63 SHIFT_HOURS_SIM
pub const BATTERY_SETTLED_DRAIN_MAX_PCT_HR: f64 = 4.0; // P63 SETTLED_DRAIN_BAR_PCT_PER_HR (settled, fg)
pub const BATTERY_SETTLE_SAVINGS_MIN_PCT: f64 = 30.0;  // P63 SETTLE_SAVINGS_MIN_PCT (FE-14 on vs off)
pub const BATTERY_THERMAL_SUSTAIN_MS: f64     = 33.0;  // P63 THERMAL_SUSTAIN_MS (no throttle collapse)

// ── R1 — render-correctness DoD, IMPORTED from P63 SP-6 (P63:368-374) ─────────────────────
pub const FLOOR_PARITY_DELTA_MAX: f64 = 0.02;  // P63 SP-6 PARITY_PERCEPTUAL_DELTA_MAX (WebGPU/WebGL2/CPU)
```

**Rejected alternatives (DECART one-liners, standard item 19):**
- **A P71-owned offer window (keep P52's 60 s, or pick a new value)** — rejected: P65 is now the
  single authority for the accept-timeout (one courier-independent value, the structural
  anti-scoring property `P65:107`); two windows would race and one of them would be the scoring
  back-door P65 forbids. The surface renders P65's `deadline_ts`, full stop.
- **A lighter/battery-optimized alternative courier UI** — rejected: §16.34 accepted the full-wgpu
  tradeoff knowingly; §16.51 rules the budget-device path is an efficient kernel, not a second UI.
  A fork here would be the DOM-for-forms hybrid §16.30 struck.
- **Turn-by-turn spoken navigation as part of "voice-primary"** — rejected: §16.53's voice-primary
  is *command/state* voice (offer, accept, status), not route narration — DZ-10 Phase-9b + P51
  anti-scope (`P51:154`) keep TBT out of Wave-0.
- **Gating accept on a `CommitToken`** — rejected: no money moves on accept (K7 amount is
  hub-derived); `CommitToken` is P60's money seam. Reusing P64's read-back+affirm *pattern* for
  reliability is right; requiring the *token* is category error.
- **Asserting a battery number now** — rejected: `P63-VERDICTS.md` absent ⇒ SP-5 is `Blocked`;
  writing an assumed %/h as measured violates `ground-truth-over-proxy`/`verified-by-math`. The
  gate is defined, the assertion waits (§5).
- **A per-courier "missed offers / accept rate" indicator** — rejected hard: it is exactly the
  scoring vector P65 §4.6 / P52 §4.1 red-line; the CI no-scoring gate extends over P71 (§4.1).

---

## 3. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

P71's items are named **R1-R6** (the courier-surface **R**endering & real-time layer) to keep them
distinct from P52's preserved **K1-K8**. Dependency order below; all buildable against the
now-on-paper P65/P51/P64/P63 contracts + P52's landed kernel folds.

### 3.1 Sequencing note (what R-items assume)

R-items assume P52's kernel-side folds (K1 duty, K2 inbox fold, K4 PoD, K5 statement, K6 invite,
K7 attestation) exist as consumable state — they are the substrate P71 renders. Where a P52 fold
must change to consume P65 (K2's offer source + deadline), R2 states the exact edit. Nothing in R1/
R4 waits on real GPU beyond the SP-6 harness; nothing in R5 waits on anything but SP-5's number.

### 3.2 The six build items

**R1 — Full-wgpu courier app realization (supersedes P52's O18a deferral).**
*Spec.* The `apps/courier` Tauri mobile shell (`CourierShell::TauriMobile`) hosts a full-wgpu
surface rendering P52's K1-K8 screens through P38a's pipelines (G2 particles, G3 SDF/text, G5
settle, G6 a11y-mirror) — the same paradigm as P69/P70, no DOM-visible widget (P38 zero-visible-DOM
gate applies, `P52:78-81`). The a11y-mirror is P58's (imported, not built here). Render-correctness
is the P63 SP-6 floor-parity gate at `FLOOR_PARITY_DELTA_MAX`.
- **RED:** `r1_courier_shell_renders_duty_and_run` — the duty screen (K1) and a fixture run screen
  (K3) compose real frames through P38a (not the CPU-only stub P52:626); `r1_floor_parity_courier_corpus`
  — the courier scene corpus (offer card, run screen w/ `TrackFrame`, PoD capture, earnings) passes
  `floor-parity` at ΔE ≤ 0.02 on WebGPU, WebGL2, and CPU rungs (P63 SP-6 harness, imported).
- **Adversarial:** `r1_no_adapter_degrades_to_cpu_floor` — on a courier device with no WebGPU
  adapter, the surface renders through the WebGL2→CPU floor (FE-16 ladder), never a blank/panic
  ("no GPU ⇒ no courier UI" is unreachable — P63 SP-1 adversarial (i) inherited); `r1_a11y_mirror_present`
  — every K-screen has its P58 mirror node (role/name/state), including live PoD-capture state (the
  §16.30 "cover every screen" requirement); `r1_no_visible_dom_widget` — the P38 zero-visible-DOM
  grep gate covers `apps/courier` (a DOM widget here fails an *existing* gate, no new gate needed).

**R2 — Dispatch integration: bind the K2 inbox to P65, superseding the 60 s window.**
*Spec.* Edit P52's K2 inbox fold so its offer source is P65's `DispatchEvent::Offered{courier,
deadline_ts}` (not a raw `ClaimOffered` frame with a surface-computed 60 s). `OfferCard.deadline_ts`
= P65's value; **delete `OFFER_DECISION_WINDOW_SECS`**. Accept/decline emit `DispatchInput::Accept`/
`Decline` (P65:288-289) — the claim-level `ClaimAccepted`/`ClaimReleased` vocabulary (P52 K2)
survives *beneath* P65's driver, which maps the inputs to the claim transitions. `SurfaceOfferState`
transitions: `Idle → Live` on `Offered`, `Live → Accepted` on `Assigned`, `Live → Passed{stale}` on
`Advanced{TimedOut}` / `StaleAccept`.
- **RED (event-sequence form):** `r2_offer_from_p65_renders_deadline` — a P65 `Offered{deadline_ts}`
  frame ⇒ `Live{card}` with `card.deadline_ts == offered.deadline_ts` (the surface renders P65's
  clock, computes none); `r2_accept_emits_dispatch_input` — courier accept ⇒ `DispatchInput::Accept`
  emitted, then a P65 `Assigned` frame ⇒ `Accepted{run}`; `r2_timeout_is_p65_not_surface` — with no
  courier action, the surface emits **no** expiry frame; the state advances only when P65's inbound
  `Advanced{TimedOut}` arrives (proves the surface owns no timeout).
- **Adversarial:** `r2_stale_accept_renders_passed` — accept arrives after P65 already advanced ⇒
  inbound `StaleAccept` ⇒ `Passed{stale:true}` rendered as "offer passed to another courier", exactly
  one `Assigned` ever (P65's late-accept gate, `P65:367`, consumed not re-proven);
  `r2_no_double_accept` — two rapid accepts on one `Live` ⇒ the second does not construct (the `Live`
  witness is consumed on the first `emit_accept`); `r2_island_accept_pending` — offline island: accept
  renders `pending-unconfirmed`, `Accepted` only on rejoin when P65 confirms `Assigned` (P52's
  offline-honesty test `P52:255-259`, re-run against the P65 driver — never a locally-fabricated
  claim); `r2_no_missed_offer_metric` — a `Passed` offer increments **no** counter and shows **no**
  "you missed N" text (no-scoring, §4.1).

**R3 — Voice-primary in motion + the timeout-approaching UX (§16.53).**
*Spec.* `input_profile_for(state)` returns `InputProfile::CourierInMotion` (P64:98) **only** while an
`ActiveRun` is in transit; `Balanced` when Idle / AtPickup / Reviewing (§16.50 equal channels). The
offer UX in `CourierInMotion`: on `Live`, the offer is **spoken read-back** (venue, dropoff area,
distance, payout) via P64's audio channel; the courier says "accept"/"yes" within `AFFIRM_TIMEOUT_SECS`
→ `emit_accept`, or "skip"/"decline"/silence-to-deadline → the safe pole (no accept — P65's hub
timeout then advances). **As `deadline_ts` approaches** (final `OFFER_URGENCY_WINDOW_SECS`), the field
**intensifies visually** *and* the **audio urgency cue rises** (`offer_urgency` → `AudioParams`,
tremolo/pitch climbing) — parity-pinned so eyes-on-road and eyes-on-screen get the same "window
closing" signal (P2). Run-screen actions (mark-picked-up, mark-delivered via K4, cash via K7) are
voice `Intent::Command`s in `CourierInMotion`. Voice runs **AiMode-independently** (P64:448 — a no-AI
venue's courier still has voice). Native `inference/` crate (P64:85), never WASM.
- **RED:** `r3_in_motion_sets_voice_profile` — an in-transit `ActiveRun` ⇒ `CourierInMotion`; an idle
  courier ⇒ `Balanced` (a fold-derived assertion, not a stored flag); `r3_offer_readback_then_affirm`
  — a `Live` offer under `CourierInMotion` ⇒ spoken read-back emitted, then a `VoicePhrase{"accept"}`
  (P64 `RawInput::VoicePhrase`, P64:119) ⇒ `emit_accept`; `r3_voice_decline_advances` —
  `VoicePhrase{"skip"}` ⇒ `emit_decline` (P65 advances immediately).
- **Adversarial:** `r3_urgency_cue_rises_to_deadline` — as `now_ts → deadline_ts` in the final 10 s,
  `offer_urgency().tremolo_hz` strictly increases and the visual field intensity is parity-pinned to
  it (assert the same stake drives both — no drift, P2); `r3_silence_is_safe_pole` — no affirmation by
  `deadline_ts` ⇒ **no** accept emitted; P65's inbound `Advanced{TimedOut}` drives the state (silence
  never accepts — the safe default); `r3_equal_channels_off_motion` — at pickup (not in transit), a
  tap and a voice command are equally accepted (no voice bias where safety doesn't demand it, §16.50);
  `r3_voice_survives_aimode_off` — with `AiMode::Off`, `VoiceSource` still yields intents (P64:448 —
  gating voice on AiMode would silently break no-AI venues); `r3_ambiguous_never_auto_accepts` — a
  voice phrase matching both "accept" and a nav command ⇒ `Rejected`, never an auto-accept (P64's
  deterministic-classifier rule, `P64:66-68`, inherited — the AI never resolves a consequential
  courier action).

**R4 — Map/route/ETA run screen realized (P51 consumption, rendered).**
*Spec.* The `ActiveRun` screen renders P51's `map_scene` layers (roads/route/marker) + `TrackFrame`
{est, v_mps, eta_s, remaining_m, route_version} (P51:354) as the courier consumer of P51's
"two-consumers, one implementation" `TrackFrame` (P51:355) — **consumed verbatim, P52 §3.3 already
bound it**; P71 renders it through full-wgpu (P52's `#[ignore="O18a"]` on the run screen is lifted).
`Arriving` (P51 `TrackEvent`, geo.rs `is_arriving`) unlocks K4's PoD step. The `CourierPositionUpdated`
privacy invariant (emit only assignment-accept → delivery-complete, P51:359) is honored: `track`
lives inside `ActiveRun`, so a track render outside a run is unrepresentable (P52 §4.1, kept).
- **RED:** `r4_run_renders_trackframe_glyphs` — a fixture `TrackFrame` ⇒ ETA/remaining glyphs present
  in the composed frame (pixel-region assertion, P38 oracle discipline; P52's `k3_run_renders_trackframe`
  now on the *rendered* path); `r4_arriving_unlocks_pod` — a `TrackEvent::Arriving` ⇒ the PoD capture
  step (K4) becomes reachable.
- **Adversarial:** `r4_no_track_honest_state` — P51 island/no-GPS (`TrackFrame` absent) ⇒ the run
  screen renders order + honest no-track state, never a stale marker as live (P51 `route_version`
  consumed; P52's stale-track test on the rendered path); `r4_position_render_only_in_run` — a track
  render outside an `ActiveRun` does not construct (privacy window, compile-level); `r4_no_routing_code`
  — a grep gate: no router/Kalman/map math in `apps/courier` (P51's lane — any such code is a scope
  violation, P52 §5 "map/estimator code inside P52 modules = NOT done").

**R5 — Battery gate as DoD, sourced from P63 SP-5 (conditional on SP-5 landing).**
*Spec.* Define the courier battery gate from P63's SP-5 bars (§2): a full-shift (`BATTERY_SHIFT_HOURS`
= 6 h) synthetic-shift run on `HwClass::BudgetAndroid` must show settled-screen drain ≤
`BATTERY_SETTLED_DRAIN_MAX_PCT_HR` (4 %/h), FE-14 settle-ON beating settle-OFF by ≥
`BATTERY_SETTLE_SAVINGS_MIN_PCT` (30 %), and sustained frame time ≤ `BATTERY_THERMAL_SUSTAIN_MS`
(33 ms) with no thermal collapse. **P71 does not run the rig** (P63 SP-5's, `P63:83-85`); it
consumes SP-5's `VerdictRecord`. **Because `P63-VERDICTS.md` is absent this pass**, the gate's
assertion is `#[ignore = "P63-SP5-baseline"]` until SP-5 files a real (non-`Blocked`) number — the
ignored-not-deleted convention. When SP-5 lands, the ignore is lifted and the courier's
FE-14-settled steady state (a rider watching a settled map, P63 SP-5's exact target state) is
gated against the measured baseline.
- **RED (defined-now):** `r5_battery_gate_defined_ignored` — the gate exists, imports the P63 bars,
  and is `#[ignore="P63-SP5-baseline"]` (asserting the number is owed, not faked); a review-grep
  confirms no battery %/h is written as measured anywhere in P71.
- **Adversarial (when un-ignored):** `r5_settle_saving_gate` — settle-ON vs settle-OFF delta ≥ 30 %
  (if not, FE-14's "0 rAF on a settled screen" is overstated for the courier and P71 must plan a
  lower nav-mode frame floor — P63 SP-5's own `Refines` path); `r5_no_emulator_number` — an SP-5
  verdict carrying `HwClass::Emulator` is rejected (P63 forbids it; P71 must not consume a
  meaningless number).

**R6 — M2 phase-level end-to-end falsifier (the M2 gate).**
*Spec.* Extend P52's `p52_courier_end_to_end` (`P52:492`) into `p71_m2_delivery_end_to_end` — the
**real** first-delivery-order flow that defines M2 (`SYNTHESIS §3`): from a fresh un-enrolled device,
redeem invite (K6) → duty On (K1) → a **real P65 `DispatchSession` offers this courier** (not a
fixture frame) → **accept voice-primary** under `CourierInMotion` (R3) → run screen renders P51
`TrackFrame` (R4) → PickedUp → arrive → PoD 2-of-3 settled (K4) → Delivered → cash attestation (K7) →
statement row (K5). The accept drives P65's `Assigned` → the hub order-service's `Ready→InDelivery`
(P65 M6 seam, `P65:405`). This is M2: a real delivery order dispatched, accepted, and completed
through **this exact flow**.
- **RED:** `p71_m2_delivery_end_to_end` — RED today at the first rendered step (no `apps/courier`
  shell; no P65 binding; no voice profile); GREEN = the M2 transaction is real end-to-end.
- **Not-done clauses (item 17):** the M2 test passing on a *fixture* offer instead of a live P65
  `DispatchSession` = NOT done (the dispatch binding is the whole point); a surface-owned offer
  timeout surviving anywhere = NOT done (P65 is the authority); voice unreachable while in transit =
  NOT done (§16.53); any courier score/accept-rate/missed-count field = NOT done; routing/Kalman/map
  code inside `apps/courier` = NOT done (P51's lane); the battery gate asserting an un-measured
  number as GREEN = NOT done (must be `#[ignore]` until SP-5 lands).

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11-16)

### 4.1 Hazard-safety as math (item 6) — reachability, not prose

- **A courier cannot self-assign or grab another's order** — the offer set is P65's (`DispatchSession`
  over `assign`'s HRW order); the surface can only `emit_accept` from a `Live` state it was *offered*
  (the `SurfaceOfferState::Live` witness); the hub (single-writer, `hub_ring`) resolves the rest. The
  "grabbed someone else's order" state has no representation. (Inherits P52 §4.1 + P65 §5.1.)
- **The surface owns no offer timeout** — after R2, there is no `OFFER_*_WINDOW` constant in P71; the
  only expiry authority is P65's `deadline_ts`. "Two racing timeouts" is unrepresentable (there is one).
- **No-scoring is structural** — P71 stores/renders no per-courier decline/timeout/accept metric;
  `Passed` carries `stale: bool` (an offer property) not a courier counter; `ci-no-courier-scoring.sh`'s
  file set extends over `apps/courier` + the K2-rev fold (a score/rate/count field is a CI failure,
  not a review catch — the P65 §4.6 Test C teeth, extended here).
- **A courier cannot fake delivery** — `Delivered` stays emit-gated on K4's `is_settled()` (k distinct
  hub signatures); the surface holds no signing authority (P52 §4.1, unchanged). Voice cannot bypass
  it: a voice "mark delivered" still routes through the same K4 gate (the friction-equivalent
  read-back applies, and the k-of-n signature is hub-side regardless).
- **Accepting moves no money** — no `CommitToken` is constructible or required on this surface; the
  one money-adjacent act (K7 cash) uses a hub-derived amount (`P52:394`). "A courier attests an
  arbitrary figure" is unrepresentable (unchanged from P52 K7).
- **Species-agnostic (§17.6)** — the accept P71 emits is `DispatchInput::Accept{courier}`, byte-identical
  whether a human said "accept" or an autonomous agent's controller emitted it; no protocol contract
  P71 consumes carries a "human" bit. The unsafe state "the courier protocol assumes a human" is
  absent by construction — humanity lives only in the render layer (voice/field/taps), which a machine
  courier simply replaces.
- **Privacy** — position rendering lives inside `ActiveRun` (the P51 accept→complete window); a track
  render outside a run does not construct (P52 §4.1 + P51 §4.6, kept). The pre-accept `OfferCard`
  carries only a **coarse** dropoff zone; the precise pin unlocks on accept (symmetric with the
  position-emit window).

### 4.2 Schemas for scaling (item 8)

- **Offers/courier**: ≪ 1 live offer at a time (`SurfaceOfferState` is single-valued, at-most-one —
  P52 no-batching law). Break point: none — the fold is O(1).
- **Render load**: axis = SDF segment count/viewport (map + route + card); bounded by P51's
  `map_layer/scene_2k_segments` bench (P51:478, cited not duplicated); break point = the fused-polyline
  SDF P51 names.
- **Battery**: axis = continuous-render hours × device class; the SP-5 6 h shift on budget Android is
  the stated point; break point (per SP-5's own `Refines` path) = a nav-mode power tier if the settled
  saving < 30 %.
- **Voice**: axis = phrases/min per courier (≪ a few); node-local, never gossiped (§4.4).

### 4.3 Isolation / bulkhead (item 11) + error-propagation gates (item 14)

The surface is a consumer of folds/events and an emitter of intents — a surface crash corrupts nothing
(no shared mutable state with kernel/hub; P38 §4.3 bulkhead inherited). The native `inference/` voice
crate is a separate thread/process from the wgpu render loop (P64 §5 bulkhead) — a dead `VoiceSource`
degrades to tap channels (§16.50 equal-channel resilience), it never stalls rendering. Named CI/compile
gates turning P71's bug classes into build failures: the no-scoring grep (extended over `apps/courier`);
the P38 zero-visible-DOM grep (R1); the `SurfaceOfferState::Live` witness on `emit_accept`/`emit_decline`
(no double-accept, no accept-outside-offer); the R4 no-routing-code grep (P51's lane); the R5
no-emulator-battery + no-asserted-number review-grep (P63 SP-5 discipline).

### 4.4 Mesh awareness (item 12)

Frames consumed/emitted, budgets inherited: P65 `Offered` offer-out ≤ ~48 B, accept/decline-in ≤ ~40 B
(P65 §5.3) at human cadence; P51 `CourierPositionUpdated` ≤ 32 B at ≤ 0.5 Hz (P51 §5.3); PoD signature
collection ≈ 7 KiB once per delivery (P52 §4.4). Voice, intent, and friction are **entirely node-local**
(P64 §5 — zero gossip; a `CommitToken` never crosses the wire, and none exists here anyway). Duty events
node-local (P52 §4.4). Nothing in P71 needs transport beyond what P34/P37/P61 already carry.

### 4.5 Rollback / self-healing vocabulary (item 13, used precisely)

- **Self-Termination leg (claimed):** typed refusals everywhere (accept only from `Live`; the deleted
  offer-window means no surface-side expiry to mis-fire); the no-visible-DOM / no-scoring / no-routing
  gates are hard invariant boundaries, not policy. Mechanical rollback: every module is additive
  (`apps/courier`, the K2-rev fold edit, the voice-profile wiring); deleting P71 restores P52's
  O18a-stub state (with the honest note that it re-opens M2 — the surface is un-rendered again).
- **Self-Healing leg (claimed narrowly):** offer-timeout → P65 `Advanced` → next-ranked courier is
  genuine error-correction, but it is **P65's** self-heal, consumed here, not P71's; a dropped voice
  channel degrading to taps (P64 §5) is the one P71-local redundancy claim.
- **Snapshot-Re-entry: NOT claimed** — surface state is always fold-derived; recovery = re-fold from
  the order's committed status + the current online set + logged events (the P65/P52 re-derivation).

### 4.6 Living memory (item 15) + tensor/spectral (item 16) + Linux discipline (item 9)

Item 15: the inbox/statement/run are temporal folds; completed runs demote to History (P52's demoted
tier, unchanged); `VoiceProfile`/onboarding familiarity persist in the P66 wallet (P64 §5, demote-never-
delete). Item 16: honestly N/A for new math — the surface computes nothing (HRW, dispatch, claim Law,
PoD crypto, Kalman track, route math, friction field, ASR are ALL upstream authorities); the one reuse
is P64's `AudioParams`/field parity for the urgency cue (an existing Laplacian-substrate signal, not new
math). Stated, not decorated (Anu/Ananke). Item 9 verdicts: **ALREADY-EQUIVALENT** — one dispatch
authority (P65), one track authority (P51), one voice/intent authority (P64), one battery authority
(P63) — P71 forks none; **REINFORCES** — one offer-timeout authority (P65's `deadline_ts`, superseding
P52's second one — a single-source-of-truth *repair*, not a new fork); **EXTENDS** — full-wgpu render +
voice-primary as the courier realization of the P38a paradigm; **GAP** honestly named — the SP-5 battery
*number* is owed (P63-VERDICTS absent), so the battery gate is conditional, and turn-by-turn spoken
navigation stays deferred (DZ-10 Phase-9b).

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| R1 render | K-screens are O18a CPU stubs (`P52:626`) | `r1_courier_shell_renders_duty_and_run`; `r1_floor_parity_courier_corpus` (ΔE ≤ 0.02, WebGPU/WebGL2/CPU); no-adapter degrade; a11y-mirror present; no-visible-DOM | floor-parity gate (imported from P63 SP-6) |
| R2 dispatch | K2 owns a 60 s window (`P52:139`) | offer+deadline from P65; accept/decline emit `DispatchInput`; surface emits no expiry; stale-accept→`Passed`; no double-accept; island-accept pending; **no missed-offer metric** | surface-owns-no-timeout + no-scoring rows |
| R3 voice | "NOT voice" (`P52:101`) | in-transit ⇒ `CourierInMotion`; offer read-back+affirm; voice decline advances; urgency cue rises to deadline; silence = safe pole; equal channels off-motion; voice survives AiMode::Off; ambiguous never auto-accepts | silence-is-safe-pole + urgency-parity rows |
| R4 map/run | run screen `#[ignore="O18a"]` | `TrackFrame` glyphs rendered; `Arriving` unlocks PoD; no-track honest state; position render only in run; **no routing code in apps/courier** | no-routing-code grep + privacy-window rows |
| R5 battery | no gate; §16.34 question unanswered | gate defined, bars imported from P63 SP-5, `#[ignore="P63-SP5-baseline"]`; no %/h written as measured; (un-ignored when SP-5 lands) settle-saving ≥ 30 %, settled ≤ 4 %/h, no throttle | battery-gate-defined + no-emulator-number rows |
| **R6 M2** | first rendered step RED (no shell/dispatch/voice) | `p71_m2_delivery_end_to_end`: invite→duty→**real P65 offer**→**voice accept**→**rendered run**→PickedUp→PoD 2-of-3→Delivered→cash→statement | the M2 end-to-end falsifier (ledger row) |

**Phase-level gate (the M2 falsifier, `SYNTHESIS §3`):** `p71_m2_delivery_end_to_end` GREEN = **M2 is
real** — a delivery order dispatched by P65, accepted voice-primary through this surface, and
completed through PoD to a settled statement row. **M2 gate = P71 + P65 live** on one manually-
provisioned hub (SYNTHESIS §3 build sequence).

**Not-done clauses (restated):** M2 passing on a fixture offer, not a live `DispatchSession` = NOT
done. Any surface-owned offer window surviving = NOT done. Voice unreachable in transit = NOT done. A
courier score/accept-rate/missed-count anywhere = NOT done. Routing/Kalman/map code in `apps/courier`
= NOT done. The battery gate marked GREEN on an un-measured (or emulator) number = NOT done — it must
be `#[ignore="P63-SP5-baseline"]` until SP-5's real `VerdictRecord` lands, then un-ignored and green.

**Battery-DoD honesty (the task's explicit requirement):** P63's SP-5 verdict this pass is **`Blocked`**
(`P63-VERDICTS.md` absent; SP-5 has no physical budget-Android device yet, `P63:346`). Therefore P71's
battery gate is **conditional on SP-5 landing first**: the bars and method are fixed now (from P63 §2),
the *number* is owed. When SP-5 files a real `Confirms`/`Refines` verdict, the ignore is lifted; if SP-5
files `Contradicts` (e.g. settle saves < 30 %, or a 6 h nav segment throttles), P71 applies SP-5's
one-line refinement (a nav-mode power tier) before the gate goes green. No battery number is asserted
until measured (`ground-truth-over-proxy`, `verified-by-math`).

---

## 6. Benchmark plan (item 10) — small, honest, mostly inherited

The surface computes nothing hot; budgets are inherited and cross-checked, not invented:
- **Frame budget** = P38 §6's 16.6 ms split; the run screen's map layers are P51's
  `map_layer/scene_2k_segments` bench (cited, not duplicated).
- **Offer round-trip** = P37 §6 route budgets apply to the accept/decline relays (one new series in
  the same bench, RED-commit-first); P65's own `dispatch/tick_steady_state` (< 5 µs, `P65:585`) covers
  the driver side.
- **Voice latency** = P64 §7 bench 1 (`classify` p99 < 2 ms) + bench 2 (`MoonshineAsr::feed` ~107 ms) —
  cited, P64 owns them; P71 asserts the offer read-back → affirm loop fits the 30 s window with margin
  (trivial, stated not micro-benched).
- **Battery** = **P63 SP-5's rig** (P71 declares the metric/threshold, P63 owns the harness) — the one
  measured number that is *owed* until SP-5 lands (§5).
- **Floor-parity** = P63 SP-6's `floor-parity` harness on the courier scene corpus (the one CI-runnable
  render gate, headless via the CPU reference).
Fold benches (inbox/statement at 10²-row scale) deliberately unmeasured — decorative benching refused.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 W3 (P71 row), §3 (M2 gate; build sequence) ·
`BLUEPRINT-P52-courier-working-surface.md` (**the doc P71 revises** — K1-K8 preserved §1.1;
`OFFER_DECISION_WINDOW_SECS` superseded §1.2-A; "NOT voice" superseded §1.2-B; O18a extended §1.2-C) ·
`BLUEPRINT-P65-dispatch-orchestrator.md` (**dispatch contract** — `DispatchEvent`/`DispatchInput`/
`OFFER_TIMEOUT_SECS`/`LiveOffer`/late-accept/no-scoring; consumed §3.2 R2/R6) ·
`BLUEPRINT-P51-open-map-routing.md` (**map/track contract** — `TrackFrame`/`map_scene`/`CourierTrack`/
position-privacy; consumed §3.2 R4) · `BLUEPRINT-P64-intent-engine-friction-voice.md` (**voice/intent
contract** — `VoiceSource`/`InputProfile::CourierInMotion`/`RawInput::VoicePhrase`/`AudioParams`/
AiMode-independence; consumed §3.2 R3) · `BLUEPRINT-P63-shell-platform-spike.md` (**battery baseline +
floor-parity + shell** — SP-5 bars, SP-6 gate, P39-rev mobile ruling; consumed §3.2 R1/R5) ·
`BLUEPRINT-P58-a11y-mirror-everywhere.md` (a11y mirror + Playwright harness, imported by R1) ·
`BLUEPRINT-P38-webgpu-render-engine.md` (render substrate + zero-visible-DOM gates) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.30/§16.34/§16.51 (full-wgpu courier tradeoff),
§16.53 (voice-primary-in-motion exception), §16.50 (equal channels), §17.6 (species-agnostic protocol) ·
`docs/regressions/REGRESSION-LEDGER.md` (rows named §5). Memory: `SOVEREIGN-EVENT-EXCHANGE` stance
("trust = signed capability, NEVER reputation" — §4.1 no-scoring) · `ground-truth-over-proxy-2026-07-07`
+ `verified-by-math-2026-07-07` (§5 battery-number honesty) · `performance-priority-over-minimal-change-2026-07-17`
(the full-wgpu perf/battery reconciliation) · `cross-branch-todo-map-2026-07-10` (P65 lives in
`bebop-repo`; P71 consumes its events over the wire, does not import the crate) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (§4.6 honest N/A, no decorative spectral).
**Supersedes:** P52's `OFFER_DECISION_WINDOW_SECS` (deleted, → P65's `OFFER_TIMEOUT_SECS`) and P52's
"NOT voice" anti-scope (→ §16.53 voice-primary); **extends** P52's O18a render deferral into the
realized full-wgpu app. **Feeds:** **M2** (first delivery order) with P65.

---

## 8. Hermetic principles honored (item 20 — load-bearing only)

- **P2 CORRESPONDENCE** (one concept, one authority): the supersede in §1.2-A *creates* correspondence
  where P52+P65 had two offer-timeout authorities — after P71 there is exactly one (P65's `deadline_ts`).
  One track authority (P51's `TrackFrame`, the two-consumers rule honored from the courier side), one
  voice authority (P64), one battery authority (P63). The urgency cue's audio and visual are
  parity-pinned (same signal, P64 §4.3) so they cannot drift.
- **P4 POLARITY** (safe-directed default): the offer's two poles — accept (effortful: affirmation
  token) vs decline/silence (zero-effort: the safe pole) — mean doing nothing never accepts a delivery;
  the timeout advances via P65, never via a fabricated acceptance.
- **P6 CAUSE-AND-EFFECT** (determinism as law): every §3 claim is an event-sequence falsifier (offer→
  accept→assign, timeout→advance, stale-accept→passed) over pure folds of the P65/P51/P52 event
  history — not an end-state check.
- **P7 GENDER** (paired verification, no self-certification): the surface certifies nothing about
  itself — `Delivered` is refereed by K4's k-of-n hub signatures, the offer order by P65's independent
  HRW ranking, render-correctness by P63's independent floor-parity oracle, no-scoring by the
  independent CI gate. The battery claim is refereed by P63's rig, never by P71 asserting it.

(Other principles not load-bearing here; not claimed decoratively — §4.6.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cross-doc cites; the three supersede/extend seams; P63-VERDICTS-absent finding) |
| 2 DoD | §5 (per-item + the M2 phase falsifier + the conditional battery gate) |
| 3 spec/event-driven TDD | §2 spec-first; §3 RED-first; event-sequence assertions throughout R2/R3/R4/R6 |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §3.2 (stale-accept, no-double-accept, island-accept, silence-safe-pole, ambiguous-never-auto-accepts, no-adapter degrade, no-track honest state, no-emulator-battery) |
| 6 hazard-safety as math | §4.1 (self-assign/fake-delivery/no-money/no-scoring/species-agnostic/privacy reachability args) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 (offers/render/battery/voice, each with a break point or honest none) |
| 9 Linux discipline | §4.6 (ALREADY-EQUIVALENT/REINFORCES/EXTENDS/GAP incl. the owed SP-5 number) |
| 10 benchmarks+telemetry | §6 (inherited budgets cross-checked; the one owed measured number named; decorative benches refused) |
| 11 isolation/bulkhead | §4.3 (surface + native inference bulkheads; named gates) |
| 12 mesh awareness | §4.4 (per-frame byte budgets inherited; voice/intent node-local) |
| 13 rollback/self-heal vocabulary | §4.5 (Self-Termination claimed; Self-Healing = P65's, consumed; Snapshot refused) |
| 14 error-propagation gates | §4.3 (no-scoring grep, no-visible-DOM, Live-witness, no-routing grep, no-asserted-battery) |
| 15 living memory | §4.6 (History demoted tier; VoiceProfile in wallet) |
| 16 tensor/spectral | §4.6 (honestly N/A — all math upstream; the one reuse is P64's field parity) |
| 17 regression ledger | §5 (rows named) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §0/§1 (P52 K1-K8 + P65/P51/P64/P63 contracts ALL consumed; six rejected alternatives §2) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

**Repo:** all P71 edits land in **`/root/dowiz`** (`apps/courier` shell, `engine`/`kernel` wiring). P65
is **read-only reference** in `/root/bebop-repo` — P71 consumes its `DispatchEvent`/`DispatchInput` as
wire frames, it does not edit or import the dispatch crate. Dependency order below; T-items EXTEND
P52's T1-T7 (P52's kernel folds K1/K4/K5/K6/K7 are the substrate — build/consume them first).

1. **T1 (R2 — the supersede first, it is the contract).** Edit P52's K2 inbox fold so its offer source
   is P65's `DispatchEvent::Offered{courier, deadline_ts}`; **delete `OFFER_DECISION_WINDOW_SECS`**;
   add `OfferCard`/`SurfaceOfferState` (§2); `emit_accept`/`emit_decline` → `DispatchInput` (Live
   witness only). RED-first: `r2_offer_from_p65_renders_deadline`, `r2_accept_emits_dispatch_input`,
   `r2_timeout_is_p65_not_surface`, `r2_stale_accept_renders_passed`, `r2_no_double_accept`,
   `r2_island_accept_pending`, `r2_no_missed_offer_metric`. Extend `ci-no-courier-scoring.sh`'s file set
   over the K2-rev fold + `apps/courier` (prove teeth: add a `missed_count: u32`, confirm CI RED,
   remove). Acceptance: fold tests green; gate teeth shown in the commit message.
2. **T2 (R3 — voice).** Wire `input_profile_for(state) → InputProfile::CourierInMotion` (P64) while in
   transit; the offer read-back + affirmation loop; `offer_urgency` reusing P64 `AudioParams`; run-screen
   voice `Intent::Command`s. Native `inference/` (P64:85), never WASM. RED-first: `r3_in_motion_sets_voice_profile`,
   `r3_offer_readback_then_affirm`, `r3_voice_decline_advances`, `r3_urgency_cue_rises_to_deadline`,
   `r3_silence_is_safe_pole`, `r3_equal_channels_off_motion`, `r3_voice_survives_aimode_off`,
   `r3_ambiguous_never_auto_accepts`. Acceptance: voice tests green; ASR behind `#[ignore]`-until-model
   (P64's pattern).
3. **T3 (R1 + R4 — the rendered surface).** Create `apps/courier` (Tauri mobile shell,
   `CourierShell::TauriMobile`, P63/P39-rev) hosting P38a rendering K1-K8 + the run screen with P51
   `map_scene`/`TrackFrame` (lift P52's `#[ignore="O18a"]` on the run screen). Import P58's a11y-mirror
   + P63 SP-6's `floor-parity` harness. RED-first: `r1_courier_shell_renders_duty_and_run`,
   `r1_floor_parity_courier_corpus`, `r1_no_adapter_degrades_to_cpu_floor`, `r1_a11y_mirror_present`,
   `r1_no_visible_dom_widget`, `r4_run_renders_trackframe_glyphs`, `r4_arriving_unlocks_pod`,
   `r4_no_track_honest_state`, `r4_position_render_only_in_run`, `r4_no_routing_code`. Acceptance:
   floor-parity green on the courier corpus; no map/router/Kalman code in `apps/courier` (grep).
4. **T4 (R5 — battery gate, conditional).** Define the courier battery gate importing the P63 SP-5 bars
   (§2), `#[ignore = "P63-SP5-baseline"]`. RED-first: `r5_battery_gate_defined_ignored` (gate exists,
   number owed, no %/h written as measured — review-grep). Leave the assertion ignored until
   `P63-VERDICTS.md` lands a real SP-5 `VerdictRecord`; when it does, lift the ignore and add
   `r5_settle_saving_gate` + `r5_no_emulator_number`. Acceptance: gate present and correctly ignored;
   no asserted battery number anywhere.
5. **T5 (R6 — the M2 falsifier).** Write `p71_m2_delivery_end_to_end` (§3.2 R6): invite→duty→**real P65
   `DispatchSession` offer**→**voice accept under `CourierInMotion`**→**rendered run w/ `TrackFrame`**→
   PickedUp→PoD 2-of-3→Delivered→cash→statement. Drive the accept through P65's `tick` (a real driver,
   not a fixture frame) and the `Assigned`→`Ready→InDelivery` seam (P65 M6). Add the §5 ledger rows.
   Acceptance: M2 end-to-end green on the CPU-floor render path (GPU legs behind SP-6's rungs); the not-
   done clauses (§5) all hold.

**Forbidden in this phase (for the zero-context reader):** no P71-owned offer-timeout constant (P65 is
the authority — the deleted `OFFER_DECISION_WINDOW_SECS` must not reappear); no lighter/alternative
courier UI (full-wgpu only, §16.30/§16.34/§16.51); no turn-by-turn spoken navigation (DZ-10 Phase-9b);
no score/rating/rank/accept-rate/missed-offer field ANYWHERE (CI-locked); no map/routing/Kalman code in
`apps/courier` (consume P51); no money arithmetic on the surface (K7 amount is hub-derived); no DOM-
visible widgets (P38 gates apply); no `CommitToken` gate on accept (money-only, none moves here); no
asserted/emulator battery number written as measured (P63 SP-5 owns the rig; the gate is conditional on
its real number); no editing/importing the bebop2 dispatch crate (consume its events over the wire).
