# ADR — Postgres Privilege Hardening (SECURITY DEFINER `search_path` + operational `BYPASSRLS`)

Status: DRAFT (proposed). Date: 2026-06-29. Deciders: System Architect + DB owner (operator places migrations).
Red-line: RLS + `packages/db/migrations/**`. Companion: `docs/design/pg-privilege-hardening/proposal.md`.

## Context

Two coupled CRITICAL security-audit findings, both verified against live source:

1. **DEFINER `search_path` (ITEM 1).** Exactly **4** live `SECURITY DEFINER` functions in `public` run without a
   pinned `search_path`: `app_member_location_ids()` (the RLS lynchpin, `1780310071220`),
   `upsert_menu_version(uuid)` (`1780338982020`), `bump_menu_version_trigger_fn()` (`1780338982021`), and
   `read_public_menu(text,text)` (latest live `1790000000065`). A mutable `search_path` on a DEFINER fn owned by a
   bypass-class role lets a caller who can create a shadowing object (notably in `pg_temp`, searched first for
   relations) hijack the `memberships`/`products`/`locations` references → RLS bypass / privilege escalation. The 4
   newest staged DEFINER fns (P6/deliver-v2) already pin `search_path`; only the old fns are deficient. The existing
   staged artifact (`docs/security/SECURITY-DEFINER-search-path.migration.ts`) claimed "13" (inflated — counts every
   `CREATE OR REPLACE` and non-DEFINER fns) and pins `pg_catalog, public` (insufficient vs `pg_temp`).

2. **Operational `BYPASSRLS` (ITEM 2).** `.env` (PROD) sets `DATABASE_URL_OPERATIONAL` to connect as
   `deliveryos_api_user`, which `1780691681296` granted `BYPASSRLS`. The hot path therefore bypasses every
   `FORCE` RLS policy; tenant isolation rests on app `WHERE` clauses alone. The purpose-built NOBYPASSRLS role
   `deliveryos_operational_user` (`1790000000015`) exists but is unused and SELECT-only. This is the root cause of
   the failing `pnpm verify:rls`. The boot-guard (`packages/db/src/index.ts:32-39`) only rejects `postgres`, not
   bypass roles.

## Decision

1. **ITEM 2 first** — `ALTER ROLE deliveryos_api_user NOBYPASSRLS` (idempotent, exception-swallowed; reuses the
   role's existing DML grants → minimal blast radius). Additionally harden `createOperationalPool()` to fail-fast on
   any connection whose role has `rolbypassrls`.
2. **ITEM 1 second** — one consolidated, idempotent `DO`-block: `ALTER FUNCTION <oid::regprocedure> SET search_path
   = pg_catalog, public, pg_temp` for every `prosecdef` function in `public` lacking a `search_path` config. No
   function bodies are touched (behavior-identical; grants preserved across `ALTER FUNCTION`).
3. **Value = `pg_catalog, public, pg_temp`** (not `public`, not `pg_catalog, public`): `pg_catalog` first stops
   built-in shadowing; `public` before an *explicitly demoted* `pg_temp` stops relation shadowing. `= ''` + full
   schema-qualification (strictest) is rejected for the hot path due to body-rewrite risk on the ~150-line
   `read_public_menu`; it is the standard for future green-field DEFINER fns.
4. **Both gated** by red→green proofs on staging before any prod apply; `verify:rls` gains a permanent DEFINER-pin
   check and a `rolbypassrls` probe; a static `scripts/guardrail-definer-search-path.mjs` forbids future
   unpinned DEFINER CREATEs in migrations.

## Consequences

- **Positive:** RLS becomes the real hot-path boundary; the DEFINER escalation vector (incl. the lynchpin) is
  closed; `verify:rls` goes green and stays green via two new gates; no function bodies re-transcribed → zero
  drift/transcription risk on the hottest read; all `EXECUTE` grants preserved.
- **Negative / risk:** flipping NOBYPASSRLS enforces RLS immediately — a latent un-tenant-scoped query could
  surface; mitigated by staging lifecycle E2E + `verify:rls` before prod, with prod flip as a separate explicit
  operator step (never re-grant BYPASSRLS as a workaround).
- **Forward-only:** both migrations have documented no-op `down()` (reversal re-introduces the vulnerability).
- **Follow-ups (out of scope, tracked in proposal §10):** decide `read_public_menu_all_locales` INVOKER-vs-DEFINER
  (R1) and lock it; `REVOKE TEMPORARY`/`REVOKE CREATE` from the operational role (R2/R6); long-term consolidation to
  a single NOBYPASSRLS operational role with unified grants (R4); re-pin the staged `= public` fns to the stronger
  value (R5).

## Alternatives considered

- ITEM 1: `= public` (insufficient vs `pg_temp`); `= ''` + qualify (strictest but hot-path body-rewrite risk);
  per-function `ALTER` (signature drift). → consolidated `DO`-block + `pg_catalog, public, pg_temp`.
- ITEM 2: switch env to `deliveryos_operational_user` (needs full DML re-grant; split-grant risk → write 500s);
  boot-guard-only (turns a hole into an outage, not a fix). → `ALTER ROLE … NOBYPASSRLS` + boot-guard as
  defense-in-depth.

## Proof gates (red→green)

- **ITEM 1 (live):** `pg_proc` DEFINER-without-`search_path` query returns 4 rows (RED) → 0 rows (GREEN) post-migration; wired into `verify:rls`.
- **ITEM 1 (static):** guardrail fixture migration without `SET search_path` → exit 1 (RED) → with it → exit 0 (GREEN); ledger row added.
- **ITEM 2:** `verify:rls` exits 1 on anonymous leak + `rolbypassrls` probe returns 1 row (RED) → `verify:rls` all-green + probe 0 rows (GREEN).
