# S7-COURIER/DISPATCH Port — Council Packet · OPEN QUESTIONS

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Everything the live Triadic Council (architect / breaker / counsel /
> human) must decide before S7 courier/dispatch is ported. Each question has options + a lane-R3
> recommendation — a *starting position for friction*, not a decision. S7 composes courier auth (the
> S2-parity obligation outside S2's gate), the dispatch/assignment state machine, and courier money
> (cash-as-proof + a settlement ledger shared with S5 + the 085 double-pay landmine). Docs only.

Legend: **[AUTH]** courier auth/JWT/session · **[STATE]** dispatch/assignment lifecycle · **[MONEY]**
courier money/settlement · **[SEC]** security/tenancy · **[HEALTH]** worst-file remediation ·
**[INFRA]** cutover/topology · **[MIGRATION]** DB-draft interaction. 🔴 = red-line, operator sign-off
required.

---

### Q1 🔴 [AUTH] Courier auth/JWT + session-liveness — reuse the S2 minter, carry the per-request bind
`courier/auth.ts` (path-owned S7) mints body-kid RS256 JWTs via the SAME `signAuthToken` S2 owns, runs a
refresh-rotation family machine (reuse→revoke family), and every courier request re-reads `courier_sessions`
by `jti` (`courierSessionValid`: present ∧ ¬revoked ∧ ¬expired ∧ still-a-member). CourierClaims =
`{sub, role:'courier', activeLocationId, jti}`.
- **(a) REUSE the S2 body-kid minter/verifier for courier tokens (no second impl) + CARRY the per-request
  session-liveness bind (the same S6 REV-S6-2 predicate, no cache) + port the refresh-family machine
  faithfully + keep the `!jti` dev-gate fail-closed on prod.** *(recommend)*
- **(b) Give S7 its own courier JWT minter** — rejected: a kid-handling drift breaks cross-verification (a
  Rust-S7 courier token must verify under the S2 verifier that REST `verifyAuth` and the S6 WS admission both
  use); two minters is the JWT-parity break threat (S7-T2).
- **(c) Verify the JWT only, skip the session re-read** (trust the 14-day/24 h expiry) — rejected: a
  logged-out / owner-deactivated / password-changed courier keeps a live tail for up to 14 days (the exact S6
  WS-T8 gap, here on REST + the courier's own money/GPS surface).

**R3 recommendation:** (a). The minter is S2's; the session-liveness is the ADR-0004 owner-P-d pattern applied
to couriers and the S6 REV-S6-2 predicate — one shared check for REST + WS. **🔴** because (i) a second minter
is a silent cross-verify break and (ii) dropping the bind is an immediate-revocation failure on a lower-trust
role. **This couples S7's flip to the S2 verifier's cutover re-ratification** — named, not re-opened. Owner:
S7 lead + S2 lead + operator.

### Q2 🔴 [STATE] Dispatch/assignment state — the actor-gate, honest-dispatch, the single-mutator funnel
The lifecycle (offered/assigned/accepted/picked_up → delivered/cancelled/rejected/offered_expired) is driven
by courier-scoped, status-guarded `FOR UPDATE` mutations; honest-dispatch finds a courier before advancing to
IN_DELIVERY (no orphan, no fake courier); every order-side transition funnels through S5's `updateOrderStatus`.
- **(a) CARRY the actor-gate (`AND courier_id=$ AND status=$ FOR UPDATE`) on every mutation; CARRY
  honest-dispatch's find-then-advance ordering + the synthetic-courier exclusion; keep the single-mutator
  funnel (S7 owns assignment/shift writes, S5 owns the order write via `updateOrderStatus`); carry the shared
  cancel/abort release-and-reoffer rail; carry both accept branches flag-guarded.** *(recommend)*
- **(b) Let S7 hand-`UPDATE orders.status` on the delivery paths** — rejected: re-opens the C-2 trap the
  shared rail closed (a raw exit that lies about the resulting order state); the folds (R2-3, L-A) drift.
- **(c) Advance to IN_DELIVERY first, assign a courier after** — rejected: an order at IN_DELIVERY with no
  courier is an orphan with no recovery affordance (the no-trap red-line F1).

**R3 recommendation:** (a). The machine (S5) says *possible*; the actor-gate says *this courier, this state*;
honest-dispatch says *a real available courier or nothing*. **🔴** because a dropped `courier_id` predicate is
a cross-courier hijack (S7-T1), a dropped find-then-advance is an orphaned paid order, and a dropped
synthetic-exclusion is fake dispatch (S7-T5). Owner: S7 lead + operator + breaker (attack the cross-courier +
the accept-vs-sweep race).

### Q3 🔴 [MONEY] Courier settlements/payouts — shared S5 ledger, the DB-fn boundary, the 085 watermark
Courier `/me/payouts` reads the SAME `courier_payouts`/`settlement_items` the owner reads (S5); the write
authority (pending→approved→paid) is owner (S5); generation is the SECURITY-DEFINER DB fn
`app_generate_settlements`, which migration draft 085 rewrites to catch-up/idempotent/immutable behind a
`2026-07-10` watermark. Integer money end-to-end.
- **(a) The port CALLS the DB fn (never re-implements settlement math in Rust); keeps ONE canonical
  payout-read DTO with a role-scoped projection (courier keeps the stricter PII redaction); FIXES the broken
  settlement-read tenancy seat (`with_tenant` in one real tx); and SURFACES the 085 watermark as an
  operator-owned timing gate (bump the three literals if the apply slips past 2026-07-10).** *(recommend)*
- **(b) Re-implement the settlement aggregation in Rust** — rejected: re-opens every C2 double-pay path 085
  closed (SKIP-LOCKED loss, phantom-count, non-idempotent re-run); the money engine is deliberately Postgres.
- **(c) Fork a separate courier payout-read shape** — rejected: a courier seeing a different amount than the
  owner approved is a trust-breaking money bug; keep one DTO.

**R3 recommendation:** (a). "Schema-rich, runtime-minimal": the money is in the DB fn; the port is a thin
caller. **🔴** on (i) the 085 watermark (a double-pay hazard the operator owns — erring early double-pays old
cash rows) and (ii) the broken settlement-read seat (post-B3 the `settlement_items` bare-`current_setting`
policy ERRORS, not just 0-rows). Owner: operator (watermark) + S7 lead (DTO + seat) + S5 lead (owner-side
write parity).

### Q4 🔴 [MONEY/STATE] completeDelivery + cash-as-proof — the no-partial-handover rule
`completeDelivery` is the single completion primitive: `paid_full` requires `cash===total` → 422; the honest
no-cash tail (refused/cancelled_on_door) → order CANCELLED + no HOLD; the cash-as-proof HOLD ledger (ON
CONFLICT); the crypto prepaid auto-resolve (409 if not-yet-paid, dark).
- **(a) CARRY every clause verbatim** — the exact `cash===total` equality (not `>=`); the outcome→status map
  (refused ⇒ CANCELLED, never DELIVERED); the idempotent HOLD (ON CONFLICT `(order_id,type)`); the crypto
  auto-resolve dark; the immutable `delivery_trace` (ON CONFLICT); the `refund_due` obligation delegated to
  the S5 L-A fold (run inside the tenant-seated tx). *(recommend)*
- **(b) Relax `cash===total` to `cash>=total`** — rejected: partial-handover is the exact money-leak the
  cash-as-proof rule closes; over-collection is a customer dispute the HOLD cannot reconcile.

**R3 recommendation:** (a). **🔴** because a bypassed cash assert is a partial-handover money leak (S7-T11) and
a mis-mapped outcome tells the customer "Delivered" for refused food + writes a phantom settlement row. Owner:
S7 lead + operator.

### Q5 🔴 [HEALTH] `shifts.ts` worst-health — which latent defects FIX-IN-PORT vs CARRY
`courier/shifts.ts` is the repo's worst-health file (1.0/10). Audited defects (proposal §7): D1 arbitrary-row
shift selection (no status/ORDER BY/LIMIT); D2 three divergent shift-mutation surfaces; D3 active-assignment
guard lacks `location_id`; D4 geofence skipped on pin-less location; D5 rate-limit keyed on the raw
authorization header; D6 raw `{statusCode,error}` throws; D7 possibly-null tenant seat; D8 best-effort sensor
SAVEPOINT.
- **(a) FIX-IN-PORT the 🔴 correctness/structure/guard defects with documented E2E deltas** — D1
  (deterministic single-row selection matching `openShift`), D2 (one shift-state service / single writer), D6
  (typed errors → envelope), D7 (non-null `TenantId`). **Low-risk FIX** D3 (add `location_id` belt), D5 (key
  on `sub`). **CARRY + document** D4 ("no pin ⇒ no geofence" accepted-risk), D8 (correct pattern). *(recommend)*
- **(b) CARRY all defects verbatim for strict parity** — rejected: re-ships the worst-health file's arbitrary
  shift-row bug through a deliberate rewrite; the port is the one clean moment to collapse three shift writers
  into one.

**R3 recommendation:** (a). D1/D2 are the score-drivers — collapsing three shift writers into one deterministic
service is the highest-leverage correctness fix in S7. **🔴** on D1 (an arbitrary-row transition can
mis-state a courier's shift, blocking go-offline or corrupting the on_delivery guard). Owner: S7 lead +
operator (D1 E2E delta sign-off).

### Q6 🔴 [SEC] Courier tenancy GUC — service root for courier, owner root for owner-management, fix the drift
Courier self-actions seat `app.current_tenant=activeLocationId` (service root — courier is NOT an
owner-membership; `courier_tenant_update` keys on it); owner-side courier management seats `app.user_id`
(owner root, via the mis-named `withTenant(db, ownerId)`). Several reads have broken/missing seats.
- **(a) CARRY the two-root split with the S3 REV-10 non-confusable `UserId`/`TenantId` types (so an owner
  write is never mis-seated as a tenant write, and the `withTenant(db, ownerId)` naming trap is resolved);
  FIX-IN-PORT the broken/missing seats** — courier settlements (pool + is_local + no BEGIN), `owner/couriers`
  `/live` (no BEGIN) + `/details` (no seat), owner settlements reads — routing each through the correct
  combinator in one real tx with a NOBYPASSRLS probe; keep the belt-and-suspenders `WHERE courier_id/location_id`
  predicates; carry the `courier_sessions` liveness read as a privileged auth read (not `with_tenant`).
  *(recommend)*
- **(b) Carry the broken seats verbatim (rely on BYPASSRLS)** — rejected: the never-copy leak class; the
  instant B3 flips, the settlement/live/details reads ERROR or match 0 rows (the `settlement_items` policy
  hard-errors).
- **(c) Seat `app.user_id` on courier self-actions** — rejected: wrong root (a courier has no owner
  membership); matches 0 rows under FORCE-RLS.

**R3 recommendation:** (a). Courier self = service root (`app.current_tenant`); owner-management = owner root
(`app.user_id`). **🔴** — the broken seats are a B3-readiness correctness fix on the courier money + roster
reads, and the naming trap is the exact S3 REV-10 confusable. Owner: S7 lead + operator + B3-council (the
policy search_path pin the courier reads inherit).

### Q7 🔴 [INFRA/MIGRATION] Cutover — in-flight delivery crossing the flip + the 085 watermark timing
accept/pickup/delivered/cancel are stateless REST over an authoritative DB; the WS tail is S6 (drains on the
S6 low-delivery-window schedule, REV-S6-5); settlement generation is a stack-agnostic DB fn; money is NOT one
atomic surface (owner settlement writes S5, courier reads S7).
- **(a) Atomic per-surface flip of the whole S7 REST family (S3 REV-7), scheduled in the SAME low-delivery
  window as the S6 drain (REV-S6-5, operator sign-off — not prose); gate on a cross-stack delivery-completion
  probe (pickup on X, deliver on Y → one DELIVERED/HOLD/trace) + session-liveness parity across stacks +
  payout-read parity with the owner read; verify the 085 watermark BEFORE any settlement apply; rollback =
  proxy flag-flip; keep crypto + grace-cancel dark.** *(recommend)*
- **(b) Hard-flip S7 REST outside a low-delivery window** — rejected: maximizes couriers mid-delivery at the
  flip; the S6 council already bound the low-delivery-window constraint onto the courier surface.
- **(c) Block S7 on the S5/S6 flips being simultaneous** — rejected: money is not one atomic surface; the
  S5/S7 flips are independent, bounded by the shared payout-read/write parity gate, not by co-timing.

**R3 recommendation:** (a). S7 is mostly stateless-HTTP over an authoritative DB (the good case) — the risk is
concentrated in the in-flight delivery + the shared session-liveness + the 085 timing. **🔴** on (i) the
low-delivery-window schedule (inherited S6 canon) and (ii) the 085 watermark verified pre-apply (erring early
double-pays regardless of stack). Owner: architect + operator + S6 lead (co-schedule) + breaker (attack the
cross-stack delivery completion + the settlement timing).

---

## Decision-ordering note for the council
**Q1 (auth/session)**, **Q2 (dispatch state)**, **Q3 (courier money)**, and **Q4 (cash-as-proof)** are
**port-blocking** — no S7 write builds before all four are settled, because they define the load-bearing
seams (the session bind, the actor-gate, the settlement-fn boundary, the cash assert). Decide them first.

**Q5 (shifts.ts)** is **build-shaping, not blocking** — the FIX-IN-PORT deltas (D1/D2/D6/D7) are internal to
the shift surface; decide the fix-vs-carry line before writing that handler, but it does not gate the rest of
S7.

**Q6 (tenancy)** is **port-blocking for the affected reads** (settlement/live/details) and **B3-coupled** —
the seat fixes must land with the read handlers.

**Q7 (cutover)** is **cutover-blocking, not build-blocking** — the Rust code can be built + dark-verified
before it settles, but the **flip** cannot happen until the cross-stack probe + session-liveness parity are
green, the low-delivery-window is scheduled (with S6), and the 085 watermark is verified.

**The single most likely breaker escalation:** the **courier session-liveness gap carried forward** (Q1c) —
a logged-out/deactivated courier keeping a 14-day live tail over their own money + the customer GPS is the S6
WS-T8 threat replayed on REST; the port is the natural moment to close it, and CARRYing the JWT-only path
would re-ship it. **The runner-up:** the **085 watermark timing** (Q3/Q7) — a settlement apply that slips past
2026-07-10 double-pays old cash rows regardless of how carefully the port is written.

**The single most likely counsel flag:** the **`/regenerate` cross-tenant blast** (Q3, Q-CROSS-TENANT-REGEN)
— an owner at location A running the settlement sweep for ALL tenants is defensible as idempotent-under-085
but must be an explicit accepted-risk with an owner, not a silent carry; and the **courier-vs-owner PII split**
(Q-PII-MASK) — the owner `/details` returning plaintext customer name/phone while the courier history masks it
is a role-tier decision the counsel should ratify, not the port inherit silently.
