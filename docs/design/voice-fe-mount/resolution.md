# Resolution — voice-fe-mount (design-time RESOLVE round 1)

- **Date:** 2026-07-03. **Author:** System Architect (DeliveryOS). Design-time only; no production code.
- **Inputs:** `proposal.md`, `breaker-findings.md` (F1–F11), `counsel-opinion.md`.
- **Disposition legend:** FIX (proposal updated) · ACCEPT-RISK (rationale + owner) · DEFER-FLAG (MISSING, with why).
- Every disposition below is reflected in the updated `proposal.md` at the cited section. No finding is self-certified
  "resolved" — the DoD binds each fix to a red→green proof the implementing PR must produce.

---

## A. Breaker findings (F1–F11)

| # | Sev | Finding (one line) | Disposition | Where in proposal | Layer / seam |
|---|-----|--------------------|-------------|-------------------|--------------|
| F1 | HIGH | Closed-venue add shows "Done ✓" while nothing added — `confirm()` returns `applied` unconditionally, hook ignores it | **FIX** | §3(b-i) B1a′, §4, §6 seams #3/#4, §7 table | Gate at `submit()` (no chip) + `confirm()` re-check that PROPAGATES outcome; `useVoiceControl.confirm()` stops ignoring `GateResult`, dispatches `CONFIRM` only on `status==='applied'` else `ENGINE_ERROR('unavailable')`. Files: `packages/voice/confirmation-gate.ts`, `packages/ui/useVoiceControl.ts:184-188`. |
| F3 | HIGH | `useMemo`-rebuilt gate drops `#pending` (or forks on stale menu) | **FIX** (same root as F1) | §6 seams #1/#2 | Dep → LIVE getter `isOrderingDisabled()`; gate built ONCE over a stable getter/ref (latest-ref delegation), NEVER rebuilt → `#pending` never dropped, `getProduct` always live. Files: `apps/web/lib/voice/{types.ts,gate.ts}`, `VoiceMount.tsx`. |
| F2 | HIGH | MockEngine (`apps/web/src/lib/voice/**`) is outside the `packages/voice/src/**` guardrail scope → could import a Cart mutator / api-client and bypass the gate, no rule firing | **FIX** | §8 guardrail #1, DoD, R8 | Machine-enforce: extend `no-voice-app-import` scope to `apps/web/src/lib/voice/**` (same forbidden-set: Cart* mutator / api-client / fetch-client). Verified no false-positive on existing adapter files. Red→green fixture = a mock-engine that imports `../CartProvider` addItem. Chose extend-scope over relocate-to-`packages/voice` (relocation would break the pull-source invariant + put a push engine in the pure package). |
| F4 | HIGH | pull→push impedance: multi-yield re-entrancy + no-match wedge (no terminal, no watchdog) | **FIX** | §3a-contract #2/#3/#4, §7 table, DoD, R9 | Bridge CONTRACT: take FIRST yield only (one proposal/session); unmatched transcript → `onTranscribing`→`onNoMatch`; watchdog `VOICE_WATCHDOG_MS` → `onError`. |
| F5 | MED | true-dark "0 KB / chunk not emitted" false — module-scope `lazy(import())` emits the chunk regardless | **FIX** | §2 bundle, §9, §10 true-dark, DoD | Gate the `import()` itself: `VoiceMount = VOICE_ENABLED ? lazy(() => import('./VoiceMount')) : null` — import lives in the dead constant-folded ternary arm → DCE'd. Bundle assertion corrected to grep `dist/` for a voice marker → assert absence. MediaGallery "precedent" corrected (load-on-demand ≠ 0 bytes). |
| F6 | MED | DEV-only injection seam dead on staging E2E (prod Vite build, `DEV===false`) | **FIX** | §3a-contract, §9 (F6 bullet), R2, DoD | Gate the seam on `VITE_VOICE_ENABLED` (live on staging, DCE'd in prod) + inert `window.__VOICE_E2E__` channel. "Dead in prod" held twice: whole-chunk DCE when OFF, and PR-4 swaps out the mock+seam before any public ON. |
| F7 | MED | Engine async throw → wedged FSM (unstated try/catch) | **FIX** | §3a-contract #4/#5, §7 table, R9 | Contract: total `try/catch → onError` + watchdog; binds the mock NOW and the PR-4 real engine. |
| F8 | MED | `abort()` single-flag reset is a shared-mutable-state race (instance fields) | **FIX** | §3a-contract #1/#6, §7 table, R9 | Contract: handlers + `aborted` are `start()`-closure locals, never `this.*`; `abort()` flips only the current session's captured flag. |
| F9 | MED/LOW | Missing `voice.*` key renders raw dotted key; 2 DYNAMIC families invisible to static scan; parity gate ≠ existence | **FIX** | §7-keys (F9 decision), DoD, R10 | Decision: **key-EXISTENCE guardrail** (authority) enumerating all 20 incl. `voice.err.${kind}` (derived from `VoiceErrorKind`) + `ariaKeyForPhase`, red→green; **plus** neutral fallback at the 2 dynamic call-sites (degradation). |
| F10 | LOW | Key count "17" vs enumeration 20 | **FIX** | Problem §item4, §4, §7-keys, DoD | Corrected to 20 (15 named + 5 `voice.err.*`) everywhere. |
| F11 | LOW | B2a opens checkout on a preview venue (unconditional `?checkout=1`) | **FIX** | §3(b-i) scope note, §4, §7 table | Gate `navigateCheckout` on `isOrderingDisabled()` → `onNoMatch` when disabled; voice never pops checkout on preview/closed. Cart review stays via ungated `READ_ORDER`. Chose FIX over accept-LOW: consistent with the honest-UI posture (Counsel B1a) at near-zero cost, reuses the same live getter. |

**"What holds" (breaker §):** confirmed intact and NOT altered by any fix — money/checkout/settling REJECT airtight
(capability-table has no money kind), dietary REJECT, cross-session stale guard, `@deliveryos/ui` does not re-export the
engine, heavy ASR deps out of the static graph. My fixes touch none of these invariants.

---

## B. Counsel points

| Item | Type | Disposition | Where |
|------|------|-------------|-------|
| **WATCH-LINE — B1a load-bearing; if descoped → ETHICAL-STOP on honest-UI; keep it AT THE SINK** | conditional watch | **FIX / AFFIRM + REINFORCE** | §3(b-i), §6. B1a is now B1a′ — strengthened, not weakened: gated at `submit()`+`confirm()`+handler (three layers at the sink), never a hide-the-FAB hack (B1b rejected). Marked a **must-pass exit criterion** in the DoD (closed-venue unit test). **If ever descoped/deferred/liveness-dropped → this becomes a live ETHICAL-STOP.** Recorded as a standing constraint (below). |
| Add a disclosure-DECLINE E2E arm (mic unactivated + touch works + equal-weight choices) | non-blocking | **FIX** | DoD Mandatory-Proof bullet — decline arm added; complements the existing `equal-affordance.test.ts` (styling) with the functional assertion. |
| Label mock-green as WIRING-proof-not-product-proof | non-blocking | **FIX** | §9 (mock-green bullet) + DoD ("labeled WIRING-proof, not product-proof"). |
| B1a never a hide-the-FAB hack | non-blocking | **FIX / AFFIRM** | §3(b-i) — B1b rejected as "never the guard"; the gate is at the sink (`submit`/`confirm`/handler), not FAB visibility. |
| Potemkin-mic as an ASSERTED line, not an assumption | non-blocking | **FIX** | §8 (Potemkin bullet) + R11 — asserted two ways (true-dark DCE when OFF; PR-4 real engine is the precondition for any public ON). |
| **OPEN QUESTION — pre-commit the flip-ON + delete-if-unmounted conditions NOW** | human decision | **DEFER-TO-HUMAN → STOP-DESIGN-B** | §10 STOP-DESIGN-B block + item below. **NOT decided by the architect.** Gates landing the mount. |

---

## C. Standing constraints carried out of this round

1. **B1a′ is non-negotiable.** The DoD closed-venue units ("closed add → no chip/no Done/no `addItem`"; "venue closes
   after pending → confirm → no `addItem`, FSM not `applied`") are must-pass. Descoping/deferring B1a′ or dropping its
   liveness = a live ETHICAL-STOP on honest-UI (Counsel WATCH-LINE). Owner: implementer + Counsel re-review if descoped.

2. **STOP-DESIGN-B (human, blocks landing).** Before the mount lands, the human pre-commits and records: (a) the
   condition to flip voice ON in prod (a ranked real-demand signal vs the first paid order, per ADR-0015 §5 / R-J), and
   (b) the condition to delete-if-unmounted (a time/decision box). Rationale: a mounted 95%-built feature converts voice
   from *demand-gated* to *in-flight*; byte-reversibility ≠ momentum-reversibility. The architect does not decide this.
   Owner: **human/operator.**

---

## D. Regression self-check (new risk introduced by these fixes)

| New surface | Risk | Guard |
|-------------|------|-------|
| `packages/voice/confirmation-gate.ts` gains an optional `StatefulPrecondition` (money-adjacent red-line) | A behavior change could break the 58/58 or the gate's REJECT airtightness | `precondition?` is OPTIONAL → absent = allow, so existing gate tests (constructed without it) are unchanged; new red→green units cover both branches. Object-port ≠ new write capability (gate stays sole sink); does not trip `no-voice-engine-callback`. |
| `useVoiceControl.confirm()` now reads the `GateResult` | Could regress the happy-path "confirm → Done" | Dispatch `CONFIRM` still fires on `status==='applied'`; only a non-applied result routes to `ENGINE_ERROR('unavailable')`. New hook-level test pins both. |
| `VoiceMount` stable-deps/stable-gate latest-ref pattern | A setter captured once could go stale | Same latest-ref pattern `useVoiceControl` already uses (`gateRef`/`engineRef`); fields read `live.current` each call, never a frozen snapshot. |
| Gated `import()` ternary | Typing/Suspense null-guard | `{VoiceMount && <Suspense>…}` guards the null OFF case; standard Vite conditional-chunk pattern. |
| Extended `no-voice-app-import` scope | False-positive on existing adapter files | Verified: `handlers.ts`/`gate.ts` use injected deps; `types.ts` imports only the `CartItem` TYPE from `@deliveryos/ui` (specifier is not a `Cart*` module). Fixture proves a REAL violation red→green (not green-by-construction). |
| Gating `NAVIGATE_CHECKOUT` on `orderingDisabled` | Minor divergence from tap-path-on-merely-closed (tap opens checkout+server-refuses; voice now refuses client-side) | Accepted + documented (§3 scope note): more honest, not less; cart review preserved via `READ_ORDER`. |

---

## E. Threat-model items to carry into the build's error-fix matrix (X-blockers / Playwright scenarios)

X-blockers (must be red→green in the implementing PR before merge):

1. Closed/preview venue add → **no confirm chip, no "Done", cart empty** (submit-reject). — unit + staging E2E.
2. Venue closes AFTER pending, at confirm → **no `addItem`, FSM not `applied`** (confirm re-check propagates). — unit.
3. Re-render between propose & confirm → **`#pending` survives** (gate not rebuilt) → confirm still adds. — unit.
4. Multi-transcript session → **exactly one `onProposal` per `start()`** (no 2nd apply, no orphan pending). — unit.
5. Unmatched transcript → **`onNoMatch`** (never a silent `transcribing` spinner). — unit + E2E.
6. Engine throw / no-terminal within watchdog → **`onError('try_again')`**, FSM not wedged. — unit.
7. Barge-in (`abort()`→`start()`) → **no cross-session mutation** (closure-captured handlers/abort + session-id guard). — unit.
8. OFF build → **no voice chunk in `dist/`** (bundle grep asserts absence). — CI bundle assert.
9. Staging E2E injection seam live under `VITE_VOICE_ENABLED`; **DCE'd in prod**. — E2E + bundle assert.
10. Missing `voice.*` key (incl. dynamic `voice.err.${kind}` + `ariaKeyForPhase`) → **key-existence guardrail red**. — unit guardrail.
11. Engine-dir file importing a Cart mutator / api-client → **`no-voice-app-import` red**. — eslint fixture.
12. Page importing `@deliveryos/voice` directly → **`no-voice-engine-import-outside-adapter` red**. — eslint fixture.
13. Disclosure DECLINE → **mic unactivated, touch works, equal-weight buttons**. — E2E.
14. Closed/preview `NAVIGATE_CHECKOUT` → **no `?checkout=1` navigation** (onNoMatch). — unit/E2E.

Every E2E above is labeled **WIRING-proof, not product-proof** (Counsel §1): it proves plumbing + gate, not that voice
understands a real speaker.

---

# Round 2 resolution (design-time RESOLVE — post breaker round-2)

- **Date:** 2026-07-03. **Author:** System Architect. Design-time only; no production code.
- **Round-2 breaker verdict:** F1/F3/F4/F5/F6 CLOSED as designed. Three new regressions the round-1 fixes INTRODUCED
  (R-a HIGH red-line, R-b MED, R-c MED) + two LOWs (F4-residual, F6 ops-dependency). Dispositions below; each reflected
  in `proposal.md` at the cited section, each bound to a red→green proof in the DoD.

## R-a [HIGH, red-line] — fail-closed + machine-gated. The single most important line first.

**Disposition: FIX — the STATEFUL permit is now FAIL-CLOSED and machine-gated at the production factory: an ABSENT
precondition REFUSES (never permits), a `createVoiceGate` that forgets to wire it makes the open-venue happy-path unit
go RED, and the closed-venue DoD test now runs through `createVoiceGate(deps)` — so the exact wiring gap the breaker
found (`gate.ts:14` builds the gate with no precondition) cannot ship silently.**

Round-1 made `StatefulPrecondition` OPTIONAL with "absent = allow" (resolution §D row 1) — a silent honest-UI bypass at
the sole write sink, because the production factory `apps/web/src/lib/voice/gate.ts:14` builds
`new ConfirmationGate(createVoiceHandlers(deps))` with no precondition. Resolved on ALL FOUR sub-requirements:

| # | Requirement | Resolution (exact seam) | Where |
|---|-------------|-------------------------|-------|
| (1) | Fail-closed, not opt-in | `ConfirmationGate.submit()`/`confirm()` STATEFUL branch computes `const canApply = this.#precondition?.canApplyStateful() ?? false;` — **absent (`?? false`) ⇒ REFUSE**, identical to a false one: `submit()` returns `rejected('stateful-precondition-missing-or-failed')` and never holds `#pending`; `confirm()` clears `#pending` and returns `rejected`. A STATEFUL proposal is confirmable ONLY when a precondition is present AND true. READ_ONLY/REJECT never read it → unchanged. | §6 seam #3, §3 B1a′, §4, §7 table row 1 |
| (2) | Machine-gate the production wiring | **Red→green unit on the production factory `createVoiceGate`** (§8 guardrail #3): open venue → add works; closed venue → refused; **red fixture** = revert `createVoiceGate` to `new ConfirmationGate(createVoiceHandlers(deps))` (drop the precondition) → the fail-closed default makes the OPEN-venue arm FAIL (add refused) → RED; restore → GREEN. The fail-closed semantics turn the omission LOUD (all adds refused) instead of silent (empty-cart "Done"). An optional eslint "≥2 args to `new ConfirmationGate`" rule is structural backup only — it cannot prove the arg is a *real* precondition, so the factory unit is the authority. | §8 guardrail #3, DoD guardrails item |
| (3) | Test the production path | The closed-venue DoD units MUST construct via `createVoiceGate(deps)`, **not** the raw `new ConfirmationGate(handlers, precondition)` constructor — a raw-constructor test would pass even if the factory forgot the wiring (the exact hole). Pinned in the DoD. | DoD closed-venue item |
| (4) | Apply-outcome honesty | `confirm()` no longer reports `applied` on a no-op add: `#apply` returns whether `deps.addItem` actually fired (`addToCart: (args) => boolean`, `false` on every drop path in `handlers.ts:58-88`); `confirm()` returns `applied` only when a real mutation occurred, else `rejected('apply-noop')` → hook → neutral pill. "Done ✓" now requires a real cart line, whether the block came from the venue OR a stale/unavailable product. | §6 seam #4, §7 table new row, DoD, R13 |

Existing STATEFUL-apply gate tests (which relied on the removed permissive default) are migrated to pass a satisfied
precondition — a required, visible test migration, not a silent behavior shift. READ_ONLY/REJECT tests unchanged.

## R-b [MED] — choice: **(b) downgrade to parity + server-authoritative; DROP the "live centerpiece" framing.**

**Choice (b), NOT (a).** (a) as specified — recompute `orderingDisabled` from venue hours + wall clock at getter-read —
is **not contained; it is a red-line violation**: it duplicates the server-authoritative `ENFORCE_VENUE_HOURS` decision
(ledger #65) client-side (server is authoritative for status), and a timezone/DST/clock-skew mismatch would make voice
*contradict* both the server AND the tap path — a NEW honest-UI failure strictly worse than parity. Confirmed root cause:
`MenuPage.tsx:458-472` sets `venueStatus` **once** per `[slug]`; no interval/visibilitychange/wall-clock re-poll
(`closesAt`/`storeHours` are display-only, `:807-810,:1759-1766`) → a time-based close with the tab open keeps
`orderingDisabled===false` the **whole session** (round-1's "≈one render window" was a mischaracterization — corrected).

- The getter STAYS (it is live w.r.t. React re-renders + the point-of-action `submit()`/`confirm()` checks — real and
  load-bearing for the sub-render-window residual and any future refreshed snapshot). What is dropped is the *session-long
  wall-clock liveness over-claim*.
- The honest claim is now: **parity with the tap path (both read the same snapshot); server (#65) authoritative** at
  checkout. A genuine liveness fix = re-poll the shared `venueStatus` (fixes BOTH paths), a separate `MenuPage`-hotspot
  change, owner human/product — NOT smuggled into this dark mount.

**Counsel WATCH-LINE re-evaluation (recorded for the human):** descoping the wall-clock liveness does **NOT** re-trigger
the honest-UI ETHICAL-STOP. The ethical-stop guards "voice affirms ('Done ✓') what the server denies" and "voice worse
than / bypassing the tap path's guard" (the original F2 SAFETY finding). B1a′ delivers point-of-action honesty AND brings
voice to **exact parity** with the incumbent tap path, which already ships with this identical snapshot-staleness under an
accepted server-authoritative posture. Parity with an already-accepted honest-UI contract is not a new ethical
regression. **The hazard was the OVER-CLAIM (asserting a liveness guarantee that doesn't exist), not the parity.** The
corrected "parity, server-authoritative" framing is ethically clean. Human/Counsel sign-off requested only on the
downgrade wording; not a stop. (§10 R1.)

## R-c [MED] — corrected over-claim: SPECIFIER gate, not capability gate.

**Disposition: FIX the claim + keep the gate + flag PR-4.** The `no-voice-app-import` scope-extension is a genuine
red→green **specifier gate** — it catches the canonical `Cart*` / api-client / fetch-client *import* under
`apps/web/src/lib/voice/**` (incl. `mockEngine.ts`); keep it. Corrected claim (§8 guardrail #1, R8): it does **NOT**
machine-cover a bare `fetch('/api/orders',{POST})`, which needs no import and slips through. **INERT in this dark PR** —
the MockEngine does zero network (+0 external calls, §2), so there is no fetch sink to bypass to. **PR-4 X-blocker (not
this-PR):** the network-capable real engine needs a compensating *capability* control — a single allowlisted api-client +
ban raw `fetch` in `apps/web/src/lib/voice/**` (eslint `no-restricted-globals`/`no-restricted-syntax`), and/or a CSP
`connect-src` allowlist / network guard. Recorded §10 R14 + DoD PR-4 X-blocker item.

## LOWs

- **F4-residual (fresh single-transcript per `start()`):** pinned in §3a-contract point #2 — each `start()` reads a FRESH
  single-transcript source that is CONSUMED (cleared) on read (`window.__VOICE_E2E__` yields one transcript/session), so a
  barge-in or second `start()` never replays the prior transcript; the provider/matcher is per-session, never a long-lived
  shared iterator. Deterministic E2E: transcript N drives session N only.
- **F6 ops-dependency (staging build-arg):** named in the DoD — **staging deploy MUST pass
  `--build-arg VITE_VOICE_ENABLED=true`** (staging-deploy-flags class: `flyctl deploy` bakes VITE flags at build time; an
  unset flag defaults OFF → the whole voice chunk DCE's on staging too → the MicFab never renders → the E2E
  **silently cannot run** = false-green). Gated by the deploy command + a pre-E2E `voice-fab`-present assertion. The
  OFF-build bundle-absence assertion (F5) is a SEPARATE prod-parity build, not the staging proof build.

## Regression self-check (round-2 fixes vs F1–F6 and money REJECT)

| Fix | Re-opens F1–F6? | Touches money REJECT? |
|-----|-----------------|-----------------------|
| R-a(1) fail-closed default | **No — STRENGTHENS F1** (dishonest "Done"): absent⇒refuse is stricter than round-1. F3 (`#pending` never dropped) untouched — gate still built once. | **No.** The precondition sits in the STATEFUL branch, AFTER `submit()`'s REJECT check (`confirmation-gate.ts:63-64`); REJECT kinds (money/checkout/dietary/settling) still return `rejected` before any precondition read. Capability-table has no money kind — unchanged. |
| R-a(2)/(3) machine gate + factory-path test | No — additive test/guardrail. | No. |
| R-a(4) apply-noop honesty | **No — STRENGTHENS the honest-"Done" property (F1 family)** via a second path. Bridge contract (F4/F7/F8) untouched. | No — `#apply`/`addToCart` boolean return is on the STATEFUL add path only; REJECT never reaches `#apply`. |
| R-b (b) parity downgrade | No — F1/F3 point-of-action honesty (submit/confirm re-check, `#pending` survival) is UNCHANGED; R-b corrects a *claim* about a residual F1/F3 never fixed. | No. |
| R-c claim correction + PR-4 flag | No — F2 mechanism (specifier import gate) kept; only the claim scope corrected + a PR-4 flag added. F5/F6 untouched. | No. |
| LOW F4-residual / F6 build-arg | No — additive to §3a-contract + DoD; F4/F5/F6 mechanisms unchanged. | No. |

**"What holds" (unchanged from round 1):** money/checkout/settling REJECT airtight, dietary REJECT, cross-session stale
guard, `@deliveryos/ui` does not re-export the engine, heavy ASR deps out of the static graph. No round-2 fix touches these.

## FINAL X-blocker / Playwright list for the build's error-fix matrix (supersedes §E round-1)

Red→green in the implementing PR before merge (▲ = new/changed in round 2):

1. Open venue → `ADD_TO_CART` submit `pending-confirm` → confirm `applied` + `addItem` — **through `createVoiceGate(deps)`**. — unit. ▲
2. Closed/preview venue add → **no confirm chip, no "Done", cart empty** (submit-reject) — through `createVoiceGate(deps)`. — unit + staging E2E. ▲
3. **`createVoiceGate` with NO precondition → open-venue add REFUSED** (fail-closed machine gate, R-a(2)). — factory unit, red fixture. ▲
4. Product goes unresolvable/unavailable/requires-modifiers between propose & confirm → **confirm → no `addItem`, FSM not `applied`** (apply-noop honesty, R-a(4)). — unit. ▲
5. Venue closes AFTER pending, at confirm → **no `addItem`, FSM not `applied`** (confirm re-check, absent⇒refuse). — unit.
6. Re-render between propose & confirm → **`#pending` survives** (gate not rebuilt) → confirm still adds. — unit.
7. Multi-transcript session → **exactly one `onProposal` per `start()`**; **fresh transcript per `start()`, no replay on barge-in** (F4-residual). — unit. ▲
8. Unmatched transcript → **`onNoMatch`** (never a silent `transcribing` spinner). — unit + E2E.
9. Engine throw / no-terminal within watchdog → **`onError('try_again')`**, FSM not wedged. — unit.
10. Barge-in (`abort()`→`start()`) → **no cross-session mutation** (closure-captured handlers/abort + session-id guard). — unit.
11. OFF build → **no voice chunk in `dist/`** (bundle grep asserts absence) — SEPARATE prod-parity build. — CI bundle assert.
12. Staging proof build → **MicFab present** (deploy passed `--build-arg VITE_VOICE_ENABLED=true`); pre-E2E `voice-fab` visible (F6 false-green guard). — E2E precondition. ▲
13. Missing `voice.*` key (incl. dynamic `voice.err.${kind}` + `ariaKeyForPhase`) → **key-existence guardrail red**. — unit guardrail.
14. Engine-dir file importing a Cart mutator / api-client → **`no-voice-app-import` red** (specifier gate). — eslint fixture.
15. Page importing `@deliveryos/voice` directly → **`no-voice-engine-import-outside-adapter` red**. — eslint fixture.
16. Disclosure DECLINE → **mic unactivated, touch works, equal-weight buttons**. — E2E.
17. Closed/preview `NAVIGATE_CHECKOUT` → **no `?checkout=1` navigation** (onNoMatch). — unit/E2E.

**Deferred / not-this-PR (flags, not build blockers):** R14 PR-4 network-capability control (allowlisted client + raw-`fetch`
ban / CSP) — PR-4 X-blocker. R1 wall-clock liveness re-poll — separate `MenuPage` change (human/product). STOP-DESIGN-B
(flip-ON + delete-if-unmounted conditions) — human, gates landing. R-b downgrade wording — Counsel/human sign-off.

Every E2E above is labeled **WIRING-proof, not product-proof** (Counsel §1).
