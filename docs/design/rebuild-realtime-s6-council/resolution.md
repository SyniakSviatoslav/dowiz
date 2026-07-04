# S6-REALTIME-WS — Council RESOLVE

> **Verdict: PROCEED-WITH-REVISIONS. No ETHICAL-STOP (counsel).** Packet-status 🟡 — NOT
> COUNCIL-APPROVED until operator signs §3. Seats: architect (packet) · breaker (1 CRIT / 3 HIGH /
> 6 MED / 1 LOW) · counsel (PROCEED-WITH-REVISIONS) · lead (this RESOLVE). WS is the gentlest cutover
> (stateless socket, DB-decoupled fan-out) BUT the fan-out plumbing has a silent-blackout class the
> proposed health signal cannot see — that is the load-bearing fix.

## 1. Frozen revision set

- **REV-S6-1 (breaker C1, CRIT — undetectable pooler blackout).** A `LISTEN` orphaned on the 6543
  tx-pooler succeeds with NO error/end → `checkHealth()` returns `'ok'` forever while ZERO NOTIFYs
  arrive (`message-bus.ts:238,91`). The proposed detector observes *disconnect*; the real failure is
  *connected-but-mute*. REV: the port's health MUST be an **active liveness probe** — a self-`NOTIFY`
  heartbeat the listener must echo-receive within N seconds, else `degraded`; the DoD test exercises
  the connected-but-mute case (not just disconnect). Pin the listener to `DATABASE_URL_SESSION` (5432)
  by config guardrail. This converges with REV-S6-6 (the heartbeat is also what makes the status-dot
  truthful).
- **REV-S6-2 (breaker H1 + counsel #2, HIGH — logged-out courier live tail).** The per-frame guard
  re-reads `courier_assignments` only, never `courier_sessions` (`courier-room-authz.ts:69-72`), so
  the S2 `CourierSession` bind at upgrade closes the *reconnect* vector but NOT the *mid-stream* one:
  a logged-out/kicked-but-still-bound courier keeps streaming customer GPS for up to the token life.
  REV: **answer against the records — does courier deactivation reset the `courier_assignments`
  binding?** YES → the per-frame binding read evicts ≤ its TTL, closed; document it. NO → add
  session-revocation eviction on the same `evict` path (per-frame session-liveness), so a
  de-authorized courier cannot watch someone's live GPS until they happen to reconnect.
- **REV-S6-3 (breaker H2, HIGH — typed-serde vs byte-verbatim are mutually exclusive).** Node relays
  an OPAQUE `JSON.stringify({room,data:msg})` passthrough (`websocket.ts:218`); a typed serde
  re-decode/encode drops unmodeled producer fields, reorders keys, reformats numbers (52→52.0) →
  breaks the Q6 golden-frame cutover gate + silently drops fields on a cross-stack reconnect. REV: for
  cutover PARITY the WS relay CARRIES the opaque passthrough VERBATIM during overlap (do not
  typed-decode); typed `Event` enums land only in a POST-cutover FE-lockstep release. This resolves the
  Q6 tension (parity-first, types-later).
- **REV-S6-4 (breaker H3, HIGH — unquantified session ceiling).** The session-mode ceiling is named
  THE scaling gate but asserted with no number; prod already draws ~9 session backends (message-bus +
  worker + free-tier-watch, each `createSessionPool` max 3) before the net-new Rust listener. REV:
  **measure** the real 5432 session-conn budget and prove the net-new listener fits BEFORE the flip
  (cutover gate); "+1, assumed fine" is not acceptable — it can cascade into C1.
- **REV-S6-5 (counsel #1, Q5 — reinstate the signed cutover canon).** The packet dropped the S6 canon
  the cutover council already signed: **flip in a low-delivery window** + **gradual drain (not a
  synchronous mass-close)** + DoD "the courier's assigned order + cash-to-collect survive with ZERO
  courier-visible loss," not merely "the row is intact via refetch." Reinstate — all free (jitter +
  per-IP rate limit already exist). No connection-migration / new reconnect-ceiling needed (WS is
  genuinely stateless — do not over-engineer).
- **REV-S6-6 (counsel #3+#5, Q4 — silent staleness under a green light).** A NOTIFY lost while the
  listener reconnects leaves the socket OPEN → no delta, no reconnect → silent staleness; for owner
  `order.created` that is a **silently-missed first paid order under a green status dot**. Cheap fixes
  using existing seams: (a) fire `Event::Resync` on listener degraded→healthy (REV-S6-1's heartbeat is
  the trigger); (b) bind the WS status-dot truth-signal to **listener** health, not just socket
  liveness; (c) rewrite the accepted-risk as an honest residual + name the transactional outbox as
  future-hardening with a trigger (not a silent defer).
- **REV-S6-7 (MED/LOW register).** Matcher `Upgrade`-vs-`Connection` header divergence
  (`cutover-matcher.ts:163-167`) — align to the WS spec; permanent JWT-in-URL residual → adopt
  `Sec-WebSocket-Protocol bearer.v1`, `?token=` dual-accept flagged for same-release removal; owner-guard
  first-frame drop; duplicate fan-out during reconnect (client-dedup by event id); `Principal::Channel`
  needs a default-deny; URL-token verify-swallow → log+close. Each CARRY-or-fix noted at build.

## 2. Question resolutions
- Q1 → WS admission via `Sec-WebSocket-Protocol` bearer (S2 verifier, body-kid) + REV-S6-2 session-liveness. 🔴
- Q2 → one generic `RelayGuard<Policy>` + per-frame cross-tenant re-authz (ADR-0013). 🔴
- Q4 → REV-S6-1 active heartbeat + REV-S6-6 Resync + honest NOTIFY residual. 🔴
- Q5 → REV-S6-5 low-delivery-window flip + gradual drain, no migration. 🔴
- Q6 → REV-S6-3 opaque passthrough during overlap; typed events post-cutover.
- Q8 → lifecycle expiry → 4401; anonymous → REST-poll (no WS).

## 3. 🔴 OPERATOR SIGN-OFF (blocks build)
Q1 (admission + token transport + session-liveness) · Q2 (fan-out re-authz) · Q4 (heartbeat + Resync +
NOTIFY residual) · Q5 (cutover drain window) · plus name the S6 overlap end-trigger + owner (counsel #4,
the un-cut-vine twin — session-conn budget as the forcing function).

## 4. Build/cutover DoD deltas
Active-heartbeat health test (connected-but-mute, REV-S6-1) · per-frame session-liveness / binding-reset
proof (REV-S6-2) · opaque-passthrough golden-frame both directions (REV-S6-3) · measured session-conn
budget pre-flip (REV-S6-4) · gradual-drain + zero-courier-loss cutover test (REV-S6-5) · Resync-on-recover
+ listener-bound status dot (REV-S6-6).
