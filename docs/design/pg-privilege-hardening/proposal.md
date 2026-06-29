# Design Proposal — Postgres Privilege Hardening (`pg-privilege-hardening`)

Status: DRAFT for breaker/council review. Design-time only — NO production/migration code in this change.
Author: System Architect (DeliveryOS). Red-line: touches RLS + `packages/db/migrations/**`.
Coupled items: ITEM 1 (SECURITY DEFINER `search_path`), ITEM 2 (operational-role `BYPASSRLS`). Both 🔴.

---

## 1. Problem + non-goals

### ITEM 1 — SECURITY DEFINER functions run without a pinned `search_path` (CRITICAL)
A `SECURITY DEFINER` function executes with the privileges of its *owner* (here a `BYPASSRLS`/superuser-class
deploy role). If its `search_path` is not pinned, name resolution follows the *caller's* `search_path`. A caller who
can place an object in an earlier-resolved schema (notably `pg_temp`, which Postgres searches **first** for
relation/type names unless explicitly demoted) can shadow a table the definer reads — e.g. a fake `memberships`
or `products`. The lynchpin `app_member_location_ids()` resolves `memberships`; shadowing it forges tenant
membership → **RLS bypass / privilege escalation across the whole tenant boundary**.

### ITEM 2 — operational pool connects as a `BYPASSRLS` role (CRITICAL-if-prod)
VERIFIED in `/root/dowiz/.env` (the PROD env per project memory):
```
DATABASE_URL_OPERATIONAL=postgresql://deliveryos_api_user.<ref>:<pw>@...pooler.supabase.com:6543/postgres
```
The hot path connects as **`deliveryos_api_user`**, which `1780691681296_ops-location-alerts-policy.ts` set
`ALTER ROLE deliveryos_api_user BYPASSRLS`. With `BYPASSRLS`, **every `ENABLE`+`FORCE` RLS policy in the system is
silently inert** on the hot path: tenant isolation rests entirely on app-code `WHERE` clauses, not the database.
The NOBYPASSRLS role designed for this (`deliveryos_operational_user`, `1790000000015_operational-pool-role.ts`)
exists but is **not used** by any env and has only `SELECT` grants. This is also the root cause of the failing
`pnpm verify:rls`: the script (`packages/db/scripts/verify-rls.ts`) drives the operational pool, expects an
anonymous (no `app.user_id`) query to return 0 rows, but `BYPASSRLS` returns all rows → "Isolation leak" → exit 1.

The `createOperationalPool()` boot-guard (`packages/db/src/index.ts:32-39`) only rejects `current_user='postgres'`;
`deliveryos_api_user` passes that guard while still bypassing RLS. The guard is **necessary but insufficient**.

### Non-goals
- Not rewriting the `read_public_menu` query logic, JSON shape, or any cache contract (behavior-identical only).
- Not editing already-applied historical migrations (forward-only; history is immutable).
- Not migrating the SSR/crawler reader's security model in this change (see §10 — `read_public_menu_all_locales`
  is currently `SECURITY INVOKER`, a separate flagged finding, not in ITEM-1 scope).
- Not introducing `REVOKE TEMPORARY` on the Supabase-pooled database unless staging proves it is permitted
  (raised as an accepted-risk / defense-in-depth option, §10).

---

## 2. Back-of-envelope

**How many DEFINER functions actually need fixing.** Verified by reading the *latest* `CREATE [OR REPLACE]` of each
function across all 17 `SECURITY DEFINER`-matching migration files (only the latest definition is live):

| Function (latest def) | Latest migration | `SECURITY DEFINER` now? | `search_path` pinned? | In ITEM-1 scope |
|---|---|---|---|---|
| `app_member_location_ids()` | `1780310071220` | **YES** | no | **FIX** (lynchpin) |
| `upsert_menu_version(uuid)` | `1780338982020` | **YES** | no | **FIX** |
| `bump_menu_version_trigger_fn()` | `1780338982021` | **YES** | no | **FIX** |
| `read_public_menu(text,text)` | live head `1790000000065`; staged `…072` | **YES** | no | **FIX** (hottest read) |
| `app_current_user()` | `1780310071220` | no (sql STABLE, INVOKER) | n/a | not in scope |
| `menu_schedule_matches(...)` | `1790000000062` | no (sql IMMUTABLE) | n/a | not in scope |
| `product_available_now(...)` | `1790000000062` | no (plpgsql STABLE) | n/a | not in scope |
| `read_public_menu_all_locales(text)` | `…035` live / `…072` staged | **no — INVOKER** (lost DEFINER at `…033`) | n/a | flagged §10, not ITEM-1 |
| `app_is_shadow_location(uuid)` | staged `…070` | YES | **yes (`= public`)** | already compliant |
| `read_preview_menu(text)` | staged `…070` | YES | **yes (`= public`)** | already compliant |
| `claim_transfer(text,uuid)` | staged `…071` | YES | **yes (`= public`)** | already compliant |
| `anonymize_stale_delivery_trace(interval)` | staged `…073` | YES | **yes (`= public`)** | already compliant |

**Net: exactly 4 live DEFINER functions need the `search_path` pin** — not the "13" claimed by the existing
staged artifact `docs/security/SECURITY-DEFINER-search-path.migration.ts` (that count double-counts every
`CREATE OR REPLACE` occurrence and includes non-DEFINER functions like `app_current_user`, `menu_schedule_matches`,
`product_available_now`). The 4 newest staged DEFINER functions (P6/deliver-v2) already pin `search_path` — the
"new code follows the rule" lesson held; only the **old** functions are deficient.

**Migration size.** The fix is signature-agnostic and body-free: one idempotent `DO`-block that `ALTER FUNCTION …
SET search_path` over every `prosecdef` function in `public` lacking a pin. ~20 lines. Zero function bodies touched
→ **zero transcription risk on the ~150-line `read_public_menu`** (the dominant risk if we rewrote bodies).

**`verify:rls` current state.** RED — fails at the first anonymous-count assertion because the operational role is
`BYPASSRLS` (ITEM 2). ITEM 1 cannot even be observed by `verify:rls` until ITEM 2 is fixed (a `BYPASSRLS` role makes
RLS moot regardless of `search_path`). **Ordering is forced: ITEM 2 first, then ITEM 1's gate becomes meaningful.**

**Blast radius of redefining `read_public_menu` (the hottest read).** It serves every storefront's menu on the
public hot path (operational pool, `OPERATIONAL_POOL_SIZE` default 20). If we `CREATE OR REPLACE` the body and get
one token wrong → every menu returns wrong/empty/500 → storefront down for all tenants. **Therefore we do NOT
redefine the body.** `ALTER FUNCTION … SET search_path` changes only `proconfig`, leaving the body byte-identical →
blast radius reduced to "does the pinned path still resolve every referenced object" (yes: all refs are
`public` tables + `pg_catalog` built-ins). `ALTER FUNCTION` on the same signature **preserves all `EXECUTE`
grants** (Postgres only drops grants on `DROP`, not `ALTER`/`CREATE OR REPLACE` of the same signature).

**Connection budget (sanity).** Operational pool max ≈ 20 (Supavisor txn mode :6543, multiplexed); session pool
max = 3 (:5432); plus workers + analytics + one-shot migrations. None of this change adds connections; the
`verify:rls` run uses the existing two pools. Flipping the operational role to NOBYPASSRLS does not change pool
sizing — it changes *what rows the same connections can see*.

---

## 3. Options (≥2, with tradeoffs + named concept)

### ITEM 1 — value of `search_path`

- **Option A — `SET search_path = public` on each fn.** Concept: minimal pin.
  Tradeoff: **insufficient.** `pg_catalog` is implicitly searched, and crucially `pg_temp` is implicitly searched
  **first** for relation/type names whenever it is *not explicitly listed*. So `= public` still lets a caller who
  can `CREATE TEMP TABLE memberships` shadow `public.memberships`. Also still allows built-in shadowing edge cases.
  Rejected as primary. (Note: this is also the form the existing P6/deliver-v2 staged fns use — see §10 follow-up.)

- **Option B — `SET search_path = ''` + fully schema-qualify every reference.** Concept: strictest (CIS/Postgres
  "secure DEFINER" canonical). With an empty path and every object written `public.memberships`,
  `pg_catalog.now()`, etc., there is no unqualified resolution for an attacker to hijack.
  Tradeoff: requires **rewriting all 4 function bodies**, including the ~150-line `read_public_menu`, qualifying
  dozens of refs. Enormous transcription risk on the hottest read for marginal gain over Option C. A single missed
  qualification → runtime error → menu down. Rejected for the hot path; reserved as the standard for *future*
  green-field DEFINER fns.

- **Option C — `SET search_path = pg_catalog, public, pg_temp` via `ALTER FUNCTION` (no body change).** ✅ CHOSEN.
  Concept: pin resolution order AND **explicitly demote `pg_temp` to last** so it can never shadow a relation.
  `pg_catalog` first prevents built-in shadowing; `public` before `pg_temp` means `public.memberships`/`products`
  always win over any temp object; listing `pg_temp` explicitly removes its default first-position. Bodies are
  untouched → behavior-identical, zero transcription risk, grants preserved.
  **This corrects the existing staged artifact**, which uses `pg_catalog, public` and therefore leaves `pg_temp`
  implicitly first → the relation-shadowing vector stays open. (The `= public` vs `= ''` vs `pg_catalog, public`
  vs `pg_catalog, public, pg_temp` distinction is the load-bearing correctness point; see §6.)

### ITEM 1 — packaging

- **Per-fn migration** (one `ALTER FUNCTION` per function, signatures hand-written): explicit, greppable; but
  signatures drift (e.g. `read_public_menu(text,text)` default arg) → risk of "function does not exist". Rejected.
- **Option: one consolidated idempotent `DO`-block** (✅ CHOSEN): iterate `pg_proc.prosecdef` in `public` lacking a
  `search_path` config, `ALTER FUNCTION oid::regprocedure …`. Signature-agnostic, idempotent, catches any DEFINER
  fn including ones I may not have enumerated. Matches the existing staged artifact's mechanism (only the pinned
  *value* changes to Option C).

### ITEM 2 — converging the operational role

- **Option 2A — `ALTER ROLE deliveryos_api_user NOBYPASSRLS`, keep the role.** ✅ CHOSEN.
  Concept: strip the dangerous attribute from the role the app *already* uses (it already holds the correct DML
  grants accumulated across migrations — `menu_schedules`, `claim_invites`, etc.). Minimal blast radius: no env
  change, no grant re-plumbing. The reverse of the exact statement in `1780691681296`.
  Tradeoff: enforcement flips ON the moment it runs — any flow that forgot to `SET app.user_id`, or that *relied*
  on seeing cross-tenant rows, breaks. This is why staging-first + `verify:rls` green is the gate, not a guess.

- **Option 2B — switch `DATABASE_URL_OPERATIONAL` to `deliveryos_operational_user`.** Concept: use the
  purpose-built NOBYPASSRLS role. Tradeoff: that role currently has only `SELECT` (`1790000000015`) → all
  INSERT/UPDATE/DELETE on the hot path break until a full DML re-grant migration lands; grants are currently
  scattered between the two role names (some migrations grant to `deliveryos_api_user`, some to
  `deliveryos_operational_user`) → high risk of a missed grant → write 500s. Rejected for now; revisit as a clean
  long-term consolidation once grants are unified.

- **Option 2C — strengthen the boot-guard only** (reject `rolbypassrls`): defense-in-depth, but does not *fix*
  the bypass — it would make the app refuse to boot, i.e. trade a silent security hole for an outage. Adopt it as
  an *additional* guard (see §9), never as the fix.

---

## 4. Decision + rationale (ADR-format → also docs/adr/ADR-pg-privilege-hardening.md)

1. **ITEM 2 first:** `ALTER ROLE deliveryos_api_user NOBYPASSRLS` (Option 2A) + harden the boot-guard to also reject
   `BYPASSRLS` roles (Option 2C as belt-and-suspenders). Rationale: until the hot-path role enforces RLS,
   `verify:rls` cannot pass and ITEM 1's gate is unobservable. 2A reuses the already-granted role → smallest blast
   radius. Forward-only: the migration asserts the role is NOBYPASSRLS (idempotent), it does not re-create grants.
2. **ITEM 1 second:** consolidated idempotent `DO`-block `ALTER FUNCTION … SET search_path = pg_catalog, public,
   pg_temp` over all `prosecdef` public fns lacking a pin (Option C + consolidated). No bodies touched. This pins
   the 4 deficient live fns and is a no-op on the already-compliant staged fns (their `= public` would be
   re-pinned to the stronger value, harmlessly tightening them).
3. **Both gated** by red→green proofs on staging (see §9) before any prod apply.

This does not contradict existing ADRs: it *completes* the intent of `1790000000015` (NOBYPASSRLS operational
access) and of the `cleaning-loop` finding (DEFINER `search_path`), and aligns with the B12 "verify:rls as a gate"
pattern already used for import_sessions.

---

## 5. Data / migrations

Forward-only, additive, idempotent, behavior-identical. Two staged migration *artifacts* (operator places them in
`packages/db/migrations/` — a protected governance zone; numbers chosen at placement, after the current head):

- **MIG-ITEM2** (`…_operational-role-nobypassrls.ts`): an idempotent `DO` block —
  `ALTER ROLE deliveryos_api_user NOBYPASSRLS;` wrapped in an exception-swallowing block (mirrors `1780691681296`
  so it is a no-op where the role is absent). No grant changes (the role keeps its existing DML grants → no write
  path regresses by construction). down() = re-grant BYPASSRLS is intentionally **not** provided (re-introducing the
  hole); down() is a documented no-op.
- **MIG-ITEM1** (`…_secdef-search-path.ts`): the consolidated `DO`-block from §3, with the value corrected to
  `pg_catalog, public, pg_temp`. Idempotent (`NOT EXISTS … proconfig LIKE 'search_path=%'` skips already-pinned
  fns). Signature resolved via `oid::regprocedure` → no signature drift. No body changes → all `EXECUTE` grants
  preserved (Postgres preserves grants across `ALTER FUNCTION`/`CREATE OR REPLACE` of the same signature; only
  `DROP` clears them — verified semantics). Forward-only; down() = no-op (un-pinning re-introduces the vuln).

Drift avoidance across the 4 fns: **we never re-transcribe bodies** — the `DO`-block alters `proconfig` only, so
the verbatim drift problem (the reason `read_public_menu` carries 8 near-identical copies across migrations) does
not apply here. The existing staged artifact `docs/security/SECURITY-DEFINER-search-path.migration.ts` is the basis;
the only required change is the pinned value (`pg_catalog, public` → `pg_catalog, public, pg_temp`) and the corrected
header count (4, not 13).

Integer-money / RLS-FORCE invariants: untouched (no schema, no money, no policy DDL in either migration).

---

## 6. Consistency + idempotency (the `pg_temp` correctness analysis — load-bearing)

Postgres name resolution facts that drive the choice:
- `pg_catalog` is **implicitly searched before** the listed schemas unless explicitly placed → built-ins are always
  findable; placing it first explicitly is equivalent and defensive.
- `pg_temp` (the session temp schema) is, **for relation and type names, implicitly searched FIRST — before
  `pg_catalog`** — *whenever `pg_temp` is not explicitly listed*. It is never searched for function/operator names.
- Therefore:
  - `= public` → effective order `pg_temp, pg_catalog, public` → a temp `memberships` **shadows** `public.memberships`. INSUFFICIENT.
  - `= pg_catalog, public` (the staged artifact) → `pg_temp` still unlisted → still searched first for relations → **still shadowable.** INSUFFICIENT for the relation vector (only the built-in/function vector is closed).
  - `= pg_catalog, public, pg_temp` → `pg_temp` explicitly demoted to last → `public` resolves relations before any temp object → **closed**, while built-ins remain safe. CHOSEN.
  - `= ''` + full schema-qualification → strictest (no unqualified resolution exists), but body rewrite risk on the hot read → reserved for green-field fns.

Idempotency: the `DO`-block alters only fns whose `proconfig` lacks `search_path=` → re-runs are no-ops; the
role-flip is idempotent (NOBYPASSRLS of an already-NOBYPASSRLS role is a no-op).

---

## 7. Failures + degradation

- **MIG-ITEM1 references a non-public schema?** None of the 4 fns reference `extensions`, `pgboss`, or `pg_temp`
  objects — all refs are `public` tables + `pg_catalog` built-ins (`now()`, `count()`, `jsonb_*`, `gen_random_uuid`
  is via column DEFAULTs not the fns). `pg_catalog, public, pg_temp` covers every reference. If a future fn needs
  `extensions` (e.g. a non-core function), that fn must list it explicitly — the guardrail (§9) forces an explicit,
  reviewed decision rather than silent reliance on a mutable path.
- **`read_public_menu` breaks → storefront down.** Mitigated by *not* touching the body; the only failure mode is a
  pinned path that fails to resolve an object — impossible here (verified refs). The staging golden no-op proof
  (§9) catches it before prod regardless.
- **ITEM 2 flip surfaces a flow that never set `app.user_id`.** This is the *intended* exposure of latent bugs, but
  it must not happen in prod blind. Degradation plan: run `verify:rls` + the full lifecycle E2E on **staging** under
  the NOBYPASSRLS role first; any newly-failing query is a real isolation bug to fix *before* prod. If a critical
  flow cannot be fixed in-window, the *fallback* is to defer the prod flip behind an explicit operator step (the
  migration is idempotent and can be applied independently of code deploy) — never silently re-grant BYPASSRLS.
- **No cascade:** neither migration calls an external service; both are single transactional DDL statements.

---

## 8. Security + tenant isolation

- **Exploit precondition (ITEM 1).** Requires a caller that can both `SET search_path` and create a shadowing
  object (`CREATE TEMP TABLE` needs `TEMPORARY` on the database — granted to `PUBLIC` by default in Postgres, so the
  operational role likely *can* create temp tables) on the operational connection. In a parameterized app with no
  SQL injection this is not directly reachable — so ITEM 1 is **defense-in-depth that becomes load-bearing the
  moment ITEM 2 makes RLS real** and the moment any SQL-injection or compromised-worker path exists. Fix it
  regardless: a DEFINER fn without a pinned path is an unconditional latent escalation.
- **Operational role privileges (ITEM 2).** `deliveryos_api_user` currently: `BYPASSRLS` (the bug), LOGIN, DML on
  tenant tables. After 2A: NOBYPASSRLS → `FORCE` RLS becomes the real boundary on the hot path; app `WHERE` clauses
  become defense-in-depth instead of the sole control. `REVOKE CREATE ON SCHEMA public` was set for
  `deliveryos_operational_user` but **not** for `deliveryos_api_user` — recommend mirroring that revoke as a
  follow-up (accepted-risk §10), and optionally `REVOKE TEMPORARY ON DATABASE … FROM PUBLIC`/the role to kill the
  `pg_temp` vector at the source (defense-in-depth on top of Option C).
- **Does any app write depend on bypass?** Hot-path writes (`orders`, `courier_assignments`, etc.) set the tenant
  GUC (`SET LOCAL app.user_id` / `app.current_tenant`) and rely on policies that admit that context — these are
  *designed* for NOBYPASSRLS and are exactly what `verify:rls` exercises. The risk is any *un-tenant-scoped* admin
  or sweep query running on the operational pool; those must be identified by the staging run (§9). The DEFINER
  readers (`read_public_menu`) are unaffected by the role flip (they run as their owner).
- **Is `ALTER ROLE … NOBYPASSRLS` safe?** Safe as a statement; the *risk is behavioral* (newly-enforced RLS), which
  is precisely what the staging gate de-risks. It is the correct converged end-state.

---

## 9. Operability

- **Staging-first.** Apply MIG-ITEM2 then MIG-ITEM1 on `dowiz-staging-db` via the standard proxy + node-pg-migrate
  flow before any prod consideration.
- **Observability < 1 min.** The authoritative live check (add to `verify:rls`, exit(1) on any row):
  ```sql
  SELECT p.oid::regprocedure
  FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
  WHERE n.nspname = 'public' AND p.prosecdef
    AND NOT EXISTS (SELECT 1 FROM unnest(coalesce(p.proconfig,'{}')) c WHERE c LIKE 'search_path=%');
  ```
  Plus a role check: `SELECT rolname FROM pg_roles WHERE rolname='deliveryos_api_user' AND rolbypassrls;` must
  return 0 rows.
- **ITEM 1 GATE (red→green).** Before MIG-ITEM1 on staging: the query returns the 4 deficient fns (RED). After:
  0 rows (GREEN). Wired into `verify:rls` so it stays green forever.
- **ITEM 2 GATE (red→green).** Before MIG-ITEM2: `verify:rls` exits 1 at the first anonymous-count leak (RED) and
  the `rolbypassrls` probe returns 1 row. After: `verify:rls` passes all tenant-isolation assertions + the
  `rolbypassrls` probe returns 0 rows (GREEN). This is the proof that the failing `verify:rls` is fixed.
- **The static guardrail** (`scripts/guardrail-definer-search-path.mjs`), specified — deterministic, runs in CI/
  pre-commit, no DB needed:
  - Read every `packages/db/migrations/**/*.ts`. Concatenate file text (catches multi-line template literals and
    `const FN_x = \`…\`` forms — it is text-level, not AST).
  - Regex-extract each `CREATE (OR REPLACE )?FUNCTION … ` header up to the body delimiter (`AS $tag$` / `AS $$` /
    `LANGUAGE …`). For each header that contains `SECURITY DEFINER`, assert `SET search_path` appears within the
    same `CREATE … <body-start>` span. Fail-list any that don't.
  - **History is immutable** → a frozen ALLOWLIST (committed `// definer-baseline.json`) of the pre-existing
    historical offender occurrences (the 4 functions' historical `CREATE`s + the `*_PRIOR` rollback bodies in
    064/065) that MIG-ITEM1 remediates at runtime. Any offender NOT in the allowlist → exit(1). The allowlist is
    frozen; new migrations cannot append to it (a second guardrail asserts the allowlist file is unchanged, or
    simpler: only enforce on files whose numeric prefix > the MIG-ITEM1 number — historical files are exempt,
    new files are not).
  - **red→green proof:** add a throwaway migration fixture above the baseline with `SECURITY DEFINER` and no
    `SET search_path` → guardrail exits 1 (RED); add `SET search_path = pg_catalog, public, pg_temp` → exits 0
    (GREEN); delete the fixture. Record the row in `docs/regressions/REGRESSION-LEDGER.md`.
- **Rollback.** Both migrations are forward-only with documented no-op down() (reversal re-introduces the
  vulnerability). Operational reversal, if ever forced, is a manual, reviewed `ALTER ROLE … BYPASSRLS` /
  un-pin — never automatic.
- **Scaling-gate / flag:** none needed — these are one-shot DDL with no runtime feature surface. The boot-guard
  hardening (reject `rolbypassrls`) ships with the next API deploy and is itself a fail-fast gate.

---

## 10. Open / accepted risks

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R1 | `read_public_menu_all_locales` silently lost `SECURITY DEFINER` at `…033` (CREATE OR REPLACE omitted it) and is now `SECURITY INVOKER` — the *inverse* of ITEM 1. It works only because menu tables carry `public_select` (`USING true`) policies. | **Flag (defer-decide).** Not ITEM-1 scope. Decide deliberately: keep INVOKER (relies on `public_select`, arguably safer) vs restore DEFINER+pinned path. Whichever is chosen must be locked by a guardrail so the next CREATE OR REPLACE can't flip it again silently. | DB owner + architect |
| R2 | Option C `pg_catalog, public, pg_temp` closes relation-shadowing but does not stop the operational role from creating temp tables at all. | **Accept + recommend follow-up:** `REVOKE TEMPORARY ON DATABASE … FROM PUBLIC` (and the role) if Supabase permits — staging-test first; defense-in-depth on top of C. | DB owner |
| R3 | ITEM 2 flip exposes a latent un-tenant-scoped query in prod that staging didn't cover. | **Accept with gate:** the staging lifecycle E2E + `verify:rls` is the mitigation; prod flip is a separate explicit operator step; never re-grant BYPASSRLS as a "fix". | operator |
| R4 | Two role names (`deliveryos_api_user` DML-granted vs `deliveryos_operational_user` SELECT-only) with split grants is confusing and invites future misconfig. | **Defer:** long-term consolidation to one NOBYPASSRLS role (Option 2B done properly with a unified grant migration). Out of scope here. | architect |
| R5 | Existing staged P6/deliver-v2 DEFINER fns use `= public` (insufficient per §6). | **Fix-forward:** MIG-ITEM1's `DO`-block re-pins them to `pg_catalog, public, pg_temp` (they lack the stronger value); the guardrail then forbids regressions. | DB owner |
| R6 | Boot-guard only checks `current_user='postgres'`. | **Fix:** extend `createOperationalPool` to also reject a connection whose role `rolbypassrls` is true (Option 2C), fail-fast on boot. | API owner |
| R7 | `packages/db/migrations/` is protect-paths-blocked; these are *artifacts* requiring operator placement. | **Accept:** standard handoff (mirrors 068–073). Numbers assigned at placement. | operator |
