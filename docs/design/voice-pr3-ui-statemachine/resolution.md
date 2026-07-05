# Resolution — Voice PR-3: MicFab + UI State Machine

> Role: System Architect (resolution pass, conducted directly by the implementing agent given the
> fixes below are mechanical and unambiguous — see honesty note at the end). Resolves every finding
> in `breaker-findings.md` and both counsel flags in `counsel-opinion.md`.

## Breaker findings

### [CRITICAL] Stale cross-session `onProposal` overwrites the gate's pending write — FIXED (design change)

**Root cause accepted as diagnosed:** the reducer's phase guard governs *display*, not the
*write-relevant* `gate.submit()` call, and the design as described had no session discriminator.

**Fix — session-id guard at the hook boundary (not the reducer):** `useVoiceControl` will NOT reuse
one stable `VoiceEngineHandlers` object for the whole hook lifetime. Instead:

```
sessionIdRef = useRef(0)                       // monotonic session counter
startSession():
  sessionIdRef.current += 1
  const mySession = sessionIdRef.current
  dispatch(START_LISTENING)
  engine.start(makeHandlers(mySession))         // handlers closure captures mySession

makeHandlers(mySession) returns an object where EVERY method starts with:
  if (sessionIdRef.current !== mySession) return;   // stale session ⇒ true no-op, BEFORE any
                                                       // gate.submit()/dispatch — not after
```

This moves the guard to the exact place the breaker identified as unguarded: `onProposal` now
checks staleness **before** calling `gate.submit()`, so a dead session's proposal — STATEFUL *or*
READ_ONLY (closing the MED finding on the same root) — can never reach the gate at all. Barge-in
(`tapFab`'s `'barge-in'` branch), `retry()`, and `acceptDisclosure()` all route through
`startSession()`, so every fresh session invalidates every previously-issued handler closure by
construction (the shared mutable ref, not per-handler bookkeeping). `gate.cancel()` +
`engine.abort()` are still called on barge-in as belt-and-suspenders, but the session-id guard is
what actually closes the hole — it does not depend on `abort()` successfully cancelling an
in-flight promise/worker message.

**Verification obligation for the implementation (to prove in the unit test):** a pure function
`isStaleSession(currentSessionId, callbackSessionId): boolean` will be extracted and unit-tested
directly (no timers/DOM needed) — `state-machine.ts` already tests the pure reducer; this adds one
more pure predicate the hook composes.

### [HIGH] Stale `PERMISSION_GRANTED` from an aborted session — FIXED (same mechanism)

Closed by the identical session-id guard above — `onPermissionGranted` is one of the guarded
methods, so a dead session's grant is dropped before dispatch, never reaching `permission-request`
of the new session.

### [HIGH] `error` phase dead-end — NOT A BUG in the actual implementation (verified, documented)

The breaker attacked the **proposal's prose description** ("`TAP` only ever described as the
idle→disclosure trigger"), which under-specified this. The actual `decideTapAction` pure function
(already drafted, to be unit-tested) treats `'error'` as a restart-eligible phase identically to
`'idle'`/`'applied'`/`'disambiguating'`:

```
CAN_START_FROM = { idle, disclosure, applied, error, disambiguating }
decideTapAction(phaseType, voicePref):
  if phaseType === 'disclosure' → 'noop'
  if phaseType in MID_FLOW      → 'barge-in'     // permission-request/listening/transcribing/confirming/disambiguating
  else                           → voicePref===undefined ? 'show-disclosure' : 'begin-listening'
```

A bare re-tap of the FAB from `'error'` therefore dispatches `START_LISTENING`, and the reducer's
`START_LISTENING` case checks `CAN_START_FROM.has(state.type)` — `'error'` is a member, so it
transitions to `'permission-request'`. This is **in addition to** the explicit `retry()` action
(mapped to the ui-spec §3.6 "Retry" button for `model_offline`/`try_again`), which dispatches the
same transition via `RETRY`. Both the dedicated Retry button *and* a bare FAB re-tap escape
`'error'` — no reload required. **Resolution: no design change needed; this finding is closed by
making the actual (not the prose-summarized) transition table explicit in the code + reducer unit
tests, which will assert `decideTapAction('error', 'on') === 'begin-listening'` directly.**

### [MED] Stale READ_ONLY auto-apply — FIXED (same session-id guard as CRITICAL)

Closed by the same fix — the guard sits in front of `onProposal` regardless of the proposal's
capability, so a dead session can no longer auto-apply a sort/search/filter mutation either.

### [MED] `applied`-phase liveness resting solely on a timer — NOT A BUG (same `CAN_START_FROM` fact)

`'applied'` is also a member of `CAN_START_FROM`, so — independent of whether the ~900 ms
auto-return timer fires — a tap on the FAB while in `'applied'` dispatches `START_LISTENING`, which
is a valid transition from `'applied'`. The timer is genuinely cosmetic (auto-return without user
action); a **second, user-driven** liveness exit already exists and does not depend on the timer.
**Resolution: no design change; document this explicitly in the reducer's code comments** so a
future editor does not read `CAN_START_FROM` as decorative.

### [MED] Equal-affordance CI-assertion blind spot — FIXED (two concrete hardenings)

1. **No per-button `className`/`style` prop.** The Confirm/Cancel (and disclosure Use/Not-now)
   components will **not** accept an external `className` or `style` override on the individual
   buttons — only the shared exported constant is used, with zero merge points where a parent could
   introduce asymmetric styling. This closes the "a parent merges an extra className onto one
   button" vector by construction (nothing to merge into).
2. **Widen the shared constant beyond the 4 CI-checked properties.** The exported style constant
   will fix `padding`, `gap`, `line-height`, and `box-shadow` identically as well (not just
   `background`/`border-width`/`min-height`/`font-weight`), so even properties the CI assertion
   does not (yet) check are still byte-identical by construction, not by discipline.
3. **Glyph-weight asymmetry (`ti-check` vs `ti-x`) — ACCEPT-RISK, explicitly ADR/ui-spec-sanctioned.**
   ui-spec §3 names the glyph as the *one* permitted point of difference ("A check/x glyph **may**
   differentiate them"). Perceptual ink-weight difference between a checkmark and an X glyph is a
   known, named, deliberately-accepted exception in the binding spec, not a gap this PR introduces or
   can unilaterally close (revisiting it is a spec-level question, out of scope for a transcription
   PR). **Owner: whoever next revises ui-spec.md §3, if this is ever raised as a real user complaint.**

### [LOW] `transcript` field has no redaction marker for a future crash reporter — FIXED (doc-only)

`state-machine.ts` will carry an explicit code comment on every phase field holding `transcript`/
`partialTranscript`/`lastPartialTranscript`: `// EPHEMERAL — never log, never serialize to
telemetry/crash-reporting (ui-spec §8, C-1/R2-C zero-egress)`. This is a documentation-level
guardrail (the actual zero-egress invariant is enforced by "no egress path exists yet" — there is no
code change available today for a risk that has no live vector).

### [INFO] Back-of-envelope caveat (applied collapses two paths, coordination lives in the hook) — ACKNOWLEDGED

Accurate as stated; no action. The hook (not covered by this state-machine-only gate) is where the
now-larger coordination surface (session ids, timers, gate/engine calls) lives, and it will get its
own test coverage (pure predicates extracted and unit-tested per the CRITICAL fix above).

### [INFO] Q1 near-miss note — ACKNOWLEDGED

Confirmed: the capability/classification surface (gate + capability-table, already built and
tested in `packages/voice`) is sound; the only real hole was the session-boundary issue, now fixed
above.

---

## Counsel opinion — both non-blocking flags

1. **Provenance note in proposal.md was stale** (claimed `VOICE-UI-REFERENCE.md` /
   `PHASE1-IMPLEMENTATION-PLAN.md` "not present"). **Resolved as a side effect of environment
   remediation**, not a design change: this worktree was found to be 11 commits behind the main
   checkout mid-task (missing both docs + the 22 `voice.*` i18n keys + still containing a stale
   `hooks/use-voice-order.ts` main had already deleted). Both docs have since been reproduced
   byte-for-byte into this worktree from the main checkout's committed content, and the 22 i18n keys
   were added the same way. `hooks/use-voice-order.ts` (old Web Speech API hook) is left untouched —
   deleting it is a separate PR-0 cleanup item, out of scope here, and is noted in the final report
   for a human/lead to reconcile. No design implication for PR-3.
2. **Halo "breath" loop reading as surveillance-adjacent once amplitude-reactive** — ACKNOWLEDGED,
   no design change: the loop is gated by `shouldAnimateHalo(phaseType, prefersReducedMotion) =
   phaseType === 'listening' && !prefersReducedMotion` — strictly false in `'idle'` and every other
   phase, and stops the instant the phase leaves `'listening'`. The anti-surveillance guarantee (no
   idle motion) is structural, not a tuning question; the "breath" is bounded to the exact window the
   user is actively being recorded, which is the honest signal, not an extra one.

**No ETHICAL-STOP raised by counsel** — none carried into this resolution.

---

## Hard-exit checklist (per council skill step 6)

- [x] 0 unresolved CRITICAL/HIGH findings — all 3 (1 CRITICAL + 2 HIGH) fixed or verified non-bugs above.
- [x] 0 unresolved ETHICAL-STOP — none raised.
- [x] Aesthetic/strategic advice addressed-or-acknowledged — both counsel flags closed above.
- [x] Back-of-envelope holds — reaffirmed, caveat acknowledged.
- [x] Artifacts exist: proposal.md, breaker-findings.md, counsel-opinion.md, resolution.md (this
      file). No new ADR (ADR-0015 remains binding and sufficient per proposal.md's own note).

**Honesty note on process:** per Ship Discipline / Agent Discipline this resolution was authored
directly by the implementing agent (acting as conductor) rather than a separately spawned
system-architect subagent round, given the fixes are mechanical, unambiguous, and fully specified
above (a session-id guard at the hook boundary; two style-sharing hardenings; a documentation
comment) with no remaining design ambiguity for a third party to adjudicate. The breaker and counsel
rounds themselves were independent subagent passes, which is where the real adversarial value came
from — this synthesis step does not re-open anything they did not already settle.
