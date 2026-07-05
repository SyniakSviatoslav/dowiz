# Design Proposal — Mount the voice FE into the storefront (DARK behind `VITE_VOICE_ENABLED`)

- **Status:** DESIGN-TIME (no production code). Date: 2026-07-03.
- **Author:** System Architect (DeliveryOS).
- **Change class:** serious / red-line-adjacent (voice → cart). Runs the triadic/serious gate before landing.
- **Co-located ADR draft:** `docs/design/voice-fe-mount/ADR-DRAFT-voice-fe-mount.md`.
- **Anchors:** ADR-0015 (`docs/adr/0015-voice-control.md`) §5/§6; `docs/design/voice-control/FE-INTEGRATION-PLAN.md`;
  ledger rows #62/#63 (voice PR-0 guardrails), #65 (`ENFORCE_VENUE_HOURS` closed-venue gate), #68 (voice-FE-drift).

---

## 1. Problem + non-goals

### Problem
The voice FE is built and proven on-branch — `packages/voice` (read-only engine core), `packages/ui/src/voice`
(MicFab + FSM + confirm/read-back/error UI), `apps/web/src/lib/voice` (headless adapter: `ConfirmationGate` +
handlers + menu-context). It compiles clean against HEAD, 58/58 unit green, engine-isolation guardrails green.
It is **not mounted** — nothing on the storefront constructs the engine, threads the setters, or renders the FAB.
This proposal designs the mount: wire the three tiers into `apps/web/src/pages/client/MenuPage.tsx`, **DARK behind
`VITE_VOICE_ENABLED` (default OFF)**, READ-ONLY voice, client-only, no place-order/pay.

Mounting surfaces five gaps recon flagged (each designed in §3–§7):
1. **BLOCKER** — no `VoiceEngine` implementation exists. `useVoiceControl` requires `engine.start(handlers)/abort()`
   (`packages/ui/src/voice/types.ts:80-89`); `packages/voice` providers are pull-based matcher sources
   (`MockProvider.intents()` → `AsyncIterable<IntentProposal>`), never a push/callback session driver.
2. **SAFETY** — closed-venue / preview ordering bypass. `apps/web/src/lib/voice/handlers.ts:58-88` `addToCart`
   checks only `available`/`hasRequiredModifiers`, **not** `orderingDisabled`. The tap path enforces the ledger-#65
   `ENFORCE_VENUE_HOURS` guard (`MenuPage.tsx:451,691,707` `orderingDisabled = isClosed || isPreview`); voice
   bypasses it → a voice add on a closed/preview venue.
3. `onNavigateCheckout` targets `setCheckoutOpen` which lives in the **parent** `ClientLayout.tsx:49`, not MenuPage.
4. **Zero `voice.*` i18n keys** exist — components call `t('voice.*')` (20 keys — 15 named + the 5-member
   `voice.err.*` family — enumerated §7-keys; two families are resolved DYNAMICALLY and are invisible to a static scan).
5. Toast has **no Undo affordance**; `onReadOnlyApplied` expects "toast + Undo".

### Non-goals (explicit)
- **Real microphone / ASR.** No `getUserMedia`, no VAD, no `WhisperProvider`/`TransformersTranscriber`, no
  `@huggingface/transformers`/`onnxruntime-web` load, no R2 model fetch. Real capture is **PR-4**, a drop-in
  replacement of the same `VoiceEngine` port. Out of scope here.
- **Launch.** This is a DARK deploy (flag OFF). Launch is a separate, human-gated act (ADR-0015: demand evidence +
  launch-blockers B1/B2/B3 + the `/api/public/voice-config` runtime hot-kill + the CSP R2 widening).
- **Admin/courier voice.** Removed from the active build (ADR-0015 scope narrowing 2026-06-30).
- **Any new voice→money grammar.** No place-order/pay/checkout-field-write/payment/finalize kind (ADR-0015 §6).
- **New endpoints, migrations, workers, telemetry.** Asserted zero in §5.

---

## 2. Back-of-envelope

### Traffic / connection budget (the load this change adds server-side)
Voice is **client-only**. Pilot scale is small (order of 1–5 active tenants, single-digit orders/min at a busy
venue). This change adds, server-side:

| Resource | Delta |
|---|---|
| DB connections (API + worker + analytics + migrations budget) | **+0** |
| New HTTP endpoints | **+0** (this PR) |
| Background workers | **+0** |
| Migrations | **+0** |
| External network calls (this PR) | **+0** (MockEngine is scripted client-side text; no model fetch, no config GET) |

The connection budget (`packages/config/src/index.ts`) is untouched. The scaling-gate verdict is trivial: nothing
to scale. (The REAL engine's two calls — a fail-closed `/api/public/voice-config` GET + an R2 model GET — are
ADR-0015 launch-phase concerns, **not** this dark mount.)

### Bundle-size delta of the dark mount
- **OFF (`VITE_VOICE_ENABLED` unset/`!== 'true'`): 0 KB — but ONLY if the `import()` itself is gated (F5 fix).**
  A module-scope `const VoiceMount = lazy(() => import('./VoiceMount'))` guarded only at RENDER
  (`{VOICE_ENABLED && <VoiceMount/>}`) does **NOT** elide the chunk: `lazy()` is not a PURE call, so Rollup/terser emit
  the `./VoiceMount` chunk in every build regardless of the render guard. The cited `MediaGallery` "precedent" is a
  **load-on-demand** pattern (chunk present in `dist/`, fetched only when rendered) → it gives 0 *runtime* cost, NOT
  0 *bytes*. For genuine true-dark the dynamic `import()` must live in the **dead arm of a constant-folded ternary** so
  the whole expression tree-shakes away:
  ```ts
  const VOICE_ENABLED = import.meta.env.VITE_VOICE_ENABLED === 'true'; // Vite `define` → literal → const-folded
  const VoiceMount = VOICE_ENABLED ? lazy(() => import('./VoiceMount')) : null; // OFF: `false ? … : null` → import() DCE'd
  // render: {VoiceMount && <Suspense fallback={null}><VoiceMount {…props} /></Suspense>}
  ```
  With `VITE_VOICE_ENABLED` unset the ternary folds to `false ? … : null`, the only `import('./VoiceMount')` reference
  sits in the eliminated arm, and the chunk is **never emitted**. Provable by a bundle assertion (see §9 / DoD). Zero
  bytes, zero critical-path delta.
- **ON: ~20–30 KB gzip, one lazy chunk, loaded AFTER menu paint (non-blocking).** Composition estimate:
  - `packages/ui/src/voice/*` (MicFab + FSM + 8 UI components, ~1,800 LOC per ledger #68) ≈ **10–14 KB gzip**.
  - `apps/web/src/lib/voice/*` adapter (~500 LOC) ≈ **~4 KB gzip**.
  - `@deliveryos/voice` engine-core actually reachable here (types + `capability-table` + `confirmation-gate` +
    `matcher` + `mock-provider` + `normalize` + `dietary-denylist`; **not** whisper/transformers) ≈ **~5–8 KB gzip**.
  - **Model / WASM: 0 KB** in this PR (real ASR = PR-4).

### Render / perf cost
- **OFF:** one `=== 'true'` boolean short-circuit per `MenuPage` render → **~0**. No hook runs, no component renders,
  no engine constructs, no import evaluates.
- **ON, idle:** MicFab is **static** (no idle pulse — ui-spec §1 anti-surveillance). Zero `requestAnimationFrame`
  while idle. The single rAF loop (`MicFab.tsx:71-90`) runs **only** during `listening`. Negligible.

---

## 3. Options (≥2 each) for the two hard decisions

### Decision (a) — the `VoiceEngine` adapter shape (the BLOCKER)

The UI port (`packages/ui/src/voice/types.ts:80-89`) is a **push/callback session driver**:
`start(handlers: VoiceEngineHandlers)` + `abort()`, where `VoiceEngineHandlers` are narrow **report** callbacks
(`onPermissionGranted`, `onPartialTranscript`, `onTranscribing`, `onProposal`, `onNoMatch`, `onAmbiguous`,
`onError`). Critically, the engine **never** receives a write-capable closure: `useVoiceControl` itself calls
`gate.submit()` inside `onProposal` (`useVoiceControl.ts:103-120`). `@deliveryos/voice` exposes a **pull** source
(`MockProvider.intents()` → `AsyncIterable<IntentProposal>`). The adapter must bridge pull → push.

| Option | Concept | Tradeoffs |
|---|---|---|
| **A1 — MockEngine-in-adapter** (`apps/web/src/lib/voice/mockEngine.ts`) | Adapter (anti-corruption layer): a `createMockVoiceEngine(...)` implementing the UI `VoiceEngine` port, backed by `@deliveryos/voice`'s `matchIntent`/`MockProvider`. `start(handlers)` synthesizes the session lifecycle a real mic would emit (`onPermissionGranted` → optional `onPartialTranscript` → `onTranscribing` → **exactly one** terminal of `onProposal`/`onNoMatch`/`onAmbiguous`/`onError`), consuming **one** scripted transcript per session per the **bridge contract (§3a-contract)** — one-proposal-per-session, no-match→`onNoMatch`, watchdog→`onError`, closure-captured handlers+abort (F4/F7/F8). The whole async body is `try/catch → onError` (F7). | **+** Ships dark AND is E2E-testable NOW (MockProvider is deterministic — the proof harness the FE was built for). **+** Real getUserMedia+VAD+WhisperProvider (PR-4) is a drop-in replacement of the SAME port + the SAME §3a-contract — no UI/adapter change. **+** Lives in `apps/web` — the allowed boundary; keeps `packages/voice` a pure pull-source (ADR-0015 §5 / R2-F). **−** Needs a deterministic transcript-injection seam for E2E, gated on **`VITE_VOICE_ENABLED`** (live on staging, DCE'd in prod — F6), reading an inert test channel (`window.__VOICE_E2E__`, §10-risks R2). **−** `mockEngine.ts` sits outside the current engine-purity guardrail jurisdiction → machine-covered by extending `no-voice-app-import` to `apps/web/src/lib/voice/**` (F2/§8). |

**§3a-contract — the MockEngine (and PR-4 real engine) `VoiceEngine` implementation contract.** The UI port is
*push, one-`start()`-one-utterance-one-terminal*; `@deliveryos/voice`'s `MockProvider.intents()` is *pull, 0..N yields*.
The bridge MUST satisfy, per `start(handlers)` session:
1. **Closure-capture, never instance fields (F8).** `const capturedHandlers = handlers; let aborted = false;` are `start()`
   locals. `abort()` flips *this* session's `aborted` (via a single-slot `#abortCurrent` fn the session registers), never a
   shared `this.handlers`/`this.aborted`. A barge-in (`abort()`→`start()`) makes a NEW closure; the old loop can only reach
   its OWN (now-stale) handlers, which the hook's `sessionIdRef` guard already no-ops. No cross-session handler swap.
2. **One proposal per session (F4 multi-yield).** Pull the iterator, take the **first** yielded `IntentProposal` →
   `onProposal(it)`, then STOP (do not drain). At most one `onProposal` per `start()`.
   - **Fresh single-transcript source per `start()` (F4-residual, deterministic E2E).** Each `start()` reads a FRESH
     single-transcript source and CONSUMES it — the injection channel (`window.__VOICE_E2E__`) yields exactly one
     transcript per session and is cleared on read, so a barge-in (`abort()`→`start()`) or a second `start()` never
     re-reads the PRIOR session's transcript (no accidental replay). The provider/matcher is constructed per-session over
     that one transcript, never a long-lived iterator shared across sessions. This makes the mock deterministic under the
     E2E harness: transcript N drives session N and only session N.
3. **No-match is never silent (F4 wedge).** If the consumed transcript yields nothing (matcher returns `null`), emit
   `onTranscribing()` then `onNoMatch()` — never leave the FSM in `transcribing` with no terminal.
4. **Watchdog (F4/F7).** Arm a timer at `start()` (`VOICE_WATCHDOG_MS`, ~4 s); if no terminal callback fires by then →
   `onError('try_again')`. Cleared on any terminal (`onProposal`/`onNoMatch`/`onAmbiguous`/`onError`/`onPermissionDenied`)
   or `abort()`. This is the ONLY timeout out of `transcribing` (the FSM has none — only `APPLIED_HOLD_MS`).
5. **Total try/catch (F7).** The entire async session body is `try { … } catch { capturedHandlers.onError('try_again'); }`
   (PR-4: `'model_offline'` for a fetch/model fault) — an escaped throw must become an `onError`, never an unhandled
   rejection that wedges the FSM.
6. **Abort quiescence.** After `abort()` no callback fires: the loop checks `aborted` after every `await` and returns; the
   watchdog is cleared. Belt-and-suspenders to the hook's stale-session guard.
| **A2 — engine-prop-optional / no-mount-until-PR4** | Wire only adapter+gate; leave `MicFab`/FSM unmounted until the real engine exists. | **−** Defeats the purpose: nothing E2E-proves the mount, no dark-deploy value, the ~1,800-line FE sits unexercised — exactly the ledger-#68 drift-and-rot risk we just guardrailed. **−** The mount wiring lands untested and re-verified later under time pressure. **Rejected.** |
| **A3 — provider→engine wrapper inside `packages/voice`** | Add a callback `start/abort` engine into the engine package wrapping `MockProvider`. | **−** Reverses the ADR-0015 §5 source→sink inversion the design structurally rests on: the engine's public API must be a pure `AsyncIterable` **pulled** by the sink, never a push surface (R2-F). **−** Couples `packages/voice` to the UI's handler shape. (Note: it would *not* trip `no-voice-engine-callback` — an object-port param is a `TSTypeReference`, not a `TSFunctionType` — so the guardrail alone would not catch this regression; it is an **ADR-invariant** violation, which is exactly why the invariant must not be diluted.) **Rejected on ADR-0015 §5.** |

**Decision (a): A1 — MockEngine-in-adapter.** It is the only option that ships dark, proves the mount deterministically
today, keeps the pull-source engine invariant intact, and makes PR-4 a pure port swap.

### Decision (b) — closed-venue fail-closed mechanism + checkout-nav threading

**(b-i) Closed-venue fail-closed** (the SAFETY gap):

| Option | Concept | Tradeoffs |
|---|---|---|
| **B1a′ — `isOrderingDisabled()` LIVE getter, gated at the GATE's `submit()`/`confirm()` (primary, FAIL-CLOSED) + `addToCart` handler (defense-in-depth)** | Change the dep from a `boolean` snapshot to a **getter** `isOrderingDisabled: () => boolean` on `VoiceStorefrontDeps`. Inject a READ-only **`StatefulPrecondition` object-port** `{ readonly canApplyStateful: () => boolean }` = `() => !deps.isOrderingDisabled()` into the `ConfirmationGate`. The STATEFUL permit is **CONTINGENT + FAIL-CLOSED** (R-a): `submit()` computes `canApply = precondition?.canApplyStateful() ?? false` before holding `#pending`, so an **absent OR false** precondition → `rejected` (NOT `pending-confirm`) → **no confirm chip, no "Done"** (F1a). `confirm()` **re-checks** the precondition (absent⇒refuse) AND requires a real mutation (`#apply` returns whether `addItem` fired) → `rejected` on a mid-session close OR a no-op add (F1/F3/R-a(4) honest-Done). `addToCart` also checks `deps.isOrderingDisabled()` first (belt-and-suspenders). **The production factory `createVoiceGate` MUST wire the precondition — enforced by a red→green unit on the factory (R-a(2)), not by an opt-in default.** | **+** Honest at the correct layer AND fail-closed: a closed/preview venue *never produces a confirmable proposal*, and a mis-wired sink *refuses all* STATEFUL rather than silently permitting (the UI never affirms what the server denies — Counsel B1a WATCH-LINE). **+** Live via a **stable getter the gate closes over** — the gate is built ONCE, never rebuilt, so `#pending` is never dropped (kills F3). **+** READ_ONLY browse/sort/search still works on a closed venue. **+** Object-port (not a bare closure) → does not trip `no-voice-engine-callback` and gives the gate no new write capability (it stays the sole write sink). **−** Touches `packages/voice` (money-adjacent) — a small, fail-closed generalization; the `precondition?` param stays type-optional so READ_ONLY/REJECT unit tests need no change, but STATEFUL is refused when it is absent, and STATEFUL-apply tests must supply one. Unit-tested red→green. |
| **B1a (superseded) — snapshot `boolean` checked only in `addToCart`, liveness via `useMemo`-rebuilt gate** | Original proposal. | **−** Fails F1: `submit()` still returns `pending-confirm` → confirm chip renders → `confirm()` returns `applied` unconditionally → "Done ✓" on an empty cart (dishonest). **−** Fails F3: rebuilding the gate on `orderingDisabled` change drops `#pending` (or, if `getProduct` is omitted from the memo, closes over a stale menu). **Superseded by B1a′.** |
| **B1b — mount hack: hide MicFab / block voice when closed** | Don't render the voice path at all when `orderingDisabled`. | **−** The task's + Counsel's named anti-pattern ("hiding the affordance is not the same as refusing the write"). **−** Kills READ_ONLY browse-by-voice on a closed venue for no reason. **−** Not defense-in-depth at the write sink. **Rejected — never the guard.** |

**Decision (b-i): B1a′** — LIVE-getter precondition gated at the GATE (`submit()` no-chip + `confirm()` honest-Done) with
the `addToCart` handler as a second layer, and the gate built **once** over a stable getter/ref (§6, kills F3). The
honest property: on a closed/preview venue a voice "add X" yields **neutral no-match copy, never a confirm chip and never
a "Done" pulse.** Scope note (revised, F11): **`ADD_TO_CART` AND `NAVIGATE_CHECKOUT` are both gated** on
`isOrderingDisabled()` — `navigateCheckout` returns `onNoMatch({kind:'NAVIGATE_CHECKOUT', reason:'ordering-disabled'})`
when disabled, so voice never pops a checkout sheet on a preview/closed venue ("checkout never renders on preview"). Cart
review on a closed venue stays available via `READ_ORDER` (pure read of the user's own cart, ungated). Voice never
finalizes; the server remains authoritative (ledger #65 `409 VENUE_CLOSED`) as the last line.

**(b-ii) Checkout-nav threading** (`onNavigateCheckout` → parent `setCheckoutOpen`):

| Option | Concept | Tradeoffs |
|---|---|---|
| **B2a — reuse the `?checkout=1` deep-link seam** | `onNavigateCheckout = () => navigate({ search: '?checkout=1' })`. `ClientLayout.tsx:77-86` already listens for `?checkout=1`, opens the checkout sheet, and strips the param (`replace`). | **+** Boring & proven: reuses the EXACT seam the `/checkout` route redirect uses; zero new prop/context/state; self-cleaning. **+** No new contract on MenuPage/ClientLayout. **−** One (self-replaced) history entry; a URL round-trip vs the cart button's direct `setCheckoutOpen`. |
| **B2b — `useOutletContext`** | ClientLayout passes `<Outlet context={{ onNavigateCheckout: () => setCheckoutOpen(true) }} />`; the voice mount reads it. | **+** Typed, direct, no URL round-trip. **−** New outlet-context contract MenuPage now depends on; must handle context-absent (tests / MenuPage rendered standalone). More wiring for a one-liner. |
| **B2c — window `CustomEvent` bus** | Dispatch `dos:openCheckout`; ClientLayout adds a listener → `setCheckoutOpen(true)` (precedent: `dos:bounceCart`, `ClientLayout.tsx:62`). | **+** Fully decoupled; matches an existing pattern. **−** Untyped global event; another listener; weakest contract. |

**Decision (b-ii): B2a — reuse `?checkout=1`.** It is the most boring, adds the least new surface, reuses a seam
explicitly designed for "open checkout over the menu, deep-link friendly," and self-cleans. `onNavigateCheckout` is
pure navigation (READ_ONLY, no write). The mount component owns the `useNavigate` call, keeping `MenuPage.tsx`'s
diff to a single guarded line.

---

## 4. Decision (ADR-format summary)

**Context.** Mount the built+proven voice FE dark, resolving the engine BLOCKER, the closed-venue SAFETY bypass, the
parent-owned checkout-nav, the missing i18n keys, and the toast-undo gap — without violating ADR-0015 §6.

**Decision.**
- **Mount architecture:** a single new lazy component `apps/web/src/pages/client/VoiceMount.tsx` (the PR-3 "mount
  site"). `MenuPage.tsx` gains **one** guarded line — `{VoiceMount && <Suspense><VoiceMount {…setters} /></Suspense>}`,
  where `VoiceMount = VOICE_ENABLED ? lazy(() => import('./VoiceMount')) : null` so the chunk is truly DCE'd when OFF
  (F5) — passing its setters (`setSortBy`, `setMacroLens`, `setSelectedCategory`, `setSearchQuery`, `toggleCompare`,
  `getProduct`, `addItem`, `isOrderingDisabled` (getter), `filterLensesEnabled`) as props. All voice wiring (deps → gate →
  engine → hook → UI) is quarantined inside `VoiceMount`. This keeps the dark diff minimal (true-dark friendly) and
  the churn off the 99.x%ile `MenuPage` hotspot.
- **Engine:** **A1 MockEngine-in-adapter** (`apps/web/src/lib/voice/mockEngine.ts`) — bridges the pull-source
  `@deliveryos/voice` matcher to the UI's push `VoiceEngine` port per the **§3a-contract** (one-proposal-per-session,
  no-match→`onNoMatch`, watchdog→`onError`, total try/catch, closure-captured handlers/abort); real mic is a PR-4 port
  swap of the SAME port + contract.
- **Closed-venue:** **B1a′** — `isOrderingDisabled()` LIVE getter on `VoiceStorefrontDeps` + a READ-only
  `StatefulPrecondition` object-port injected into the `ConfirmationGate`, **fail-closed on absence** (`?? false`): a
  STATEFUL permit is contingent on a precondition that is present AND satisfied, so a mis-wired sink refuses rather than
  silently permits (R-a). `submit()` refuses a STATEFUL proposal (absent or false → no confirm chip), `confirm()`
  re-checks (absent⇒refuse) AND requires a real cart mutation (`#apply` returns whether `addItem` fired → honest "Done"
  only when a line was actually added, R-a(4)), `addToCart` checks as a second layer; gate built ONCE over a stable getter
  (no rebuild → `#pending` never dropped, §6). The **production factory `createVoiceGate` MUST wire the precondition**,
  enforced by a red→green unit on the factory that exercises `createVoiceGate(deps)` (not the raw constructor) — R-a(2)/(3).
  Both `ADD_TO_CART` and `NAVIGATE_CHECKOUT` are gated (F11). **Liveness scope (R-b):** the getter is live w.r.t. React
  re-renders and the point-of-action `submit()`/`confirm()` checks; it is NOT a wall-clock refresh of the venue-open
  *snapshot* — that stays server-authoritative (#65). Claim corrected below (§6 / R1) from "live centerpiece" to
  "parity with the tap path; server authoritative."
- **Checkout-nav:** **B2a** — reuse the `?checkout=1` deep-link seam, gated on `isOrderingDisabled()` at the
  `navigateCheckout` handler (F11).
- **i18n:** **20** `voice.*` keys (15 named + 5-member `voice.err.*`) added ONCE to `packages/ui/src/lib/i18n-catalog.ts`
  (the key-major SSoT), en+sq+uk, via `scripts/i18n-add.ts` — plus a **call-site key-existence guardrail** that asserts
  all 20 (incl. the two DYNAMIC families `voice.err.${kind}` + `ariaKeyForPhase(phase)`) exist, red→green (F9/F10, §7-keys).
- **Toast Undo:** **v1 plain toast** (`useToast().showToast`, existing API). Undo (a Toast action-button API
  extension) is **deferred** — READ_ONLY changes are trivially user-reversible (§7-toast).

**Rationale.** Boring & proven at every seam (existing lazy pattern, existing `?checkout=1` seam, existing
`i18n-add`/`useToast`); the schema-rich/runtime-minimal principle (the seams — DI deps, port, gate — already exist;
the mount only wires them and the runtime stays dark); failure-first (the closed-venue fail-closed and the
degradation matrix are designed before the happy path); and every ADR-0015 §6 invariant preserved by construction
(the adapter is the sole `@deliveryos/voice` importer; the gate is the sole write sink; the engine is write-incapable).

**Consequences.** +0 server load / +0 endpoints / +0 migrations; 0 KB when OFF; ~20–30 KB lazy when ON. The mount is
E2E-provable today via MockProvider. Real ASR, the runtime hot-kill endpoint, and the CSP R2 widening remain
ADR-0015 launch-gate exit criteria, out of this PR.

---

## 5. Data / migrations

**NONE. Asserted.** This change adds **zero** migrations, zero tables, zero columns, zero DB reads/writes, zero
`packages/db/migrations/` entries. The voice path is client-only: the MockEngine is scripted text, the matcher/gate
run in the browser, and every mutation the adapter performs is a client-cart or client-UI-state setter that already
exists and is already authz'd (`CartProvider.addItem`, the `MenuPage` setters). The Phase-0 research corpus
(ADR-0015 §8.1) is a separate consented regime, not touched here. No forward-only migration, no RLS surface, no
`ENABLE+FORCE` change — because there is no new persisted state. (If future opt-in telemetry is ever added it is a
separate RED-LINE forward-only migration per ADR-0015 — explicitly out of scope.)

---

## 6. Consistency + idempotency (the ADD_TO_CART confirm path)

- **Confirm-once (gate single-shot).** `ConfirmationGate.confirm()` (`confirmation-gate.ts:76-84`) reads `#pending`,
  nulls it, then applies. A second confirm sees `null` → no-op reject. Double-tap the confirm chip → one add.
- **Stale-session guard.** `useVoiceControl`'s `sessionIdRef` (`useVoiceControl.ts:82-134`) invalidates every
  callback from a superseded session; a barge-in re-tap runs `gate.cancel()` + `engine.abort()` and bumps the id, so
  a late `onProposal` from a dead session can never overwrite `#pending` or auto-apply. The MockEngine's own
  closure-captured `abort()` flag (§3a-contract #1/#6) is belt-and-suspenders to this.

- **Closed-venue: gate at `submit()`/`confirm()`, read LIVE — no gate rebuild (F1 + F3 root fix).** The original
  design deferred the check to `addToCart` (`#apply`-time) and handled liveness by rebuilding the gate via `useMemo`
  keyed on `orderingDisabled`. That path is **doubly broken** and is REPLACED:
  - *F1 (dishonest "Done"):* a STATEFUL `ADD_TO_CART` still reaches `submit()` → `pending-confirm` → confirm chip
    renders; on confirm, `ConfirmationGate.confirm()` returns `{status:'applied'}` **unconditionally**
    (`confirmation-gate.ts:83`) and `useVoiceControl.confirm()` (`useVoiceControl.ts:186-187`) **ignores the result**
    → FSM `CONFIRM → applied` "Done ✓" with an empty cart. A handler early-return in `addToCart` cannot fix this — the
    apply outcome is not propagated back to the FSM.
  - *F3 (`#pending` drop / stale menu):* rebuilding the gate constructs a fresh `ConfirmationGate` with `#pending=null`.
    `createVoiceGate`'s deps include `getProduct`/the menu resolver, which are **not** referentially stable → `exhaustive-deps`
    forces them into the memo array → the gate is rebuilt on ordinary re-renders, silently dropping a pending proposal
    between "propose" and "confirm" (or, if `getProduct` is omitted to stop the churn, the gate closes over a stale menu).

  **Resolution (the exact seams):**
  1. **Dep is a LIVE getter, not a snapshot.** `VoiceStorefrontDeps.orderingDisabled: boolean` → `isOrderingDisabled: () => boolean`.
  2. **Gate built ONCE over a stable getter/ref — never rebuilt.** `VoiceMount` keeps a latest-value ref of its live inputs
     and builds a single stable deps object whose fields delegate to that ref, then constructs the gate once:
     ```ts
     const live = useRef({ getProduct, addItem, setSortBy, /* …setters… */, orderingDisabled, filterLensesEnabled });
     live.current = { getProduct, addItem, /* … */, orderingDisabled, filterLensesEnabled }; // refreshed each render
     const deps = useMemo<VoiceStorefrontDeps>(() => ({
       getProduct: (id) => live.current.getProduct(id),          // LIVE menu read (kills F3 stale-menu fork)
       isOrderingDisabled: () => live.current.orderingDisabled,   // LIVE venue read
       addItem: (i) => live.current.addItem(i),
       /* every setter delegates to live.current — stable identity */
     }), []);                                                     // built ONCE → deps identity stable
     const gate = useMemo(() => createVoiceGate(deps), [deps]);   // built ONCE → #pending NEVER dropped (kills F3)
     ```
     This is the same latest-ref pattern `useVoiceControl` already uses for `gate`/`engine` (`useVoiceControl.ts:73-78`).
     Because `deps`/`gate` never change identity, `#pending` survives every re-render; because the getters read
     `live.current`, `getProduct`/`isOrderingDisabled` are always current — **no rebuild, no stale fork**.
  3. **Gate refuses a STATEFUL proposal at `submit()` when the precondition is absent OR false → no confirm chip
     (F1a; FAIL-CLOSED, R-a).** `createVoiceGate(deps)` injects a READ-only `StatefulPrecondition` object-port
     `interface StatefulPrecondition { readonly canApplyStateful: () => boolean }` =
     `{ canApplyStateful: () => !deps.isOrderingDisabled() }` into `ConfirmationGate`. **The STATEFUL permit is CONTINGENT
     and FAIL-CLOSED, not opt-in.** Round-1 made the precondition OPTIONAL with "absent = allow" — a silent honest-UI
     bypass: the production sink `gate.ts:14` builds `new ConfirmationGate(createVoiceHandlers(deps))` with NO precondition,
     so a one-line omission (or a PR-4 re-mount) would let a closed-venue STATEFUL through → "Done ✓ on empty cart"
     regresses. **REVERSED.** In `submit()`, the STATEFUL branch computes
     `const canApply = this.#precondition?.canApplyStateful() ?? false;` **BEFORE** holding `#pending` — so **absence of a
     precondition (`?? false`) REFUSES**, exactly as a false one does: return
     `{status:'rejected', reason:'stateful-precondition-missing-or-failed'}`, never `pending-confirm`, never `#pending`. A
     STATEFUL proposal becomes confirmable ONLY when a precondition is present AND returns true. `useVoiceControl.onProposal`
     (`:112-118`) already maps a non-applied/non-pending result to `NO_MATCH` → neutral copy, **no confirm chip, no "Done".**
     Consequence (the loudness that replaces the silent bypass): a sink that forgets to wire the precondition does not
     silently *permit* — it *refuses ALL* STATEFUL adds, which the "open venue → add works" happy-path unit catches RED.
     READ_ONLY and REJECT branches never read the precondition → unchanged → existing READ_ONLY/REJECT tests stay green;
     existing STATEFUL-*apply* tests (which relied on the removed permissive default) MUST be updated to pass a satisfied
     precondition — a required test migration, not a silent behavior shift.
  4. **`confirm()` re-checks the precondition AND requires a real mutation → honest "Done" (F1 mid-session-close +
     R-a(4) apply-noop).** Contract change (voice-scoped):
     - `ConfirmationGate.confirm()` (`confirmation-gate.ts:76-84`): before `#apply`, fail-closed re-check
       `const canApply = this.#precondition?.canApplyStateful() ?? false; if (!canApply) { this.#pending = null; return {status:'rejected', reason:'stateful-precondition-failed'}; }`
       (absent precondition ⇒ refuse, same as false). Then **PROPAGATE THE APPLY OUTCOME:** `#apply` now returns whether a
       real cart mutation occurred — `addToCart` (the sole STATEFUL handler) returns `true` iff `deps.addItem` was actually
       called, `false` on every DROP path (`missing-product-or-qty` / `unresolved-product` / `unavailable` /
       `requires-modifiers`, `handlers.ts:58-88`, where it calls `onNoMatch` and returns without `addItem`). `confirm()`
       returns `const mutated = this.#apply(p); return mutated ? {status:'applied'} : {status:'rejected', reason:'apply-noop'};`
       — so a product that went unresolvable/unavailable/required-modifiers between propose and confirm yields **no "Done"**,
       not a "Done ✓" over an unchanged cart. (Handler signature: `VoiceHandlers.addToCart: (args) => boolean`; other
       READ_ONLY handlers stay `=> void`, `#apply` returns `true` for their branches — READ_ONLY shows no "Done ✓" pulse,
       so their milder submit-time "applied"-on-no-op over-report is an accepted residual, R12, not this fix's target.)
     - `useVoiceControl.confirm()` (`useVoiceControl.ts:184-188`): **stop ignoring the result** —
       `const r = gateRef.current.confirm(); dispatch(r.status === 'applied' ? { type:'CONFIRM' } : { type:'ENGINE_ERROR', kind:'unavailable' });`.
       `ENGINE_ERROR` transitions from ANY phase (`state-machine.ts:130-133`), including `confirming`, to a neutral
       `error/unavailable` pill (non-retryable). So "Done" (`applied`) renders **only** when a cart line was actually
       added — whether the block came from a mid-session close (precondition) OR a no-op add (apply-noop). The reducer's own
       `VoiceGate.confirm()` return type is already `VoiceGateResult` (`types.ts:39`) — no UI type change, the hook simply
       consumes what it currently discards.
  5. **`addToCart` handler keeps a top-of-function `isOrderingDisabled()` check** (→ `onNoMatch('ordering-disabled')`) as
     defense-in-depth — a third layer at the sink, for any direct-handler path.

  **Two residuals, honestly separated (R-b corrects the round-1 mischaracterization):**
  - *(caught) The sub-render-window between `submit()` and `confirm()`* where a status change has already re-rendered:
    seam #4's confirm re-check reads the LIVE getter → honest neutral copy, no dishonest "Done", no cart mutation.
  - *(NOT caught client-side, server-authoritative) A wall-clock close while the tab is open.* `MenuPage.tsx:458-472`
    sets `venueStatus` **once** per `[slug]`; there is NO interval / `visibilitychange` / wall-clock re-poll (`closesAt`
    and `storeHours` are display-only, `:807-810,:1759-1766`). So a venue that closes at 22:00 with the tab open keeps
    `orderingDisabled === false` for the **whole session** — the getter is "live" only w.r.t. re-renders, not the clock.
    Round-1's "≈ one render window" was wrong: the real window is the session. **This is NOT closed client-side by
    design (R-b decision (b)):** the honest claim is downgraded to **parity with the tap path** — the add-to-cart button
    reads the SAME stale `orderingDisabled`, so both paths behave identically, and the **server is the single authority**
    for venue-open (ledger #65 `409 VENUE_CLOSED` at checkout). Client-re-deriving open-ness from `storeHours` + wall clock
    was REJECTED: it duplicates the server-authoritative `ENFORCE_VENUE_HOURS` decision (a red-line — server authoritative
    for status), risks timezone/DST divergence, and could make voice *contradict* both the server and the tap path — a
    NEW honest-UI failure, strictly worse than parity. A genuine liveness fix (re-poll the shared `venueStatus`) would fix
    BOTH tap and voice and belongs in its own change on the `MenuPage` hotspot, not smuggled into this dark mount (§10 R1,
    owner human/product). Counsel WATCH-LINE re-evaluated in §10 R1: parity does **not** re-trigger the honest-UI
    ethical-stop; the over-claim did.
- **Cart idempotency.** `CartProvider.addItem` dedups by `${productId}_${JSON.stringify(options)}`
  (`CartProvider.tsx:91`); the voice item id is deterministic (`voice_${product.id}`). Re-adding the same product
  merges quantity — no duplicate lines.
- **Server authority.** The voice add is **client-cart only**, fully user-reversible (remove from cart). The server
  re-prices and re-validates status/venue-open at order time (money invariants; ledger #56/#65) — voice never
  finalizes, so no distributed idempotency key is needed (there is no server write to make idempotent).

---

## 7. Failures + degradation (each path: fail-safe, zero cascade)

Voice is a **strictly additive** layer. Its every failure degrades to "voice off / touch unaffected" — it can never
block, slow, or error the menu hotpath.

| Failure | Behavior | Fail-safe property |
|---|---|---|
| Engine throws mid-session (async body) | Total `try/catch` (§3a-contract #5) → `onError('try_again')` → `ErrorPill` (neutral copy + Retry). | No unhandled rejection, no wedged FSM (F7). |
| Engine emits no terminal (hang) | Watchdog `VOICE_WATCHDOG_MS` (§3a-contract #4) → `onError('try_again')`. | FSM can never sit forever in `transcribing` (F4/F7). |
| Provider yields nothing for the transcript | Bridge emits `onTranscribing()`→`onNoMatch()` (§3a-contract #3). | No silent spinner; neutral no-match copy reachable (F4). |
| Multi-yield from `MockProvider.intents()` | Bridge takes the FIRST yield only (§3a-contract #2) → exactly one `onProposal`. | No 2nd apply, no orphaned `#pending` (F4). |
| Barge-in (`abort()`→`start()`) mid-session | Old loop reads its OWN closure-captured `aborted`/handlers (§3a-contract #1); hook `sessionIdRef` no-ops any late callback. | No cross-session handler swap, no stale mutation (F8). |
| Mic denied (PR-4 real engine) | `onPermissionDenied` → error phase; MicFab shows `voice.err.mic_denied`. | Voice off; menu unaffected. |
| No match / below confidence | `onNoMatch` → neutral "didn't catch that" + optional did-you-mean chips. | **Never a wrong apply.** |
| Ambiguous tie | `onAmbiguous` → `DisambiguationChips`; human picks or cancels. | Never guess-executes. |
| Unmappable intent (`popularity` sort, uncovered lens, `requires-modifiers`, unresolved product) | `onNoMatch(reason)` — no setter call (`handlers.ts` drop-rather-than-guess). | No mutation at a wrong value/price. |
| **Closed/preview venue add, OR precondition not wired (at `submit()`)** | `precondition?.canApplyStateful() ?? false` → false OR absent → `submit()` returns `rejected` → `NO_MATCH`; **no confirm chip renders.** | **Fail-closed: no "Done", no cart mutation; a mis-wired sink REFUSES, never silently permits** (F1a/R-a; parity with ledger #65). |
| **Venue closes AFTER pending, at `confirm()`** | `confirm()` re-check (absent⇒refuse) → `rejected`; hook dispatches `ENGINE_ERROR('unavailable')` → neutral pill; nothing applied. | **Honest: "Done" only when a line was truly added** (F1/F3; §6 seam #4). |
| **Confirmed add is a no-op** (product went unresolvable/unavailable/requires-modifiers between propose & confirm) | `#apply` returns false (no `addItem`) → `confirm()` returns `rejected('apply-noop')` → neutral pill. | **Honest: no "Done ✓" over an unchanged cart** (R-a(4)). |
| **Closed/preview `NAVIGATE_CHECKOUT`** | `navigateCheckout` returns `onNoMatch({reason:'ordering-disabled'})`; no `?checkout=1` navigation. | Checkout never pops on preview/closed (F11). |
| REJECT intent (money/checkout-write/dietary/settling) | Gate returns `rejected` → `NO_MATCH` (neutral). | No handler exists to call (`confirmation-gate.ts`); fail-closed default. |
| Model offline (PR-4) | Runtime config → voice OFF (ADR-0015 R2-A). | Absence = OFF; never a page block. |
| Missing `voice.*` key at a call-site | Dynamic-call-site neutral fallback (`t(key, NEUTRAL)`) renders neutral copy, not a raw dotted key; the key-existence guardrail fails CI (F9). | Never `voice.err.foo` shown to a customer; the gap is caught before merge. |

**External calls in this PR: zero** → no timeout/retry/circuit-breaker needed here (MockEngine is in-process). The
real engine's two calls (fail-closed `/api/public/voice-config` GET whose only failure mode is "voice OFF", + the
R2 model GET) are ADR-0015 launch-phase, out of scope. The failure-first ordering above is authored ahead of the
happy path per the design discipline.

### i18n keys (§7-keys) — **20** `voice.*` keys, owned by `packages/ui/src/lib/i18n-catalog.ts` (F9/F10)
The UI (`packages/ui/src/voice/*`) calls `t('voice.*')`; the catalog currently has **0** such keys. The keys live in
the **UI** catalog (not `apps/web`) because the components rendering them are in `packages/ui` and the catalog is
the single key-major SSoT with the sq/en/uk parity gate. Enumerated from source — **15 named**:
`voice.fab_label`, `voice.listening`, `voice.transcribing`, `voice.applied`, `voice.retry`, `voice.did_you_mean`,
`voice.confirm`, `voice.cancel`, `voice.confirm_add`, `voice.setting_label`, `voice.disclosure_body`,
`voice.disclosure_use`, `voice.disclosure_decline`, `voice.read_order_title`, `voice.read_order_total` — **plus the
5-member** error family `voice.err.{mic_denied, model_offline, no_match, try_again, unavailable}` = **20 total** (the DoD's
"17" was wrong; an implementer sizing to 17 under-adds — F10). Added via `scripts/i18n-add.ts` (en+sq+uk each), must pass
`scripts/i18n-parity.mjs`.

**Decision (F9) — a call-site key-EXISTENCE guardrail, not just parity + per-site fallback.** `translate()`
(`i18n.ts:38`) returns `hit || fallback || key`, and every voice call-site OMITS the fallback → a missing key renders
the literal `voice.retry` to the customer. `i18n-parity.mjs` enforces sq/en/uk *symmetry* (all-three-or-none), NOT that a
call-site key *exists* — and **two families resolve DYNAMICALLY**, invisible to any static scan: `ErrorPill.tsx:42`
`` t(`voice.err.${kind}`) `` (5 kinds from the `VoiceErrorKind` union) and `MicFab.tsx:94` `t(ariaKeyForPhase(phase))`
(`voice.listening/transcribing/applied/err.mic_denied/fab_label`). Fix = **both**, defense-in-depth:
1. **Authority — a guardrail** (unit test / `i18n-catalog` assertion) enumerating the REQUIRED 20 keys (deriving the 5
   `voice.err.*` from the `VoiceErrorKind` union so the two dynamic families are covered) and asserting each exists in
   en+sq+uk. Red→green: delete one key → test fails. This is what a static scan + parity gate both miss.
2. **Graceful degradation — a neutral fallback at the two DYNAMIC call-sites** (`t(\`voice.err.${kind}\`, NEUTRAL_ERR)`,
   `t(ariaKeyForPhase(phase), 'Voice')`) so even a future gap degrades to neutral copy, never a raw dotted key.

### Toast Undo (§7-toast)
`useToast()` exposes `showToast(message, variant)` only — no action/undo button (`Toast.tsx:14`).
`onReadOnlyApplied` (`useVoiceControl.ts:47`) hands the mount `(proposal, result)` so it *can* show "toast + Undo",
but the prior value to restore lives only in the mount's setters and the gate applies synchronously.
- **v1 (chosen): plain confirmation toast** — `showToast(t('voice.applied…'), 'success')`. READ_ONLY changes
  (sort/search/lens/category/compare) are UI-local and **trivially user-reversible** (re-tap the sort chip, clear
  search). ADR-0015 §6 explicitly classes READ_ONLY as "UI-local reversible, no safety assertion." A plain toast is
  adequate.
- **Deferred: Toast action-button API extension** (`showToast(msg, variant, { actionLabel, onAction })` + a snapshot
  of prior state captured in the mount before apply). Recorded as an accepted-risk defer (§10 R5), not built — it is
  a UX nicety, not a safety requirement.

---

## 8. Security + tenant isolation

- **Client-only, no PII, no new network.** In this PR there is **no audio** (MockEngine = scripted text) and **no
  egress**. No cookies, no secrets, no new fetch. The menu context crossing into the matcher is `{id, name}` only
  (`menuContext.ts` — no price/availability/PII). Voice reads only the loaded tenant menu + the user's OWN cart
  (`READ_ORDER`), never a cross-tenant surface.
- **Potemkin-mic — an ASSERTED line, not an assumption (Counsel §1).** `VITE_VOICE_ENABLED=true` is a **staging/E2E
  artifact only**; a real user **never** meets the mock mic. A mic-shaped affordance backed by a scripted no-op is a
  bounded untruth that must never face a public storefront. This is carried by construction, two ways: (1) OFF in prod →
  the whole VoiceMount+mockEngine+injection chunk is DCE'd (§2/§9, F5), and (2) **PR-4 real engine is the precondition
  for any public ON** — the MockEngine (and its transcript-injection seam) is a PR-4 port swap, gone before launch. No
  one flips the flag for a demo and calls it live.
- **ADR-0015 §6 invariants preserved (by construction):**
  - Engine write-incapable — the MockEngine only reports lifecycle events and emits `readonly IntentProposal` data;
    it holds no mutator (the hook calls `gate.submit()`, not the engine).
  - `ConfirmationGate` is the SOLE write sink — built once in `apps/web/src/lib/voice/gate.ts`.
  - **`MenuPage` NEVER imports `@deliveryos/voice` directly** — it imports `VoiceMount` (local) and UI from
    `@deliveryos/ui`; `VoiceMount` imports the adapter (`apps/web/src/lib/voice`) + `@deliveryos/ui`; only the
    adapter imports `@deliveryos/voice`. The adapter is the single boundary.
  - Money/checkout/settling intents fail-closed to REJECT (ledger #63; no `kind`, no handler).
  - Guardrails `no-voice-app-import` / `no-voice-engine-callback` (scoped to `packages/voice/src/**`) stay green — the
    `StatefulPrecondition` added to `ConfirmationGate` is an **object-port** (a `TSTypeReference` param, like
    `handlers: VoiceHandlers`), NOT a `TSFunctionType` param, so it does not trip `no-voice-engine-callback`; it is a
    READ-only predicate that gives the gate no new write capability (the gate stays the sole write sink).
- **New guardrail #1 — engine-purity in the adapter dir (F2, machine-enforced not prose).** A1 puts the engine in
  `apps/web/src/lib/voice/mockEngine.ts`, **outside** the `packages/voice/src/**` scope of `no-voice-app-import` — so a
  future edit (or the PR-4 successor) could `import { useSharedCart }`/an api-client and call `addItem` directly,
  bypassing the `ConfirmationGate`, with **no gate firing** (the ledger-#67 green-by-construction class). Fix: **extend
  the `no-voice-app-import` scope to also cover `apps/web/src/lib/voice/**`** with the same forbidden-set (Cart*
  mutator, `api-client`, fetch-client packages). Verified no false-positive on the existing adapter files — none import a
  Cart mutator or api-client directly (`handlers.ts`/`gate.ts` use INJECTED `deps`; `types.ts` imports only the `CartItem`
  *type* from `@deliveryos/ui`, whose specifier is not a `Cart*` module). Proof: a fixture `…voice-app-import…mockEngine`
  that `import`s `../CartProvider`'s `addItem` fires RED; remove the import → GREEN (a real violation, not
  green-by-construction).
  - **Scope of this gate is corrected (R-c) — it is a SPECIFIER gate, NOT a capability gate.** It machine-covers the
    **canonical import path** (a `Cart*` mutator / `api-client` / fetch-client *import* under `apps/web/src/lib/voice/**`,
    incl. `mockEngine.ts`) — genuinely red→green and worth keeping. It does **NOT** cover a bare capability that needs no
    import: `fetch('/api/orders', {method:'POST'})` uses the global `fetch` and slips through. **In THIS dark PR that gap
    is INERT** — the MockEngine does **zero** network (`+0 external calls`, §2), so there is no fetch sink to hit and
    nothing to bypass to. **But PR-4's real engine is network-capable** (it fetches `/api/public/voice-config` + the R2
    model), so a PR-4 successor could add a `fetch`-based write sink the specifier gate misses. **PR-4 X-blocker (not a
    this-PR blocker):** PR-4 needs a compensating *capability* control — route all voice egress through a single
    allowlisted api-client (ban raw `fetch` in `apps/web/src/lib/voice/**` via an eslint `no-restricted-globals`/
    `no-restricted-syntax` rule) and/or a CSP `connect-src` allowlist / network guard. Recorded §10 R14.
- **New guardrail #3 — the production stateful sink MUST wire a precondition (R-a(2), fail-closed by machine).** The
  fail-closed gate semantics (§6 seam #3, `?? false`) mean a `createVoiceGate` that forgets the precondition doesn't
  silently *permit* — it *refuses ALL* STATEFUL. That is caught by a **red→green unit on the production factory**: assert
  `createVoiceGate({...deps, isOrderingDisabled: () => false})` (open venue) → `ADD_TO_CART` `submit()` returns
  `pending-confirm` → `confirm()` returns `applied` + `addItem` called; and `createVoiceGate({...deps, isOrderingDisabled:
  () => true})` (closed) → `submit()` returns `rejected`, no pending, `confirm()` → `rejected`, `addItem` NOT called. **Red
  fixture:** revert `createVoiceGate` to `new ConfirmationGate(createVoiceHandlers(deps))` (drop the precondition) → the
  OPEN-venue arm FAILS (add no longer works, fail-closed refuses) → RED; restore → GREEN. This proves the factory wires
  the precondition; a bare eslint "≥2 args to `new ConfirmationGate` in `apps/web/src/lib/voice/**`" rule is an optional
  structural backup (it cannot prove the arg is a *real* precondition, so the factory unit is the authority).
- **New guardrail #2 — pages reach voice only via the adapter (R7).** `no-voice-engine-import-outside-adapter` bans
  `@deliveryos/voice` imports under `apps/web/src/pages/**` and anywhere in `apps/web` except `apps/web/src/lib/voice/**`,
  proven red→green. (Distinct invariant from #1: #2 stops a page reaching the engine specifier; #1 stops the engine dir
  reaching a mutator. Both are needed — F2.) Prose is not a gate.

---

## 9. Operability

- **Dark flag + kill path.** `VITE_VOICE_ENABLED` build-arg, default OFF. OFF → statically pruned (true dark; §2).
  For the dark mount there is **no runtime instant-kill needed** — it is never on in prod. When we LAUNCH (future
  PR), ADR-0015's SW-exempt runtime hot-kill (`/api/public/voice-config`, `cache:'no-store'`, fail-closed) + the
  `VOICE_KILL` predicate + the single-flag CSP R2 widening are REQUIRED (the cache-first SW makes a build-flag alone
  fail-open for returning visitors). Recorded as a launch-gate dependency, not this PR.
- **Health degraded-vs-down.** Voice failure = "voice off," never storefront-down. No health endpoint changes; no
  new liveness surface. There is nothing new that can be "down."
- **Observability (<1 min).** Client-only; no server telemetry added (ADR-0015 requires any future counter be
  actor-anonymous by test — none here). Dark-state is directly observable and now genuinely provable (F5): with the
  `import()` gated in the dead ternary arm (`VoiceMount = VOICE_ENABLED ? lazy(() => import('./VoiceMount')) : null`), an
  OFF build (`VITE_VOICE_ENABLED` unset) emits **no** voice chunk — a bundle assertion greps `dist/assets/**` for a
  voice-unique marker (e.g. the `VoiceMount` chunk name / a `data-testid="voice-fab"` string) and asserts absence. E2E
  asserts the MicFab renders only when ON.
- **E2E injection seam — gated on `VITE_VOICE_ENABLED`, not `DEV` (F6).** The DoD requires Playwright on **staging**,
  which runs a production Vite build (`import.meta.env.DEV === false`) — so a `DEV`-gated transcript-injection seam is
  **dead on the one env the proof requires**. Fix: gate the seam on `VITE_VOICE_ENABLED` (live on staging where the flag
  is set, DCE'd in prod with the rest of the voice chunk). The seam additionally reads an inert test channel
  (`window.__VOICE_E2E__`, set by the Playwright fixture) so it is a no-op even on staging unless a test drives it.
  "Dead in prod" is doubly held: (1) the whole chunk is eliminated when the flag is off, and (2) at launch the
  MockEngine+seam is swapped for the PR-4 real engine.
- **Mock-green is WIRING-proof, not product-proof (Counsel §1 / non-blocking ask).** A Playwright pass against the
  deterministic MockProvider proves the plumbing + gate wiring — it proves **nothing** about whether a real person
  speaking Albanian into a real mic is understood (that is the separate human-mic Phase-0 feasibility gate). This label
  travels on the mount E2E so a green suite never manufactures false "voice works" confidence that softens the demand
  gate.
- **Rollback.** Flag-flip OFF, or revert the mount commit. **Zero data change** → instant, riskless rollback.
- **Scaling-gate.** +0 connections/workers/endpoints → no gate impact.

---

## 10. Open / accepted risks (each with owner)

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R1 | Mid-session venue-close vs. a pending add (R-b: two residuals, honestly separated). | **Point-of-action FIXED (F1/F3, §6):** LIVE `isOrderingDisabled()` getter; gate refuses STATEFUL at `submit()` (no chip), re-checks at `confirm()` (absent⇒refuse), and requires a real mutation (R-a(4)) — honest "Done" only on real apply; gate built ONCE (no rebuild → `#pending` never dropped). The re-render window between submit/confirm is caught by the confirm re-check. **Session-long wall-clock staleness NOT closed client-side — DECISION (b), downgraded to PARITY:** `venueStatus` is a page-load snapshot with no re-poll (`MenuPage.tsx:458-472`), so a time-based close with the tab open keeps `orderingDisabled===false` all session — but the tap path reads the same snapshot, so voice is at **exact parity**, and the **server (#65) is authoritative** at checkout. Client re-derivation from `storeHours`+clock REJECTED (duplicates a server-authoritative status → red-line + divergence risk). **Counsel WATCH-LINE re-eval:** parity with the incumbent honest-UI posture does **NOT** re-trigger the honest-UI ethical-stop — the *over-claim* was the hazard, not the parity; the corrected "parity, server-authoritative" framing is ethically clean. **Accept parity residual;** a real liveness fix (re-poll shared `venueStatus`, fixes BOTH paths) is a separate `MenuPage`-hotspot change. | implementer (point-of-action) + human/product (liveness fix + Counsel sign-off on downgrade) |
| R2 | E2E transcript-injection seam must be live on staging yet dead in prod (F6). | **Design:** seam gated on **`VITE_VOICE_ENABLED`** (live on staging, DCE'd in prod) + reads an inert `window.__VOICE_E2E__` channel; PR-4 real engine replaces it before launch. **Accept.** | implementer |
| R3 | Bundle delta when ON (~20–30 KB gzip lazy). | Non-blocking, post-paint, only when flag ON; genuinely 0 KB when OFF via the gated `import()` (F5). **Accept.** | implementer |
| R4 | No demand evidence (ADR-0015 Counsel §5). | This PR is DARK and does not launch; launch stays gated on the human demand decision + B1/B2/B3. **Defer to human.** | human |
| R5 | Toast Undo deferred (v1 plain toast). | READ_ONLY changes are user-reversible; ADD_TO_CART is confirm-gated (the human tap IS the undo point) — the defer never touches a STATEFUL apply (Counsel non-blocking). **Accept-risk / defer-flag.** | product |
| R6 | Real ASR + `/api/public/voice-config` hot-kill + CSP R2 widening not in this PR. | Required ADR-0015 exit criteria before ON-in-prod; explicitly out of scope. **Defer-flag.** | PR-4 + human |
| R7 | The "pages reach voice only via the adapter" invariant was prose. | **Fixed:** `no-voice-engine-import-outside-adapter` eslint rule, red→green (§8 guardrail #2). | implementer |
| R8 | The MockEngine dir sat outside engine-purity jurisdiction (F2). | **Fixed for the import path (SPECIFIER gate, R-c):** `no-voice-app-import` scope extended to `apps/web/src/lib/voice/**`, red→green fixture (§8 guardrail #1) — catches the canonical `Cart*`/api-client *import*. **NOT a capability gate:** a bare `fetch('/api/orders',{POST})` needs no import → slips through; INERT in this PR (MockEngine does zero network, §2). PR-4 network-sink control tracked at R14. **Fix (import path) + defer (capability, PR-4).** | implementer |
| R9 | The pull→push bridge could multi-yield / wedge / swap handlers on abort / throw (F4/F7/F8). | **Fixed:** the §3a-contract binds one-proposal-per-session, no-match→`onNoMatch`, watchdog→`onError`, total try/catch, closure-captured handlers/abort — applies to the mock NOW and the PR-4 real engine. **Fix.** | implementer |
| R10 | A missing `voice.*` key (esp. the two dynamic families) shows a raw dotted key (F9). | **Fixed:** call-site key-existence guardrail (authority) + neutral fallback at the 2 dynamic call-sites (degradation), red→green (§7-keys). **Fix.** | implementer |
| R11 | Potemkin-mic — a listening-looking mock mic must never face a public user (Counsel §1). | **Asserted line (§8):** `VITE_VOICE_ENABLED=true` is staging/E2E-only; PR-4 real engine is the precondition for any public ON; carried by the true-dark DCE + R2-A fail-closed config. **Accept (asserted, not assumed).** | human + PR-4 |
| R12 | Round-1 made `StatefulPrecondition` OPTIONAL ("absent = allow") — a silent honest-UI bypass at the sole write sink (`gate.ts:14` builds the gate with no precondition) (R-a). | **Fixed — REVERSED to FAIL-CLOSED:** `submit()`/`confirm()` compute `precondition?.canApplyStateful() ?? false`, so an absent precondition REFUSES STATEFUL (no chip, no "Done", no apply) exactly as a false one does. Omission is loud (all adds refused → happy-path unit RED), never a silent permit. **Machine-gated** by a red→green unit on `createVoiceGate` (§8 guardrail #3); the closed-venue DoD test goes through `createVoiceGate(deps)`, not the raw constructor (R-a(3)). | implementer |
| R13 | `confirm()` reported `applied` even when `addToCart` was a no-op (unresolved/unavailable/requires-modifiers) → dishonest "Done ✓" via a non-venue path (R-a(4)). | **Fixed:** `#apply` returns whether `addItem` actually fired; `confirm()` returns `applied` only on a real mutation, else `rejected('apply-noop')` → hook → neutral pill, no "Done". Milder READ_ONLY submit-time over-report is an accepted residual (no "Done ✓" pulse, `onNoMatch` fires) — see below. | implementer |
| R14 | The engine-purity guardrail is a specifier gate, not a capability gate — a `fetch`-based write sink needs no import and slips through (R-c). | INERT this PR (MockEngine = zero network, §2). **PR-4 X-blocker:** the network-capable successor needs a compensating capability control — single allowlisted api-client + ban raw `fetch` in `apps/web/src/lib/voice/**` (eslint `no-restricted-globals`/`no-restricted-syntax`), and/or CSP `connect-src` allowlist / network guard. **Defer-flag (PR-4), NOT a this-PR blocker.** | PR-4 + human |
| R12b | READ_ONLY submit still returns `applied` on a no-op (e.g. unmapped sort) even though nothing changed. | **Accept residual (R-a(4) scoped to STATEFUL "Done"):** READ_ONLY shows no "Done ✓" pulse and the handler already fires `onNoMatch` (neutral copy); uniform apply-outcome honesty for READ_ONLY would change existing READ_ONLY submit-test expectations — deferred as a consistency nicety, not a safety gap. | product |

### Explicit true-dark statement (mandatory)
**When `VITE_VOICE_ENABLED` is unset (or `!== 'true'`), the mount renders NOTHING** — no MicFab, no hook, no gate,
no engine, no adapter evaluation, no `@deliveryos/voice` import — and the voice lazy chunk is **not emitted** into
the storefront bundle, because the dynamic `import('./VoiceMount')` lives in the DEAD arm of the constant-folded
ternary `VOICE_ENABLED ? lazy(() => import('./VoiceMount')) : null` (F5 — a module-scope `lazy()` guarded only at
render would NOT achieve this). **Zero regression on the storefront hotpath: 0 KB, 0 render cost beyond one boolean
short-circuit.** The OFF-build bundle assertion (§9/DoD) is the proof.

### STOP-DESIGN-B (human decision — do NOT decide in design; Counsel §5 open question)
**Before this mount lands, the human must pre-commit — now, while unattached — two conditions, and record them:**
(1) the condition under which voice is **flipped ON in prod** (a real, ranked demand signal weighed against the first
paid order, per ADR-0015 §5 / R-J), and (2) the condition under which the mount is **deleted-if-unmounted** (a
time/decision box after which an unlaunched voice mount is removed rather than left to accrete momentum). Rationale
(Counsel): a mounted 95%-built feature stops being psychologically speculative and re-frames voice from *demand-gated*
to *in-flight*; reversibility of bytes is not reversibility of organizational momentum. The mount is safe; the residual
weight is the mount's gravity against the gate it is deferred behind — a **human** call, not the architect's. This item
gates landing the mount. Owner: **human/operator.**

---

## Definition of Done (exit checklist for the implementing PR)
- [ ] `apps/web/src/lib/voice/mockEngine.ts` implements the UI `VoiceEngine` port (start/abort) backed by the
      `@deliveryos/voice` matcher, satisfying the **§3a-contract**: one-proposal-per-session, no-match→`onNoMatch`,
      watchdog→`onError('try_again')`, total `try/catch`, closure-captured handlers+abort. Unit red→green for each:
      multi-yield→one apply; unmatched transcript→`onNoMatch` (not a silent spinner); no-terminal→watchdog `onError`;
      thrown body→`onError`; barge-in→no cross-session mutation. Injection seam gated on **`VITE_VOICE_ENABLED`** + inert
      `window.__VOICE_E2E__` (F4/F6/F7/F8).
- [ ] `VoiceStorefrontDeps.orderingDisabled` is a LIVE getter `isOrderingDisabled(): boolean`. `ConfirmationGate` takes a
      READ-only `StatefulPrecondition` object-port, **FAIL-CLOSED (R-a):** `submit()`/`confirm()` compute
      `precondition?.canApplyStateful() ?? false`, so an **absent** precondition REFUSES STATEFUL (no chip / no apply)
      exactly as a false one does — never a silent permit. `confirm()` also requires a **real mutation** — `#apply` returns
      whether `addItem` fired; `confirm()` returns `applied` only then, else `rejected('apply-noop')` (R-a(4)).
      `useVoiceControl.confirm()` consumes the result and only dispatches `CONFIRM` on `status==='applied'` (else
      `ENGINE_ERROR('unavailable')`). `addToCart` + `navigateCheckout` check `isOrderingDisabled()` (F1/F3/F11).
      **These closed-venue units MUST construct via the production factory `createVoiceGate(deps)`, NOT the raw
      `new ConfirmationGate(handlers, precondition)` constructor (R-a(3))**, so the wiring gap is proven. Unit red→green:
      **open venue → `ADD_TO_CART` submit `pending-confirm` → confirm `applied` + `addItem` (factory wired the
      precondition)**; **closed venue add → no chip, no "Done", no `addItem`**; **`createVoiceGate` with NO precondition →
      open-venue add REFUSED (fail-closed machine gate, §8 guardrail #3) RED**; **venue closes after pending → confirm → no
      `addItem`, FSM not `applied`**; **product goes unresolvable/unavailable/requires-modifiers between propose & confirm →
      confirm → no `addItem`, gate returns non-`applied`, FSM not `applied` (apply-noop honesty, R-a(4))**; **re-render
      between propose & confirm → `#pending` survives → confirm adds**; **closed-venue checkout-nav → no `?checkout=1`**.
      Existing STATEFUL-apply gate tests migrated to pass a satisfied precondition (they relied on the removed permissive
      default) — READ_ONLY/REJECT tests unchanged.
- [ ] `VoiceMount.tsx` builds ONE stable `deps` (latest-ref delegation) + ONE `gate` (**never rebuilt**) + engine, renders
      the UI, wires `onNavigateCheckout → ?checkout=1` (gated), `onReadOnlyApplied → showToast`, persists `voicePref`.
- [ ] `MenuPage.tsx` gains ONE guarded line with the gated `import()` (`VoiceMount = VOICE_ENABLED ? lazy(() => import(...)) : null`);
      **OFF build emits no voice chunk** — bundle-assertion greps `dist/` for a voice marker and asserts absence (F5).
- [ ] **20** `voice.*` keys (15 named + 5 `voice.err.*`) in `i18n-catalog.ts` (en+sq+uk); `i18n-parity.mjs` green; **plus
      a key-existence guardrail** enumerating all 20 (incl. the dynamic `voice.err.${kind}` + `ariaKeyForPhase` families,
      deriving the 5 err kinds from `VoiceErrorKind`) red→green; neutral fallback at the 2 dynamic call-sites (F9/F10).
- [ ] Guardrails: **`no-voice-app-import` scope extended to `apps/web/src/lib/voice/**`** (SPECIFIER gate — import path
      only, R-c) with a real red→green fixture (F2/#1); **`no-voice-engine-import-outside-adapter`** red→green (R7/#2);
      **factory-wiring unit on `createVoiceGate`** — open-venue add works, closed-venue add refused, precondition-omitted →
      RED (fail-closed machine gate, R-a(2)/#3); `no-voice-engine-callback` / `capability-table` stay green (the object-port
      `StatefulPrecondition` does not trip them).
- [ ] **Staging deploy MUST pass `--build-arg VITE_VOICE_ENABLED=true` (F6 ops-dependency).** Per the
      staging-deploy-flags class (`flyctl deploy` bakes VITE flags at build time; an unset flag defaults OFF), a staging
      build without this build-arg DCE's the whole voice chunk on staging too — the MicFab never renders and the E2E
      **silently cannot run** (green-because-absent, a false-green). The deploy command + a pre-E2E assertion that the
      MicFab is present (`voice-fab` visible) gate the proof; the OFF-build bundle-absence assertion (F5) is a SEPARATE
      prod-parity build, not the staging proof build.
- [ ] Mandatory Proof: Playwright E2E on staging (`VITE_VOICE_ENABLED=true`), labeled **WIRING-proof, not
      product-proof** — mic tap → scripted transcript → read-back/confirm → cart line `toBeVisible`; **closed-venue arm**
      → no confirm chip, no "Done", no cart line; **disclosure-DECLINE arm** → "Not now" leaves mic unactivated + touch
      fully working, and the two choices are visually equal weight (Counsel C-2; `equal-affordance.test.ts` covers the
      styling, this covers the function). Plus voice unit (58/58 retained + the new red→green cases; STATEFUL-apply units
      migrated to supply a precondition) + whole-repo typecheck. Ledger row added (behavior change).
- [ ] **PR-4 X-blocker recorded (NOT gating this PR):** the network-capable real engine needs a compensating capability
      control for the fetch-sink gap the specifier gate misses (single allowlisted api-client + ban raw `fetch` in
      `apps/web/src/lib/voice/**`, and/or CSP `connect-src` allowlist) — §8 guardrail #1 / §10 R14.
- [ ] **STOP-DESIGN-B answered by the human** (flip-ON condition + delete-if-unmounted condition recorded) BEFORE the
      mount lands.
