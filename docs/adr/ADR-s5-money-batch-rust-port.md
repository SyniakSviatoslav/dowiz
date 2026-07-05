# ADR — S5 MONEY batch → Rust port (R2b strangler tail)

- Status: **ACCEPTED with RESOLVE-R1 + R2 deltas** (R1: Ламач C1/H1-H3/M1-M6/L1-L3 + Counsel STOP-1/STOP-2; **R2: Ламач
  N1-N5 + Counsel OPEN-1/OPEN-2 + M5 crypto forward-gate** — `docs/design/s5-money-batch-rust-port/resolution.md`).
  Flag-A fix **re-architected** (Decision 2) and **R2-corrected**: lock-order unification is **GLOBAL** (all 5 courier
  paths, not deliver-only — a partial reorder both left and created an AB-BA), the raced-terminal predicate is
  **narrowed** (DELIVERED-by-self is an idempotent replay, not a race), the reconcile is **observably idempotent**, and
  STOP-1 "owner-visible" is **honestly downgraded + gated to S6**. One money-red-line human sign-off remains open
  (operator ratifies the raced-terminal reconcile rule + two-phase surfacing + places M-A/M-B).
- Date: 2026-07-05 (RESOLVE R1 + R2 applied same day)
- Deciders: System Architect (proposal) → Ламач/Counsel RESOLVE (applied) → operator (prod-flip go + M-A/M-B placement)
- Supersedes/relates: does not contradict ADR-audit-fix-money, ADR-deliver-v2-cash-as-proof, ADR-0017 (payments),
  ADR-0004 (owner-token revocation), ADR-0013 (courier realtime authz), ADR-rebuild-cutover-harness,
  ADR-rust-money-newtype. Extends the REV-S5-1..9 resolution (`docs/design/rebuild-orders-s5-council/resolution.md`).
- Full design: `docs/design/s5-money-batch-rust-port/proposal.md`.

## Context

9/10 surfaces serve on Rust; the strangler tail keeps 23 S5 (money) routes on Node behind the front-door
(`docs/design/rebuild-plan/backend-tail-batches.md` §R2b). S5 is the crown-jewel red-line — a port defect is a charge
or state defect. The batch must port at byte-parity (status AND body; 400-vs-422 class) and MUST resolve the held
courier-deliver **conflict-bool race** (reliability-gate flag A, `courier/assignments.rs:1156-1170`) because the
deliver/assign-courier routes overlap it. The 6 sibling S5 routes already ported
(`rebuild/crates/api/src/routes/orders/{mod.rs,pg.rs}`) prove the pattern and the reuse of the `apply_transition`
engine hold at parity on staging (L2/L4/L5/L6/L7 trace).

## Decision

1. **Port strategy — "verbatim-alias port"** (Option A): 1:1 per-route ports onto the existing `OrdersRepo` /
   `Pg*Repo` outcome-enum pattern; transition-bearing order-actions **reuse `apply_transition` unchanged** (bless
   reuse over redesign, packet §R2b); settlements/refunds/messages get small repos following `courier/settlements.rs`.
   Reject "settlement-engine consolidation" (Option B) as premature redesign on a red-line surface (re-opens the
   ledger-#77 sqlx-cast bug class, delays cutover, violates boring-proven / monolith-first).
2. **Flag A — RE-ARCHITECTED by RESOLVE R1 (the Fig-1 draft below is SUPERSEDED).** RESOLVE proved (source-verified)
   that the order row is already locked (H2 — the "take FOR UPDATE" step is a no-op), the promised `409
   ORDER_RACED_TERMINAL` is dead code (C1 — the terminalize fold flips `ca` out of `picked_up`, so the gate 404s
   first), and the live defect is an AB-BA deadlock (H1 — courier deliver locks `ca→o`, cancel/owner-proxy lock
   `o→ca`). The adopted fix, in ONE shared completion primitive both the live courier deliver and the owner-proxy
   deliver call: **(a) GLOBAL reorder to o→ca — RESOLVE R2 (N1): on ALL FIVE courier paths that lock both rows
   (accept/pickup/deliver/cancel/abort), not deliver-only** (a partial reorder both left the AB-BA for the others AND
   created a new deliver-vs-cancel/abort pair); the discipline is "always lock `orders` before `courier_assignments`",
   proven by a deadlock MATRIX probe, not a single pair; **(b) distinguish raced-terminal from not-found with a NARROW
   predicate — RESOLVE R2 (N2)**: `ca.status='delivered'` → idempotent 200 echo (a successful-deliver retry is NOT a
   race — the broad "any terminal" predicate would fabricate a phantom reconcile + false alert); raced-terminal fires
   only on a NON-delivered terminal (order CANCELLED/REJECTED or ca cancelled/rejected); else 404; **(c) durable +
   observably-idempotent cash-truth on raced-terminal** — a same-tx `courier_cash_ledger 'reconcile'` row (M-A) with the
   alert + 409-body **gated on `rowcount=1`** (retry echoes the same `reconcileId`, no second alert — RESOLVE R2 N5) + a
   distinct `409 ORDER_RACED_TERMINAL` the courier UI renders as a human instruction. **"Owner-visible" is honestly
   scoped (R2 N3): the ledger is audit-only (mig 028), the alert is a no-op seam (S6) — the row is durable + auditable +
   courier-instructed-live NOW; the owner-proactive surface is a gated S6/owner-FE deliverable** (STOP-1: the truth is
   durable, not ephemeral-409-only); **(d) structural write-gating** — all post-transition writes live inside the happy
   branch entered only after the locked order is confirmed IN_DELIVERY (H2), with `apply_transition`'s bool retained as a
   defense-in-depth assert. Business rule ratify-gated (operator, money red-line): "first order-status commit wins; the
   loser's physical cash becomes a durable, auditable reconcile obligation (owner-proactive surfacing gated to S6)."
   *(Superseded draft, for the record: "lock-serialize + honor-the-bool + fail-closed-on-race" — a no-op lock + a dead
   409 branch.)*
3. **Promotions — Node-keep + fail-loud (C1′, was POTEMKIN)** — RESOLVE STOP-2: do NOT mount an affirmative
   `{promotions:[]}` stub (it asserts "zero promotions" to a tenant that has them — a data-hiding dark-pattern the
   moment S3 mis-flips to Rust). Keep discount CRUD **Node-served** behind a front-door hard-guard; any Rust mount is
   **fail-loud** (`503 PROMOTIONS_NOT_PORTED`), never affirmative-empty.
4. **TWO forward-only operator-placed red-line migrations now required (was: none)** — RESOLVE C1/H3: **M-A** adds
   `'reconcile'` to `courier_cash_ledger.type` CHECK (STOP-1 durability) **+ carries the obligation-sum guardrail
   (R2 OPEN-2): `'reconcile'` is owner-mediated audit, excluded from any future Σhold — scope hold-sum to `type='hold'`,
   never `type<>x`**; **M-B** status-guards the `app_generate_settlements` DEFINER bump + adds per-location scope +
   **a spillover destination for late in-period earnings (R2 N4 — a bare guard silently underpays; route the earning to
   a mutable payout, never silent-skip)** (H3 self-poison / M6). M-B *modifies* an existing DEFINER (does not add one);
   the webhook resolver `payment_location_by_provider_ref` (mig …083) stays as-is. The route runtime is still
   schema-light, but the money-integrity findings force these two seams. Drafts + owners in `resolution.md` §migrations;
   neither inlined by the port.

## Invariants preserved (non-negotiable)

- Integer money — `::bigint` reads → `domain::Lek` (ledger #77); no `numeric→f64` on a money column.
- Exactly-once — `request_hash` (create) + status-guarded `WHERE id=$ AND status=$expected` (transitions) +
  per-status-guarded settlement UPDATEs + insert-wins ledger (refund/webhook, `ON CONFLICT … DO NOTHING`).
- Anti-race — optimistic status-guard everywhere; pessimistic `FOR UPDATE` with a **global lock order: `orders` before
  `courier_assignments` on ALL FIVE courier paths (accept/pickup/deliver/cancel/abort)**, matching the o→ca
  owner/customer paths (flag-A fix — the GLOBAL reorder closes every AB-BA; a deliver-only reorder both leaves and
  creates one, H1 / R2 N1).
- RLS FORCE, membership-first — assert active-owner membership (JOIN or seated GUC) BEFORE any location-scoped SELECT;
  never trust the baked `activeLocationId` (ADR-0004). Webhook seats `app.current_tenant` inside the write tx.
- PII — reveal-customer-contact writes the `customer_contact_reveals` audit row inside the tenant tx BEFORE returning
  plaintext (audit-before-reveal); decrypt/mask parity, widen nothing; PII off the bus.
- Fail-closed webhook — HMAC bad → 401 (never 200-swallow); crypto off → 404; unknown ref → 200 ack; real error → 500
  (Plisio retries).
- NO_AUTO_DEGRADE — S5 money never degrades silently at the front-door; a failing route surfaces its real error.

## Status-code parity contract (ledger #78)

Node `sendError(400,'VALIDATION_FAILED')` → Rust `validation_failed_400` (never a 422 default). Settlement wrong-state
→ 409. Webhook arms → 401/400/404/200/500 exactly. Refunds-when-off → `{refunds:[]}` (GET) vs 404 (POST). Every arm
probed node-vs-rust.

## Consequences

- **Positive:** minimal blast radius; boring & proven; schema untouched; every money DECISION stays a pure
  unit-tested function; consistent with the 6 already-ported S5 routes; the flag-A fix removes a genuine cross-surface
  money race with a pessimistic-lock + guarded-write pattern the codebase already uses elsewhere.
- **Negative / accepted:** 23 near-mechanical ports; duplicated status-guarded-UPDATE shape across settlements
  (accepted — it mirrors Node, which IS the parity contract); promotions stays a non-zero Node keep-set (accepted —
  strangler permits it); flag-A fix touches LIVE S7 code (mitigated: guarded, per-arm concurrency-probed, L7 re-trace);
  **two operator-placed red-line migrations required (M-A/M-B) — the batch is no longer zero-migration**; **four
  conscious parity-DEVIATIONS** (M1 timeout / M2 no-empty-degrade / M3 no-ciphertext / M6 date-validation — security &
  reliability > byte-parity, each probed at the corrected shape) and **four conscious CARRIES** (M5→S8, L2, L3, L1 —
  latent weaknesses preserved with owners), ledgered in proposal §7a/§7b.
- **Operability:** dark-mount + per-route parity probe + `#[ignore]` live-PG suite wired into CI (the one systemic
  ratchet) + per-tx `statement_timeout` on row-lock paths + explicit operator S5-money prod-flip.

## RESOLVE R1 + R2 outcome (owners in proposal §10; adjudication in `resolution.md`)

**R2 adjudicated (all FIX; validated against HEAD):** N1 global o→ca on all 5 courier paths + deadlock-matrix probe ·
N2 narrow raced-terminal predicate + delivered-replay 200 echo · N3 honest STOP-1 downgrade + gated S6 owner-surface
(Counsel OPEN-1) · OPEN-2 obligation-sum semantics (`'reconcile'` excluded from Σhold) · N4 M-B spillover destination
(reject silent-skip; loud-RAISE is the interim) · N5 observable idempotency · M5 crypto forward-gate. **Counsel R2:**
STOP-1/STOP-2 SATISFIED.

**R1 adjudicated:** C1+STOP-1 REVISE+FIX (durable reconcile + M-A) · H1 FIX (reorder o→ca) · H2 FIX (structural gating) ·
H3 DEFER-FLAG (M-B) · M1/M2/M3/M6-edge FIX (conscious deviations §7a) · M4 FIX (per-column cast table + write-side
probes) · M5 DEFER-FLAG (S8) · L1 ACCEPT · L2 DEFER-FLAG · L3 ACCEPT (R9 downgraded) + DEFER-FLAG (preventive) ·
STOP-2 REVISE (C1′ no lying stub). **Still standing (R5/R6/R7/R9):** webhook GUC-seat-in-tx · settlement
throwing-RLS-in-tx · row-lock statement_timeout (now a conscious deviation, R7) · reveal audit-before-return (now
"preserve", R9). **Human-gated:** operator ratifies the raced-terminal reconcile money rule + places M-A/M-B; product
holds the physical-world-witness open question (R13, framing only).

## Verification (Definition of Done)

- Per route: node-vs-rust parity probe (status + body, 400-vs-422 class) GREEN — **except** the §7a deviations, which
  assert the *corrected* shape (M2 no-empty-degrade, M3 ciphertext-absent, M6 date-400) as documented exceptions.
- `#[ignore]` live-PG cargo probe per route (sqlx bind/decode boundary) GREEN in CI — now including **write-side binds**
  (M4: `amount_minor` int4/nullable; text+CHECK columns bound as text, not enum).
- Flag-A deadlock **MATRIX** probe (R2 N1) — {accept,pickup,deliver,cancel,abort} × {customer_cancel,owner_order_action}:
  **never `503`/`40P01`** on any cell (a deliver-only reorder goes RED on deliver×{cancel,abort} — the exact regression
  caught). Money arm: deliver-wins → 200 + `hold`; cancel-wins → deliver `409 ORDER_RACED_TERMINAL` + `'reconcile'` row
  (CASH → no `refund_due`), **never both**; red→green.
- Raced-terminal replay-idempotency probe (R2 N2 + N5): successful-deliver retry → idempotent 200 echo, no new
  `'reconcile'`, no alert; raced-terminal retry → same `reconcileId`, exactly one alert total; red→green.
- Settlement-generation probe (H3 + R2 N4): approve payout P → deliver one more cash order in P → generate →
  **no `payout immutable` RAISE, other tenants unaffected, the late delivery is EVENTUALLY settled (courier paid) and
  `payout_sums` stays green per payout** (a silent-skip variant is RED); red today, green after M-B.
- STOP-1 owner-surface (R2 N3): the durable `'reconcile'` row + live courier 409 ship now; the owner-proactive surface
  (alert transport + reconcile-queue read) MUST land before the S5-money prod-flip is declared STOP-1-complete.
- Reliability-gate L7 courier-deliver trace re-run GREEN pre/post the race fix.
- **M-A and M-B staged on staging-DB (boot-guard) before the reconcile branch / route 13 are un-gated** (was: "no new
  migration"). Until placed, raced-reconcile and regenerate stay fail-loud/Node-kept.
- Reveal-contact: audit row written before any plaintext returned (probe: failed audit → no PII in response) —
  invariant preserved (R9), widen nothing.
