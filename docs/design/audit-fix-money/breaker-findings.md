# Breaker findings — MONEY audit fixes (Council STEP 2, ATTACK)

- **Target:** `docs/design/audit-fix-money/proposal.md` + `docs/adr/ADR-audit-fix-money.md`
- **Method:** every claim re-verified against HEAD source on `feat/phase0-safety-hardening`; break scenarios are grounded in the actual code paths cited below, not the proposal's self-description.
- **Scope of authority:** signals only. No fixes. Deterministic gates/council decide.

**Severity counts:** CRITICAL 2 · HIGH 1 · MED 6 · LOW 3 · (1 attempted-break that HELD, recorded for confidence)

---

## CRITICAL

### C1 — M-1 folds a fail-closed money INSERT into the fleet-wide timeout sweep → one bad write wedges ALL timeout cancellation
**Invariant violated:** liveness of a safety-net sweep must not be coupled to an unrelated money-ledger write; a fail-closed policy on a SHARED atomic batch turns a per-row failure into a fleet-wide outage.

`app_sweep_timeout_orders()` (`packages/db/migrations/1790000000078_phase2-sweep-fns.ts:13-22`) is **one `LANGUAGE sql` statement** — a single CTE that cancels *every* overdue PENDING order across *all* tenants in one pass, invoked by the 1-minute worker `apps/api/src/workers/order-timeout-sweep.ts:72` (`SELECT * FROM app_sweep_timeout_orders()`). M-1 adds a `refund_due` INSERT into `payment_events` as another CTE, chosen deliberately **fail-closed** (proposal §3.2: "the cancel of a paid order MUST NOT commit").

Because the fn is atomic and cross-tenant, if the `payment_events` insert fails for **any single order**, the whole statement aborts → **no order cancels that tick** → they stay PENDING → next tick re-selects the same poisoned row → the sweep is permanently wedged for **every tenant**. The worker's `catch` just logs and returns (`order-timeout-sweep.ts:116-118`), so this is silent and self-reinforcing.

Concrete trigger, near-term and planned: the **NOBYPASSRLS (B3) flip** (a known launch blocker — memory `launch-blocker-councils`). The fn sets **no `app.current_tenant`** and has no membership; `payment_events`' dual RLS `WITH CHECK` (`1790000000083:76-81`) admits `location_id ∈ app_member_location_ids()` (empty for a system actor) **OR** `= current_setting('app.current_tenant', true)` (NULL) → both false → the insert raises. Once ≥1 crypto-paid order has timed out (exactly LC6's target case), every fleet-wide timeout cancel halts. Note the existing `order_status_history` insert in the same fn (`:19-21`) is *already* not post-flip-safe (policy `1780338982015:19-20` is member-only, no GUC branch) — so M-1 does not create the fragility class, but it adds a **money** table to it and makes cancellation depend on it.

Blast radius: kitchens/couriers/customers stranded on un-cancellable orders; the sweep is also the detector for a stuck queue (`order-timeout-sweep.ts:44-57`), so the outage blinds its own alarm. This is strictly worse than the leak it prevents. **This is the single most dangerous flaw in the design.**

### C2 — Settlement self-backfill resurrects historically-lost rows → invites out-of-band DOUBLE payment (real money OUT)
**Invariant violated:** a self-healing backfill must not re-assert obligations that may already have been discharged outside the system.

M-2 §4.1(1) drops the lower period bound: scan becomes `delivered_at < p_period_end` + `NOT EXISTS settlement_items`. The proposal celebrates that "the first deploy's next run automatically recovers every historically dropped row … the fix IS the backfill." But the rows were *lost from reconciliation* precisely because `app_generate_settlements` SKIP-LOCKED/crashed them out (`1790000000078:160-197`) — meaning the courier↔venue **cash was very plausibly settled in person** while the system had no record. On first post-deploy run the catch-up sweeps that entire backlog into one fresh `pending` payout (`courier_payouts`), and the owner UI (`owner/settlements.ts:14-72`) presents it as owed. Approving/paying it (`/pay` :162-203) pays the courier **again** for cash they already kept at the door.

This fires **unconditionally on the first run** — no flag, no B3 flip — and moves real money in the *opposite* direction of the leak being fixed. There is no dedup against out-of-band settlement, and the proposal downgrades it to "display-level note only" (§4.3, §5 M-2), which understates it. (It is the most *certain-to-fire* issue here; C1 has the larger blast but a conditional trigger.)

Note: **double-settle is correctly prevented** — `settlement_items_assignment_uniq` (`1780421100045:15`) is a hard unique on `assignment_id`, so an item can never land in two payouts and `total_earned` can't double-count. The hazard is not DB double-settle; it is a fresh *system* obligation for cash already reconciled by humans.

---

## HIGH

### H1 — DEP-1 (route LC3 customer-cancel through `updateOrderStatus`) grafts the fail-closed fold onto a context-free customer connection → re-breaks the exact path, now fail-closed
**Invariant violated:** a mutator's embedded RLS-bearing writes require the caller to have established a member/tenant context; grafting it onto a path that hasn't is a latent fail-closed trap.

The customer post-dispatch cancel (`apps/api/src/routes/customer/orders.ts:307-336`) today runs raw `UPDATE orders SET status='CANCELLED', cancelled_at=now(), cancellation_reason=$1` on a bare `db.connect()` client — and **`orders` has no `cancelled_at`/`cancellation_reason` column** (confirmed: only `courier_assignments` defines them; `orders` create table `1780310074262:21-43` has neither), so this route 500s on every call (LC3 confirmed dead). DEP-1 mandates rerouting it through `updateOrderStatus` to inherit D2.

But `updateOrderStatus` + the D2 fold does `INSERT INTO payment_events …` (proposal §3.2) and the customer route holds **no member and sets no `app.current_tenant`**. For a **crypto-paid** order — the one case that most needs the refund — the fold's `payment_events` `WITH CHECK` fails (dual policy, C1 analysis) → fold raises → **fail-closed → the customer cancel 500s again**, now leaving the customer stuck IN_DELIVERY with funds held *and* unable to cancel. Masked today by BYPASSRLS; detonates on the B3 flip. The cash-immutable trigger (`1780421100052:24-36`) is *not* the blocker here (at IN_DELIVERY the assignment is `picked_up`, `cash_collected=false`, so `prevent_cash_mutation` is a no-op and the lost `settlement_reversal` GUC doesn't matter) — the RLS context is.

---

## MEDIUM

### M2 — Cross-transaction lost-refund is closed by the pincer, but ONLY for funnel/webhook paths; every raw `UPDATE orders SET status` remains an un-backstopped leak
The cancel-fold + webhook-fold pincer is actually **sound** for the race: the webhook (`payments-webhook.ts:65-70`) takes the `orders` row lock when it flips `payment_status`, so a concurrent cancel serializes behind it and the webhook's post-lock re-read of `orders.status` (proposal §3.4) sees CANCELLED and inserts `refund_due`; symmetrically the cancel-fold sees `status='paid'` once the webhook commits. I could not break the two-writer race **given both folds ship together**. The residual: any terminal path that does a **raw** `UPDATE orders SET status` bypasses both folds with no backstop — the customer path (pre-DEP-1) and any future sinner. The only guard is the proposed grep/eslint ban on raw status UPDATE, which is advisory-strength (see M4).

### M3 — The mismatch (over/under-payment) crypto black hole stays OPEN — the fix keys strictly on `status='paid'`
Every fold selects `payments … WHERE status='paid'` (`deliveryCompletion.ts:130-134`, proposal §3.2/§3.4). The webhook records over- and under-payment as type `'mismatch'` with **no status flip** (`payments-webhook.ts:84` comment). So a cancelled order that received **real crypto** — including an **overpayment** where the customer sent *more* — carries no `refund_due` and never surfaces in `owner/refunds.ts:25-30`. The proposal defers this to "M3, out of scope" (§3.6), but it is the same "crypto refund black hole" the change claims to close, and it leaks received funds. At minimum the ADR should stop describing LC6 as closed.

### M4 — The mirror-oracle lint/grep ratchet is trivially evadable
The proposed gate (§2.5, P12) fires on `assert.equal(actual, expected)` where `expected` inline-calls an export of the same/mirror module. Three evasions that a grep or a shallow AST rule miss: (a) alias the fn — `const f = serverApplyTax; … expected = f(…)`; (b) hoist — `const expected = serverApplyTax(…); assert.equal(x, expected)` (no call node in arg 2); (c) launder through `estimateOrderTotal`, whose body calls `applyTax`. Without a declared mirror-registry the rule is a speed-bump, not a guarantee; the disease (implementation-derived expectation) recurs.

### M5 — Demoting the 432-combo parity matrix thins server-composition coverage during the hotfix window
`fee-parity.test.ts:52-76` currently cross-checks FE≡BE over 12×6×2×3 = 432 combos. The *server* composition (`orders.ts:509-511`) is inline in the route handler — **not a pure fn** — so it has no unit test; the only server-composition proof pre-Option-B is one E2E fixture (P3: single rate, inclusive). Between commit 1 (hotfix) and commit 4 (Option B extracts `composeOrderTotal`), demoting the matrix to a "drift detector" removes combinatorial cross-plane composition coverage without an equivalent server-side matrix to replace it. The property test P2 (`inclusive ⇒ total===sub+fee`) only helps if it runs against BOTH planes, which requires Option B first.

### M6 — §3.5's 409 only PARTIALLY closes money-H1 (no-binding IN_DELIVERY still silent-strands)
The widening throws `409 ASSIGNMENT_ACTIVE` only when an **active** binding (`offered/assigned/accepted/picked_up`) exists. An order left IN_DELIVERY after its binding drained (offer expired / courier abort → `bindingRelease` reverts to READY, but a race or manual state can leave IN_DELIVERY with no active binding) → owner PATCH `{DELIVERED}` finds no active binding → **passes 200**, skipping `completeDelivery`'s cash-as-proof spine → the original silent strand (no `delivery_trace`, no hold) persists. The proposal's framing that this "closes money-H1's stranding" overstates; it closes only the active-binding half. (The legitimate owner-forced-delivery flow IS covered: owner-proxy `/deliver` exists at `owner/dashboard.ts:447` and routes through `completeDelivery` — so CC-1 is a real but non-dead-end contract change.)

### M7 — Receipt/label incoherence reproduces the original "everyone stares at a wrong-looking number"
Post-fix, for an inclusive venue `estimateOrderTotal` still returns `taxTotal` (e.g. 200) for display while `total` no longer adds it (`packages/ui/src/lib/money.ts:79-85` after the ternary). Unless the FE relabels this line to "incl. VAT," a receipt renders `subtotal 1200 + tax 200 = total 1350` — an apparent arithmetic error — recreating exactly the "customer, owner, and total all agree on a number that looks wrong" failure mode the fix set out to kill. The proposal notes `taxTotal` "keeps its informational meaning" but never specifies the FE presentation change.

---

## LOW / context

### L1 — M-2 preserves the cross-tenant no-GUC pattern → whole settlement generation errors post-B3-flip
`app_generate_settlements` writes `courier_payouts` / `settlement_items` / `settlement_audit_log` across many tenants without ever setting `app.current_tenant`; their policies are bare `location_id = NULLIF(current_setting('app.current_tenant', true),'')::uuid` (`1790000000077:84-85`, `1780421100045:20-21`). Works only under today's BYPASSRLS. Pre-existing, not worsened by M-2 — but M-2 was the natural moment to address it and the redesign is silent on it.

### L2 — M-1/M-2 `CREATE OR REPLACE` mechanics can hard-fail the migration on prod
The proposal shows only fn bodies. Two forward-only foot-guns: (a) the return signatures must match EXACTLY (`app_sweep_timeout_orders() RETURNS TABLE(id uuid, location_id uuid)`; `app_generate_settlements(timestamptz,timestamptz) RETURNS void`) or `CREATE OR REPLACE` errors "cannot change return type of existing function"; (b) the `REVOKE ALL FROM PUBLIC` + `GRANT EXECUTE TO dowiz_app` block (078 pattern) must be re-emitted. `hashtext('app_generate_settlements')` → int4 → `pg_advisory_xact_lock(bigint)` cast is fine. A migration that raises mid-deploy trips the boot-guard. Mechanical, but it's a red-line migration on prod.

### L3 — Settlement indefinite-deferral corner
If a courier's current-period payout is always non-`pending` (owner approves/pays instantly each period) before the next catch-up runs, §4.1(2) skips the pair each time and late items roll forward forever. Realistic cadence makes it a deferral, not a loss; noted for the "unsettled backlog indicator" open question (§8.3).

---

## Attempted break that HELD (recorded for council confidence)

### The tax fix does NOT regress the non-inclusive path, and the P1 constants recompute correctly
Enumerating all four combinations against the actual `applyTax` (`apps/api/src/lib/money.ts:14-21`) and the fixed composition `total = subtotal + fee + (priceIncludesTax ? 0 : taxTotal)`:

| incl | rate | applyTax(1200/1000…) | chargedTax | total | verdict |
|---|---|---|---|---|---|
| false | 0 | 0 (short-circuit `:5`) | 0 | sub+fee | ✓ unchanged |
| false | >0 | additive tax `:21` | **taxTotal** | sub+fee+tax | ✓ **identical to today** (ternary only zeroes the inclusive arm) |
| true | 0 | 0 | 0 | sub+fee | ✓ |
| true | >0 | extracted tax `:14-18` | **0** | sub+fee | ✓ **the fix** |

The exclusive path is provably untouched — the ternary cannot double-subtract or under-charge it (it never alters the `false` arm). Independently recomputed P1 vectors all check out: `applyTax(1200,0.2,true)=200`, total 1350; `applyTax(1075,0.075,true)=75`, total 1075; `applyTax(1000,0.2,false)=200`, total 1450; `applyTax(1000,0.0745,false)=(1000·74500+500000)/1e6=75`. The cash-422 backstop (`orders.ts:514`) only relaxes as `total` drops, so no new rejection. No finding — the tax-math core is sound; the risk lives entirely in the guardrail/coverage/consolidation surface (M4–M6) and the LC6/settlement folds (C1, C2, H1).
