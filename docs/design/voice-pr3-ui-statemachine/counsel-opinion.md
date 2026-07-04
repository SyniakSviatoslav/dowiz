# Counsel Opinion — Voice PR-3: MicFab + UI State Machine (transcription review)

> Register: Counsel — good · beauty · wisdom. Advisory. Date: 2026-07-03.
> Scope: NARROW. Reviews the pure FSM reducer transcription (`voiceReducer` + injected
> `types.ts`) against the binding, already-resolved feature ethics in
> `docs/design/voice-control/counsel-opinion.md` (3 rounds, "PHASE 0/1 ETHICS: CLEAR") +
> ADR-0015 + `ui-spec.md`. I do **not** re-open money / dietary-safety / surveillance
> questions already settled there. I ask one thing: does this transcription *faithfully carry*
> those settled commitments, or does it quietly drop or invert one?
> **Verdict in one line: faithful — the safety commitments are preserved, and one is preserved
> *structurally* (the type system forbids the violation). No grounded red line is crossed. Two
> non-blocking notes: a stale honesty-claim to correct, and an aesthetic-honesty caution on the
> halo. No ETHICAL-STOP.**

---

## 1. Reasoning by lens (only what is load-bearing)

### Fidelity of the four safety commitments (the whole point of this review)

I checked each of the four commitments the brief names, against the reducer as described + the
written `types.ts` contract:

1. **No idle animation (anti-surveillance, R-E) — PRESERVED.** ui-spec §1 and the visual-language
   reference both keep idle = static button; motion begins only after an explicit tap. The reducer
   is the right place for this to *not* be a concern — a pure `(state,event)=>state` has no timers
   and no DOM, so it cannot emit an idle animation. The constraint lives in the hook/CSS, correctly.
   Nothing here weakens it.

2. **Equal-affordance confirm/cancel (anti-dark-pattern, C-2/STOP-2) — PRESERVED, and correctly
   *not this file's job*.** The reducer carries no button weight; the equality assertion is a
   computed-style CI test (ui-spec §3, VOICE-UI-REFERENCE §4). The reducer *does* honour the
   deeper C-2 intent — it moves focus to the chip *container*, not to a default-Enter "primary,"
   so a STATEFUL write cannot be an accidental Enter (ui-spec §7). That is the anti-dark-pattern
   commitment expressed as *state*, and it is intact.

3. **Fail-safe-to-no-write on cancel / timeout / Esc / outside-tap — PRESERVED, structurally.**
   `CANCEL/RESET` are idempotent no-ops from non-cancelable phases (proposal §5); `CONFIRM` fires
   `gate.confirm()` **only** from the `confirming` phase; a stray late engine callback after a
   barge-in `RESET` is a state-guarded silent no-op. Every non-decision path (timeout, Esc,
   outside-tap, engine error) resolves to *not writing*. The gate is `consumed-once`. This is
   fail-closed by the shape of the transition table, not by a runtime check that can rot. Good.

4. **"Never a message implying a safety decision was made" for dietary-dropped / rejected
   proposals — PRESERVED, and this is the strongest finding: it is enforced *by the type system*.**
   This is the one the brief most wants confirmed, so I am explicit.

   **The collapse of ALL rejections — including dietary-category-touch-only — to one neutral
   `voice.err.no_match` copy is the CORRECT reading of the binding spec, not an accident.**
   ui-spec §2 (diagram, the "dietary-named category / excluded kind → silently DROPPED →
   ERROR:no-match copy" branch) and §3 (the exclusion table + "a chip that echoes a mis-parsed
   allergen is *itself* a trusted safety assertion off a noisy channel — C1") *require* that a
   dietary/allergen match produce **the same neutral copy as a genuine low-confidence miss**, with
   no dietary-specific wording. This is deliberate **under-disclosure**, and it is the ethically
   correct direction: the *over*-disclosure ("we dropped your allergen filter — use touch") would
   be the violation, because that sentence asserts the system *understood and handled* a
   safety-critical intent off a channel it must never be trusted on. Neutral is honest; specific
   would be a false safety assertion.

   What makes the transcription genuinely elegant (beauty-as-safety): `types.ts:51` defines
   `VoiceErrorKind = 'mic_denied' | 'model_offline' | 'no_match' | 'try_again' | 'unavailable'` —
   **there is no `dietary_*` / `rejected` error kind.** A dietary REJECT therefore *cannot* be
   given its own copy; the type system forecloses the violation. The gate's `REJECT`/`rejected`
   result has nowhere to land except `NO_MATCH → voice.err.no_match`. The safety property is the
   *shape of the enum*, not a reviewer's vigilance. That is exactly the "schema rich, runtime
   minimal / safety = graph shape" register the original opinion praised, carried faithfully into
   PR-3.

### Aesthetics / conceptual integrity
The pure-reducer choice honours "schema rich, runtime minimal": the reducer *is* the statechart,
no interpreter runtime. Zero deps, zero network, zero write capability. `types.ts` is type-only
(erased at build). This is genuine elegance, not seductive elegance — consistent with the original
opinion's praise of the DI-boundary safety shape. No friction.

### Epistemic — one honesty correction (kindly)
The proposal's "Spec provenance (honesty note)" states that `PHASE1-IMPLEMENTATION-PLAN.md §3` and
`VOICE-UI-REFERENCE.md` are **"not present in this tree."** **Both files exist**
(`docs/design/voice-control/PHASE1-IMPLEMENTATION-PLAN.md`,
`docs/design/voice-control/VOICE-UI-REFERENCE.md`), and the sibling code file *cites the one it
calls absent* — `types.ts:4` grounds the injected-props decision in "PHASE1-IMPLEMENTATION-PLAN.md
§3 requires the component to take the engine/gate as INJECTED props." Almost certainly a timing
artifact (VOICE-UI-REFERENCE is dated today, 2026-07-03). Low-stakes for the reducer, but worth a
one-line correction because (a) the claim of *absence* is used to justify which spec is
authoritative, and (b) an honesty note that is itself inaccurate erodes the trust the note exists
to build. Recommend: replace the "not present" clause with "authored concurrently; ui-spec §2/§3
remains the authoritative state-machine spec the reducer transcribes." (Minor sibling nit:
`types.ts:49` and the brief both cite a "§3.6 error matrix"; the error matrix actually lives in
ui-spec **§2** — there is no §3.6. Citation hygiene, not substance; the reading is correct
regardless of the number.)

---

## 2. ETHICAL-STOP

**None.** Zero grounded red-line crossings. This is a pure presentation-state reducer: zero write
capability, zero PII, zero egress, flag-dark, non-mounted. Every grounded line the original opinion
swept CLEAR for Phase 0/1 across three rounds remains honoured, and this file introduces no new
surface that could cross one:
- **human-in-loop / zero-autopilot** — STATEFUL needs a `confirming`-phase human tap; READ_ONLY
  auto-apply is reversible + non-safety. Honoured.
- **server-authoritative** — the reducer emits proposals/UI-state only; it holds no write sink.
- **soft-confirm-not-a-trap** — focus lands on the chip container, no default-Enter primary.
- **"never imply a safety decision was made"** — enforced by the absent dietary error kind (§1.4).
- **STOP-1 (worker/courier surveillance)** — untouched; still a deferred Phase-3/4 gate. This
  reducer is storefront-only and adds no per-actor / timing dimension.

Manufacturing friction here would be exactly the over-reach the mandate warns against.

---

## 3. Non-blocking aesthetic / strategic advice (take or leave)

- **The halo's "alive" sliver lives in the *autonomous breath*, not the amplitude ring — and only
  that sliver is worth watching (Q3).** The reference doc self-corrects the strongest version of the
  surveillance-adjacency worry already: it *rejects* the orb-primary precisely because "a glowing
  orb at rest implies alive/listening — collides with no-idle," keeps the `ti-microphone` glyph,
  and starts all motion on tap. So idle stays honest. During *listening* the mic genuinely is open
  (post-tap, post-disclosure), so an animation saying "I'm capturing now" is honest, not deceptive
  — and the amplitude-reactive `::before` ring (`--amp` from real RMS) is the *most* honest possible
  cue: it moves iff real audio is entering. Beauty aligned with truth. **The one place "alive"
  creeps past "actively capturing" is the `::after` `mic-bloom` — an autonomous ~1800ms "breath"
  loop that continues *regardless of amplitude*, i.e. it keeps breathing while the user is silent.**
  A biological metaphor ("breath") that persists through silence edges back toward "an entity that
  is attentive even when I'm not speaking." It is inside `listening` so it is **not** a red-line
  issue — but if you want the surface to say exactly what the architecture is (a momentary capture
  *tool*, not a present *companion*), consider dropping the autonomous breath and letting
  amplitude-reactivity carry the whole "listening + reacting" signal: **still when silent, moving
  when spoken to.** That is the most honest listening animation, and it is *cheaper* (one rAF-driven
  property, no second keyframe loop). Purely aesthetic; not a gate.
- **Don't over-invest the storefront halo.** The original opinion's finding stands: the confirm-tax
  makes storefront voice a thin win; the value lives in Phase-4 courier (hands-busy). Polish the
  halo proportionally — it is a convenience affordance, not the product.

---

## 4. Steel-man of a rejected option (Q4)

**Steel-man xstate (proposal §3, option c — rejected as over-engineering):** the strongest case is
*governance*, and it is aimed squarely at the future the whole feature is most ethically exposed to
— STOP-1 courier voice. If voice ever reaches Phase 3/4, the machine grows hierarchical/parallel
states (a courier who is simultaneously "on a delivery" *and* "in a voice session"), more intents,
and — critically — the very surveillance-gradient constraints STOP-1 wants *encoded, not
conventional*. xstate would buy three things a hand-rolled `switch` does not: (1) a formal,
*inspectable statechart* (`@xstate/inspect`) that a non-coder — the Breaker, or a human at the
STOP-1 gate — could **audit visually** without reading TypeScript, turning a safety/surveillance
machine into a reviewable artifact; (2) first-class guard/invariant primitives that could express
"no reachable transition ever enters a transcript-persistence state" as a *statechart property*
rather than a prose promise; (3) SCXML-standard serialization → the machine becomes a spec, not just
code. For a feature whose ethics depend on invariants *holding across future phases under an
employment power asymmetry*, a formally-verifiable, human-auditable chart has a real argument the
proposal's one-line "over-engineering" dismissal does not fully weigh.

**Why the rejection nonetheless stands (honest steel-man, not a reversal):** the safety property
here does **not** live in the state machine — it lives in the `ConfirmationGate` DI boundary
(zero-write-capability by graph shape) and in the *absent dietary enum* (§1.4). xstate's guards
would not add safety the DI boundary already gives structurally. A 9-node `switch` returning plain
objects is *already* auditable in ~100 lines, and a pure `(state,event)=>state` is the single
easiest thing to port *into* xstate later *if* courier voice actually lands and the graph actually
explodes — so deferring costs almost nothing, while adopting now pays a runtime-dep tax against "no
new deps / runtime minimal" for a Phase-3/4 maybe still behind STOP-1. The rejection optimizes
correctly for *cheap-now, portable-later*. What I would record, though: the governance-auditability
argument deserved a *sentence* in the options table ("if Phase-3/4 courier voice ships, revisit
xstate for a human-auditable statechart of the surveillance-sensitive machine"), not silence — so
the trade is documented as *made*, not *unseen*.

---

## 5. The open question nobody asked

**Is the halo borrowing the visual grammar of a thing this feature has deliberately refused to be —
and does that borrowed grammar quietly promise what the architecture honestly won't deliver?**

The entire visual lineage in VOICE-UI-REFERENCE.md is the ChatGPT / Siri "AI companion" aesthetic:
a soft luminous orb/halo, engineered over years to make an AI feel *present, attentive, alive, with
you.* But dowiz's voice is, by hard construction, the *opposite* of a companion: stateless,
on-device, push-to-talk, zero memory, zero identity, zero persistence, zero egress — a dumb
ephemeral command parser that forgets you the instant the utterance resolves. The whole design has
fought, across three council rounds, for *substance-honesty* (no wake-word, no persistence, safety =
graph shape). The imported orb-lineage halo is the one place where the *surface* may be writing a
check the *architecture* has intentionally refused to cash — where the look whispers "an intelligent
presence is listening to you" while the truth is "a stateless matcher parsed six seconds of audio
and threw it away." That is not a red line; nothing is deceived into a *transaction*. But it is an
aesthetic-honesty question worth a human's eye before the halo ships: **do we want voice to *feel*
like a companion it will never be — and if the feeling and the fact diverge, which one is the
product actually selling?** The design's own integrity has been its honesty; the halo is where I'd
ask us to keep the surface as truthful as the substance.

---

**PR-3 FSM TRANSCRIPTION: FAITHFUL — no ETHICAL-STOP, no new friction. The four safety commitments
are preserved, and the dietary-collapse-to-neutral one is preserved *structurally* (no dietary error
kind exists to violate it). Two non-blocking notes: correct the stale "docs not present" honesty
claim, and consider whether the autonomous listening "breath" (and the companion-orb grammar it
borrows from) says more than this deliberately-stateless tool honestly is.**
