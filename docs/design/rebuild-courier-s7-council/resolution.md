# S7-COURIER/DISPATCH — Council RESOLVE

> **Verdict: PROCEED-WITH-REVISIONS. No ETHICAL-STOP (counsel).** Packet-status 🟡 — NOT
> COUNCIL-APPROVED until operator signs §3. Seats: architect (packet) · breaker (1 CRIT / 3 HIGH /
> 3 MED / 1 LOW + 4 verified-negatives) · counsel (PROCEED-WITH-REVISIONS) · lead (this RESOLVE).
> Root theme: the packet's tenant-seat census was INCOMPLETE and affirmatively mislabeled two
> seat-broken read surfaces as correct — the port must seat the complete set.

## 1. Frozen revision set

- **REV-S7-1 (breaker CRIT-1 + HIGH-2 — incomplete tenant-seat census).** Multiple courier READS are
  bare-pool / seat-without-BEGIN in the old stack and 404 or silently-0 post-B3: `courier/assignments.ts:110`
  (`GET /assignments/:id` sets `app.current_tenant` with NO `BEGIN` → discarded → 404 on the courier's
  own active assignment), all four `courier/me.ts` reads (`/me:40`, `/me/audit-log:97`, `/me/earnings:181`
  → silent "0 earned", `/me/history:252`). The packet's §8 "carry the family, writes correctly seat"
  is FALSE for reads. REV: enumerate the **COMPLETE** set of courier reads+writes and route EVERY one
  through the proper `with_tenant` transaction — Rust `with_tenant` is transactional by construction, so
  these bare-pool/no-BEGIN bugs are **FIXED-BY-PORT**, but only if the census is complete (nothing left
  bare). DoD: a discriminating NOBYPASSRLS probe on every courier READ (not just writes) — each returns
  the courier's own rows post-flip; a bare-pool path is a build failure.
- **REV-S7-2 (breaker HIGH-1 — fake dispatch).** The honest-dispatch availability query
  (`lib/dispatch.ts:27-40`) has NO synthetic-courier exclusion — it lives only in the roster
  (`owner/couriers.ts:40`). A seeded synthetic courier + available shift → a real paid order binds to a
  non-human, violating the Q2 🔴 "no fake courier" ethical pillar. REV: FIX-IN-PORT — port the
  synthetic-courier exclusion INTO the availability query; test with a seeded synthetic + shift → real
  order does NOT bind.
- **REV-S7-3 (breaker HIGH-3 + counsel #3 — 085 applied, not "verified").** The settlement catch-up
  logic lives in `app_generate_settlements`, but 085 is an UN-APPLIED draft; the live fn is mig-078's
  lossy `>= p_period_start`. The §11 DoD gates on "085 verified" — must be "085 **APPLIED**". 085 is a
  HARD pre-flip dependency (shared with S5 REV-S5-7). The owner `/regenerate` cross-tenant blast is
  COUPLED: idempotent-safe ONLY post-085 — today it runs ALL tenants through the pre-085 lossy fn (a
  live pre-existing hazard the port carries). Settlement math stays in the DB DEFINER fn — Rust is a
  thin caller, never re-implements it. Inherit the 085 forcing-function into the S7 DoD.
- **REV-S7-4 (breaker MED-1/2 — shifts.ts, worst file).** D1 (shift-select with no status/ORDER BY/LIMIT
  → arbitrary-row transition) FIX-IN-PORT — but with the **status-filter** (`shifts.ts:26-31`), NOT the
  packet's `DATE=CURRENT_DATE` (which reintroduces overnight-shift corruption). `/me/shift/end`
  (`shifts.ts:122-126`) has the SAME arbitrary-row defect — fix both (status-filter + ORDER BY + LIMIT).
- **REV-S7-5 (breaker MED-3).** Settlement-read failure modes split (`/me/payouts` silent-empty vs
  `/me/payouts/:id` 500 from the still-bare `settlement_items` policy) — fixed-by-port via complete
  seating (REV-S7-1); document both.
- **REV-S7-6 (breaker LOW).** Honest-dispatch is lock-free check-then-act (`dispatch.ts:18-52`, caller
  `orders.ts:891` no `FOR UPDATE`); double-dispatch is contained ONLY by the frozen partial-uniques
  (mig 073) → surfaces as 500. CARRY verbatim (parity) + register; the partial-unique is the real guard.
- **REV-S7-7 (counsel #1 — session-liveness is CARRY).** `courierSessionValid` is real, per-request,
  revokes on deactivate/suspend/password-change (breaker verified-negative). Q1(a) reuse-the-bind is
  the recommended option (deactivated courier → 401 next request). REV: flag JWT-only-verify as
  FORBIDDEN (the downgrade risk) + prove the predicate parity on both stacks. Courier auth reuses the S2
  minter/verifier (no kid-drift — verified-negative).
- **REV-S7-8 (counsel #2+#5 — cash-as-proof + courier agency).** `cash===total` CARRY verbatim (it
  PROTECTS the courier — integer minor units, no rounding class; refuses to record a paid_full till-debt
  against uncollected cash; `refused_payment`→CANCELLED honest exit). Two named product decisions
  (owner + trigger, NOT money-input in the byte-parity port): (a) a **tip/change affordance** (today a
  "keep the change" forces the courier to make change or lie); (b) a **courier-agency lever** — the 422
  must be courier-READABLE (not a raw code), plus a payout-flag/dispute path so an underpaid courier
  isn't structurally silent (money-surface is read-only for the courier, `disputed` written only by the
  owner).
- **REV-S7-9 (counsel #4 — PII minimality).** Owner `/details` returns plaintext `name + phone +
  delivery_address × 20` (the packet missed the ADDRESS). Accepted-risk with owner + trigger: does the
  owner need a PERMANENT plaintext archive, or mask older-than-the-active-window? GPS consent boundary
  (position only accepted/picked_up; movement retention-purged) is ratified.

## 2. Question resolutions
- Q1 → courier auth reuses S2 verifier + REV-S7-7 session-liveness. 🔴
- Q2 → actor-gate (verified complete) + REV-S7-2 synthetic-exclusion port + REV-S7-6 register. 🔴
- Q3 → shared-ledger role-projection (no leak, verified) + DB-fn thin-caller + REV-S7-3 085-APPLIED. 🔴
- Q4 → cash-as-proof CARRY + REV-S7-8 tip/change + courier-agency product flags. 🔴
- Q5 → shifts.ts REV-S7-4 (status-filter fixes).
- Q6 → courier tenancy GUC + REV-S7-1 COMPLETE seat census + discriminating read probe. 🔴
- Q7 → in-flight delivery cutover rides S6 REV-S6-5 (low-delivery window + gradual drain); cross-stack pickup-X/deliver-Y probe.

## 3. 🔴 OPERATOR SIGN-OFF (blocks build)
Q1 (auth + session-liveness, coupled to S2 re-ratification) · Q2 (actor-gate + synthetic-exclusion) ·
Q3 (shared ledger + DB-fn + **085 APPLIED** before any settlement flip) · Q4 (cash-as-proof + the two
courier-agency product flags) · Q6 (complete seat census + read probe). Cutover: Q7.

## 4. Build/cutover DoD deltas
Complete-census discriminating NOBYPASSRLS read probe (REV-S7-1) · synthetic-exclusion-in-availability
test (REV-S7-2) · 085-APPLIED gate + pre-085 /regenerate hazard flagged (REV-S7-3) · shifts.ts
status-filter fix + /me/shift/end (REV-S7-4) · session-liveness predicate parity (REV-S7-7) ·
tip/change + courier-agency named product tickets (REV-S7-8) · PII-minimality accepted-risk (REV-S7-9).
