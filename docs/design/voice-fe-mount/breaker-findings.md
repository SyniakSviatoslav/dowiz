# Breaker findings — voice-fe-mount (design-time, round 1)

No CRITICAL. Money/checkout/settling REJECT holds by construction (capability-table has no money kind → fail-closed; gate `#apply` has no such case; VoiceHandlers has no such method). The damage is **fail-closed-on-write but fail-OPEN-on-UX**, plus a **downgraded enforcement invariant**. Ranked hardest-first.

## [HIGH] F1 · B-CONSIST — Closed-venue add renders "Done ✓" while nothing is added
B1a's `orderingDisabled` check is at the top of adapter `addToCart`, but the confirm path signals success independently: `ConfirmationGate.confirm()` (`confirmation-gate.ts:76-84`) returns `{status:'applied'}` unconditionally after `#apply`; `useVoiceControl.confirm()` (`useVoiceControl.ts:184-188`) ignores the result; reducer `CONFIRM` (`state-machine.ts:140-143`) → `applied` with no check. **Break:** closed venue → proposal → confirm chip → tap → `addToCart` early-returns (no `addItem`) → `confirm()` still `applied` → MicFab "Done" pulse, cart empty. Steady-state, fires every time. Violates the `applied`-phase invariant + ledger-#65 tap-path parity (tap returns early with NO success toast).

## [HIGH] F2 · B-SEC — MockEngine lives outside the engine-purity guardrail jurisdiction
`no-voice-app-import` / `no-voice-engine-callback` scope only `/packages/voice/src/` (`eslint-plugin-local/src/index.js:758,690`). A1 puts the engine in `apps/web/src/lib/voice/mockEngine.ts` — outside both. The proposed `no-voice-engine-import-outside-adapter` only bans the `@deliveryos/voice` specifier under `pages/**`; it does NOT stop an engine→mutator import. So `mockEngine.ts` (and its PR-4 successor) could `import { useSharedCart }`/an api-client and call `addItem` directly, bypassing `ConfirmationGate`, with no guardrail firing. ADR-0015 §6 downgraded machine-checked→prose for exactly the new component (green-by-construction, ledger #67 class).

## [HIGH] F3 · B-CONSIST — `useMemo`-rebuilt gate silently drops `#pending`
Rebuilding the gate = new `ConfirmationGate` with `#pending=null`; safe only if every `createVoiceGate` dep is referentially stable. Setters + `addItem` are stable; **`getProduct`/menu-resolver are not** unless memoized → `exhaustive-deps` forces them into the memo array → gate rebuilt every render. **Break:** re-render between proposal and confirm → `#pending` lost → `confirm()` hits fresh empty gate → FSM still shows `applied/Done` (F1 mechanism). Omit `getProduct` to stop churn → gate closes over stale menu → read-after-write on `menu_version`. A hidden fork.

## [HIGH] F4 · B-FAIL — pull→push impedance mismatch (multi-yield + no-match wedge)
`MockProvider.intents()` (`mock-provider.ts:24-29`) yields one-per-transcript-in-list and SKIPS unmatched/below-`MIN_CONFIDENCE` (null → nothing yielded). UI port is one-start-one-utterance-one-proposal (`types.ts:80-83`). **Multi-yield:** draining N in one non-stale session → 2nd+ `onProposal` re-enters `gate.submit()`; READ_ONLY 2nd applies immediately, STATEFUL 2nd orphans a `#pending` the FSM (stuck `applied`) never surfaces. **No-match wedge:** unmatched transcript yields nothing → no `onNoMatch`/`onError`/terminal → FSM sits in `transcribing` with NO watchdog (only `APPLIED_HOLD_MS` exists) → permanent spinner, escapable only by re-tap. §7 "no match → onNoMatch neutral copy" is unreachable on this path.

## [MED] F5 · B-OPS — true-dark "chunk not emitted / 0 KB" is false as designed
`const VoiceMount = lazy(() => import('./VoiceMount'))` is module-scope; the `{VOICE_ENABLED && <VoiceMount/>}` guard gates RENDER, not the module-scope `import()`. Rollup/terser won't DCE an imported non-PURE `lazy()` → chunk emitted even when the flag is compile-time false (the cited MediaGallery precedent ships its chunk in every build). Runtime hotpath cost stays ~0 (chunk never fetched when gated off), but the DoD "OFF build emits no voice chunk (bundle assertion)" is **unsatisfiable as written**. Fix direction: gate the `import()` itself behind the constant.

## [MED] F6 · B-OPS — proof seam contradiction (DEV-only injection vs staging E2E)
R2 gates transcript injection on `import.meta.env.DEV`. The DoD requires a Playwright E2E on **staging** (production Vite build, `DEV===false`). The DEV seam is dead on staging → the mandatory proof (tap→transcript→cart `toBeVisible`, + closed-venue arm) can't run in the one env the rule requires. Fix direction: gate the seam on `VITE_VOICE_ENABLED` (live on staging, dead in prod), not `DEV`.

## [MED] F7 · B-FAIL — engine async throw → wedged FSM (unstated try/catch)
`VoiceEngine.start()` returns void; §7 claims "throw → onError('try_again')" but that needs the whole async session body try/caught with an `onError` in catch. An escaped throw → unhandled rejection, no `onError` → FSM stuck (no watchdog, F4). PR-4 real mic/model inherits the same wedge, where throws are likelier.

## [MED] F8 · B-CONSIST — `abort()` single-flag reset is a shared-mutable-state race
Cross-session safety rests on callbacks closing over their own `sessionId`. If MockEngine's in-flight loop reads instance fields (`this.handlers`/`this.aborted`) rather than the handlers captured at ITS `start(handlers)`, then barge-in (`abort()` then `start()`, `useVoiceControl.ts:155-164`) resets the flag + swaps handlers → old loop pushes into the NEW (non-stale) session → real mutation from a superseded utterance. Only the closure-capture form is safe; the design doesn't pin it.

## [MED/LOW] F9 · i18n — missing key renders raw dotted key to the customer
`translate()` returns `hit || fallback || key` (`i18n.ts:38`); every voice `t('voice.*')` OMITS the fallback → a missing key shows literal `voice.retry` to the user (degraded, not neutral copy). Two are DYNAMIC (`ErrorPill.tsx:42` `t(\`voice.err.${kind}\`)`, `MicFab.tsx:94` `t(ariaKeyForPhase(phase))`) — invisible to static scan; `i18n-parity.mjs` enforces sq/en/uk symmetry, not call-site existence.

## [LOW] F10 · Docs — key count inconsistent (DoD "17" vs enumeration 20)
15 named + 5-member `voice.err.*` family = 20. An implementer sizing to 17 under-adds.

## [LOW] F11 · B-CONSIST — B2a opens checkout on a preview venue
`ClientLayout.tsx:77-86` handles `?checkout=1` → `setCheckoutOpen(true)` unconditionally (no preview/orderingDisabled check). Voice `NAVIGATE_CHECKOUT` pops checkout on a preview venue, contradicting "checkout never renders on preview." Empty-cart auto-close mitigates → LOW; no order can be placed (money REJECT holds).

## What holds (not inflated)
Money/checkout/settling REJECT airtight; dietary REJECT; cross-session stale guard; `@deliveryos/ui` does not re-export the engine; heavy ASR deps stay out of the static graph (`transformers-transcriber.ts:74-77` non-literal `import(spec)`).
