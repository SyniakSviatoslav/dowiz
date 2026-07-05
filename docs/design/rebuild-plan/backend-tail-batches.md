# Backend strangler-tail — batch plan + council packets (R1, 2026-07-05)

Keep-set regenerated at HEAD 6b04a828: **67 routes** { S5:23, S8:17, S3:16, S10:5, S7:3, S2:2, S1:1 }.
Port loop per batch (Ship Discipline + ledger #77/#78 ratchets): port → `#[ignore]` live-PG cargo
suite for the touched routes → deploy `dowiz-rust-staging` → parity probe vs node (status codes AND
body shape; 400-vs-422 class) → regen keep-set → staging E2E → commit. Red-line batches get a
triadic council BEFORE code.

## R2a — CLEAN (no council; port now)
| route | surface | notes |
|---|---|---|
| GET /api/owner/couriers | S7 | token-derived, spa-proxy.ts:452; LATERAL shift + deliveries subquery + SAVEPOINT rating; **PII-parity: port node's exact decrypt/mask behavior, widen NOTHING** |
| POST /api/owner/courier-invites + POST /couriers/invites | S7 | twin mounts, one handler |
| PATCH /api/owner/locations/:locationId | S3 | owner location update; RLS: assert_active_owner_membership FIRST (bind class) |
| POST .../products/:productId/confirm-allergens | S3 | flag confirm; allergen single-source rules (ADR-0014) — no value rewriting |
| GET .../degradation | S3 | read-only |

**NODE-FOREVER (document, do not port):** `GET /branding-preview/:slug` (S1) — SPA-shell fallback;
rust has no SPA shell by design (front-door already keeps SPA fallback unmapped).

## R2b — S5 MONEY batch (23) — COUNCIL PACKET
Scope: **order-actions** assign-courier / deliver / pickup / mark-no-show / reveal-customer-contact
(PII) / verify; **settlements** list/get/approve/dispute/pay/reopen/regenerate; **refunds** list +
sent; **order reads** owner orders list, dashboard snapshot, customer order status, rating;
**messages** get/post/read; **plisio webhook** (crypto, ADR-0017); **promotions (S3-tagged but
pricing-affecting)** CRUD+validate — POTEMKIN-deferred per prior decision, council to confirm.
- Cheap aliases: order-ACTIONS largely alias the PROVEN owner_update_status transition engine
  (h_t note) — council should bless reuse, not redesign.
- OVERLAP with R3: deliver/assign-courier touch the courier-deliver conflict-bool race — one
  council should resolve both (apply_transition conflict-bool must not be discarded).
- Invariants: integer money; exactly-once (request_hash); status-guarded UPDATE anti-race;
  RLS FORCE membership; reveal-contact = PII audit trail; webhook = fail-closed secret.

## R2c — S8 FORCE-RLS batch (17) — COUNCIL PACKET
notifications status/targets(GET+PUT)/test/telegram-connect-init; push owner+customer sub/unsub;
signals list/compute/ack/dismiss; alerts list/ack/ack-all.
- Blocker class: needs SECURITY DEFINER fns → `packages/db/migrations/` = red-line, drafts via
  `docs/design/ci-rust-live-pg/` pattern, **operator-placed** (085-089 precedent).
- Invariants: FORCE-RLS proofs with a REAL second tenant (test-integrity #5); push subscriptions
  carry endpoint PII; DEFINER search_path guardrail (ledger #33).

## R2d — S10 provisioning (5) — COUNCIL PACKET (light)
onboarding POST + onboarding/start; activation status/pickup/publish. Provisioning = tenant
creation; prior port of onboarding status-code parity (#78) applies; publish touches live menus.

## R2e — S2 customer OTP (2) — DEFER-DARK candidate
OTP_ENABLED=false globally (operator freeze, [[otp-disabled-money-fix]]). Recommendation: port
dark behind the same flag at tail-end, council folded into R2b (auth adjacency), no urgency.

## DEFERRED-BY-DESIGN (prior councils/decisions — confirm at R2b council, else stay node-kept at cutover)
menu-import anonymous/preview/commit (provider dependency), brand GET/PUT/generate (money/media +
AI), menu/translate (AI). Strangler pattern allows prod cutover with a non-zero keep-set — node
keeps serving these behind the front-door until their own effort.
