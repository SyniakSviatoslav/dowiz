# Design Proposal — S5 MONEY batch → Rust port (R2b)

Status: RESOLVED (design-time; RESOLVE rounds R1 **and R2** applied — see `resolution.md`). Author: System Architect.
RESOLVE R2 deltas (2026-07-05): **lock-order unification made GLOBAL** (the R1 draft reordered only deliver → left
AB-BA for accept/pickup/cancel/abort AND introduced a new deliver-vs-cancel deadlock — N1); **raced-terminal predicate
narrowed** to exclude DELIVERED-by-self (the broad predicate manufactured a phantom reconcile + false alert on a
successful-deliver retry — N2); **reconcile made observably-idempotent** (alert gated on `rowcount=1`, replay echoes the
existing `reconcileId` — N5); **STOP-1 "owner-visible" honestly downgraded + gated** (the ledger is audit-only, nothing
reads it — the owner-surface is a gated S6 deliverable — N3 / OPEN-1); **M-B redesigned to a spillover destination** (a
bare status-guard silently underpays; the loud RAISE is honester than a silent skip — N4); **'reconcile' obligation-sum
semantics fixed** (excluded from any future Σhold — OPEN-2); **M5 crypto-launch forward-gate** added.
RESOLVE R1 deltas: Flag-A fix **re-architected** (lock-reorder o→ca + structural write-gating + durable
raced-terminal reconcile, replacing the dead `409` of the prior draft); batch is **no longer zero-migration** (two
operator-placed red-line drafts, M-A/M-B); promotions ship **Node-kept + fail-loud**, not a `{promotions:[]}` stub;
§7a conscious parity-DEVIATIONS and §7b conscious CARRIES added. Read `resolution.md` for per-finding adjudication.
Canon: `docs/design/rebuild-plan/backend-tail-batches.md` §R2b; `docs/ops/reliability-gate-cutover-2026-07-05.md` (flag A);
`docs/design/rebuild-orders-s5-council/resolution.md` (REV-S5-1..9); ADR-audit-fix-money; ADR-deliver-v2-cash-as-proof; ADR-0017 (payments).
ADR draft: `docs/adr/ADR-s5-money-batch-rust-port.md`.

---

## 1. Problem + non-goals

**Problem.** The strangler tail keeps **23 S5 (money) routes on Node** behind the front-door while 9/10 surfaces
serve on Rust. S5 is the crown-jewel red-line: every route touches `orders` / `payments` / `courier_assignments` /
`courier_payouts` — a port defect is a **charge defect or a state defect**, not a cosmetic one. The batch must be
ported to Rust at **byte-parity** (status codes AND body shape) without loosening any of the proven invariants, and
it must **resolve the held courier-deliver conflict-bool race** (reliability-gate flag A) because the deliver /
assign-courier routes in this batch overlap it.

The 23 routes (keep-set at HEAD `6b04a828`):

| # | group | route | Node handler | engine-alias? |
|---|---|---|---|---|
| 1 | order-action | POST `/:loc/orders/:id/assign-courier` | `owner/dashboard.ts:215` | partial — own tx + `apply_transition(IN_DELIVERY)` |
| 2 | order-action | POST `/:loc/orders/:id/pickup` | `owner/dashboard.ts:379` | NO — assignment-only advance (order stays IN_DELIVERY) |
| 3 | order-action | POST `/:loc/orders/:id/deliver` | `owner/dashboard.ts:447` → `deliveryCompletion.ts` | YES via `completeDelivery`→`apply_transition` — **race** |
| 4 | order-action | POST `/:loc/orders/:id/mark-no-show` | `owner/signals.ts:198` | YES `apply_transition(CANCELLED)` + counters |
| 5 | order-action | POST `/:loc/orders/:id/reveal-customer-contact` | `owner/reveal-contact.ts:15` | NO — PII reveal + audit |
| 6 | order-action | GET `/:loc/orders/:id/verify` | `owner/dashboard.ts:539` | NO — read, decrypt+mask |
| 7 | settlement | GET `/:loc/settlements` | `owner/settlements.ts:14` | NO |
| 8 | settlement | GET `/:loc/settlements/:id` | `owner/settlements.ts:75` | NO |
| 9 | settlement | POST `/:loc/settlements/:id/approve` | `owner/settlements.ts:110` | status-guarded UPDATE |
| 10 | settlement | POST `/:loc/settlements/:id/pay` | `owner/settlements.ts:162` | status-guarded UPDATE |
| 11 | settlement | POST `/:loc/settlements/:id/dispute` | `owner/settlements.ts:206` | status-guarded UPDATE |
| 12 | settlement | POST `/:loc/settlements/:id/reopen` | `owner/settlements.ts:257` | status-guarded UPDATE |
| 13 | settlement | POST `/:loc/settlements/regenerate` | `owner/settlements.ts:301` → `settlement-cron.ts` | trigger→worker |
| 14 | refund | GET `/:loc/refunds` | `owner/refunds.ts:17` | DARK (prepaid off → `{refunds:[]}`) |
| 15 | refund | POST `/:loc/refunds/:pid/sent` | `owner/refunds.ts:43` | DARK (prepaid off → 404) |
| 16 | read | GET `/:loc/dashboard/snapshot` (= owner orders list) | `owner/dashboard.ts:23` | NO — heavy DTO + PII |
| 17 | read | GET `/api/customer/orders/:id/status` | `customer/orders.ts:21` | NO — heavy DTO + PII |
| 18 | read | POST `/api/customer/orders/:id/rating` | `customer/orders.ts:219` | NO — upsert |
| 19 | messages | GET `/api/orders/:id/messages` | `order-messages.ts:124` | NO |
| 20 | messages | POST `/api/orders/:id/messages` | `order-messages.ts:32` | NO — preset registry + tri-role authz |
| 21 | messages | POST `/api/orders/:id/messages/read` | `order-messages.ts:161` | NO |
| 22 | webhook | POST `/webhook/payments/plisio` | `payments-webhook.ts:13` | NO — HMAC + DEFINER + idempotent |
| 23 | promotions | CRUD+validate (S3-tagged, pricing-affecting) | `owner/promotions.ts` | **Node-kept + fail-loud** (RESOLVE C1′, STOP-2) — NOT a `{promotions:[]}` stub |

Already ported (the proof engine-reuse works, do not re-touch): `POST /orders`, `GET /orders/:id`,
`PATCH /orders/:id/status`, `POST /customer/orders/:id/cancel`, `.../confirm`, `.../reject`, `.../metadata`
(`rebuild/crates/api/src/routes/orders/{mod.rs,pg.rs}` — `owner_update_status` / `owner_order_action` /
`customer_cancel` / `apply_transition`).

**Non-goals.**
- ~~No schema change, no new migration, no new SECURITY DEFINER function.~~ **AMENDED by RESOLVE R1.** Two forward-only,
  atomic, **operator-placed** red-line migrations are now required and drafted (neither inlined by the port): **M-A** adds
  `'reconcile'` to `courier_cash_ledger.type` CHECK (STOP-1 durable cash-truth); **M-B** status-guards the
  `app_generate_settlements` DEFINER bump + adds per-location scope + a **spillover destination for late in-period
  earnings** (H3 self-poison / M6 / RESOLVE R2 N4 — a bare guard silently underpays). The route *runtime* port
  remains schema-light ("схема багата, рантайм мінімальний"), but the money-integrity findings force these two seams —
  see §5. No *other* schema change; no re-implementation of settlement math in a route.
- No settlement/pricing **math** re-implementation (settlement generation stays worker-owned; refund/pay are pure
  state reads/writes — REV-S7-3 posture).
- No redesign of the proven transition engine (`apply_transition`) — bless reuse (packet §R2b).
- No prod flip. Dark-mount + parity-probe only; the S5-money prod-flip is an explicit operator go
  (reliability-gate §Verdict).
- Not in this batch: `preflight` + track-token on create (deferred S5), courier `GET /orders/:id` read verdict
  (ADR-0013, S6/S7), notification/telegram fan-out (S8), honest-dispatch ENGINE (S7 — the ORDERING is carried,
  the engine is stubbed "no courier" per REV-S5-9 L2).

---

## 2. Back-of-envelope

**Scale (MVP, operator to confirm — no hard telemetry, estimated from the memory corpus / storefront scale).**
- Locations N ≈ 10–50 active; peak ≈ 1–2 orders/min/location at lunch/dinner rush.
- Aggregate peak ≈ **50–100 orders/min ≈ 1–2 orders/s**.
- Money-path writes per order lifecycle ≈ create + ~5–6 transitions (confirm→preparing→ready→in_delivery→
  pickup→deliver) + folds ≈ **~8 write-tx/order** → aggregate peak ≈ **~15 money write-tx/s**. Trivially inside one
  Postgres and a small axum pool. This batch is **not** a scale problem; it is a **correctness/parity** problem.
- Webhook (Plisio) volume is bounded by crypto-prepaid order count (flag OFF today) — effectively 0/s at cutover,
  low even when lit; provider retries are the only burst source and are absorbed by idempotent insert-wins.

**Connection budget (the ADR-required cross-cut: API + worker + analytics + migrations combined).**
The S5 batch **adds no new pool** — the Rust api process already runs (9/10 surfaces flipped) and these 23 routes
mount into the SAME pools. Physical Postgres connections traverse Supavisor (transaction-pool mode). Sizes:

| consumer | pool | max | notes |
|---|---|---|---|
| Rust api operational | `db.rs:105` | **20** | statement cache off (Supavisor tx-mode), hot-path CRUD — S5 routes live here |
| Rust api session | `db.rs:117` | **3** | LISTEN/NOTIFY, advisory locks |
| Node api operational | `packages/db/src/index.ts` | **8** | still serving keep-set until each surface flips |
| Node api session/worker | `packages/db/src/index.ts:51` | **3** | workers/analytics/settlement-cron |
| migrations | node-pg-migrate | 1 (transient) | forward-only, run before boot-guard |

During strangler BOTH api stacks run → worst-case simultaneous physical demand ≈ 20 + 3 + 8 + 3 + 1 ≈ **35**,
well inside Supavisor/Postgres `max_connections`. **The S5 port does not move this number** — it re-routes existing
traffic from the Node-8 pool to the Rust-20 pool. As Node surfaces retire, Node-8 demand falls. Net: budget-neutral.
**Watch item (RESOLVE M1 corrected):** the deliver / assign-courier / pickup / customer-cancel paths take
`SELECT … FOR UPDATE` and hold a physical connection for the tx. Node's `SET LOCAL statement_timeout=4500` is on the
**create** path only (`orders.ts:124`) — the deliver/assign/pickup paths have **no** bound (`dashboard.ts:447-536`). So
the Rust port must set a per-tx `statement_timeout` on these row-lock txs as a **conscious deviation that EXCEEDS Node
parity** (§7a) — a wedged lock self-aborts as a fast 5xx instead of pinning 1 of 20 to exhaustion (§9). Reliability >
byte-parity on an operational bound the client never sees.

**Port cost (route count × cost).** ~8–9 engineer-days, dominated by the two heavy reads, not the many cheap aliases:

| slice | routes | est | driver |
|---|---|---|---|
| order-actions (assign/pickup/deliver/no-show/verify/reveal) | 6 | ~2.5d | 3 reuse `apply_transition`; deliver carries the race fix |
| settlements | 7 | ~1.5d | one small repo, status-guarded UPDATEs + audit log (courier::settlements pattern) |
| refunds | 2 | ~0.5d | DARK; payment_events read + insert-wins |
| reads (dashboard snapshot + customer status + rating) | 3 | ~2.0d | heavy DTOs, PII decrypt+mask, ETA compose |
| messages | 3 | ~1.0d | preset registry + tri-role authz |
| plisio webhook | 1 | ~1.0d | HMAC fail-closed + DEFINER + idempotent |
| race fix + live-PG probes | — | ~1.0d | shared completion primitive + concurrency test |
| promotions Node-keep + fail-loud guard | 1 | ~0.1d | front-door hard-guard + optional 503 mount (no CRUD port; STOP-2) |
| race-fix migrations M-A/M-B (operator-placed) | 2 | — | drafts only; operator places on staging-DB before boot |

**Probe budget.** ~30 distinct `#[ignore]` live-PG cases (shared fixtures) + one node-vs-rust parity harness pass
(status + body shape, 400-vs-422 class) per route = 23 parity checks. Both gate the batch (§9).

**Blast radius.** 100% money/lifecycle. The race fix additionally touches `courier/assignments.rs` — **already LIVE
on staging (S7)** — so the fix's blast radius spans S7, not just this batch. Mitigation: dark-mount, front-door
`NO_AUTO_DEGRADE` for S5, parity probe, per-surface operator prod-flip.

---

## 3. Options (≥2, with the concept each applies)

### Option A — "verbatim-alias port" (RECOMMENDED)
**Concept: strangler-fig + repo-trait outcome-enum port (the proven S2/S3/S5-already pattern).** Port each of the 23
routes 1:1 onto the existing `OrdersRepo` / `Pg*Repo` shape (`orders/mod.rs`): a thin repo method per route returning
an **outcome enum**, mapped to HTTP in the handler; a `Fake*Repo` for handler tests; a `Pg*Repo` porting the real SQL,
exercised by `#[ignore]` live-Postgres probes. Order-actions that are transitions **reuse the PROVEN
`apply_transition` engine verbatim** (assign-courier, deliver, mark-no-show already have the engine — only the
surrounding tx differs). Settlements/refunds/messages get their own small repos following `courier/settlements.rs`.
The race fix is a **surgical change to the shared completion primitive**, not a redesign.

- Tradeoffs: **+** minimal blast radius; **+** boring & proven; **+** schema untouched; **+** every business DECISION
  stays a pure, unit-tested function (money/idempotency/actor-gate/folds); **+** matches the 6 already-ported S5 routes
  (consistency). **−** 23 near-mechanical ports is unglamorous; **−** some duplicated status-guarded-UPDATE shape
  across settlements (accepted: it mirrors Node exactly, which is the parity contract).

### Option B — "settlement-engine consolidation" (REJECTED)
**Concept: unify order + payout + payment state machines into one Rust "money state machine" service** with a single
transition table and one completion primitive, then generate the 23 routes from it.

- Tradeoffs: **+** elegant single source of transition truth long-term. **−** **large blast radius on a red-line
  surface at cutover time**; **−** re-derives proven logic → re-opens the exact `sqlx-cast` bug class (ledger #77) at
  scale, one redeploy at a time; **−** violates monolith-first / boring-proven / "don't redesign the proven engine"
  (packet §R2b explicitly says *bless reuse, not redesign*); **−** delays the strangler cutover the whole tail is
  gated on. Premature consolidation — rejected. (Revisit post-cutover as a refactor with its own ADR if the
  settlement surface grows a real second consumer.)

### Option C (promotions sub-decision only) — RESOLVED to **C1′ (Node-keep + fail-loud)**
**Concept: strangler non-zero keep-set.** Promotions is S3-tagged but pricing-affecting. Sub-options considered:
(C1) **POTEMKIN** — inert Rust stubs, list→`{promotions:[]}`. **WITHDRAWN by RESOLVE (STOP-2):** an affirmative
`{promotions:[]}` *asserts* "zero promotions" to a tenant that has live ones — a silent data-hiding dark-pattern on a
pricing-affecting surface, and a NO_AUTO_DEGRADE violation, the moment anyone mis-flips S3→Rust before the real port.
(C2) **Port-now** — full CRUD+validate port; deferred (its own pricing-composition probe set).
**Decision — C1′ (Node-keep + fail-loud):** promotions stays on the **Node keep-set** (strangler permits a non-zero
keep-set; R4 already accepts it) behind a front-door **hard-guard** that promotions/S3 pricing routes *physically cannot*
be flipped to Rust until the real port. If any Rust mount exists for routing completeness it is **fail-loud**
(`503 PROMOTIONS_NOT_PORTED`), never affirmative-empty — a mis-flip fails **loudly**, not by hiding a tenant's promotions.
Un-lightability is provable (the guard) → STOP-2 lifted.

---

## 4. Decision + rationale (ADR-format → `docs/adr/ADR-s5-money-batch-rust-port.md`)

**Decision (post-RESOLVE R1):** Adopt **Option A (verbatim-alias port)** + **Option C1′ (Node-keep + fail-loud
promotions)**. Reuse `apply_transition` unchanged for all transition-bearing order-actions. Resolve flag A NOT with the
prior draft's "lock-serialize + honor-the-bool" (which RESOLVE proved was a no-op lock + a dead 409 branch — C1/H1/H2)
but with the re-architected **"reorder o→ca + structural write-gating + durable raced-terminal reconcile"** fix (§7).
Two operator-placed red-line migrations (M-A/M-B) are now required (§5). Option B (money-state consolidation) is recorded
as a **planned post-cutover consolidation with a race-count tripwire** (Counsel steel-man): on the 3rd cross-surface
money race, consolidate under its own ADR — Fix is the patch, Option B is the vaccine.

**Rationale.** (1) The 6 already-ported S5 routes prove the pattern holds at byte-parity through the real
router/extractor stack (`orders/mod.rs` handler_tests + the staging L2/L4/L5/L6 trace). (2) The invariant set (integer
money, exactly-once via `request_hash`, status-guarded anti-race UPDATE, RLS FORCE membership-first, PII audit,
fail-closed webhook) is **already encoded** in the Node handlers and the ported engine — the port's job is to preserve
them, not re-invent them. (3) Boring & proven beats a clever consolidation on the one surface where a bug is a charge
defect. (4) The race fix is a business-money decision; the RESOLVE round (R1) ran and **corrected the mechanism** — the fix is
re-architected (reorder + gate + durable reconcile), not the prior patch. What remains is a single operator money-red-line
sign-off (ratify the raced-terminal reconcile rule + place M-A/M-B), never a silent patch. (5) STOP-1 is honored by
making the cash-truth **durable** (in-tx ledger row), not ephemeral; STOP-2 by refusing a lying stub.

---

## 5. Data / migrations

**Forward-only migrations required: TWO (RESOLVE R1 — both red-line, operator-placed, atomic, drafted NOT inlined).**
The route *runtime* is schema-light, but two money-integrity findings force two seams the prior draft missed:
- **M-A (STOP-1 durable cash-truth):** `ALTER TABLE courier_cash_ledger DROP CONSTRAINT <type_check>, ADD CONSTRAINT …
  CHECK (type IN ('hold','release','settle','reconcile'))`. Additive to the CHECK domain, no backfill. Enables the
  same-tx durable `'reconcile'` row on a raced-terminal deliver (§7). Settlement-safe: `app_generate_settlements`
  selects only `courier_assignments.status='delivered'` (mig 078:166,176) → a `'reconcile'` row on a `cancelled`
  assignment is never swept into a payout. **Obligation-sum semantics (RESOLVE R2 OPEN-2, load-bearing) — carry the
  meaning in the migration comment:** `'reconcile'` is an **owner-mediated audit obligation, NOT part of the
  hold/release/settle cash-cycle**; it is closed by owner reconciliation (S6 surface / a future `'reconcile_settle'`
  contra), never by the automatic settlement cycle. Any FUTURE integration that sums the ledger into a courier cash
  obligation (Σholds − Σcontras — none exists today; the ledger is audit-only, mig 028:3-7) **MUST scope the hold-sum to
  `type='hold'` explicitly** (or `type IN ('hold')` / `IN ('release','settle')`), **never `type <> 'x'`** — else
  `'reconcile'` is swept into an *uncloseable* obligation against the courier (the opposite of protecting them). This is
  a guardrail note in the M-A migration + a comment update to mig 028; close-trigger owned by the settlement-worker owner
  when release/settle are wired. Owner: operator (placement) + settlement-worker owner (obligation-sum semantics).
- **M-B (H3 self-poison + M6 scope + N4 spillover) — REDESIGNED (RESOLVE R2 N4).** A **bare** status-guard on the bump
  is REJECTED: it silently underpays. Two failure shapes, both silent (verified against mig 078:178,181,189 and the
  `payout_sums` smoke-check, `smoke-checks.ts:178-182`):
  - *guard-the-bump only* → the settlement_item INSERT (078:181, bound to the immutable payout's id) still lands but the
    guarded bump (078:189) is skipped → `SUM(settlement_items.amount) > courier_payouts.total_earned` → **breaks the
    `total_earned == Σitems` invariant** (smoke-check RED) **and underpays** the courier by that item.
  - *skip the courier/period when the payout ≠ pending* → the late assignment is never inserted; it stays
    `delivered ∧ cash ∧ NOT EXISTS settlement_item` (078:178) → **re-selected forever for the same immutable period,
    never settled** → the courier is never paid for that delivery.

  The current LOUD `prevent_payout_mutation` RAISE (mig 052:8-12) — a noisy DoS on the whole sweep — is **honester than
  either silent underpayment**, so it is the interim posture until M-B has a real destination. **M-B design requirement
  (non-negotiable):** a late in-period earning whose natural-period payout is already `approved`/`paid` MUST be routed
  to a **mutable (`pending`) payout**, never silent-skipped, preserving `total_earned == Σitems` **per payout**.
  Recommended destination — **supplemental payout** (period-honest): relax `courier_payouts`'s
  `UNIQUE(courier_id,location_id,period_start,period_end)` to include a `generation_seq`/`supplemental` discriminator
  (folded into M-B), so a straggler for an immutable period P opens a fresh `pending` payout P′ for the *same* period; the
  item attaches to P′ and bumps P′.total_earned; P is never touched, the invariant holds per payout, the courier IS paid
  in the next cycle, and `settlement_audit_log` records `action='spillover'`. Lighter alternative — **carry-forward** (no
  unique-key change): attribute the straggler to the courier's current `pending` payout (widen the item eligibility lower
  bound + drop the delivered_at→period coupling), audited as `spillover_from_prior_period` — sacrifices period-attribution
  fidelity for zero schema-key change. **Also** in M-B (unchanged from R1): status-guard the bump so P is never mutated,
  and add optional `p_location_id` for per-location scope (M6 cross-tenant fan-out). Until M-B lands, route 13
  (regenerate) is Node-kept / fail-loud (§9) and the cron's immutable-period stragglers stay the LOUD RAISE — never a
  Rust re-trigger of a poisoned sweep, and never a silent-skip patch. Owner: settlement-worker owner (destination design)
  + operator (placement).

Both stage on staging-DB before boot-guard (085–089 precedent). Every other table/function the 23 routes touch already
exists and is operator-placed:

| surface | tables / fns (all pre-existing) |
|---|---|
| order-actions | `orders`, `courier_assignments`, `courier_shifts`, `courier_cash_ledger`, `delivery_trace`, `courier_audit_log`, `order_status_history`, `payment_events` |
| reveal-contact | `customer_contact_reveals` (Node inserts it — `reveal-contact.ts:49`) |
| settlements | `courier_payouts`, `settlement_items`, `settlement_audit_log` |
| refunds | `payments`, `payment_events` |
| messages | `order_messages` |
| webhook | `payments`, `payment_events`, `orders`, DEFINER `payment_location_by_provider_ref` (mig `…083_payments-ledger.ts`) |

This is a **mostly-runtime port with two forced seams** (M-A/M-B above): the read/write seams are cut, but STOP-1's
durability and H3's self-poison cannot be closed in application code alone. **No new tenant-resolver DEFINER is needed**
— the webhook resolver is already placed and operator-owned; **M-B modifies an existing DEFINER** (`app_generate_settlements`)
rather than adding one, and **M-A widens a CHECK domain** (no new function). The prior draft's assertion "the race fix
is code-only" was wrong (RESOLVE): the durable reconcile record has nowhere to live under the existing
`type IN ('hold','release','settle')` CHECK — hence M-A. Both are forward-only, atomic, operator-placed (085–089
precedent, `packages/db/migrations/` = red-line).

**RLS FORCE — name the boundary per route.** Membership-first, asserted BEFORE any location-scoped SELECT:
- **Owner order-actions / settlements / refunds / reveal / verify:** run inside `with_user(owner_user_id)` (seats
  `app.user_id`) OR `with_tenant(location_id)` with the membership-JOIN folded INTO the authorizing query
  (`JOIN memberships m … m.role='owner' AND m.status='active'`, `dashboard.ts:632`, `pg.rs:531`). The JOIN **is** the
  tenant boundary — a bare `WHERE id=$1` leaks cross-tenant under the (still-BYPASSRLS) operational pool. `orders`,
  `courier_payouts`, `payment_events`, `customer_contact_reveals`, `order_messages` all have RLS `ENABLE`+`FORCE`;
  the seated GUC + JOIN is the belt-and-braces (load-bearing once B3 removes BYPASSRLS).
- **Customer reads/rating/cancel:** bound to `customer_id = token.sub` (order-scope), mutation inside
  `with_tenant(location_id)` where `location_id` comes from the ownership-verified read (LC3 GUC dance,
  `customer/orders.ts:324`, `pg.rs:669`).
- **Webhook:** no member exists → DEFINER resolver returns only `location_id` → `set_config('app.current_tenant', …)`
  seats the dual-policy GUC arm (`payments-webhook.ts:41`). Fix-by-port: Node seats the GUC on a bare auto-committed
  statement (can land on a different pooled connection); the Rust port MUST seat it inside the same
  `with_tenant`/`BEGIN…COMMIT` tx as the writes (the `courier/settlements.rs` REV-S7-1 fix pattern).
- **Settlements RLS parity trap (carry from `courier/settlements.rs` module doc):** `settlement_items`'s policy is the
  **throwing** `current_setting(...)` form (no missing-ok arg) — under NOBYPASSRLS a missing seat is a 500, not empty
  rows. Every settlement method MUST run inside a real `with_tenant` tx.

**Money = integer, always — per-column cast table (ledger #77, RESOLVE M4: read-only rule was insufficient; write-side
binds and text-vs-enum columns must be disambiguated).** The prior "reads `::bigint`, binds `::enumtype`" lump invited
both over-casting text+CHECK columns to nonexistent enums AND missing the encode side. Verified column types
(mig 083:56,55,30,20; 045:10; 043:12):

| column(s) | pg type | BIND (write) | READ | note |
|---|---|---|---|---|
| `payment_events.amount_minor` | int4, **NULLABLE** | `i32`/`Option<i32>` (or `$n::int4`) | `Option<i64>` via `::bigint` | webhook **binds** it (`$3`, `payments-webhook.ts:52`) — encode-side #77; must be `Option` |
| `settlement_items.amount`, `courier_payouts.total_earned`, `deliveries_count` | int4 | `i32`/`Option<i32>` | `::bigint`→`i64`/`Lek` | write-side binds on generate/regenerate |
| `orders.total`, `subtotal` | int money | — | `::bigint`→`i64`/`Lek` | read-side only in this batch |
| `payments.status`, `orders.payment_status`, `payment_events.type` | **text + CHECK** | `&str`/`String`, **NO** enum cast | `String`, no cast | over-casting to a nonexistent enum is itself a #77 landmine |
| `orders.status`, `orders.payment_outcome`, `orders.payment_method` | true enum | cast `::order_status`/`::payment_outcome`/`::payment_method` | cast `::text` | live-path convention (`assignments.rs:1089,1175`) |

No `numeric→f64` on a money column, ever. The `#[ignore]` live-PG suite gains **write-side (bind) cases** on
`amount_minor` (int4 + nullable) and the text-vs-enum columns — not just read-side decode — closing the WRITE half of
ledger #77 the read rule missed.

---

## 6. Consistency + idempotency

- **Order transitions:** the status-guarded UPDATE `… WHERE id=$ AND status=$expected RETURNING id` (0 rows → 409
  CONFLICT) is the anti-race primitive — already in `apply_transition` (`pg.rs:785`). Every ported order-action funnels
  through it; **no hand-UPDATE of `orders.status`** (Q-ORDER-FUNNEL).
- **Settlement lifecycle (transitions only):** each transition is a status-guarded UPDATE — `WHERE id=$ AND
  location_id=$ AND status='pending'` (approve), `='approved'` (pay), `IN('pending','approved')` (dispute), `='disputed'`
  (reopen); 0 rows → 409 (`settlements.ts:124/180/224/275`). These are exactly-once by construction; the Rust port
  preserves the predicate verbatim. **Scope correction (RESOLVE H3):** "exactly-once by construction" applies to the
  *transitions*, NOT to settlement *generation*. The generation sweep `app_generate_settlements` (mig 078:160-197) is
  the opposite — it **self-poisons the entire all-tenant sweep**: it bumps `total_earned` with **no status guard**
  (078:189), and `prevent_payout_mutation` (mig 052:8-12) RAISEs on any `total_earned` change to an `approved`/`paid`
  payout, aborting the whole one-tx-over-all-locations function. Normal ops trigger it (owner approves payout P →
  courier delivers one more cash order in P → next cron/regenerate re-selects P, inserts the item, bumps → RAISE → every
  tenant's generation fails). Fixed by **M-B** (§5, status-guard the bump). Until M-B, route 13 is Node-kept/fail-loud
  (§9). A settlement-generation concurrency probe is added to the DoD (approve→deliver-more→generate → assert no RAISE,
  other tenants unaffected).
- **Refund `sent`:** insert-wins idempotent — `ON CONFLICT (provider, provider_payment_id, type) DO NOTHING`
  (`refunds.ts:65`) + residual guard `refunded ≤ captured ≤ amount`. Port keeps the composite conflict target.
- **Rating:** upsert `ON CONFLICT (order_id) DO UPDATE` (exactly-once per order, editable in 24h window,
  `customer/orders.ts:249`).
- **Messages (RESOLVE L2 — the one write WITHOUT an idempotency guard):** `POST .../messages`
  (`order-messages.ts:32`) has no idempotency key — a client network-retry duplicates a message. Not money, not gating.
  Ported verbatim (parity); a client-supplied `Idempotency-Key` (or `(order_id, sender, hash, minute)` dedup) is a
  post-port ticket (§7b carry). Owner: port author.
- **Plisio webhook — replay + monotonicity (ADR-0017 C3):**
  - insert-wins ledger `ON CONFLICT (provider, provider_payment_id, type) DO NOTHING RETURNING id`
    (`payments-webhook.ts:45`) — same-status replays (Plisio resends `txn_id`) no-op; the pending→completed
    progression is admitted (composite unique includes `type`).
  - status flips run **only on a genuinely new event** (`rowcount=1`) and are themselves guarded/monotonic
    (`status NOT IN ('refunded','paid')`, `payment_status IN ('pending','authorized')`).
  - the L-B pay-after-cancel fold records `refund_due` atomically in the same tx when the order is already terminal
    (`payments-webhook.ts:77`) — idempotent via the mig-086 per-payment partial unique; a race with the L-A/L-C
    cancel-side writers resolves to exactly one row.
  - **Monotonicity correction (RESOLVE M5):** the webhook writes are monotonic **per-table, NOT cross-table**. An
    out-of-order `failed → completed` flips `payments.status='paid'` (`WHERE status NOT IN ('refunded','paid')` admits
    `'failed'`, line 60-62) but NOT `orders.payment_status` (`WHERE payment_status IN ('pending','authorized')` skips
    `'failed'`, line 66-68), and refund_due needs `o.status IN ('CANCELLED','REJECTED')` (line 82-83) → **funds in
    limbo** (`payments=paid`, `orders=failed`, no obligation). Crypto is dark → latent. Disposition: the port
    **preserves this verbatim** (parity — widening a dark surface now expands scope) but records it as a conscious CARRY
    (§7b); the real fix (re-drive orders/refund_due from `'failed'` too) is owned by **S8/payments** when crypto lights.
  - The Rust port MUST preserve the exact ack semantics (§7).
- **`request_hash` exactly-once (create — already ported):** unchanged; noted for completeness — the fingerprint is
  the integer-projected canonical (REV-S5-2), dedup is hash-first (`state.rs`), backstopped by the
  `idempotency_keys(key, location_id)` unique (loser 23505 → 409, no orphan).

---

## 7. Failures + degradation (failure-first — every external call: timeout + fallback, zero cascade)

**S5 front-door contract: `NO_AUTO_DEGRADE`.** Money must never degrade silently. A failing Rust S5 route surfaces its
real error status (503 transient / 500 internal / the specific 4xx), and the front-door does **not** substitute a
phantom-success body or silently fall back to Node mid-request. (Contrast: a read-only surface may degrade; a money
mutation may not — a "soft success" on a charge/settlement is a data-integrity incident.)

**Per external call / dependency:**
- **Postgres (every route):** transient PG classes (`40001` serialization, `40P01` deadlock, `57014`
  statement-timeout, `53300`/`08*` conn) → **503 retryable** (`orders.ts:724` `TRANSIENT_PG` set, already mirrored by
  `CreateOutcome::Transient`). Non-transient → 500 INTERNAL with the real cause LOGGED (correlation id), opaque to the
  client. Zero cascade: each route owns one tx; a failure rolls back that tx only (single-order/single-payout blast
  radius, never a batch — ESC-2 posture).
- **Row-lock txs (deliver/assign/pickup/customer-cancel):** bound with a per-tx `statement_timeout` (~4500ms) so a
  wedged `FOR UPDATE` self-aborts as a fast 5xx, never pins a pool slot to exhaustion. **RESOLVE M1 correction:** this
  is NOT "Node parity" — Node's deliver/assign/pickup run `BEGIN` with **no** `statement_timeout` (`dashboard.ts:447-536`;
  the 4500ms is create-only, `orders.ts:124`). So the Rust bound is a **conscious deviation from parity** (§7a),
  justified: pool non-exhaustion > byte-parity on an operational bound the client never observes.
- **Plisio webhook (fail-closed spine):** bad/garbled signature → **401** (never 200-swallow); missing ref → 400;
  crypto off / wrong provider → 404; unknown ref → **200 ack** (stop redelivery, nothing to write); real DB error →
  **500** (let Plisio retry). These exact codes are the money source-of-truth contract — probe every arm.
- **PII decrypt (dashboard/verify/settlements/customer-status):** `decryptPII` failure must degrade to a masked/null
  field, never 500 the whole read and never leak ciphertext (Node returns `null` on absent cipher — port matches).
- **`refund_due` fold (inside a cancel/no-show/deliver transition):** fail-closed **per order** (ESC-2) — the fold
  failing aborts THIS order's transition and propagates (SAVEPOINT `refund_due_fold`, `pg.rs:817`), never silent,
  never a batch. Already in the ported `apply_transition`.
- **`order_status_history` audit + ETA synthesis:** best-effort inside a SAVEPOINT — a failure rolls back to the
  savepoint and never fails the (already-applied) transition (`pg.rs:849`; nuance B, by design).

### Flag A — the courier-deliver race (RE-ARCHITECTED by RESOLVE R1 — the prior draft was misdiagnosed)

**RESOLVE correction (C1 + H1 + H2, source-verified).** The prior draft (Fix-1: "lock-serialize + honor-the-bool")
was **wrong on the mechanism** and its promised remedy was **dead code**:

- **H2 — the lock is already held.** The order row is locked from `assignments.rs:1091` (`courier_assignments ca JOIN
  orders o … FOR UPDATE`, no `OF` → locks `orders` too). The later `SELECT status::text FROM orders WHERE id=$1` (1157)
  reads an **already-locked** row. "Take FOR UPDATE at the top to close the TOCTOU window" is a **no-op** — there is no
  open window. The real footgun is that the assignment terminalize (1127), shift-free (1142), `payment_outcome` (1175),
  `delivery_trace` (1180) and cash 'hold' (1196) run **before** the transition result is known, safe today only by
  *emergent interaction* (JOIN locks o + funnel terminalizes ca + `picked_up` gate), not by local logic.
- **C1 — the promised `409 ORDER_RACED_TERMINAL` cannot fire.** Every `→CANCELLED` funnels through `apply_transition`,
  whose terminalize fold flips the assignment out of `picked_up` in the **same committed tx**
  (`pg.rs:800-813`: `UPDATE courier_assignments SET status='cancelled' … WHERE order_id=$1 AND status IN
  (…,'picked_up')`). So the instant an order is observably CANCELLED its assignment is already `cancelled`, and the
  deliver gate `… WHERE ca.status='picked_up' FOR UPDATE` (1091) returns 0 rows → `DeliveredOutcome::NotFound` →
  **404** at 1097-1099 — long before the bool at 1163-1164. The `bool=false` case is unreachable; the courier who
  physically collected cash gets a bare 404 and **no ledger row at all** — the *mirror* of the phantom (STOP-1).
- **H1 — the live defect is an AB-BA deadlock.** courier deliver locks **ca→o** (1091); `customer_cancel` locks
  **o→ca** (`pg.rs:674`→`803`); owner-proxy deliver locks **o→ca** (`dashboard.ts:470`→`483`). My §7 claim ("the same
  discipline as assign/pickup") was false — courier deliver is the ONE path that locks `ca` first. In the
  pickup→deliver window a concurrent deliver+cancel deadlock → `40P01` → one aborts → mapped to 503.

**Re-architected fix (lands in ONE shared Rust completion primitive both the live courier deliver AND the new
owner-proxy deliver call — the fork is closed for real, not aspirationally). Four moves:**

1. **Global lock-order unification — `orders` before `courier_assignments`, on EVERY path (H1 / RESOLVE R2 N1).**
   **RESOLVE R2 correction (source-verified):** the R1 draft reordered ONLY the deliver primitive. That was wrong on
   two counts. (i) It leaves the AB-BA **un**closed for the OTHER courier paths that lock `ca→o` and can contend with
   the `o→ca` owner/customer paths; (ii) reordering deliver *alone* to `o→ca` **introduces a NEW deadlock pair**
   deliver(`o→ca`) vs cancel/abort/pickup/accept(`ca→o`) that did not exist before (they were all `ca→o`, mutually
   consistent). The concept-ledger's "o→ca everywhere — deadlock-free" was therefore **factually false** after a
   deliver-only reorder. The fix is a **discipline, not a patch**: adopt a single **global resource order — always
   acquire the `orders` row lock before the `courier_assignments` row lock** — and reorder EVERY path that touches both.
   Complete map (source-verified against HEAD):

   | path | source | today | after fix | notes |
   |---|---|---|---|---|
   | courier **accept** (offer + legacy) | `assignments.rs:934,969` → `advance_order_swallow_illegal` | `ca→o` | `o→ca` | locks ca `FOR UPDATE`, then `apply_transition` (o) |
   | courier **pickup** | `assignments.rs:1050-1052` → `advance_order_swallow_illegal:1067` | `ca→o` | `o→ca` | ca `FOR UPDATE`, then advance o to IN_DELIVERY |
   | courier **deliver** | `assignments.rs:1088-1091` (JOIN `FOR UPDATE`) | `ca→o` | `o→ca` | ca is the driving table → ca locked first today |
   | courier **cancel** | `assignments.rs:1226-1231` (`FOR UPDATE OF ca`) → `release_binding_and_reoffer:1244` | `ca→o` | `o→ca` | ca locked, then `apply_transition`/`re_enqueue` (o) |
   | courier **abort** | `assignments.rs:1264-1268` (`FOR UPDATE OF ca`) → `release_binding_and_reoffer:1277` | `ca→o` | `o→ca` | same rail as cancel |
   | **customer_cancel** | `pg.rs:674` (`orders … FOR UPDATE`) → `apply_transition:686` | `o→ca` | `o→ca` ✓ | already correct — the reference order |
   | **owner_order_action** / mark-no-show | `pg.rs:576` (`apply_transition`: UPDATE orders → terminalize ca) | `o→ca` | `o→ca` ✓ | already correct — the reference order |

   Per-path shape (identical to the deliver primitive, applied everywhere): resolve `order_id` from the assignment with
   a *non-locking* read; `SELECT … FROM orders WHERE id=$order_id FOR UPDATE` (order first); then lock the assignment
   with its status-guard. `apply_transition`'s internal UPDATE-orders-then-terminalize-ca is already `o→ca`, so once the
   caller has taken the order lock first the whole path is consistently `o→ca`.
   ```sql
   -- deliver primitive (the same o→ca shape is applied to accept/pickup/cancel/abort)
   -- (1) resolve order_id, NO lock on ca; absent/wrong-courier → genuine 404
   SELECT order_id FROM courier_assignments WHERE id = $1 AND courier_id = $2;
   -- (2) lock the ORDER first (o), read money-authoritative fields
   SELECT status::text, total::bigint, payment_status, payment_method::text
     FROM orders WHERE id = $order_id FOR UPDATE;
   -- (3) lock the ASSIGNMENT (ca) second, status-free (feeds the raced-terminal branch)
   SELECT id, shift_id, status FROM courier_assignments WHERE id = $1 AND courier_id = $2 FOR UPDATE;
   ```
   With the global order applied, no two paths can hold `{ca,o}` in inverted order → **zero AB-BA across the whole
   courier×owner×customer surface**, not just deliver-vs-customer_cancel. The DoD probe (§9) is a **matrix**:
   {accept, pickup, deliver, cancel, abort} × {customer_cancel, owner_order_action} → assert **never `40P01`/`503`** on
   any cell. (accept/pickup are pre-IN_DELIVERY so their contention window with customer_cancel is narrow, but the
   ordering must still be uniform — an inconsistent order is a latent deadlock regardless of current reachability.)
2. **Distinguish raced-terminal from not-found — with a NARROW predicate (C1 + RESOLVE R2 N2).** The R1 predicate
   ("terminal OR ca terminal → raced-terminal") was **too broad**: `DELIVERED` and ca `'delivered'` are *also* terminal,
   so a network retry of a **successful** cash-deliver would fall into the raced-terminal branch and manufacture a
   phantom `'reconcile'` row (the `UNIQUE(order_id,type)` on the ledger does NOT block it — the prior success wrote a
   `'hold'` row, a different `type`), a false owner alert, and a misleading `409 "cancelled while you delivered"` for an
   order the courier genuinely delivered. Today that same retry is a clean idempotent 404 (the `status='picked_up'` gate
   returns 0 rows). The corrected branch order (assignment present, after the o→ca locks):
   - **`ca.status='delivered'` → replay-of-success (idempotency-first).** This is a retry of a prior successful
     completion → return an **idempotent echo** `DeliveredOutcome::Delivered { order_status: <locked order.status> }`
     (200); **write nothing, alert nothing.** (Conscious deviation §7a: parity/today returns 404 here; the echo is
     strictly safer — a retry of a real cash delivery must never fabricate a reconcile obligation against the courier,
     and 404 could push the courier UI to re-attempt. Observable-idempotency, same posture as the webhook §6 insert-wins.)
   - **`order.status ∈ {CANCELLED, REJECTED}` OR `ca.status ∈ {cancelled, rejected}` (a NON-delivered terminal) →
     raced-terminal** (move 3). This is the only branch that emits a reconcile obligation.
   - **`order.status=IN_DELIVERY` AND `ca.status='picked_up'` → normal completion** (byte-identical happy path).
   - **else** (assignment absent / wrong courier / never-picked-up) → **404** (parity with today).
   The predicate keys on *specific* terminal states, never a blanket "terminal", so DELIVERED-by-self can never be
   misread as a race.
3. **Durable cash-truth on raced-terminal (STOP-1) — observably idempotent (RESOLVE R2 N5).** When the courier
   reported cash (`outcome.is_paid_full()` with a concrete `cash_amount`) and the order raced a NON-delivered terminal:
   - write **one durable row in the same tx** — `courier_cash_ledger` of the **new type `'reconcile'`** (M-A), and
     **gate the side-effects on the insert actually being new** (mirrors the webhook `rowcount=1` discipline, §6):
     ```sql
     INSERT INTO courier_cash_ledger (courier_id, location_id, order_id, type, amount)
       VALUES ($1,$2,$3,'reconcile',$4) ON CONFLICT (order_id, type) DO NOTHING RETURNING id;
     -- new row → RETURNING yields the reconcileId; publish the alert; return 409{amount, reconcileId}
     -- conflict (retry) → 0 rows → SELECT id FROM courier_cash_ledger WHERE order_id=$3 AND type='reconcile'
     --                    → return 409{amount, <existing reconcileId>}, publish NO second alert (idempotent echo)
     ```
     So the owner alert fires **exactly once** (only on `rowcount=1`) and the courier's retried `409` carries the **same
     `reconcileId`** — never `None`, never a duplicate alert. Settlement-safe (generation selects only
     `status='delivered'` assignments → a `'reconcile'` row on a `cancelled` assignment is never swept — §5 M-A).
   - **owner alert** (§21 alert-friction, PII-free / claim-check): publish `COURIER_CASH_RECONCILE_DUE
     {orderId, courierId, locationId, amount, reason:'raced_terminal'}` **post-commit, only when the insert was new** —
     durability lives in the ledger row, the alert is best-effort (a lost alert never erases the truth; this is exactly
     why ephemeral-409-only failed).
   - return a **distinct `409 ORDER_RACED_TERMINAL`** carrying `{amount, reconcileId}` → the courier UI renders a
     **human instruction** ("cancelled while you delivered; you collected X — [return] / [hand in for reconcile]"), not
     a red error (Counsel dignity requirement; coupled FE work, owner FE/S6). No paid_full trace, no
     `payment_outcome='paid_full'`, no assignment→`delivered`. Order stays CANCELLED + `refund_due` **if a paid payment
     exists** (crypto — customer protected); on a raced CASH order there is no `refund_due` (no `paid` payments row —
     the fold at `pg.rs:817-826` is inert for cash), so the `'reconcile'` row is the **sole money-conservation record**
     of the cash the courier physically holds (courier protected). Narrative is **true**, neither false-delivered nor
     erased.
   - **"Owner-visible" is HONESTLY SCOPED (RESOLVE R2 N3 / Counsel OPEN-1).** Source check: `courier_cash_ledger` is an
     **audit-only** trail (mig 028:3-7 — `'release'`/`'settle'` are reserved-but-never-written, and *nothing reads or
     sums it*; owner cash figures come from `courier_assignments.cash_amount` + `settlement_items`), and the alert
     transport is a **no-op seam until S6** (`pg.rs:876`). So at cutover the reconcile truth is **durable + auditable +
     courier-instructed-live (the 409 body)**, but NOT yet **owner-proactively-surfaced**. This is stated honestly, not
     claimed away: the owner-facing surface (real alert transport + an owner **reconcile-queue read** of
     `type='reconcile'` rows with no close row) is a **gated S6 / owner-FE deliverable that MUST land before the
     S5-money prod-flip is declared STOP-1-complete** (§9, R16). Shipping the durable row NOW is still strictly better
     than the R1-inherited erasure (a bare 404 with no record) — we ship the truth and gate the surfacing, we do not
     block the truth on the surface.
4. **Structural write-gating (H2).** All post-transition writes (terminalize, shift-free, `payment_outcome`,
   `delivery_trace`, cash 'hold') live **inside the happy branch**, entered only after the locked order is confirmed
   `IN_DELIVERY` and `ca` is `picked_up`. `apply_transition`'s returned bool becomes a **defense-in-depth assert**
   (under the lock it must be `true`; `false` ⇒ impossible interleave ⇒ abort as a logged 500, never proceed) — the
   opposite of today's discard at 1164.

`DeliveredOutcome` gains a `RacedTerminal { collected_cash: Option<i64>, reconcile_id: Option<Uuid> }` variant mapped
to 409 in the handler (`assignments.rs:653-670` pattern). Both the live courier deliver and the owner-proxy deliver
(which already 409s on non-IN_DELIVERY at `dashboard.ts:474-477` but writes NO reconcile today) route through this one
primitive → both gain the durable reconcile.

**Reachability (does the reconcile write re-open H1?)** No — in the raced case the cancel tx has committed and released
its locks, so the deliver tx acquires the order lock uncontended and writes the `'reconcile'` row under the order lock
in o→ca order; the ledger insert does not invert the row-lock graph. No new AB-BA.

**Recorded money decision (operator sign-off, STOP-1):** ratify "whoever commits order-status first wins; the loser's
physical cash becomes a durable, auditable reconcile obligation" as the money rule, place M-A, **and ratify the
two-phase surfacing (RESOLVE R2 N3): the durable ledger row + live courier 409 ship in this batch; the owner-proactive
surface (alert transport + reconcile-queue read) is a gated S6/owner-FE follow-up that gates the STOP-1-complete
declaration of the S5-money prod-flip.** This is the one human money-red-line sign-off the round routes to a person.

**Fix-2 (compensating-record only) — still rejected**, and now doubly so: it fabricates a `delivery_trace` on a
cancelled order AND leans on a compensating write instead of preventing the incoherent state.

---

### 7a. Conscious DEVIATIONS from byte-parity (RESOLVE — security/reliability > byte-parity; each is deliberate, not a probe failure)

Parity preserves known behavior — including latent weaknesses. These four places the port **intentionally diverges**
from Node; each parity probe asserts the *corrected* shape, documented as an exception:

| # | Node behavior (parity would preserve) | Rust deviation | justification | source |
|---|---|---|---|---|
| M1 | deliver/assign/pickup: no `statement_timeout` | per-tx `statement_timeout` ~4500ms on row-lock money txs | pool non-exhaustion (20-conn) > byte-parity | `dashboard.ts:447-536` |
| M2 | `GET /settlements`: `catch → {payouts:[]}` (silent degrade) | query failure surfaces real 503/500, correlation-id logged; "0 rows" ≠ "read failed" | NO_AUTO_DEGRADE for money reads (couriers-unpaid-with-green-health) > byte-parity | `settlements.ts:69-71` |
| M3 | `GET /settlements/:id`: `SELECT p.*, full_name_encrypted` → ciphertext + `approved_by_owner_id` in body | typed explicit-column SELECT; decrypt+mask name to `charAt(0)+'***'`; NEVER emit ciphertext or internal ids | PII data-minimization / no-ciphertext-egress > byte-parity; also kills body-drift on new columns | `settlements.ts:83,106` |
| M6 | `regenerate`: `referenceDate: z.string()` (accepts garbage → `Invalid Date`) | `z.string().datetime()`, reject non-ISO → 400 at the edge, never propagate `Invalid Date` into period math | validate-at-edge | `settlements.ts:304,314` |
| N2 | courier deliver retry on an already-`delivered` assignment → 404 (`status='picked_up'` gate 0 rows) | `ca.status='delivered'` → **idempotent 200 echo** (no write/alert) | a retry of a genuinely-successful cash delivery must never fabricate a reconcile obligation nor push the UI to re-attempt; observable-idempotency > byte-parity (404) | `assignments.rs:1091` |

### 7b. Conscious CARRIES — latent weaknesses preserved verbatim (parity), with a named owner + close-trigger (NOT fixed in this parity port)

Distinguishing "preserve an invariant" (mandatory) from "preserve an incidental weakness" (separate ticket) — the
explicit list the epistemic note demanded:

| # | carried weakness | why not now | owner / close-trigger |
|---|---|---|---|
| M5 | webhook out-of-order `failed→completed` diverges `payments=paid` / `orders=failed` → funds in limbo | crypto dark; widening a dark surface expands scope; probe asserts today's behavior so the port doesn't accidentally re-diverge | S8/payments — fix (re-drive orders/refund_due from `'failed'`); **RESOLVE R2 forward-gate (Counsel): the S8 M5-fix is a HARD GATE on lighting crypto — `CRYPTO_ENABLED`/`PAYMENTS_PREPAID` must NOT flip on until M5 is fixed + probed, else a real webhook race strands funds in limbo** |
| L2 | `POST .../messages` has no idempotency key → client-retry duplicates | not money, not gating; parity | port author — post-port `Idempotency-Key`/dedup ticket |
| L3 | reveal-contact preventive control is only `rateLimit:10/min` (≈600 PII reveals/hr); revoked ≤24h owner token can bulk-harvest | audit-before-return is already correct (R9 = invariant to preserve, not risk to fix); tightening `reason`/rate is a behavior change vs parity | security — post-port ticket (mandatory `reason` + harvest-anomaly alert + tighter rate) |
| L1 | owner-proxy deliver enum omits `delivered_prepaid` (4 vs courier 5) → crypto-prepaid can't be owner-proxy-delivered | crypto dark; match narrower owner edge for parity | port author — revisit with M5/S8 |

R9 is **downgraded** from "risk to fix" to "invariant to preserve" (Counsel epistemic note): the port must keep the
`customer_contact_reveals` INSERT inside the tenant tx, committed before plaintext is returned (`reveal-contact.ts:33-55,69-74`),
and widen nothing.

## 8. Security + tenant isolation

- **Auth:** JWT RS256-only, zero cookies (inherited from the Rust extractor stack). Owner routes narrow via
  `OwnerClaimsExt` (a courier/customer token cannot reach an owner handler — `orders/mod.rs`
  `owner_status_with_courier_token_is_401`). Customer routes via `CustomerClaimsExt` + `require_order` (token's
  `orderId` claim must equal the path — closes the S2 T-12 cross-order bug, proven at HTTP).
- **Tenant isolation:** membership-JOIN / seated-GUC per §5. **Assert active-owner membership BEFORE any
  location-scoped SELECT** — folded into the authorizing query, not a trust of the baked `activeLocationId` (a
  removed/downgraded owner holds a valid ≤24h token — ADR-0004).
- **PII (reveal-customer-contact) — audit trail is load-bearing:** the reveal writes
  `customer_contact_reveals(order_id, customer_id, location_id, revealed_by_owner_id, reason)` in the SAME tenant tx
  BEFORE returning the plaintext name/phone (`reveal-contact.ts:49`). The Rust port MUST write the audit row inside
  the `with_user`/`with_tenant` tx and return `{orderId, customerId, name, phone}` only after it lands (never
  reveal-then-audit; a failed audit must NOT leak the contact). The bus event stays PII-free (`{orderId, revealedAt}`).
- **PII masking parity (dashboard/verify/customer-status/settlements-LIST):** port Node's EXACT decrypt/mask behavior,
  widen NOTHING (R2a PII-parity rule). `maskName`/`maskPhone` on customer fields; courier name → first-char+`***`,
  phone → last-4; courier messenger exposed only while the order is non-terminal (`courier/orders.ts:89`). **Exception
  (RESOLVE M3, §7a):** `GET /settlements/:id` is the one place the port **narrows below Node** — Node ships
  `full_name_encrypted` ciphertext + `approved_by_owner_id` via `SELECT p.*` (`settlements.ts:83,106`); the Rust port
  uses a typed explicit-column SELECT, decrypts+masks the name (parity with the list route), and emits **no ciphertext
  and no internal ids**. "Widen nothing" and "leak-nothing" both hold; the deliberate divergence is documented, and the
  parity probe asserts the ciphertext-absent shape.
- **Webhook:** fail-closed HMAC (§7); DEFINER resolver is the only tenant resolution without a member; zero secrets in
  code (provider key from env/secret store).
- **No PII to AI / no PII on the bus:** unchanged (claim-check — the bus carries only non-PII status/ids;
  `orders.ts:625`).
- **Money authority is server-side:** client never sets `total`/`status`/`amount`; settlement amounts are read from
  rows some other surface wrote (REV-S7-3), never recomputed here.

---

## 9. Operability

- **Dark-mount + flag/scaling-gate:** mount all 23 routes dark (main.rs pattern), front-door keeps them Node-served
  until each is parity-proven; per-surface prod-flip is an explicit operator go (S5-money is on the reliability-gate
  NOT-YET-GO list).
- **Parity probes (the batch gate):** for each route, a node-vs-rust probe asserting **status code AND body shape**,
  with explicit **400-vs-422 class** coverage (ledger #78: Node `sendError(400,'VALIDATION_FAILED')` →
  `validation_failed_400`, never a 422 default; the settlement 409s; the webhook 401/400/404/200/500 arms; refunds
  404-when-off vs `{refunds:[]}`-when-off asymmetry).
- **`#[ignore]` live-PG cargo suite (ledger #77 guard):** every route gets a real-Postgres probe exercising the
  sqlx bind/decode boundary — enum casts on BOTH bind and read, text+CHECK columns bound as text (NOT enum), and the
  **write-side** int4/nullable money binds (RESOLVE M4: `amount_minor` bind, not just read). This is the one systemic
  ratchet (reliability-gate §"the one systemic ratchet"): wire it into CI so the whole psql-literal-passes /
  sqlx-bind-fails family is caught at build time, not one redeploy at a time.
- **Concurrency probes (RESOLVE — assert a SPECIFIC status per arm, not "not both"; the prior "not both" let a
  `40P01→503` pass as green):**
  - *deadlock MATRIX (RESOLVE R2 N1 — not just deliver-vs-customer_cancel):* on a shared order, run each courier path
    that locks both rows **{accept, pickup, deliver, cancel, abort}** concurrently against each o→ca path
    **{customer_cancel, owner_order_action}** → assert **never `40P01`/`503`** on any cell (global o→ca ordering closes
    every AB-BA). A `503`/`40P01` on any cell is a **RED regression**, not an accepted arm. The prior deliver-only
    reorder would go RED on the deliver×{cancel,abort} cells (the newly-introduced pair) — that is the exact regression
    this matrix catches.
  - *deliver vs customer_cancel* on one IN_DELIVERY cash order (the money arm): **deliver-wins** → `200` +
    `courier_cash_ledger 'hold'`, order DELIVERED; **cancel-wins** → deliver gets **`409 ORDER_RACED_TERMINAL`** + a
    `'reconcile'` ledger row (CASH → **no** `refund_due`, `pg.rs:817` inert for cash), order CANCELLED. Assert **never
    `503`/`40P01`** and **never both** money outcomes.
  - *raced-terminal replay idempotency (RESOLVE R2 N2 + N5):* (a) **successful-deliver retry** — deliver a cash order to
    completion, then re-POST the same deliver → assert an **idempotent 200 echo**, **no new `'reconcile'` row**, ledger
    row-count for the order **unchanged**, **no owner alert** (the broad R1 predicate would RED here — phantom reconcile
    + false alert). (b) **raced-terminal retry** — after a cancel-wins race, re-POST deliver → assert the **same
    `reconcileId`** in the 409 body (not `None`), **row-count still 1**, and **exactly one** `COURIER_CASH_RECONCILE_DUE`
    alert total across both attempts (alert gated on `rowcount=1`).
  - *settlement generation self-poison + spillover (H3 + RESOLVE R2 N4):* approve courier X's period-P payout → X
    delivers one more cash order in P → run `generate`/`regenerate` → assert **no `payout immutable` RAISE**, other
    tenants' payouts still generate, **and the late delivery is EVENTUALLY settled** (a `settlement_item` lands on a
    `pending` payout — supplemental P′ or carry-forward — the courier is paid), with `payout_sums`
    (`total_earned == Σitems`) **still green per payout**. A variant that leaves the delivery unsettled, or breaks
    `payout_sums`, is RED — a silent-skip patch must not pass. Red today, green after M-B.
- **STOP-1 owner-visible reconcile surface — a prod-flip gate (RESOLVE R2 N3 / Counsel OPEN-1):** the durable
  `'reconcile'` row + live courier 409 ship in this batch, but the ledger is audit-only (mig 028:3-7) and the alert is a
  no-op seam (`pg.rs:876`) — nothing surfaces the obligation to the owner. **Before the S5-money prod-flip is declared
  STOP-1-complete**, the owner-proactive surface MUST land: real `COURIER_CASH_RECONCILE_DUE` transport (S6) + an owner
  **reconcile-queue read** (`courier_cash_ledger WHERE type='reconcile'` with no close row). Until then the operator can
  query reconcile rows directly (they are durable), and the raced-terminal branch may ship dark/gated with the durable
  row landing — but the "owner-visible" claim is NOT asserted. Owner: S6 / owner-FE + operator (R16).
- **Health degraded-vs-down:** S5 routes report degraded (dependency slow) distinctly from down (pool exhausted);
  a money route returning 503 is "retryable," a 500 is "paged." Observability <1 min: every S5 500 logs the real
  cause + correlation id (`orders/mod.rs:347` posture) — a money 500 with no logged cause is an operability hole.
- **Rollback:** dark code is reversible by front-door flag flip (no deploy). The race fix touches LIVE
  `assignments.rs` (S7) — ship it behind its own guard or as a same-behavior-on-no-race change so a rollback is a
  front-door reroute, and re-run the reliability-gate L7 deliver trace green before/after.
- **Regenerate (route 13) — RESOLVE H3/M6.** It triggers `app_generate_settlements` (mig 078), which **self-poisons
  the all-tenant sweep** (H3) and **ignores `:locationId`** / accepts an unvalidated `referenceDate` (M6). Dispositions:
  (a) the root fix is **M-B** (status-guard the DEFINER bump + add `p_location_id` scope) — operator-placed, red-line;
  (b) **until M-B lands**, route 13 is **Node-kept / fail-loud** (`503 SETTLEMENT_GEN_NOT_PORTED`), never a Rust
  re-trigger of a poisoned sweep; (c) if/when exposed on Rust, it enqueues a `jobs/runner.rs` job (thin trigger) and
  **never inlines settlement math** (Counsel advice #4); (d) the `referenceDate` edge validation (`z.string().datetime()`
  → 400) is fixed in the port now (no migration, §7a M6). Do not claim "exactly-once by construction" for generation
  (§6 scope correction).

---

## 10. Open / accepted risks (justification + owner)

| # | risk | class | disposition | owner |
|---|---|---|---|---|
| R1 | Flag-A fix touches LIVE S7 `assignments.rs` — a regression strands couriers. **RESOLVE re-architected the fix** (reorder o→ca + structural gating + durable reconcile; the prior "lock-serialize + honor-bool" was a no-op lock + dead 409). **RESOLVE R2 (N1): the reorder must be GLOBAL** — accept/pickup/deliver/cancel/abort ALL lock ca→o today; a deliver-only reorder leaves AB-BA for the rest AND introduces a new deliver-vs-cancel/abort deadlock | red-line | **RESOLVE-gate**: council ratifies; global o→ca on all 5 courier paths (orders-lock-first discipline); **deadlock-MATRIX probe** {accept,pickup,deliver,cancel,abort}×{customer_cancel,owner_order_action} never 503/40P01 red→green; per-arm money probe (200+hold / 409+reconcile, never both); L7 re-trace green pre/post; **depends on M-A** | council + operator |
| R2 | S5 prod-flip is money — a parity miss charges/mis-settles a real customer | red-line | **accept-with-gate**: dark-mount + NO_AUTO_DEGRADE + per-route parity probe + explicit operator prod-flip | operator |
| R3 | Regenerate/cron **self-poisons the all-tenant settlement sweep** (H3) + inlining gen-math would fork money truth. **RESOLVE R2 (N4): a bare status-guard on the bump silently underpays** (breaks `total_earned==Σitems` or never-settles) | red-line | **defer-flag → M-B (redesigned)**: status-guard the bump + per-location scope + a **spillover destination** (supplemental payout OR carry-forward) so late in-period earnings reach a mutable payout — **never silent-skip; the loud RAISE is the interim, honester than silent underpayment**; route 13 Node-kept/fail-loud until then; never inline the math | settlement-worker owner + operator |
| R4 | Promotions Node-kept (non-zero keep-set) behind a fail-loud front-door guard | accepted | **accept (C1′)**: strangler permits a non-zero keep-set; NO `{promotions:[]}` stub (STOP-2); discount path is `discountTotal=0` today (REV-S5-6); port later with its own pricing probes | System Architect |
| R5 | Webhook GUC-seat-on-bare-statement parity trap (Node seats on auto-commit) | correctness | **fix-by-port**: seat inside the same `with_tenant`/BEGIN tx as the writes (REV-S7-1 pattern) — do NOT copy Node's bare seat | port author |
| R6 | Settlement `settlement_items` throwing-RLS policy → 500 (not empty) under NOBYPASSRLS if seat missing | correctness | **fix-by-port**: every settlement method inside a real `with_tenant` tx | port author |
| R7 | Row-lock txs (deliver/assign) pin a pool slot if wedged | operability | **fix-by-port (conscious DEVIATION §7a)**: per-tx `statement_timeout` ~4500ms — Node has NONE on these paths (M1), so this EXCEEDS parity; reliability > byte-parity | port author |
| R8 | Scale numbers are estimates (no hard telemetry) | assumption | **accept**: even 10× the estimate (~15 write-tx/s → 150) is inside one Postgres + a 20-conn pool; revisit only if telemetry contradicts | System Architect |
| R9 | Reveal-contact audit ordering. **RESOLVE downgraded (L3)**: audit-before-return is ALREADY correct in Node — this is an invariant to PRESERVE, not a risk to fix | invariant | **fix→preserve**: keep the `customer_contact_reveals` INSERT in-tx before plaintext; widen nothing | port author |
| R10 | Two red-line migrations (M-A `courier_cash_ledger 'reconcile'`; M-B `app_generate_settlements` guard) must be operator-placed; absent → reconcile path + regenerate ship half-working | red-line (schema) | **gate**: raced-reconcile branch and route 13 stay fail-loud/Node-kept until M-A/M-B land on staging-DB (boot-guard); drafts in `resolution.md` §migrations | operator |
| R11 | Webhook out-of-order `failed→completed` → funds in limbo (M5); ported verbatim | correctness (latent, crypto dark) | **carry (§7b)**: preserve behavior + probe asserts it; real fix owned by S8 when crypto lights | S8 / payments |
| R12 | Raced-terminal `409` must render as a human instruction, not a red error (courier dignity; Counsel) | UX (dignity) | **coupled requirement**: courier UI action-state ("cancelled while you delivered; you collected X — [return]/[hand in]") | FE / S6 |
| R13 | DB adjudicates a physical-world dispute it did not witness (Counsel open question §5) | strategic / ethical | **HUMAN (no batch change)**: reconcile obligation stops the DB lying; a dignified long-run design lets both parties jointly attest — framing for product, not a requirement of this port | product |
| R14 | reveal-contact preventive control is weak (harvest rate; L3) | security (post-port) | **defer-flag**: post-port ticket — mandatory `reason` + harvest-anomaly alert + tighter rate; NOT in this parity port | security |
| R15 | `messages` write has no idempotency key (L2) | correctness (non-money) | **defer-flag**: post-port `Idempotency-Key`/dedup; parity for now | port author |
| R16 | STOP-1 "owner-visible" is aspirational at cutover — the ledger is audit-only (mig 028:3-7) + alert is a no-op seam (`pg.rs:876`); nothing surfaces the reconcile obligation to the owner (RESOLVE R2 N3 / Counsel OPEN-1) | red-line (money-visibility) | **gate**: durable row + live courier 409 ship now; owner surface (alert transport + reconcile-queue read) is a gated S6/owner-FE deliverable that MUST land before S5-money prod-flip is declared STOP-1-complete; STOP-1 language honestly downgraded to "durable + auditable + courier-instructed-live" | S6 / owner-FE + operator |
| R17 | `'reconcile'` swept into a future courier obligation-sum → an *uncloseable* debit against the courier (RESOLVE R2 OPEN-2) | correctness (forward) | **fix-by-comment (M-A) + defer-flag**: `'reconcile'` is owner-mediated audit, EXCLUDED from any Σhold; the future settlement integration must scope hold-sum to `type='hold'`, never `type<>x`; no such sum exists today (audit-only) | settlement-worker owner |
| R18 | Raced-terminal predicate breadth — treating DELIVERED-by-self as a race would fabricate a phantom reconcile + false alert on a successful-deliver retry (RESOLVE R2 N2) | red-line (money) | **fix**: narrow predicate to NON-delivered terminals only; `ca.status='delivered'` → idempotent 200 echo (no write/alert); replay-idempotency probe red→green | port author |

---

### Concept ledger (named — post-RESOLVE R1+R2)
strangler-fig · repo-trait outcome-enum port · monolith-first / boring-proven · status-guarded UPDATE (optimistic
anti-race) · **global resource ordering — always lock `orders` before `courier_assignments`, on EVERY path
(accept/pickup/deliver/cancel/abort + customer/owner); a PARTIAL reorder is worse than none — it both leaves and creates
AB-BA (R2 N1 corrects "o→ca everywhere" from an aspiration to a proven discipline)** · pessimistic `FOR UPDATE`
(serialize, NOT "close an already-closed window") · **raced-terminal distinct outcome, NARROW predicate (404 not-found
vs 409 raced-terminal vs 200 idempotent delivered-replay echo — specific terminals, never blanket-terminal; DELIVERED-
by-self is a replay, not a race — R2 N2)** · **durable cash-truth / reconcile-obligation (in-tx ledger row is authority;
alert is best-effort)** · **observable idempotency (side-effects gated on `rowcount=1`; replay echoes existing id, never
re-alerts — webhook §6 posture applied to reconcile — R2 N5)** · **audit-only ledger vs cash-cycle ledger (`'reconcile'`
is owner-mediated audit, excluded from any Σhold obligation-sum; scope hold-sum to `type='hold'`, never `type<>x` — R2
OPEN-2)** · **honest-scoping of "owner-visible" (durable+auditable now, owner-proactive surface gated to S6 — don't claim
a surface that doesn't exist — R2 N3)** · **structural write-gating (branch on locked status BEFORE writes, not
capture-bool-after)** · idempotency insert-wins / hash-first · SAVEPOINT best-effort fold · fail-closed webhook (HMAC +
DEFINER tenant resolve) · **status-guarded DEFINER bump + spillover destination (settlement-gen self-poison fix that does
NOT silently underpay — late earning → mutable payout, loud-RAISE > silent-skip — R2 N4)** · RLS FORCE membership-first ·
claim-check (PII off the bus) · NO_AUTO_DEGRADE money front-door · **fail-loud > affirmative-empty (no lying stub —
STOP-2)** · **conscious parity-DEVIATION vs conscious CARRY (security/reliability > byte-parity, explicitly ledgered) +
forward-gate (crypto launch gated on the S8 M5-fix — R2)** · dark-mount + flag scaling-gate · integer money (per-column
cast table: int4 bind/read, text+CHECK no-cast, enum cast both sides) · the-one-systemic-ratchet (`#[ignore]` live-PG
suite in CI, now with write-side binds) · **race-count tripwire for Option B consolidation (patch now, vaccine on 3rd
cross-surface race)**.
