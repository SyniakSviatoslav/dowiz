# ADR — Security hardening batch (blue-team sweep 2026-07-02)

- Status: **APPROVED-pending-operator** — all three Breaker HIGH resolved in design (see
  `resolution.md`); remaining gates are human/operator: OR-1 pool-role confirmation (live-exposure) +
  OR-10 #9 ingress guarantee. Per-tier + per-OR sign-off required (not a single batch stamp — Counsel §3.5).
- Date: 2026-07-02
- Deciders: System Architect (proposer), Council (auth/RLS/money red-line), Operator + DB owner.
- Supersedes / relates: does NOT supersede `ADR-pg-privilege-hardening` (B3) — this ADR front-loads its
  route/app-layer subset and hands findings #2/#3 to the B3 track with a sequencing contract. Honors
  ADR-0003 (JWT/dev-kid), ADR-0004 (owner-token revocation), ADR-0010 (error envelope / server-authoritative
  correlation), ADR-0013 (courier realtime authz). Full design: `docs/design/security-hardening-2026-07/proposal.md`.

## Context

A blue-team sweep found 9 red-line findings (8 from the sweep + #9 from a parallel perf sweep, anti-abuse).
The unifying root is **identity-split × RLS-reliance**: privileged read/write paths omit an explicit
`location_id` predicate and lean on RLS, while couriers/customers carry no `userId` to seat the member GUC.
The live operational pool connects as **`dowiz_app` (`rolbypassrls=t`)** — RLS is **inert on the hot path**,
so app-layer predicates are the *only* tenant boundary today; and the C1 anonymous policies are table-wide
TRUE on any no-context connection (fail-open, latent under bypass, a siphon post-flip). The architect cannot
read Fly secrets, so the deployed `DATABASE_URL_OPERATIONAL` role remains an **open operator input**; the
batch is designed to be correct under **both** BYPASSRLS and NOBYPASSRLS.

## Decision

Adopt **Option C — both, sequenced** (see proposal §3):

1. **Tier 1 (ship now, pool-agnostic):** make the app layer the authoritative tenant boundary + fix the
   transport/authz/anti-abuse findings — #1 (orders **authorize-by-JOIN** for owners + **`courierReadVerdict`
   binding-scope** for couriers — closes cross-tenant AND the insider-removal read AND the within-tenant
   courier PII read OR-3; revised per Breaker B2/Counsel §3.2), #3 (pin the C2 keystone definer), #4 (WS
   `status='active'` **+ fan-out owner re-authz**, ≤TTL residual; revised per Breaker B3), #5 (**dual-accept
   deprecation window** for `?token=` + redact logs; revised per Breaker #5), #6 (spa-proxy live
   owner-recheck), #7 (explicit invite membership predicate), #8 (customer `sub` as throttle/idempotency
   identity), #9 (global `Fly-Client-IP` key **iff** operator confirms edge-only ingress; else scope
   narrower — OR-10). Plus the anti-orphan artifact for #2 and the SAFE riders.
2. **Tier 2 (B3-coupled, ship dark then flip-gated):** #2 — narrow the C1 anonymous SELECT/UPDATE policies
   to fail-closed; lands in the B3 Phase-1 policy migration *with* the courier/anon GUC seating (migration
   077 RC-set) and is only *effective* at the operator-gated NOBYPASSRLS flip.
3. **Root-class guardrail (batch DoD) — RESTRUCTURED per Breaker B1:** the **primary DoD is per-finding
   BEHAVIORAL red→green tests** (real proof of isolation — a substring sweep cannot certify that a
   body-sourced value is authorized, and could not even detect #7). The static `WHERE location_id` sweep is
   a **cheap secondary anti-omission lint only** (reframed claim + fixed to include `locations`/`couriers`/
   `courier_invites`), wired into `verify:all` (`ci:true`). Definer-pin: static gate already `ci:true` for
   regressions; the runtime pin is verified at boot-guard/staging, NOT claimed as CI-continuous. New static
   escape comments require a named reviewer (Counsel §3.6).

Rejected: Option A alone (leaves RLS permanently inert — no defense-in-depth); Option B alone (does not fix
the live route bugs and hits the KNOWN TRAP — flipping before C1-narrowing + courier GUC seating leaks or
breaks).

## The known trap (why sequencing is load-bearing)

Applying NOBYPASSRLS naively **leaks** (C1 fail-open) or **breaks** (courier/notification paths with no
`app.user_id` → 0 rows). Therefore: app-layer predicates (#1,#7) + the C1 narrowing (#2) + courier GUC
seating (077 RC4/RC5) MUST all land **before** any role flip. Tier 1 predicates make the system correct
*regardless* of pool role; #2 ships dark (inert under bypass) and is proven via `SET LOCAL ROLE dowiz_app`
per-policy proofs before the flip. The flip itself stays in ADR-pg-privilege-hardening / B3, operator-gated.

## Consequences

**Positive:** live cross-tenant read/write bugs (#1,#7) closed immediately under the current pool; the
insider-removal window closed for owners (#4,#6) AND couriers (#1, via `courierReadVerdict`); the courier
within-tenant cross-customer PII read (OR-3) closed in Tier 1; a bearer-token transport leak (#5) reduced;
anti-abuse throttles restored to per-attacker (#8,#9); the recurring class caught by **behavioral tests**
(primary DoD) + a static lint; B3 unblocked but not rushed.

**Negative / accepted:** a transitional period where RLS is still inert (covered by app predicate +
behavioral tests — strictly better than today) — **time-boxed to the B3 flip, with the #2 anti-orphan
artifact as the forcing trigger** (Counsel §3.3); #4's fan-out re-authz bounds the owner revocation window
to **≤TTL, not zero** (OR-9, parity with couriers); #5 keeps the transport-leak surface (log-redacted) live
for the dual-accept window rather than lock out SW-cached clients; #9's global rekey is conditional on the
operator ingress guarantee (OR-10); leaked-token puddle (OR-8) is a named follow-up.

**Adopt C, pay its premium (Counsel §4 steel-man of Option B).** Option C is only defensible *because* it
carries the explicit #2 anti-orphan mechanism (ledger row + skip-registered probe that ships WITH Tier 1,
before the #2 fix). Without it, C inherits B3's demonstrated failure mode — an orphaned partial RLS narrowing
(exactly what happened in migration 077). The anti-orphan artifact is not optional decoration; it is the
premium that makes the sequenced approach honest.

## Migrations (forward-only, atomic, RLS FORCE preserved, integer-money untouched)

- #3: `ALTER FUNCTION app_member_location_ids() SET search_path = pg_catalog, public, pg_temp;`
  (metadata-only). Do not widen the definer baseline.
- #2 (B3 track): `DROP POLICY IF EXISTS … ; CREATE POLICY …` narrowing anon SELECT/UPDATE on `orders`,
  `order_items`, `customers`, `idempotency_keys` to a scope-GUC discriminator (fail-closed). RLS already
  ENABLE+FORCE — do not re-toggle. Inert under bypass; proven before the flip.
- All migrations operator-gated (protect-paths), staging-first, prod human-gated.

## Open decisions (operator — hard gates, not background risk)

1. **OR-1 pool role — LIVE-EXPOSURE gate.** Confirm the deployed `DATABASE_URL_OPERATIONAL` role +
   `rolbypassrls` (documentary evidence: `dowiz_app` BYPASSRLS = Case A). If **unexpectedly NOBYPASSRLS
   today**, #2 is a **live table-wide cross-tenant PII siphon → promotes to Tier 1**; the batch is NOT
   "PII-resolved" without the *recorded* confirmation (Counsel §2). NOT "Tier 1 ships regardless."
2. **OR-10 #9 ingress guarantee.** Confirm the Fly edge sets/overwrites `Fly-Client-IP` on ALL ingress
   (HTTP + WS upgrade + internal). If unconfirmed, do NOT rekey the auth brute-force limiters onto the
   header (keep `request.ip`); apply `clientIp()` only to the funnel routes.
3. **OR-5 connect-guard hardening.** Reject on `rolbypassrls` (not just the literal `postgres` name).
4. **OR-8 leaked-token remediation** (rotate/invalidate + `?token=` log scrub) — owner Operator + Security.

Per Counsel §3.5, record the human decision **per tier + explicitly on OR-1 and OR-10** — not a single
batch stamp.

## Guardrail / DoD (red→green)

- **PRIMARY = per-finding behavioral tests** (Breaker B1): owner-B order → 404; ex-courier own-location
  order → denied; courier → other-courier's venue order → denied (OR-3); body-`locationId` invite → 404;
  revoked owner evicted from WS fan-out ≤TTL (no further frames); revoked-owner spa-proxy write → 401/404;
  message-auth works + `?token=` logged; #9 (prod-forced) distinct `Fly-Client-IP` → distinct buckets,
  spoofed XFF → shared bucket; defined customer `customerId`. **No finding is "done" without its behavioral
  test proven red→green** (project charter). A substring lint does NOT satisfy this.
- **SECONDARY = static anti-omission lint** `scripts/guardrail-route-tenant-predicate.mjs` (new, DB-less,
  baseline'd, `ci:true`, table set includes `locations`/`couriers`/`courier_invites`; new escape comments
  require a named reviewer). Reframed claim: flags obvious unscoped queries — does NOT prove isolation, does
  NOT catch the #7 body-sourced class.
- **Definer-pin:** `scripts/guardrail-definer-search-path.mjs` (existing, `ci:true`) catches new unpinned
  definers; the *live* pin for `app_member_location_ids` (#3) is verified at boot-guard/staging, NOT in
  DB-less CI (honest limitation — Breaker #3).
- **#2 anti-orphan (ships WITH Tier 1, before the fix):** REGRESSION-LEDGER red-line row + skip-registered
  named `verify:rls` probe that cannot be removed without narrowing C1.
