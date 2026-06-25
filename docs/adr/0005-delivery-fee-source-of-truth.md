# ADR-0005 — Delivery-fee source of truth at checkout

Status: Proposed (design-time) · Date: 2026-06-25 · Branch: fix/design-system-consistency
Supersedes the hardcoded `deliveryFee = 200` in `CheckoutPage.tsx:342`.

## Context

The storefront checkout hardcodes a flat `200` delivery fee (`apps/web/src/pages/client/CheckoutPage.tsx:342-343`)
and derives the `Porosit • {total}` CTA from it. The order-create payload sends **no** fee; the server
is authoritative (`apps/api/src/routes/orders.ts:519-566`):

- enforces `min_order_value` (422 MIN_ORDER_NOT_MET),
- waives the fee when `subtotal >= free_delivery_threshold`,
- applies distance tiers (`delivery_tiers`) else `delivery_fee_flat` (422 DELIVERY_NOT_CONFIGURED / NOT_DELIVERABLE),
- adds `applyTax(...)`, computes `total = subtotal + fee + tax - discount`, all **integer minor units**.

The public `/public/locations/:slug/info` (`apps/api/src/routes/public/menu.ts:213-282`) exposes
`currency_code`/`currency_minor_unit` but **not** the fee/threshold/tax config, so the client cannot
mirror the server. Worked divergence: at `free_delivery_threshold = 2000, delivery_fee_flat = 250`, a
2000 subtotal is charged 2000 (waived) but the client shows 2200 — a 200 over-quote exactly at the
incentive boundary; a 1500 subtotal is charged 1750 but shows 1700 — a 50 under-quote.

## Decision (hardened post-RESOLVE — Breaker C1/C2/H1/H3/M2, Counsel A1 + ETHICAL-STOP-1)

**Option A′ — the client estimate is a HINT ONLY; the server `total` is reviewed BEFORE any cash commit
and IS the collected sum.** The replicate-where-deterministic / estimate-where-not shape stays, but the
cash-door correction is load-bearing.

1. Extend `/info` (additive) with `min_order_value`, `free_delivery_threshold`, `delivery_fee_flat`,
   `tax_rate`, `price_includes_tax`, and a **precomputed public `has_distance_tiers` boolean** (read from
   the already-public `locations` row — NOT an RLS-subject `EXISTS`, which the public `/info` role would
   read as FALSE for every venue and silently take the lying exact path; M2). Fields ride the existing
   cached `/info` row — no new query budget.
2. **Money math — SHARED building blocks + a GATED client mirror (Approach M, NEW-3-H1; Counsel A1, C2).**
   Verified: `@deliveryos/domain` has no money module today; the server computes the total **inline** at
   `orders.ts:560-565` (fee ladder interleaved with `ROLLBACK`/`INSERT`); no `computeOrderTotal` exists.
   Making the client equal the server "by construction" would require refactoring the authoritative money
   create-path (a 🔴 money red-line) — which the §1.1 non-goals disclaim. **Decision: Approach M, NOT R.**
   - Extract the **pure building blocks** `applyTax`/`computeLineTotal`/`assertNonNegative` **verbatim**
     from `apps/api/src/lib/money.ts` into `@deliveryos/domain` (isomorphic; they have no impure
     interleaving — safe), and re-point `apps/api`'s imports there (a pure, characterization-pinned move).
   - The client uses a separate pure **`estimateOrderTotal`** that **mirrors** the server fee-selection
     arithmetic (computable case only). **The server create-path is NOT refactored to call it** — zero
     change to the charged amount; the red-line is held.
   - "Displayed == charged" is guaranteed by the **permanent parity guardrail** (computes the REAL server
     total, asserts `estimateOrderTotal == serverTotal`, red→green — see Acceptance) + the runtime cash-422
     backstop, NOT by a shared-by-both-sides import. The fn is named `estimateOrderTotal` so no reader
     infers the server calls it.
   - **Approach R** (refactor `orders.ts:518-565` into a pure fn both sides import, equality "by
     construction") is the recorded **deferred end-state**, taken as its own behaviour-preserving,
     characterization-gated change **when the server money math grows** (e.g. nonzero `discountTotal` /
     promo). Until then the gate IS the guarantee. (YAGNI: today the only divergent-risk input,
     `discountTotal`, is hardcoded `0` at `orders.ts:564`.)

   Pinned contract (H3): `applyTax` operates entirely in **stored minor-unit integers** — two BigInt
   branches (tax-excluded forward half-up; tax-included back-out), `rateMicro` quantised to 6-dp
   micro-units; the `minorUnit` argument is informational and MUST NOT trigger coarser rounding (the
   server's `_minorUnit` is dead, `lib/money.ts:23`). The CTA shows the exact `total` for flat-fee venues
   **as a pre-review hint**.
3. When the venue is **distance-tiered** (`has_distance_tiers` true OR unknown — fail-safe) or `/info` is
   unavailable, the client degrades to `Porosit • {subtotal}+` with an equal-weight reason line
   **"delivery fee depends on your address — confirmed at checkout"** (Counsel A2). The order still submits.
4. `min_order_value` is a FE soft-gate (disable Order + inline message), **delivery AND pickup** (matches
   the server, which checks above the pickup branch, `orders.ts:519`; H1), over a **modifier-inclusive
   subtotal** (the shared `computeLineTotal`, so FE subtotal == server subtotal). Backed by the server 422.
5. **Cash (C1 + NEW-C1, the critical fix — by COMPUTABILITY, not a new endpoint):** the displayed
   estimate is NEVER the cash collected. The authoritative total is obtained as follows, with NO
   `/orders/quote` endpoint built:
   - **Computable venue** (flat-fee + threshold + tax — the demo + dominant case): the FE computes
     `reviewTotal` via the client mirror `@deliveryos/domain` `estimateOrderTotal` over public `/info`
     inputs. `reviewTotal` **equals** the server total over the parity-gated matrix (Approach M — the server
     does NOT call this fn; equality is CI-gated, not "by construction"), so it is the authoritative cash
     figure; cash `min`/change-due/door figure are keyed to it. The server charge path is untouched.
   - **Tiered venue** (`has_distance_tiers` true/unknown — RLS hides tiers from the public role): the client
     cannot compute the fee, so it shows NO exact cash pre-commit figure (`{subtotal}+`); the order submits,
     the server computes `total`, and the **courier delivery screen shows it as "collect: X"** — the courier
     collects the server-confirmed amount. The customer is never asked to pre-commit an exact cash figure
     the design can't back. (A read-only `/orders/quote` endpoint calling the same shared module is the
     recorded DEFERRED option for when tiered venues become common — NOT built now; YAGNI.)
   Tip is shown as a separate explicit line (collected = `total + tip`; `total` excludes tip,
   `orders.ts:565`). The FE handles `CASH_AMOUNT_TOO_LOW` (orders.ts:570) with a designed re-prompt — this
   is also the NEW-H1 backstop for the ≤30s `/info`-cache vs live-create-read window (the 422 catches any
   under-quote; the re-prompt re-shows the new total; the design adds no transactional quote-lock).
6. The **server-returned `total`** (orders.ts:760-761) is the final authority and is the figure surfaced
   to the courier delivery screen ("collect: X", door-handover parity, Counsel §5/A3). The replicated
   client number can never become the charge.

Money invariant: **integer ALL, no float anywhere**; `clientTotal == serverTotal` for flat-fee venues is
**CI-gated** by the permanent parity guardrail (Approach M — client mirror, server charge path untouched),
not "by construction" (the server does not call the client fn).

## Acceptance criteria (ETHICAL-STOP-1 — recorded, not assumed)

- **AC-PARITY-GATE (the load-bearing ship-blocker for Approach M):** a **permanent, never-deletable**
  parity guardrail proven **red→green** that, over {subtotal ∈ [0, min−1, min, threshold−1, threshold,
  threshold+1]} × {fee flat values} × {tax 0 / included (back-out) / excluded} × {`minor_unit` 0 and
  non-zero}, computes the **REAL server total** (via a characterization fixture exercising the actual
  `orders.ts` fee arithmetic, or by driving `POST /orders` and reading back `total`) and asserts
  `estimateOrderTotal(sameInputs) === serverTotal`. Red = mutate the client mirror by 1 minor unit → fail.
  A `docs/regressions/REGRESSION-LEDGER.md` row records it. This gate is what makes the hand-maintained
  mirror safe (drift = CI build break, never a silent prod mis-quote).
- **AC-CASH-PARITY:** before a cash order is placed, the UI shows the **server-equal `total`** (the
  gated client mirror, not a loose estimate) in a review step the customer passes through; the cash figures
  are keyed to it; the figure last reviewed == the courier's "collect: {total}" door figure (+ separate tip
  line).
- **AC-CASH-422:** the FE handles `CASH_AMOUNT_TOO_LOW` with a designed re-prompt showing the updated
  server total — never the generic failure.
- **E2E (staging):** a cash checkout where fee config ≠ the old hardcoded path asserts the review shows the
  **server**-equal total, the order persists with `total` == the reviewed figure, and the courier screen
  renders the same "collect: {total}"; plus a forced `CASH_AMOUNT_TOO_LOW` re-prompt assertion. The
  AC-PARITY-GATE test + this door-handover E2E + the cash-422 E2E are the mandatory ship-blockers.

## Consequences

- Trust win on the common (flat-fee) case: the CTA hint matches the charge and the free-over-threshold
  incentive becomes live; the **collected** sum is always the server-reviewed total (cash-safe).
- No false precision on tiered venues; no order is ever blocked by a client guess.
- A **permanent parity guardrail** (also the fallback if the share is ever broken): `clientTotal ===
  serverTotal` across {subtotal at min−1/min/threshold−1/threshold/threshold+1} × {flat/tiered} ×
  {tax 0 / **included (back-out)** / excluded} × {`minor_unit` 0 and non-zero}. Never deletable.
- `CHECKOUT_FEE_REPLICATION` (default on) now toggles only the cosmetic pre-review hint; neither state can
  mis-collect after C1.

## Resolved / open

- **(RESOLVED, M2):** `has_distance_tiers` is a precomputed public boolean with a client fail-safe
  (unknown → degrade); no RLS-subject read. Owner: Data agent.
- **(RESOLVED, C2/A1 + NEW-3-H1 — Approach M):** the pure building blocks
  (`applyTax`/`computeLineTotal`/`assertNonNegative`) are shared in `@deliveryos/domain`; the client total
  (`estimateOrderTotal`) is a **gated mirror** of the server fee arithmetic — the server create-path
  (`orders.ts:518-565`) is NOT refactored to call it (the 🔴 money red-line is held; zero change to the
  charged amount). The parity guardrail (real server total == client mirror, red→green, permanent) is the
  guarantee. **Deferred:** Approach R (extract the create-path arithmetic so both sides share one fn,
  equality by construction) — taken as its own behaviour-preserving change when server money math grows
  (nonzero `discountTotal`/promo). **Owned residual:** mirror drift, caught by the CI gate (never silent).
  Owner: Product/eng + API agent.
- **(accepted, M1 / NEW-H1):** ≤30s `/info` cache staleness means a computable-venue `reviewTotal` and the
  live create-txn `total` are two reads. Accepted: the server 422 (`cashPayWith < total`) + AC-CASH-422
  re-prompt make every under-quote safe-by-direction; no transactional quote-lock is built (it would need
  the deferred `/orders/quote` endpoint). A `checkout_fee_divergence` counter surfaces the window in <1 min.
  Owner: Product.
- **(accepted, NEW-H2):** the FE recomputes `subtotal` modifier-inclusive via the shared `computeLineTotal`
  over the freshly-loaded menu `price_delta`s (the public menu carries them, `MenuPage.tsx:458-466`). A
  modifier `price_delta` drifting within the ≤30s menu-read window is caught by the same 422 backstop.
  Owner: Product.
