# Breaker Findings — Stage-21 Cash Reconciliation (B1)

Adversarial pass over `docs/design/stage21-reconciliation/proposal.md`. Read-only verification against
live source/migrations. Ranked by money-loss exploitability. **No fixes** — mechanism integrity only.

Verification anchors (live tree): `1780421100052_payout-trigger-gps-precision.ts`,
`1790000000028_courier-cash-ledger.ts`, `1780421100043_courier-payouts-scaffold.ts`,
`1790000000015_operational-pool-role.ts`, `1780421100065_lockdown-nontenant-api-surface.ts`,
`apps/api/src/workers/settlement-cron.ts`, `apps/api/src/routes/owner/settlements.ts`,
`apps/api/src/routes/courier/settlements.ts`, `apps/api/src/routes/courier/me.ts`,
`apps/api/src/workers/backup/smoke-checks.ts`, `apps/api/src/lib/deliveryCompletion.ts`.

---

## [CRITICAL] B-DATA · the rename breaks the `prevent_payout_mutation` trigger → every `courier_payouts` UPDATE throws

`§5.1` proposes `ALTER TABLE courier_payouts RENAME COLUMN total_earned TO collected_total;` + a COMMENT,
and `§9` calls it "unflagged (pure honesty fix, **no behavior change**)." It is not.

`prevent_payout_mutation()` (migration `1780421100052:6-16`) is a BEFORE-UPDATE trigger whose **PL/pgSQL body
references `OLD.total_earned` / `NEW.total_earned`** (line 9-10). `RENAME COLUMN` does **not** rewrite stored
function bodies. The instant the rename lands, the trigger fires on **every** UPDATE to `courier_payouts` and
raises `record "new" has no field "total_earned"`.

Break scenario (no flag, no new code needed): owner taps Approve/Pay/Dispute/Reopen
(`owner/settlements.ts:122,178,222,273` all do `UPDATE courier_payouts SET status=…`) → trigger throws → 500.
The daily settlement-cron `UPDATE courier_payouts SET … total_earned = total_earned + $2`
(`settlement-cron.ts:100-105`) throws on the column ref **and** the trigger. **The entire payout
state-machine is dead** the moment the migration runs. The DoD (§DoD 1-6) has **no** test that an UPDATE to
`courier_payouts` still succeeds after the rename, so it ships green.

**Violated invariant:** forward-only money migration must not break a live dependent object; "no behavior
change" is false — a RENAME on a 🔴 table with a trigger/function dependent is a behavior change.

---

## [CRITICAL] B-CONSIST · no constraint prevents `release` + `settle` coexisting on one order → negative net (phantom courier credit) OR silent owner loss

`§4` asserts the two contra types are "mutually exclusive per order **by construction** … enforced by
`NOT EXISTS` + the existing `UNIQUE(order_id, type)`." Verified false: in migration `028` the only guards are
`CHECK (type IN ('hold','release','settle'))` and `UNIQUE (order_id, type)`. `UNIQUE(order_id,type)` permits
`(X,'release')` **and** `(X,'settle')` simultaneously — different `type` values. The mutual exclusion lives
only in a **read-time `NOT EXISTS`**, which is TOCTOU under READ COMMITTED (the endpoints use plain `BEGIN`,
no SERIALIZABLE).

Break scenario A — concurrent (race): refund-tx for order X commits `release X 3000`; settle-tx (owner
shift-close) had already `SELECT`ed X as outstanding before that commit, then inserts `settle X 3000`
(`ON CONFLICT(order_id,'settle')` does not collide with the release row). Result: X has
`hold − release − settle = 3000 − 3000 − 3000 = −3000`. The courier's net shows a **3000 credit they never
earned**.

Break scenario B — sequential, no concurrency (the adversarial variant the task names: *refund AFTER the cash
drop*): shift closes first → `settle B 3000` written. Customer refunds B post-delivery. The release path
"only targets orders with a hold and no settle" (§4) → release **skipped** → **no contra for a refund that
physically returned cash to the customer**. Ledger reads `hold − settle = 0` ("clean, owes nothing") while the
owner has paid the customer 3000 out of their own till with **zero ledger record**. The owner silently eats
the refund. (If instead the release is written anyway → −3000 phantom credit, scenario A.) Either branch is
money corruption; the worked example (§2) only nets to zero because it hard-codes refund-*before*-settle.

**Violated invariant:** netting integrity (`hold − release − settle = 0`) must be a structural guarantee, not
a read-time assumption; mutual-exclusion of contra types is unenforced.

---

## [HIGH] B-SEC · `courier_payouts` FORCE RLS is a placebo (runtime role is BYPASSRLS) — and if it weren't, it self-DoSes the owner UI + cron

`§5.3` claims `ALTER TABLE courier_payouts FORCE ROW LEVEL SECURITY` "closes the owner/BYPASSRLS bypass." This
is mechanically wrong both ways:

- The codebase states the app/session pool runs as a **BYPASSRLS** role — `1780421100065:8,21,35`
  ("App accesses them via operational/session pools with **BYPASSRLS roles**"; "API roles … use BYPASSRLS").
  `FORCE` only subjects the **table owner** to its own RLS; it has **no effect on a separate BYPASSRLS login
  role** — those bypass RLS regardless of FORCE. So the 🔴 money-RLS gap stays exactly as open as before. The
  "fix" changes nothing for the actual writer.
- Conversely, if the writer were NOBYPASSRLS: `owner/settlements.ts` sets **no** tenant GUC (grep:
  zero `set_config`/`withTenant`), and the policy is `location_id = current_setting('app.current_tenant')::uuid`
  (`1780421100043:23`). Under FORCE that `current_setting` with no `true` fallback **raises** when the GUC is
  unset → the list handler's `try/catch` swallows it and returns `{ payouts: [] }` (silent money-blindness:
  owner sees no settlements), the detail/approve handlers 500. The cron (`settlement-cron.ts`, also no GUC,
  needs INSERT/UPDATE which the SELECT-only operational role `1790000000015:33` doesn't even hold) is denied →
  **settlement generation DoS**.

There is no role choice under which FORCE both (a) closes the gap and (b) keeps the cron+owner path alive —
the proposal's security claim and its "connection budget delta = 0 / boring & proven" claim are mutually
exclusive. Mirror of the B3/B4 NOBYPASSRLS lesson.

**Violated invariant:** a money-table RLS hardening must actually bind the live writer role; FORCE without a
NOBYPASSRLS writer + a context the writer sets is theater.

---

## [HIGH] B-DATA · two-ledger divergence — a refund moves the cash-ledger + assignment but NOT the already-generated `collected_total`/`settlement_items` snapshot

`§2` claims "the two ledgers agree: 5000 collected = 5000 reconciled = 0 owed." That holds **only if the
nightly cron has not yet generated a payout for the period.** The cron (`settlement-cron.ts:60-105`) snapshots
`total_earned += cash_amount` and creates `settlement_items` at 2 AM, and **never removes** an item
(`NOT EXISTS settlement_items` only ever *adds*).

Break scenario: cron runs at 02:00 with B included → `collected_total` (renamed) = 6500, `settlement_items`
includes B. Next day B is refunded post-delivery → refund path sets `cash_amount=NULL` + writes `release B`
(§6) but touches **neither** `collected_total` **nor** `settlement_items`. Now:
`collected_total = 6500` (stale snapshot the owner UI surfaces as "cash to reconcile") vs cash-ledger net
`6500 − 1500 = 5000`. The owner is shown **1500 of phantom cash to collect** that no longer exists. There are
now four divergent money figures for the same shift (`collected_total`, `SUM(settlement_items.amount)`,
`SUM(assignments.cash_amount where collected)`, `hold−release−settle`), and the refund only moves two of them.

**Violated invariant:** read-after-write coherence across the settlement snapshot and the cash ledger; a
refund must not leave a money figure the owner acts on stale.

---

## [HIGH] B-CONSIST · all-or-nothing reconciliation → a 1-unit miscount strands the whole shift as permanent phantom debt

`§6`/`§7`/`RK-1`: if `confirmed_total ≠ SUM(holds)` the server records `status='discrepancy'` and writes
**zero** settle rows ("MVP = full-reconcile or none"). The discrepancy-handling UI is DEFER-FLAG.

Break scenario: owner counts 6499, computed 6500 (one coin / rounding in a cash count). No settles written →
**all** of A+B+C holds (6500) remain outstanding → courier shows owing the **entire shift** despite having
handed over essentially all of it, with **no clearing path** until the deferred UI ships. Back-of-envelope:
450 orders/day, ~3 couriers × 5 locations; if even 5% of shift-closes have a small miscount, that fraction of
couriers carry full-shift phantom debt indefinitely, compounding nightly. This is the "courier eternally
owing" failure the task names, reachable with no malice — just an off-by-one cash count.

**Violated invariant:** reconciliation must not let a trivial discrepancy invert into a large standing debt
with no mechanism to clear it.

---

## [MEDIUM] B-DATA · `prevent_ledger_mutation` trigger collides with the existing `ON DELETE CASCADE` from `orders`

`§5.2` adds a `BEFORE UPDATE OR DELETE` trigger on `courier_cash_ledger` that `RAISE`s on any DELETE. But
`courier_cash_ledger.order_id` is `REFERENCES orders(id) **ON DELETE CASCADE**` (migration `028:15`).

Break scenario: any path that deletes an `orders` row (GDPR hard-erase, test teardown, admin cleanup) triggers
the cascade DELETE into `courier_cash_ledger` → `prevent_ledger_mutation` raises → **the order delete fails**.
Immutability and the FK's cascade are in direct contradiction; whichever the owner relied on for order
deletion now throws. The proposal asserts the trigger "does not impede §4" (true for INSERT contras) but never
checks the inbound cascade.

**Violated invariant:** a new immutability guard must be compatible with existing referential actions on the
same table.

---

## [MEDIUM] B-SEC/authority · the debt is written against a courier who cannot read the ledger and never acknowledges it

R1 (owner-only confirm) + `§8`/`RK-3`: the `courier_cash_ledger` policy is member-only
(`location_id IN (SELECT app_member_location_ids())`, migration `028:27`); couriers live in
`courier_locations`, not memberships, so **under FORCE a courier cannot SELECT their own holds/settles**
(proposal admits this is DEFER-FLAG). Combined with no courier-side declaration (R2 deferred), a `settle` (or a
withheld settle) is created purely on owner action with **no courier-visible artifact and no courier ack**.

Break scenario (mechanism, not fairness): owner declines to confirm a real drop → courier shows owing forever,
and the courier has **no read path** to the ledger to evidence the dispute. The mechanism produces an
unfalsifiable, courier-invisible debt record. (Fairness is Counsel's; the integrity gap — a money record with
no second-party visibility or assent — is the mechanism defect.)

**Violated invariant:** a per-courier money obligation should have a courier-readable record before it is
relied upon.

---

## [MEDIUM] B-ANTIPATTERN · RK-5 coherence "guardrail" is a CI test, not a structural guard — it does not stop a half-built reversal at runtime

`RK-5`/`§6` ship the `release` primitive ahead of its refund caller and claim "the coherence guardrail goes
red the moment a refund path forgets the contra." The guardrail (§6, DoD-6) is a **test** comparing cash-ledger
net to "`SUM(cash_amount)` of un-`settle`d delivered assignments." Two problems: (1) `courier_assignments`
has no `settle` concept (the term is the cash-ledger's, not the assignment's) → the second term is
ill-defined and likely diverges from both `collected_total` and `settlement_items`; (2) a test only goes red
if a future refund path **also ships a test that exercises it** — a refund caller merged without that test
silently corrupts nets in prod with no red. "Red the moment a refund forgets" overstates a CI artifact's
reach; nothing structural enforces the contra at write time.

**Violated invariant:** a money-coherence claim should rest on a DB constraint/trigger, not on future authors
remembering to extend a test.

---

## [MEDIUM] B-DATA · `release` amount is the full hold — a partial post-delivery refund over-reverses

`§4`/`§6`: `release` is one row, `amount = hold` (full order total). A partial refund (customer refunded part
of an order) would write a full-hold `release`, crediting back more than was returned.

Break scenario (when the refund caller lands per RK-5): order total 3000, customer refunded 1000 → release
writes 3000 → net `hold − release = −2000` phantom courier credit. The primitive bakes in a full-order-only
assumption with no `amount` parameter tied to the actual refund, and `cash_amount=NULL` (full wipe) on the
assignment compounds it.

**Violated invariant:** a contra must reverse the actual amount moved, not a fixed full-hold figure.

---

## [LOW] B-OPS/B-DATA · the rename also breaks the backup money-integrity smoke check (and that check can't see refund divergence anyway)

`backup/smoke-checks.ts:145-152` (`checkPayoutSums`) queries `cp.total_earned` and asserts
`cp.total_earned = SUM(settlement_items.amount)`. After the rename it throws `column total_earned does not
exist` → the `payout_sums` smoke check false-reds → noise/blockage in the backup-verify pipeline (a money
safety net). Separately, even when working, both terms are stale snapshots, so it cannot detect the §HIGH
refund divergence — it stays green while the real cash differs.

**Violated invariant:** a rename on a 🔴 table must enumerate every reader; a money smoke check must not be
silently broken by it.

---

## [LOW] B-DATA · currency is hard-coded `'ALL'` / sums mix currency_code without grouping

`owner/settlements.ts:147` publishes `currency: 'ALL'` hard-coded, and the cron sums
`addedTotal += ca.cash_amount` across assignments while only *reading* per-assignment `loc.currency_code`
(`settlement-cron.ts:61,95`) — no GROUP BY currency. Single-currency in-variant mitigates today, but the
reconciliation/`collected_total` figure has no currency dimension, so any multi-currency location silently
sums mixed minor units into one integer. Integer money itself is clean (no float found).

**Violated invariant:** a money total must carry/segregate its currency; a hard-coded currency on a money
event is a latent mixing bug.

---

### Net

The two CRITICALs are independent of the refund caller existing: the **trigger break** (rename vs
`prevent_payout_mutation`) fires on the next owner tap, and the **unconstrained `release`+`settle`
coexistence** corrupts nets on any refund-after-settle or settle∥refund race. The **FORCE-RLS placebo** means
the headline security win of the change does not exist against the live BYPASSRLS writer. None of these are
caught by the proposed DoD (§DoD 1-6).
