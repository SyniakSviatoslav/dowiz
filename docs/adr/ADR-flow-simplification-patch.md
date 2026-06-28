# ADR — Flow Simplification Patch

**Status:** Proposed → **RESOLVE-hardened (round 3)** — converged design-level, 1 NEEDS-HUMAN (§3); design-only
**Date:** 2026-06-28
**Deciders:** Triadic Council (Architect seat authored; Breaker + Counsel hardened; RESOLVE rounds 1+2+3 folded in)
**Supersedes / relates:** ADR-0004 (owner-token revocation, role re-derive), ADR-0005 (delivery-fee
mirror + cash-422), ADR-deliver-v2-cash-as-proof (order machine edges, no-trap folds),
P6 claim council verdict (`docs/design/p6-claim-council-verdict.md`).
**Proposal:** `docs/design/flow-simplification-patch/proposal.md`.

## Context

The product converged on a single runtime shape: **cash-only payment, delivery-only order-type,
operator-provisioned onboarding.** The UI still carries structure built for choices that no longer
exist at runtime (order-type switch, time-slot/promo affordances, a multi-tap kitchen lane, a
self-serve onboarding funnel, a 3-surface checkout). The patch removes that structure and collapses
sequences into one surface where the page transition adds nothing — **without deleting the foundation
seams** that let the deferred capabilities return.

Grounding established the decisive facts:
- The "cart" is already a bottom-sheet `ResponsiveDialog` over the menu (`ClientLayout.tsx:159–223`),
  not a page; only `/s/:slug/checkout` is a separate route. The panel-over-menu primitive exists in
  **both** normal and embed mode.
- `CreateOrderInput.type` is `z.enum(['delivery','pickup'])` **required, no default**
  (`legacy.ts:42`); promo/time-slot/scheduled are **not in the create contract at all**.
- The order machine already permits `CONFIRMED → IN_DELIVERY` (`order-machine.ts:20`); the owner's
  manual READY tap is an optional kitchen lane, not a gate.
- The P6 claim backend is **shipped** (routes `claim.ts`, `acceptClaim`/`claim_transfer`
  SECURITY-DEFINER, migration `…071`) with **zero web-UI callers** — the owner claim surface is the
  only missing piece.

## Decision

Six resolutions, plus the §6 reconciliation stance.

1. **§1 Checkout = panel over the menu (2 surfaces).** Reuse the existing `ResponsiveDialog`
   bottom-sheet; the cart bar opens the checkout panel directly; cart contents become a
   collapsed-by-default summary at the top. Retire `/s/:slug/checkout` to a **redirect seam** (opens
   the menu with the panel open) to preserve deep links / refresh-continuity. **Close = close + cart
   intact** (no trap). Normal == embed by construction.

2. **§2 Cart bar = item count + running items-subtotal** *(RESOLVE: reversed from "count only").* Counsel's
   cash-as-proof steel-man (§9) is decisive — the running subtotal is *signal, not chrome*: the instrument a
   cash-constrained customer uses to shop within the money they can produce at the door. Cut the cart
   *drawer* (scaffolding), keep the **items subtotal** on the bar (always-known, integer, server-mirrorable,
   no `feeKnown=false` ambiguity because it excludes the fee). The **all-in total** (items + delivery fee)
   resolves in the panel — "+ delivery fee at checkout" when distance-tier-unknown, exact otherwise. Server
   total + cash-422 remain authoritative.

3. **§3 Information floor (RESOLVE: R3 decided — optional-but-inviting).** One surface; INFO does not
   collapse. Required floor = **phone + a droppable location (map-pin primary OR text fallback)**;
   entrance/apartment/notes **optional but contextually present** (inline, clearly-optional placeholders —
   **never hidden behind a "more" toggle**), a **client-side** relaxation (server already tolerates their
   absence). Counsel §3: "optional must mean skippable, never hidden" — the demotion must not load the
   least-served customer (phone/language/hearing/buzzer) with the one interaction they can least afford.
   **Rejected, recorded:** address-on-menu; per-item buy-now. New ≈5 actions (when no preflight-ack fires),
   repeat ≈1–2. **RESOLVE-2: the floor is a THREE-way NEEDS-HUMAN** (hard-required | optional-but-inviting |
   **contextually-required, pin-confidence-gated**). Architect recommends **contextually-required** — required
   on a low-confidence pin (multi-unit/area geocode), optional on a high-confidence one — friction proportional
   to real last-50-metre risk; **server-tolerant, no contract change.** Product/ops ratify the rule + the
   pin-confidence threshold; build does not advance on the field until ratified.

4. **§4 Order-type/time-slot/promo removed from customer UI, kept in foundation.** Add
   `type: z.enum(['delivery','pickup']).default('delivery')` to `CreateOrderInput` (additive,
   forward-only, **no migration**) so omitting the field defaults to delivery instead of 400-ing; the
   client also keeps sending `'delivery'` for requestHash determinism. Pickup stays fully supported
   server-side. Promo and time-slot need **no contract work** (never in the contract) — pure UI removal.

5. **§5 Owner 2 taps → 1.** UI-only: when `status==='CONFIRMED'`, `OrderCard` shows **"Send for delivery"**;
   drop the standalone "Mark Preparing"/"Mark Ready" from the default card. **No state-machine change, no
   migration.** PREPARING/READY remain valid states. The READY toggle is a **deferred location-level seam** —
   add `locations.kitchen_flow_enabled` additively **only when the toggle UI is built**, not speculatively.

   **§5 RESOLVE / F1 (no-trap — RESOLVE-2 re-grounded):** the round-1 "dispatch primitive returns
   `{dispatched:false}`" was **fictional** — `/assign-courier` (`dashboard.ts:214`) requires an explicit
   `courierId` (`:219–222`) and **409s on IN_DELIVERY** (`:235–237`); it never auto-discovers. The only
   auto-discovering path is the raw PATCH `/orders/:id/status`, which is **the only endpoint the card's single
   `onUpdateStatus` callback reaches** and which flips to IN_DELIVERY **unconditionally first** (`orders.ts:779`)
   then leaves a silent orphan on no courier (`:786–800,824`). (a) **Make that endpoint honest:** run the
   courier lookup **before** the status flip; courier found → IN_DELIVERY + assign; **none → stay at current
   status**, return `{status:<current>, dispatched:false, reason:'no_courier'}` (server-authoritative — the real
   status), card shows "awaiting courier". This is **flag-independent** (the card never touches
   `/assign-courier`, so the no-orphan property holds regardless of `COURIER_OFFER_HANDSHAKE_ENABLED`; all four
   `OWNER_TWO_TAP`×handshake cells non-orphaning — matrix in `resolution.md` R2-2). **RESOLVE-3 / R3-2: the
   dispatch is `'offered'`-aware** — the mig-073 partial uniques police `'offered'` (`…073:22–24,32–33`) but
   the lookup excludes only `('assigned','accepted','picked_up')` (`orders.ts:792–794`); so (i) **already-bound
   guard** (any active binding → no-op + `offer_pending`/`already_assigned` signal, never a 2nd INSERT) and
   (ii) **exclude `'offered'` from the lookup** — else "Send for delivery" on a pending-offer order INSERTs a
   conflicting active row → 23505 → uncaught 500. (b) **IN_DELIVERY recovery REUSES the shipped asymmetric
   `releaseBindingAndReoffer`** (RESOLVE-3 / R3-4 — corrects round-2's raw `onUpdateStatus('READY')`): the
   central fold `orderStatusService.ts:129–140` blanket-cancels the binding regardless of `asg_status`, so a
   raw revert on a `picked_up` binding sets the order READY **with the food out** — contradicting the shipped
   `/abort` (`bindingRelease.ts:37–40` sends `picked_up`→CANCELLED). The owner recovery loads the active
   binding and calls `releaseBindingAndReoffer`: pre-pickup (`assigned`/`accepted`) → READY + cancel +
   re-enqueue (re-dispatchable); `picked_up` → **CANCELLED, never READY**. Re-assign **409s** on IN_DELIVERY so
   it is NOT the recovery. **RESOLVE-3 / R3-3:** the awaiting-courier re-dispatch path = the **owner re-tapping
   "Send for delivery"** once a courier is online (the §5 endpoint is pull-based and never enqueues
   `courier_dispatch_queue`; the async worker is off this path and currently broken — `courier-dispatch.ts:76
   this.boss` → DEFER-FLAG). An order **never reaches IN_DELIVERY with no courier and no recovery, and the
   recovery never lies on `picked_up`, under every flag cell.** Guardrails G-F1a + G-F1a-2 (already-bound no-op,
   not 500) + G-F1b-i/ii (asymmetric recovery) + G-F1c (awaiting→re-tap loop closes), red→green.

   **§5 RESOLVE / F3+F4+F5 (RESOLVE-2 re-decided — honest signal without a fabricated timestamp):** the round-1
   `preparing_at`-auto-stamp is **dropped** — stamping a physical "kitchen started" event that didn't happen is
   a data-layer lie (`preparing_at` feeds `fetchOrderDelta`/`OrderProgress`) and starts the ETA decay at
   accept. Instead: `preparing_at` stays NULL (honest); drive the **ETA off a REAL, always-non-NULL base**
   `prepRemaining = max(0, prep − minutesSince(COALESCE(preparing_at, confirmed_at, created_at)))` (RESOLVE-3 /
   R3-5 — round-2's "decay off `confirmed_at`" was **unplumbed**: `etaGather.ts:192` selects `created_at`/
   `preparing_at` only, and a directly-CONFIRMED order may have NULL `confirmed_at`; so add `confirmed_at` to
   the SELECT + arg, with `created_at` as the always-non-NULL floor) (hard-zero only on `ready_at`/real
   pickup — fixes `etaGather.ts:81`); render the **progress "Preparing" step as an active
   PROCESS label** ("Preparing your order"), ✓ only when a real timestamp is non-NULL (fixes
   `OrderProgress.tsx:84–102`). `kitchenAhead` is grounded-clean (SUMs only `status='PREPARING'`, which 2-tap
   orders never are — `etaGather.ts:101`). Guardrails G-F3 (decay off `confirmed_at`), G-F5. **DEFER-FLAG:** no
   live consumer learns `ready_at − preparing_at` (`etaGather.ts:184–217`); a future duration-learner **must
   filter `ready_at IS NOT NULL` and `preparing_at IS NOT NULL`** and resumes with the kitchen-flow toggle.

6. **§6 Onboarding → CLAIM = a surface/activation delta over the SHIPPED P6 vertical, NOT a rebuild.**
   Build only: the owner claim **web surface** (token → working preview → light-edit items/prices/theme/
   radius, **no** full palette/layout/zone editors → call shipped `/claim/accept`), the
   equally-prominent **decline** (`/claim/decline`), and the **K4 approve writer** if unbuilt. **Reuse,
   do not duplicate, `claim_transfer`.**

   **§6 reconciliation (the contradiction resolved):** the patch's "one action … goes live" is
   **revised**. Claim is one action that **takes ownership + binds owner login** (what `acceptClaim`
   already does). **Go-live is a separate, gated act** (review menu → confirm allergens into empty
   fields → publish), per the shipped council bindings **CC2 (claim→review→publish stays three acts)**,
   **CC3 (allergen confirmation is a distinct deliberate act)**, and **H-publish-coupling**
   (`published_at` stays NULL through claim; publish only via the gated activation path). The owner gets
   a **working, demoable** service immediately (the patch's real value); **public orderability** waits
   on the gate. This preserves the never-orderable B3 invariant and the AI-allergen safety architecture.

   **§6 RESOLVE / F2 (claim-token transport — protect the surface, not `claim_transfer`):** the API is clean
   (token in **body**), but a `/claim?token=` web surface would leak the 256-bit sole-authority token via
   URL → history → **Referer** (map/font/analytics third parties) → **CDN/access logs** within the 72h TTL,
   re-opening ownership-theft + griefing-erase. The surface MUST: (1) deliver the token in the URL
   **fragment** (`/claim#t=…`) and immediately `history.replaceState` to scrub it **before any third-party
   resource loads**, holding it **in-memory only** (never localStorage/sessionStorage; cookies already
   forbidden); (2) resolve the preview via a **read-only token-in-body** POST (public shadow data only,
   no-enumeration); (3) **authenticate the owner IN-PAGE (no navigation)** — RESOLVE-2: auth is zero-cookie
   JSON-token so the SPA registers/OTPs **with the invited email** by **fetch, not navigation**, preserving the
   in-memory token; then POST `{token}` in the **body** to the shipped `verifyAuth` `/claim/accept`; (4) keep
   **decline** no-auth token-in-body (H-decline), now leak-free.

   **§6 RESOLVE-3 / F2 recipient binding (round-2 OVERSTATEMENT corrected + web NULL-hash refusal):** the
   invite carries `invited_contact_hash` (`…071:27`, **nullable**); `claim_transfer` enforces
   `sha256(lower(trim(users.email)))==invited_contact_hash` → `CONTACT_MISMATCH`/403 **only when non-NULL**
   (`…071:64–68`, explicit *"Token-only when invited_contact_hash IS NULL"*). **Honesty correction:** round-2
   called this *"proof of control of the invited email"* — **overstated.** It is a **string compare** against
   the claimer's `users.email`, and the auth path (`local.ts:88–108`) does **no email-ownership verification**
   (OTP off), so it proves only that the claimer **registered under that email string** — defeatable by
   registering the restaurant's scraped public contact. A **speed-bump, not identity proof.** **(a) FIXED on
   the web surface:** the net-new preview + accept paths **refuse a token-only (NULL-hash) invite** (generic
   "link no longer valid", no enumeration) → the *"binds any authed account"* theft is **unreachable via the
   web path**; the operator mint still MUST supply `invitedContact` (G-F2d binds at mint, **G-F2g** enforces at
   the surface). **`claim_transfer` UNTOUCHED.** **(b) ACCEPT-RISK + DEFER-FLAG:** the email-ownership gap is a
   **pre-existing P6/auth weakness**, not introduced here (vertical dark; delta does not worsen it). The real
   fix — **email-ownership verification / OTP before claim** — is **out of scope → DEFER-FLAG to the P6/auth
   owner.** Residual: F2's web surface rests on no-leak (transport) + real-contact-mint (G-F2d) + the
   **deferred** unverified-email gap. **RESOLVE-3 / R3-6:** decline-erase is destructive and token-only for the
   72h channel life (transport protects only on-page) → DEFER-FLAG a shorter decline TTL / soft-delete grace to
   the P6/acquisition owner. Scrub-before-init (R2-6): `/claim` excluded from telemetry + the fragment scrub is
   the first pre-init statement. Guardrails G-F2a/b/c (token never in URL/hash/Referer/query; body-only) + G-F2d
   (CONTACT_MISMATCH active) + **G-F2g (web refuses NULL-hash; `owner_id` stays NULL)** + G-F2e (claim completes
   with no navigation + no storage) + G-F2f (scrub before telemetry init), red→green.

   **§6 RESOLVE / Counsel binding condition — PROTECTED FRICTION:** the council records concurrence with the
   **revised** ADR §6 **only** — the proposal's original "one action … goes live" prose is **NOT a build
   source.** The claim → review → publish three-act sequence (CC2) and allergen confirmation as a distinct
   deliberate act into empty fields (CC3) are annotated **PROTECTED FRICTION** — friction that exists for
   **consent and allergen safety, explicitly distinct from the incidental cart/page friction this patch
   removes** — so a future simplification pass never mistakes the consent gate for scaffolding. **RESOLVE-2:
   this lands as CODE, not prose** — a named in-code marker at the claim/activation seam + **build-time
   guardrails G-PF1** (`published_at` NULL through claim; activation requires `menu_confirmed_at`) and **G-PF2**
   (allergen confirmation a distinct authenticated act writing only into empty fields, never auto-confirming an
   AI guess) — so collapsing the gate trips a deterministic red. Surface dignity (binding the implementer): **CC1** the honest
   Art-14 notice dominates the **first** screen, the working preview comes **second** (preview-before-notice
   would launder the consent); **H-decline parity** — decline equally prominent + account-free
   (claim-louder-than-decline is the dark-pattern tell the P6 verdict named).

## Red lines preserved

- **Order contract / state-machine:** §4 additive default only; §5 reuses an already-legal edge — no
  migration, no machine change.
- **Money (cash-only, integer):** untouched; §1/§2 only relocate where the form renders.
- **Claim ownership-transfer (RLS / SECURITY-DEFINER):** §6 surface **calls** `acceptClaim`; **no** new
  inline ownership UPDATE; token = sole transfer authority (IDOR-closed); org/location derived from the
  matched invite; `/claim/accept` stays `verifyAuth`-only; role re-derives post-claim (ADR-0004).
  **RESOLVE/F2:** token transport = fragment + immediate scrub, in-memory, **body-only** — never
  URL/Referer/log/query.
- **No-trap states:** §1 panel close = close + intact cart (no order pre-confirm) **+ RESOLVE/F6** browser
  Back closes the panel and stays on the storefront; **RESOLVE/F1** dispatch keeps the order CONFIRMED on
  no-courier (no IN_DELIVERY orphan) + an IN_DELIVERY recovery branch; deliver-v2 folds intact; removing
  order-type cannot 422 (the default guarantees it).
- **Embed/normal merge:** §1 **removes** the divergence rather than adding one.
- **Server-authoritative / money-integer:** §2 keeps only the *items* subtotal on the bar (a
  server-mirrorable display); the all-in total + cash-422 + state machine stay authoritative; §5 ETA reads
  server timestamps. No math changed.
- **i18n (al/en):** new/changed strings (subtotal label + "fee at checkout", "Send for delivery" /
  "awaiting courier" / reassign, optional-field placeholders, claim page + Art-14 notice) ship with al/en
  parity via `scripts/i18n-add.ts` + the parity gate.

## Consequences

**Positive:** customer new-order ≈ -3 actions, repeat ≈ -2; owner order-to-courier -2 taps; one fewer
storefront route; normal==embed; onboarding becomes "open a working service" by reusing a shipped
backend. No new load, no new external calls, no new tables (except a possibly-already-shipped K4 writer).

**Negative / trade-offs accepted (RESOLVE-updated):** the cart *drawer* is cut but the running **subtotal
stays on the bar** (Counsel cash-as-proof reversal); orders can reach IN_DELIVERY before food-readiness on
the 2-tap path **only via an accepted dispatch** (no-courier keeps CONFIRMED; READY toggle re-inserts the
beat; customer ETA/progress now timestamp-honest); pickup loses its self-serve entry until re-enabled;
self-serve onboarding is de-emphasized but **kept test-warm/reversible** (Counsel §6 guard-the-exit); the
`/checkout` URL changes (redirect seam + selector updates); the **operator +N** effort is on the ledger.

**Risks (RESOLVE dispositions):** R1 (route/selectors — accept); **R2 reversed** (keep subtotal); **R3
decided** — optional-but-inviting, one human ratification routed to product/ops; R4 (**must-fix:**
requestHash stability); **R6/R7 resolved** (F1 gated-dispatch + recovery, F3/F5 timestamp honesty, F4
DEFER-FLAG: no live `ready_at−preparing_at` learner today); **R8 resolved** (§6 go-live separated + ratified
revised-only); R9 (K4 shipped? — implementer verifies); **R10 tightened** (self-serve reversible).
**RESOLVE-3 DEFER-FLAGs:** (R3-1) **email-ownership verification before claim** (OTP / inbox-bound
magic-link) — the `invited_contact_hash` match is a string compare on an unverified email, defeatable by
registering the scraped contact; owner = **P6/auth seat** (the web path already refuses NULL-hash, G-F2g, but
the identity-proof gap is theirs). (R3-3) async auto-pickup — fix `courier-dispatch.ts:76 this.boss`→
`this.queue` + extend its `'offered'`-blind lookup (`:55–58`) before wiring the §5 no-courier path to it;
owner = **implementer**. (R3-6) decline TTL / soft-delete grace window; owner = **P6/acquisition seat**. Full
disposition table: `docs/design/flow-simplification-patch/resolution.md`.

## Migration

- **§1, §2, §3 (now):** **none** — UI composition only.
- **§5 (now):** **none migration-wise**, but RESOLVE-2/3 add **server-side code** (no DB change): reorder the
  PATCH `/orders/:id/status` IN_DELIVERY handler so the courier lookup precedes the status flip and no-courier
  keeps the order CONFIRMED (F1); make the dispatch **`'offered'`-aware** (already-bound guard + exclude
  `'offered'` from the lookup, R3-2); route IN_DELIVERY recovery through the shipped `releaseBindingAndReoffer`
  (asymmetric pre-pickup→READY / picked_up→CANCELLED, R3-4); **no `preparing_at` write** (F4 auto-stamp
  dropped); ETA decays off `COALESCE(preparing_at, confirmed_at, created_at)` — add `confirmed_at` to the
  `etaGather` SELECT + arg (R3-5) — and progress "Preparing" is a process label (F3/F5).
- **§4:** **none** — Zod default in `packages/shared-types`, backward-compatible; not a DB migration.
- **§5 (deferred):** future additive `locations.kitchen_flow_enabled boolean NOT NULL DEFAULT false`
  (RLS already FORCE; default covers existing rows; forward-only) — **only when the toggle is wired.**
- **§6:** **reuse P6 migs 068–071** (the transfer is shipped). A new migration only if the **K4
  `allergens_confirmed` writer** was never built (additive, owner-scoped, RLS FORCE). **No change to
  `claim_transfer`.**

## Flag / rollout

| Piece | Mechanism |
|---|---|
| §1 panel + Back-as-history / §2 count+subtotal bar / §4-UI toggle removal | `FLOW_SIMPLIFIED_CHECKOUT` (FE flag, staged) |
| §4 schema default | none — backward-compatible, ship unflagged |
| §5 OrderCard 2-tap + gated dispatch + IN_DELIVERY recovery | `OWNER_TWO_TAP` (FE flag, staged); reuses deliver-v2 offer-handshake flag |
| §6 claim surface | gated by the **already-dark** P6 activation (`PROVISION_OPS_SECRET` + migs 068–071) |

**Guardrails red→green before merge (RESOLVE escalated this from one to a set):**

| ID | Proves | Red today because |
|---|---|---|
| §4 requestHash | omit-`type` and send-`type:'delivery'` hash identically | (new correctness guard) |
| **G-F1a / G-F1a-2** (RESOLVE-2/3) | no-courier PATCH→IN_DELIVERY → `{status:'CONFIRMED',dispatched:false}` + row stays CONFIRMED; an already-bound (`'offered'`/active) order → no 500, no 2nd INSERT, `offer_pending`/`already_assigned` signal | `orders.ts:779` flips first → orphan (`:824`); `:792–794` `'offered'`-blind → 23505 (`…073:22–24,32–33`) |
| **G-F1b-i / G-F1b-ii / G-F1c** (RESOLVE-3) | IN_DELIVERY+`accepted`→ recovery→READY+cancel+re-offer; IN_DELIVERY+`picked_up`→ **CANCELLED not READY**; CONFIRMED+awaiting → owner re-tap once courier online → IN_DELIVERY | `OrderCard.tsx:221–236` no branch; central fold `orderStatusService.ts:129–140` blanket-cancels picked_up; §5 never enqueues a re-dispatch |
| **G-F2a/b/c/d/e/f** (RESOLVE-2) | token never in URL/hash/Referer/query (body-only); CONTACT_MISMATCH recipient-match active; claim completes with no navigation + no storage; scrub before telemetry | a `/claim?token=` surface would leak it; mint must bind contact; SPA must fetch-auth in-page |
| **G-F2g** (RESOLVE-3) | web claim (preview/accept) against a NULL-hash invite → refused (generic), `owner_id` stays NULL — token-only theft unreachable via web | `…071:64–68` permits NULL-hash bind to any authed user; `claim_transfer` untouched |
| **G-F3 / G-F5** (RESOLVE-3) | ETA decays off `COALESCE(preparing_at,confirmed_at,created_at)` (≈13 not 0/flat-15; NULL-`confirmed_at`→`created_at`); progress Ready dot not a false ✓; "Preparing" a process label | `etaGather.ts:81–86,192` flat for CONFIRMED, `confirmed_at` not selected; `OrderProgress.tsx:84–102` fills by statusIndex |
| **G-§2** (RESOLVE-2) | bar shows a labeled subtotal; fee surfaces on address-resolve, not at confirm | bar copy + fee-sequencing unspecified |
| **G-F6 / G-F9** | Back closes panel + stays on storefront; empty-cart deep-link → closed panel | panel not a history entry; reconcile trigger unspecified for the new seam |
| **G-PF1 / G-PF2** (RESOLVE-2) | `published_at` NULL through claim; allergen-confirm a distinct act into empty fields | consent gate prose-only until annotated in code |

Everything else is covered by existing checkout/owner E2E, the visual-regression net, and the shipped
P6/claim tests. Full disposition + re-grounding: `docs/design/flow-simplification-patch/resolution.md`.
