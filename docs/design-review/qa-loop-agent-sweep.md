# Cross-tenant Realtime QA Loop — adversarial agent sweep + hardening (v2)

**Date:** 2026-06-27 · Three independent agents (critique/system-breaker · security-sentinel ·
test-scout/QA) attacked the blind spots of `e2e/tests/cross-tenant-realtime-qa.spec.ts` (v1, which
ran 6/6 green on staging). They converged hard: **v1's headline claims — real-time, cross-tenant —
were the weakest assertions in it.** v2 implements the fixes.

## Converged findings (ranked)

| # | Severity | Blind spot (v1) | Why it's a false-green |
|---|----------|------------------|------------------------|
| 1 | CRITICAL | Real-time = `dash.refs() > 0` | Collector subscribed AFTER the order existed (catches the on-subscribe snapshot); needs only ONE message ever; never checks delta CONTENT → status-broadcast could be fully dead and it stays green. |
| 2 | CRITICAL | Customer real-time untested | The customer `order:<id>` room is never inspected anywhere; "customer sees live updates" rested on one DOM render. |
| 3 | CRITICAL | Isolation uses an all-zero FAKE tenant | An empty room can't leak even if the guard is wide open → proves nothing. |
| 4 | HIGH | `collector` swallows WS errors | `ws.on('error', resolve)` → a socket that errors yields empty `msgs` → the isolation `length===0` PASSES even if the WS is just down. |
| 5 | HIGH | No positive control + permissive `[401,403,404]` | If order-token reads were globally broken (404 for everything), every cross-order negative passes vacuously; 404-vs-403 can mask an info leak. |
| 6 | CRITICAL(sec) | Courier password hardcoded + account persists | `qa-courier-pass-123` on a never-deleted account = a standing reusable credential on staging. |
| 7 | MED | Idempotency bug → orphan order | The probe `post([])` used its own key; on the CLEAN path it returns 201 and creates an UNTRACKED order every run. |
| 8 | MED | Role-3 skip cascades | On courier 429 the ENTIRE real-time block (the only WS assertion) was skipped → a green run could assert zero real-time. |
| 9 | LOW | Fixed sleeps, self-pollution, velocity/rate-limit drift | Rots into flakiness; confirmed live (my iteration rate-limited order-create + courier auth). |

## v2 — implemented

- **Real-time is now real:** owner dashboard AND customer `order:<id>` collectors subscribe BEFORE the
  transitions; per transition we baseline the count and assert a **NEW** delta for the order arrives on
  **both** rooms (proves per-transition broadcasting + the user-facing customer path), plus a soft
  content check that the delta carries the new status string. (#1, #2)
- **Error-swallow guard:** `wasOpened()` asserts both WS connections actually opened before the
  real-time assertions — a silent error now fails, not passes. (#4)
- **Real isolation:** customer-A subscribes to **order-B's real room** (a room A is not a member of) and
  must receive zero deltas — a real authz test, not an empty fake. Kept the owner fake-room + unauth +
  customer-can't-PATCH; added a body-leak check (order-B's tag absent from A's response). (#3)
- **Positive control:** the customer reads their OWN order = 200 before the negatives → the cross-order
  denials are no longer vacuous. (#5)
- **Random courier password** per run (`Qa1!<uuid>`) → no reusable backdoor. (#6)
- **Idempotency branch:** the probe creates-and-tracks on the clean path (no orphan); the soft path
  confirms. All created orders tracked → cleaned up. (#7)
- **Decoupled real-time from courier:** real-time is its own test (owner-only lifecycle), always run;
  the courier dimension is a separate test that skips only itself on 429. (#8)

## Deferred — with reason (honest)

- **A real second OWNER/tenant** (owner-A reads tenant-B) — infeasible: staging has no
  `/auth/local/register`, and courier-invites are location-scoped, so a fully-isolated 2nd owner can't be
  provisioned via API. v2 covers the achievable real vector (cross-order/customer-room + the WS membership
  guard). The owner-level cross-tenant read is guard-verified in code, not live-tested. **The one assertion
  still missing.**
- **DELIVERED transition** — `delivered` 400'd in probes (needs more than a bare call; likely dropoff/cash
  params). The courier proof stops at picked-up → IN_DELIVERY. Worth a follow-up to resolve the 400.
- **Hard-block / OTP / no_show soft-confirm, idempotency-replay, illegal-transition 409, terminal-UI
  steps** — the QA agent confirmed these are already covered by `flow-order-creation.spec.ts`,
  `api-real.spec.ts`, `flow-order-lifecycle-trace.spec.ts`, `client/order-stepper.spec.ts`. Not duplicated
  here (this loop's job is the cross-tenant/multi-role/real-time integration, not the per-endpoint matrix).
- **Courier account deletion** — no API; the random password mitigates the backdoor risk; the account
  persists (tagged email `qa-courier+<ts>@dowiz.dev` for a purge sweep).
- **Deterministic waits over fixed sleeps** — partially: real-time now uses `expect.poll` per transition;
  a couple of fixed sleeps remain for snapshot-absorption + the isolation quiet-window.
