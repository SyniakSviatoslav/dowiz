# ADR DRAFT — Voice FE mount into the storefront (DARK behind `VITE_VOICE_ENABLED`)

> **DRAFT — design-time, no production code.** Co-located intentionally. `docs/adr/` is protect-paths gated;
> the canonical numbered ADR is authored there at build time from this draft. Proposed number: **ADR-0021**
> (verify next-free at promotion). Extends — does not contradict — **ADR-0015 (voice-control)**.

- **Status:** PROPOSED (draft). Date: 2026-07-03.
- **Slug:** `voice-fe-mount`
- **Design doc:** `docs/design/voice-fe-mount/proposal.md`
- **Relates to:** ADR-0015 §5/§6 (the parent voice ADR — this is the mount phase of its Phase-1); ADR-001
  (monolith-first — the mount adds no service); the dark-flag pattern; ledger #62/#63 (voice guardrails),
  #65 (`ENFORCE_VENUE_HOURS`), #68 (voice-FE drift).

## Context

The voice FE is built and proven on-branch (`packages/voice` engine core, `packages/ui/src/voice` UI+FSM,
`apps/web/src/lib/voice` adapter; 58/58 unit green; engine-isolation guardrails green) but **unmounted**. Mounting
it into `apps/web/src/pages/client/MenuPage.tsx` DARK behind `VITE_VOICE_ENABLED` (default OFF), READ-ONLY,
client-only, surfaces five gaps: (1) **no `VoiceEngine` implementation** — the UI port needs a push
`start(handlers)/abort()` driver; `packages/voice` exposes only a pull `AsyncIterable<IntentProposal>` source;
(2) `handlers.ts addToCart` checks `available`/`hasRequiredModifiers` but **not `orderingDisabled`** → the ledger-#65
closed-venue/preview guard the tap path enforces is bypassed by voice; (3) `onNavigateCheckout` targets
`setCheckoutOpen` in the parent `ClientLayout`, not `MenuPage`; (4) **zero `voice.*` i18n keys** exist; (5) the Toast
API has no Undo affordance `onReadOnlyApplied` anticipates. Non-negotiable: ADR-0015 §6 (engine write-incapable;
`ConfirmationGate` the sole write sink; `MenuPage` never imports `@deliveryos/voice` directly; money/checkout/
settling fail-closed to REJECT; guardrails stay green).

## Decision

Mount the voice FE dark via a single quarantined component, resolving each gap at the correct architectural seam:

1. **Mount site.** New lazy `apps/web/src/pages/client/VoiceMount.tsx`. `MenuPage.tsx` gains ONE guarded line
   (`{VOICE_ENABLED && <Suspense><VoiceMount {…setters}/></Suspense>}`), reusing the existing `lazy()` pattern.
   All voice wiring is inside `VoiceMount` → minimal, true-dark diff off the `MenuPage` hotspot.

2. **Engine — MockEngine-in-adapter (Decision a / A1).** `apps/web/src/lib/voice/mockEngine.ts` implements the UI
   `VoiceEngine` port, bridging `@deliveryos/voice`'s pull matcher (`matchIntent`/`MockProvider`) to the push
   handler surface (synthesizes `onPermissionGranted → onTranscribing → onProposal`; `abort()` sets an abort flag
   backing the FSM `sessionIdRef` guard). Ships dark, E2E-provable via MockProvider today; real getUserMedia+VAD+
   WhisperProvider (PR-4) is a drop-in port swap. Rejected: A2 no-mount-until-PR4 (leaves the FE unexercised — the
   ledger-#68 drift risk); A3 push-engine-in-`packages/voice` (reverses the ADR-0015 §5 pull-source inversion).

3. **Closed-venue fail-closed (Decision b-i / B1a).** Add `orderingDisabled: boolean` to `VoiceStorefrontDeps`;
   `addToCart` checks it FIRST — before `getProduct`/`addItem` — returning `onNoMatch({reason:'ordering-disabled'})`.
   The fix lives in the adapter handler (the sole write sink's dispatch), mirroring the tap path
   (`MenuPage.tsx:691,707`) and ledger #65. Liveness: the mount rebuilds the gate via `useMemo` keyed on
   `orderingDisabled` so a mid-session close is fail-closed (a swap between proposal and confirm drops the pending →
   nothing added). Only `ADD_TO_CART` is gated; `NAVIGATE_CHECKOUT` stays READ_ONLY-ungated (server refuses closed
   orders — parity with the tap path). Rejected: B1b mount-hack hide (kills READ_ONLY browse on a closed venue; not
   defense-in-depth at the sink).

4. **Checkout-nav (Decision b-ii / B2a).** `onNavigateCheckout` navigates to `?checkout=1`, reusing the existing
   `ClientLayout.tsx:77-86` deep-link seam (opens the checkout sheet, strips the param). Pure navigation, no write,
   no new contract. Rejected: B2b `useOutletContext` (new contract, context-absent handling); B2c window-event
   (untyped, weakest contract).

5. **i18n.** 17 `voice.*` keys added ONCE to `packages/ui/src/lib/i18n-catalog.ts` (key-major SSoT, owned by the UI
   package that renders them), en+sq+uk, via `scripts/i18n-add.ts`, passing `scripts/i18n-parity.mjs`.

6. **Toast Undo.** v1 **plain confirmation toast** via the existing `useToast().showToast` — READ_ONLY changes are
   UI-local user-reversible (ADR-0015 §6). The Toast action-button API extension (+ pre-apply state snapshot) is
   **deferred** (accept-risk), not built.

**Data / migrations:** **NONE** — client-only; zero DB, zero tables/columns, zero endpoints, zero workers, zero
`packages/db/migrations/` entries; every mutation is an existing already-authz'd client setter.

**Guardrails (exit criteria, design-time — implemented red→green by the build PR):** keep
`no-voice-app-import` / `no-voice-engine-callback` / `capability-table` green; add
`no-voice-engine-import-outside-adapter` (bans `@deliveryos/voice` under `apps/web/src/pages/**` and outside
`apps/web/src/lib/voice/**`) to machine-check the "`MenuPage` only via the adapter" invariant.

## Consequences

**Positive.**
- **True dark when OFF:** 0 KB (Vite prunes the guarded lazy import), 0 render cost beyond one boolean; zero
  storefront-hotpath regression; instant riskless rollback (flag/revert, no data change).
- **+0 server load:** 0 DB connections, 0 endpoints, 0 workers, 0 migrations — the connection budget is untouched.
- **E2E-provable now:** the MockEngine + MockProvider give a deterministic Playwright path before any real mic.
- **Safety preserved by construction:** engine write-incapable; gate the sole write sink; adapter the sole
  `@deliveryos/voice` importer; closed-venue add fail-closed; money/checkout/settling REJECT; guardrails green.
- **PR-4 is a port swap:** the real ASR engine replaces the MockEngine behind the same `VoiceEngine` interface.

**Negative / costs.**
- ON adds a ~20–30 KB gzip lazy chunk (non-blocking, post-paint).
- `orderingDisabled` liveness needs the `useMemo` gate-rebuild (accepted residual: a one-render race, server-refused
  at checkout).
- Toast Undo deferred (accepted — user-reversible READ_ONLY).
- Real ASR, the runtime `/api/public/voice-config` hot-kill, and the CSP R2 widening remain ADR-0015 launch-gate
  exit criteria — this mount stays dark until the human demand decision + launch-blockers B1/B2/B3.

## Alternatives considered
- **A2 no-mount-until-PR4** — rejected (unexercised FE, drift risk).
- **A3 push-engine inside `packages/voice`** — rejected (reverses ADR-0015 §5 pull-source inversion).
- **B1b hide MicFab when closed** — rejected (kills READ_ONLY browse; not sink-level defense-in-depth).
- **B2b outlet-context / B2c window-event for checkout-nav** — rejected (more/weaker contract than reusing the
  proven `?checkout=1` seam).
- **Extend the Toast API for Undo now** — deferred (UX nicety, not a safety requirement).

## Status

**PROPOSED (draft).** Requires the serious/triadic gate (voice→cart is red-line-adjacent) + Breaker/Counsel
dispositions in `docs/design/voice-fe-mount/resolution.md` before landing. **Accept-risk:** ON bundle delta,
`orderingDisabled` one-render residual, Toast-Undo defer, MockEngine test-seam. **Defer-flag:** real ASR / hot-kill
endpoint / CSP widening / demand evidence (ADR-0015 launch gate). **Needs-human-decision (does not block the dark
mount):** the launch decision (demand + B1/B2/B3). This ADR mounts voice **dark and ready**, not launched.
