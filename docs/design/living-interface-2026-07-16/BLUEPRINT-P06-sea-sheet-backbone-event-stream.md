# BLUEPRINT-P06 — Sea & Sheet backbone + the one-field `S(t)` event stream (the J2 ordering authority)

> **Status:** execution-ready blueprint (design detail, not implementation). Arc: living-interface.
> Date: 2026-07-16. **Planning only — this document writes/edits no product code, CI, or canon.**
>
> **Scope:** roadmap **Phase 6** — the Sea/Sheet shell backbone (**DZ-01..05**) rendered against the
> Phase 3/4/5 GPU foundation, plus the **one genuinely new piece of design work this phase owns**: making
> the event stream's **kernel-validated monotonic ordering authority** concrete and consumable as `S(t)`
> (**DZ-06 + the J2 fix**). Unlike Phase 2 (pure integration, no new design), Phase 6 has exactly one
> net-new deliverable — the ordering stream — and §5 is its home.
>
> **Primary sources (already-designed, not re-litigated):** `LIVING-INTERFACE-ROADMAP.md` §4 (Phase 6
> row), §5 (**J2 — the most dangerous joint**), §7 (the J2 payload/impedance correction), §8 (the G11
> fast-path ruling); `dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md` (DZ-01..06);
> `BLUEPRINT-P01-brand-token-pipeline.md` (the canonical `resolve()` DZ-02 consumes).
> **Downstream consumers already blueprinted (their consumption is NOT this doc's job):**
> `BLUEPRINT-P07-sonification-phase0.md` (audio renderer), `BLUEPRINT-P08-living-memory-viz-phase0.md` (viz).
> **Locked prerequisites cited by ID:** Phase 3 (FE-04/05/06 GPU render + brand table), Phase 4
> (FE-07/08/09/14/16 field dynamics + money guard), Phase 5 (FE-10 Green's + FE-12 spectral).

---

## 1. Current-state evidence (what exists, cited precisely)

**The two-layer grammar and the 3-act state machine are specified, not built (DZ-01).** DZ-01 (`BLUEPRINTS-
DOWIZ-INTERFACES.md:46-67`) defines a shared `<Shell>`: (A) a field-backdrop Море drawn `z-under` (FE-04),
(B) a Sheet of SDF content (FE-05) that **rises FLUID at ζ≈0.8** with 26px top corners (`--brand-radius`),
a spectral-edge rim and a grip, and (C) an act state machine `arrive→choose→receive` = **3 URL states**
with working-back. CURRENT (звірено): two stacks coexist (legacy React `apps/web` + canonical Astro/Svelte
`web/`); the particle-cloud is the Море seed; the reference artifact holds a full three-act
`hero→sheet-rise→sea-develops` but is not generalized into a shell. No unified `<Shell>` exists yet.

**Tokens + `<Money>` now descend from the canonical `resolve()` (DZ-02 → P01).** DZ-02
(`:69-90`) fixes the 3-tier model: **T1** = the 5 owner-touched inputs; **T2 DOWIZ-FIXED** (Sea physics,
`--spectral`, `--sea-tint`/`--sea-backdrop`, the **new `--font-mono`**, `--money-ink`, `--ease-*`); **T3**
internal Cosmo-Noir. The load-bearing change since DZ-02 was written: the derivation is no longer implicit —
it is **`brand-resolve::resolve(&T1Inputs) -> ResolvedTokens`** (P01 §2), a zero-dep native+wasm crate that
emits resolved CSS literals (never `color-mix()`) + a linear-RGBA GPU table under one `token_hash`. By Phase
6 the GPU token UBO is already wired (P01 §3, roadmap Phase 3). DZ-02's Phase-6 residue is UI-facing: the
`<Money>` component (`--font-mono` tabular-nums `--money-ink`, renders the kernel integer-cent, **no tween
prop exists**), the ESLint `no-number-anim-on-price` rule, and the **4 legacy money-tween sites**
(`ClientLayout:154`/`EarningsPage`/`DashboardPage:421`/`AnalyticsPage:262`) converted to `<Money>` snap.

**The spectral edge is DOWIZ-fixed, not per-brand (DZ-03, corrected by roadmap §7/J4).** DZ-03 (`:92-111`)
specifies `--spectral` + three cinematic transitions (DIVE = ζ=1 anchored on the pressed button, ring-wipe +
refraction + chromatic; SHEET-RISE = FLUID; SEA-DEVELOPS → DZ-04) with an "attending speedup" (6s→1.4s).
**Correction to DZ-03's TARGET text:** DZ-03 says the spectral edge is "re-derived per brand"; roadmap §7
(J4) supersedes this — the `--spectral` edge is **DOWIZ-fixed T2/T3, brand-invariant** (cross-tenant
legibility + bloom-contrast control require it); only the ambient Sea it floats over carries the owner tint.
This blueprint builds the corrected T2 spectral edge.

**OrderStatus→Море already has its semantics; it lacks the continuous binding (DZ-04).** DZ-04 (`:117-138`)
CURRENT: the `order_machine` 10 states are **mirrored in a JS `channel.js`** that validates transitions in
the browser **without the server**; the particle-cloud VOCAB already maps status→{color,energy,swirl}
(order-amber → delivered-gold = the terracotta→gold semantic); `CourierTrack` fires a one-shot 24-particle
burst on mount. The gap DZ-04 closes: replace the discrete burst with a **continuous field where
color/energy/swirl = f(OrderStatus)**, amplitude growing, colour travelling terracotta→gold, TIDE ζ>1.
This is the production realization of the **"Tide over Bedrock"** prototype: the Sea (Tide) develops over the
immovable kernel-validated OrderStatus (Bedrock) — the Sea can never show a state the Bedrock has not folded.

**The Green's-function feedback vocab is specified over FE-10 (DZ-05).** DZ-05 (`:140-159`): one Green's
mechanism maps every event to a field source (tap→δ ripple ζ0.6; add-cart→ingest pulse + cart-pill money
snap; success/delivered→Gaussian HEAT bloom; error/reject→high-λ shake; loading→sustained source;
order-placed→amber burst; anomaly→agitation). Particles are tracers seeded ∝|∇U|, advected by U̇ — "2
renderers, 1 field." CURRENT: particle-cloud VOCAB covers 5 events; legacy per-component feedback
(framer `whileTap`/toast/haptic) is what DZ-05 unifies away.

**DZ-06 is where the local ordering authority lives — verified against the kernel.** DZ-06 (`:161-181`)
specifies a **local event-log (OPFS + SQLite-WASM in-browser / native SQLite off-web; pgrust spirit) +
`fold_transitions` replay on load = canonical state with no round-trip**, an outbox that drains online, and
three env-signals (`navigator.onLine`, open-hours, geolocation). The kernel authority DZ-06 wraps is **real,
tested code** (re-grepped against live HEAD):

- **`kernel/src/order_machine.rs`** — `OrderStatus` (10 states, `:7-19`); the transition table
  `allowed_next` (`:64`); **`assert_transition(from, to) -> Result<(), TransitionError>`** (`:123`) — the
  local `decide` half, returns `Err(TransitionError::Illegal(..))` (`:91`, `:131`) for a disallowed edge and
  `Err(SameStatus)`/`Err(ScaffoldDisabled)` for the others; **`fold_transitions(start, steps) ->
  Result<OrderStatus, (TransitionError, OrderStatus)>`** (`:140`) — the deterministic reducer that replays a
  transition sequence and **stops at the first illegal step** (`:148`), proven by
  `green_happy_path_pending_to_delivered` (`:704`) and `green_fold_stops_at_first_illegal` (`:737`).
- **`kernel/src/event_log.rs`** — `MeshEvent { prev:[u8;32], actor_pubkey:[u8;32], actor_seq:u64,
  payload:Vec<u8> }` (`:134`, with `actor_seq:u64` at `:140`); content-id **`event_id() =
  sha3_256(prev‖actor_pubkey‖actor_seq‖payload)`** (`:148`; `sha3_256` at `:30`); the persistence seam
  **`EventStore` trait** (`:162`) with the durable-reread **`get`** (`:169`, the G4/G5 session-boundary
  seam), the offline stand-in `MemEventStore` (`:187`), and the std-only durable `FileEventStore`
  (`crate::hydra`, `:160`); **idempotency by `AppendOutcome::Duplicate([u8;32])`** (`:227`, enum at `:221`)
  — a re-appended content-id is a **structural no-op** (proven `dup_event_is_idempotent_no_state_change`,
  `:450`); local-first `append` chains `prev` to the tip (`:257`, chaining at `:261-265`); and the
  **decide-before-commit gate `commit_after_decide`** (`:300`) which runs the `decide` closure **before**
  persisting and persists nothing on rejection (`decide_rejection_is_not_committed`, `:483`). The test
  `write_succeeds_offline_with_kernel_decide` (`:497-527`) commits `Pending→Confirmed` through
  `assert_transition` as the `decide` closure with **zero network IO** — this is the exact wiring DZ-06
  needs, already demonstrated in-kernel.

**Conclusion of §1:** DZ-01..05 are UI compositions over the Phase 3/4/5 engine — real work, but no new
*design*. DZ-06's ordering spine — `event_log` (dedup/order) under `order_machine` (validate/fold) — is
the only net-new architecture, and it is the J2 fix. §§2-4 sequence the shell; §5 designs the spine.

---

## 2. DZ-01..03 — shell / spectral-edge sequencing

The three shell items are strictly ordered because each renders into the previous one's structure.

**Step 1 — DZ-01 `<Shell>` lands first (the layer + act structure everything else needs).** Build the
two-layer grammar and the act state machine before any content or reactivity: (A) the Море field-backdrop
(FE-04, `z-under`, `--sea-tint` over `--sea-backdrop`) rendered under **every** screen and **every** role;
(B) the Sheet (FE-05 SDF, rises FLUID ζ≈0.8, 26px corners, grip); (C) the `arrive→choose→receive` act state
machine bound to **3 URL states** with working-back. Enforce the assignment rule (ambient/transition/
tracking/feedback → Море; content/word/price/decision → Sheet) and the owner split-pane variant. DZ-04's
"OrderStatus→Море" needs Act 3's tracking layer to exist first — so **DZ-01 precedes DZ-04**.

**Step 2 — DZ-02 tokens + `<Money>` into the shell (parallelizable with Step 1's content pass).** The GPU
token table is already wired (P01 Phase 3); DZ-02's Phase-6 work is the UI surface: instantiate the 3-tier
consumption inside `<Shell>`, ship the `<Money>` component against `--font-mono` + `--money-ink`, add the
ESLint rule, and convert the 4 legacy money-tween sites. This is a dependency of any Sheet that shows price
or brand chrome, so it lands **with DZ-01's first content pass**, not after DZ-03.

**Step 3 — DZ-03 spectral edge + transitions (after DZ-01's Sheet exists).** The spectral rim sits on the
Sheet and the DIVE/SHEET-RISE transitions move **between the act states DZ-01 defines**, so DZ-03 lands after
DZ-01. Build the **T2 brand-invariant** `--spectral` (roadmap §7/J4 correction — not per-brand), the
button-anchored ζ=1 DIVE (ring-wipe + refraction + chromatic; reduced-motion → scroll), the FLUID
SHEET-RISE, and the attending-speedup. SEA-DEVELOPS is deferred to DZ-04. Colour-space note (roadmap §7/J4):
the spectral/bloom palette must be sourced from `resolve()`'s **linear-RGBA** GPU table (P01 §5), or the Sea
ships a subtle brand-wide wrongness; this is a Phase-3-inherited constraint, not new work here.

---

## 3. DZ-04 — OrderStatus → Море (terracotta→gold, sea-develops): the Tide over Bedrock in production

DZ-04 is the concrete, production form of the "Tide over Bedrock" prototype. The **Bedrock** is the
kernel-validated OrderStatus — the canonical status produced by `fold_transitions` (§5). The **Tide** is the
continuous Море field whose parameters are a pure function of that Bedrock.

**Design.** In Act 3 (receive/tracking), replace `CourierTrack`'s one-shot burst with a **continuous forcing
target** `f(OrderStatus)` driving the field equation `MÜ + ΓU̇ + c²LU = S`:

| OrderStatus | field target (colour · energy · swirl) |
|---|---|
| Pending / Confirmed / Preparing / Ready | ember drift, energy 0.3, low swirl (the order maturing) |
| InDelivery / PickedUp | teal, swirl 1.6 (courier motion) |
| Delivered | gold bloom, burst 1.8 (the distinctive resolution moment) |
| Rejected / Cancelled | blood, swirl 3.4 (failure) |
| *(illegal transition)* | red recoil (see §5 — driven by a local `Err(Illegal)`) |

Amplitude **grows** and colour **travels terracotta→gold** as the status advances along the fold-validated
chain; waves turn with state under TIDE ζ>1 (FE-08). The Sheet carries step-pills + an **honest ETA range**
(never a single value or 0). The binding is **local**: the status the Tide reads is the canonical status from
DZ-06's `fold_transitions` replay, validated by `assert_transition` with **no server round-trip**. Money on
the tracking Sheet is `<Money>` snap (never a value tween). Because the Tide only ever renders a status the
Bedrock has folded, an out-of-order or illegal wire event **cannot** make the Sea show an invalid state — the
J2 coupling made visible. **DZ-04 depends on DZ-06's ordering spine (§5)** and on DZ-05's vocab (§4); build
the spine before wiring the Tide.

---

## 4. DZ-05 — the Green's-function feedback vocab (the general event→source machine)

DZ-05 is the general machine of which DZ-04 is the OrderStatus specialization. Build it as **one** event→
`FieldSource` vocab table (over FE-10 Green's-function), so every action and event is a field source and no
per-component feedback code survives:

- `tap → δ` ripple (ζ0.6, expanding front); `add-cart →` ingest pulse + cart-pill `<Money>` snap;
  `success/delivered →` Gaussian HEAT bloom; `error/reject →` high-λ shake (decays); `loading →` sustained
  source; `order-placed →` amber burst; `anomaly →` agitation.
- Particles are **tracers** seeded ∝|∇U|, advected by U̇: ripples + particles are **two renderers of one
  field** (the same "one operator" thesis the whole arc rests on). Reduced-motion → static state, still
  legible via text/pills — motion is never the sole information channel.

The single design constraint that ties DZ-05 to this phase's new deliverable: **every entry in the vocab
fires off the one ordered `S(t)` stream (§5), keyed by `t_logical`, never off raw wire-arrival order or a
component-local event handler.** DZ-05 defines the *mapping* (event → source shape); §5 defines the *schedule*
(when each source is injected). Build order: DZ-05's vocab machine is the substrate; DZ-04 is the OrderStatus
row(s) plus the Act-3 sea-develops behaviour; both draw their firing schedule from §5.

---

## 5. THE ORDERING-AUTHORITY DESIGN (DZ-06 + the J2 fix) — the phase's one net-new deliverable

This is the "one kernel-validated monotonic ordering authority" the roadmap's most dangerous joint (§5, J2)
requires, and it is the only genuinely new design in Phase 6. It is **two layers of authority feeding one
stream**, and — per the roadmap §7 correction — the two layers do **not** cover the same renderers, so this
section names exactly which layer is in scope for the G11 fast-path and which is the common substrate that
serves the (off-path) viz/audio later.

### 5.1 Layer A — the common ordering substrate (`event_log.rs`; serves BOTH renderers)

Every field-relevant event — local input or wire frame — is materialized as a **`MeshEvent { prev,
actor_pubkey, actor_seq, payload }`** (`event_log.rs:134`) and content-addressed by **`event_id() =
sha3_256(prev‖actor_pubkey‖actor_seq‖payload)`** (`:148`). This layer supplies three guarantees, all common
to the order path, the audio path, and the viz path:

1. **Idempotency (dedup, no TTL).** Append via `EventLog::append`/`commit_after_decide` (`:257`/`:300`); a
   re-received content-id returns **`AppendOutcome::Duplicate`** (`:227`) — a structural no-op
   (`dup_event_is_idempotent_no_state_change`, `:450`). This is the mechanism that makes jittered
   dupes/reorders harmless **without** a timeout window — the exact property J1/J2 need.
2. **Per-actor happens-before.** `actor_seq:u64` (`:140`) + the `prev` hash-chain (`:135`, bound to the tip
   at `:261-265`) give a monotone per-actor order.
3. **Epoch pinning (reserved for the viz).** The stream envelope carries an `epoch:u64` field. Phase 6 does
   not build the `LayoutKeyframe` path, but it **reserves `epoch` in the envelope** so Phase 8's viz can pin
   each `ActivityDelta` to a layout epoch and buffer (never mis-apply) a delta whose epoch ≠ the current
   layout — P08 §3.3 consumes exactly this field.

### 5.2 Layer B — order-path validation + total order (`order_machine.rs`; the G11-critical half)

Order events carry an encoded OrderStatus transition in `payload`. The **`decide` closure** passed to
`commit_after_decide` (`event_log.rs:300`) is exactly **`assert_transition(prev_status, next_status)`**
(`order_machine.rs:123`) — the wiring already demonstrated by `write_succeeds_offline_with_kernel_decide`
(`event_log.rs:497-527`), which commits `Pending→Confirmed` through the real Law with zero network IO.

- An **`Err(TransitionError::Illegal)`** (`order_machine.rs:91`/`:131`) means the event is **never persisted
  and never emitted downstream** — the illegal/acausal pair cannot reach any renderer. This is precisely the
  "illegal transition → red recoil, validated locally, **no server round-trip**" done-test: the recoil is
  driven by the local `Err`, not a server rejection.
- **Canonical replay:** on reload, **`fold_transitions(start, steps)`** (`:140`) replays the persisted
  transition sequence into the canonical current status with **no round-trip** — stopping at the first
  illegal step, so a corrupted tail can never fabricate a forward state.

### 5.3 The derived scheduling key `t_logical` (Phase 6 defines it)

There is **no field named `t_logical` in the kernel** (verified — `causal.rs` is Pearl do-calculus,
unrelated). `t_logical` is a **derived presentation-schedule key defined by Phase 6**:

> **`t_logical` = the index of this event in the fold-validated replay** — monotone by construction; ties
> broken by `actor_seq`, then by `event_id`.

This is the single authoritative ordering key that both downstream blueprints already name and consume:
P07's 32-byte `postMessage` record carries `t_logical` at offset 12 as "the primary ordering key"
(BLUEPRINT-P07 §3.3), and P08's `ActivityDelta.t_logical` is "the SINGLE scheduling authority" (BLUEPRINT-P08
§3.3/§4.3). Phase 6 is where it is *produced*.

### 5.4 The `S(t)` stream contract (what Phase 6 emits)

The `S(t)` forcing stream both viz and audio consume **originates here**. For each accepted, deduped,
fold-ordered event, Phase 6 emits one **`FieldSource`** record whose **ordering envelope is common** and
whose **payload is renderer-specific**:

```
FieldSource {
  t_logical: u64,      // §5.3 — the monotone schedule key (fold-replay index)
  event_id:  [u8;32],  // == MeshEvent::event_id — downstream idempotency hint (dedup for free)
  actor_seq: u64,      // causal tiebreak
  epoch:     u64,      // reserved (§5.1.3) — viz pins to layout; order/audio ignore
  payload:   <renderer-specific>   // §7/J2 correction: NOT unified across renderers
}
```

The **envelope** (`t_logical`, `event_id`, `actor_seq`, `epoch`) is the J2 fix — it is the "one ordered
stream" for **WHEN** to fire. The **payload** is deliberately **not** unified (roadmap §7/J2 correction): the
order/DZ-04 path carries the `OrderStatus` transition; the viz path carries `{signal_type, kind, energy,
node}` (`node` = index into a `LayoutKeyframe`); the audio path carries `on_event(kind, count) + FieldPos`.
**Phase 6 ships the envelope + the order-path payload only** (the DZ-04/Phase-9a need). The viz payload and
the `Signal → audio-event` adapter are **explicit Phase-8 deliverables** (roadmap §5/J2), not Phase-6 work —
this blueprint does not design them.

### 5.5 Where it lives, and how it replaces `channel.js` (DZ-06)

DZ-06 supplies the **browser-side `EventStore` implementation** (OPFS + SQLite-WASM; native SQLite off-web;
`PgEventStore`/pgrust in the node runtime) behind the kernel's `EventStore` trait (`event_log.rs:162`) —
`MemEventStore` (`:187`) stays the offline test stand-in, `FileEventStore` the egress-free durable variant;
all are the same seam, so the ordering logic is identical across them. The local render/input loop calls
`commit_after_decide` **before any network IO** (Contract §3, local-first); the outbox drains committed
events to the server asynchronously, and the **server never re-runs `decide`** — it verifies signatures only
(`event_log.rs` module doc). That is why validation and replay need no round-trip. This wiring **replaces the
legacy hand-maintained JS mirror `channel.js`** (DZ-04 CURRENT) with the canonical kernel-wasm
`assert_transition` — no drift between a JS mirror and the Rust Law; the mirror's deletion (RW-02) is
scheduled in Phase 9a, and Phase 6 stands up the wasm successor alongside it. Env-signals ride the same log:
`navigator.onLine`, the open-hours model (→ ambient Море brightness/dormancy), and the geolocation feed
(→ courier marker kinematics as field flow) are all local inputs folded into the event stream.

### 5.6 The J2 scope fence (the sharp part)

- **In scope for G11:** only **Layer B** (order_machine validate + fold) is strictly required for the G11
  fast-path — DZ-04's sea-develops and Phase 9a's order-critical UI consume the fold-validated `OrderStatus`
  stream. Phase 8's memory-viz is **off** the G11 path (roadmap §8), so its viz-specific half of J2 is **not**
  a Phase-6 gate.
- **Landed here anyway, because it is nearly free:** **Layer A** (event_id dedup, actor_seq, the reserved
  `epoch`) is the *same* code the order path already uses. Reserving the common envelope now is exactly what
  lets Phase 7 (audio) and Phase 8 (viz) plug into the identical `t_logical`-keyed stream **later without
  re-plumbing either scheduling loop** — the "cheap to design in, very expensive to retrofit" property that
  makes J2 the most dangerous joint. Building the envelope once, here, is the mitigation.
- **Explicitly NOT designed here:** Phase 7/8's consumption. P07 already specifies its 32-byte `postMessage`
  records + worklet min-heap; P08 already specifies its `ActivityDelta`/epoch-pinning + the `Signal→audio`
  adapter as its **own** deliverable. Phase 6's whole contract to them is one sentence: *here is the single
  `t_logical`-keyed, `event_id`-deduped, `fold_transitions`-validated monotonic sequence — schedule off it,
  never invent your own clock.* That contract is the deliverable.

---

## 6. Acceptance criteria (falsifiable — the three roadmap done-tests, expanded)

**Done-test 1 — three acts, Море everywhere, reduced-motion legible (DZ-01):**
1.1 The three acts (arrive/choose/receive) are **3 distinct URL states**; browser-back restores the prior
    act with its state intact.
1.2 Море renders **under every screen and every role** (backdrop present in all 3 acts).
1.3 The Sheet **rises over Море** (FLUID ζ≈0.8), it does not slide in.
1.4 `prefers-reduced-motion` → Море is a **static gradient**; every state stays legible via text/pills (no
    information carried by motion alone).
1.5 The assignment rule holds (ambient/transition/tracking/feedback→Море; content/word/price/decision→Sheet);
    the owner split-pane variant renders.

**Done-test 2 — status advance + illegal transition, validated locally (DZ-02/03/04/06):**
2.1 A legal status advance (e.g. `Preparing→Ready`) → **amplitude jump + terracotta→gold shift** on the Sea
    (RED: static field).
2.2 An illegal transition (e.g. `Pending→Ready`) → **red recoil**, produced by a **local**
    `assert_transition` `Err(Illegal)` (`order_machine.rs:123`) — a network trace shows **0 requests** for the
    validation itself (the successor to `channel.js` is the kernel wasm, not a hand-maintained JS mirror).
2.3 `<Money>` on tracking is an **integer snap, never a tween** (ESLint `no-number-anim-on-price` + the
    `money_guard` compile barrier); the 4 legacy tween sites are converted.

**Done-test 3 — reload replay reconstructs canonical state, no round-trip (DZ-06):**
3.1 Reload → **`fold_transitions` replay** over the persisted local event-log reconstructs the canonical
    current `OrderStatus` with **0 server requests** for the reconstruction.
3.2 A duplicated/reordered persisted event is a **structural no-op** (`AppendOutcome::Duplicate`,
    `event_log.rs:227`) — replay is idempotent regardless of arrival order.
3.3 Env-signals verified: offline → **pure-local render** (0 server calls); closed-hours → Море calm/dark;
    GPS → courier field flow; supplies `localStorage` reconciled into the event-log.

**Ordering-authority conformance (the new deliverable — falsifiable against the downstream contracts):**
4.1 The `S(t)` stream emits **exactly one `FieldSource` per accepted event**, keyed by `t_logical` = the
    fold-replay index (monotone; ties by `actor_seq` then `event_id`).
4.2 An illegal or duplicate event emits **no `FieldSource`** (dedup + validate happen before emit).
4.3 The ordering envelope (`t_logical`, `event_id`, `actor_seq`, `epoch`) matches the fields P07's 32-byte
    record and P08's `ActivityDelta` already consume — contract conformance, so both can schedule off it
    without inventing a clock.

---

## 7. What this unblocks

**Phase 9a (the G11 fast-path) consumes this ordering stream directly.** The order-critical product surface
— **DZ-07** (client menu/detail/checkout/**track**), **DZ-08** (courier delivery), and the
`/reliability-gate` order-lifecycle trace (ONE order `/s/:slug` → L0–L11 → delivered+feedback) — all render
against the fold-validated `OrderStatus` stream this phase produces. The reliability-gate's **exactly-once,
cross-surface-consistent** property is precisely the `event_id` idempotency (Layer A) + the `fold_transitions`
canonical replay (Layer B) established here; DZ-04's sea-develops reads the same Bedrock. Phase 9a depends on
**Phase 6 only** (transitively 3/4/5) — verified in roadmap §8; it does **not** wait on Phase 7 or Phase 8.

**Phase 7 (audio) and Phase 8 (viz) — if built on the growth-substrate track — plug into the identical
stream, off the G11 path.** Because §5 reserved the common envelope, P07's worklet and P08's viz both
schedule off the one `t_logical`-keyed sequence with no re-plumbing; the only surviving 7/8 coupling (the
`Signal→audio` adapter) is scoped to Phase 8, not here. Neither is on the critical path to the first real
order — which is exactly the sequencing the operator's commercial-delivery-first ruling (roadmap §8) requires.

---

*End BLUEPRINT-P06. Design detail only — no product code, CI config, or canon edited. Sequences DZ-01..06
over the Phase 3/4/5 engine and delivers the phase's one net-new design: the kernel-validated monotonic
`S(t)` ordering authority (`event_log`'s `event_id`/`actor_seq`/`AppendOutcome::Duplicate` dedup under
`order_machine`'s `assert_transition`/`fold_transitions`), exposed as a `t_logical`-keyed stream that Phase
9a consumes to land G11 and that Phases 7/8 later ride without re-plumbing. All kernel citations re-verified
against live HEAD (`kernel/src/order_machine.rs`, `kernel/src/event_log.rs`). Honors the roadmap §7/J2
correction: the order_machine half is the G11-in-scope layer; the event_log-only envelope is the common
substrate; the viz payload + Signal→audio adapter are Phase-8 deliverables, not designed here.*
