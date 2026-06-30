# Counsel Opinion — Voice Control (ADR-0015)

> Register: Counsel — good · beauty · wisdom. Advisory. Date: 2026-06-30.
> Reviews `docs/design/voice-control/proposal.md` + `docs/adr/0015-voice-control.md`.
> Verdict in one line: **architecturally honest, ethically pre-loaded, strategically premature,
> and quietly inverted on who it actually serves.** No grounded red line is crossed in Phase 0/1.
> One scoped, deferred ETHICAL-STOP attaches to Phase 3/4 (worker voice). Friction, not a veto.

---

## 1. Reasoning by lens (only what is load-bearing)

### Ethics / honesty / consent
The privacy architecture is the strongest part of this design and I will not pretend otherwise:
zero audio egress, ephemeral in-memory ring buffer, transcript in component state only, counters-only
telemetry, no wake-word (FORBIDDEN, not deferred). This genuinely honours "nul PII у ШІ" and
claim-check. **By construction, not by promise** — which is the right kind of safety.

Two honest corrections, both of which *help* the proposal:

- **"Audio = biometric" is an overclaim, and overclaiming is its own honesty cost.** Whisper performs
  ASR, not speaker identification. GDPR special-category (Art. 9) biometric data applies only when
  processing is *for the purpose of uniquely identifying a natural person* — command transcription is
  not that purpose. The audio is **personal data** (correct; incidental speech can carry PII) but it is
  **not Art-9 biometric processing**. This *lowers* the legal burden (no mandatory explicit-consent
  regime triggered) and should be stated accurately rather than inflated — inflated risk language is
  how a team later rationalises a heavier control that the dignity case never actually required.

- **The consent boundary is adequate but the disclosure sheet is undesigned, and that is where a
  dark-pattern could be born.** `getUserMedia` is the browser's consent for *mic access*, not the
  product's consent for *processing*. The one-time on-device sheet is fine — **provided it offers a
  real decline.** A sheet whose only button is "OK" is a soft-confirm trap (grounded red line:
  "soft-confirm-не-пастка", "honest UI"). See conditional STOP-2. Lean: ship a real persistent
  opt-in/off setting, not just a dismissable sheet — dignity, not law, is the argument.

### Aesthetics / conceptual integrity
The "engine holds zero write capability by DI boundary" invariant is *elegant* in the load-bearing
sense: the safety property is the shape of the dependency graph, not a runtime check that can rot.
"Schema rich, runtime minimal" is honoured — Phase 0/1 add zero tables, zero endpoints, zero RLS.
The `MockProvider`/`WhisperProvider` split makes the unsafe thing (a live mic) deterministically
testable. This is genuine elegance, not seductive elegance. No friction here.

One integrity caution: the `READ_ONLY` auto-apply / `STATEFUL` confirm split is clean *as a table*,
but its lived texture is where voice succeeds or annoys (see §4).

### Strategy / long horizon
The launch trigger is **the first real paid order**. Voice is flag-dark across all four phases and
explicitly does not touch the money/dispatch/RLS path — which per project memory is exactly where the
MVP is **NO-GO** (B1 inverted-money, B2 dead-dispatch, B3 RLS). So the strategic truth is plain:
**voice is a beautifully engineered detour from the thing that actually blocks launch.** That is not a
reason to reject the *design* — designing it now while it is cheap and dark is defensible — but it is a
reason to be explicit that **building Phase-0 code is opportunity cost against launch-blockers**, and to
sequence it accordingly. The thing we will regret in a year is not "we designed voice carefully"; it is
"we polished input ergonomics while the payment path was still inverted."

Second-order: a 130 MB on-device model creates a quiet platform commitment (CSP origin, supply-chain
surface, a locale-keyed second model in Phase 2). Reversible, but each phase raises the exit cost.

### User dignity / autonomy
Confirm-then-execute keeps the human as the one who acts — good. The voice layer cannot finalize money,
cannot mutate by construction, and every touch/keyboard path stays whole ("strictly additive"). This
respects autonomy: voice is offered, never imposed. Hold that line into Phase 3/4 (see STOP-1).

### Accessibility — the inversion nobody costed
This is my sharpest finding, and it is a *justice* finding, not a red line.

Voice's noblest justification is the user who struggles with text: the elderly, the low-literacy, the
diaspora speaker more fluent in spoken Albanian than in a typed UI. **But the design, as written, routes
the benefit away from exactly those people:**

1. **The capability floor hides the mic on low-end / low-RAM Android** (§2.2, §7). Cheap phones belong
   disproportionately to the poor and the elderly — the very users for whom voice would matter most.
   So voice ships to the WebGPU-equipped (younger, wealthier) and **vanishes for those who need it
   most.** An accessibility feature that is present in inverse proportion to need is closer to
   accessibility *theatre* than accessibility.

2. **The confirm gate and the disclosure sheet are text.** A user who cannot read the menu cannot read
   the "did you mean" chips or the on-device disclosure either. So the illiterate-user justification
   partly defeats itself: they can speak the command but cannot read the confirmation they must tap.

Neither point is a red line and neither kills the feature. But if accessibility is ever cited as a
*reason* for voice, that citation is currently not earned by the design. Earn it (audio confirm read-back
for low-literacy; honesty about who the capability floor excludes) or drop the accessibility framing and
call it what Phase 0/1 actually is: a convenience for capable devices.

### Epistemic — the missing evidence
The Phase-0 corpus measures **can it work** (IRA ≥ 85%, dangerous-misfire ≤ 2%). Excellent gate. But
nothing in either document measures **does anyone want it**. There is no demand signal from Albanian
food-delivery customers — only an accuracy gate. We are rigorously de-risking the engineering of a
feature whose user-pull is assumed. That asymmetry is the load-bearing unexamined assumption.

---

## 2. ETHICAL-STOPs (grounded red lines only)

**Phase 0 and Phase 1 (client storefront): ZERO grounded crossings. Clean.** The storefront design
honours every relevant red line — human-finalizes, server-authoritative, no PII to the model,
no persistence, gesture-only. I am not manufacturing friction where none is earned.

### STOP-1 (scoped + deferred-trigger) — worker/courier voice is a labour-surveillance gradient
- **Grounded line:** Ethics Charter "no AI for surveillance-for-harm" + the project's standing
  courier-dignity precedent (GPS *only during active delivery*, GPS-сміття-відкинуто,
  кур'єр-завершує). A microphone in the hands of a *worker* under a *manager's* platform is a
  structurally different power relation than a customer's own mic on their own phone.
- **Why it is real, not hypothetical:** Phase 4 (courier) is explicitly named as **"highest intrinsic
  value"** — hands-busy is genuinely where voice earns its keep. High value plus an employment power
  asymmetry is precisely the gradient down which "ephemeral, counters-only" quietly becomes
  "let's retain transcripts for dispute resolution" → "let's measure courier response latency" →
  worker monitoring. The proposal currently forbids all of this (no persistence, no transcript log).
  The STOP exists to make that prohibition a **recorded, load-bearing human decision** before the
  phase that tempts it — not buried prose in a design doc one refactor away from erosion.
- **What it does:** it does **not** block Phase 0/1/2 — those proceed. It attaches at the Phase 3/4
  boundary: before admin/courier voice ships, a human must record a decision affirming, as a guardrail
  (red→green, not convention), that courier/worker voice retains **no transcript, no audio, no
  per-worker timing telemetry**, and is never surfaced to a manager view. Friction proportional to the
  gradient; the human is final.

### STOP-2 (conditional — activates only if the disclosure ships OK-only)
- **Grounded line:** "soft-confirm-не-пастка" / honest-UI / consent-as-real-choice.
- **Trigger:** if the one-time on-device disclosure sheet ships with no genuine decline path (only an
  acknowledge button that gates access), it is a dark-patterned consent and crosses the line. As long
  as the sheet offers a real "not now / use touch" that leaves touch fully functional, **no STOP** —
  this is just a design constraint to honour. Stated so the Breaker and I can hold the build to it.

---

## 3. Non-blocking advice (aesthetic / strategic — take or leave)

- **MicFab honesty about latency.** On the WASM path a 5 s utterance can take 15–40 s. A mic that looks
  instant but isn't is a small lie. If WASM-fallback is ever shown (vs hidden), the affordance must
  signal "this will be slow" — or honour the capability floor and stay hidden. Hidden is the more honest
  default; resist the urge to "let weak devices try anyway."
- **The confirm tax can erase the value.** For add-to-cart, *speak → read chip → tap confirm* is slower
  than just tapping the dish. Voice only wins when it is faster than touch. On the storefront that
  margin is thin — which is the real reason Phase 4 (courier, truly hands-busy) is where the value
  lives. Don't over-invest the storefront UX; invest the honesty of *when the mic appears*.
- **When voice is appropriate vs irritating:** appropriate = hands-occupied, repetitive, eyes-elsewhere
  (courier carrying bags). Irritating = a seated customer browsing a menu they can already see and tap.
  Let the MicFab placement reflect that — quiet and discoverable on storefront, prominent on courier.
- **Strategic sequencing:** keep Phase-0 spike genuinely cheap (laptop probe per the doc) and do not let
  it pull engineering attention from the launch-blockers. Voice earns prod only after the first paid
  order is real.

---

## 4. Steel-man of a rejected option

**Steel-man: do NOT build voice now — text-first, ship the launch-blockers instead.** This is the
option the proposal never seriously entertains, and it is the strongest one. Albanian food delivery's
binding constraint is **not input friction** — it is trust, payment correctness, and dispatch (the
open NO-GO blockers). A customer abandons because the price was wrong or no courier came, never because
typing "gyro" was tedious. Voice adds a 130 MB download, a heavy supply-chain dep, a CSP origin, and a
new failure matrix — for a storefront benefit that the confirm-tax mostly cancels. The *cleanest* design
is the feature not built: every hour on voice is an hour not on the inverted-money path. This deserved a
real row in "alternatives considered" and got none.

**Secondary steel-man: server-side transcription (the §3.2 reject) actually serves the excluded user
better.** The proposal rejects it on privacy + infra grounds, both real. But it rejected it *for the
WebGPU-haves*. A small fine-tuned `sq` model served from one ephemeral, no-persistence, disclosed
endpoint would give **consistent accuracy on the cheap phones the capability floor hides** — i.e., it
could actually reach the elderly-poor user that the client-side design routes around (§1, accessibility).
The privacy cost is genuine and the infra cost (no GPU) is real today — so the rejection stands — but it
should be recorded that the chosen option optimises *privacy and infra* at the cost of *equity of reach*,
and that this trade was made, not discovered.

---

## 5. The open question nobody asked

**Who is voice actually for — and is the answer "the user," or "the engineer"?**

Every gate in this proposal measures whether voice *works* (IRA, dangerous-misfire, RTF, OOM). Not one
measures whether an Albanian food-delivery customer *wants to talk to a menu*. The design is a rigorous,
honest answer to a question of feasibility wrapped around an unexamined assumption of desire. Before
Phase-1 code, the question to put to a human is not "can we hit 85% on `sq`?" — the engineering will
answer that. It is: **"What is the evidence that anyone is asking for this, and if there is none, are we
building voice because users need it or because it is the most interesting thing on the board?"** A
beautiful answer to a question nobody asked is still just polish — and we have launch-blockers open.

---

## ROUND 2 — re-attack on the RESOLVE dispositions (2026-06-30)

> Read `resolution.md` + the revised `proposal.md` + ADR-0015. Per-STOP verdict below, in the
> requested words. **Phase 0/1 remains clean of any grounded red-line crossing.** One round-1 STOP
> dissolved, one correctly still-standing (deferred by design), the accessibility framing honestly
> dropped, demand correctly gated. The C2 fix opened **one new ethical surface** (recording real
> speakers) — it is **not** a new ETHICAL-STOP, but it **extends an existing NEEDS-HUMAN** with named
> conditions. Friction stays proportional.

### STOP-2 (disclosure dark-pattern) → **DISSOLVED**
This is a real dissolve, not cosmetics. Three things make it structural, not a relabel:
(1) the **default state is voice-off** ("Not now" is the no-op default) — so it is *not* OK-by-default
and *not* a hidden decline; (2) a **persistent on/off setting** in storefront prefs backs it, so the
choice survives the session rather than being a one-shot sheet you can only acknowledge; (3) a
**guardrail** asserts the decline path leaves the mic unactivated and touch working (red→green, not
prose). The cleanest part is implicit and worth making explicit: because voice is off-by-default, the
sheet appears **after** the user taps the mic — it *informs a gesture already made*, it does not *trap a
gesture not yet made*. That is the honest shape of consent.

One **non-blocking** caution (not a STOP — it cannot cross a line that is already structurally honored):
the guardrail tests *function* (mic stays off, touch works), not *affordance weight*. A decline path can
still be quietly dark if "Use voice" ships as a bright primary CTA against a greyed, small "Not now."
Keep the two choices **visually equivalent** in weight; do not let a passing functional test license a
lopsided button hierarchy. This is a design-review item, not a gate.

### STOP-1 (worker/courier surveillance) → **STILL-STANDS — correctly, as a deferred friction-gate (does NOT need to harden before Phase 0/1)**
The architect honored my round-1 scope **exactly**: constraints baked now (zero transcript/audio/
per-worker-timing, never surfaced to a manager, courier-opt-in), encoded as a Phase-3/4 guardrail, with
the *entry decision itself* recorded as NEEDS-HUMAN — not blocking Phase 0/1/2. The STOP is not
dissolved (the human decision lives at a future boundary and cannot be pre-discharged) and it is not
blocking. "Still-stands" is the accurate state: an open, recorded gate to be discharged by a human at
the Phase-3 boundary.

On the operator's specific question — *does the red line need a harder formulation before Phase 0/1 is
coded?* — **No, and here is why it is safe:** the Phase-0/1 build does **not** pre-install the
surveillance affordance. The telemetry shape is already actor-anonymous (`{intent_kind, matched,
confidence_bucket, locale}` — no user id, no worker id, no timing) and the engine holds zero write/timing
capability by construction. Adding per-worker latency telemetry in Phase 3/4 would be a **new** surface,
which is exactly where the gate sits. So the gradient is not built today and then policed later; it is
simply not built.

**One strengthening (raise the friction slightly, cheaply, now — not a STOP):** make the
**actor-anonymous telemetry schema its own guardrail from Phase 0/1**, not deferred to Phase 3/4. A test
asserting the counter record carries **no per-actor / per-worker dimension** (no `courier_id`,
`user_id`, latency field) locks the surveillance gradient *structurally at its cheapest point* — before
there is any worker mic to tempt it — rather than relying solely on a future human to refuse a
schema-growth. The earlier you forbid the column, the less anyone has to be virtuous later.

### Accessibility framing → **DISSOLVED by honest drop — and the regression the operator fears is foreclosed**
Dropping the accessibility justification and relabeling Phase 0/1 "a convenience for capable (WebGPU)
devices" is the **honest** move, and it does **not** create the "build for the rich, market as inclusion
later" trap — because the disposition installs an explicit **tripwire**: if accessibility is *ever cited
as the reason* for voice, that triggers NEEDS-HUMAN **and** the gap (TTS confirm read-back + an honest
reckoning with whom the WebGPU floor excludes) **must be closed first**. The bait-and-switch is named and
pre-refused. Good.

The honest relabel removes the *marketing* dishonesty; it does **not** remove the *distributional* fact —
the feature still routes benefit to the WebGPU-haves (younger/wealthier) and hides from the cheap-phone
elderly-poor for whom voice would matter most. That fact is now **disclosed** (R-K, R-L, the secondary
steel-man), which is the right outcome — disclosure, not denial. One thing must actually happen for the
honesty to hold: **R-K's measured WebGPU-availability rate has to be published in the Phase-0 gate
report, not merely promised.** A measurement that is owed but never printed lets the exclusion quietly
disappear. Make "who this excludes" a number on the page, not a footnote in intent. Non-blocking.

### Demand evidence → **NEEDS-HUMAN — correctly recorded, and sufficient**
R-J is exactly proportionate: it does not block the cheap Phase-0 feasibility probe, it blocks the
**expensive** Phase-1 commitment, and it forces the desire-vs-feasibility asymmetry (my §5) into a human
decision ranked against the real launch-blockers (B1/B2/B3). Nothing more is owed here. Sufficient.

### NEW surface (regression check) — the consented eval-corpus → **NO new ETHICAL-STOP; STRENGTHENED NEEDS-HUMAN on the corpus**
The C2 fix is the right architecture (two cleanly separated regimes; the production runtime keeps its
absolute zero-egress invariant). But the operator is right that it **opened a new ethical surface**:
recording real Albanian speakers — including the Gheg/diaspora accents the gate specifically wants — is
a new data-processing activity with its own dignity weight. I checked it against the grounded lines and
**it does not cross one as designed**: it is consented PII (correct), explicitly *not* Art-9 biometric
(correct), off-tenant + encrypted + no production read (correct), and **time-boxed deletion with proof**
— which is GDPR-correct for a consented research artifact and does **not** violate "anonymize-not-delete"
(that principle governs *operational* customer data, not consented research data that carries an explicit
deletion right). So: no new STOP.

What the doc does **not** yet say, and what the existing NEEDS-HUMAN must absorb, is the part the operator
named — **consent quality and vulnerable-population recruitment.** "Written consent from recruited adult
speakers" is necessary but not sufficient; consent must be *freely given*, and the wanted population
(migrant/diaspora Albanian speakers, possibly precarious) is exactly where freeness can quietly erode. I
am **extending the C2/R-I "assign a data-controller before recording" gate** (already NEEDS-HUMAN) to
carry named conditions, not just a name:
- recruitment must be **non-coercive** — in particular **not drawn from the platform's own couriers/
  workforce under any implied-benefit pressure** (that would collapse STOP-1's power-asymmetry concern
  into the corpus itself);
- **fair compensation**, an explicit **withdrawal right**, and consent scoped to the *specific* recording
  protocol and retention window;
- a documented **vulnerable-population safeguard** for the diaspora/migrant cohort.

This is **friction folded into a human moment that already exists**, not a new blocking STOP — and it
gates only **real-device human recording**, not the cheap Phase-0 laptop probe (the architect's own
voice, no recruitment) and not the Phase-0/1 engine build. Keep that distinction sharp so this gate does
not accidentally stall the cheap feasibility spike.

**One guardrail recommendation (the seam where research-tooling becomes covert customer-recording):** the
§8.1 prohibition — `VITE_VOICE_TRANSCRIBE_DEBUG` forbidden on any public deployed `/s/:slug` — is
currently **prose**. Make it a **guardrail** (like the bundle/true-dark assertion): CI fails if the debug
overlay / transcribe-capture path can be built into a public storefront image. The debug flag is the
exact mechanism by which "consented research corpus" could silently become "we recorded real customers" —
forbid it by build, not by good intentions. Non-blocking, but cheap and high-leverage.

### Round-2 bottom line
**Phase 0/1 (engine build + cheap laptop probe): clean — proceed under the gates already recorded.** No
grounded red line is crossed. The two hard human gates before money is spent or speakers are recorded are
correctly placed: (1) the **demand decision** before any Phase-1 code; (2) the **data-controller +
recruitment-ethics decision** before any real-device human recording. My only additions are *strengthen-
the-cheapest-point* moves, not vetoes: lock the actor-anonymous telemetry schema as a guardrail now, make
the debug-overlay prohibition a guardrail, publish R-K's exclusion number, and keep the decline
affordance visually equal. The design is more honest after RESOLVE than before it — which is the
direction integrity is supposed to move.

**The question nobody asked, round 2:** the corpus will record *real Albanian voices* to prove a feature
no one has yet asked for. Before we ask fifteen diaspora speakers to lend us their voices, can we say —
honestly, to them — *what this is for and that anyone wanted it*? If the demand gate is still unanswered
when we recruit, we are asking real people to fund the feasibility of our curiosity. Answer the demand
question **before** the recruitment one, not after.

---

## ROUND 3 — FINAL re-examine (2026-06-30)

> Scope: narrow confirmation, not a new sweep. I traced my four round-2 cheap-strengthening asks into
> the live `proposal.md` / `resolution.md` / `ADR-0015`, then re-ran the grounded red-line set against
> the Phase 0/1 activity envelope (engine build + cheap laptop probe, flag-dark). Verdict at the foot.

### The four asks — all wired, by citation (not promise)
1. **Actor-anonymous telemetry schema as its own guardrail from Phase 0/1 — VERIFIED IN.**
   `proposal.md:353-356` makes it enforcement item 5 ("locked from Phase 0/1"); `proposal.md:413-417`
   states it in §8; it is a listed CI-lane test (`proposal.md:611`); `ADR-0015:82-83` and the §6
   guardrail list carry it; `resolution.md:484-488` (C-1 FIX). The forbidden columns are explicit and
   complete: **no `courier_id`, no `user_id`/actor id, no latency/timing field.** The surveillance
   gradient (STOP-1) is now forbidden *at the column*, before any worker mic exists — the cheapest
   possible point. Good.
2. **Disclosure "Use voice" / "Not now" visually equal — VERIFIED IN, as an *asserted* exit criterion,
   not prose.** `proposal.md:434-438` ("same affordance weight ... not a bright primary CTA against a
   greyed, small ghost"); listed as a test at `proposal.md:608`; `resolution.md:489-492` (C-2 FIX, exit
   criterion). My round-2 worry — that a passing *functional* decline test could license a *lopsided*
   hierarchy — is closed: the weight equality is itself asserted.
3. **WebGPU-availability rate as a required gate-artifact field — VERIFIED IN.** `proposal.md:227-229`
   ("Required gate-artifact field ... a printed number, not a promise ... the gate report is incomplete
   without this line"); risk row R-K `proposal.md:641` ("REQUIRED printed field"); `resolution.md:493-495`
   (C-3 FIX). The exclusion ("who this feature routes around") is now a number the report cannot omit,
   not an intention. This is what keeps the honest accessibility *drop* from quietly becoming a hidden
   accessibility *failure*.
4. **§8.1 corpus consent conditions — all six VERIFIED IN.** `proposal.md:474-487` carries
   non-coercive recruitment, **explicitly NOT the platform's own couriers/workforce**, fair compensation,
   withdrawal right, protocol-scoped consent, and a documented vulnerable-population safeguard — and
   correctly scopes them to **real-device human recording only**, explicitly **not** the laptop probe and
   **not** the engine build (`proposal.md:486-487`). Mirrored at `ADR-0015:94-97`; `resolution.md:496-502`
   (C-4 FIX). The one seam I most cared about — that the corpus power-asymmetry could silently re-import
   STOP-1 by recruiting couriers — is named and pre-refused.

All four are *load-bearing wired*, not merely acknowledged. I have no remaining round-2 ask outstanding.

### Phase 0/1 grounded-red-line sweep (final)
I walked each grounded line against what Phase 0/1 actually *does* (build the engine package; run a
cheap local laptop probe in the architect's own voice; everything flag-dark, nothing deployed-live):
- **human-in-loop / zero-autopilot** — confirm-then-execute; READ_ONLY auto-apply is reversible +
  non-safety; STATEFUL needs a human tap. Honoured.
- **server-authoritative** — voice produces proposals only; server stays canon for price/status. Honoured.
- **zero-PII-to-AI / claim-check** — zero audio egress, on-device, ephemeral ring buffer; the model is
  generic ASR, no tenant context. Honoured. (The consented corpus is a *separate* regime, gated, and
  does **not** enter the Phase 0/1 laptop probe or engine build.)
- **soft-confirm-not-a-trap** — STOP-2 dissolved; off-by-default, real decline, persistent setting,
  visually-equal affordance asserted. Honoured.
- **cash→friction / courier-completes** — cash-settling actions (`arrived`/`completeDelivery`) excluded
  from voice *by construction*; courier is Phase 4 (separately gated). Not reachable in Phase 0/1.
- **anonymize-not-delete** — applies to operational customer data; the corpus is consented research data
  carrying an explicit deletion right (already cleared, round 2). No conflict; and the corpus is not a
  Phase 0/1 activity anyway.
- **a11y WCAG-AA** — accessibility framing honestly *dropped* (not claimed); voice is strictly additive,
  touch remains whole. No Phase 0/1 crossing; the future re-claim is tripwired.
- **"schema rich, runtime minimal" / trigger = first real paid order** — zero tables/endpoints/RLS in
  Phase 0/1; flag-dark; demand-gated before Phase-1 code. Honoured.

**Result: no grounded red line is crossed by the Phase 0/1 activity envelope, and none is left
unresolved.** The residual ethical weight does not live *inside* Phase 0/1 — it lives at two future
boundaries, and both are correctly placed as gates *before* the action that would touch them, not as
debts accrued *during* Phase 0/1.

### Status of the two NEEDS-HUMAN gates + STOP-1 (confirmed)
- **Demand-evidence gate (R-J / Counsel §5)** — **OPEN, recorded, correctly placed.** Blocks **Phase-1
  engineering**, not the Phase-0 laptop probe. A human must record either a demand signal or explicit
  speculative acceptance, ranked against B1/B2/B3, before any Phase-1 code. `proposal.md:640`,
  `ADR-0015:170-172,222-225`. Sufficient as written.
- **Corpus controller + recruitment-ethics gate (R-I / C-2[corpus]/C-4)** — **OPEN, recorded, correctly
  placed.** Gates **real-device human recording only** — not the laptop probe, not the engine build.
  Carries the named conditions (non-coercive, not-our-workforce, fair pay, withdrawal, protocol-scoped,
  vulnerable-pop safeguard). `proposal.md:474-487,639`, `ADR-0015:94-97,217`. Sufficient as written.
- **STOP-1 (worker-voice entry, Phase 3/4)** — **STILL-STANDS, deferred-trigger, non-blocking for
  0/1/2.** Constraints baked now; the entry decision is a recorded human call at the Phase-3 boundary;
  the telemetry-schema guardrail (C-1) now forbids the gradient at the column from Phase 0/1.
  `resolution.md:252-261`, `proposal.md:645` (R-O), `ADR-0015:124-126`. It does not need a harder
  formulation before Phase 0/1 is coded — the surveillance affordance is *not built*, not merely
  *policed*.

### Final note
No new ETHICAL-STOP. No new friction. The design moved in the direction integrity is supposed to move
across all three rounds: each pass made a claim *more honest* or a property *machine-checked* rather than
prose. The only thing still owed before real money/voices are spent is **two recorded human decisions** —
which is exactly where a human, not Counsel, is meant to be final.

The question nobody asked, unchanged and still first in line: **answer the demand question before the
recruitment one.** Do not ask fifteen real diaspora speakers to fund the feasibility of a curiosity no
one has yet said they want.

**PHASE 0/1 ETHICS: CLEAR**
