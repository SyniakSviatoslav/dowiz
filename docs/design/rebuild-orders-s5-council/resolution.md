# S5-ORDERS/MONEY Port — Council RESOLVE

> **Verdict: PROCEED-WITH-REVISIONS. No ETHICAL-STOP (counsel).** Packet-status: **🟡 — NOT
> COUNCIL-APPROVED until the operator signs §4.** Seats: architect (packet) · breaker
> (1 CRIT / 2 HIGH / 3 MED / 2 LOW) · counsel (PROCEED-WITH-REVISIONS) · lead (this RESOLVE).
> The breaker overturned the packet's central Q3 premise — the RESOLVE corrects it. Heavy linkage
> to `docs/design/rebuild-cutover-harness/` (Q6 request-hash is a shared cutover gate).

## 1. Frozen revision set

- **REV-S5-1 (breaker C1 → CRIT — Q3 create-tenancy INVERTED).** The packet's claim that order-create
  INSERTs need a GUC seated is FALSE: the admitting policy is
  `anonymous_insert … WITH CHECK (app_current_user() IS NULL)` (`app_current_user`=`app.user_id`;
  `migrations/1780315000000_customer-rls.ts:6-22`, `core-identity.ts:70-72`). Anonymous checkout
  passes **iff `app.user_id` is UNSET**. Correct port: the anonymous create path seats **NO**
  `app.user_id` (leave it NULL) — seating a user/tenant GUC there would BREAK it post-B3. Do NOT copy
  the packet's `with_tenant(app.current_tenant)` on create — it is inert (wrong GUC) and hides the real
  invariant. The build must first enumerate the ACTUAL admitting policies for BOTH create paths
  (anonymous vs authenticated-customer, if the latter exists) and match each. The NOBYPASSRLS DoD probe
  must **discriminate**: assert an anonymous create with `app.user_id` NULL is ADMITTED and (defensively)
  that setting `app.user_id` on the anonymous path is REJECTED — not the packet's non-discriminating
  "seating helps" check. Customer-scoped WRITES (cancel) still bind `customer_id` via the S2
  `require_order` seam (that half stands, T-12).
- **REV-S5-2 (breaker H1 → HIGH — request-hash cross-stack drift).** The idempotency request-hash
  embeds `JSON.stringify`-serialized f64 lat/lng (`lib/order-canonical.ts:37-50`); V8 `42` vs Rust ryu
  `42.0`, `-0` vs `-0.0`, unpinned quantity typing → divergent SHA-256 across stacks. A single golden
  vector cannot cover the float domain. REV: define a **canonicalization CONTRACT** provable across both
  stacks — pin the numeric string format (fixed-decimal or rounded-integer coords), or hash over a
  normalized integer projection, and prove it with a property test over the float domain (not one
  vector). De-escalation: this is a duplicate **ORDER**, not a duplicate PAID charge (§9.3: no charge at
  create) — the packet/counsel "duplicate paid order" framing was overstated; correct it. Shared gate
  with cutover-harness Q8.
- **REV-S5-3 (breaker H2 → HIGH — strict-parse drift is broader than messenger_kind).** The Q5b fix
  (`messenger_kind` 3→6) is necessary but INSUFFICIENT: `CreateOrderInput.strict().parse()`
  (`shared-types/src/legacy.ts:72`) also lacks a `receiver{}` key, while the FE sends it top-level
  (`CheckoutPage.tsx:332-334`), the route reads it (`orders.ts:592-594`), and the DB has the columns
  (`mig 1790000000074:33-36`) → "deliver to someone else" (ADR-0016) 400s TODAY. REV: **full FE→schema
  field audit** of the checkout payload; FIX-IN-PORT the whole `CreateOrderInput` to admit exactly what
  the live FE sends (messenger_kind 6-value + receiver + any other drift found), each gated on a
  DB-CHECK/column confirmation.
- **REV-S5-4 (breaker M1 → money math simplify).** i128 is UNNECESSARY: subtotal is int4 (≤2.147e9),
  taxRate is a fraction, so `sub·rateMicro` ≤~2.15e15 « i64 max 9.2e18 (~100× headroom;
  `orders.ts:32-33`). Use **i64** (matches the frozen `Lek(i64)` domain newtype); the "overflow vector"
  DoD is unfalsifiable theater → DROP it. The REAL money guards stay: byte-parity vs the zero-import
  money vectors + the `inclusive ⇒ total = subtotal+fee` property + LC1 `chargedTax = price_includes_tax
  ? 0 : taxTotal`. Counsel's overcharge concern is met by byte-parity, NOT by i128.
- **REV-S5-5 (breaker M2).** Port the idempotency **delete-and-recreate** branch (`orders.ts:406-411`)
  omitted from packet §6.
- **REV-S5-6 (counsel #1 — Potemkin promo, the sharpest ethical point).** CARRY `discountTotal=0` in the
  port (do NOT thread a new customer money-input into the tx being proven byte-identical — S5-T11). BUT
  the accepted-risk row must be **re-scoped from "unbuilt feature" to the truth**: a fully-built, routed,
  guardrail-tested owner Promotions CRM (`PromotionsPage.tsx`, `/owner/promotions`) lets an owner create
  `SUMMER20 −20%`, toggle active, watch a `0/100` counter — with NO redemption runtime (`current_uses`
  never increments; `discountTotal` always 0). The **owner** (Albanian small-biz launch persona) is
  misled by their own tool; the customer sees no promo input at all. Owner + near-term trigger: build
  redemption via its OWN council, OR honestly label/hide the promo surface on Node until then. Not a
  silent Potemkin re-ship.
- **REV-S5-7 (counsel #3 + breaker — 085 watermark).** The `2026-07-10` literal (×3 sites) is a timing
  landmine: apply-early = silent double courier payout (platform money; courier won't complain). Pull
  from a Q7 footnote into a **tracked gate** with a pre-apply assert `literal >= apply_date` across all
  three literals. Out of S5 build scope (S7/settlement), operator-owned, but needs the forcing function.
- **REV-S5-8 (counsel #2 — cutover honesty).** "rollback = flip the flag" is true at the fleet level,
  misleading at the ORDER level — a duplicate order is NOT undone by a flag (human cleanup; cash = a
  second delivery the customer is asked to pay for). Every money-surface flip = separate human go/no-go
  + a **manual cleanup plan written BEFORE the flip**; the request-hash byte-identity probe runs on real
  overlap traffic BOTH directions before flip. Folds into the cutover-harness RESOLVE.
- **REV-S5-9 (register — L1/L2).** L1: port the `ORDER_CONFIRMED/REJECTED` bus folds
  (`orderStatusService.ts:286-290`). L2: pickup `READY→IN_DELIVERY` bypasses honest-dispatch (actor-gate
  covers only CANCELLED, `orderAuthz.ts:11-27`) — CARRY verbatim + register.

## 2. Seat disposition
| Finding | Sev | Disposition |
|---|---|---|
| Breaker C1 | CRIT | ACCEPTED → REV-S5-1 (packet premise inverted) |
| Breaker H1 | HIGH | ACCEPTED → REV-S5-2 (+ framing de-escalated) |
| Breaker H2 | HIGH | ACCEPTED → REV-S5-3 (full schema audit) |
| Breaker M1 | MED | ACCEPTED → REV-S5-4 (i64, drop i128 theater) |
| Breaker M2 | MED | ACCEPTED → REV-S5-5 |
| Breaker M3 | MED | ACCEPTED → REV-S5-2 (framing corrected) |
| Breaker L1/L2 | LOW | REV-S5-9 |
| Counsel Potemkin | — | REV-S5-6 |
| Counsel 085 / cutover / i128-affirm | — | REV-S5-7 / REV-S5-8 / superseded by M1 (i64 safe) |
Confirmed-sound baseline (breaker): state-matrix byte-parity, S2 `require_order` seam exists, mig-086
stack-agnostic, `applyTax` byte-portable, Q5b DB-CHECK admits 6.

## 3. Question resolutions
- **Q1 → (a)** integer i64 money composition + LC1; **discountTotal=0 CARRY with REV-S5-6 re-scope**. 🔴
- **Q2 → (a)** state folds + actor-gate as a layer; REV-S5-9 folds. 🔴
- **Q3 → CORRECTED (REV-S5-1)**: anonymous create seats NO app.user_id; match real admitting policies;
  discriminating probe. Customer cancel binds customer_id (S2 seam). 🔴
- **Q4 → (a)** idempotency + REV-S5-5; request-hash canonicalization = REV-S5-2 (cutover gate).
- **Q5a → defer** sales_channel entity (DB frozen); CARRY `x-channel`→metadata.
- **Q5b → FIX-IN-PORT, BROADENED (REV-S5-3)** — full CreateOrderInput schema, not just messenger_kind. 🔴
- **Q6 → REV-S5-2 + REV-S5-8** (request-hash both-dirs probe + human go/no-go per money flip). 🔴
- **Q7 → 086-before-flip + REV-S5-7 085 tracked gate.** 🔴

## 4. 🔴 OPERATOR SIGN-OFF REQUIRED (blocks COUNCIL-APPROVED → build)
1. **Q3 corrected tenancy** — anonymous create seats no user GUC; discriminating NOBYPASSRLS probe.
2. **Q1** — i64 money + LC1 + discountTotal=0 CARRY with the REV-S5-6 owner-honesty re-scope + trigger.
3. **Q5b broadened** — full checkout-payload schema fix (messenger_kind 6 + receiver + audit).
4. **Q6** — request-hash canonicalization contract + both-directions real-bytes probe + per-money-flip
   human go/no-go (shared with the cutover-harness RESOLVE).
5. **Q7** — mig-086 lands before the S5 flip; the 085 `2026-07-10` tracked pre-apply-assert gate.

## 5. Build/cutover DoD deltas
- Discriminating NOBYPASSRLS create probe (REV-S5-1) · request-hash property test over the float domain +
  cross-stack byte-identity (REV-S5-2) · FE→schema field-audit test (REV-S5-3) · money byte-parity vs
  vectors + inclusive-property, NO overflow-vector (REV-S5-4) · idempotency delete-recreate arm
  (REV-S5-5) · owner-honesty accepted-risk row + trigger (REV-S5-6) · 085 pre-apply assert (REV-S5-7).
