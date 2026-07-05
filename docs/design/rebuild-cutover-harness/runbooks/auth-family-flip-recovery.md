# Auth-family flip recovery runbook (AR-4, S2 cutover amendment RESOLVE)

> **Why this exists:** family-revoke is NOT rollback-recoverable — rolling a flag back to
> `node` does not resurrect a DELETEd refresh family. Every prior seat optimized the
> *frequency* of false revokes; this runbook is for whoever one *hits*. Counsel: "a support
> ticket is not recovery, it is abandonment." This document is a **named precondition** for
> every auth-family flip (S2, and S7 whose courier mint carries the same token shape — AR-3).

## Scope

Applies whenever `cutover_flags` moves an auth-family surface (S2 owner/customer auth, S7
courier) between stacks, in EITHER direction, on ANY environment. The C2 posture
(atomic flip + refresh-quiesce) minimizes the window; this runbook covers the residue.

## 1. What a victim looks like

| Actor | Symptom | Trigger class |
|---|---|---|
| Owner | silent logout, next admin action 401s, re-login demanded | family revoked by cross-flip reuse-detection false positive |
| Courier **mid-shift** | app drops to login screen while carrying cash/deliveries | courier_sessions family revoked or rotation raced the flip |
| Customer | track-page link dead | track token family invalidated |

## 2. Immediate triage (operator, <5 min)

1. `SELECT surface, target, readiness_ok, updated_by, updated_at FROM cutover_flags WHERE surface IN ('S2','S7');`
   — confirm what flipped and when. Correlate victim reports with `updated_at`.
2. Check reuse-revocations in the flip window:
   - owners: `SELECT family_id, count(*) FROM auth_refresh_tokens GROUP BY family_id;` — a family that VANISHED between snapshots was revoked. The revoke DELETE leaves no row: rely on API logs (`Token reuse detected. Family revoked.`) filtered to the flip window.
   - couriers: same over `courier_sessions`.
3. If revocations cluster in the flip window (>2 within 5 min of the flip): **halt further
   flips**, keep the current target (do NOT flap back — the families are gone either way),
   and proceed to recovery below.

## 3. Owner recovery

- Owner re-login with existing credentials (argon2 local or Google/Telegram) restores a fresh
  family immediately — no support action required. Session TTL/refresh UX is restored on first
  login (ADR-0004 24h access / 7d family).
- If the owner cannot re-login (OAuth outage etc.): support-mediated re-auth is the LAST
  resort, not the plan (see counsel note above). Escalate to the on-call operator.

## 4. Courier IN-SHIFT re-auth (the fast path — written for the courier's reality)

A courier mid-delivery gets a **shorter path than a full login**:

1. The courier app keeps `activeLocationId` + courier id in local state even after a 401.
2. Owner (or dispatcher) issues a fresh **courier invite** for the SAME courier from the
   admin Couriers screen (`POST /api/owner/locations/:id/courier-invites`) and shares the
   invite link over whatever channel already reaches the courier (the pair are usually
   already talking about the active delivery).
3. Courier taps the invite link → redeem → new `courier_sessions` family → back on shift.
   Active assignments are keyed to the courier row, NOT the session — deliveries, cash-ledger
   state (deliver-v2 cash-as-proof HOLD) and settlement rows survive the re-auth untouched.
4. Target: **< 2 minutes**, no desk, no support ticket, one hand on the phone.

Verify after re-auth: courier sees the same active assignment list (`GET /api/courier/assignments`)
and the shift row still carries the original `started_at`.

## 5. Customer track-token recovery

Track links are minted per order. Recovery = re-send the track link from the order's
communication channel (owner order view → resend). No credential exists to reset.

## 6. Post-incident

- Append the event to `docs/regressions/REGRESSION-LEDGER.md` (class: auth-flip revoke).
- If ANY false family-revoke occurred: the next auth-family flip is BLOCKED until the
  quiesce procedure is tightened (extend the drain window) and re-proven on staging.
- Feed the victim count into the AR-1 posture review — 0 victims across a staging flip
  cycle is the DoD for attempting the same on prod.

## 7. Quiesce procedure reference (C2, AR-1)

Executed AT flip time by the operator/lead:

1. Confirm parity interlock GREEN: `node scripts/rebuild-cutover/parity-interlock.mjs`
   (sets nothing itself — GREEN is the precondition for `readiness_ok=true`).
2. Enter quiesce: watch `/api/auth/refresh` + courier redeem/refresh traffic
   (API logs) — on staging this is normally zero; on prod pick a low-traffic minute.
3. Single atomic flip: `UPDATE cutover_flags SET target='rust', readiness_ok=true, updated_by='<who>: <why>' WHERE surface='S2';`
   (one statement, one surface — never batch S2+S7 in one UPDATE; S7 follows only after S2
   soaks green).
4. Exit quiesce when the first post-flip refresh rotates cleanly on the new stack
   (log line) with no reuse-revocation in the following 5 minutes.
5. Rollback (routing only): same UPDATE back to `node`. Families minted during the rust
   window remain valid on Node (cross-verifiable tokens — that is what the interlock proved).
