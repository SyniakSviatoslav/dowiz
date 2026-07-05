# Design proposal — MONEY audit fixes: LC1 tax double-charge · LC6 crypto refund black hole · Settlement money loss

- **Status:** **REVISED v2** — post-council RESOLVE round (see `resolution.md` for per-finding dispositions). Design-time only; NO production code in this change. Conductor re-runs the breaker on this revision.
- **Inputs:** `docs/design-review/audit-money-orders-2026-07-03.md` (C1, C3, H5), `docs/design-review/AUDIT-SYNTHESIS-2026-07-03.md` (LC1, LC6, A4, B9), `breaker-findings.md`, `counsel-opinion.md`.
- **Red-lines touched:** money (all three), state-machine (LC6 fold), `packages/db/migrations/` (M-1, M-2 below).
- **ADR draft:** `docs/adr/ADR-audit-fix-money.md`.

---

## 0. Source verification (every finding re-checked at HEAD of `feat/phase0-safety-hardening`)

| Claim | Where verified | Verdict |
|---|---|---|
| Inclusive tax extracted then re-added | `apps/api/src/routes/orders.ts:509-511` — `taxTotal = applyTax(subtotal, tax_rate, price_includes_tax, …)` then `total = subtotal + deliveryFee + taxTotal - discountTotal` | **VERIFIED** |
| `applyTax` extraction math itself | `apps/api/src/lib/money.ts:14-18` — inclusive branch returns `sub - net` (correct extraction, BigInt half-up) | **VERIFIED CORRECT** — the bug is ONLY the composition at the callsite |
| FE mirror repeats the double-add | `packages/ui/src/lib/money.ts:79-86` — `total = subtotal + deliveryFee + taxTotal` | **VERIFIED** |
| `price_includes_tax` defaults TRUE | `packages/db/migrations/1780338982014_location_commerce.ts:9` — `boolean NOT NULL DEFAULT true` | **VERIFIED**; note `tax_rate` defaults **0** (line 8) — the bug is ARMED by default but FIRES only once an owner sets a rate |
| Parity test pins mirror==mirror | `apps/api/tests/fee-parity.test.ts` — `assert.equal(mirrorApplyTax(…), serverApplyTax(…))`; total oracle is `sub + fee + serverApplyTax(…)` — the expected value is COMPUTED FROM the implementation under test | **VERIFIED** — the guardrail certifies the bug |
| Isolated tax test is honest | `apps/api/tests/money-tax.test.ts` — literal constants (`applyTax(1200,0.2,true,0)===200`) | VERIFIED — keep as-is; it proves extraction, not composition |
| `refund_due` has exactly one writer | `apps/api/src/lib/deliveryCompletion.ts:129-145` is the only INSERT of `type='refund_due'` (grep: provider.ts/plisio.ts mention it in comments only; `owner/refunds.ts` writes `refund_sent`) | **VERIFIED** |
| Webhook flips paid with no order-status check | `apps/api/src/routes/payments-webhook.ts:58-70` — `UPDATE orders SET payment_status='paid' WHERE … payment_status IN ('pending','authorized')` | **VERIFIED** (audit said :64-69; actual :65-70) |
| Terminal paths that cancel with no refund hook | timeout sweep `packages/db/migrations/1790000000078_phase2-sweep-fns.ts:13-22` via `apps/api/src/workers/order-timeout-sweep.ts:72`; owner PATCH `apps/api/src/routes/orders.ts:891`; mark-no-show `apps/api/src/routes/owner/signals.ts:237`; grace-cancel `apps/api/src/workers/courier-offer-sweep.ts:241`; courier abort `apps/api/src/lib/bindingRelease.ts:40-43`; `updateOrderStatus` itself (`apps/api/src/lib/orderStatusService.ts`) has zero payments awareness | **VERIFIED** — 5 sanctioned paths + the raw customer-cancel (LC3, currently 500s) all drop the obligation |
| Refunds queue reads only `refund_due` | `apps/api/src/routes/owner/refunds.ts:25-30` | **VERIFIED** |
| Settlement window + SKIP LOCKED loses rows | `1790000000078_phase2-sweep-fns.ts:160-197` — pair AND item scans bounded to `[p_period_start, p_period_end)`; items `FOR UPDATE OF ca SKIP LOCKED`; cron (`apps/api/src/workers/settlement-cron.ts:29-49`) generates each period exactly once | **VERIFIED** — a skipped/missed row is outside every future window |
| Paid payouts mutate | fn `ON CONFLICT … DO UPDATE SET status = courier_payouts.status` then UNCONDITIONAL `deliveries_count/total_earned` bump (:188-190); `/pay` route sets `status='paid'` (`owner/settlements.ts` pay handler) with no immutability guard against later generation | **VERIFIED** |
| Bonus defect found during verification | fn counts `v_added_items`/`v_added_total` even when `ON CONFLICT (assignment_id) DO NOTHING` inserts nothing → concurrent runs inflate payout totals relative to items | NEW (subsumed by Fix 3's aggregate-recompute) |

---

## 1. Back-of-envelope money impact

**LC1 (live).** Overcharge fraction on an inclusive-priced order = `r/(1+r)` of the subtotal.
At Albanian VAT r=0.20 → **16.7 % of every cart**. Example: 1,200 ALL cart → customer pays 1,400 + fee.
A venue at 30 orders/day, avg subtotal 1,500 ALL → 30 × 250 = **7,500 ALL/day ≈ 225,000 ALL/month (~€2,250) per venue**, silently.
Blast radius TODAY: venues with `tax_rate > 0` (default 0) — i.e. one Settings action arms it; `price_includes_tax` is already TRUE by default. The FE mirror shows the same wrong number, so the customer, the owner, and the parity test all agree on the wrong total — **no one can notice**.

**LC6 (dark — flags off; gate before flag-flip).** Loss per event = **100 % of order principal** (crypto is irreversible; obligation never recorded → owner queue never shows it). Expected leak at flip = crypto GMV × non-fulfillment rate. At even a 5 % cancel/timeout/reject rate, **5 % of all crypto revenue becomes permanently kept customer money** — a trust/legal bomb, not just a bug.

**Settlement (live).** Each lost item = one delivery's cash (avg ~1,500 ALL) permanently absent from courier↔venue reconciliation. A crashed/locked 2 AM run loses **an entire day per courier-location pair**. The paid-payout bump is the inverse: money owed that no surface will ever display (courier was paid the pre-bump total).

---

## 2. Fix 1 — LC1 inclusive-tax double-charge

### 2.1 The correct math (invariant)

For `price_includes_tax = true`, the tax is INSIDE the subtotal. `applyTax`'s extraction stays informational (receipt line "includes VAT: X", persisted to `orders.tax_total` unchanged):

```
taxTotal      = applyTax(subtotal, rate, includesTax, minorUnit)   // unchanged — extraction OR addition amount
chargedTax    = includesTax ? 0 : taxTotal                          // NEW — inclusive tax is never additive
total         = subtotal + deliveryFee + chargedTax - discountTotal
```

Invariant to pin forever: **`price_includes_tax=true ⇒ total === subtotal + deliveryFee - discountTotal` exactly, for every rate.**

### 2.2 Option A — in-place fix (server + mirror, two edits)

Change the composition at both callsites: `apps/api/src/routes/orders.ts:511` and `packages/ui/src/lib/money.ts:84` (plus the mirror's `taxTotal` stays reported for display).

- **Pros:** minimal diff (2 files + tests); zero build-graph churn; fastest path to stopping a live overcharge; trivially reviewable under the money red-line.
- **Cons:** the composition rule now lives in TWO places — exactly the duplication that let the mirror-lock certify this bug. Any future consumer (reorder pricing, owner `/verify`, promotions) can re-fork the rule.

### 2.3 Option B — shared order-pricing module (single composition authority)

Extract `applyTax` + a new pure `composeOrderTotal({subtotal, deliveryFee, taxRate, priceIncludesTax, minorUnit, discountTotal}) → {taxTotal, total}` into ONE shared, dependency-free package importable by both planes — `packages/shared-types/src/money.ts` is the natural home (pure TS, already in both dependency graphs; a stale `dist/money.d.ts` shows it once lived there). `apps/api/src/lib/money.ts` and `packages/ui/src/lib/money.ts` become re-exports (or die); `estimateOrderTotal` keeps only the FE-specific fee-knowability logic and delegates composition.

- **Pros:** kills the mirror-drift CLASS — there is no mirror left to drift; parity test becomes structurally unnecessary for composition; one place to add discounts/fees later; ADR-0005's "server is SoT of what is charged" preserved (server still executes the shared fn).
- **Cons:** touches the workspace build graph (pnpm workspace dep edges, tsconfig refs); a subtly wrong extraction would now be wrong EVERYWHERE at once with no cross-check — mitigated only by the independent-constant tests (§2.5); larger review surface during an active-overcharge window.

### 2.4 Recommendation — A then B, two commits, same council approval

1. **Commit 1 (hotfix):** Option A + the independent-expectation tests (§2.5) proven red→green. Stops the live overcharge in the smallest reviewable diff.
2. **Commit 2 (class-kill):** Option B consolidation, behavior-frozen by the tests from commit 1 (they don't reference either implementation, so they survive the move untouched — that's the point).

Do NOT ship B alone as the fix: coupling a money-math correction to a build-graph refactor maximizes both review load and rollback blast radius.

### 2.5 Fee-parity test rewrite — independent oracle design

The current test's disease: `expected = sub + fee + serverApplyTax(…)` — the oracle IS the implementation. Redesign into three layers:

1. **Independent-constant correctness tests** (new file, e.g. `apps/api/tests/order-total-composition.test.ts`): table-driven vectors whose expected values are **hand-derived literal constants with the derivation in a comment**, computed WITHOUT calling `applyTax`/`estimateOrderTotal`/any module under test. Examples (derivations shown; final vectors to be authored at implementation with independent tooling — bc/spreadsheet):
   - inclusive: `subtotal=1200, fee=150, r=0.2` → `total === 1350`, `taxTotal(display) === 200` (1200×0.2/1.2).
   - inclusive rounding: `subtotal=1075, fee=0, r=0.075` → `total === 1075`, `taxTotal === 75`.
   - exclusive: `subtotal=1000, fee=250, r=0.2` → `total === 1450`, `taxTotal === 200`.
   - exclusive half-up boundary: `subtotal=1000, r=0.0745` → `taxTotal === 75`, `total = 1075 + fee`.
2. **Property/invariant test:** for ALL rates in the matrix, `priceIncludesTax=true ⇒ total === subtotal + fee` (this needs no oracle at all — it is the definition of inclusive pricing) and `priceIncludesTax=false ⇒ total - subtotal - fee === taxTotal`.
3. **Parity test demotion is DEFERRED to the Option-B commit (breaker M5 fix).** Until `composeOrderTotal` exists and the property test runs against the shared fn, the 432-combo parity matrix keeps running **unchanged** — commit 1 only ADDS coverage, never removes it. Additionally commit 1 adds a **route-level server composition integration matrix (P3b)**: ≥4 request-level vectors (inclusive/exclusive × zero/boundary rates) against POST `/orders`, because the server composition is inline in the route handler and otherwise has no matrix coverage pre-Option-B. At the Option-B commit the parity test's header is rewritten to state it proves FE==BE agreement, NOT correctness (correctness lives in (1)/(2)), and it shrinks to fee-ladder-only parity.

**Anti-recertification ratchet (Tier-1 candidate for the librarian) — strengthened per breaker M4:** call-shape detection (`expected` built by calling the module under test) is alias/hoist/launder-evadable, so the rule is redesigned around a mechanically checkable structure instead:
- expected values for money-composition tests MUST live in **data-only vector files** (`*.vectors.ts`/`.json`) containing **only literal initializers and zero import statements** — a `tools/eslint-plugin-local` rule verifies both properties reliably (no AST call-analysis to evade);
- composition test files may import **only** the module under test + the vector file (import-allowlist rule); any other import — including aliases, re-exports, or `estimateOrderTotal` laundering — is rejected at the import site, which aliasing/hoisting cannot dodge;
- each vector carries a mandatory derivation comment (bc/spreadsheet arithmetic shown).
**ACCEPT-RISK (residual, documented):** generating the vector file offline by running the implementation and pasting its outputs is not mechanically detectable by any static gate — this residual is inherent to oracle independence. Owner: reviewer discipline (derivation comments are the review hook) + the ledger row. Stated in the test-file header so the limitation is never mistaken for coverage. Ledger row in `docs/regressions/REGRESSION-LEDGER.md` on landing.

### 2.6 Data / restitution (operator decision — NOT auto-remediated)

Historical rows are the record of what WAS charged — never retro-mutate money rows. Affected-order enumeration for the operator (read-only):

```sql
SELECT o.id, o.location_id, o.created_at, o.tax_total AS overcharge_minor
FROM orders o JOIN locations l ON l.id = o.location_id
WHERE l.price_includes_tax AND l.tax_rate > 0
  AND o.tax_total > 0 AND o.created_at < :fix_deployed_at;
```

(`discountTotal` is hardcoded 0 today, so pre-fix `overcharge == tax_total` exactly on inclusive venues.)

**Restitution — ESC-1, encoded as a closure requirement (counsel STOP, adopted).** The forward hotfix ships immediately regardless; but LC1 remediation is **not "closed"** until:

1. **VAT trace (precondition — run BEFORE the restitution decision):** establish whether `orders.tax_total` / `orders.total` feed any VAT filing, owner report, accounting handoff, or export surface (owner analytics export included). Albania is an inclusive-VAT market: if any venue's VAT reporting keyed off the inflated figures, venues may have **over-remitted VAT to the state**, which turns LC1 from a discretionary goodwill question into a **compliance obligation with a third party in the loop**. The decision record must state which case holds, with the trace as evidence.
2. **Operator DECISION RECORD exists** — `{decision: refund | partial | notify-only | documented-no-action-with-cause, owner, date, rationale}`, filed in `docs/decisions/`. Silence must not default to keep-the-money; "no action" is a permitted outcome only when written down and signed.
3. The decision packet carries: where the money now sits (venue revenue vs over-remitted VAT vs platform); the **refundable (crypto/card) vs practically-unreachable (cash)** split — cash customers paid a courier and may only be repairable via future credit or disclosure; and the three-actor note (fault = platform's pricing engine, gain = venue's pocket under the 0%-commission model, harm = customer).

The decision itself is **needs-human-decision** — the operator decides; this design only removes the option of deciding by silence. Historical money rows are never retro-mutated in any outcome.

**Migration: NONE. Contract change: NONE** (response shapes unchanged; `tax_total` keeps its informational meaning).

### 2.7 FE presentation of inclusive tax (breaker M7 fix — in scope, same change)

Post-fix an inclusive receipt must never render as `subtotal 1200 + tax 200 = total 1350` — an apparent arithmetic error that recreates the "everyone stares at a wrong-looking number" failure mode. Spec:

- `estimateOrderTotal` gains an **additive** `chargedTax` field (`0` when inclusive) alongside the existing informational `taxTotal`; the FE renders the tax line from `taxTotal` but ONLY as an addend when `chargedTax > 0`.
- Inclusive venues render the line as informational: **"includes VAT (r%) — X"** (never a `+` row); exclusive venues keep the additive `Tax` row. i18n keys (al/en) for the "includes VAT" label added to `i18n-catalog.ts` via the parity-gated helper.
- Proof **P7b**: E2E on an inclusive staging fixture — receipt shows the "includes" label, no additive tax row, and displayed `total === subtotal + deliveryFee`.

---

## 3. Fix 2 — LC6 crypto refund black hole

### 3.1 Principle

**Invariant (ADR-0017 C2, restated — two-tier after breaker C1):** *whenever an order is in — or enters — a terminal non-fulfilled state (`CANCELLED`, `REJECTED`) while a `payments` row for it is (or becomes) `status='paid'`:*
- **Tier A (transactional):** on every per-order path (funnel, webhook) and whenever the structural trigger is healthy, a `refund_due` event exists **in the same transaction**;
- **Tier B (bounded-lag floor, every writer, every failure mode):** a `refund_due` event exists within **≤1 reconciler tick (~60s)**, OR a surfaced operator alert (DRIFT + Sentry + operator-visible) exists for the miss. *Never silent, and never allowed to block a batch.*

`DELIVERED`/`PICKED_UP` are fulfilled terminals — no obligation. Cash orders are structurally unaffected (no `payments` row ever reaches `'paid'`).

Two directions close the pincer (cancel-after-pay → where terminals are entered; pay-after-cancel → where `paid` is written), and two structural layers close every writer the pincer can't see.

### 3.2 Placement decision — FOUR decoupled layers (replaces v1's single fail-closed fold; fixes breaker C1/M2, adopts counsel's trigger hybrid)

**Design doctrine (from C1):** liveness of any batch/sweep must never be coupled to a money-ledger write; recording failures must surface to a human, never freeze a queue (ESC-2).

**L-A — app-fold in `updateOrderStatus`** (`apps/api/src/lib/orderStatusService.ts`), immediately after the status-guarded UPDATE succeeds, for `newStatus ∈ {CANCELLED, REJECTED}`:

```sql
INSERT INTO payment_events
  (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified)
SELECT p.id, p.location_id, p.provider, p.provider_payment_id, 'refund_due', p.amount_minor, p.currency_code, true
FROM payments p WHERE p.order_id = $1 AND p.status = 'paid'
ON CONFLICT (provider, provider_payment_id, type) DO NOTHING
```

- **Idempotency:** free, by the existing `payment_events_idem_unique (provider, provider_payment_id, type)` (`1790000000083_payments-ledger.ts`) — at most one `refund_due` per payment, replay-safe. Usually a no-op in practice: L-C fires first inside the same UPDATE statement and this insert hits the conflict — that redundancy is the point (defense-in-depth).
- **Fail-closed per-order, fail-LOUD, with a human exit (ESC-2):** if the insert fails, only **that order's** cancel tx aborts (single-order blast radius — never a batch) AND the failure **must** emit Sentry + a DRIFT counter + an operator-visible alert as part of the fold's contract. **Operator force-terminal escape hatch:** a conscious operator can force the cancel/reject with this fold SAVEPOINT-swallowed; doing so writes an audit row and fires the same friction-alert, and L-D keeps retrying the recording (and alarming) until it lands. Friction, never a verdict; the flip-gate checklist (§5) carries "escape hatch + alert path proven on staging".
- **Coverage via the funnel:** owner PATCH, mark-no-show, grace-cancel, courier abort (`bindingRelease`), `completeDelivery`'s refused tail (remove its own §5 insert in the same change — single writer restored; the unique makes the transition safe) — all already call `updateOrderStatus` with CANCELLED/REJECTED.

**L-B — webhook fold (pay-after-cancel)** — unchanged, §3.4.

**L-C — minimal `SECURITY DEFINER` trigger on `orders` (counsel's hybrid, ADOPTED — the structural floor).** AFTER UPDATE OF `status` WHEN new status ∈ {CANCELLED, REJECTED}: performs the same idempotent `refund_due` insert for paid payments of that order. Properties that make it C1-safe and B3-flip-safe:
- **Non-throwing by construction:** body wraps the insert in `BEGIN … EXCEPTION WHEN OTHERS THEN` (swallow → `RAISE WARNING`); a failure can NEVER abort the enclosing statement — so firing inside `app_sweep_timeout_orders()`'s fleet-wide atomic CTE cannot wedge the sweep. Its misses are deterministically caught by L-D's alarm, which is the real detector.
- **RLS-safe under FORCE, pre- and post-B3:** `SECURITY DEFINER` + per-row GUC dance — save `current_setting('app.current_tenant', true)`, `set_config('app.current_tenant', NEW.location_id::text, true)`, insert (dual policy's GUC arm `1790000000083:73-81` admits it), **restore the saved value** (set_config is tx-scoped, not fn-scoped — restore is mandatory or the trigger would contaminate the caller's tenant context; breaker should attack this).
- **Why a trigger now (not "if a third bypass appears"):** the grep-gate exempts exactly the DEFINER-fn class where LC6's original leak lived; "add the fold to each DEFINER fn by hand" is the remember-it-everywhere model that already failed. The trigger needs no allowlist and no memory — it covers present and future writers, raw UPDATEs included (proof P14). Owner-queue visibility needs only the row (`owner/refunds.ts:25-30` reads `payment_events` directly — no bus dependency); customer-facing workflow/notifications stay app-layer in L-A.
- **Migration M-1** (replaces v1's sweep-fold M-1, which is **dropped**): trigger fn + trigger, forward-only. `app_sweep_timeout_orders()` is **not modified at all** — sweep liveness is fully decoupled from money writes.

**L-D — reconciliation pass `app_reconcile_refund_due()` (Migration M-3).** `SECURITY DEFINER` PL/pgSQL fn, called by the existing timeout-sweep worker (`order-timeout-sweep.ts`) each tick right after the sweep:
- scans terminal non-fulfilled orders × `payments.status='paid'` × `NOT EXISTS refund_due` (bounded, indexed scan);
- inserts **per-row with `BEGIN/EXCEPTION` isolation** (one poisoned row never blocks the rest) + the same GUC dance per row;
- returns `{inserted, failed(order_id, reason)[]}`; the worker emits DRIFT counters + Sentry + an operator-visible alert for any `failed` rows or for any obligation older than N ticks — **the deterministic alarm of last resort** (proof P15);
- also scans terminal non-fulfilled orders carrying `mismatch` events (§3.6) and surfaces them in the same alert channel — surfaced, never auto-obligated.

### 3.3 The two previously-sanctioned bypasses

1. **Timeout sweep** — no longer a special case. The sweep stays byte-identical; L-C records the obligation in the same tx when healthy (trigger fires under the sweep's own UPDATE), L-D covers any miss within a tick. One poisoned row can no longer halt fleet-wide cancellation (proofs P6, P6b). *(Noted, out of scope: the sweep's pre-existing `order_status_history` insert is already not post-B3-safe — carried on the B3-flip checklist, DEP-2.)*
2. **Customer post-dispatch cancel** (`customer/orders.ts:308-341`, currently 500s on phantom columns — LC3). **DEP-1 is now a specified precondition, not an inheritance hand-wave (breaker H1 fix).** The LC3 fix MUST: (a) verify order ownership via the customer session token; (b) resolve the order's `location_id` and establish tenant context on the tx — `SELECT set_config('app.current_tenant', $loc, true)` — the exact precedent already live at `apps/api/src/routes/payments-webhook.ts:41` (DEFINER resolver → GUC → dual-policy GUC arm); (c) call `updateOrderStatus` inside that tx. L-A's insert then passes RLS under FORCE, pre- and post-B3 — the crypto-paid customer cancel (the case that most needs the refund) works instead of 500ing (proof P4b). Even a residual fold failure is alert-plus-escape (ESC-2), never a silent wedge, with L-C/L-D behind it.

### 3.4 Webhook: pay-after-cancel (`payments-webhook.ts` 'completed' branch)

In the same tx, after flipping `payments.status='paid'`:

```sql
-- order already terminal-non-fulfilled? record the obligation now.
INSERT INTO payment_events (…, 'refund_due', …)
SELECT … FROM payments p JOIN orders o ON o.id = p.order_id
WHERE p.provider='plisio' AND p.provider_payment_id=$1
  AND p.status='paid' AND o.status IN ('CANCELLED','REJECTED')
ON CONFLICT (provider, provider_payment_id, type) DO NOTHING
```

Decision: the `payments.status='paid'` + `orders.payment_status='paid'` flips STAY even on terminal orders — the money truth is that funds arrived; `paid + refund_due(unmatched)` is precisely the "paid-awaiting-refund" state the owner queue (`owner/refunds.ts:25-30`) already renders. Suppressing the flip would hide received money — worse. (Note for counsel: `payment_status='paid'` on a CANCELLED order becomes a meaningful, expected state; owner dashboard MAY later badge it — cosmetic, not this change.)

### 3.5 Widening the R2-3 fold to ALL terminals (`orderStatusService.ts:134`)

Today the assignment-terminalize fold runs on `CANCELLED` and `IN_DELIVERY→READY` only. Widen:

- `newStatus ∈ {CANCELLED, REJECTED}` → existing fold behavior (terminalize active binding, free shift). REJECTED is PENDING-only in the machine so an active binding is near-impossible — the fold is a no-op safety net there.
- `newStatus ∈ {DELIVERED, PICKED_UP}` — refuse with **409 in BOTH strand arms (breaker M6 fix — v1 covered only the first):**
  - **(a) active binding exists** (`offered/assigned/accepted/picked_up`) → `409 ASSIGNMENT_ACTIVE` instead of silently stranding (money H1's root);
  - **(b) order is `IN_DELIVERY` with NO `delivered` assignment** (binding drained by offer-expiry/abort race or manual state) → `409 USE_DELIVER_FLOW` — otherwise the PATCH passes with no attestation and the original silent strand survives via the back door.
  Rationale: DELIVERED must never be reachable without `completeDelivery`'s cash-as-proof spine; auto-terminalizing would fabricate a delivery with no attestation. `completeDelivery` terminalizes the assignment BEFORE calling `updateOrderStatus`, so the sanctioned path (including owner-proxy `/deliver`, `owner/dashboard.ts:447`) sees no active binding, is not `IN_DELIVERY`-without-attestation, and passes untouched; pickup orders have no binding. **Escape preserved:** a delivery order that was never dispatched (zero assignments, never entered IN_DELIVERY — phone/manual flow) remains owner-PATCHable to DELIVERED: there is nothing to strand and no courier cash in play.
- **Contract change (flagged, CC-1 widened):** owner PATCH `{status: DELIVERED}` changes from silent-200-and-strand → 409 (`ASSIGNMENT_ACTIVE` | `USE_DELIVER_FLOW`). FE must surface "complete via /deliver" for both codes. This is the H1 fix folded in structurally; the rest of H1/H4 (owner-proxy defaults) stays a separate council item.

### 3.6 Failure modes considered

| Mode | Behavior |
|---|---|
| `refund_due` insert fails on a **funnel** path | that single order's cancel tx aborts (per-order fail-closed, L-A) + Sentry/DRIFT/operator alert + operator force-terminal escape (§3.2, ESC-2) — never silent, never fleet-wide |
| `refund_due` insert fails **inside the sweep** (via L-C) | trigger swallows per-row (`RAISE WARNING`) — **sweep cancels everything regardless** (C1 fix, proof P6b); L-D records the obligation next tick or alarms |
| L-C and L-D both fail persistently on a row | L-D's `failed` return → DRIFT + Sentry + operator-visible alert every tick until resolved — un-recorded but never un-alarmed |
| Webhook replay (Plisio resends) | `payment_events_idem_unique` — no dup events, no dup obligations |
| Cancel and webhook race (both write `refund_due`) | same unique — exactly one row wins; both flips are status-guarded; L-C's insert in the same tx hits the same unique |
| Order with multiple payments rows | `SELECT … WHERE status='paid'` covers each; one obligation per paid payment |
| Over/underpaid (`mismatch`) then cancelled | **surfaced, not auto-obligated (breaker M3, partial fix):** L-D detects terminal orders carrying `mismatch` events and raises an operator alert (proof P16) — real received crypto on a dead order is never silent. Auto-`refund_due` is deliberately NOT created: the obligation amount is ambiguous (over/under vs recorded amount) and mis-stating a money obligation is worse than alert-plus-human. **Scoped claim:** this design closes LC6 **for `status='paid'` payments only**; full mismatch disposition = follow-up item `M3-mismatch-disposition` (§5), a precondition for calling crypto refunds "done" |
| Flags off (today) | all layers match zero rows (no paid payments exist) — ships dark, zero behavior change until crypto flip |

**Migrations: M-1** (trigger fn + trigger; sweep untouched) **+ M-3** (`app_reconcile_refund_due()`). **Contract change:** the widened 409 above (CC-1). Everything else is additive/internal.

---

## 4. Fix 3 — Settlement: idempotent, no-loss generation

### 4.1 Redesign of `app_generate_settlements` (Migration M-2, forward-only `CREATE OR REPLACE`)

Four coordinated changes; the function's period params become a LABEL for new payouts, not a filter that can lose money:

1. **Watermarked catch-up scan (kills both loss modes go-forward; breaker C2 fix — historical rows are NOT auto-swept).** Pair discovery AND item selection replace `delivered_at >= p_period_start` with `delivered_at >= <CATCHUP_FLOOR>` — a **literal deploy-watermark timestamp baked into the fn body at migration-authoring time** (immutable, forward-only, no config dependency); keep `delivered_at < p_period_end` + the existing `NOT EXISTS settlement_items` anti-join. Any **post-watermark** row missed by a run — SKIP LOCKED skip, crashed 2 AM job, whole missed day — is swept by the NEXT run into the next period's payout: those rows cannot have been reconciled out-of-band pre-fix, so resurrection is safe. **Pre-watermark rows are never touched by cron** (proof P9b) — they were lost from reconciliation for weeks and the courier↔venue cash was plausibly settled in person; auto-asserting them invites paying couriers twice (breaker C2). They move ONLY through the operator-gated backfill (1b). `FOR UPDATE OF ca SKIP LOCKED` is KEPT (dropping it would let one long app transaction stall the fleet-wide sweep); it is safe because a post-watermark skip is a deferral, not a loss.

   **1b. Historical backfill — operator-gated, flagged, never auto-paid (C2 fix + counsel §4 legibility, adopted).**
   - **Pre-count first:** a read-only report (script/DEFINER read fn) enumerates pre-watermark eligible rows (`delivered + cash_collected + NOT EXISTS settlement_items`) per courier×location pair — count + sum — so the operator sees the recovery magnitude BEFORE anything is created.
   - **Explicit invocation only:** a separate `app_backfill_historical_settlements()` (same M-2 migration, **never called by cron or any worker**) creates the rows only when the operator consciously runs it after reviewing the pre-count.
   - **Flagged as caught-up:** backfilled items get `settlement_items.backfilled = true` (additive column, M-2); owner UI and courier view render "N caught-up deliveries from before <fix-date> — total X" so a heavy payout is approved knowingly, and a courier watching a `pending` figure understands why it includes older deliveries.
   - **Never auto-pay:** rows land `status='pending'`; `/pay` is untouched and human-gated — three human gates total (pre-count review → explicit backfill run → per-payout pay). The operator MAY legitimately backfill only a subset or none (pairs already reconciled in person get `documented-no-action` in the ops record).
2. **Paid-payout immutability.** After the payout upsert, `SELECT … FOR UPDATE` the payout row; **if `status <> 'pending'` → skip the pair this run** (insert nothing, bump nothing). The unsatisfied items remain unsettled (`NOT EXISTS` still true) and roll into the NEXT period's fresh `pending` payout via (1). No second payout is ever forced into a closed period (the `(courier,location,period)` unique stays intact); a paid payout's numbers never move again.
3. **Aggregate-recompute totals (kills the counter bug).** Replace the incremental `deliveries_count/total_earned` bump with `UPDATE courier_payouts SET deliveries_count = (SELECT count(*) FROM settlement_items WHERE payout_id=…), total_earned = (SELECT COALESCE(sum(amount),0) …) WHERE id=… AND status='pending'`. Idempotent by construction; immune to `ON CONFLICT DO NOTHING` phantom counts; double-guarded by the pending check.
4. **Single-flight.** `PERFORM pg_advisory_xact_lock(hashtext('app_generate_settlements'))` at fn start — cron + `/regenerate` + any future caller serialize; combined with (3), even a lost race is harmless rather than inflating.

**Self-healing property (rescoped after C2):** from the watermark forward, the system automatically recovers every dropped row — no future data-repair scripts. Historical recovery is a deliberate, legible, operator-pulled act, not a deploy side-effect.

**Perf note (additive, optional in M-2):** partial index `ON courier_assignments (courier_id, location_id) WHERE status='delivered' AND cash_collected AND settlement_item_id IS NULL` so the catch-up anti-join stays cheap as history grows.

### 4.2 Doctrine — why §2.6 "never retro-mutate" and §4.1 backfill are coherent (counsel seam, resolved in-text)

**Auto-mutating money-out: never. Auto-surfacing an existing obligation into a human-gated `pending` queue: allowed.** A refund is money out the door — contested, irreversible → always a human decision (§2.6). A payout row is the surfacing of a debt that already exists (courier delivered, collected cash, is owed) into a queue a human still approves and pays. And after C2, pre-watermark history does not even auto-*surface* — it is operator-pulled with a magnitude pre-count (§4.1.1b). No historical money row is ever mutated in either direction.

### 4.2b Out-of-scope but recorded

`/regenerate`'s cross-tenant reach + unvalidated date (audit L4) is unchanged here — the catch-up design makes manual regeneration mostly unnecessary; L4 stays its own (authz) council item. `settlement-cron`'s pg-boss `singletonKey` no-op (synthesis R-E) is mitigated by 4.1(4) at the DB layer regardless of queue semantics. Settlement fns' cross-tenant no-GUC writes post-B3 (breaker L1) are **DEFER-FLAGGED** to the B3-flip checklist (DEP-2, §5) — the L-C/L-D per-row GUC pattern is the template to copy there.

### 4.3 Failure modes

| Mode | Behavior |
|---|---|
| Run crashes mid-fn | fn is one atomic tx — clean rollback, next run catches up in full (post-watermark) |
| Row locked by courier app at 2 AM | skipped → next run's catch-up picks it up (proof P8) |
| Whole day with no run | next successful run sweeps the gap (proof P9) |
| Late item, period payout already paid | pair skipped → next period's pending payout (proof P10) |
| Concurrent cron + regenerate | advisory xact lock serializes; recompute idempotent anyway (proof P11) |
| Owner approves/pays WHILE generation holds the payout FOR UPDATE | `/pay`'s guarded UPDATE waits on the row lock → ordered, consistent either way |
| Pair skipped every period (owner pays instantly each time — breaker L3) | **ACCEPT-RISK with mitigation:** deferral-not-loss (`settlement_items_assignment_uniq` guarantees no double-settle). Mitigation: a backlog drift metric — oldest unsettled `delivered + cash_collected` item age, alert at 7 days — plus an owner-UI "unsettled backlog" indicator. Owner: platform operator |

**Migrations: M-2** (fn + watermark + `settlement_items.backfilled` additive column + operator backfill fn + optional index). **Contract change: NONE** (route responses unchanged; payout periods may contain older post-watermark deliveries — surfaced via the backfilled/caught-up display notes, §4.1.1b).

---

## 5. Migration & contract-change register (flags for conductor)

| ID | What | Class |
|---|---|---|
| M-1 | `refund_due` structural-floor **trigger** (fn + trigger on `orders`, SECURITY DEFINER, non-throwing, per-row GUC save/restore) — **sweep fn NOT modified** (v1's sweep-fold dropped per breaker C1) | 🔴 migration, forward-only |
| M-2 | `CREATE OR REPLACE app_generate_settlements()` per §4.1 (watermark literal) + `settlement_items.backfilled` additive column + `app_backfill_historical_settlements()` operator fn + optional partial index | 🔴 migration, forward-only |
| M-3 | `app_reconcile_refund_due()` reconciler fn (SECURITY DEFINER, per-row exception isolation) + worker call + DRIFT/Sentry/operator alert wiring | 🔴 migration + worker change |
| CC-1 | Owner PATCH → `409 ASSIGNMENT_ACTIVE` (active binding) **or** `409 USE_DELIVER_FLOW` (IN_DELIVERY, no delivered assignment) for DELIVERED/PICKED_UP (was silent 200 + strand) | contract change — FE handling required for both codes |
| CC-2 | `payment_status='paid'` on CANCELLED/REJECTED orders becomes an expected state (paid-awaiting-refund) | semantic note, no schema/API shape change |
| CC-3 | `estimateOrderTotal` return gains additive `chargedTax`; inclusive receipts render "includes VAT (r%)" informational line (§2.7) | additive contract + FE presentation, i18n al/en |
| ESC-1 | Restitution: operator DECISION RECORD required for closure; **VAT-trace is a precondition to the decision** (§2.6) | **needs-human-decision** — encoded, not deferred |
| ESC-2 | Fail-closed paths fail to a surfaced alert + operator force-terminal escape; **flip-gate checklist item:** "escape hatch + alert path proven on staging before crypto flag-flip" (§3.2) | resolved-by-design; gate at flip |
| DEP-1 | LC3 customer-cancel fix MUST (a) verify ownership, (b) `set_config('app.current_tenant', loc, true)` on the tx (precedent `payments-webhook.ts:41`), (c) route through `updateOrderStatus` — proof P4b under FORCE-RLS non-bypass role | cross-design dependency, spec'd |
| DEP-2 | Post-B3 RLS strategy for system DEFINER surfaces: `app_generate_settlements` writes + sweep's `order_status_history` insert (breaker L1; pre-existing) — template = L-C/L-D per-row GUC pattern | **DEFER-FLAG** → B3-flip checklist |
| FLAG-1 | `M3-mismatch-disposition`: auto-obligation policy for over/under-paid (`mismatch`) funds — surfaced-only in this design (§3.6, P16); precondition for declaring crypto refunds fully closed | **MISSING (flagged)** — own council item, money red-line |

**Migration mechanics checklist (breaker L2):** `CREATE OR REPLACE` return signatures must match EXACTLY (`app_generate_settlements(timestamptz,timestamptz) RETURNS void`; sweep untouched); re-emit the `REVOKE ALL FROM PUBLIC` + `GRANT EXECUTE TO dowiz_app` block per the 078 pattern for every new/replaced fn; `pg_advisory_xact_lock(hashtext(...))` int4→bigint cast verified fine. Staging first per Ship Discipline; M-1/M-2/M-3 run on staging DB before deploy (boot-guard).

---

## 6. Red→green proof matrix (each written RED first; independent expected values, never mirrors)

| # | Fix | Proof (fails on today's code, passes after) | Independence guarantee |
|---|---|---|---|
| P1 | LC1 | `estimateOrderTotal(1200, {r:0.2, inclusive, fee:150}).total === 1350` + server composition vectors — **literal hand-derived constants** in a zero-import vector file, derivation comments mandatory | constants derived outside the codebase (bc/spreadsheet), vector file structurally cannot reference the implementation (§2.5 lint) |
| P2 | LC1 | property: inclusive ⇒ `total === subtotal + fee` across the whole rate matrix | oracle is the DEFINITION of inclusive pricing — no implementation referenced |
| P3 | LC1 | E2E: POST `/orders` on a staging fixture location (`tax_rate=0.2, price_includes_tax=true`) → `response.total === subtotal + deliveryFee` asserted via `request.*` | server-independent arithmetic in the test |
| P3b | LC1/M5 | route-level integration matrix: POST `/orders` with ≥4 vectors (inclusive/exclusive × zero/boundary rates) → response totals equal literal expected constants | server composition covered combinatorially BEFORE Option B; vectors from the same zero-import file |
| P4 | LC6 | integration: order + `payments(status='paid')` → `updateOrderStatus(CANCELLED)` → `SELECT count(*) FROM payment_events WHERE type='refund_due' … === 1`; run twice → still 1 (idempotent) | expectation is a DB state count, not a mirror |
| P4b | LC6/DEP-1 | customer cancel of a **crypto-paid** dispatched order under a **FORCE-RLS non-bypass role** → 200 + refund_due row (today: 500) | DB state + status code; exercises the GUC path, not BYPASSRLS |
| P5 | LC6 | webhook cancel-then-pay: CANCELLED order + pending payment → signed 'completed' POST → refund_due row exists AND owner `/refunds` lists it | end-state assertion via independent SELECT + route response |
| P6 | LC6 | timeout sweep: PENDING order past `timeout_at` + paid payment → `SELECT app_sweep_timeout_orders()` (fn byte-identical) → order CANCELLED **and** refund_due row exists (via L-C trigger) | DB state count; proves the floor without touching the sweep |
| P6b | LC6/C1 | **poison-row liveness:** force the refund insert to fail (e.g. constraint sim on payment_events) → sweep run still cancels ALL overdue orders across tenants; reconciler reports the row as `failed` and the alert fires | liveness asserted on orders table; alert asserted on reconciler return — v1's design fails this test by construction |
| P7 | LC6/H1 | owner PATCH `DELIVERED`: (a) active `picked_up` assignment → **409 ASSIGNMENT_ACTIVE**, assignment untouched; (b) IN_DELIVERY with drained binding → **409 USE_DELIVER_FLOW**; then `/deliver` path completes normally; (c) never-dispatched zero-assignment order → 200 | today (a)/(b) return 200 + strand → RED by status-code assert |
| P7b | LC1/M7 | E2E inclusive receipt: "includes VAT" label visible, NO additive tax row, displayed total === subtotal + fee | DOM assertions against literal expected strings/values |
| P8 | STL | session A holds `FOR UPDATE` on a delivered (post-watermark) assignment; run generate(period) → skipped; release; run generate(period+1) → item now in a payout | today: second run's window excludes it → RED |
| P9 | STL | delivered (post-watermark) row on day D, no run for D; run generate(D+1) → item present in D+1 payout | today: permanently lost → RED |
| P9b | STL/C2 | **pre-watermark** delivered+cash row → cron generate runs → row NOT in any payout; only `app_backfill_historical_settlements()` creates it, `pending` + `backfilled=true` | pins the watermark: auto-resurrection of history is a test failure |
| P10 | STL | payout paid via `/pay`; late unsettled item in that period; run generate → paid payout's `total_earned` UNCHANGED and item lands in next pending payout | today: paid payout bumps → RED |
| P11 | STL | run generate twice (and once with a pre-existing settlement_item) → `deliveries_count/total_earned === (SELECT count/sum FROM settlement_items)` exactly | aggregate equality against the items table, not against the fn's own counters |
| P12 | ratchet | vector-file lint: fires on a fixture vector file WITH an import / non-literal initializer, and on a test file importing beyond {module-under-test, vectors}; passes on the rewritten tests | the gate itself proven red→green on known-bad fixtures |
| P13 | STL/C2 | backfill flow: pre-count report totals === later-created backfilled rows' totals; created rows are `pending`+flagged; no code path from cron/worker reaches the backfill fn (grep + call-graph assert) | report-vs-created equality; absence asserted structurally |
| P14 | LC6/M2 | raw `UPDATE orders SET status='CANCELLED'` (deliberately bypassing the funnel) on a paid order → refund_due row exists (L-C trigger) | DB state count; today RED (no trigger exists) |
| P15 | LC6/ESC-2 | seed a terminal+paid order with refund_due missing (trigger disabled in fixture) → one reconciler run inserts it; seed a persistently-failing row → reconciler returns it in `failed` and the worker emits the DRIFT/Sentry/operator alert | end-state + alert-channel assertion, independent SELECTs |
| P16 | LC6/M3 | terminal order carrying a `mismatch` payment event → reconciler surfaces it in the operator alert listing; NO refund_due auto-created | both the presence (alert) and the absence (no auto-obligation) asserted |

Anti-cheat commitments: no `.only`/skip, no inflated timeouts, no `expect(true)`; every P# lands with a `docs/regressions/REGRESSION-LEDGER.md` row; the demoted parity test's header explicitly states it is drift-only so a future reader cannot re-promote it to a correctness proof.

---

## 7. Rollout order (reordered per counsel — live harms before dark work; nothing couples settlement behind LC6)

1. **LC1 Option A** + P1–P3b + §2.7 receipt presentation + P7b (live customer harm — smallest diff first; parity matrix untouched).
2. **Settlement M-2** + P8–P11 + P9b + P13 (live courier harm; watermarked; historical backfill is a separate operator act after the pre-count).
3. **LC6 layers** (L-A fold + L-B webhook + L-C trigger M-1 + L-D reconciler M-3 + §3.5 widened 409 + escape hatch) + P4–P7, P14–P16 (dark until crypto flip; CC-1 needs one FE affordance; flip gated on the ESC-2 checklist item).
4. **LC1 Option B** consolidation + P12 ratchet + parity-test demotion (behavior frozen by P1/P2; demotion only now — breaker M5).

ESC-1 (restitution decision record, VAT-trace precondition) runs in parallel from step 1 and gates **closure** of the LC1 remediation, never the hotfix itself.

## 8. Council round 1 — question dispositions (see `resolution.md` for the full board)

1. Fail-closed wedge hunt → breaker found it (C1, worse than anticipated: fleet-wide). **Resolved** by the four-layer redesign (§3.2): no batch is ever fail-closed; per-order failures alert + have an operator escape.
2. Owner-forced DELIVERED dead-end → verified NOT a dead-end: owner-proxy `/deliver` exists (`owner/dashboard.ts:447`); guard widened to both strand arms (§3.5) with the never-dispatched escape preserved.
3. Late-item deferral UX → **accepted with mitigation**: backlog age drift metric + owner-UI indicator (§4.3); plus `backfilled` flags for historical rows (§4.1.1b).
4. ESC-1 restitution → encoded as closure requirement with VAT-trace precondition (§2.6); the decision itself is the operator's.
