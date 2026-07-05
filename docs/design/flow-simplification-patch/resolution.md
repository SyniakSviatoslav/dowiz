# Resolution — flow-simplification-patch (Architect seat, RESOLVE round)

**Seat:** Architect (Triadic Council) · **Date:** 2026-06-28 · design-only, no production code.
**Inputs:** `breaker-findings.md` (F1–F9), `counsel-opinion.md` (§1–§10 + binding condition), `proposal.md`.
**Re-grounded @ HEAD** (`feat/mvp-sensor-seams`) against live source. Each disposition cites file:line.

> **Convergence verdict at the bottom.** Net: 2 HIGH FIXED (design changed), 4 MED FIXED, 2 LOW
> FIXED/ACCEPT, 1 DEFER-FLAG; all Counsel points dispositioned; binding condition recorded in the ADR.
> **One item routed to a human decision** (§3 required-field floor — product/ops ratification).

---

## A. Re-grounding (the facts each disposition stands on)

| Claim | Live source @ HEAD | Status |
|---|---|---|
| Auto-assign silently no-ops on no courier (rowCount 0 → 200) | `orders.ts:785–821` (`if ((availRes.rowCount ?? 0) > 0)` guards the INSERT; else nothing, 200 at `:824`) | **CONFIRMED** |
| OrderCard has no IN_DELIVERY action branch | `OrderCard.tsx:222–236` — branches only for PENDING/CONFIRMED/PREPARING/READY | **CONFIRMED** |
| Owner reassign reverts IN_DELIVERY→READY + offer-handshake keeps order off IN_DELIVERY until accept | `dashboard.ts:214,235,284–288,326–337` ("is NOT driven to IN_DELIVERY … until acceptance") | **CONFIRMED — reusable for F1** |
| `/claim/accept` binds to authed user, token in **body**; `/claim/decline` no-auth token-in-body | `routes/public/claim.ts:17–42, 68–82` (API reads `request.body`, never query) | **CONFIRMED — API clean; surface is the gap** |
| ETA zeroes kitchen time at IN_DELIVERY | `etaGather.ts:79–86` (`['READY','IN_DELIVERY','PICKED_UP'] → prepRemaining = 0`) | **CONFIRMED** |
| preparing_at/ready_at stamped only on transition into those states | `orderStatusService.ts:11–18` (STATUS_AT_COLUMN) | **CONFIRMED** |
| No live consumer learns `ready_at − preparing_at` | `etaGather.ts:184–217` (`synthesizeAndPersistEtaWindow` uses configured `prep_time_minutes` + `preparing_at` decay; **does not read `ready_at`**) | **CONFIRMED — F4 is future-risk, not a live break** |

---

## B. Breaker findings — disposition

### F1 (HIGH, no-trap) — unassigned IN_DELIVERY orphan + no card recovery → **FIXED (both a + b)**

The load-bearing finding. §5 moves dispatch to right-after-Accept, which turns the rare no-courier orphan
into the routine case, and the shipped OrderCard has **no IN_DELIVERY branch** to recover it. Decision:
adopt **both** breaker options, reconciled with the already-shipped deliver-v2 offer-handshake.

**(a) Gate the transition on an actual assignment — the order never enters IN_DELIVERY without a courier.**
The §5 "Send for delivery" action does **not** route through the raw status-PATCH whose auto-assign
silently no-ops (`orders.ts:785–821`). It routes through the **dispatch primitive** whose contract is *attempt
to assign*, with two honest outcomes:
- **Courier available** → with the deliver-v2 offer-handshake flag ON, an `offered` assignment is written and
  the order **stays CONFIRMED** until the courier accepts; the existing `/accept` route then drives
  `→ IN_DELIVERY` (`dashboard.ts:326–337` already does exactly this — "NOT driven to IN_DELIVERY … until
  acceptance"). No orphan window.
- **No courier available** → the primitive returns `{ dispatched: false, reason: 'no_courier' }` and the
  order **stays CONFIRMED** (or READY). The OrderCard then renders an explicit **"awaiting courier / assign"**
  affordance instead of a silent 200. The order is never left in IN_DELIVERY-with-no-courier.

This makes the no-courier case a *visible pending state on a legal status*, not an invisible orphan on a
terminal-ish one. It also means the raw auto-assign-then-silently-skip path (`orders.ts:785–821`) is **no
longer the default owner surface** — when it does run (legacy / handshake-off), F1(b) catches the residue.

**(b) Add an IN_DELIVERY recovery branch to OrderCard.** Render **re-assign** and **revert-to-READY**
(both already exist server-side: `dashboard.ts:214` assign-courier accepts CONFIRMED/PREPARING/READY and
`:284–288` reverts IN_DELIVERY→READY) whenever `status==='IN_DELIVERY'`. This closes a **latent bug that
exists today** (the breaker is right that the card has *zero* IN_DELIVERY action even on the current
READY→Assign path), so an unassigned IN_DELIVERY — from any path, legacy or new — is never a dead end.

**Guardrail (red→green), owner: implementer:**
- **G-F1a** (API/integration): owner dispatches with **zero available couriers** → assert the order
  `status` stays `'CONFIRMED'` (NOT `'IN_DELIVERY'`) and the response carries `dispatched=false`. *Red today*
  (raw PATCH leaves an IN_DELIVERY orphan at 200).
- **G-F1b** (component/E2E): an order with `status==='IN_DELIVERY'` and no live assignment renders a visible
  recovery action (`[data-testid=order-reassign]` and/or revert). *Red today* (no IN_DELIVERY branch in
  `OrderCard.tsx`).

**Red-line satisfied:** no-trap — an order can never reach IN_DELIVERY with no courier and no recovery
affordance. (a) prevents the orphan from forming; (b) recovers any that pre-exist.

---

### F2 (HIGH, security) — claim token transport leak → **FIXED (transport delta only; `claim_transfer` untouched)**

The API is already clean (token in **body**, `claim.ts:21,72`). The break is the **web surface delta**: a
`/claim/:token` or `/claim?token=…` page would write the 256-bit sole-authority token into URL → history →
Referer (to map-tile/font/analytics third parties) → CDN/access logs, within the 72h TTL, re-opening the
ownership-theft and griefing-erase vectors. Resolution — specify the surface transport:

1. **Delivery via URL fragment, then immediate scrub.** The operator claim link is `…/claim#t=<token>`. The
   **fragment is never sent in the Referer header (per spec), never in server/CDN access logs, never in the
   query string.** On mount the SPA (a) reads `location.hash`, (b) `history.replaceState(null,'','/claim')`
   to remove it from the address bar and the history entry **before any third-party resource loads**, (c)
   holds the token in an **in-memory variable only** (never `localStorage`/`sessionStorage` — avoids
   persistence + XSS-exfil surface; zero-cookie red-line already forbids cookies).
2. **Preview = read-only, token-in-body, no enumeration.** Resolving the token to its working preview is a
   POST with the token in the **body** (returns only already-public shadow menu/branding — no PII), generic
   on a bad/expired/used token (never reveal whether a slug is a claimable shadow, K2). This is the one
   net-new endpoint the surface needs; it obeys the same body-only + no-enumeration + rate-limit rules as
   the shipped `/claim/*`.
3. **Accept = authenticate first, then bind server-side.** The owner logs in / OTPs to a real RS256 JWT
   **before** accept; the SPA then POSTs `{ token }` in the **body** to the shipped
   `/claim/accept` (`verifyAuth`-only, binds to the authed `userId`, `claim.ts:23–26`). Token never touches
   the query string. Role re-derives post-claim (ADR-0004); the page honours `reauth: true`.
4. **Decline stays no-auth token-only** (H-decline — the unconsented restaurant erases without an account),
   POST body, from the in-memory fragment value, rate-limited (already `max 10/min`, `claim.ts:70`). The
   griefing concern is **closed by transport** (the token no longer leaks via URL/Referer/logs); "anyone with
   the token can erase" remains *by design* because the token is delivered only to the restaurant's
   verified contact.

**Guardrail (red→green), owner: implementer:**
- **G-F2a** (E2E): after the claim page mounts, `expect(page.url())` contains **neither** the token in
  `search` **nor** in `hash`; the address bar is `/claim`.
- **G-F2b** (E2E request-intercept): **no** outbound request URL contains the token and **no** Referer header
  carries it; the token appears **only** in POST request bodies.
- **G-F2c** (API): `/claim/accept?token=…` with an empty body → `400 VALIDATION_FAILED` (proves query is
  ignored — the server reads body only).

**Red-line satisfied:** token = sole authority is never placed on a transport that leaks to third
parties/logs; `claim_transfer` SECURITY-DEFINER and the no-inline-UPDATE rule are untouched (surface calls
`acceptClaim` only).

---

### F3 (MED) — customer ETA lies (kitchen time zeroes at IN_DELIVERY before food exists) → **FIXED**

Drive the customer's remaining-kitchen-time from **timestamps, not the status label.** Combined with the
F4 fix (auto-stamp `preparing_at` on Accept), `prepRemaining` becomes
`max(0, prep − minutesSince(preparing_at))` whenever `preparing_at` is set, and only hard-zeroes when there
is an authoritative "food out of kitchen" signal — i.e. `ready_at IS NOT NULL` (kitchen-flow) **or** a real
courier pickup. On the 2-tap path (IN_DELIVERY before ready, `ready_at` NULL) the time-decay governs, so the
ETA no longer claims the food left the kitchen the instant the owner taps dispatch. This is strictly *more*
honest for the normal flow too (decay by actual cook time rather than a status heuristic).

**Guardrail G-F3** (unit, `etaGather`): a `CONFIRMED→IN_DELIVERY` order with `preparing_at = now()−2min`,
`ready_at = NULL`, `prep = 15` returns `prepRemaining ≈ 13` (NOT 0). *Red today* (`etaGather.ts:81` forces 0
at IN_DELIVERY). Owner: implementer.

---

### F4 (MED) — preparing_at/ready_at NULL forever starves prep-learning → **FIXED (auto-stamp) + DEFER-FLAG (observed-ready learning)**

Re-grounding settles severity: **there is no live consumer of `ready_at − preparing_at`** today
(`synthesizeAndPersistEtaWindow` uses configured `prep_time_minutes` + `preparing_at` decay, never `ready_at`
— `etaGather.ts:184–217`). So nothing is *currently* starved; the risk is future. Two-part disposition:

- **FIXED now — auto-stamp `preparing_at` on Accept.** In the 2-tap default, owner Accept (→CONFIRMED) is
  the start-cooking signal, so stamp `preparing_at = now()` alongside `confirmed_at`. This restores the
  decay base for F3 and the customer "preparing" signal (F5) with **no migration** (`preparing_at` column
  exists, `orderStatusService.ts:13`). Use `COALESCE` so a later explicit PREPARING transition
  (kitchen-flow toggle) never double-stamps.
- **DEFER-FLAG — observed kitchen-duration learning.** `ready_at` is genuinely absent on 2-tap orders by
  design (no READY beat). When a duration-learner is built, it **must filter to `ready_at IS NOT NULL`
  samples** (never treat NULL as a 0-minute cook) — otherwise the corpus biases toward zero. Observed-ready
  learning resumes when the deferred `kitchen_flow_enabled` toggle is wired (which re-introduces the explicit
  READY stamp). Tracked as an open item, owner: product (toggle) + implementer (learner filter rule).

**Guardrail G-F4** (unit): if/when the learner lands, a NULL-`ready_at` order is **excluded** from the
duration sample. Recorded as a *pre-condition on the future learner*, not a gate on this patch.

---

### F5 (MED) — progress bar asserts kitchen stages that never happened → **FIXED**

`OrderProgress` must mark each step "done" from **timestamp presence**, not `statusIndex`. A step is checked
only if its `at` timestamp is non-NULL; otherwise it renders as current/pending even when `statusIndex` is
past it. On a 2-tap order (`confirmed_at` set, `preparing_at` set via F4, `ready_at` NULL): Confirmed ✓,
Preparing active, **Ready is not shown as a passed ✓ with a blank time.** The bar shows what actually
happened, never a checkmark over a stage the kitchen never entered.

**Guardrail G-F5** (component): `OrderProgress` with `status='IN_DELIVERY'`, `readyAt=null` → the Ready step
is **not** rendered completed/✓ (no checkmark + blank timestamp). *Red today* (`OrderProgress.tsx:84–102`
fills every dot ≤ statusIndex). Owner: implementer.

---

### F6 (MED) — browser-Back from open checkout panel exits the storefront → **FIXED**

Make the open panel a **history entry** (the standard modal-as-history pattern), so browser Back closes the
panel instead of leaving `/s/:slug`:
- Opening the panel = `pushState('?checkout=1')` (one entry).
- Closing the panel (close button **or** browser Back/`popstate`) = pop → menu, cart intact, still on
  `/s/:slug`.
- The retired `/s/:slug/checkout` deep-link does `history.replaceState` → `/s/:slug?checkout=1` (the dead
  route never lingers in history; no back-stack growth per toggle).

**Guardrail G-F6** (E2E): with the panel open, browser **Back** → menu visible, URL still `/s/:slug`
(storefront not exited), cart intact. Owner: implementer. **No-trap satisfied.**

---

### F7 (LOW) — "≈5 actions" omits preflight-ack round-trips + the open required-field floor → **ACCEPT-RISK (with disclosure)**

The ≈5 figure is a *target with a stated dependency*, not a settled count. Accepted with explicit
disclosure in the proposal: the count holds **only** under the §3 floor decision (below) and **excludes**
conditional E27 preflight-acknowledge taps (`legacy.ts:70–71`), which appear only on distance-tier /
far-address orders. The honest framing: new-order ≈5 *when no preflight ack fires and entrance/apt are
optional*; +1 render+tap when a preflight code returns. Owner: architect (disclosure) — recorded in §0.

---

### F8 (LOW) — error message path shifts "type required" → "delivery required" → **ACCEPT-RISK**

Cosmetic envelope change only: with `.default('delivery')`, omitting both `type` and `delivery` now fails the
superRefine (`legacy.ts:74–76`, path `['delivery']`) instead of the enum. The order still 400s (floor pin
enforced); `requestHash` is unaffected (it hashes the **resolved** value, `order-canonical.ts:42`). Accepted;
the R4 hash-stability guardrail (kept) is good hygiene, not guarding a live risk. Owner: implementer (update
any test asserting the old `type`-missing issue in the same change).

---

### F9 (LOW) — redirect-seam state matrix (empty / stale / mid-order deep-link) → **FIXED (matrix specified)**

The reconcile trigger moves from route-mount to **panel-open**; specify the matrix so the seam never strands:

| Deep-link state | Behaviour |
|---|---|
| **Empty cart** (`?checkout=1`, no stored items) | Render menu, **panel closed**, cart intact-empty. Nothing to check out → no panel. |
| **Stale cart** (items removed / price-changed) | Run the existing `cartReconcile` (`CartProvider.tsx:62–68`) **on panel-open, before the total renders**. All items dropped → panel closes to menu; prices adjusted → panel shows reconciled total + the existing "prices updated" notice. |
| **Mid-order deep-link** (order already placed) | No order exists pre-confirm; `clearCart()` runs only on success (`CheckoutPage.tsx:510`) → a stale draft with no cart yields a **closed** panel. No partial-order state. |

**Guardrail G-F9** (E2E): deep-link `?checkout=1` with empty cart → panel **not** visible, menu visible.
Owner: implementer. **No-trap satisfied.**

---

## C. Counsel points — disposition

### Counsel §1/§2/§9 (running total) — the steel-man → **REVISED (design changed): keep the running subtotal early**

Counsel is right and grounded: in a **cash-as-proof** product the running subtotal is *signal, not
scaffolding* — it is the instrument a cash-constrained customer uses to shop within the money they can produce
at the door. The patch's "feeKnown=false ambiguity" argument cuts the **other** way (it argues for showing
cost early with honest labeling, not hiding it). The §2 decision changes:

- **Cut the cart *drawer* (the genuine redundant surface).** Keep the **running items-subtotal on the cart
  bar** ("N items · {subtotal}"). The subtotal is always-known, integer, server-mirrorable — it carries no
  `feeKnown=false` ambiguity because it is **items only**, not the all-in total.
- The **delivery fee** (small, distance-tier, sometimes unknown) resolves in the panel: "+ delivery fee at
  checkout" when `feeKnown=false`, exact otherwise. The cash-422 backstop + server total stay authoritative
  (ADR-0005).

This threads the needle: the cash customer sees what their *food* costs as they build (the number they most
need), and the fee resolves honestly at the decision point — never hidden until confirm. It also restores
coherence with the patch's own principle (cut scaffolding, keep signal). **Owner: product ratifies the
copy; architect changed the design.**

### Counsel §2/§5/§10 (customer progress signal + IN_DELIVERY honesty) → **REVISED (design changed)** — see F3/F4/F5

The owner saves the tap; the customer keeps the signal and the model keeps its timestamps. Resolved by:
auto-stamp `preparing_at` on Accept (F4) → a lightweight customer "preparing" beat **decoupled from the
owner's tap burden**; timestamp-driven ETA (F3) and progress bar (F5) so the IN_DELIVERY label never
over-promises "out for delivery" before the food is made. Counsel's two care costs (lost progress signal,
label-stops-being-true) are both mitigated without re-imposing the owner READY tap.

### Counsel §3 (optional fields — "optional must mean skippable, never hidden") → **REVISED (R3 decided): optional-but-inviting**

Decision on the R3 open item: **required floor = phone + a droppable location (map-pin primary OR text
address).** `entrance` / `apartment` / `notes` are **optional but contextually present** — rendered inline
as clearly-optional fields (e.g. placeholder "Apartment / entrance (optional)"), **never buried behind a
"more" toggle.** This honours both dignity arguments: it spares the confident repeat customer two taps while
ensuring the least-served customer (limited phone comfort / language barrier / hearing difficulty /
buzzer-only) — for whom "the courier can call" is the failure, not the safety net — is not fighting a hidden
field to specify their door. **Routed to product/ops for ratification** (the only item needing a human sign-off
on the business question "should entrance/apt be hard-required?"); architect default + recommendation =
optional-but-visible.

### Counsel §4 (binding ratification condition — PROTECTED FRICTION) → **RECORDED in the ADR**

Two things recorded in ADR Decision §6:
1. The council concurs with the **revised** ADR §6 only; the proposal's original "one action … goes live"
   prose is **not a build source.**
2. The §6 **claim → review → publish** three-act consent sequence (CC2), and **allergen confirmation as a
   distinct deliberate act into empty fields** (CC3), are annotated as **PROTECTED FRICTION** — friction that
   exists for consent and allergen safety, **explicitly distinct from the incidental cart/page friction this
   patch removes** — so a future simplification pass never mistakes the consent gate for scaffolding.

Plus the two §6-surface dignity notes (RECORDED, to bind the implementer): **CC1 sequencing** — the honest
Art-14 notice ("you didn't ask for this; here's what we did and your options") dominates the **first** screen;
the working preview comes **second** (preview-before-notice would launder the consent). **H-decline parity** —
decline stays equally prominent and account-free; claim-louder-than-decline is the dark-pattern tell the P6
verdict already named.

### Counsel §6 (operator effort on the ledger) → **RECORDED (design changed: ledger row added)**

The action-count table now carries the **operator +N** row (scrape → AI build → branded shadow →
hostile-recipient Art-14 notice, per shadow). The simplification is real for customer (−3) and owner (−2)
**because** complexity concentrates on a third party — a legitimate first-wave "do things that don't scale"
bootstrap, but it appears **on the ledger**, not only in prose.

### Counsel §6 (guard the exit — keep self-serve reversible) → **RECORDED (R10 tightened)**

Self-serve onboarding is de-emphasized **but kept test-warm and reversible**: its E2E stays green and its
seam stays warm, so the operator-effort bootstrap does not quietly become the only path the week operator
capacity runs out. Owner: product. Tracked as R10 (tightened).

### Counsel §8 (ETHICAL-STOP test) → **NO STOP** — acknowledged, no change beyond the binding condition above.

### Counsel §7/§10 (agent-health: "mostly already built" risks under-caring the consent UX) → **ACKNOWLEDGED**

No design change; recorded as guidance: the claim **surface + K4 allergen writer** is net-new and is the
entire ethical payload of §6 — the care budget stays on the new surface, not the shipped bytes.

---

## D. Disposition summary table

| ID | Sev | Finding (one line) | Disposition | Proof / where |
|---|---|---|---|---|
| F1 | HIGH | Unassigned IN_DELIVERY orphan + no card recovery | **FIXED (a+b)** | proposal §5 (gated dispatch + IN_DELIVERY recovery branch); G-F1a/b |
| F2 | HIGH | Claim token leaks via URL/Referer/logs | **FIXED (transport)** | proposal §6 (fragment+scrub, body-only, auth-first); G-F2a/b/c |
| F3 | MED | Customer ETA lies (zeroes at IN_DELIVERY) | **FIXED** | proposal §5 (timestamp-driven ETA); G-F3 |
| F4 | MED | preparing_at/ready_at NULL starves learning | **FIXED + DEFER-FLAG** | proposal §5 (auto-stamp preparing_at); G-F4 (future learner filter) |
| F5 | MED | Progress bar asserts stages that never happened | **FIXED** | proposal §5 (timestamp-driven dots); G-F5 |
| F6 | MED | Browser-Back exits storefront | **FIXED** | proposal §1 (panel = history entry); G-F6 |
| F7 | LOW | ≈5 omits preflight ack + open floor | **ACCEPT-RISK** | proposal §0 (disclosure) |
| F8 | LOW | Error message path shift | **ACCEPT-RISK** | proposal §4 (R4 hash guardrail kept) |
| F9 | LOW | Redirect-seam empty/stale/mid-order matrix | **FIXED** | proposal §1 (matrix); G-F9 |
| C§1/2/9 | — | Running subtotal removed (signal, not chrome) | **REVISED — keep subtotal** | proposal §2 (Option B-hybrid) |
| C§3 | — | Optional fields must be skippable not hidden | **REVISED — optional-but-inviting** | proposal §3 (R3 decided); **human ratify** |
| C§4 | — | PROTECTED FRICTION + revised §6 only | **RECORDED** | ADR §6 |
| C§6a | — | Operator +N absent from the metric | **RECORDED — ledger row** | proposal §0 table |
| C§6b | — | Keep self-serve reversible | **RECORDED — R10 tightened** | proposal §6 R10 |

---

## E. Red-lines re-verified intact

- **No-trap:** F1 (gated dispatch + IN_DELIVERY recovery), F6 (Back stays on storefront), F9 (no
  empty/stale strand) — all green. ✅
- **Consent-before-publish:** §6 unchanged; three-act gate held + annotated PROTECTED FRICTION. ✅
- **Server-authoritative:** money total + cash-422 + state machine still authoritative; §2 keeps only the
  *items* subtotal (a server-mirrorable display), §5 ETA reads server timestamps. ✅
- **Money-integer:** untouched; no math changed. ✅
- **RLS FORCE:** no schema change in this round; deferred `kitchen_flow_enabled` + any K4 writer inherit
  FORCE. ✅
- **claim_transfer untouched:** F2 protects the *surface*; SECURITY-DEFINER fn + no-inline-UPDATE unchanged. ✅
- **i18n al/en:** new strings (subtotal label, fee-at-checkout, "awaiting courier"/reassign, claim Art-14
  notice, optional-field placeholders) ship via `scripts/i18n-add.ts` + parity gate. ✅

---

## F. Convergence

**CONVERGED — design-level.** All 9 breaker findings and all Counsel points are dispositioned with a fix,
an accepted-risk justification, or a tracked defer-flag; both HIGHs changed the design (no longer "preserved
behavior" hand-waving). Five new red→green guardrails (G-F1a, G-F1b, G-F2a/b/c, G-F3, G-F5, G-F6, G-F9) plus
the retained §4 requestHash guardrail are the build-time gate — code is **not** written; these prove the
fixes when it is.

**One item needs a human decision:** §3 required-field floor (entrance/apartment **hard-required** vs
**optional-but-inviting**) — a product/ops business call. Architect recommends optional-but-inviting; the
patch does not advance to build on that field until product ratifies.

**DEFER-FLAG tracked:** F4 observed kitchen-duration learning (no live consumer today; resumes with the
deferred `kitchen_flow_enabled` toggle; future learner must filter `ready_at IS NOT NULL`).

---

# RESOLVE round 2 — re-grounding the round-1 HIGH fixes against REAL contracts

**Seat:** Architect · **Date:** 2026-06-28 · design-only, no production code.
**Inputs:** breaker `RE-ATTACK round 2` (R2-1…R2-8), counsel `RE-EXAMINE round 2` (R2.1…R2.6), round-1
`proposal.md`/`resolution.md`. **Re-grounded @ HEAD** (`feat/mvp-sensor-seams`) against files actually read
this round (file:line below). The breaker is correct: two round-1 fixes (F1, F2) were grounded on a
**dispatch primitive that does not exist** and a **recipient-check that round-1 said could not exist**. Both
are now re-grounded against the live mechanisms. The `preparing_at`-auto-stamp (round-1 F4) is **re-decided**.

## A2. Re-grounding (the REAL contracts each round-2 disposition stands on)

| Claim | Live source @ HEAD (read this round) | Status |
|---|---|---|
| `/assign-courier` **requires explicit `courierId`** (400 if missing), 404 on order-not-found, **409 if status ∉ {CONFIRMED,PREPARING,READY}** (i.e. rejects IN_DELIVERY), 404 if courier not active. Never auto-discovers, never returns `dispatched:false`. | `dashboard.ts:219–222` (400), `:232` (404), `:235–237` (409), `:246` (404), returns `{success,offered,assignmentId}` `:337` / `{id,orderId,courierId,status:'assigned'}` `:368` | **CONFIRMED — round-1 "dispatch primitive returns dispatched:false" is FICTIONAL** |
| The PATCH status handler calls `updateOrderStatus(IN_DELIVERY)` **unconditionally FIRST** (`:779`), THEN looks up a courier (`:786–799`), INSERTs the assignment only if `rowCount>0` (`:800`); on `rowCount=0` the order is **already IN_DELIVERY**, no assignment, returns `{id, status:newStatus}` 200 = the orphan. | `orders.ts:779, 786–800, 824` | **CONFIRMED — this is the auto-discover path AND the only path the card can reach** |
| `OrderCard` has **one** mutation callback `onUpdateStatus(id,newStatus)` → PATCH `/orders/:id/status`; actions only for PENDING/CONFIRMED/PREPARING/READY; READY → `handleAction('IN_DELIVERY')`; **no IN_DELIVERY branch**, **no path to `/assign-courier`** (which needs `courierId`). | `OrderCard.tsx:12, 29–33, 221–236` | **CONFIRMED** |
| The offer-handshake branch is gated on `COURIER_OFFER_HANDSHAKE_ENABLED==='true'`; OFF (shipped default) force-inserts `accepted` + drives `updateOrderStatus(IN_DELIVERY)`. This branch is on `/assign-courier`, **NOT** the card's PATCH path. | `dashboard.ts:322, 340–353` | **CONFIRMED — handshake is orthogonal to the §5 card surface** |
| `updateOrderStatus(IN_DELIVERY→READY)` runs the **central deliver-v2 fold** that terminalizes the active courier_assignment + frees the shift in the same tx, **idempotently** (already used by owner-reassign `dashboard.ts:288`). | `orderStatusService.ts:129–140` | **CONFIRMED — this is the working IN_DELIVERY recovery transition** |
| The claim invite **carries an intended-recipient identity**: `invited_contact_hash`; `claim_transfer` enforces `sha256(lower(trim(users.email)))==invited_contact_hash` → `CLAIMERR:CONTACT_MISMATCH` (→403) **when the hash is non-NULL**; token-only when NULL. | `migrations/…071:27,56,62–69`; route maps `CONTACT_MISMATCH→403` `claim.ts:35`; `mintClaimInvite(invitedContact?)` `claim.ts(module):50–67` | **CONFIRMED — round-1 "no recipient check / cannot exist" is FALSE; a real email-match defense exists, gated on the operator minting with the contact** |
| `kitchenAhead` SUMs only orders `WHERE status='PREPARING'`; 2-tap orders are CONFIRMED→IN_DELIVERY, **never status='PREPARING'**, so they are excluded entirely. ETA decay only fires for `status='PREPARING'`; `confirmed_at` is stamped at CONFIRMED. | `etaGather.ts:81–86, 93–101`; `orderStatusService.ts:92` | **CONFIRMED — no kitchenAhead skew from a 2-tap order; defuses R2-5 compounding** |

## B2. Round-2 dispositions

### R2-1 (HIGH) — the dispatch primitive is fictional → **FIXED (re-grounded; relocated to the REAL endpoint)**

Round-1's "`dashboard.ts:214` returns `{dispatched:false}` and keeps CONFIRMED" is **retracted** — no
endpoint produces that shape (`/assign-courier` requires `courierId`, 404/409s, never auto-discovers). The
only auto-discovering path is `orders.ts:785`, which is also **the only endpoint the card's single
`onUpdateStatus` callback can reach.** So the fix must land **there**, not on a primitive that doesn't exist.

**Decision — option (b), made honest on the real endpoint, plus (c) recovery.** Reorder the PATCH
`/orders/:id/status` handler so that for `newStatus==='IN_DELIVERY' && type==='delivery'` the
**courier-availability lookup runs BEFORE** `updateOrderStatus(IN_DELIVERY)`:
- **courier found** → `updateOrderStatus(IN_DELIVERY)` + INSERT assignment + shift→`on_delivery` (exactly
  today's body, just reordered). Transition `CONFIRMED→IN_DELIVERY` (already-legal edge,
  `order-machine.ts:20`).
- **`rowCount===0` (no courier)** → **do NOT call `updateOrderStatus`**; leave the order at its current
  status; return `{ id, status: <currentStatus>, dispatched: false, reason: 'no_courier' }` (HTTP 200,
  **server-authoritative — the response reports the REAL status, not the requested one**). The order **never
  enters IN_DELIVERY with no courier.**
- For all other `newStatus` values the handler is unchanged.

This needs **no courier-picker** (the lookup auto-selects the sole courier in the single-courier MVP shop)
and **no new card plumbing** — §5's "Send for delivery" on CONFIRMED wires through the existing
`onUpdateStatus('IN_DELIVERY')` unchanged, now safe by construction.

**Option (a) explicit-assign-only weighed & deferred:** picking a courier (or auto-selecting the sole one)
via `/assign-courier` needs a *second* card callback + a `courierId` picker (the card has neither). It is the
right surface for **multi-courier** dispatch UX later; for the MVP it is strictly heavier than making the
endpoint the card already calls honest. Deferred, recorded.

**Guardrail G-F1a (REVISED to the real shape):** PATCH `/orders/:id/status` `{status:'IN_DELIVERY'}` against
a location with **zero available couriers** → response body `{ status:'CONFIRMED', dispatched:false,
reason:'no_courier' }` **and** `SELECT status FROM orders WHERE id=…` still `'CONFIRMED'`. *Red today*
(`orders.ts:779` already flipped the row to IN_DELIVERY and `:824` returns `{status:'IN_DELIVERY'}`).
Asserts the **real** endpoint's real return shape — not the fictional `dashboard.ts` primitive.

### R2-2 (HIGH) — cross-flag dependency → **FIXED (option a: §5 is orphan-safe on its own; matrix declared)**

The no-orphan property in R2-1 comes **entirely** from the `orders.ts:785` honesty fix, which has **zero flag
dependency**. The card never routes through `/assign-courier`, so `COURIER_OFFER_HANDSHAKE_ENABLED`
(`dashboard.ts:322`) is **orthogonal** to the §5 card surface. §5 launches under `OWNER_TWO_TAP`
independently; it does **not** require the handshake ON.

**Flag-interaction matrix — `OWNER_TWO_TAP` × `COURIER_OFFER_HANDSHAKE_ENABLED` (after the R2-1 fix); every cell non-orphaning:**

| | handshake **OFF** (shipped default) | handshake **ON** |
|---|---|---|
| **two-tap OFF** | Card READY→Assign → `orders.ts:785` honest: courier→IN_DELIVERY+assign; none→stays READY, `dispatched:false`. **No orphan** (also fixes today's latent READY-path orphan). | Same card path (PATCH status, **not** `/assign-courier`) — handshake unused by the card. **No orphan.** |
| **two-tap ON** | Card CONFIRMED→"Send for delivery" → `orders.ts:785` honest: courier→IN_DELIVERY+assign; none→stays CONFIRMED, `dispatched:false`. **No orphan.** | Same card path; handshake unused by the card. **No orphan.** The separate manual-pick `/assign-courier` handshake surface is independently non-orphaning (`offered` keeps the order CONFIRMED until accept, `dashboard.ts:324–337`). |

**Declared dependency:** none. §5 does **not** couple to the handshake; the handshake remains a concern of
the separate `/assign-courier` manual-pick surface. Reconciliation with shipped deliver-v2: the central fold
(`orderStatusService.ts:129–140`) covers both surfaces.

### R2-3 (HIGH) — first-time-owner claim with no account → **FIXED (re-grounded; in-page fetch-auth + REAL recipient check)**

Two round-1 errors corrected by live source:
1. **The recipient check exists.** `claim_transfer` (`migrations/…071:62–69`) enforces the authed user's
   `email` sha256 == `invited_contact_hash` → `CONTACT_MISMATCH`/403 **when the hash is non-NULL**. Round-1's
   "there cannot be a recipient check" is false.
2. **The token-loss contradiction dissolves in an SPA.** The breaker assumed login is a **navigation** that
   destroys the in-memory token. It is not: auth is **zero-cookie JSON-token** (`/auth/*` returns the JWT in
   the body), so the SPA authenticates by **fetch, not navigation** — the `/claim` page never unloads and the
   in-memory token survives the auth step.

**Decision — the claim surface authenticates IN-PAGE, no navigation, and the operator mint binds the recipient:**
1. **Transport:** operator link `…/claim#t=<token>` (fragment — never Referer/logs/query). On mount: read
   `location.hash` → in-memory module ref → `history.replaceState(null,'','/claim')` **before any third-party
   resource / SDK init** (R2-6). **Never** `localStorage`/`sessionStorage`/cookie.
2. **Preview:** POST token-in-**body**, read-only public shadow data, generic-on-fail, no enumeration (one
   net-new endpoint, same rules as `/claim/*`).
3. **Account + accept happen in-page (no navigation):** the brand-new owner registers / OTPs via **fetch** to
   the existing `/auth/*` **with the invited email** (the contact the Art-14 notice went to); the JWT is held
   in memory. Then POST `/claim/accept` `{token}` in the **body** + `Authorization: Bearer`. Because no
   navigation occurs, the in-memory token is preserved; the token is **never** persisted or re-URL'd.
   `claim_transfer` email-match enforces authed-email == invited-contact.
4. **Recipient binding (the F2 strengthening):** the operator mint **MUST** supply `invitedContact`
   (`mintClaimInvite(invitedContact)`) so `invited_contact_hash` is non-NULL and the email-match defense is
   active. This converts "token = sole authority, binds to any authed account" into **"token + proof of
   control of the invited email."** `claim_transfer` is **UNTOUCHED** (the check already exists).
5. **Burn-then-leave:** accept sets `used_at`; any post-accept navigation to `/admin` cannot replay a
   consumed token.
6. **Decline:** in-page POST `{token}`, no auth (H-decline), leak-free by the same transport.

**Guardrails:** G-F2a/b/c retained, plus:
- **G-F2d** (integration): an invite minted via the acquisition flow has `invited_contact_hash` **NOT NULL**;
  a claim by an authed user whose email ≠ invited contact → **403 CONTACT_MISMATCH**. *Proves the recipient
  binding is active, not token-only.*
- **G-F2e** (E2E): the full claim (preview → register-with-invited-email → accept) completes with **no
  full-page navigation** between reading the fragment and POSTing accept, and **no** `localStorage`/
  `sessionStorage` key is written. *Proves the first-time-owner happy path works without persisting/re-URLing
  the token.*

### R2-4 (MED) — IN_DELIVERY recovery: re-assign 409s; only revert works → **FIXED (revert-to-READY single-step; re-dispatch is a fresh action)**

Grounded: `/assign-courier` 409s on IN_DELIVERY (`dashboard.ts:235–237`); the machine allows
IN_DELIVERY→READY; `updateOrderStatus(IN_DELIVERY→READY)` runs the central fold that terminalizes the
binding + frees the shift idempotently (`orderStatusService.ts:129–140`); the card reaches `updateOrderStatus`
via its single callback.

**Decision:** the OrderCard IN_DELIVERY branch renders **one** recovery action — **"Revert to READY"** →
`onUpdateStatus('READY')` → PATCH status → `updateOrderStatus(IN_DELIVERY→READY)` → central fold. Single
callback, single working step. It does **not** render a "re-assign" button on IN_DELIVERY (that would
409). After revert, the owner re-taps **"Send for delivery"** from READY (the honest R2-1 auto-assign) as a
**fresh** action. The two-step is honest (revert works, re-dispatch works); round-1's "single-tap re-assign
on IN_DELIVERY" is retracted (it 409s and the single callback can't carry a `courierId`).

**Reconciliation with deliver-v2 D1/D2:** the revert flows through `updateOrderStatus` (no raw path) — exactly
as `dashboard.ts:288` already does for owner-reassign — so the central fold is the single terminalization
point, present and future.

**Guardrail G-F1b (REVISED):** an order with `status==='IN_DELIVERY'` renders `[data-testid=order-revert-ready]`
whose handler calls `onUpdateStatus('READY')` (asserting against the real single-callback contract; **not** a
re-assign that 409s). *Red today* (no IN_DELIVERY branch, `OrderCard.tsx:221–236`).

### R2-5 (MED) — `preparing_at=confirmed_at` inverts honesty → **FIXED (re-decided: drop the auto-stamp; ETA off `confirmed_at`; "Preparing" is a process label)**

Round-1 F4's "auto-stamp `preparing_at` on Accept" is **superseded.** Stamping a physical "kitchen started"
event that did not occur is a **data-layer lie** (worse than a copy label — `preparing_at` is read by
`fetchOrderDelta`/`OrderProgress`), and it starts the ETA decay at accept-time.

**Re-decision:**
- **Do NOT auto-stamp `preparing_at`.** It stays NULL on 2-tap orders — honest: the kitchen never marked
  preparing.
- **F3 ETA (revised):** `prepRemaining = max(0, prep − minutesSince(preparing_at ?? confirmed_at))` while
  `ready_at IS NULL`; hard-zero only on `ready_at` non-NULL or a real pickup. On the 2-tap path
  (`preparing_at` NULL) the decay runs off **`confirmed_at`** — a REAL event (the owner accepted), not a
  fabricated one. No invented timestamp; honest decay.
- **F5 progress (revised):** the "Preparing" step renders **active/in-progress** (never ✓) when the order is
  past CONFIRMED and `ready_at IS NULL` — a **process label driven by status**, not by a fabricated
  `preparing_at`. A step shows ✓ only when its **real** timestamp is non-NULL.
- **Copy rule:** the customer state copy reads **"Preparing your order"** (process / in-progress), **never**
  "Your food is being cooked right now" (an asserted physical action). Process, not assertion.
- **kitchenAhead — grounded clean:** the SUM filters `status='PREPARING'` (`etaGather.ts:101`); 2-tap orders
  are never status='PREPARING', so they never enter the estimator and **no zero-interval skew exists.** The
  breaker's compounding does not materialize at HEAD.

This still satisfies Counsel's care concern (the customer keeps a "Preparing" beat + a live ETA across the
confirmed→moving gap) **without** fabricating data — strictly stronger than the round-1 copy-only mitigation.

**Guardrail G-F3 (REVISED):** a CONFIRMED order with `confirmed_at = now()−2min`, `preparing_at NULL`,
`ready_at NULL`, `prep=15` → `prepRemaining ≈ 13` (NOT 0, NOT a flat 15). *Red today* (`etaGather.ts:81–86`:
CONFIRMED is neither in the zero-list nor PREPARING → returns the flat `orderPrep=15`, no decay). G-F5
unchanged (Ready not ✓ when `ready_at` NULL).

### R2-6 (MED) — fragment scrub races boot telemetry → **FIXED (concrete: exclude /claim from telemetry + scrub as the first pre-init statement)**

The `/claim` surface is net-new; we control its boot. **Both** measures (defense in depth):
1. **The `/claim` route is on the telemetry exclusion list** — no app-level error-SDK / page-view beacon
   initializes on it (it reads no `location.href` for that route).
2. **Scrub-before-init:** a synchronous pre-init shim (top of the entry module, before any SDK import/init)
   runs `if (location.pathname==='/claim' && location.hash) { __claimToken = parse(location.hash);
   history.replaceState(null,'','/claim'); }` **before** any code that could read `location.href`. Telemetry,
   if ever present, then sees a scrubbed href.

**Guardrail G-F2f:** a boot-order assertion that the scrub shim executes before any telemetry/SDK init on
`/claim` (the analytics init is either not invoked on `/claim` or only after the href is scrubbed); combined
with G-F2b (request-intercept) confirming **no** outbound body/URL/Referer — incl. the first pageload beacon —
carries the token.

### R2-7 (MED) + Counsel — subtotal-without-fee → **FIXED (copy + sequencing rule)**

Grounded: the fee is distance-tier (`feeKnown=false` until an address exists); preflight surcharge codes ride
`acknowledged_codes` (`legacy.ts:70–71`). Counsel R2.1 concurs subtotal-only is the **honest floor** (you
cannot surface a fee you do not yet have at browse time). Rule:
- **The bar label reads as a subtotal, never a bare all-in price.** "N items · {subtotal}" carries an explicit
  **subtotal / nën-total** token (i18n al/en) so it cannot masquerade as the total.
- **The fee surfaces in the panel the INSTANT a tier is known** — on **pin-drop / address-resolve** — never
  deferred to the confirm tap. Where the fee is flat/known, show it on **panel entry**. While `feeKnown=false`
  the panel shows "+ delivery fee at checkout". Preflight surcharge (`acknowledged_codes`) surfaces at the
  **same** moment (address-resolve), not at confirm.
- **Sequencing:** subtotal (browse) → fee resolves on address (panel) → all-in total before confirm. The
  customer never meets a number that grows at the final tap. Server total + cash-422 stay authoritative.

**Guardrail G-§2:** E2E — the bar text contains the subtotal i18n token (not a bare currency reading as
total); on address-resolve the panel renders the fee/all-in line **before** the confirm action is enabled.

### §3 (human decision) — three-way floor → **NEEDS-HUMAN (architect recommends contextually-required, pin-confidence-gated)**

Per Counsel R2.3 the floor decision is recorded as a **three-way** choice, not a binary:
**hard-required | optional-but-inviting | contextually-required (pin-confidence-gated).**

**Architect recommendation: contextually-required.** entrance/apartment are **required when the map-pin is
low-confidence** (multi-unit / area-level geocode, pin far from a snapped address) and **optional when
high-confidence** (single-unit, pin snapped to a known address). Rationale: friction proportional to real
last-50-metre failure risk — it protects the vulnerable *non-filler* (the system asks for them exactly when
omission would cause a failed delivery + the phone call they cannot take) without taxing the confident user.
**Server-tolerant: a client-side conditional gate, no contract change** (the server already accepts the order
without these fields — proposal §3, grounded). Floor stays **optional-but-inviting** if a pin-confidence
signal is not readily available at the seam.

**NEEDS-HUMAN:** product/ops ratify (a) the business rule and (b) the pin-confidence threshold. Build does not
advance on this field until ratified.

### PROTECTED FRICTION — durable code-level guardrail → **RECORDED as a build-time guardrail (not prose)**

The consent gate must survive the **next** simplification pass as code, not prose. At the claim/activation
seam, when built:
- A **named in-code PROTECTED-FRICTION marker** on the three-act sequence (CC2) and the allergen-confirmation
  act into empty fields (CC3) — e.g. `// PROTECTED-FRICTION (P6 council CC2/CC3): consent + allergen gate —
  do not collapse`.
- **G-PF1** (guardrail): `published_at` stays NULL through claim — activation requires `menu_confirmed_at`
  (structurally true today; assert it red→green so a future fold can't quietly publish).
- **G-PF2** (guardrail): allergen confirmation is a **distinct authenticated act that writes only into empty
  fields** — assert an AI guess is never auto-confirmed and the act is not folded into `accept`/`publish`.
- Added to the §6 build-time guardrail set so collapsing the gate trips a **deterministic red**, not just a
  prose warning.

## C2. Out-of-scope defect observed (recorded, not in the §5 path) — **DEFER-FLAG**

`courier-dispatch.ts:76` calls `this.boss.send(...)` but the constructor stores `this.queue` (no `this.boss`)
— the async re-enqueue path would throw on the first no-courier retry. **Not** on the §5 card surface (the
card never enqueues this worker), so it does not affect any round-2 disposition, but it is a real latent bug
in the async dispatch worker. Owner: implementer (separate fix + regression).

## D2. Round-2 disposition table

| ID | Sev | Finding (one line) | Disposition | Real grounding @ HEAD |
|---|---|---|---|---|
| R2-1 | HIGH | Dispatch primitive is fictional | **FIXED — relocate to `orders.ts:785`, made honest** | `dashboard.ts:219–246` (no auto-discover), `orders.ts:779,786–800,824` (orphan), `OrderCard.tsx:12,221–236` (single callback); G-F1a revised |
| R2-2 | HIGH | No-orphan silently flag-coupled to handshake | **FIXED — decoupled (option a); 4-cell matrix** | `dashboard.ts:322,340–353` (handshake on a different endpoint than the card) |
| R2-3 | HIGH | Auth-first vs no-account self-contradiction | **FIXED — in-page fetch-auth + real recipient check** | `migrations/…071:62–69` (CONTACT_MISMATCH), `claim.ts:17–42`, `claim(module):50–67`; G-F2d/e |
| R2-4 | MED | Re-assign 409s; only revert works | **FIXED — revert-to-READY single-step; re-dispatch fresh** | `dashboard.ts:235–237` (409), `orderStatusService.ts:129–140` (fold); G-F1b revised |
| R2-5 | MED | `preparing_at=confirmed_at` inverts honesty | **FIXED — re-decided: no auto-stamp; ETA off `confirmed_at`; copy label** | `etaGather.ts:81–86,93–101`, `orderStatusService.ts:11–18,92`; G-F3 revised |
| R2-6 | MED | Scrub races boot telemetry | **FIXED — exclude /claim from telemetry + scrub pre-init** | net-new surface; G-F2f |
| R2-7 | MED | Subtotal under-states cash-at-door by fee | **FIXED — subtotal label + fee on tier-known, never deferred** | `legacy.ts:70–71`; G-§2 |
| §3 | — | Field floor three-way | **NEEDS-HUMAN — recommend contextually-required** | server-tolerant, no contract change |
| PF | — | PROTECTED FRICTION durability | **RECORDED — code-level marker + G-PF1/G-PF2** | `published_at`/`menu_confirmed_at` gate |
| R2-8 | LOW | Optional fields strand the least-served pre-ratification | **DEFER — folded into §3 NEEDS-HUMAN** | build does not advance pre-ratification |
| (obs) | — | `courier-dispatch.ts:76 this.boss` | **DEFER-FLAG** | not on the §5 path |

## E2. Red-lines re-verified intact (round 2)

- **No-trap:** R2-1 (honest endpoint keeps CONFIRMED on no-courier) + R2-4 (working revert recovery) — an
  order can never reach IN_DELIVERY with no courier and no recovery, under **every** flag cell. ✅
- **Server-authoritative:** R2-1 returns the **real** status (not the requested one); §2 fee + cash-422 +
  machine authoritative; R2-5 ETA reads real timestamps (`confirmed_at`), fabricates none. ✅
- **claim_transfer untouched:** R2-3 strengthens via the **operator-mint contract** (always supply
  `invitedContact`) — the SECURITY-DEFINER fn + no-inline-UPDATE are unchanged; the email-match check already
  exists. ✅
- **Money-integer / RLS FORCE:** no schema change this round; subtotal is integer display. ✅
- **i18n al/en:** new strings (subtotal label, "no_courier / awaiting courier", "Revert to READY", claim
  recipient-mismatch message) ship via `scripts/i18n-add.ts` + parity gate. ✅
- **Consent-before-publish:** PROTECTED FRICTION now lands as code-level G-PF1/G-PF2, not prose. ✅

## F2. Convergence — round 2

**CONVERGED — design-level, re-grounded.** All three new HIGHs (R2-1/R2-2/R2-3) are re-grounded on the REAL
mechanisms and fixed with guardrails asserting **real** return shapes; the four MEDs are fixed (R2-5
re-decided at the data layer); the round-1 fictional primitive and the false "no recipient check" are
retracted and corrected. The fixes hold under the **shipped flag state** (handshake dark, `OWNER_TWO_TAP`
independent).

**Remaining NEEDS-HUMAN (1):** §3 field-floor — architect recommends **contextually-required
(pin-confidence-gated)**; product/ops ratify the rule + threshold (server-tolerant, no contract change).
Build does not advance on that field until ratified.

**DEFER-FLAGs:** F4 observed kitchen-duration learner (unchanged); `courier-dispatch.ts:76 this.boss` latent
bug (out of the §5 path).

---

# RESOLVE round 3 — re-attacking the round-2 fixes against the binding lifecycle + the recipient identity

**Seat:** Architect · **Date:** 2026-06-28 · design-only, no production code.
**Inputs:** breaker `RE-ATTACK round 3` (R3-1…R3-6), round-1/2 `proposal.md`/`resolution.md`.
**Re-grounded @ HEAD** (`feat/mvp-sensor-seams`) against files actually read this round (file:line below).
The breaker is correct on every count: round 2 (a) **overstated** the recipient binding as "proof of
control of the invited email" when registration has no email-ownership proof; (b) left the §5 lookup
blind to `'offered'`, which the mig-073 partial uniques now police → a hard 500; (c) never specified how
an "awaiting courier" order re-dispatches; (d) specified a blanket IN_DELIVERY→READY recovery that
**contradicts the shipped `/abort`** for a `picked_up` binding; (e) named `confirmed_at` as the ETA decay
source when the code selects `created_at`/`preparing_at`. Each is re-grounded and corrected below.

## A3. Re-grounding (the REAL contracts each round-3 disposition stands on)

| Claim | Live source @ HEAD (read this round) | Status |
|---|---|---|
| `invited_contact_hash` is **nullable**; mig comment is explicit: *"Token-only when invited_contact_hash IS NULL (ops minted without a contact)"*. The email-match is a `sha256(lower(trim(v_email)))` **string** comparison against the claimer's `users.email` — it proves the claimer **registered that email string**, NOT that they control the inbox. | `migrations/…071:27` (nullable col), `:64–68` (NULL-skip + string match) | **CONFIRMED — round-2 "proof of control of the invited email" is OVERSTATED** |
| Owner sign-in is argon2-only against `users.password_hash`; **no email-ownership verification** anywhere on the auth path (OTP disabled per memory). A user can register with the restaurant's scraped public email and set a password. | `routes/auth/local.ts:88–108` | **CONFIRMED — the email-match is defeatable by registering the scraped contact** |
| The §5 courier-availability lookup excludes couriers in `('assigned','accepted','picked_up')` — **NOT `'offered'`**. | `orders.ts:792–794` | **CONFIRMED** |
| mig-073 makes BOTH partial uniques include `'offered'`: `courier_assignments_order_active_uniq (order_id) WHERE status IN ('offered','assigned','accepted','picked_up')` and `courier_one_active_assignment (courier_id) WHERE status IN (same four)`. | `migrations/…073:22–24, 32–33` | **CONFIRMED — an INSERT against an order/courier already carrying an `'offered'` row violates the unique → 23505 → uncaught 500** |
| The §5 `orders.ts:785` path **never enqueues** into `courier_dispatch_queue`; only `releaseBindingAndReoffer` (abort/cancel re-offer) does. The async `CourierDispatchWorker` consumes that queue. | `bindingRelease.ts:28–34` (the only enqueue on this surface), `courier-dispatch.ts:17–19, 32–33` | **CONFIRMED — the async worker is NOT on the §5 awaiting-courier path** |
| `courier-dispatch.ts:76` calls `this.boss.send(...)`; the constructor stores `this.queue` (no `this.boss`). The re-enqueue runs **after** `COMMIT` (`:75`), so on no-courier the retry throws a `TypeError` → the order is never re-queued. | `courier-dispatch.ts:10–14, 75–77` | **CONFIRMED — real latent bug; off the §5 path** |
| The central fold blanket-cancels the active binding on `IN_DELIVERY→{READY,CANCELLED}` **regardless of `asg_status`**, including `'picked_up'`. | `orderStatusService.ts:129–140` | **CONFIRMED — a raw revert-to-READY on a `picked_up` binding sets the order READY with the food already out** |
| The shipped `/abort` routes through `releaseBindingAndReoffer`, which **branches on `asg_status`**: `picked_up` + IN_DELIVERY → `updateOrderStatus(CANCELLED)` (honest terminal, no re-offer); IN_DELIVERY pre-pickup → `READY` + re-enqueue; flag-ON accept → no transition + re-offer. | `routes/courier/assignments.ts:472–520`, `bindingRelease.ts:37–52` | **CONFIRMED — the honest asymmetric primitive already exists; the owner recovery must REUSE it, not invent a raw revert** |
| `synthesizeAndPersistEtaWindow` SELECTs `o.created_at`, `o.preparing_at` — **never `o.confirmed_at`** — and passes `createdAt`/`preparingAt` into `gatherOrderEtaRange`, whose decay only fires for `status='PREPARING'` off `preparingAt`. | `etaGather.ts:192, 216–217, 81–86` | **CONFIRMED — round-2's "decay off `confirmed_at`" is UNPLUMBED (field neither selected nor passed)** |

## B3. Round-3 dispositions

### R3-1 (HIGH, B-SEC) — the web-claim recipient binding is weak → **(a) FIXED on the web surface · (b) ACCEPT-RISK + DEFER-FLAG to P6/auth · round-2 OVERSTATEMENT corrected**

**Correction first (the council values this over a clean table).** Round-2 wrote that the
`invited_contact_hash` check converts "token = sole authority" into *"token + **proof of control of the
invited email**."* That is **overstated.** Grounded at HEAD: `claim_transfer` does a **string** compare
`sha256(lower(trim(users.email))) == invited_contact_hash` (`…071:65–68`); the auth path
(`local.ts:88–108`) performs **no email-ownership verification** (OTP disabled). So the check proves only
that the claimer **registered an account whose email column equals the invited contact** — and the
invited contact is the restaurant's **public, scraped** email. An attacker who registers with that email
passes the match. The binding is a **speed-bump** (raises the bar from "any authed account" to "an account
registered under the specific scraped contact string"), **not** cryptographic proof of identity. The honest split:

**(a) FIXED NOW — the web claim surface refuses a token-only (NULL-hash) invite.** This is the delta this
patch's web surface CAN close. Hard precondition: the net-new **preview** endpoint and the **accept** path
require the matched invite to have `invited_contact_hash IS NOT NULL`; a token-only invite returns the
**same generic "link no longer valid"** (no enumeration, K2). `claim_transfer` is **UNTOUCHED** (it still
technically permits NULL-hash for any non-web/ops path; the web surface simply never reaches it with a
token-only invite). This makes the *"binds any authed account"* theft **unreachable via the web path**.
- **G-F2g (new, red→green):** a web claim (preview **or** accept) against a token whose invite has
  `invited_contact_hash IS NULL` → refused with the generic error, and `organizations.owner_id` stays
  NULL. *Red until the precondition exists* (today there is no web surface; `acceptClaim` would call
  `claim_transfer`, which binds a NULL-hash token to any authed user).
- This **strengthens** the round-2 G-F2d (operator mint MUST supply `invitedContact`): G-F2d binds at mint
  time, G-F2g enforces at the consuming surface even if a NULL-hash invite somehow exists.

**(b) ACCEPT-RISK (this patch) + DEFER-FLAG (P6/auth owner) — the email-ownership gap is PRE-EXISTING.**
The deeper defeat (register the scraped email → pass the string match) is a property of **shipped P6 + the
auth path**, not introduced here. Disposition:
- **ACCEPT-RISK for this patch:** the vertical is **dark** (gated on `PROVISION_OPS_SECRET` + migs 068–071);
  the surface delta does **not worsen** the gap and **adds** the non-NULL-hash precondition (strict
  improvement). The patch must not *claim* to close email-verification.
- **DEFER-FLAG to the P6/auth owner:** the real fix is **email-ownership verification before claim** — an
  email/OTP confirmation that the claimer controls the invited inbox (or a magic-link claim that lands in
  that inbox), so the match proves control, not registration. Out of this patch's scope.
- **Residual, stated plainly:** F2's web surface ultimately rests on **three** legs — (1) the token not
  leaking (transport hardening §6/F2, this patch), (2) the operator minting with a real recipient contact
  (G-F2d, this patch), and (3) the invited email not being attacker-registerable (the **deferred** gap).
  Legs 1–2 are closed here; leg 3 is the P6/auth owner's. **Owner: P6/auth seat** (provisioning + auth).

### R3-2 (MED) — the §5 lookup is blind to `'offered'` → **FIXED (already-bound = no-op/clear signal; lookup excludes `'offered'`)**

Grounded break: with R2-1's reorder, "Send for delivery" on a CONFIRMED order that **already has an
`'offered'` binding** (handshake-on manual offer, courier hasn't accepted) runs the lookup, finds a
courier, and INSERTs a second active `('assigned')` row for that `order_id` → violates
`courier_assignments_order_active_uniq` (`…073:22–24`) → 23505 → the handler's catch has no `statusCode`
→ **uncaught 500** (`orders.ts:825–832`; the tx rolls back so the order stays CONFIRMED, but the owner
gets a 500, not the honest "awaiting"/"offer pending"). The same 23505 fires via
`courier_one_active_assignment` (`…073:32–33`) if the lookup picks a courier already carrying an
`'offered'` row for **another** order (the lookup doesn't exclude `'offered'` couriers).

**Decision — make the §5 dispatch binding-aware, two parts:**
1. **Already-bound guard (the primary fix):** before any status flip or INSERT, the handler checks whether
   the order already has an **active** binding (`status IN ('offered','assigned','accepted','picked_up')`).
   If yes → **no-op**, do **not** call `updateOrderStatus` or INSERT; return a clear server-authoritative
   signal: `{ status:<current>, dispatched:true, reason:'already_assigned' }` for assigned/accepted/picked_up,
   and `{ status:'CONFIRMED', dispatched:false, reason:'offer_pending' }` for `'offered'` (the order stays
   CONFIRMED until the courier accepts — consistent with the handshake semantics; "Send for delivery" must
   **not** force an `'offered'` order to IN_DELIVERY). The card renders "offer pending" / "courier assigned",
   never a 500.
2. **Exclude `'offered'` from the courier-availability lookup** so it matches `courier_one_active_assignment`:
   `c.id NOT IN (… status IN ('offered','assigned','accepted','picked_up') …)` (`orders.ts:792–794`). A
   courier mid-offer is not "available." *(The same 3-state→4-state correction applies to the async worker
   `courier-dispatch.ts:55–58` — folded into the R3-3 DEFER-FLAG, not the §5 path.)*

**Guardrail G-F1a extended (red→green):**
- **G-F1a-2:** PATCH `/orders/:id/status` `{status:'IN_DELIVERY'}` on an order that **already has an
  `'offered'` (or any active) binding** → **no 500, no second INSERT**; response is the clear
  `offer_pending` / `already_assigned` signal and the DB row is unchanged. *Red today* (the INSERT throws
  23505 → 500).

### R3-3 (MED) — "awaiting courier" must not be an undispatchable dead end → **FIXED (owner re-tap is the specified re-dispatch path) + DEFER-FLAG (worker bug + auto-pickup)**

Grounded: the §5 honest endpoint (`orders.ts:785`) is **pull-based** — it dispatches only on the owner tap
and **never enqueues** `courier_dispatch_queue`. The async `CourierDispatchWorker` is **not** wired to this
path (only `releaseBindingAndReoffer` enqueues, `bindingRelease.ts:31`). So an "awaiting courier" order does
**not** auto-dispatch when a courier later comes online via the §5 path.

**Decision — the specified, real re-dispatch path is the owner re-tapping "Send for delivery".** The
CONFIRMED+awaiting card's affordance **is** the re-dispatch button: once a courier is online, the owner
re-invokes the same honest `orders.ts:785` action → courier found → IN_DELIVERY + assign. This is a real,
working, non-dead-end loop with **no new plumbing** (the single-courier MVP shop owner sees the awaiting
state and re-taps). The "awaiting courier" state is a **visible pending state on a legal status
(CONFIRMED)**, recoverable by the same action that created it.
- **G-F1c (new, red→green):** an order left CONFIRMED+awaiting after a no-courier dispatch, re-dispatched
  via the §5 action **once a courier is available**, reaches `IN_DELIVERY` with an assignment. *Proves the
  awaiting→delivering loop closes* (no dead end).

**DEFER-FLAG (not on the §5 path):** the async auto-pickup (a courier coming online auto-claiming awaiting
orders) is a **future enhancement** and is currently **broken** — `courier-dispatch.ts:76` calls
`this.boss.send` but the constructor stores `this.queue` (`:10–14`), so the no-courier re-enqueue throws a
`TypeError` **after** `COMMIT` (`:75`) and the retry never schedules; the worker's courier-availability
lookup also carries the same 3-state (`'offered'`-blind) exclusion as R3-2 (`:55–58`). Wiring the §5
no-courier path to enqueue this worker is **explicitly NOT done now** (it would inherit the bug). Owner:
implementer (separate fix + regression for both the `this.boss` ref and the `'offered'` exclusion).

### R3-4 (MED) — revert-to-READY must not strand food-already-out → **FIXED (recovery REUSES `/abort`'s asymmetric `releaseBindingAndReoffer`; round-2 raw-revert corrected)**

**Correction:** round-2's R2-4 specified the IN_DELIVERY recovery as `onUpdateStatus('READY')` →
`updateOrderStatus(IN_DELIVERY→READY)` **with no `asg_status` guard.** Grounded, that central fold
(`orderStatusService.ts:129–140`) blanket-cancels the active binding regardless of `asg_status` — so on a
`'picked_up'` binding it cancels the binding **and sets the order READY while the food is out with the
courier.** That regresses the customer's status and **contradicts the shipped `/abort` + deliver-v2
honesty** (`bindingRelease.ts:37–40` sends `picked_up`→CANCELLED, the no-cash terminal). Round-2's
"single-step revert-to-READY" is **unsafe for `picked_up`** and is corrected.

**Decision — the owner IN_DELIVERY recovery routes through the SHIPPED asymmetric primitive
`releaseBindingAndReoffer` (`lib/bindingRelease.ts`), NOT a raw `updateOrderStatus('READY')`.** An
owner-scoped recovery endpoint loads the order's active binding `FOR UPDATE` and calls
`releaseBindingAndReoffer(asgStatus, ordStatus, …)`, which already encodes the only correct branch:
- `asg_status ∈ {assigned, accepted}` (food still at the venue) + IN_DELIVERY → **READY** + binding
  cancelled + re-enqueue → re-dispatchable. (For an `'assigned'`-but-not-accepted binding the recovery is
  equally valid; the SELECT admits `('offered','assigned','accepted','picked_up')`.)
- `asg_status = picked_up` (food is out) → **CANCELLED** (the honest terminal), **never READY**.

The owner recovery **does not invent a new path**: it reuses the exact rail `/abort` and `/cancel` use, so
the central fold remains the single terminalization point and the deliver-v2 invariant ("no order leaves
IN_DELIVERY without its binding terminalized in the same tx, asymmetrically by `asg_status`") holds for
the owner surface too. *(Product may alternatively choose to **disable** recovery on `picked_up` and show
"courier has the food — contact courier" — also acceptable; the architectural floor is simply **never
READY-with-food-out**.)*

**Guardrail G-F1b corrected (red→green), split:**
- **G-F1b-i:** an IN_DELIVERY order with an `accepted` (pre-pickup) binding → owner recovery → order
  `READY` + binding `cancelled` + re-offerable. *Red today* (no IN_DELIVERY recovery branch on the card).
- **G-F1b-ii (the R3-4 guard):** an IN_DELIVERY order with a `picked_up` binding → owner recovery → order
  `CANCELLED` (NOT READY); binding terminalized. *Red today* (a raw `updateOrderStatus('READY')` would set
  READY + cancel the picked_up binding — the food-out lie).

### R3-5 (LOW) — the ETA decay source is unplumbed → **FIXED (decay base = `COALESCE(preparing_at, confirmed_at, created_at)`; round-2 `confirmed_at`-only corrected)**

**Correction:** round-2's revised G-F3 said the ETA "decays off `confirmed_at`." Grounded,
`synthesizeAndPersistEtaWindow` selects `o.created_at`, `o.preparing_at` (`etaGather.ts:192`) and passes
`createdAt`/`preparingAt` (`:216–217`) — `confirmed_at` is **neither selected nor passed**, and
`gatherOrderEtaRange` only decays in the `status='PREPARING'` branch (`:81–86`). So "decay off
`confirmed_at`" is **unplumbed**, and the breaker's hazard is real: a directly-CONFIRMED order may have
**NULL `confirmed_at`** (it is stamped only on the CONFIRMED *transition* via `STATUS_AT_COLUMN`,
`orderStatusService.ts:11–18` — an order inserted/seeded at CONFIRMED bypasses it).

**Decision — specify the real, non-NULL-safe source.** The decay base is
`COALESCE(preparing_at, confirmed_at, created_at)`:
- add `o.confirmed_at` to the SELECT (`etaGather.ts:192`) and a `confirmedAt` field to `GatherArgs`
  (small, additive — round-2 elided this plumbing);
- in `gatherOrderEtaRange`, add a **non-terminal, post-PENDING** decay branch:
  `prepRemaining = max(0, orderPrep − minutesSince(preparingAt ?? confirmedAt ?? createdAt))` while
  `ready_at` is NULL and status is not in the food-out set; `created_at` is the ultimate fallback and is
  **always non-NULL** (every order has it), so the NULL-`confirmed_at` hazard is closed by construction.
- `confirmed_at` is the honest base when present (owner-accept = kitchen-start in the 2-tap model);
  `created_at` (order placed) is the safe, always-present floor; `preparing_at` wins when an explicit
  kitchen-flow PREPARING transition stamped it.

**G-F3 corrected:** a CONFIRMED order with `confirmed_at = now()−2min`, `preparing_at NULL`, `ready_at NULL`,
`prep=15` → `prepRemaining ≈ 13` (NOT 0, NOT a flat 15); and a CONFIRMED order with **`confirmed_at NULL`**
and `created_at = now()−2min` → `prepRemaining ≈ 13` (proves the `created_at` fallback). *Red today*
(`etaGather.ts:81–86` returns the flat `orderPrep` for CONFIRMED; `confirmed_at` not in the SELECT).

### R3-6 (LOW) — the decline destructive-token is protected only ON the page, not across its 72h life → **NOTED (residual) + DEFER-FLAG (P6 owner: TTL / soft-delete grace)**

Grounded: the §6/F2 transport hardening (fragment + scrub, body-only) protects the token **while the user
is on `/claim`** — it does **not** address the token's **72h life in the delivery channel** (the
operator's email/SMS to the scraped contact). For 72h, anyone with access to that channel (forwarded
message, inbox compromise, shoulder-surf) can invoke the **destructive** decline → `declineAndErase` →
`hardDeleteShadow` (H-decline, irreversible). The transport fix does not shrink that window.

**Disposition — NOTED residual + DEFER-FLAG.** For **this** patch: ACCEPT — decline-erase is **shipped P6
behavior** (token-only, account-free by H-decline design; the token reaches only the verified contact) and
the transport delta does **not worsen** it. The residual blast-radius reduction is the **P6 owner's**:
recommend (defer) either a **shorter decline TTL** (decline-erase need not live the full 72h claim TTL) or
a **soft-delete grace window** so an erroneous/hostile decline within the window is recoverable rather than
an immediate `hardDeleteShadow`. Owner: P6/acquisition seat. *(No new guardrail in this patch; tracked as a
P6 hardening item.)*

## C3. Round-3 disposition table

| ID | Sev | Finding (one line) | Disposition | Real grounding @ HEAD |
|---|---|---|---|---|
| R3-1 | HIGH | Web-claim recipient binding weak | **(a) FIXED web refuses NULL-hash (G-F2g) · (b) ACCEPT-RISK + DEFER-FLAG email-verify to P6/auth** | `…071:27,64–68` (nullable + string match), `local.ts:88–108` (no email proof) |
| R3-2 | MED | §5 lookup blind to `'offered'` → 23505/500 | **FIXED — already-bound no-op/signal + exclude `'offered'`; G-F1a-2** | `orders.ts:792–794` vs `…073:22–24,32–33` |
| R3-3 | MED | "awaiting courier" re-dispatch unspecified | **FIXED — owner re-tap is the path (G-F1c) · DEFER-FLAG worker `this.boss` + auto-pickup** | `orders.ts:785` (no enqueue), `bindingRelease.ts:31`, `courier-dispatch.ts:10–14,75–77` |
| R3-4 | MED | Blanket revert-to-READY strands `picked_up` | **FIXED — recovery REUSES `releaseBindingAndReoffer` asymmetry; G-F1b-i/ii** | `orderStatusService.ts:129–140` (blanket) vs `bindingRelease.ts:37–48` + `assignments.ts:472–520` |
| R3-5 | LOW | ETA decay off `confirmed_at` unplumbed | **FIXED — base = `COALESCE(preparing_at,confirmed_at,created_at)`; G-F3 corrected** | `etaGather.ts:192,216–217,81–86`, `orderStatusService.ts:11–18` |
| R3-6 | LOW | Decline token protected on-page, not 72h | **NOTED residual + DEFER-FLAG (TTL/soft-delete) to P6** | `claim.ts:68–82` (no-auth destructive), 72h TTL |

## D3. Round-2 dispositions corrected this round (honesty ledger)

| Round-2 claim | Correction |
|---|---|
| G-F2d: recipient binding = *"token + **proof of control of the invited email**"* | **Overstated.** It is a **string match on a self-asserted, unverified email** (registration has no email-ownership proof; OTP off) → defeatable by registering the scraped contact. It is a **speed-bump**, not proof of control. Web path now also **refuses NULL-hash** (G-F2g); the identity-proof gap is **DEFER-FLAG to P6/auth**. |
| R2-4 / G-F1b: IN_DELIVERY recovery = raw `onUpdateStatus('READY')` (single step, no guard) | **Unsafe for `picked_up`** (central fold cancels the binding + sets READY with food out — contradicts `/abort`). Corrected to **route through `releaseBindingAndReoffer`** (asymmetric: pre-pickup→READY, picked_up→CANCELLED). G-F1b split into i/ii. |
| G-F3 / R2-5: ETA "decays off `confirmed_at`" | **Unplumbed** (`confirmed_at` neither selected nor passed; directly-CONFIRMED may be NULL). Corrected to `COALESCE(preparing_at, confirmed_at, created_at)` with `confirmed_at` added to the SELECT + `created_at` as the always-non-NULL floor. |
| R2-1 §5 dispatch (no `'offered'` consideration) | **Incomplete** — blind to the `'offered'` binding the mig-073 uniques police → 500. Corrected with the already-bound guard + `'offered'` exclusion (R3-2). |

## E3. Red-lines re-verified intact (round 3)

- **No-trap:** R3-2 (already-bound → clear signal, never a 500-trap) + R3-3 (awaiting→re-tap→IN_DELIVERY,
  no dead end) + R3-4 (recovery is asymmetric, never READY-with-food-out) — every IN_DELIVERY path remains
  recoverable and honest. ✅
- **Deliver-v2 honesty / `picked_up`→CANCELLED:** R3-4 reuses the shipped `releaseBindingAndReoffer`; the
  owner recovery cannot set READY on a picked-up order. The central fold stays the single terminalization
  point. ✅
- **claim_transfer untouched:** R3-1 adds the NULL-hash refusal at the **web surface** (module/route layer);
  the SECURITY-DEFINER fn + no-inline-UPDATE are unchanged. ✅
- **Server-authoritative:** R3-2 returns the **real** status + an honest `reason`; R3-5 decays off real
  timestamps (`created_at`/`confirmed_at`), fabricates none. ✅
- **Money-integer / RLS FORCE:** no schema change this round (R3-5 adds a SELECT column + arg, no DB
  migration; `confirmed_at` already exists). ✅
- **i18n al/en:** new strings ("offer pending" / "already assigned", "link no longer valid" generic claim
  error) ship via `scripts/i18n-add.ts` + parity gate. ✅

## F3. Convergence — round 3

**CONVERGED — design-level, re-grounded, with three round-2 overstatements explicitly corrected** (the
recipient-binding strength, the raw revert-to-READY, the `confirmed_at` decay source). The one HIGH
(R3-1) is split honestly: the web-surface delta is **fixed** (refuse NULL-hash, G-F2g); the
email-ownership-verification gap is a **pre-existing P6/auth weakness** → **ACCEPT-RISK for this dark
patch + DEFER-FLAG to the P6/auth owner**, with the residual stated plainly (the surface rests on
no-leak + real-contact-mint + the unverified-email gap). The three MEDs are fixed against the real binding
lifecycle (already-bound guard, owner re-tap loop, asymmetric recovery); the two LOWs are fixed/noted.

**Remaining NEEDS-HUMAN (unchanged, 1):** §3 field-floor — architect recommends contextually-required
(pin-confidence-gated); product/ops ratify.

**DEFER-FLAGs (carried + new):**
- **(R3-1)** email-ownership verification before claim (OTP / inbox-bound magic-link) — **P6/auth owner**.
- **(R3-3)** async auto-pickup: fix `courier-dispatch.ts:76 this.boss`→`this.queue` + extend its lookup to
  exclude `'offered'` (`:55–58`) before wiring the §5 no-courier path to it — **implementer**.
- **(R3-6)** decline TTL / soft-delete grace window — **P6/acquisition owner**.
- **(carried)** F4 observed kitchen-duration learner (`ready_at`/`preparing_at` NOT NULL filter) —
  **implementer + product (kitchen-flow toggle)**.

**New guardrails this round (red→green before merge):** G-F2g (web refuses NULL-hash), G-F1a-2 (already-
bound dispatch is a no-op/signal, not a 500), G-F1c (awaiting→re-tap→IN_DELIVERY loop closes), G-F1b-i/ii
(asymmetric recovery: pre-pickup→READY, picked_up→CANCELLED), G-F3 corrected (COALESCE decay base).
