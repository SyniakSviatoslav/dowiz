# Escalation — WebSocket JWT in `?token=` URL (red-line: auth surface, ADR-0013)

**Status:** ESCALATED — awaiting human decision. No auth code written.
**Date:** 2026-06-30 · **Raised from:** QA loop 2 (admin/courier) · **Branch:** feat/mvp-sensor-seams
**Surface:** red-line (auth) → touches ADR-0013 courier-realtime-authz.

## Finding (CONFIRMED, CRITICAL)

The realtime WS authenticates by accepting the JWT access token as a URL query param.

- Main app: client sets it `apps/web/src/lib/useWebSocket.ts:50`; server reads it `apps/api/src/websocket.ts:155`.
- Order-status widget: `apps/api/src/client/status/ws.ts:17` (`/ws/orders/:id?token=…`) — **URL-token only, no message fallback**.
- **Server logs it.** Pino `req` serializer logs `req.url` unconditionally (`apps/api/src/lib/logger.ts:83-88`); `redact` covers headers only, **not query params** (`logger.ts:92-96`). So every URL-token WS upgrade writes `GET /ws?token=<full JWT>` to app/Fly/aggregator logs.
- Also visible in the browser console (observed live on staging).

**Replay window** (token TTLs, verified): courier **14d** (`routes/courier/auth.ts:136`), customer **7d** (`packages/platform/src/auth/jwt.ts:131`), owner **24h** (`routes/auth.ts:149`, ADR-0004). Bearer tokens → anyone with log read access (ops/support/eng/aggregator/infra) can impersonate until expiry.

**Prod is affected**, not just staging: `origin/main` carries `useWebSocket.ts:50` (the URL-token client line). Owner/courier tokens have been landing in prod logs.

## Council verdicts (3 independent read-only analysts)

- **security-sentinel → CRITICAL.** The dominant vector is the **logger** (`logger.ts:85`), not the browser console. Removing the client URL token is *necessary but not sufficient* — the serializer must strip query strings, else cached/old clients + the widget keep leaking. TTLs corrected above.
- **system-breaker → the original "just remove URL token" proposal is NOT robust if it touches the server.** Concrete breaks:
  - `StatusWSClient` is URL-token-**only** (`status/ws.ts:17-28`, no `{type:'auth'}` send) → dropping server URL support → 5s authTimeout closes it (1008) → no reconnect → customers lose live tracking permanently.
  - `apps/api/tests/websocket-churn.test.ts:54-63` (P1-WSDUP regression guardrail) auths **URL-only** → server removal turns it CI-red (ship-blocker).
  - **5s auth-budget regression (HIGH):** URL token auths at upgrade with zero round-trips; message-only adds open→send→network inside the 5s ceiling → can fail on slow mobile / reconnect storms → "never authenticates" loop.
  - MEDIUM: deleting the URL token removes the `authPromise` serialization (`websocket.ts:180-183`) that protects pipelined auth+subscribe; widens the missed-events window before first subscribe.
  - Clean: ADR-0013 courier room-authz/relay-guard is channel-agnostic — **no regression** there.
- **counsel → no ETHICAL-STOP.** Full council is over-engineered (no contested trade-off in the *log* fix). Right ratchet = ADR-0013 addendum + regression guardrail + one human ack on the auth diff. Real couriers'/owners' creds in logs clears the "polish" bar; phased rollout is also the *ethical* default ("don't break the courier mid-shift to save the courier").

## Recommended plan (phased, council-informed)

**P1 — log redaction (safe, no auth/contract change, no test breakage, fixes the CRITICAL vector for BOTH clients):**
strip the query string (or redact `token`) from `req.url` in the Pino `req` serializer (`logger.ts:83-88`) before logging. Guardrail test: a synthesized `req.url` with `?token=…` must log with the token absent/`[REDACTED]`. Reversible, deployable now.

**P2 — browser-console leak (client URL token) — genuine design decision, GATED:**
removing `useWebSocket.ts:50` regresses the 5s auth budget (breaker HIGH). Options to weigh in an ADR-0013 addendum: (a) raise/adapt `authTimeout`; (b) carry the token in `Sec-WebSocket-Protocol` (browser-supported, never logged in URL); (c) ephemeral one-time WS ticket. Needs the design call + human ack.

**P3 — `StatusWSClient` widget + server URL removal — GATED, depends on P2:**
give the widget a non-URL auth channel first (message or subprotocol), update `websocket-churn.test.ts` off URL-only, then remove server URL-token support. Phased over ≥1 overlap release for rollback safety.

## Open obligations (counsel)

- Scrub Fly/proxy access logs that captured `?token=…` (prod = real obligation; staging = hygiene).
- Rotation: only if a long-lived (courier 14d) token in prod logs is considered compromised; short TTLs otherwise age out. Human call.

## Gate

No P2/P3 auth-channel code until a human approves the ADR-0013 addendum. P1 (log redaction) is reversible and recommended to ship immediately pending the operator's go.
