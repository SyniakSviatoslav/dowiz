# ADR — B3 Deep Auth Hardening (NOBYPASSRLS ramp + auth red-line layer)

- **Status:** Proposed (design-time; gated by system-breaker + counsel)
- **Date:** 2026-07-03
- **Deciders:** System Architect (proposer), Council, DB owner, Operator
- **Supersedes / builds on:** `ADR-pg-privilege-hardening` (direction), the shipped Tier-1 auth batch
- **Related:** `docs/design/b3-auth-hardening/proposal.md` (full analysis, back-of-envelope, options),
  `docs/design/pg-privilege-hardening/remediation-plan.md` (Phase 1–4), ADR-0003 (dev-kid), ADR-0004
  (owner-token revocation), ADR-0013 (courier realtime authz)
- **Red-line:** auth / money / RLS / `packages/db/migrations/**` — every migration + role/credential step
  is operator-gated (protect-paths).

## Context

App-layer authorization is strong post-Tier-1, but it is the only layer: the runtime operational DB role
is BYPASSRLS-class (the pool boot-guard rejects only literal `postgres`, not `rolbypassrls`). One forgotten
`WHERE location_id` is an un-netted cross-tenant leak. Additionally: (a) a leaked operational credential is
still unrotated; (b) WS auth still dual-accepts a `?token=` URL param (leaks via logs/Referer/SW cache);
(c) the rate limiter is process-local (brute-force bypass across Fly machines); (d) IDOR checks are
scattered per-route; (e) a prod↔staging schema drift (`telegram_connect_tokens` `owner_id` vs `user_id`)
makes the already-staged Phase-1 RLS policies unsafe to run on prod as-written.

Scale is small (design horizon ~100 locations, ~100 orders/min peak). Boring, proven, reversible wins.

## Decision

1. **Make the runtime pool RLS-enforced via a progressive, reversible ramp — Option B (txn-scoped
   role-switch ratchet), not a big-bang flip.** Create a dark `NOBYPASSRLS` role `dowiz_app_rls`,
   `GRANT dowiz_app_rls TO <login-role>`, and in `withTenant` (+ courier/webhook context setters) prepend
   `SET LOCAL ROLE dowiz_app_rls` inside the existing `BEGIN…COMMIT` when a per-lane flag
   (`RLS_ENFORCE_OWNER|_COURIER|_ANON`, default off) is on. Ramp owner → courier → anon. Converge to a
   permanent `ALTER ROLE <login-role> NOBYPASSRLS` (Option A) only after a full green soak.
2. **Anti-orphan = fail-CLOSED for tenant rows** (NULL tenant key → denied), with `users`/`auth_refresh_tokens`
   **role-restricted, not row-restricted** (RC2 `TO dowiz_app USING(true)`) as the deliberate lockout
   firebreak. Pre-flip audit back-fills/covers any legitimate orphan.
3. **Resolve the prod↔staging drift first.** A forward-only, idempotent reconciliation migration converges
   `telegram_connect_tokens` to the canonical column (verified by `information_schema` introspection per
   environment) **before** the Phase-1 policy migration runs on prod. RLS policies are written against
   prod's actual, reconciled schema — never a column that may not exist on the target.
4. **Remove JWT from the WS URL** (message/subprotocol auth only), telemetry-gated: keep dual-accept
   (`WS_URL_TOKEN_ACCEPT` default on) until `logTokenDeprecation` usage → 0, migrate the URL FE client
   (`apps/api/src/client/status/ws.ts`) to message auth (the pattern already used by
   `apps/web/src/lib/useWebSocket.ts`), then turn the flag off.
5. **Introduce a central owner→tenant-context preHandler** (`CENTRAL_TENANT_PREHANDLER`, default off) that
   resolves+asserts owner→location scoping once (reusing the RC3 `app_owner_location()` DEFINER resolver),
   adopted route-by-route with existing per-route checks **kept** until each route is proven covered.
6. **Rotate the leaked operational credential now**, with a dual-valid overlap window, and move toward
   `dowiz_app` as the single LOGIN + NOBYPASSRLS operational role (retiring `deliveryos_api_user`), with the
   canonical secret tied to the ci-pre-prod P2 single-source-of-truth. No secret in git; incident scrub is
   a hard precondition.
7. **Move auth-sensitive rate limits to a shared pg-backed store** (`RATE_LIMIT_STORE=memory|pg`, default
   memory), with an explicit **degrade-to-in-memory** fallback if the store is unavailable — never
   fail-closed on auth (that = lockout), never global fail-open.
8. **Ship the SAFE-NOW surface wins first** (proposal §6): flag-gated BYPASSRLS boot-guard (W1), guardrail
   tests pinning argon2 params (W2) and refresh reuse-detection (W3), a `Set-Cookie` grep-gate documenting
   the zero-cookie/no-CSRF posture (W4), a kid-rotation runbook (W5), and a TTL review (W6, no change).

## Options considered (NOBYPASSRLS rollout)

- **A · Big-bang role flip** — simplest, one probe, but atomic fleet-wide blast radius; rollback reopens B3
  entirely; no canary. *Rejected as the ramp mechanism; kept as the converged end-state.*
- **B · Txn-scoped role-switch ratchet** — **chosen.** Per-lane, flag-reversible-in-seconds enablement over
  a dark NOBYPASSRLS role; canary lane order; no connection-budget doubling. Cost: one `SET LOCAL ROLE` per
  enforced txn; partial enforcement during ramp (honestly no worse than today on un-wrapped paths).
- **C · Shadow parallel-pool canary** — real-traffic canary but doubles the connection budget (fights the
  §2 budget) and adds routing + a second failure surface. *Rejected as over-engineered for this scale;
  fallback only if B's per-txn role-switch misbehaves under load.*

## Consequences

**Positive.** Genuine defense-in-depth beneath app authz; reversible-by-flag ramp; a leaked credential
retired; a transport leak (URL token) closed; brute-force closed across machines; IDOR surface shrunk
structurally; a latent prod-deploy-breaking drift caught before it fires.

**Negative / costs.** `SET LOCAL ROLE` per enforced owner txn (cheap, but real) plus longer-lived
connection pinning → connection-budget pressure that must be gated (§9 scaling-gate; the 2026-06-20
outage class). Two role names during the ramp. Enforcement is partial until every path is wrapped.
Several steps are money/auth/RLS red-lines requiring council sign-off (R-2, R-3).

**Accepted risks (owner in proposal §10):** un-enumerated 0-row path (R-1, netted by E2E + anomaly metric +
flag revert), RC2 role-restriction (R-2, council), RC4 money-table current_tenant writes (R-3, council),
customer 7d bearer (R-11).

## Rollback

Every risky element reverts without an emergency migration: RLS lanes and preHandler and WS-URL and limiter
by **flag**; the credential by **secret repoint** over the dual-valid overlap window. Only the final
converged `ALTER ROLE … NOBYPASSRLS` is a migration-level state, deferred until after a long green soak; its
revert (`ALTER ROLE … BYPASSRLS`) is understood to reopen B3 and is therefore a break-glass, not a routine,
lever — which is precisely why Option B keeps the runtime toggle as the primary control.

## Verification (red→green, per harness discipline)

- Per-policy proofs under `SET LOCAL ROLE dowiz_app_rls` + GUC in throwaway txns (remediation plan §Per-phase).
- Full lifecycle E2E under enforcement per lane (anon checkout → owner accept → courier cash-as-proof →
  telegram-webhook → notifications fan-out) on staging before each prod lane.
- `verify:rls` gains: the `rolbypassrls` state probe (asserts the *intended* ramp state) + anon-leak probes.
- Guardrails: W1 boot-guard test, W2 argon2-param pin, W3 refresh-reuse regression, W4 `Set-Cookie` grep-gate.
- Drift: `information_schema` introspection assertion in the ci-pre-prod preflight before Phase-1 on prod.
