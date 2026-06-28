# Breaker Findings — flow-simplification-patch

**Seat:** Breaker (Triadic Council) · attack-only, no fixes · **Date:** 2026-06-28
**Grounded @ HEAD** (branch `feat/mvp-sensor-seams`) against live source. Round 1.

Verdict preview at the bottom. Each finding: `[SEVERITY] vector · finding · concrete break/number · violated invariant · evidence`.

---

## CRITICAL

_None._ The two surfaces most likely to hold a CRITICAL — §4 contract default and §6 inline ownership UPDATE — are clean:
- **§4 is sound at the parse layer.** `buildRequestHash` hashes the **resolved** `input.type` (`apps/api/src/routes/orders.ts:187` → `lib/order-canonical.ts:30,42`). With `.default('delivery')`, an omitted `type` resolves to `'delivery'` before both the superRefine and the hash, so omit-vs-send produce an identical hash. The "belt-and-suspenders client still sends delivery" is unnecessary but harmless. R4's guardrail is worth having but the risk it guards is near-zero — **downgraded to LOW** (F8).
- **§6 transfer authority is intact server-side.** `acceptClaim` calls only the `claim_transfer` SECURITY-DEFINER fn; the route derives org/loc from the matched invite, never a request param (`modules/acquisition/claim.ts:97-112`, `routes/public/claim.ts:26-28`). The patch's "no inline UPDATE" red-line holds **as long as the new surface obeys it** — the risk is on the transport, not the SQL (see F2).

---

## HIGH

### F1 — §5 turns the no-courier orphan from a rare edge into the default, and the OrderCard has zero recovery action for an unassigned IN_DELIVERY order (soft-trap)
**Vector:** B-FAIL / B-CONSIST / no-trap.
**Break scenario (concrete):** A restaurant with one courier who starts a shift at 18:00 accepts orders from 17:00. Under §5, the owner's first post-Accept tap is **"Send for delivery / Assign" → `CONFIRMED→IN_DELIVERY`**. The synchronous auto-assign query (`orders.ts:785-800`) finds **no `cs.status='available'` courier**, `availRes.rowCount === 0`, the `if` body is skipped — **no assignment row, no error, HTTP 200** (`orders.ts:824`). The order is now `IN_DELIVERY` with **no courier**. The owner OrderCard renders actions only for `PENDING / CONFIRMED / PREPARING / READY` (`packages/ui/src/components/admin/OrderCard.tsx:222-236`) — **there is no `IN_DELIVERY` branch**, so the card shows the order with **no actionable button**. Recovery to re-dispatch lives only behind the dark-flagged manual reassign path (`dashboard.ts /assign-courier`). On the shipped default surface the order is stranded.
**Why it is a regression, not "preserved behavior" (the proposal's R6 minimizes this):** today the only path to `IN_DELIVERY` is the manual `READY → Assign` tap (`OrderCard.tsx:234-235`) — a *deliberate dispatch moment* the owner performs when a courier is on hand. §5 moves the dispatch to **immediately after Accept**, before the kitchen starts, maximizing the window in which no courier is online. The orphan **probability shifts from rare to routine** for any venue whose couriers aren't online 100% of opening hours (every pre-order, every gap between shifts).
**Number:** 1 courier, 1-hour pre-shift order window, 5 pre-orders → 5/5 auto-orphan, each with no owner-facing recovery button.
**Violated invariant:** no-trap (an order must never reach a state with no operator action and no owner); B-FAIL fallback (a failed dispatch must surface, not silently succeed).
**Evidence:** `orders.ts:785-820` (silent no-op on `rowCount 0`), `OrderCard.tsx:222-236` (no IN_DELIVERY action), machine `IN_DELIVERY:['DELIVERED','CANCELLED','READY']` (`packages/domain/src/order-machine.ts:28` — revert exists but the card exposes no button to trigger it).

### F2 — §6 puts the SOLE transfer-authority token onto a web URL → Referer/history/log leakage re-opens the IDOR-class concern the shipped model closed
**Vector:** B-SEC (token transport), B-FAIL.
**Break scenario (concrete):** The patch's claim surface is `/claim/:token` or `/claim?token=…` (proposal §6 Option A, ADR §6). A 256-bit single-use token with a **72h TTL** (`modules/acquisition/claim.ts:12`) now lives in the browser URL. The claim/light-edit page edits **radius** (map), theme, prices — it will load third-party resources (map tiles, fonts, any analytics beacon). Each carries a **`Referer: …/claim?token=…` header to the third party**, plus the token is written to **browser history, CDN/access logs, and any error-tracking breadcrumb**. Because **the token is the sole authority** (`claim.ts` K2; `acceptClaim(pool, token, userId)` binds to *whatever account is authenticated*), a leaked token within 72h enables two attacks:
1. **Ownership theft:** attacker logs in as themselves, POSTs the leaked token to `/claim/accept` → `claim_transfer` assigns the shadow org to the **attacker's** `userId` (`claim.ts:26-28`, `modules/acquisition/claim.ts:103`).
2. **Griefing erase:** `/claim/decline` is **no-auth, token-only** (`routes/public/claim.ts:68-82`); a leaked token → `declineAndErase` → `hardDeleteShadow` wipes the legit restaurant's preview (`modules/acquisition/claim.ts:119-143`).
**Regression vs shipped model:** the shipped P6 vertical never exposed the token on a leaky web surface — transfer was operator-driven behind `PROVISION_OPS_SECRET` (server-side). The proposal's §6 security section addresses inline-UPDATE/IDOR-by-param but is **silent on token-in-URL transport leakage**, yet claims the claim red-line is "preserved."
**Violated invariant:** B-SEC token = sole authority must not be exposed on a transport that leaks to third parties/logs; the proposal's own "token is the sole transfer authority (IDOR-closed)" claim.
**Severity rationale (HIGH not CRITICAL):** the vertical is dark (migs 068-071 unplaced, `PROVISION_OPS_SECRET` unset), the stolen asset is a not-yet-orderable shadow built from public data with the raw blob erased on transfer, and publish is still gated. But the proposal explicitly re-opens a closed IDOR-class surface and asserts it is preserved — that assertion is false as written.
**Evidence:** `routes/public/claim.ts:17-42` (accept binds to authenticated user), `:68-82` (decline no-auth token-only), `modules/acquisition/claim.ts:12` (72h TTL), proposal §6 "Security/tenant" (no transport-leak treatment).

---

## MEDIUM

### F3 — §5 makes the customer-facing live ETA lie: kitchen time zeroes the instant the owner taps "Send for delivery," before the food exists
**Vector:** B-CONSIST (read honesty), B-DATA.
**Break scenario:** `etaGather.ts:81` computes remaining kitchen time as **0** for any status in `['READY','IN_DELIVERY','PICKED_UP']`; full kitchen time only for `CONFIRMED`, decaying for `PREPARING`. Under §5 the order jumps `CONFIRMED → IN_DELIVERY` at dispatch — the live ETA immediately drops kitchen time to 0 ("food is out of the kitchen") **even though the kitchen has not started** (this is exactly R6's "order can reach IN_DELIVERY before readiness"). The customer's live ETA under-estimates by the entire prep duration. The promised_window written at CONFIRMED (`etaGather.ts:178,236`) is now contradicted by a live ETA that says the food already left.
**Violated invariant:** customer-facing ETA honesty (don't tell the customer food is en route when it is not). R7 ("confirm no downstream consumer requires READY") under-scoped this — the ETA engine doesn't *require* READY, it *misbehaves* without it.
**Evidence:** `apps/api/src/lib/etaGather.ts:79-83`.

### F4 — §5 strands `preparing_at` / `ready_at` as NULL on every 2-tap order, starving the prep-time learning that feeds promised_window
**Vector:** B-DATA (data-quality decay over time).
**Break scenario:** `orderStatusService.ts:13-14` writes `preparing_at`/`ready_at` only on transition into those states. The 2-tap path skips both, so **every** simplified order records `preparing_at = NULL, ready_at = NULL`. Any analytics/learning computing kitchen duration (`ready_at − preparing_at`) — including the prep-time synthesis behind `promised_window` — degrades as the corpus fills with NULLs. Back-of-envelope: 100% of 2-tap orders contribute zero prep-duration samples; within one flag-rollout window the prep-time model trains on a shrinking, biased tail (only kitchens that kept the manual lane).
**Violated invariant:** don't silently destroy the signal that another shipped feature (prep-time/ETA, ADR mvp-sensor-seams) depends on. R7's "display/filter only" assessment is incomplete — `etaGather`/prep-learning is a real consumer.
**Evidence:** `apps/api/src/lib/orderStatusService.ts:13-14`, `apps/api/src/lib/etaGather.ts` (prep history feed).

### F5 — §5 customer progress bar asserts kitchen stages that never happened
**Vector:** B-CONSIST (UI honesty).
**Break scenario:** `OrderProgress.tsx` builds steps `CONFIRMED → PREPARING → READY → IN_DELIVERY → DELIVERED` and fills every dot up to `statusIndex` (`OrderProgress.tsx:84-102`). On a 2-tap order at `IN_DELIVERY`, `statusIndex` points past PREPARING and READY, so both dots render as **passed/checked** while their `at` timestamps (`preparingAt`, `readyAt`) are **NULL** (F4). The customer sees "Preparing ✓ Ready ✓" with blank times for stages the kitchen never marked.
**Violated invariant:** status-page copy must not claim states the order never entered. The proposal's own header comment (`OrderProgress.tsx:13-15`) documents the linear chain it now silently skips.
**Evidence:** `packages/ui/src/components/client/OrderProgress.tsx:59-102`.

### F6 — §1 Option A (route retired, panel = in-memory/query state) breaks browser-Back semantics that the `/checkout` route gives today
**Vector:** B-ANTIPATTERN / no-trap-adjacent UX.
**Break scenario:** Today `/s/:slug/checkout` is a real route (`ClientRoutes.tsx:13`); browser **Back** from checkout returns to the menu (history pop). The architect chose **Option A** (retire the route, panel state in `?checkout=1`/in-memory) and explicitly acknowledged Option B preserves URL/back-button. If the panel is in-memory only, **Back from an open checkout panel leaves `/s/:slug` entirely** (navigates to the prior site), discarding the storefront — the muscle-memory "Back to menu" now exits the store. If implemented as a `?checkout=1` query the back-stack grows an entry per open/close. Either way the proposal asserts "back/close on the panel = close + cart intact" but that is the *panel close button*, not the *browser Back button* — the latter regresses.
**Violated invariant:** no-trap / navigation continuity (Back must not strand the customer off the storefront). Proposal §1 conflates panel-close with browser-Back.
**Evidence:** proposal §1 Decision (Option A, route→redirect seam), `apps/web/src/routes/ClientRoutes.tsx:13`.

---

## LOW

### F7 — §3 "≈5 / ≈1-2 actions" omits preflight-acknowledge round-trips and the unresolved required-field floor
**Vector:** B-ANTIPATTERN (back-of-envelope honesty).
**Detail:** the create contract carries `acknowledged_codes` (E27 preflight) and `otp_code` (`legacy.ts:70-71`). When preflight returns codes (e.g. distance-tier "fee at checkout", far-address), the customer must acknowledge → an **extra render + tap** not counted in "≈5". If the council keeps `entrance`/`apartment` required (R3 explicitly *unresolved*), the floor is 4 fields + name, not the ≈5 the table claims as already-decided. The number is presented as settled while R3 is still open.
**Violated invariant:** a back-of-envelope must include the required inputs, not assume the optimistic resolution of an open decision.
**Evidence:** `packages/shared-types/src/legacy.ts:70-71`, proposal §0 table vs §3 R3 (open).

### F8 — §4 error-contract message path changes from "type required" to "delivery required" (additive default's only observable edge)
**Vector:** B-CONSIST (error-contract stability).
**Detail:** with `type: z.enum([...]).default('delivery')`, a payload omitting **both** `type` and `delivery` no longer fails on the `type` enum; it passes the type check, then fails the superRefine (`legacy.ts:74-76`) with `delivery is required for delivery orders` keyed on path `['delivery']`. Any client/test asserting the prior `type`-missing validation issue breaks. The order still 400s (correct — the floor pin is still enforced), so this is cosmetic to the contract but a real change to the error envelope. The requestHash-stability concern (R4) is **lower than the proposal frames it** since the hash already uses the resolved value (`order-canonical.ts:42`) — the guardrail is good hygiene, not a live risk.
**Violated invariant:** none hard; error-contract verbatim-stability (minor).
**Evidence:** `packages/shared-types/src/legacy.ts:42,72-77`, `apps/api/src/lib/order-canonical.ts:42`.

### F9 — §1 redirect seam: deep-link to `/s/:slug/checkout` with an empty or stale cart
**Vector:** B-FAIL (degradation), B-CONSIST.
**Detail:** the cart is `localStorage`-keyed per location (`dos_cart_${locationId}`, `CartProvider.tsx:62`) — so the **cross-tenant** merge attack is closed (good; not a finding). But the redirect seam must replicate `CheckoutPage`'s empty-cart handling: a cold deep-link to `…/checkout` with no stored cart should open the menu with a *closed* panel (proposal says "degrade to closed-panel + intact cart"), and a **stale** cart (items removed/price-changed since) must still hit the `cartReconcile` path before the panel renders a total. This is unproven for the new seam (the reconcile lives in `CheckoutPage`'s mount effect today); R1 accepts the selector churn but not the empty/stale deep-link state matrix.
**Violated invariant:** none hard if the mount effect is preserved; flagged because the seam moves the reconcile trigger from a route mount to a panel-open and that wiring is unspecified.
**Evidence:** `apps/web/src/lib/CartProvider.tsx:62-68`, proposal §1 Failures/degradation.

---

## Regression check vs shipped invariants
- **Cash/integer money:** untouched — §1/§2 only relocate the form; no money math changed. PASS.
- **Claim inline-UPDATE red-line:** server path clean (F2 is transport, not SQL). PASS on SQL, FAIL on transport claim.
- **Order machine:** no new edge; `CONFIRMED→IN_DELIVERY` already legal (`order-machine.ts:20`). PASS — but the *exercised-by-default* edge surfaces F1/F3/F4/F5 downstream.
- **Embed/normal merge:** the patch removes a divergence; no new one introduced. PASS (no finding).

---

## Net verdict: **NOT converged.**
**Open CRITICAL: 0 · Open HIGH: 2** (F1 unassigned-IN_DELIVERY orphan + no card recovery; F2 token-in-URL transport leak re-opening the claim IDOR-class concern). **MEDIUM: 4** (F3 ETA lies, F4 prep-signal decay, F5 progress-bar false stages, F6 Back-button regression). The two HIGHs are the gate: §5's "preserved behavior" framing (R6/R7) under-counts a probability shift into a routine soft-trap, and §6's "security preserved" claim is contradicted by the chosen token transport. Architect to resolve F1/F2 before this advances.

---

# RE-ATTACK round 2 — attacking the round-1 fix-set

**Seat:** Breaker · attack-only, no fixes · **Date:** 2026-06-28 · regression round.
**Scope:** ONLY the round-1 fixes (F1–F9 dispositions + §2/§3 revisions in `resolution.md` / revised `proposal.md`),
attacked for NEW holes they introduced. Closed findings not re-litigated unless a fix reopened them.
Grounded @ HEAD (`feat/mvp-sensor-seams`).

## HIGH

### R2-1 — F1(a) fix is specified against a "dispatch primitive" contract that does not exist; the three real assign mechanisms each violate one of its three required properties
**Vector:** B-FAIL / B-ANTIPATTERN (fix grounded on a fictional primitive) / no-trap.
**The fix's claim (resolution B.F1(a), proposal §5):** "Send for delivery" routes through "the dispatch primitive
(`dashboard.ts:214`)" which (i) auto-finds a courier, (ii) on none returns `{ dispatched:false, reason:'no_courier' }`
**synchronously**, and (iii) keeps the order CONFIRMED. **No such primitive exists.** The three live assignment
mechanisms each break a different one of those three properties:
- **`dashboard.ts:214 /assign-courier`** (the one the fix cites) **requires an explicit `courierId` in the body**
  (`dashboard.ts:219–222`) and returns **`404 NOT_FOUND`** when that courier isn't active (`:246`). It **never
  auto-discovers** a courier and **never** emits `dispatched:false/no_courier`. So a 2-tap "Send for delivery" with
  **no courier-picker cannot route through it** — it would 400 on the missing `courierId`.
- **`orders.ts:785–821`** (raw PATCH auto-assign) is the only mechanism that auto-finds the nearest available courier —
  but it is exactly the **silent-no-op→200 orphan** F1 condemned, and resolution B.F1(a) explicitly says the fix does
  **not** route through it.
- **`workers/courier-dispatch.ts`** auto-finds a courier but is an **async queue worker** that retries up to
  `COURIER_DISPATCH_MAX_ATTEMPTS=5` over ~2.5 min and signals failure via a **WS event** `ORDER_DISPATCH_FAILED`
  (`courier-dispatch.ts:64–69`), not a synchronous return; it cannot give the owner a synchronous "awaiting courier"
  affordance at tap-time.
**Compounding (the card plumbing forces the wrong route):** `OrderCard` exposes a **single** mutation callback
`onUpdateStatus(id, newStatus)` → `PATCH /orders/:id/status` (`OrderCard.tsx:12,31`). It has **no** path to
`/assign-courier` (different route, needs `courierId`). So the natural §5 wiring — "Send for delivery" through the
card's existing `onUpdateStatus('IN_DELIVERY')` — lands on `orders.ts:785` raw auto-assign = **the exact orphan F1
condemned**. Re-routing to the dispatch primitive is net-new card plumbing + a courier-picker, not the "reuse" the
resolution assumes.
**Break/number:** the "no orphan by construction" guarantee rests on a primitive whose contract is unbuilt; with the
shipped card shape the most-likely implementation re-creates the original 5/5 pre-shift orphan.
**Violated invariant:** no-trap; "fix must be grounded in the cited live source" (the cited `dashboard.ts:214`
contradicts the assigned contract). G-F1a is written against a return shape (`dispatched=false`) no current endpoint produces.
**Evidence:** `dashboard.ts:214–246` (requires courierId, 404 on none), `orders.ts:785–824` (silent-200 orphan),
`workers/courier-dispatch.ts:51–69` (async, WS-signalled), `OrderCard.tsx:12,31` (single onUpdateStatus → raw PATCH).

### R2-2 — F1's "no orphan / stays CONFIRMED until accept" guarantee is silently flag-coupled to `COURIER_OFFER_HANDSHAKE_ENABLED`, which is dark by default; under the shipped flag state §5 force-drives IN_DELIVERY
**Vector:** B-OPS (scaling/flag gate doesn't actually close) / no-trap.
**Break scenario:** The "courier available → order stays CONFIRMED until the courier accepts" branch (resolution
B.F1(a), `dashboard.ts:324–337`) only executes when `process.env.COURIER_OFFER_HANDSHAKE_ENABLED === 'true'`
(`dashboard.ts:322`). That flag is **dark by default** (memory `deliver-v2`: "Offer handshake (flag-dark)"; built
flagged-dark in commits `fc6d2eb6`/`5818d991`). Under the **shipped default (handshake OFF)**, `/assign-courier`
falls to the legacy branch (`dashboard.ts:340–353`): it **force-inserts `accepted` + drives
`updateOrderStatus(... 'IN_DELIVERY')` immediately** — no acceptance wait, order on IN_DELIVERY at dispatch.
§5 ships under its **own** flag `OWNER_TWO_TAP` (proposal §5 Operability), independent of the handshake flag. So the
realistic rollout — `OWNER_TWO_TAP` ON while `COURIER_OFFER_HANDSHAKE_ENABLED` stays dark (its current state) —
**unconditionally reintroduces premature IN_DELIVERY** (the F3/F5 honesty break) at dispatch, and the entire
"no orphan window" story is unreachable. The resolution never declares this cross-flag dependency, never gates §5
on the handshake flag, and G-F1a would pass only in a flag combination that is **not** the shipped default.
**Number:** with the two flags independent, 1 of the 2 plausible launch states (two-tap-on / handshake-off) defeats the fix.
**Violated invariant:** B-OPS (a flag/gate must actually close the hazard it claims); no-trap-by-construction.
**Evidence:** `dashboard.ts:322` (flag read), `:340–353` (legacy force-IN_DELIVERY), proposal §5 (`OWNER_TWO_TAP`
separate flag), memory `deliver-v2` (handshake dark).

### R2-3 — F2's "authenticate first, then bind" + "fragment-only, never localStorage/sessionStorage/cookies" is internally contradictory for the first-time owner: authenticating destroys the only copy of the token, so the claim is unachievable without re-introducing the very leak the fix closed
**Vector:** B-SEC / B-FAIL (the fix breaks its own happy path).
**Break scenario:** The claim recipient is, by definition, a restaurant with **no owner account yet** (shadow has
`owner_id NULL`; the whole point of claim is first-time ownership). F2's accept step is **auth-first**
(resolution B.F2 step 3): the owner must obtain an RS256 JWT *before* POSTing the token to `/claim/accept`. But the
token lives **only** in the URL fragment, scrubbed into an **in-memory variable** within `/claim`, with
localStorage/sessionStorage/cookies all **forbidden** (B.F2 step 1). A first-time owner who isn't already logged in
must run login/OTP — a navigation that **leaves `/claim`** (or full-reloads it). That navigation **destroys the
in-memory token** (page context gone) **and** the fragment (already scrubbed by `replaceState` on mount, B.F2 step 1b).
On return to `/claim` there is **no token** → "link no longer valid." The claim is **unachievable for exactly the
new-owner the flow targets.** The only ways to make it work all reopen the leak the fix closed: persist the token
across the redirect (localStorage/cookie — forbidden), or keep it in the URL through the login round-trip
(Referer/history/log leak — the original F2). 
**Compounding (auth-first does not add the protection implied):** `acceptClaim` binds to **whatever** userId is
authed; there is **no** check that the authed account is the intended recipient (there cannot be — the shadow has no
owner). So auth-first changes nothing about the underlying "token = sole authority, binds to any authed account"
theft model — the entire security delta still rests on the fragment **never** leaking, which R2-3 shows the happy
path itself violates.
**Number:** 100% of first-time claimants (the designed-for population) hit the token-loss dead-end under the literal fix.
**Violated invariant:** B-SEC (sole-authority token must not be persisted on a leaky transport) **vs** functional
completion — the fix cannot satisfy both as written; "claim_transfer/token-sole-authority preserved" is asserted but
the surface is non-functional or leaky.
**Evidence:** `routes/public/claim.ts:17–42` (accept binds to authed user, token-in-body), resolution B.F2 steps 1/3
(fragment→replaceState scrub→in-memory only, no storage; auth-first), `modules/acquisition/claim.ts` (shadow owner_id NULL pre-claim).

## MEDIUM

### R2-4 — F1(b)'s advertised "re-assign" recovery action on an IN_DELIVERY orphan is blocked by the cited endpoint's own status guard; only "revert" actually works, and "re-assign" is a two-step the fix presents as one
**Vector:** B-FAIL / no-trap (recovery affordance doesn't recover).
**Break scenario:** F1(b) (resolution B.F1(b), proposal §5) promises **two** IN_DELIVERY recovery actions —
**re-assign** and **revert-to-READY** — "both already exist server-side: `dashboard.ts:214` assign accepts
CONFIRMED/PREPARING/READY." But `/assign-courier` **explicitly rejects IN_DELIVERY**: `if (order.status !==
'CONFIRMED' && !== 'PREPARING' && !== 'READY') → 409 CONFLICT` (`dashboard.ts:235–237`). So the **re-assign** button
on an IN_DELIVERY orphan **409s**; the owner must first **revert→READY** (raw PATCH → `updateOrderStatus`, which does
correctly terminalize the binding, `orderStatusService.ts:129–139`) and **then** assign — two server round-trips and
two taps, not the single "re-assign" the card advertises. Worse, `OrderCard`'s single `onUpdateStatus` callback can
only reach the raw status PATCH, **not** `/assign-courier` (needs `courierId` + a picker), so the "re-assign"
affordance can't even be wired through the existing card prop. G-F1b asserts a visible recovery action renders, but
the "re-assign" half of it is non-functional against the cited contract.
**Violated invariant:** no-trap (an advertised recovery action that 409s is not a recovery); fix grounded in cited source.
**Evidence:** `dashboard.ts:235–237` (409 rejects IN_DELIVERY), `OrderCard.tsx:12,31` (single onUpdateStatus → raw PATCH),
`orderStatusService.ts:129` (revert-only path is the one that works).

### R2-5 — F4's auto-stamp `preparing_at = now()` at Accept makes `preparing_at == confirmed_at`, INVERTING the honesty problem F3 fixed: the customer sees "preparing" and an ETA that decays from accept-time before any food is cooked
**Vector:** B-CONSIST (read honesty) / B-DATA (skewed signal).
**Break scenario:** F3 stops the ETA zeroing at IN_DELIVERY by decaying from `preparing_at`; F4 then auto-stamps
`preparing_at = now()` at the Accept tap (resolution B.F3/F4, proposal §5). Net: prep time **starts decaying the
instant the owner accepts**, regardless of when the kitchen actually starts. A slammed kitchen accepts at 18:00
(`preparing_at=18:00`) but doesn't touch the dish until 18:20; with `prep=15`, at **18:14** the customer's live ETA
shows `prepRemaining ≈ 1 min` ("almost ready") and the progress bar shows **Preparing active** — for food **nobody
has started**. F3 cured the "food already left" over-claim at the back; F4 introduces a symmetric **"food is being
made" over-claim at the front**. For a backed-up kitchen this is arguably *worse* than the status-label heuristic,
because the customer is shown a confident decaying clock that is wrong.
**Compounding (kitchenAhead skew):** the queue-ahead estimate decays *other* in-flight orders by their `preparing_at`
(`etaGather.ts:93–94`). With every accepted order now carrying `preparing_at=accept-time`, an order accepted 15 min ago
but not yet cooked (kitchen behind) contributes **0** to `kitchenAhead`, so a *new* customer's ETA **understates the
real backlog** — a systematic optimistic bias that grows with kitchen load.
**Violated invariant:** customer-facing ETA/progress honesty (the exact §2/§5/§10 Counsel care-cost the fix claimed to
mitigate, now re-created at the front of the timeline); don't poison the queue-ahead estimator.
**Evidence:** resolution B.F4 (auto-stamp `preparing_at` on Accept via `COALESCE`), `etaGather.ts:83–86`
(decay from `preparing_at`), `:93–94` (kitchenAhead decays others by `preparing_at`).

### R2-6 — F2 fragment-scrub races app-boot telemetry: a global error/analytics init that reads `location.href` at boot captures the token before the route component mounts and scrubs
**Vector:** B-SEC (residual PII/secret egress).
**Break scenario:** B.F2 step 1b scrubs the fragment "before any third-party **resource** loads," but app-level
telemetry (Sentry/error-reporter, page-view beacon) typically initialises at **bootstrap**, before the `/claim` route
component mounts and runs `replaceState`. Such an init commonly records the initial `location.href` (incl.
`#t=<token>`) as the first breadcrumb / pageload event, shipping the 256-bit sole-authority token to a third-party
ingest within the 72h TTL. The scrub-timing guarantee covers sub-resource Referer but not the boot-time href capture,
and the proposal's own guardrail G-F2b only intercepts **outbound request URLs/Referer**, not an SDK that reads
`location.href` from JS and posts it in a body it controls.
**Violated invariant:** token = sole authority must not reach third-party logs/telemetry; the fix's "no leak"
assertion is scoped to Referer/sub-resources, not boot telemetry.
**Evidence:** resolution B.F2 step 1b ("before any third-party resource loads"), G-F2b (intercepts request URL/Referer only).

### R2-7 — §2 revision (keep items-subtotal on the bar) shows a number that is authoritative-looking but systematically UNDER-states what the cash customer must produce at the door, by the omitted delivery fee (+ any preflight far-address surcharge)
**Vector:** B-CONSIST (cash-as-proof honesty), Counsel §9 inverted.
**Break scenario:** §2 now keeps "N items · {subtotal}" on the bar, justified as the cash-as-proof "signal." But the
cash-constrained least-served customer budgets to the **persistent** number they see while shopping — the subtotal —
and the **delivery fee** (and, on distance-tier / far-address orders, the E27 preflight surcharge the customer must
acknowledge, `legacy.ts:70–71`) is revealed only later, in the panel. So the customer assembles a cart sized to the
cash they brought, then at the door owes **subtotal + fee** and is short. Count-only at least made no numeric promise;
the subtotal makes a **specific, authoritative-looking promise that is wrong by exactly the variable the customer
cannot predict** (the fee). For the cash-on-hand demographic this is the shape of an unintended dark pattern just as
much as the original count-only was — the revision swapped one honesty failure for another rather than resolving it.
**Violated invariant:** cash-as-proof (the customer must be able to shop within the money they can produce — a partial
total that omits the fee defeats this at the door).
**Evidence:** proposal §2 (bar shows items-subtotal, fee resolves in panel), `legacy.ts:70–71` (preflight `acknowledged_codes`/surcharge).

## LOW

### R2-8 — §3 optional-by-default entrance/apartment strands the buzzer-only / no-elevator / language-barrier delivery for the population that disproportionately skips it (regression on the least-served, ships before the human ratification it depends on)
**Vector:** B-FAIL (last-50-metres), Counsel §3 carried-forward.
**Break scenario:** R3 lands on "optional-but-inviting" with the floor at phone+pin, and the architect default is to
ship optional **pending** a product/ops ratification that is explicitly **still open** (resolution F. "one item needs a
human decision"). Optional means the skip is the path of least resistance (one fewer tap), and the population most
likely to skip — low phone comfort, language barrier, hearing difficulty, buzzer-only building — **overlaps exactly the
population for whom "the courier can call" is the failure, not the safety net.** A skipped apartment/entrance on a
buzzer-only block = courier at the door, can't get in, customer can't take the call → failed delivery with cash in
play. The action-count "≈5" (F7) and the §3 floor are both quoted as decided in the proposal while the field-required
decision is routed to humans and unresolved — so the patch risks building optional-by-default before ratification.
**Violated invariant:** don't trade a real failure (undeliverable order for the least-served) for a saved tap; don't
advance build on an open human-ratification item.
**Evidence:** proposal §3 / resolution C§3 (optional-but-inviting, routed to product/ops, unratified),
`CheckoutPage.tsx:404–420` (entrance/apt/notes currently client-required — relaxing them is the regression surface).

---

## RE-ATTACK round 2 — net verdict: **NOT converged — the round-1 fixes opened NEW HIGHs.**

**New CRITICAL: 0 · New HIGH: 3 · New MED: 4 · New LOW: 1.**
- **R2-1 (HIGH):** F1(a)'s "dispatch primitive returns `dispatched:false`/keeps CONFIRMED" is a **fictional contract** —
  the cited `dashboard.ts:214` requires an explicit `courierId` and 404s on none; the only auto-discover path is the
  raw-PATCH orphan F1 condemned; the card's single `onUpdateStatus` callback routes to that very orphan.
- **R2-2 (HIGH):** F1's no-orphan guarantee is **silently flag-coupled** to a dark `COURIER_OFFER_HANDSHAKE_ENABLED`;
  under the shipped default (handshake off, `OWNER_TWO_TAP` on) §5 force-drives IN_DELIVERY — fix unreachable.
- **R2-3 (HIGH):** F2's auth-first + fragment-only + no-storage is **self-contradictory for the first-time owner** —
  authenticating destroys the only token copy → claim unachievable, or the token must be persisted/kept-in-URL,
  reopening the exact leak F2 closed.

**Regression on round-1 closes:** F1 reopened (R2-1/R2-2/R2-4); F2 reopened (R2-3/R2-6); F3/F4/F5 honesty re-created at
the front of the timeline (R2-5); §2 swapped one honesty failure for another (R2-7); §3 ships ahead of its ratification
(R2-8). The two HIGH dispositions (F1, F2) are **grounded on source contracts that contradict the cited code** and on a
**flag state that is not the shipped default** — they must be re-resolved before this advances. F6/F9 (panel history)
introduced no new hole and stand.

---

# RE-ATTACK round 3 — convergence check on the RESOLVE round-2 fix-set

**Seat:** Breaker · attack-only, no fixes · **Date:** 2026-06-28 · regression round 3.
**Scope:** ONLY the RESOLVE round-2 dispositions (R2-1…R2-7 + §2/§3) in `resolution.md` and the revised
`proposal.md`, attacked for NEW holes. Grounded @ HEAD (`feat/mvp-sensor-seams`) against files read this
round (file:line below). Round-1/round-2 closed items are not re-litigated except where a round-2 fix
reopened or overstated one.

> **Regression note (what round 2 got RIGHT, confirmed):** `withTenant` IS transactional
> (`packages/platform/src/auth/tenant.ts:10–16` — `BEGIN`/`COMMIT`/`ROLLBACK`), so the R2-1 reorder
> (lookup → flip → INSERT) **cannot leave an orphan**: any failure rolls the flip back to CONFIRMED. The
> "fictional dispatch primitive" retraction and the no-orphan-by-construction claim hold. R2-6 telemetry
> exclusion, R2-7/§2 subtotal+fee-sequencing, §3 NEEDS-HUMAN, and the PF code-markers introduced no new hole.
> The findings below are the residue the round-2 fixes did **not** close.

## HIGH

### R3-1 — R2-3's "recipient binding" (CONTACT_MISMATCH) is NOT the second factor the resolution claims; the asserted "token + proof of control of the invited email" is false on two independent legs
**Vector:** B-SEC (auth/ownership red-line) · attacks R2-3 + resolution §E2.
**The claim (resolution R2-3 step 4, §E2, proposal §6 step 4):** the operator mint binds `invited_contact_hash`,
which "converts 'token = sole authority, binds any authed account' into **token + proof of control of the
invited email**." **Both halves are false at HEAD:**
- **(a) The binding is OPTIONAL and NULL→token-only is a permitted, documented mint path.** `invited_contact`
  is `z.string().…optional()` in the mint schema (`apps/api/src/modules/acquisition/route.ts:43`);
  `mintClaimInvite(…, invitedContact?)` writes `invitedContact ? hashContact(invitedContact) : null`
  (`modules/acquisition/claim.ts:53,66`); the `claim_invites.invited_contact_hash` column is **nullable**
  (`migrations/…071:27`) and `claim_transfer` explicitly **skips** the email check when it is NULL —
  "*Token-only when invited_contact_hash IS NULL (ops minted without a contact)*"
  (`migrations/…071:64–68`). So a single mint that omits the contact reopens round-1's exact "binds to **any**
  authed account" theft model, and the web surface cannot detect a NULL-hash invite. G-F2d asserts NOT-NULL
  for *one* test fixture; it does not, and cannot (no NOT-NULL constraint, optional param), prevent a
  NULL-hash mint.
- **(b) Even when the hash IS bound, the match is defeatable — there is no email-ownership verification.**
  `claim_transfer` checks `sha256(lower(trim(users.email))) == invited_contact_hash`
  (`migrations/…071:62–69`), where `users.email` is whatever the claimer **self-asserted at registration**.
  Owner auth is argon2 password (`apps/api/src/routes/auth/local.ts:24,100–101`) with **no email-ownership
  step** (`grep email_verified|verifyEmail|OTP` over `apps/api/src/routes/auth/` → empty; memory: OTP
  disabled). The `invited_contact` is by design the restaurant's **public, operator-scraped** email (the
  Art-14 notice address). So an attacker who has the token can register an account **under that same public
  email** (no verification blocks it) and pass CONTACT_MISMATCH.
**Break/number:** the real security still rests **entirely** on the 256-bit token not leaking (round-1 F2);
the claimed second factor adds ~0 protection in the realistic case (public scraped email + unverified
registration, or a NULL-hash mint). The round-2 fix's transport hardening (fragment/scrub/telemetry-exclude)
is sound and genuinely lowers leak probability — but the resolution's §E2 red-line statement "claim_transfer
strengthens via the operator-mint contract … token + proof of control of the invited email" is **false as
written**.
**Violated invariant:** B-SEC — do not assert a defense (recipient binding) that the schema makes optional
and the auth layer makes bypassable; an ownership-transfer red-line must not stand on an unenforced operator
discipline + an unverified self-asserted email.
**Severity = HIGH, not CRITICAL:** the vertical is dark (migs 068–071 unplaced, `PROVISION_OPS_SECRET` unset);
the asset is a not-yet-orderable shadow built from public data; publish stays gated. But the round-2
re-resolution's central security upgrade is theater, so F2's HIGH is **not** actually retired by the binding —
only by transport.
**Evidence:** `modules/acquisition/route.ts:43`, `modules/acquisition/claim.ts:53,66`, `migrations/…071:27,62–68`,
`routes/auth/local.ts:24,100–101`, resolution §E2 + R2-3 step 4.

## MEDIUM

### R3-2 — R2-1's reorder makes the card "Send for delivery" throw an uncaught 500 in two cases the fix never models: concurrent dual-dispatch of the sole courier, and an order with a pending 'offered' handshake — because the lookup-exclusion list is inconsistent with the two DB unique indexes
**Vector:** B-FAIL (uncaught 500) / B-CONSIST (filter ≠ constraint) · attacks R2-1/R2-2.
**Break scenario:** the card's auto-assign lookup excludes couriers whose assignment status is in
`('assigned','accepted','picked_up')` — **it omits `'offered'`** (`orders.ts:792–794`). But mig 073 made
**both** active-assignment unique indexes include `'offered'`:
`courier_assignments_order_active_uniq (order_id) WHERE status IN ('offered','assigned','accepted','picked_up')`
and `courier_one_active_assignment (courier_id) WHERE status IN (…,'offered',…)`
(`migrations/…073:22–24,32–33`). Two consequences the reorder exposes:
- **(i) Pending-offer collision (handshake ON):** a manual `/assign-courier` offer leaves the order
  **CONFIRMED** with an `'offered'` row (`dashboard.ts:329–337` — never calls `updateOrderStatus`). The card
  still shows that CONFIRMED order as dispatchable; tapping "Send for delivery" → lookup does **not** exclude
  the offered courier → flip → `INSERT … 'assigned'` violates **both** unique indexes → the `orders.ts:802`
  INSERT has no `ON CONFLICT` → **500**. (`withTenant` rolls the flip back, so no orphan — but the owner gets
  an opaque 500, not the "awaiting courier" affordance the fix promised.)
- **(ii) Concurrency race widened by the reorder:** the original code ran `updateOrderStatus` (heavy: ETA
  synthesis DB round-trips + ≥3 async bus publishes, `orderStatusService.ts:164–203`) **before** the
  lookup→INSERT pair, so lookup→INSERT was ~one round-trip. The reorder puts that heavy work **between**
  lookup and INSERT, so two near-simultaneous dispatches of the **sole** courier both see it free, both flip,
  the second INSERT trips `courier_one_active_assignment` → **500** on the losing tap.
**Number:** any shop running the offer-handshake, every offered-but-unaccepted order is a CONFIRMED row that
500s on a card re-tap; the reorder turns a ~microsecond double-dispatch window into a multi-round-trip one.
**Violated invariant:** B-FAIL (a dispatch attempt must degrade to a stated state, not an uncaught 500);
B-CONSIST (the in-code availability filter must match the DB uniqueness predicate). G-F1a asserts only the
clean `rowCount===0` branch — it does **not** cover courier-found-but-INSERT-conflicts.
**Evidence:** `orders.ts:792–794,802` (exclusion omits 'offered', INSERT no ON CONFLICT),
`migrations/…073:22–24,32–33` (indexes include 'offered'), `dashboard.ts:329–337` (offer keeps CONFIRMED),
`orderStatusService.ts:164–203` (heavy I/O now inside the lookup→INSERT window).

### R3-3 — R2-1 removes the only no-courier path to IN_DELIVERY, making the shipped owner-proxy completion unreachable exactly when a courier is unavailable (the pre-shift scenario F1 targeted)
**Vector:** B-FAIL / no-trap-inverted (the fix trades "silent orphan" for "cannot dispatch at all").
**Break scenario:** owner-proxy pickup and delivery both **require the order to already be IN_DELIVERY**:
`/pickup` → `if status !== 'IN_DELIVERY' → 409` (`dashboard.ts:392–394`); `/deliver` → `if status !==
'IN_DELIVERY' → 409` (`dashboard.ts:473–475`), then `completeDelivery` (`:492–499`, the shipped deliver-v2
owner-proxy path). The **only** way a shop with no available system courier reached IN_DELIVERY was the
unconditional flip at `orders.ts:779`. R2-1 removes it for `rowCount===0` → the order **stays CONFIRMED**. So
in the exact F1 scenario (courier exists but is pre-shift/offline, 5 pre-orders), the owner can **no longer**
push any order to IN_DELIVERY, and therefore **cannot use owner-proxy `/deliver` at all** — it 409s on the
CONFIRMED order. `/assign-courier` also 404s (no active courier, `dashboard.ts:246`). The order is stuck at
CONFIRMED until a courier comes online; an owner who would have self-delivered the early order is blocked.
**Number:** F1's own 5 pre-shift pre-orders — previously 5 (orphaned-but-deliverable-by-owner), now 5
(undispatchable, owner-proxy path dead) until shift start.
**Violated invariant:** don't silently revoke a shipped capability (owner-proxy completion, deliver-v2) for
the population the fix was meant to serve; a no-courier order must have *a* forward path, not just a non-error.
**Severity = MEDIUM:** honest framing — the fix correctly prevents the orphan and the order is never *lied
about* (stays CONFIRMED). The cost is that owner-self-delivery in the no-courier window is now impossible,
which the resolution never reconciled against the owner-proxy path it relies on elsewhere.
**Evidence:** `dashboard.ts:392–394,473–475,492–499`, `orders.ts:779` (flip the fix removes for no-courier),
resolution R2-1.

### R3-4 — R2-4's blanket "Revert to READY" recovery has no assignment-status guard: reverting an order whose courier already 'picked_up' strands the food and regresses the customer status, contradicting deliver-v2's picked_up→CANCELLED honesty
**Vector:** B-CONSIST (deliver-v2 honesty) / no-trap-adjacent · attacks R2-4.
**Break scenario:** R2-4 renders a single `onUpdateStatus('READY')` button on **every** IN_DELIVERY order. An
order stays `status='IN_DELIVERY'` through courier pickup (`courier_assignments.status` goes
assigned→accepted→**picked_up** while the order is still IN_DELIVERY). The central fold in `updateOrderStatus`
terminalizes assignments `IN ('offered','assigned','accepted','picked_up')` on IN_DELIVERY→READY
(`orderStatusService.ts:129–139` — the list **includes 'picked_up'**). So an owner mis-tapping "Revert to
READY" on an **en-route, food-already-collected** order: cancels the picked_up binding, frees the shift, and
drives the customer status **backward** IN_DELIVERY→READY ("back in the kitchen") while the courier is
physically holding the food with no active task. The resolution's "exactly as `dashboard.ts:288` already
does" is a mis-cite: that revert is the *displaced* order during a **reassignment to a new courier** (the food
gets a new deliverer); the R2-4 card revert is a **dead-end** revert with no re-assignment and no
picked_up guard.
**Violated invariant:** deliver-v2 honesty (memory: picked_up→CANCELLED, not READY — food that left the
kitchen is not "READY"); customer status must not regress to a stage the order has physically passed.
**Evidence:** `orderStatusService.ts:129–139` (fold cancels 'picked_up' on revert-to-READY),
`dashboard.ts:284–290` (the reassignment-context revert R2-4 mis-cites as equivalent), resolution R2-4 +
proposal §5 F1(b).

## LOW

### R3-5 — R2-5's "ETA decays off confirmed_at" is unplumbed: the synthesis path reads preparing_at, never confirmed_at, and a directly-CONFIRMED order has confirmed_at NULL → the G-F3 "≈13 not flat 15" intent can't be met without new plumbing
**Vector:** B-DATA / B-ANTIPATTERN (DoD vs the cited consumer) · attacks R2-5.
**Detail:** `synthesizeAndPersistEtaWindow` selects `o.created_at` and `o.preparing_at` but **not**
`confirmed_at` (`etaGather.ts:192`), and `gatherOrderEtaRange`'s input carries `preparingAt`/`createdAt`, no
`confirmedAt` (`etaGather.ts:212–226`). So "decay off `preparing_at ?? confirmed_at`" requires adding
`confirmed_at` to the SELECT, the input interface, and the decay branch — it is not the "revised one-liner"
the disposition implies. Compounding: an order inserted **directly** as CONFIRMED (not via
`updateOrderStatus`) has `confirmed_at` NULL (it is stamped only by the guarded UPDATE,
`orderStatusService.ts:90–95`), so `preparing_at ?? confirmed_at` is NULL → no decay base → flat `orderPrep`,
which is exactly the value G-F3 says must NOT appear. **No NPE at HEAD** — 2-tap orders reach the
`['READY','IN_DELIVERY','PICKED_UP']` zero-branch (`etaGather.ts:81`) before any preparing_at read, so R2-5's
"no live consumer skews" stands; this is a correctness gap in the *fix's* decay base, not a current break.
**Evidence:** `etaGather.ts:81,192,212–226`, `orderStatusService.ts:90–95`, resolution R2-5 / G-F3.

### R3-6 — R2-3 decline stays no-auth, token-only, irreversible, audited to no actor; round-2 protects only the /claim-page transport, not the token's full 72h life
**Vector:** B-SEC (asymmetric destructive action) · attacks R2-3 step 6 (carried from R2 design-accept).
**Detail:** `/claim/decline` is no-auth, token-in-body, → `declineAndErase` → `hardDeleteShadow`
(`routes/public/claim.ts:68–82`). The round-2 transport fix hardens only the `/claim` boot (fragment scrub +
telemetry exclusion); the token is live for **72h** and any leak **outside** that controlled page (operator's
email client/forward, a screenshot, a browser extension on a different tab) → an unrecoverable erase with
**no record of who declined** (no auth). This is accepted-by-design per H-decline, but flagged because the
round-2 "griefing closed by transport" claim is scoped to one surface, while the destructive primitive's
exposure window is the whole token lifetime.
**Evidence:** `routes/public/claim.ts:68–82`, resolution R2-3 step 6.

## Regression check (item 5 — R2-7 / §2 / §3, quick, not re-litigated)
- **R2-7 / §2** (subtotal label + fee-on-address-resolve): no new hole. The split (items-subtotal on the bar,
  fee surfaced at pin-drop, never deferred to confirm) is internally consistent and server-authoritative. PASS.
- **§3** (three-way floor): NEEDS-HUMAN, build gated — no code advances, so nothing to attack yet. PASS.
- **R2-6** (telemetry exclusion + scrub-before-init): a defensible defense-in-depth design; not re-attacked. PASS.
- **PF code-markers / G-PF1/G-PF2:** moving the consent gate from prose to a guardrail is strictly stronger. PASS.

---

## RE-ATTACK round 3 — net verdict: **NOT fully converged — 1 new HIGH + 3 new MED + 2 new LOW.**

**New CRITICAL: 0 · New HIGH: 1 · New MED: 3 · New LOW: 2.**
- **R3-1 (HIGH):** R2-3's recipient-binding "second factor" is theater — `invited_contact` is `.optional()`
  (NULL→token-only is a documented mint path) **and**, when set, the `sha256(users.email)` match is
  defeatable because owner registration verifies no email ownership and the invited contact is a public
  scraped email. The resolution's §E2 "token + proof of control of the invited email" is false as written;
  F2's residual still rests entirely on the token not leaking. (Dark vertical → HIGH, not CRITICAL.)
- **R3-2 (MED):** the R2-1 reorder + the lookup-exclusion's omission of `'offered'` (vs the two mig-073 unique
  indexes that include it) make the card dispatch **500** on a pending-offer order and on a widened
  concurrent dual-dispatch race; G-F1a tests neither.
- **R3-3 (MED):** R2-1 makes the shipped owner-proxy completion (`/deliver` 409s unless IN_DELIVERY)
  unreachable in the exact no-courier window F1 targeted — orphan traded for undispatchable.
- **R3-4 (MED):** R2-4's unconditional revert-to-READY strands a `picked_up` (food-out) order and regresses
  the customer status, contradicting deliver-v2 honesty; the cited `dashboard.ts:288` equivalence is a mis-cite.

**What round 2 DID converge:** the fictional-primitive retraction is correct; `withTenant` transactionality
confirms the no-orphan-by-construction claim (failures roll back to CONFIRMED, not orphan); the 4-cell flag
matrix's orphan-freedom holds; R2-5's "no live consumer skew" is grounded-true; F6/F9/R2-6/R2-7/§2/§3/PF
introduced no new hole. The gate is **R3-1** (security assertion false) plus the three R2-1/R2-4 MEDs above —
all of which are dispositions that overstated their grounding (a binding that's optional+bypassable, a
"reorder = same body" that isn't constraint-consistent, an owner-proxy path the fix silently severs, a
"revert exactly as today" that isn't picked_up-safe). Architect to re-resolve before advancing.

---

# RE-ATTACK round 4 — convergence gate on the RESOLVE round-3 fix-set

**Seat:** Breaker · attack-only, no fixes · **Date:** 2026-06-28 · regression round 4 (loop-exit).
**Scope:** ONLY the RESOLVE round-3 dispositions — G-F2g (web refuses NULL-hash), R3-2 (already-bound
guard), R3-3 (owner re-tap re-dispatch), R3-4 (asymmetric recovery via `releaseBindingAndReoffer`),
R3-5 (COALESCE decay base). Attacked for NEW CRITICAL/HIGH only. Grounded @ HEAD
(`feat/mvp-sensor-seams`, `aabae9c5`) against files read this round (file:line below). No re-litigation of
accepted MED/LOW/defer-flags unless a round-3 fix reopened a red line — none did.

## Regression confirmation per round-3 fix (what holds)

**G-F2g (R3-1a) — web claim refuses token-only (NULL-hash) invites — HOLDS (server-side, not just UI).**
The refusal is asserted server-side: G-F2g requires `organizations.owner_id` stays NULL on a NULL-hash
preview/accept, not a UI-only guard. The theft vector it closes — `acceptClaim`→`claim_transfer` binding a
NULL-hash token to *any* authed account (`claim.ts:97-112`, `claim_transfer` untouched, still NULL-permissive
for non-web/ops paths) — is unreachable via the web path once preview+accept both precondition on
`invited_contact_hash IS NOT NULL`. No web accept/decline path skips the precondition for the *theft* vector:
decline (`claim.ts:119-143`) does not transfer ownership (it erases), so leaving it outside G-F2g does not
reopen the binds-to-any-account theft. Net-positive vs round-3 baseline. **No new HIGH.**

**R3-2 — already-bound no-op + `offer_pending` signal — HOLDS; the feared stuck-state is RECOVERABLE.**
Attack: owner taps "Send for delivery" on an order already carrying an `'offered'` binding → sees
`offer_pending`; the offer later expires/declines → is the order stuck CONFIRMED with a stale signal? **No.**
Both terminal paths flip the binding `'offered'`→`'offered_expired'` and leave the customer order untouched
at CONFIRMED: the durable sweep (`courier-offer-sweep.ts:41-44`, `UPDATE … SET status='offered_expired'
WHERE status='offered' AND offered_expires_at < now()`) and the courier-decline route
(`assignments.ts:544`, same flip, `cancellation_reason='courier_declined'`). `'offered_expired'` is NOT in
the active set `('offered','assigned','accepted','picked_up')` the R3-2 guard checks, so after expiry/decline
the order is bare CONFIRMED with no active binding → the owner re-tap (R3-3) dispatches fresh. The stale
"offer pending" is self-clearing within ≤1 min of the deadline. **No stuck-state. No new HIGH.**

**R3-3 — owner re-tap as the re-dispatch path — HOLDS by construction (status-driven, not hidden after tap).**
Attack: does the awaiting-CONFIRMED card actually keep a visible "Send for delivery" affordance to re-tap, or
hide it after the first tap? The card renders actions **purely by `order.status`** (`OrderCard.tsx:222-236`),
holding no per-order "already dispatched" state — `loadingAction` resets after each call
(`OrderCard.tsx:32`). A no-courier no-op leaves `status==='CONFIRMED'`, so the §5 dispatch button re-renders
unconditionally on the next paint → the awaiting→re-tap→IN_DELIVERY loop closes (G-F1c). The button is not
hidden after the first tap. **No dead end. No new HIGH.** (Build note, not a finding: at HEAD the CONFIRMED
branch renders only `order-prepare`; the §5 dispatch button on CONFIRMED is design-not-built — the re-tap
property depends on §5 wiring the dispatch action onto the status-driven CONFIRMED branch, which is the
established §5 scope.)

**R3-4 — asymmetric recovery reuses `releaseBindingAndReoffer` — HOLDS; the asymmetry is real and correct.**
Verified the shipped primitive encodes exactly the required branch (`bindingRelease.ts:37-48`): `ordStatus
IN_DELIVERY && asgStatus='picked_up'` → `updateOrderStatus(CANCELLED)` (no re-offer); IN_DELIVERY pre-pickup
→ `READY` + re-enqueue. The round-2 raw `updateOrderStatus('READY')` (which the central fold
`orderStatusService.ts:129-140` blanket-cancels regardless of `asg_status`, stranding picked-up food) is
correctly retired; G-F1b-ii asserts picked_up→CANCELLED. After a pre-pickup recovery the order lands READY,
where the existing READY-branch "Assign Courier" button (`OrderCard.tsx:234`) re-dispatches — no stuck-state.
**Could the owner still pick a raw READY transition that bypasses the asymmetry?** Not from the card: there is
no IN_DELIVERY branch and no raw IN_DELIVERY→READY affordance (`OrderCard.tsx:222-236`); the recovery must
route through the new owner endpoint (R3-4) wired to `releaseBindingAndReoffer`, and G-F1b-i/ii is the
red→green that catches a raw-revert regression. **No new HIGH.** (Plumbing note, not a finding: the recovery
needs a NEW owner-scoped endpoint AND a SECOND card callback — the card's single `onUpdateStatus`
(`OrderCard.tsx:12`) → PATCH `/orders/:id/status` cannot express the asymmetry; the resolution acknowledges
the endpoint but is silent on the second callback. Implementation cost, design-sound.)

**R3-5 — `COALESCE(preparing_at, confirmed_at, created_at)` decay base — HOLDS.** `created_at` is always
non-NULL (every order row has it), closing the directly-CONFIRMED NULL-`confirmed_at` hazard by construction.
LOW, design plumbing only (add `o.confirmed_at` to the SELECT + a decay branch). **No new HIGH.**

## Residual (NOTED, not blocking — LOW, dark vertical)

**[LOW] B-SEC/dignity · G-F2g + decline asymmetry on a legitimately-minted NULL-hash invite.** The mint param
`invitedContact` is `.optional()` (`claim.ts:53,66`) with no NOT-NULL column constraint
(`migrations/…071:27`); G-F2d (operators MUST supply a contact) is a test-fixture discipline, not a schema
gate. If an operator mints a NULL-hash invite, G-F2g refuses the **claim** (preview/accept) with the generic
"link no longer valid" — but `declineAndErase` (token-only, no-auth, not in G-F2g's preview/accept scope,
`claim.ts:119-143`) still **erases**. So a legitimately-contacted restaurant on a NULL-hash invite can DELETE
its shadow but cannot CLAIM it, and is shown a false "link invalid". This is a *stated* strict-improvement on
the theft vector (R3-1a), but the specific consequence — legit claim becomes a dead link with a false error
while decline still fires — is undocumented in the resolution. Severity LOW: the vertical is dark (migs
068-071 unplaced, `PROVISION_OPS_SECRET` unset) and it only fires under a NULL-hash mint that G-F2d forbids.
Does NOT reopen a red line (it closes the theft red line). Recommend a one-line disposition note + treating
G-F2d as a schema/route gate rather than a fixture; not a convergence blocker.

## RE-ATTACK round 4 — net verdict: **CONVERGED.**

**Open CRITICAL: 0 · Open HIGH: 0.** Every round-3 fix holds against HEAD grounding: G-F2g closes the
NULL-hash theft on the web path server-side; R3-2's already-bound signal is self-clearing
(`offered`→`offered_expired` via sweep/decline, customer order untouched) and the order stays recoverable by
owner re-tap; R3-3's re-tap is status-driven and not hidden-after-tap; R3-4 reuses the genuinely asymmetric
`releaseBindingAndReoffer` (picked_up→CANCELLED, pre-pickup→READY), with no raw-revert affordance on the card
and G-F1b-i/ii guarding the asymmetry; R3-5's `created_at` floor closes the NULL decay hazard. The three
round-2 overstatements the round-3 resolution corrected (recipient-binding strength, raw revert-to-READY,
`confirmed_at`-only decay) are each genuinely fixed. One LOW residual (G-F2g/decline asymmetry on a
forbidden-but-permitted NULL-hash mint) is NOTED for disposition; it does not block. Carried defer-flags
(R3-1 email-ownership verify → P6/auth; R3-3 worker `this.boss`+`'offered'` exclusion → implementer; R3-6
decline TTL → P6; F4 learner filter) and the §3 NEEDS-HUMAN are unchanged and out of this patch's scope.
**The loop is converged: 0 open CRITICAL/HIGH.**
