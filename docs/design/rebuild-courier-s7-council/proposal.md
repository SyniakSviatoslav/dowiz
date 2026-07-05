# S7-COURIER/DISPATCH Port — Council Packet · PROPOSAL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Description input to the live Triadic Council
> (system-architect + system-breaker + counsel + human). **No S7 code is ported to Rust until this
> packet is council-APPROVED, every quirk-register row (§10) is dispositioned one by one, and the
> operator signs the 🔴 open questions (`open-questions.md`).** This is a **red-line surface** — it
> composes **courier auth + JWT minting** (the S2-parity obligation *outside* S2's gate), the
> **dispatch/assignment state machine**, and **courier money** (cash-as-proof + settlement/payout reads
> that share the S5 settlement ledger, and the migration-085 double-pay landmine). Docs only; no product code.

- **Lane:** R3 (complete-rebuild) · **Surface:** S7 courier/dispatch (REBUILD-MAP Phase B; the courier
  strangler that rides *after* S5 orders/money + S6 realtime). **38 route rows** per
  `docs/design/rebuild-cutover-harness/route-surface-map.generated.md` (S7 census row `line 50`).
- **Date:** 2026-07-04 · **Source commit:** `fix/audit-remediation` (working tree).
- **Census SSOT:** `inventory/10-api-realtime-jobs.md` (courier routes) + the route-surface-map S7 rows
  (75–79, 96–98, 114–140, 172, 179). The S7 surface spans **five files** on the courier side
  (`courier/{auth,shifts,assignments,me,settlements}.ts`) **plus** owner-side courier management
  (`owner/{couriers,courier-invites}.ts`) **plus** the honest-dispatch engine (`lib/dispatch.ts`) and the
  completion/binding/shift services (`lib/{deliveryCompletion,bindingRelease,shiftService,courierAssignmentService}.ts`).
- **The route-surface-map surprises this packet must resolve (all confirmed against source):**
  1. **`courier/auth.ts` mints + refreshes JWTs but is path-owned S7** (rows 134–138) — it carries S2's
     **body-kid RS256 JWT-parity obligation** and the courier **session-rotation** state machine, yet sits
     *outside* the S2 council's gate (§3, Q1 🔴).
  2. **Courier settlements (S7, rows 139/140) and owner settlements (S5, rows 23–29) read the SAME
     `courier_payouts`/`settlement_items` rows** — money is **not one atomic surface**; the write
     authority (approve→pay) is S5, the courier read is S7 (§5, Q3 🔴).
  3. **`owner/couriers.ts` `GET …/orders/:orderId/route` (row 78) is S7** — the textbook CRIT-1 infix: same
     `/orders/:orderId/` prefix as every S5 order-action route, but the trailing literal `route` makes it
     S7 (courier-movement data), not S5 (§4/§8).
  4. **`courier/me.ts:110` password-change is an AUTH-class op** (full session revoke) path-owned S7 (row
     126); **`owner/…/orders/:orderId/assign-courier`** (dashboard.ts:215, S5 row 40) is a **dispatch
     effect through an S5 path**. Neither is a clean surface boundary.
- **Governing ADRs / prior councils (this surface inherits hard-won invariants — do not re-litigate):**
  - **ADR-0013** (courier WS/read authz — `courierReadVerdict`, live-binding scope) · **ADR-0003**
    (dev-login gate — the synthetic-courier / mock-auth fail-closed-on-prod boundary) · **ADR-0004**
    (owner-token P-d live-membership re-read — the *pattern* the courier session-liveness mirrors).
  - **ADR-deliver-v2-cash-as-proof** (`docs/design/…`) — the single completion primitive, the cash-as-proof
    HOLD, the no-partial-handover rule, the honest no-cash CANCELLED tail (memory
    [[deliver-v2-cash-as-proof-2026-06-28]]).
  - **ADR-audit-fix-money** — **M-2** settlement no-loss redesign + migration draft **085** (watermark
    `2026-07-10` HARD gate, the double-pay landmine); the L-A `refund_due` fold now lives in
    `updateOrderStatus` (S5), *not* in `completeDelivery` (§6).
  - **S2-auth RESOLVE** (`rebuild-auth-s2-council/`) — the **body-kid RS256** minter/verifier the Rust port
    already built; S7 courier auth **reuses** it, never re-implements it (§3).
  - **S5-orders RESOLVE** (`rebuild-orders-s5-council/`) — the `order_status.rs` matrix + the
    `updateOrderStatus` central mutator (with the R2-3 assignment-terminalize fold + the L-A refund fold);
    every S7 order-side transition **funnels through it** (§4).
  - **S6-realtime RESOLVE** — **REV-S6-2** (per-frame courier `courier_sessions` session-liveness at the WS
    tail — the SAME `courierSessionValid` REST runs) and **REV-S6-5** (flip in a **low-delivery window** +
    **gradual drain**, the signed cutover canon S7's flip inherits) (§9).
  - **S3-catalog RESOLVE REV-10** (non-confusable `UserId`/`TenantId` types) and **REV-7** (per-surface
    **atomic** cutover; the route family flips together).
- **Parity oracle:** the Playwright net (courier-facing slices: courier-login/shift, active-delivery /
  accept-pickup-deliver, cash-as-proof, cross-tenant-realtime courier arms, settlements/payouts read) **plus**
  the delivery/dispatch invariant-cluster: the assignment state-machine transitions, the cash-as-proof
  `cash===total` assert, the honest-dispatch no-orphan rule, and the settlement idempotency (which is proven
  at the **DB-function** layer, `apps/api/tests/settlements-catchup.test.ts`, because the money math is in
  Postgres, not the app). No behavior change is real without a red→green test (Mandatory Proof Rule).
  Cutover DoD in §11.

---

## 1. Port objective and the three load-bearing seams

S1 was read-only; S2 wrote **auth**; S3 wrote **owner catalog** (`with_user`); S4 wrote **media**; S5
composes **money + the order state machine**; S6 is the **WS transport**. **S7 is the courier operational
plane** — the surface where a *lower-trust* principal (the courier) drives the physical fulfilment of a paid
order, collects cash, and reads its own pay. A port defect here is a **cross-courier hijack**, a **logged-out
courier with a 14-day live tail**, a **double-paid settlement**, or a **"delivered" order whose food was
refused**.

There are **three** load-bearing seams, each an independent failure mode the port must hold simultaneously:

1. **The courier-auth/session seam (Q1 🔴).** `courier/auth.ts` mints RS256 body-kid JWTs via the **same
   `signAuthToken` minter S2 owns** (`auth.ts:131,330,460`) and runs a full **refresh-rotation** state
   machine (family reuse-detection → revoke family, `auth.ts:418–428`). Every courier REST request then
   binds the token to **live server-side session state** — `courierSessionValid` re-reads `courier_sessions`
   by `jti` and demands the session be present, un-revoked, un-expired, **and** the courier still hold
   membership in the token's `activeLocationId` (`plugins/auth.ts:24–92`). The port must **reuse the S2
   minter/verifier** (a second impl = a kid-drift cross-verify break) **and carry the session-liveness bind**
   (verifying the JWT alone is a 14-day logged-out bearer — the exact S6 WS-T8 gap, here on REST).
2. **The dispatch/assignment-state seam (Q2 🔴).** The assignment lifecycle
   (`offered/assigned/accepted/picked_up → delivered/cancelled/rejected/offered_expired`) is driven by
   courier-scoped, status-guarded, `FOR UPDATE` mutations that each carry `AND courier_id = $` — the
   **cross-courier IDOR fix** (`courierAssignmentService.ts:16–27`). **Honest dispatch**
   (`lib/dispatch.ts`) finds a courier *before* advancing an order to IN_DELIVERY (no orphan, no fake
   courier), and every order-side transition **funnels through S5's `updateOrderStatus`**. The port must
   keep the actor-gate, the no-orphan ordering, and the single-mutator funnel intact.
3. **The courier-money seam (Q3 🔴 + Q4 🔴).** Two distinct money paths: **(a) cash-as-proof at delivery**
   (`completeDelivery`: `paid_full` requires `cash === total` → 422; the till-accountability HOLD ledger;
   the honest no-cash CANCELLED tail) and **(b) settlement/payout** (courier reads `courier_payouts`/
   `settlement_items` — the **same rows the owner reads (S5)** — while the money is *generated* by the
   SECURITY-DEFINER DB fn `app_generate_settlements`, which migration draft **085** rewrites to be
   catch-up/idempotent/immutable behind a `2026-07-10` watermark). The port **must not re-implement
   settlement math in Rust** (it calls the DB fn — schema-rich, runtime-minimal) and must not re-fork the
   read shape away from the owner read.

**The sharpest S7 fact (see §5, §9):** unlike S5 (money written on the request path), S7's largest money
lives in a **DB function** and a **cron worker**, not in the axum handler. That is a gift — the port is a
*thin caller* over proven SQL — but it means the **double-pay hazard is a timing/operator concern (the 085
watermark), not a Rust-arithmetic concern**, and the **cutover cannot "fix" it** by porting carefully; it can
only *preserve* the DB fn and *not trip the watermark*.

## 2. Scope — what is S7, what is explicitly NOT

**In this packet (S7):**
1. **Courier auth** — `POST /api/courier/auth/{invites/:id/redeem,login,refresh,logout}` + `GET
   /invites/:inviteId` (rows 134–138): JWT mint (body-kid RS256, via the S2 minter), the refresh-rotation
   family machine, argon2 password/token hashing, PII encryption, the courier session table.
2. **Courier session-liveness bind** — the `courierSessionValid` re-read every courier REST/WS request runs
   (`plugins/auth.ts`). Shared with S6 (REV-S6-2) and mirrors ADR-0004 owner P-d.
3. **Shift lifecycle** — `GET /me/shift`, `POST /me/shift/{start,end}`, `POST /shifts/{transition,ping}`
   (rows 129–133): the shift state (`offline/available/on_delivery`), the GPS ping + geofence + heartbeat
   liveness, the P0-1 on-active-delivery-only position storage. **`shifts.ts` is the repo's worst-health
   file (CLAUDE.md 1.0/10) — audited in §7.**
4. **Assignment lifecycle** — `GET /me/assignments`, `GET /assignments/:id`, `POST /assignments/:id/{accept,
   reject,picked-up,delivered,cancel,abort,decline}` (rows 114–122): the offer-handshake + legacy paths, the
   actor-gate, the shared release-and-reoffer rail.
5. **completeDelivery + cash-as-proof** — the single completion primitive (`lib/deliveryCompletion.ts`): the
   `cash===total` assert, the HOLD ledger, the honest no-cash CANCELLED tail, the crypto prepaid auto-resolve
   (§6).
6. **Courier money reads** — `GET /me/payouts`, `GET /me/payouts/:id` (rows 139/140), `GET /me/earnings`
   (row 127), `GET /me/history` (row 128): the courier-side read of the settlement ledger + masked history.
7. **Courier profile** — `GET /me`, `PATCH /me/messenger`, `GET /me/audit-log`, `PATCH /me/password` (rows
   123–126): PII decrypt/mask, the password-change full-session-revoke.
8. **Owner-side courier management** — `GET/PATCH …/couriers[/:id]`, `GET …/couriers/live`, `GET
   …/orders/:orderId/route`, `GET …/couriers/:id/details`, `POST/GET/DELETE …/courier-invites` (rows 75–79,
   96–98): the roster, deactivate/role-change (with session revoke), the live map, the breadcrumb route, the
   invite mint (owner-root `with_user` write).
9. **The honest-dispatch engine** (`lib/dispatch.ts`) — the availability query + no-orphan ordering, *called*
   from S5's PATCH and from the dispatch worker, but the **assignment-binding logic is S7's**.

**NOT S7 (explicit boundary — each a separate slice):**
- **The order state machine + `updateOrderStatus`** (the matrix, the folds, the actor-gate) — **S5**. S7
  *calls* `updateOrderStatus` for every order-side transition (accept→CONFIRMED/IN_DELIVERY, picked-up→
  IN_DELIVERY, delivered→DELIVERED, no-cash tail→CANCELLED) but does not own it.
- **Settlement GENERATION** (`app_generate_settlements`, the settlement-cron worker, migration 085 + the
  backfill fns) — a **DB-function + S8-jobs** concern. S7 *reads* the resulting `courier_payouts`/
  `settlement_items` and *calls* the fn only through the owner `/regenerate` proxy (S5-path-owned, row 29).
  S7 does **not** author/apply 085 (`packages/db/migrations/` red-line) and does not re-implement the math.
- **The dispatch/offer-sweep WORKERS** (`courier-offer-sweep.ts`, `courier-dispatch.ts`) — **S8 jobs**. They
  share the assignment state machine + the `courier_dispatch_queue` journal + the `app_sweep_expired_offers`
  DEFINER fn, but the cron machinery is S8.
- **The owner settlement WRITE lifecycle** (`owner/settlements.ts` approve/pay/dispute/reopen/regenerate,
  rows 23–29) — **S5**. S7 owns only the courier *read* of that ledger.
- **WS transport / fan-out** (courier live tail, dashboard rooms) — **S6**. S7 *publishes* to the bus
  (assignment/shift/position deltas); the transport + fan-out authz is S6. The REST accept/pickup/delivered
  complete a delivery **regardless of the tail** (S6 WS-T10).
- **`POST /api/owner/…/orders/:orderId/assign-courier`** (dashboard.ts:215, row 40) — **S5-path-owned** (the
  owner manual-assign), though it is a dispatch effect. Flagged for both surfaces' cutover DoD.
- **No schema change** — the DB is frozen. Migration 085 is an **operator-placed draft**, not an S7
  deliverable.

**Back-of-envelope (why boring wins, and where the real pressure is).**
- **Scale:** target **N ≈ 10–50 active locations**, **1–3 active couriers each** at peak → **≈ 30–150
  concurrently-on-shift couriers** system-wide. Delivery taps (accept/pickup/delivered) are **low
  tens/min** — bursty at lunch/dinner. Settlement is a **once-daily 2 AM cron** (or an owner `/regenerate`).
- **The courier-specific hot write is the GPS ping** — **1 ping / 10 s / active-delivery courier**
  (`shifts.ts:305`, rate-limited per bearer). At 30–150 active couriers that is **≈ 3–15 ping-writes/sec**
  at peak, each a bounded write-tx (geofence read + `courier_positions` insert + heartbeat update). Even a
  10× headroom is ~30–150 writes/min — negligible against a 20-conn operational pool.
- **The per-request session-liveness read** (`courierSessionValid`) adds **one pool query per courier REST
  request** — at ping cadence, ~3–15 extra reads/sec. Bounded; cache-free by design (revocation must be
  immediate, ADR-0004 posture). It is a *pool-draw* line item for the cutover budget, not a QPS ceiling.
- **The cutover connection budget** (the real ceiling, per S5 §2): during the S7 overlap the SAME
  Postgres/Supavisor pool is drawn by **API operational (Rust ~20 + Node ~10)** + **workers** (offer-sweep,
  settlement-cron, dispatch, timeout-sweep) + **analytics** (owner live-map/roster reads) + **migrations**.
  The ping-cadence + per-request session read are the S7-specific additions; the flip must be **atomic +
  time-boxed** (§9), riding the **S6 low-delivery-window** (REV-S6-5).
- **Conclusion:** S7 is **not connection-bound at steady state**; boring monolith-in-`api` (no new runtime)
  is correct. The engineering risk is entirely **correctness** (auth/session bind, actor-gate, cash-as-proof,
  settlement idempotency) and **cutover concurrency of in-flight deliveries** — not throughput.

---

## 3. Concern 1 — Courier auth / JWT / session-liveness (Q1 🔴, the S2-parity-outside-S2 problem)

**The fact pattern.** `courier/auth.ts` is path-owned S7 but performs **S2-class operations**: it mints
(`signAuthToken({sub, role:'courier', activeLocationId, jti}, '14d'|'24h')`, `auth.ts:131,330,460`),
refreshes (rotation with family reuse-detection, `auth.ts:354–476`), and revokes (`logout`, `auth.ts:479`).
The **CourierClaims** shape is `{ sub=courierId, role:'courier', activeLocationId, jti=sessionId }` — distinct
from OwnerClaims (memberships) and CustomerClaims (orderId). The **session bind** is the coupling: on **every**
courier REST request `verifyAuth` re-reads `courier_sessions` by `jti` and enforces `courierSessionValid`
(present ∧ ¬revoked ∧ ¬expired ∧ courier still has membership in `activeLocationId`, `plugins/auth.ts:24–92`).

**Port contract:**
1. **Reuse the S2 minter/verifier — NEVER a second impl (🔴 S7-T2).** The Rust courier auth calls the **same
   body-kid RS256 `signAuthToken`/`verifyAuthToken` port S2 built** (`@deliveryos/platform` → the S2 Rust
   crate). A separate courier minter risks a **kid-handling drift** that breaks cross-verification: a courier
   JWT minted by Rust-S7 must verify under the S2 verifier and vice-versa (the S6 WS admission and the REST
   `verifyAuth` both use the S2 verifier). **This couples S7's flip to the S2 verifier's cutover
   re-ratification** — S7 auth rides whatever kid/rotation posture S2 settles; the packet names the coupling,
   it does not re-open S2.
2. **Carry the session-liveness bind verbatim (🔴 S7-T3).** Port `courierSessionValid` as a **per-request DB
   re-read** (no cache — revocation/deactivation/password-change/refresh-rotation must take effect
   *immediately*, not wait out the 14-day JWT). This is the SAME check S6 REV-S6-2 wires at the WS tail; the
   two must share one predicate so a revoked session is denied on **both** REST and the next WS reconnect.
   **E2E: revoke the session (logout / owner-deactivate / password-change) → the next courier REST call is
   401, parity with the WS drop.**
3. **Port the refresh-rotation family machine faithfully.** The `${sessionId}.${tokenPlain}` refresh format,
   argon2 `token_hash` (so the plaintext is never stored / never queryable — the lookup is by `sessionId`,
   `auth.ts:400`), `FOR UPDATE NOWAIT` (no refresh stampede), the **reuse-detection → revoke the whole
   `family_id`** (`auth.ts:418–428`), and the courier-status re-check (`couriers.status='active'`). The
   `courier_sessions` table is **separate** from owner sessions; S7 either reuses S2's session-rotation
   primitives over the courier table or ports them, but the **rotation invariants (single-use refresh, family
   revoke on reuse) are red-line** and get their own red→green tests.
4. **The `!jti` dev-gate branch (ADR-0003) — carry fail-closed.** A courier token without a `jti` is admitted
   **only** under the dev-login gate (`devLoginAllowed(env)`, `plugins/auth.ts:63–70`); on prod it is 401.
   The synthetic-courier / mock-auth seam (`lib/synthetic-courier.ts`, dev-only, sentinel non-email hash,
   excluded from the owner roster) rides the same gate. **Port the fail-closed-on-prod behavior exactly** —
   this is the anti-fake-dispatch boundary (§4, S7-T5).
5. **Owner-side session revocation is a courier-security control.** Owner deactivate/suspend/role-change and
   password-change all `UPDATE courier_sessions SET revoked_at = now()` (`owner/couriers.ts:114–120`,
   `me.ts:160–164`) — the session-liveness re-read (2) is what makes that revocation *effective*. Port the
   revoke-on-deactivate write **and** the read that enforces it as one coupled contract.

**Failure-first:** an invalid/expired/revoked token → **401** (never a silent pass); a courier token whose
`activeLocationId` membership was removed → 401 (`has_location=false`); a malformed refresh token → 401
`INVALID_REFRESH_TOKEN`; a reused refresh → 401 `REFRESH_REUSED` + **family revoked**; a DB error in the
liveness read → **500** (fail-closed, not fail-open — carry the `plugins/auth.ts:87–90` posture, do not
degrade to "admit on read failure").

## 4. Concern 2 — Dispatch / assignment state machine (Q2 🔴)

**The assignment lifecycle (the states + the legal edges the port must encode):**

```
                 (owner assign-courier, S5) ┐          (courier-offer-handshake, flag ON)
                                             ▼                         │
  [new] ── honest-dispatch ──► assigned ──accept──► accepted ──picked-up──► picked_up
              (lib/dispatch)      │  │                  │  │                    │
                                  │  └─reject──►rejected │  └─cancel/abort─►cancelled
                          accept-timeout (sweep)         cancel/abort─►cancelled│
                                  ▼                                   deliver ──►│
                              cancelled                     ┌──────────────────────┘
        offered ──accept──►accepted   ──decline──►offered_expired   completeDelivery:
           │                                                 paid_full/prepaid ─► delivered
     offer-expiry (sweep) ─► offered_expired                 refused/cancelled ─► cancelled
```

**What S7 must port (each an independent invariant):**
1. **The actor-gate on every mutation (🔴 S7-T1 cross-courier hijack).** Every accept/reject/picked-up/
   delivered/cancel/abort/decline reads the assignment with `WHERE id=$ AND courier_id=$courierId AND
   status=$expected FOR UPDATE` (`assignments.ts:144,192,253,325,436,501,546`;
   `courierAssignmentService.ts:21–27`). The `AND courier_id=$` predicate is the **IDOR fix** — RLS isolates
   only by *location*, so without it any courier in the same location could hijack another's assignment. The
   `status=$expected` predicate is the **anti-race** (a stale tap → 404, not a double-transition). **Carry
   both predicates on every mutation; the DoD probes a cross-courier accept → 404.**
2. **Honest dispatch — no orphan, no fake courier (🔴 S7-T5).** `attemptHonestDispatch` (`lib/dispatch.ts`)
   finds an **available, active, unbound** courier **before** advancing to IN_DELIVERY; no courier → **do not
   advance** (`{dispatched:false, reason:'no_courier'}`, `dispatch.ts:41–42`). The availability query filters
   `c.status='active' AND cs.status='available' AND courier NOT IN (active-binding set)`
   (`dispatch.ts:27–40`) — a courier is only offered a run they can take. The **synthetic/dev courier is
   excluded** from the owner roster (`owner/couriers.ts:40 AND c.email_hash <> $2`) and admissible only under
   the ADR-0003 gate. **Carry the find-then-advance ordering** (advance-then-orphan is the no-recovery
   failure) **and the synthetic exclusion**.
3. **Every order-side transition funnels through S5's `updateOrderStatus`.** accept→CONFIRMED (legacy) or
   IN_DELIVERY (offer-handshake first move, `assignments.ts:152`); picked-up→IN_DELIVERY; delivered→DELIVERED
   (via `completeDelivery`); the no-cash tail + cancel/abort→CANCELLED/READY (via `releaseBindingAndReoffer`,
   `bindingRelease.ts`). S7 **owns the assignment/shift writes**; **S5 owns the order write**. The port keeps
   this single-mutator boundary — an S7 handler must never hand-`UPDATE orders.status` (the C-2 trap the
   shared rail closed, `bindingRelease.ts:5–13`).
4. **The two accept branches (flag-gated).** `COURIER_OFFER_HANDSHAKE_ENABLED` ON: an `offered` assignment is
   the first move → accept advances the order to IN_DELIVERY (`assignments.ts:144–156`). OFF (legacy): an
   `assigned` binding → accept via `acceptCourierAssignment` (30 s accept window, `assignments.ts:160`) →
   order to CONFIRMED (idempotent, swallow if past). **Carry both, flag-guarded, dark-verifiable.**
5. **The shared cancel/abort rail (`releaseBindingAndReoffer`).** `/cancel` (accept-regret, 5-min time gate,
   `assignments.ts:447`) and `/abort` (en-route, no gate) take the SAME rail: **unconditionally free the
   binding + shift**, then take an order-side action **guarded on the locked order status** — revert
   IN_DELIVERY→READY (pre-pickup, food at venue) or IN_DELIVERY→CANCELLED (post-pickup, food out) or **no
   transition** (never advanced) — then re-enqueue to the journal. **Carry the guarded-transition rule** (a
   context-free `updateOrderStatus` would throw on an illegal edge, `bindingRelease.ts:40–55`).
6. **Who may drive what (the authorization matrix the port must encode):**
   | Principal | Route | Allowed | Gate |
   |---|---|---|---|
   | **courier** | accept/reject/picked-up/delivered/cancel/abort/decline | only their OWN assignment, only from the expected state | `AND courier_id=$ AND status=$` `FOR UPDATE` + session-liveness (§3) |
   | **owner** | assign-courier (S5 path, row 40) / couriers PATCH (row 76) | manual dispatch / deactivate+role | owner-membership + `requireLocationAccess` + `requireRole(['owner'])` |
   | **system** | offer-expiry / accept-timeout / drain / grace-cancel | the sweep edges | **S8 offer-sweep worker** — funnels through the DEFINER fn + `updateOrderStatus` |

**Failure-first:** a stale/foreign assignment tap → **404** (never a cross-courier write); an
already-terminal tap → 404; honest-dispatch with no courier → **stay put** (the owner re-taps); the
grace-cancel (dark, `DISPATCH_OWNER_GRACE_ENABLED` off) is the only path that terminalizes an order the owner
ignored — **carry it dark**, do not launch it in the port.

## 5. Concern 3 — Courier money: settlements / payouts (Q3 🔴)

**The fact pattern.** Courier `GET /me/payouts[/:id]` (`courier/settlements.ts`) reads `courier_payouts` +
`settlement_items` — the **identical rows** owner `GET …/settlements[/:id]` reads (`owner/settlements.ts`,
S5). The **write authority** is the owner lifecycle `pending → approved → paid` (+ `disputed → pending`,
`owner/settlements.ts:110–298`, S5). The **generation** is the SECURITY-DEFINER DB fn
`app_generate_settlements(period_start, period_end)` (called by the 2 AM cron `settlement-cron.ts:44` and the
owner `/regenerate` proxy `owner/settlements.ts:312–314`). Money is **integer minor units end-to-end**:
`settlement_items.amount = courier_assignments.cash_amount`, `courier_payouts.total_earned = sum(amount)::int`.

**Port contract:**
1. **Do NOT re-implement settlement math in Rust — call the DB fn (🔴 S7-T6).** The catch-up scan, the
   paid-payout immutability lock, the `ON CONFLICT (assignment_id) DO NOTHING` per-item idempotency, the
   aggregate recompute, the single-flight `pg_advisory_xact_lock`, and the `2026-07-10` watermark **all live
   in `app_generate_settlements`** (085 draft, `docs/design/audit-fix-money/migration-drafts/…085…`). The
   Rust port's only settlement-generation touch is `SELECT app_generate_settlements($1,$2)` (through the
   S5-owned `/regenerate` and the S8 cron). **"Schema-rich, runtime-minimal": the money engine is Postgres;
   the port is a thin caller.** Re-implementing the aggregation in Rust would re-open every C2 double-pay path
   085 closed.
2. **The 085 double-pay landmine is a TIMING gate, not a port bug (🔴 S7-T7, Q7).** The watermark literal
   `2026-07-10 00:00:00+00` lower-bounds the catch-up scan. Erring **LATE** is safe (rows defer to the
   operator backfill); erring **EARLY** (literal *before* the real apply) **double-pays** rows the old buggy
   fn already SKIP-LOCKED-dropped and plausibly reconciled in person. If the *rebuild schedule* slips the
   settlement apply past 2026-07-10, the operator must bump **all three literal occurrences** before apply.
   S7 does not author/apply 085 — it **surfaces the watermark as an operator-owned schedule landmine** the
   rebuild cannot silently trip.
3. **Agree byte-for-byte with the owner read (🔴 S7-T8 divergent-money-display).** The courier read and the
   owner read project the SAME `total_earned`/`deliveries_count`/`status`/`period` from the SAME rows; the
   only difference is **PII scope** (courier gets no order-id/customer data — `settlements.ts:89` "strictly no
   orderId, no assignmentId, no customer phone"; owner gets a masked courier name). **The port keeps ONE
   canonical payout-read DTO with a role-scoped projection**, not two drifting shapes — a courier seeing a
   different amount than the owner approved is a trust-breaking money bug. **Carry the courier's stricter PII
   redaction** (S7-T9).
4. **Fix the settlement-read tenancy seat (🔴 S7-T10, §8).** `courier/settlements.ts` seats the tenant with
   `db.query('SELECT set_config(app.current_tenant, $1, true)')` **on the pool, with `is_local=true` and NO
   `BEGIN`** (`settlements.ts:24–26,57–59,74–76`) — the GUC is transaction-local to an *implicit* statement
   and the subsequent `db.query` runs on a **potentially different pooled connection with no seat**. It
   "works" only because the pool bypasses RLS today; **post-B3 the `settlement_items` policy (bare
   `current_setting('app.current_tenant')::uuid`, no missing-ok) ERRORS** (`settlement-cron.ts:36` documents
   exactly this). The port routes the read through **`with_tenant(app.current_tenant=activeLocationId)` in one
   real tx** — a FIX-IN-PORT with a NOBYPASSRLS probe (the same class as the S5 create-tx seat fix).
5. **The `/regenerate` cross-tenant blast (carry-and-flag).** `owner/settlements.ts:300` instantiates a
   `SettlementCronWorker` on the request path and processes **ALL locations** (its own comment admits it) —
   a per-surface flip does not bound this route to one tenant. It shares the single-flight advisory lock with
   the cron and the 085 backfill. **This route is S5-path-owned**; S7 flags it because it invokes the S7-money
   fn. **Carry the behavior; record the cross-tenant blast as an accepted-risk with an owner** (it is
   idempotent under 085, so a redundant cross-tenant sweep is a no-op, not a double-pay — *once 085 lands*).

## 6. Concern 4 — completeDelivery + cash-as-proof (Q4 🔴)

**The single completion primitive (`lib/deliveryCompletion.ts`, ADR-deliver-v2-cash-as-proof).** BOTH the
courier `/delivered` handler (`assignments.ts:292`) and the owner-proxy `/deliver` handler call
`completeDelivery`, so the cash-as-proof HOLD + `payment_outcome` + immutable `delivery_trace` are structurally
guaranteed on **every** delivered order. It runs **inside the caller's tx** (the caller owns
`BEGIN`/`COMMIT` + the status-guarded `FOR UPDATE`).

**Port contract (each clause a red→green vector):**
1. **The no-partial-handover rule (🔴 S7-T11 cash-proof bypass).** `paid_full` REQUIRES `cashAmount ===
   total` → `CompletionError('CASH_AMOUNT_MISMATCH')` → **422 before any mutation** (`deliveryCompletion.ts:
   63–65`). `total` is the full server-authoritative order total (incl. delivery fee + tax) — the courier
   collects the whole declared amount in cash; a partial is refused. **Carry the exact equality assert** (not
   `>=`, not `<=`) and the 422.
2. **The honest no-cash tail.** `refused_goods / refused_payment / customer_cancelled_on_door` → assignment
   `cancelled`, order **CANCELLED** (so the customer never sees "Delivered" for refused food,
   `deliveryCompletion.ts:75–78`), **no HOLD**. `paid_full / delivered_prepaid` → assignment `delivered`,
   order **DELIVERED**. **Carry the outcome→status map exactly** — a refused delivery that reads DELIVERED is
   a lie to the customer + a phantom settlement row.
3. **The cash-as-proof HOLD (🔴 money).** `paid_full` only → `INSERT courier_cash_ledger (type='hold',
   amount=cashAmount) ON CONFLICT (order_id,type) DO NOTHING` (`deliveryCompletion.ts:118–123`) — the
   courier's till-accountability bond until shift reconciliation. **Idempotent** (a retried `/delivered`
   writes no second hold). **Carry the ON CONFLICT** — it is the retry-safety of the money write.
4. **The crypto prepaid auto-resolve (ADR-0017, dark).** If `payment_method='crypto' AND
   payment_status='paid'` → force `delivered_prepaid` (overrides any cash-derived outcome,
   `assignments.ts:338`); `completeDelivery` then skips the cash assert + writes **no HOLD**
   (`deliveryCompletion.ts:58–60`), precondition `payment_status='paid'` else **409 `PREPAID_NOT_PAID`**
   (`deliveryCompletion.ts:68–72`) — never mark a not-yet-confirmed crypto order delivered. **Carry dark**
   (crypto flags off); the branch ports inert.
5. **`payment_outcome` is the first-class signal.** The Zod enum forbids `paid_partial`/`pending` as delivered
   outcomes (`assignments.ts:299`); `cash_amount` is `int().nonnegative()` (M-2); legacy `cash_collected` →
   derives `paid_full`/`refused_payment` (`assignments.ts:312`). **Carry the enum + the legacy derivation.**
6. **The refund_due obligation is NOT here (it moved to S5).** `completeDelivery` no longer inserts
   `refund_due`; the CANCELLED tail's `updateOrderStatus` records it via the L-A fold (S5,
   `deliveryCompletion.ts:126–131`). S7 **relies on** the S5 fold + the mig-086 L-C trigger backstop — the
   port must ensure the CANCELLED tail runs `updateOrderStatus` inside the tenant-seated tx so the fold passes
   FORCE-RLS (§8). **Inert until crypto flips** (zero `paid` rows).
7. **The immutable trace + the shift free.** `delivery_trace` `ON CONFLICT (order_id) DO NOTHING`
   (passive crumbs: gps, route-distance, expected-min — recorded, never thresholded); the shift returns to
   `available` (`deliveryCompletion.ts:91–92`). **Carry idempotent.**

**Failure-first:** cash mismatch → 422; prepaid-not-paid → 409; a `CompletionError` ROLLBACKs before any
write (`assignments.ts:371–377`); a retried completion is a no-op (ON CONFLICT everywhere).

## 7. Concern 5 — `shifts.ts` worst-health audit (Q5 🔴, CARRY-vs-FIX per defect)

**`courier/shifts.ts` is the repo's worst-health file (CLAUDE.md 1.0/10).** A rewrite is the moment to fix
its 🔴 latent defects — but only with a documented E2E delta; everything else CARRIES for parity. Enumerated
defects, each dispositioned:

| # | Latent defect (source) | Disposition |
|---|---|---|
| **D1** | **`/shifts/transition` selects the shift with NO status filter, NO `ORDER BY`, NO `LIMIT`** — `SELECT id,status FROM courier_shifts WHERE courier_id=$ AND location_id=$ FOR UPDATE` then takes `rows[0]` (`shifts.ts:196–203`). A courier with >1 shift row (yesterday's `offline` + today's) transitions an **arbitrary** row. Diverges from `/me/shift` (status-filtered + `ORDER BY started_at DESC LIMIT 1`, `shifts.ts:26–31`) and `openShift` (`DATE=CURRENT_DATE` + `LIMIT 1`, `shiftService.ts:15–23`). | **FIX-IN-PORT (🔴 correctness):** deterministic single-row selection matching `openShift` (today's row, `ORDER BY started_at DESC LIMIT 1 FOR UPDATE`). E2E delta: a courier with a stale offline row transitions the CURRENT shift, not the stale one. |
| **D2** | **Three overlapping shift-mutation surfaces with divergent logic** — `/me/shift/start` (via `openShift` service), `/shifts/transition` (inline SQL), `/me/shift/end` (inline SQL). `openShift` scopes to today; transition does not; transition-to-available requires GPS, `openShift` does not. Drift-by-construction. | **FIX-IN-PORT (structural):** consolidate to ONE `shift_state` service (single writer) in the port — "schema-rich, runtime-minimal". Preserve each route's *external* contract (status codes/GPS requirement) but funnel through one selector. Documented as a refactor, parity-tested. |
| **D3** | **Active-assignment guard has NO `location_id` predicate** — `SELECT 1 FROM courier_assignments WHERE courier_id=$ AND status IN('assigned','accepted','picked_up')` (`shifts.ts:135–138,218–221`). A courier working two locations has an active delivery at B block going offline at A (cross-location coupling). | **FIX-IN-PORT (belt, low-risk):** add `AND location_id=$activeLocationId`. Rare (multi-location courier) but a defense-in-depth predicate that holds independent of RLS. CARRY if the council prefers strict parity. |
| **D4** | **Ping geofence range-check is SKIPPED when the location has no pin** — `if (locRes.rowCount>0 && lat && lng)` (`shifts.ts:343`); a pin-less location accepts GPS from anywhere (`GPS_OUT_OF_RANGE` never fires). | **CARRY + document:** "no venue pin ⇒ no geofence" is a defensible current property (a location that hasn't set coordinates cannot range-check). Record as an accepted-risk row; the P0-1 privacy gate (position stored only on active delivery) still holds. |
| **D5** | **Ping rate-limit `keyGenerator` keys on the full `authorization` header** (the bearer JWT, `shifts.ts:316`) — a secret-bearing string as the bucket key, and the bucket **rotates on 24 h refresh** (a refreshed courier gets a fresh bucket). | **CARRY + note:** the per-courier bucket is the intent (multiple couriers behind one NAT must not throttle each other); the token-as-key is acceptable in-memory. Port to key on the stable `sub` (courier id) from the verified claims instead of the raw header — a cleaner FIX-IN-PORT if the limiter runs post-auth. |
| **D6** | **`openShift` throws a raw object `{ statusCode, error }`** (not an `Error`) on the "cannot open in status X" branch (`shiftService.ts:44`); likewise `courierAssignmentService.ts:30,37,43`. May not serialize to the standard error envelope. | **FIX-IN-PORT (error-contract):** typed errors mapped to the sendError envelope; post-Astro FE-lockstep for the exact body shape (S2/S3 Q4 posture). |
| **D7** | **`set_config('app.current_tenant', $1, true)` with a possibly-undefined `activeLocationId`** — if a courier token lacks `activeLocationId`, the seat is null (`shifts.ts:24,77,120,193,337`). | **FIX-IN-PORT (guard):** the port's `with_tenant` takes a non-null `TenantId`; a claim without `activeLocationId` → 401 at `verifyAuth` (the session-liveness read already requires `has_location`, §3 — so this is defense-in-depth). |
| **D8** | **The ping heartbeat + geofence-event SAVEPOINT is best-effort** (`shifts.ts:399–411`) — a sensor write never fails the ping (observe-don't-control). | **CARRY verbatim** — the correct failure-isolation pattern; port the SAVEPOINT dance. |

**Net:** D1/D2/D6/D7 are 🔴 FIX-IN-PORT (correctness/structure/guard); D3/D5 are low-risk FIX candidates
(belt); D4/D8 CARRY. The worst-health score is driven by the D1/D2 selection-logic divergence — the port
collapses three shift writers into one, which is the single highest-leverage correctness fix in S7.

## 8. Tenancy — the courier GUC seam (Q6 🔴)

**Which GUC? Spelled out because S7 has BOTH roots on courier-adjacent tables:**
- **Courier self-actions seat `app.current_tenant = activeLocationId` (the SERVICE/TENANT root), NOT
  `app.user_id`.** A courier is **not an owner-membership** — the `courier_tenant_update` (and sibling)
  policies key on `app.current_tenant`. The shift/assignment/ping writes correctly `BEGIN` + `set_config
  ('app.current_tenant', activeLocationId, true)` in a tx (`shifts.ts`, `assignments.ts`) — **carry the
  family**, using the S3 REV-10 **non-confusable `TenantId`** type. This is the same service root S5 order
  writes use.
- **Owner-side courier management seats `app.user_id = ownerId` (the OWNER root).** `courier-invites.ts` uses
  `withTenant(db, ownerId, …)` (`courier-invites.ts:49,91,110`) and `couriers.ts` reads roster under
  `app.current_tenant` after the owner gate. **⚠️ Naming trap:** the Node `withTenant(db, ownerId)` combinator
  seats **`app.user_id`** despite the "tenant" name — the exact S3 REV-10 confusable the port must resolve
  with distinct `UserId`/`TenantId` types so an owner write is never mis-seated as a tenant write (or
  vice-versa). One table, **two roots**: `courier_invites` is *written* by the owner (`with_user`) and
  *read/consumed* by the anonymous redeemer under `app.current_tenant` (`courier/auth.ts:159–216`, the
  two-pass RLS dance) — carry both.
- **The broken/missing seats to FIX-IN-PORT (🔴 the never-copy leak class):**
  - `courier/settlements.ts` — **broken seat** (pool `set_config` + `is_local=true` + no `BEGIN`, §5.4).
  - `owner/couriers.ts` `/couriers/live` (`couriers.ts:152`) — **broken seat** (`set_config(…,true)` with
    **no `BEGIN`** on the same client → the GUC reverts before the SELECT). The **same file** seats correctly
    in `/couriers` (`couriers.ts:29`, with the explicit "is_local=true requires an explicit BEGIN" comment)
    and `/orders/:orderId/route` (`couriers.ts:210`) — a **within-file seat drift** the port must unify.
  - `owner/couriers.ts` `/couriers/:id/details` (`couriers.ts:254–284`) — **NO seat at all** (bare pool
    `db.query`), relying solely on the explicit `WHERE location_id` predicate.
  - `owner/settlements.ts` read routes (list/detail, S5) — bare pool, no seat; the mutation routes `BEGIN`
    but seat no tenant (the `settlement_audit_log` bare-`current_setting` policy ERRORS post-B3).
  All "work" today only under BYPASSRLS. **The port routes each through the correct combinator in one real
  tx** — `with_tenant(activeLocationId)` for courier/service reads, `with_user(ownerId)` for owner writes —
  with a NOBYPASSRLS probe. This is a B3-council-adjacent fix; the port carries it visibly (matrix row +
  probe), it does not "harden" behavior beyond restoring the correct seat.
- **Belt-AND-suspenders (carry verbatim).** Every courier/settlement statement already carries an explicit
  `WHERE courier_id=$ / location_id=$` predicate — carry these; they hold **independent of** which pool role
  is live (the identity-split root). The DoD extends the `rls-adversarial` "privileged pool queries have
  WHERE" gate to the courier/settlement reads.
- **The `courier_sessions` liveness read is a bare pool query (auth-table).** `courier_sessions` is an
  auth/session table (keyed by `courier_id`/`jti`), not a tenant-RLS table on the request path; the port
  carries it as the S2 verifier does (privileged auth read), not through `with_tenant`. Named so the port
  does not accidentally wrap it in the wrong root.

## 9. Cutover — an active delivery crossing the flip (Q7 🔴)

**The failure classes and controls (S7 is mostly stateless-HTTP over an authoritative DB — the good case):**
1. **In-flight delivery crossing the flip — bounded, structural.** accept/pickup/delivered/cancel/abort are
   **stateless REST**; the assignment/shift/order rows are the authority. A delivery picked-up on Node and
   delivered on Rust (or vice-versa) advances correctly **iff** the assignment state machine + the
   `completeDelivery`/`releaseBindingAndReoffer` folds + `updateOrderStatus` are **byte-equal** across
   stacks. The ON-CONFLICT idempotency (HOLD, trace) makes a cross-stack retry safe. **Control:** a
   **cross-stack delivery-completion probe** (pick-up on X, deliver on Y → one DELIVERED, one HOLD, one
   trace). This mirrors S6 WS-T10 ("accept/pickup/delivered are REST — delivery completes regardless of the
   tail").
2. **The WS tail is S6, and it drains on the S6 schedule (REV-S6-5).** S7's REST flip is **orthogonal** to
   the S6 WS drain, but they **share the low-delivery-window constraint**: schedule the S7 flip in the SAME
   **low-delivery window** so the fewest couriers are mid-delivery when either surface flips, and rely on the
   S6 **gradual drain** for the live tail. **Control (operator sign-off, not prose):** the S7 flip is
   scheduled in a low-delivery window (REV-S6-5), co-scheduled with the S6 drain.
3. **The session-liveness read must be identical across stacks.** During overlap a courier request may hit
   Node or Rust; both must run the SAME `courierSessionValid` predicate against the SAME `courier_sessions`
   row, or a revoked courier is admitted on one stack. **Control:** the session-liveness golden test runs
   against both stacks (the S6 REV-S6-2 predicate is the shared spec).
4. **Settlement generation is stack-agnostic.** `app_generate_settlements` is a DB fn under a single-flight
   advisory lock; the cron/`/regenerate` call it identically from either stack. The flip does not affect
   settlement money — **but the 085 watermark must be landed/verified BEFORE any settlement apply during the
   rebuild window** (§5.2, Q7): erring early double-pays regardless of stack.
5. **Money is NOT one atomic surface — the S5/S7 flips are separate (🔴 co-scheduling).** Owner settlement
   *writes* (approve/pay) are S5; courier settlement *reads* are S7. The shared `courier_payouts` rows must
   stay consistent across **two independent flips** — a courier reading on Rust-S7 while an owner approves on
   Node-S5 (or any stack combination) must see the same integer amount. **Control:** the payout-read DTO is
   ONE canonical projection (§5.3); the S5 and S7 flips do not need to be simultaneous, but the read/write
   parity gate spans both.
6. **Atomic per-surface flip (S3 REV-7) + rollback = proxy flag-flip.** The whole S7 REST family flips
   together behind the proxy; because both stacks write the SAME tables through the SAME invariants, a
   rollback mid-overlap leaves in-flight deliveries valid on either stack. The rollback plan is a proxy flag,
   not a data migration.
7. **Connection-budget ceiling (§2).** The overlap adds the ping cadence + per-request session-liveness reads
   on top of the doubled API pool draw. **Time-box the overlap**, ride the low-delivery window, monitor
   combined operational-pool utilization.

**Cutover DoD gates specific to S7 (in addition to §11):** cross-stack delivery-completion probe (one
DELIVERED/HOLD/trace) · session-liveness parity across stacks (revoke → 401 on both) · low-delivery-window
schedule co-scheduled with S6 drain · 085 watermark verified before any settlement apply · payout-read
parity with the owner read · crypto/grace-cancel stay dark throughout.

## 10. Quirk register — carry-vs-fix (default = CARRY-VERBATIM)

**FIX-IN-PORT only for a 🔴 security/correctness/build-correctness issue, each with an explicit test/E2E
delta.** Everything else CARRIES; shape-migration rows defer to post-Astro FE-lockstep.

| ID | Quirk (source) | Disposition |
|---|---|---|
| Q-JWT-MINTER | courier/auth mints body-kid RS256 via the S2 `signAuthToken` (`auth.ts:131,330,460`) | **REUSE the S2 minter/verifier** — never a second impl (kid-drift cross-verify break, 🔴 S7-T2). S7 flip couples to the S2 verifier re-ratification |
| Q-SESSION-LIVENESS | `courierSessionValid` per-request re-read (revoked/expired/membership, `plugins/auth.ts:24–92`) | **CARRY verbatim (🔴 S7-T3)** — the SAME S6 REV-S6-2 predicate; no cache; JWT-alone is a 14-day bearer. E2E: revoke → next REST 401 |
| Q-REFRESH-FAMILY | `${sessionId}.${tokenPlain}` refresh, argon2 token_hash, `FOR UPDATE NOWAIT`, reuse→revoke family (`auth.ts:391–469`) | **CARRY verbatim** — single-use refresh + family-revoke-on-reuse are red-line; own red→green tests |
| Q-DEV-GATE | `!jti` courier token admitted only under dev-login gate (ADR-0003, `plugins/auth.ts:63–70`); synthetic courier excluded from roster (`couriers.ts:40`) | **CARRY fail-closed-on-prod (🔴 S7-T5)** — the anti-fake-dispatch boundary |
| Q-ACTOR-GATE | every assignment mutation `AND courier_id=$ AND status=$ FOR UPDATE` (IDOR fix, `courierAssignmentService.ts:16–27`) | **CARRY verbatim (🔴 S7-T1)** — cross-courier hijack guard; DoD probes a foreign accept → 404 |
| Q-HONEST-DISPATCH | find-courier-before-advance; no courier → stay put (`dispatch.ts:41–42`); availability filters active∧available∧unbound | **CARRY verbatim (🔴 S7-T5)** — no orphan, no fake courier; carry find-then-advance ordering |
| Q-ORDER-FUNNEL | every order-side transition via S5 `updateOrderStatus`; cancel/abort via `releaseBindingAndReoffer` shared rail (`bindingRelease.ts`) | **CARRY verbatim** — single mutator; an S7 handler never hand-UPDATEs orders.status (C-2 trap) |
| Q-OFFER-HANDSHAKE | two accept branches (`offered`→IN_DELIVERY vs legacy `assigned`→CONFIRMED), `COURIER_OFFER_HANDSHAKE_ENABLED` (`assignments.ts:144–168`) | **CARRY both flag-guarded** — dark-verifiable |
| Q-CASH-PROOF | `paid_full` requires `cash===total` → 422 `CASH_AMOUNT_MISMATCH` (`deliveryCompletion.ts:63–65`) | **CARRY verbatim (🔴 S7-T11)** — exact equality, no-partial-handover; 422 before any write |
| Q-NOCASH-TAIL | refused/cancelled_on_door → assignment cancelled + order CANCELLED, no HOLD (`deliveryCompletion.ts:75–78`) | **CARRY verbatim** — never "Delivered" for refused food |
| Q-CASH-HOLD | `courier_cash_ledger` `type='hold'` `ON CONFLICT (order_id,type) DO NOTHING` (`deliveryCompletion.ts:118–123`) | **CARRY verbatim** — till-accountability bond; idempotent |
| Q-PREPAID | crypto auto-resolve → `delivered_prepaid`, precond `payment_status='paid'` else 409 (`deliveryCompletion.ts:58–72`) | **CARRY dark** (ADR-0017 flags off) — port inert |
| Q-REFUND-IN-S5 | `refund_due` moved to the S5 `updateOrderStatus` L-A fold; completeDelivery no longer inserts it (`deliveryCompletion.ts:126–131`) | **CARRY the boundary** — the CANCELLED tail runs updateOrderStatus in the tenant-seated tx; inert until crypto flips |
| Q-SETTLE-DBFN | settlement math lives in `app_generate_settlements` (SECURITY DEFINER); app is a thin caller (`settlement-cron.ts:44`) | **CARRY the boundary (🔴 S7-T6)** — the port calls the fn; never re-implements the aggregation in Rust |
| Q-085-WATERMARK | `2026-07-10` catch-up watermark; early literal double-pays (085 draft) | **OPERATOR TIMING GATE (🔴 S7-T7, Q7)** — bump all three literals if the settlement apply slips; S7 does not author/apply |
| Q-PAYOUT-READ-SHARED | courier `/me/payouts` reads the SAME `courier_payouts`/`settlement_items` the owner reads (`settlements.ts` both) | **CARRY ONE canonical DTO, role-scoped projection (🔴 S7-T8)** — courier keeps stricter PII redaction (`settlements.ts:89`) |
| Q-SETTLE-SEAT | courier settlement read seats tenant on the POOL with `is_local=true`, no `BEGIN` (`settlements.ts:24–26`) | **FIX-IN-PORT (🔴 S7-T10):** `with_tenant` in one real tx; NOBYPASSRLS probe (settlement_items policy ERRORS post-B3) |
| Q-COURIERS-SEAT-DRIFT | within `owner/couriers.ts`: `/couriers`+`/route` seat correctly (BEGIN), `/live` (no BEGIN) + `/details` (no seat) do not (`couriers.ts:152,254`) | **FIX-IN-PORT (🔴):** unify to one correct seat per read; NOBYPASSRLS probe |
| Q-SHIFT-SELECT | `/shifts/transition` selects shift with no status filter/ORDER BY/LIMIT → arbitrary row (`shifts.ts:196–203`) | **FIX-IN-PORT (🔴 D1):** deterministic single-row selection matching `openShift`; E2E delta |
| Q-SHIFT-TRIPLICATE | three divergent shift-mutation surfaces (start/transition/end) with drifting logic | **FIX-IN-PORT (D2 structural):** one shift-state service (single writer); preserve external contracts |
| Q-ASGN-NOLOC | active-assignment guard lacks `location_id` predicate (`shifts.ts:135–138`) | **FIX-IN-PORT (D3 belt)** — add `AND location_id=$`; CARRY if strict parity preferred |
| Q-GEOFENCE-SKIP | ping range-check skipped when location has no pin (`shifts.ts:343`) | **CARRY + document (D4)** — "no pin ⇒ no geofence"; accepted-risk row |
| Q-RATELIMIT-KEY | ping rate-limit keys on the full authorization header (`shifts.ts:316`) | **CARRY intent; port to key on `sub`** (D5) — cleaner per-courier bucket post-auth |
| Q-RAW-THROW | `openShift`/`acceptCourierAssignment` throw raw `{statusCode,error}` objects (`shiftService.ts:44`) | **FIX-IN-PORT (D6 error-contract):** typed errors → sendError envelope |
| Q-CROSS-TENANT-REGEN | `/regenerate` runs the worker for ALL locations on the request path (`owner/settlements.ts:314`) | **S5-path-owned; CARRY + accepted-risk row** — idempotent under 085 (redundant sweep = no-op) |
| Q-CRIT1-ROUTE-INFIX | `GET …/orders/:orderId/route` is S7 despite the S5 `/orders/:orderId/` prefix (`couriers.ts:205`) | **CARRY** — route it by the trailing literal `route`; a longest-prefix router cannot separate it (map note) |
| Q-DUP-INVITE-MINT | courier-invite mint exists at BOTH `owner/courier-invites.ts` and `spa-proxy.ts:742` (map rows 96/179) | **CARRY both S7; unify in the port** — two impls of one effect is a security-parity hazard (fix a leak in one, miss the other) |
| Q-PII-MASK | courier history masks customer name (`me.ts:15–27`); owner `/details` returns plaintext customer name/phone (`couriers.ts:277`) | **CARRY the role-tiered masking** — courier is lower-trust (mask); owner is higher-trust (plaintext) — but assert the split visibly (a test per role) |
| Q-EARNINGS-DISPLAY | `/me/earnings` sums `cash_amount` + `tip_amount` (display-only; payout math unchanged, `me.ts:187–223`) | **CARRY verbatim** — informational; not the settlement authority |

## 11. Cutover DoD (REBUILD-MAP, this surface)

Courier E2E slice green (courier-login/shift, active-delivery accept→pickup→deliver, cash-as-proof,
cross-tenant-realtime courier arms, settlements/payouts read) · `openapi-diff` empty for the S7 namespace ·
invariant-cluster red→green:
- **Courier auth/session** — mint via the S2 body-kid minter (cross-verify parity); refresh rotation
  (single-use, family-revoke-on-reuse); **session-liveness: revoke/deactivate/password-change → next REST
  401** (parity with the S6 WS drop); `!jti` on prod → 401.
- **Actor-gate** — a courier accepting/completing another courier's assignment → **404**; a stale-state tap
  → 404; the status-guarded transition anti-race (concurrent double-accept → one wins).
- **Honest dispatch** — no-courier → order stays put (no orphan); the synthetic courier is excluded from the
  roster + admissible only under the ADR-0003 gate.
- **Cash-as-proof** — `paid_full` with `cash≠total` → **422**; refused/cancelled_on_door → order CANCELLED +
  no HOLD; a retried `/delivered` → one HOLD (ON CONFLICT); crypto prepaid-not-paid → 409 (dark).
- **Settlement money** — the port calls `app_generate_settlements` (no Rust re-impl); the courier read equals
  the owner read (byte-for-byte integer amounts); the courier PII redaction (no orderId/customerId) holds;
  the **085 watermark is verified before any settlement apply**.
- **Tenancy** — a live NOBYPASSRLS probe asserts `app.current_tenant=activeLocationId` seated (in one real
  tx) on every courier/settlement read+write; the owner-management writes seat `app.user_id`; the
  within-`couriers.ts` seat drift (`/live`, `/details`) is fixed; `rls-adversarial` courier WHERE-predicate
  gate green.
- **shifts.ts** — the D1 deterministic shift selection (E2E delta: stale offline row not transitioned); the
  D2 single shift-state writer; D6 typed errors.
- **Cutover-concurrency** — cross-stack delivery-completion probe (one DELIVERED/HOLD/trace); session-liveness
  parity across stacks; low-delivery-window schedule co-scheduled with the S6 drain.

map-coverage zero-diff for the S7 namespaces · **council sign-off + rollback plan** (atomic proxy flag-flip
of the whole S7 family back to Node; time-boxed overlap in a low-delivery window). **No 🔴 S7 row builds
before this packet is APPROVED and the 🔴 questions (Q1/Q2/Q3/Q4/Q5/Q6/Q7) are operator-signed.**

---

**council seats to run: breaker, counsel** (architect authored; human operator signs 🔴 Q1 courier
auth/session-liveness / Q2 dispatch actor-gate + honest-dispatch / Q3 courier-money shared-ledger + 085
watermark / Q4 cash-as-proof / Q5 shifts.ts fix-vs-carry / Q6 courier tenancy GUC / Q7 in-flight-delivery
cutover).
**packet-status: 🟡 DRAFT.**

council seats: breaker, counsel
🟡 DRAFT
