# ADR-0013 Addendum (DRAFT) — WS auth-channel migration off the URL token

**Status:** PROPOSED — awaiting operator approval (the escalation gate requires a human ack
before any P2/P3 auth-channel code). **Date:** 2026-07-02.
**Parent:** ADR-0013 courier-realtime-authz · escalation.md in this directory (P1 shipped, ledger #42).

## Decision (proposed)

Carry the WS JWT in the **`Sec-WebSocket-Protocol` header** (option b of the escalation),
dual-accepted alongside the legacy `?token=` for one overlap release, then remove URL-token
support server-side.

`new WebSocket(url, ['bearer.v1', token])` — the browser sends both values in the
`Sec-WebSocket-Protocol` request header; the server validates the token, and **echoes
`bearer.v1`** as the selected subprotocol (RFC 6455 §4.2.2 requires echoing one offered value;
echoing the token back would re-leak it — never echo the second value). JWT charset
(base64url + `.`) is valid for a subprotocol token, no encoding needed.

## Why this option

- **Zero extra round-trips** — auth still happens at upgrade, so the breaker's HIGH finding
  (5s auth-budget regression on slow mobile / reconnect storms) does not materialize. The
  first-message-auth option (a) eats the budget; the one-time-ticket option (c) adds an HTTP
  endpoint + storage + expiry for no additional benefit at this scale.
- **Not logged** — the token leaves `req.url` entirely; header redaction already covers
  `sec-websocket-protocol` once added to the Pino redact list (one line, extend ledger #42's
  guardrail to assert it).
- **`authPromise` serialization survives** — upgrade-time auth keeps the existing pipelined
  auth→subscribe ordering (`websocket.ts:180-183`); the breaker's MEDIUM missed-events
  finding does not materialize.

## Rollout (phased — "don't break the courier mid-shift")

- **R1 (dual-accept):** server accepts subprotocol OR `?token=` (subprotocol wins if both);
  `useWebSocket.ts` and `StatusWSClient` (`apps/api/src/client/status/ws.ts`) switch to
  subprotocol; `websocket-churn.test.ts` gains a subprotocol arm (keeps the URL arm to prove
  dual-accept). Deploy staging → prod. Old cached clients keep working via URL path.
- **R2 (≥1 release later, operator go):** remove server URL-token acceptance; flip
  `websocket-churn.test.ts` URL arm to assert **rejection** (1008 before auth); add the
  guardrail: a `?token=` upgrade attempt must never authenticate.
- Config escape hatch: `WS_URL_TOKEN_ACCEPT=true` env flag (default false after R2) for
  emergency rollback without a deploy-revert.

## Guardrails (ratchet)

1. Extend the ledger #42 redaction test: `sec-websocket-protocol` header must be redacted
   in Pino output.
2. R1: churn test proves BOTH channels auth; R2: URL channel proves REJECTED.
3. ESLint drift rule or grep-guardrail: no `?token=` construction in client WS code
   (`useWebSocket.ts`, `status/ws.ts`) after R1.

## Out of scope (unchanged obligations from the escalation)

- Scrub historical Fly/proxy access logs that captured `?token=…` (prod — operator).
- Courier 14d-token rotation decision if prod log exposure is deemed a compromise (operator).

## Approval

- [ ] Operator approves the subprotocol design (this file → status ACCEPTED, then R1 may be built)
- [ ] Operator schedules the R2 removal release
