# Breaker findings — Voice PR-3 (MicFab + UI state machine)

> Role: System Breaker. Scope = the pure FSM reducer described in
> `docs/design/voice-pr3-ui-statemachine/proposal.md`, the binding `ui-spec.md §2/§3`, and the
> injected contract `packages/ui/src/voice/types.ts`. Grounded read-only against
> `packages/voice/src/confirmation-gate.ts` and `packages/ui/src/components/ErrorBoundary.tsx`.
> ADR-locked invariants (confirm-then-execute, dietary-touch-only, money-has-no-voice-grammar) are
> **not re-litigated** — only whether THIS design upholds them under adversarial interleaving.
> **No fixes proposed. Findings only.**

## Central thesis (what the proposal's §5 claim actually rests on)

Proposal §5 asserts: *"Stray/late engine callback after a barge-in RESET is a state-guarded silent
no-op — an event with no valid transition from the current phase returns state unchanged."*

**This claim is structurally false**, for a reason the transcription glosses over: the reducer's
phase guard only governs the reducer's *own* state field. But the write-relevant side effect —
`gate.submit(proposal)` — is called by the **hook**, on **every** `onProposal` callback, **before**
it dispatches the `PROPOSAL` event (types.ts:65 "The hook runs `gate.submit()` itself"; the hook
needs the returned `VoiceGateStatus` to pick `confirming` vs `applied`). `gate.submit()` is a direct
method call, **not** a reducer event, so it is **not phase-guarded**. The reducer being pure with
**no session/generation id** (proposal §2/§3, types.ts handlers "created once, stable identity",
:82-83) means there is **no place in the described design** where a callback can be tagged to its
originating session and rejected. "The reducer ignored the event" is conflated with "no effect
happened" — but the effect fired one call earlier. Findings 1, 2, 5 are all instances of this root.

---

## CRITICAL

**[CRITICAL] B-CONSIST / B-FAIL · Stale cross-session `onProposal` overwrites the gate's pending
STATEFUL write while the reducer's phase guard suppresses the chip re-render → the human confirms
proposal B and the cart receives proposal A (confirm-then-execute integrity broken).**

- **Grounded mechanics:** `ConfirmationGate.submit()` for a STATEFUL kind is *last-proposal-wins,
  no replay* — `this.#pending = proposal` unconditionally (`confirmation-gate.ts:70-72`).
  `confirm()` applies **whatever is pending at tap time** (`:76-84`), not what any chip rendered.
  The chip renders from the `PROPOSAL` event payload carried into reducer state (ui-spec §3: "echoes
  the *parsed* intent … {qty, item}, from the proposal"), which is a **separate channel** from
  `gate.#pending`. Nothing ties the two together with a generation id.
- **Concrete interleaving (all steps are designed-for interactions, not exotic):**
  1. Session N-1: user says "add 2× sufllaqe". Whisper inference is in flight (ui-spec worker
     timeout ~8 s → a multi-second window).
  2. User barges in (re-tap). Hook does `gate.cancel()` (pending=null), `engine.abort()`, dispatch
     `RESET` → reducer `idle`. `engine.abort()` (types.ts:84) is spec'd only as "abort any in-flight
     session"; it carries **no contractual guarantee** that an already-resolved promise / already-
     queued worker callback will not still fire the (stable) `onProposal` handler.
  3. Session N: user says "add 1× coke". `onProposal(coke)` → `gate.submit(coke)` → pending=coke →
     `PROPOSAL` → reducer `confirming`, chip = **"Add 1× Coke?"**.
  4. Session N-1's late `onProposal(2× sufllaqe)` now fires on the same stable handler → hook calls
     `gate.submit(sufllaqe)` → per `:70-72` **pending := 2× sufllaqe**. Hook dispatches
     `PROPOSAL(sufllaqe)`, but the reducer is in `confirming`, where `PROPOSAL` has no valid
     transition → **silent no-op at the reducer**. Chip **still reads "Add 1× Coke?"**.
  5. User taps **Confirm** → `gate.confirm()` applies pending = **2× sufllaqe**. The human confirmed
     Coke; the cart mutated to 2× Sufllaqe.
- **Why the reducer's purity cannot save it:** the phase guard rejects the *display* update (step 4)
  but the *write target* was already mutated (also step 4, one call earlier). The guard is on the
  wrong side of the effect. A pure `(state,event)=>state` with no generation id literally cannot
  distinguish "PROPOSAL for the session I'm in" from "PROPOSAL for a session that was aborted".
- **Invariant broken:** ADR-0015 §6 confirm-then-execute — *the thing confirmed is the thing
  written*; and proposal §6 "consumed-once / no replay" idempotency (a dead session's proposal is
  effectively replayed into the live pending slot across a session boundary).
- **Note on the "benign" branch:** if the reducer instead *does* accept `PROPOSAL` from `confirming`
  (re-rendering the chip to sufllaqe), the outcome downgrades to HIGH (display==write, but a
  wrong-utterance chip is surfaced and a fast confirmer still mis-adds). **The described design does
  not specify which branch it takes**, so it *permits* the CRITICAL branch — hence CRITICAL.

---

## HIGH

**[HIGH] B-CONSIST / B-FAIL · Stale `PERMISSION_GRANTED` from an aborted session is phase-valid in a
fresh `permission-request` phase → reducer advances to `listening` for a session whose mic grant is
not actually established.**

- **Scenario (fast double-tap):** Tap → `engine.start()` → reducer `permission-request`; native
  getUserMedia prompt / async capability probe in flight. User barges in before it resolves → hook
  `gate.cancel()` + `engine.abort()` + `RESET` → `idle`, then the second tap immediately
  `engine.start()` again → reducer back to `permission-request` (session N+1). Session N's late
  `onPermissionGranted` (a getUserMedia promise that already resolved and queued its `.then`
  microtask; abort cannot un-queue it) fires on the **same stable handler** → dispatch
  `PERMISSION_GRANTED` → reducer is in `permission-request`, for which `PERMISSION_GRANTED` **is** a
  valid transition → moves to `listening`. Session N+1's own permission may still be pending, or may
  subsequently be *denied* — the machine is now listening on a grant that belongs to a dead session.
- **Why the guard fails:** the reducer distinguishes *phases*, never *sessions*. "permission-request
  of session N" and "permission-request of session N+1" are the identical state value to a pure
  reducer with no generation id. types.ts:82-83 mandates one stable handlers instance, which is
  exactly what makes per-session discrimination impossible without an added discriminator the design
  does not have.
- **Invariant broken:** proposal §5 "state-guarded silent no-op" (the callback is neither silent nor
  a no-op — it advances the machine); B-FAIL fail-closed on permission.

**[HIGH] B-FAIL / B-OPS · The `error` phase is a candidate hard dead-end: RESET is a no-op there and
the transcription never enumerates a `TAP`-out-of-error transition — voice becomes unusable until
page reload, contradicting the error matrix's promised recovery.**

- **Grounded conflict:** proposal §5 — "CANCEL/RESET are idempotent no-ops **from non-cancelable
  phases**". The barge-in RESET set (proposal §6) is `permission/listening/transcribing/confirming/
  disambiguating` — **`error` is excluded**, so RESET-from-error is a no-op → stays in `error`.
  Meanwhile ui-spec §2 error matrix *promises* recovery: mic-denied → "tapping again re-opens the
  native prompt once"; model-offline → "Retry"; no-match → "re-prompt by tapping the FAB". That
  requires an `error --TAP--> (permission-request | idle)` reducer transition. The proposal's event
  list (§2) includes `TAP` but **only ever describes it as the idle→(disclosure|permission-request)
  trigger**; no `error`→retry transition is enumerated.
- **Failure:** if the 1:1 transcription wires `TAP` only from `idle` (the single place the diagram
  draws it), then `error + TAP` falls through to the reducer default → returns same state → **stuck
  in `error`**. `PERMISSION_GRANTED`/`PARTIAL_TRANSCRIPT` from a subsequent `engine.start()` are not
  valid from `error` either, so even re-arming the engine cannot walk the machine out. Only a reload
  clears it. Touch is unaffected, so this is voice-only — HIGH, not CRITICAL.
- **Invariant broken:** ui-spec §2 "no node dead-ends; every terminal/error offers a recovery
  affordance." The recovery button becomes a dead button.

---

## MEDIUM

**[MED] B-CONSIST · A stale cross-session `onProposal` of a READ_ONLY kind AUTO-APPLIES after
barge-in — the menu view mutates with no phase guard and no confirm at all.**

- `ConfirmationGate.submit()` applies READ_ONLY **synchronously inside submit** (`:66-68`,
  `#apply`), before any dispatch. So the same stale-callback window as the CRITICAL, but for
  `SET_SORT / SET_SEARCH / SELECT_CATEGORY / TOGGLE_COMPARE / SET_MACRO_LENS`: a dead session's late
  proposal re-sorts / re-searches / re-filters the live menu after the user has already reset and
  moved on. Reversible via the Undo toast (ui-spec §3), and dietary categories are still hard-
  dropped (`:49-61`), which caps severity — but it is a second concrete refutation of the §5 "silent
  no-op" claim: the effect is real, immediate, and unguarded by the reducer's phase.
- **Invariant broken:** proposal §5 stale-callback = silent no-op (false for READ_ONLY too).

**[MED] B-FAIL · `applied`-phase liveness depends solely on a timer the proposal itself labels
"arbitrary UX polish, not safety-relevant" — if that timer is dropped/unmounted, the machine cannot
leave `applied`.**

- Proposal §9(b): the "~900 ms applied→idle auto-return timer" is "accepted … arbitrary UX polish".
  But `APPLIED_TIMEOUT` appears to be the **only** enumerated exit from `applied`; no `TAP`-from-
  applied restart transition is described. A hook that fails to arm the timer (component remount, a
  throw between apply and `setTimeout`, or a reduced-motion/instant path that forgets to schedule it)
  leaves the FAB frozen in its check-pulse state with no event that transitions out. A *liveness*
  exit should not hang off a control the design has explicitly classified as non-safety polish.
- **Invariant broken:** B-FAIL — every phase must have a guaranteed non-cosmetic exit.

**[MED] B-SEC / B-ANTIPATTERN · The declared equal-affordance CI assertion has a perceptual blind
spot: it pins only `background / border-width / min-height / font-weight`, while the spec explicitly
permits a `ti-check` vs `ti-x` glyph — so soft weight asymmetry (and any unasserted property) passes
the gate.**

- ui-spec §3: the equality assertion covers "computed `background`, `border-width`, `min-height`,
  `font-weight`" and simultaneously blesses "A check (`ti-check`) / x (`ti-x`) glyph **may**
  differentiate them". A filled check carries more visual ink/mass than a thin x and is the
  culturally "default/good" action — a real weight cue that the four container props do **not**
  measure. Likewise `padding / box-shadow / letter-spacing / :hover / :focus color / DOM order` are
  unasserted; a parent that merges an extra `className` onto one button (spread props, a wrapper's
  `cn(...)`), or the two surfaces' different inheritance contexts (chip at `z-toast` 500 vs
  disclosure at `z-modal-backdrop` 300) can diverge on exactly those unmeasured axes while the shared
  style *constant* still guarantees only an identical class **string**, not identical **computed
  style** after cascade/merge.
- **Invariant broken:** C-2 / STOP-2 equal-affordance (a soft dark-pattern can re-enter through the
  assertion's blind spot — the exact re-introduction the spec warns against).

---

## LOW / INFO

**[LOW] B-SEC / B-DATA · `transcript: string` living in React reducer state carries no redaction
marker; current tree has no active egress path, but the "never logged/never sent" guarantee is
scoped to "this module" and does not structurally bind a later-added crash reporter.**

- Verified: no Sentry/Bugsnag/LogRocket/Datadog is wired (grep clean; only an aspirational comment
  at `apiClient.ts:93` "…grep it straight to Pino/Sentry"). `ErrorBoundary.componentDidCatch`
  forwards only `error` + React `componentStack` (`ErrorBoundary.tsx:45-47`), **not** state/props —
  so today the transcript is not egressed on a crash, and React-DevTools visibility is the user's own
  transcript on the user's own device (ui-spec §8 sanctions ephemeral in-memory). **Accepted as-is.**
  The residual: the field has no redaction/`__no_serialize` marking, so the day the contemplated
  Sentry is added with default state serialization, the zero-egress invariant (C-1/R2-C) silently
  regresses. Flagged as a latent, not a current, defect.

**[INFO] B-SCALE / B-ANTIPATTERN · Back-of-envelope (9 phases / ~17 events / 0 deps / ~0 bundle) is
accurate for the reducer, with one honesty caveat.**

- Counted: `idle · disclosure · permission-request · listening · transcribing · confirming ·
  disambiguating · applied · error` = **9** ✓. Event list = **17** ✓ (exact). Reducer imports only
  type-only `types.ts` (erased at build) → **0 runtime deps / ~0 bytes** ✓ — and the proposal
  correctly scopes this to *this file*, not the feature (PR-4's WhisperProvider/WASM model is the
  real weight, gated). No inflation. **Caveat, not a defect:** `applied` is entered from two
  semantically different paths (READ_ONLY auto-apply vs STATEFUL post-confirm) collapsed into one
  node, and Undo / go-checkout navigation / model-offline Retry are effects handled *outside* the
  reducer — so "9/17" describes the reducer's simplicity, achieved by pushing coordination into the
  unmodeled hook (PR-3b), which is precisely where Findings 1–2 and 5 live. The number is honest; it
  just is not a measure of the feature's coordination complexity.

**[INFO — near-miss, credit where due] B-CONSIST · Q1 "STATEFUL reaches the write handler WITHOUT an
explicit `confirm()`" is largely CLOSED at the gate — the residual is the CRITICAL, not a second
hole.**

- The gate is the sole classifier and sole sink: `classify(proposal.kind)` (`:45`), STATEFUL held
  pending and applied only in `confirm()` (`:76-84`), `#apply` has **no default branch** so an
  unknown `kind` falls through to nothing (`:93-119`, fail-closed), and `VoiceHandlers` (`:12-21`)
  has **no** money/checkout/dietary handler, so even a misclassified such intent has nothing to
  call. The reducer does **not** re-derive capability from the untyped `kind: string` — it consumes
  the gate's returned `VoiceGateStatus`. There is no auto-confirm timer (CONFIRM_TIMEOUT → cancel,
  fail-safe). So no path applies a STATEFUL write *without* a `confirm()` call. The only way a
  STATEFUL write is unsafe is the CRITICAL: `confirm()` **is** called (by a human) but against a
  stale, session-mismatched `#pending`. This is the sharpest real hole; the classification surface
  itself is sound.
