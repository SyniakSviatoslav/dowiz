# S7-COURIER/DISPATCH Port — Council Packet · THREAT MODEL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Adversarial input to the S7 council. Assets, trust boundaries, and the
> failure modes the Rust port must not silently introduce — on the surface where a *lower-trust* principal
> (the courier) drives physical fulfilment of a paid order, collects cash, and reads its own pay. Read
> alongside `proposal.md`. Docs only; no code.

- **Method:** STRIDE-lite over the S7 courier/dispatch surface + fold-in of the courier invariants
  (ADR-0013 read/WS authz, ADR-0003 dev-gate, ADR-deliver-v2-cash-as-proof, ADR-audit-fix-money M-2/085), the
  S2 body-kid minter + session-liveness (S6 REV-S6-2), the S5 `updateOrderStatus` funnel, and the
  cutover-concurrency class where **both stacks serve courier REST over one authoritative DB**.
- **Scope note:** the B3 (NOBYPASSRLS) flip and the policy search_path pin are **B3-council fixes**; recorded
  here because they change what S7 must hold, but their *fix* lives in that council. The **settlement
  generation fn + 085** and the **offer-sweep/dispatch workers** are **S8/DB-migration** concerns; S7 *reads*
  the settlement ledger, *calls* the DEFINER fn through the owner proxy, and *shares* the assignment machine.
  The **WS transport/fan-out** is **S6**; S7 *publishes* to the bus and completes deliveries over REST
  regardless of the tail.

---

## 1. Assets

| ID | Asset | Where it lives | Why it matters |
|---|---|---|---|
| C1 | The **courier access token + session** (`courier_sessions`: `jti`, `revoked_at`, `expires_at`, `family_id`) | `courier_sessions` (auth table) + the RS256 body-kid JWT | A 14-day/24 h bearer that authorizes GPS, delivery completion, cash collection, and the courier's own pay; a logged-out/deactivated courier must lose it *immediately*, not on expiry |
| C2 | The **assignment binding** (`courier_assignments`: status, courier_id, shift_id, cash_amount) | `courier_assignments` (tenant, RLS) | The link between a courier and a paid order; a cross-courier write is a delivery hijack; a stranded binding orphans an order or blocks a shift close |
| C3 | The **shift state** (`courier_shifts`: status, last_heartbeat_at) | `courier_shifts` (tenant, RLS) | Availability + liveness; a bad transition blocks go-offline or corrupts the `on_delivery`/`available` dispatch guard; the heartbeat is the worker-liveness signal |
| C4 | The **cash-as-proof HOLD** (`courier_cash_ledger` type='hold', amount) | `courier_cash_ledger` (tenant, RLS) | The courier's till-accountability bond = the money the courier owes the till; a missing/duplicated hold is a reconciliation loss |
| C5 | The **settlement ledger** (`courier_payouts.total_earned`, `settlement_items.amount`) | `courier_payouts`/`settlement_items` (tenant, FORCE RLS) — integer minor units | The money the owner owes the courier; **shared with S5** (owner reads/writes the same rows); a double-generated payout pays the same cash twice (085), a divergent read shows the courier a different amount than approved |
| C6 | The **delivery attestation** (`orders.payment_outcome`, `delivery_trace`) | `orders`/`delivery_trace` (tenant, RLS) — immutable trace | The server-authoritative record of what happened at the door (paid_full / refused / prepaid); a mis-mapped outcome tells the customer "Delivered" for refused food |
| C7 | The **courier GPS trail** (`courier_positions`: lat/lng, shift_id) | `courier_positions` (tenant, RLS) | Location data of a person; the P0-1 gate stores it ONLY on an active delivery (consent boundary); a leak or off-delivery write is a privacy breach |
| C8 | The **customer PII the courier touches** (`customers.phone`, address, messenger) | `customers`/`orders` (tenant, RLS) | The courier is lower-trust: history masks it, active-task exposes phone/address only while the task is live; a widened read is a roster/PII leak |
| C9 | The **tenant boundary on courier writes** (`app.current_tenant`=activeLocationId GUC) | txn-scoped GUC + RLS policies | The service-root seat that binds a courier/settlement write to the right tenant under FORCE RLS |

## 2. Trust boundaries

- **TB-1 courier → REST (`/api/courier/*`)** — a lower-trust authenticated principal. Every request runs
  `verifyAuth` (JWT crypto via the S2 verifier) **then** the **session-liveness re-read** (`courierSessionValid`
  against `courier_sessions` by `jti` + membership in `activeLocationId`). The bind is what makes revocation /
  deactivation / password-change / refresh-rotation immediate; **without it the crypto-valid JWT is a 14-day
  bearer** (S7-T3).
- **TB-2 courier → assignment (`/assignments/:id/*`)** — RLS isolates by *location* only; the **`AND
  courier_id=$` predicate** is the intra-tenant boundary between couriers. Without it any courier in the
  location can accept/complete/cancel another's assignment (S7-T1). The `status=$expected` predicate is the
  anti-race boundary.
- **TB-3 anonymous → courier mint (`/auth/invites/:id/redeem`, `/auth/login`, `/auth/refresh`)** — the JWT
  front door. The invite code (argon2), the password (argon2, timing-safe dummy verify), and the refresh
  token (`sessionId.plain`, argon2 hash, family reuse-detection) are the gates. A minter that drifts from the
  S2 body-kid verifier breaks cross-verification (S7-T2).
- **TB-4 owner → courier management (`/couriers*`, `/courier-invites*`)** — owner-membership + `requireRole
  (['owner'])` + `requireLocationAccess`. Seats the **owner root** (`app.user_id`). The F3/F4 fixes (a missing
  `requireRole` admitted a customer/co-worker) are carried. An invite may mint ONLY role `courier`
  (`courier-invites.ts:34`).
- **TB-5 courier/owner → settlement ledger (shared, C5)** — courier reads `/me/payouts` (service root); owner
  reads/writes `/settlements*` (S5). The **novel boundary within the money surface**: two roles, two flips
  (S7 read / S5 write), one ledger. A read that diverges from the write, or a seat that errors post-B3, breaks
  it (S7-T8/T10).
- **TB-6 dev → synthetic courier (`/dev/mock-auth`, ADR-0003)** — a dev-only impersonation of ONE synthetic
  identity, gated by `ALLOW_DEV_LOGIN` + `x-dev-auth-secret`, fail-closed 404 on prod, excluded from the owner
  roster. The boundary between a real courier and a test fixture; a leak (or a `!jti` token admitted on prod)
  is fake dispatch (S7-T5).
- **TB-7 stack → stack (cutover)** — during the overlap a courier's delivery may be picked-up on Node and
  delivered on Rust; a courier request may hit either stack. Trust is mediated **only** by the shared DB
  invariants (the same tables, the same `courierSessionValid` predicate, the same `completeDelivery` ON-CONFLICT
  folds, the stack-agnostic settlement fn). A divergent session-liveness or completion fold across stacks
  breaks it (S7-T13).

## 3. Port-specific failure scenarios (Rust)

| # | Scenario | Trigger in the port | Mitigation to prove red→green |
|---|---|---|---|
| **S7-T1** | **Cross-courier assignment hijack** — courier B accepts/completes/cancels courier A's delivery | Dropping the `AND courier_id=$` predicate (RLS isolates only by location) | Carry `WHERE id=$ AND courier_id=$ AND status=$ FOR UPDATE` on every mutation; E2E: courier B accepting A's assignment → **404**; the status-guarded anti-race (concurrent double-accept → one wins, the other 404) |
| **S7-T2** | **JWT-parity break** — a Rust-S7 courier token fails to verify under the S2 verifier (REST/WS), or vice-versa | Giving S7 its own courier JWT minter with drifted kid handling | REUSE the S2 body-kid `signAuthToken`/`verifyAuthToken` port; golden cross-verify test (mint on S7 → verify with the S2 verifier the WS admission uses, both directions) |
| **S7-T3** | **Logged-out courier keeps a live tail** — a revoked/deactivated/password-changed courier keeps 14-day access to GPS, delivery, cash, and its pay | Carrying the JWT-crypto-only path without the `courierSessionValid` per-request re-read | Carry the session-liveness bind (no cache); E2E: logout / owner-deactivate / password-change → the next courier REST call is **401**, parity with the S6 WS drop (REV-S6-2) |
| **S7-T4** | **Refresh-token reuse / family compromise** — a stolen refresh token mints access indefinitely | Dropping the single-use rotation, the family reuse-detection→revoke, or the argon2 token_hash | Carry `FOR UPDATE NOWAIT` + rotate-on-use + reuse→`UPDATE … WHERE family_id` revoke; E2E: replay an already-rotated refresh → **401 REFRESH_REUSED** + the whole family revoked |
| **S7-T5** | **Fake dispatch** — a fabricated/synthetic courier appears assignable, or an order reaches IN_DELIVERY with no courier | Dropping the synthetic-courier roster exclusion / the ADR-0003 gate; or advancing to IN_DELIVERY before finding a courier | Carry `AND c.email_hash <> SYNTHETIC` exclusion + the dev-gate fail-closed-on-prod; carry honest-dispatch find-then-advance (no courier → stay put); E2E: no available courier → order does NOT advance |
| **S7-T6** | **Settlement double-pay via Rust re-impl** — a re-implemented aggregation re-opens the C2 loss/double-count paths 085 closed | Porting the settlement math into Rust instead of calling `app_generate_settlements` | The port issues `SELECT app_generate_settlements($1,$2)` only; the money math stays in the DEFINER fn; settlement idempotency proven at the DB-fn layer (`settlements-catchup.test.ts`), not re-derived in Rust |
| **S7-T7** | **085 watermark double-pay** — a settlement apply that slips past 2026-07-10 sweeps already-reconciled cash into a fresh pending payout | The rebuild schedule pushing the settlement apply past the baked-in watermark literal without bumping it | Operator timing gate: verify the watermark literal ≥ the actual apply date; bump all three occurrences if the apply slips; erring LATE is safe, erring EARLY double-pays. Surfaced as an operator-owned schedule landmine |
| **S7-T8** | **Divergent money display** — the courier sees a different payout amount than the owner approved | Forking a separate courier payout-read shape from the owner read | ONE canonical payout DTO, role-scoped projection over the SAME `courier_payouts`/`settlement_items` rows; parity test: courier read integer amounts == owner read for the same payout |
| **S7-T9** | **Courier PII over-exposure** — a courier receives plaintext customer/order data beyond the active-task window | Dropping the `me.ts` masking (`maskStr(customer_name)`) or the active-status-gated messenger/photo exposure | Carry the role-tiered redaction: history masks the name; active-task exposes phone/address/messenger ONLY while status ∈ {assigned,accepted,picked_up}; settlement items carry no orderId/customerId (`settlements.ts:89`); test per role |
| **S7-T10** | **Wrong/broken tenant seat on courier money reads** — the settlement/live/details reads match 0 rows or ERROR post-B3 | Carrying the pool `set_config(…,true)`+no-`BEGIN` seat (settlements.ts) / the no-`BEGIN` seat (`couriers.ts /live`) / the no-seat bare pool (`/details`) | Route each through `with_tenant(activeLocationId)` in one real tx; NOBYPASSRLS probe (the `settlement_items` bare-`current_setting` policy hard-ERRORS, not just 0-rows) |
| **S7-T11** | **Cash-proof bypass / partial handover** — a courier marks paid_full without the full cash, or over-collects | Relaxing `cash===total` to `>=`/`<=`, or skipping the assert | Carry the exact equality → **422 CASH_AMOUNT_MISMATCH** before any mutation; the HOLD writes only `paid_full` with `cash===total`; refused/cancelled → CANCELLED + no HOLD |
| **S7-T12** | **Shift-state corruption / go-offline block** — an arbitrary-row shift transition mis-states availability, blocking go-offline or corrupting the on_delivery guard | Carrying the `shifts.ts` D1 defect (shift select with no status/ORDER BY/LIMIT → arbitrary `rows[0]`) | FIX-IN-PORT: deterministic single-row selection (today's row, ORDER BY started_at DESC LIMIT 1 FOR UPDATE); consolidate the three shift writers into one service; E2E: a stale offline row is not the one transitioned |
| **S7-T13** | **Cutover divergence** — a delivery picked-up on Node and completed on Rust records a different HOLD/trace/outcome; or a revoked courier is admitted on one stack | A divergent `completeDelivery` fold or `courierSessionValid` predicate across stacks; a non-atomic flip | Byte-equal completion folds (ON CONFLICT HOLD/trace); shared session-liveness predicate; atomic per-surface flip in a low-delivery window (REV-S6-5); cross-stack completion probe (pickup X → deliver Y → one DELIVERED/HOLD/trace) |
| **S7-T14** | **GPS off-delivery / privacy regression** — a courier's location is stored while idle, or a pin-less location leaks unbounded GPS | Silently re-adding `'assigned'` to the active set (courier-gps.ts warns against this), or removing the P0-1 on-active-delivery gate | Carry `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES=['accepted','picked_up']` (consent boundary, DEV-3); position stored ONLY on active delivery; the best-effort geofence SAVEPOINT never fails the ping; document the pin-less "no geofence" accepted-risk |
| **S7-T15** | **Cross-tenant settlement blast** — an owner at location A triggers the settlement sweep for ALL tenants | Carrying `/regenerate`'s all-locations worker call (owner/settlements.ts, S5-path-owned) unbounded | S5-path-owned; accepted-risk with an owner — idempotent under 085 (a redundant cross-tenant sweep is a no-op, NOT a double-pay, once 085 lands). Do not "optimize" it into a per-location call without confirming the fn's period-label semantics |

## 4. What the B3 RLS flip changes for S7

- **Today (BYPASSRLS):** RLS is bypassed; the explicit `WHERE courier_id/location_id` predicates + the
  actor-gate are the only live boundaries. The **broken/missing seats "work"** — the settlement read (pool +
  is_local + no BEGIN), the `/couriers/live` read (no BEGIN), the `/couriers/:id/details` read (no seat), and
  the owner settlement reads all rely on BYPASSRLS + the WHERE predicate. The danger is invisible.
- **Post-flip (NOBYPASSRLS):** RLS is authoritative. Courier/settlement reads+writes need `app.current_tenant`
  for FORCE RLS; the `settlement_items`/`settlement_audit_log` policies use a **bare `current_setting
  ('app.current_tenant')::uuid` (no missing-ok NULLIF)**, so a context-free access does **not** just see 0
  rows — it **ERRORS** (documented at `settlement-cron.ts:36`). The broken seats detonate (S7-T10). The owner
  management writes need `app.user_id` seated for the memberships-derived policy. **The mig-085 settlement
  fns' cross-tenant writes remain pre-B3 (DEFINER, owner BYPASSRLS) — a B3-flip checklist item, NOT changed
  by S7.**
- **S7's rule:** every courier/settlement access is correct **independent of which pool role is live** (belt =
  explicit predicate; suspenders = seated GUC in one real tx); the completion folds + the session-liveness
  predicate are byte-identical across stacks so the B3 flip and the Node→Rust flip are two orthogonal,
  independently-reversible events. The `courier_sessions` liveness read stays a privileged auth read (not
  `with_tenant`).

## 5. Residual risks (summary for the human)

- **The courier session-liveness gap if carried forward (S7-T3 / Q1c)** — a logged-out/deactivated courier
  keeping a 14-day live tail over their money + the customer GPS. The port is the natural moment to close it
  (the ADR-0004 owner-P-d pattern + the S6 REV-S6-2 predicate already exist). **The most likely breaker
  escalation** — the council should have the breaker attack a revoked-session-still-live scenario across both
  stacks. Owner: S7 lead + S2 lead.
- **The 085 watermark timing (S7-T7 / Q3 / Q7)** — not an S7 build dependency, but a shared timing landmine: a
  settlement apply that slips past 2026-07-10 double-pays already-reconciled cash unless the operator bumps
  all three literals. Surfaced so the rebuild schedule cannot silently trip it. Owner: operator.
- **The `/regenerate` cross-tenant blast (S7-T15 / Q3)** — an owner running the settlement sweep for ALL
  tenants. Defensible as idempotent-under-085 (a redundant sweep is a no-op), but must be an **explicit
  accepted-risk with a named owner**, not a silent carry. **A likely counsel flag.** Owner: operator (product)
  + S5 lead (the route is S5-path-owned).
- **The courier-vs-owner PII split (S7-T9 / Q-PII-MASK)** — the owner `/details` returns plaintext customer
  name/phone while the courier history masks it. A defensible role-tier decision (owner higher-trust), but the
  counsel should **ratify the split**, not have the port inherit it silently. Owner: counsel + S7 lead.
- **The GPS consent boundary (S7-T14 / C7)** — the P0-1 on-active-delivery-only storage + the `['accepted',
  'picked_up']` set is a privacy red-line; the `courier-gps.ts` comment explicitly warns a future refactor
  must not re-add `'assigned'`. Carry it visibly (matrix row + test); the pin-less "no geofence" property is a
  documented accepted-risk, not a fix. Owner: S7 lead.
- **The `withTenant(db, ownerId)` naming trap (Q6)** — the Node combinator seats `app.user_id` despite the
  "tenant" name; the port must resolve it with the S3 REV-10 non-confusable `UserId`/`TenantId` types so an
  owner-management write is never mis-seated. Accepted as a *rename-in-port*, not a behavior change. Owner: S7
  lead.

**None of C1–C9's failure modes is *introduced* by the rewrite** — each (session bind, actor-gate,
cash-as-proof, settlement idempotency, tenancy seat, GPS consent, PII redaction) is a **current** property the
port must carry **visibly** (matrix row + test). The rewrite's *new* risks are the **cutover concurrency of an
in-flight delivery** (S7-T13, TB-7) and the **JWT-parity coupling to the S2 verifier** (S7-T2) — neither of
which a prior single-stack packet faced. **Breaker-escalation candidate: the carried-forward session-liveness
gap (S7-T3) across two stacks.** **Counsel-flag candidate: the `/regenerate` cross-tenant blast (S7-T15) + the
courier/owner PII split (S7-T9)** — both defensible only as explicit, owned accepted-risks, never by silence.

council seats: breaker, counsel
🟡 DRAFT
