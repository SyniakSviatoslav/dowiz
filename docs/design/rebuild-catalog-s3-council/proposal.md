# S3-CATALOG Port — Council Packet · PROPOSAL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Description input to the live Triadic Council
> (system-architect + system-breaker + counsel + human). No S3 code is ported to Rust until
> this packet is council-APPROVED, every quirk-register row (§7) is dispositioned one by one,
> and the operator signs the 🔴 open questions (`open-questions.md`). Docs only; no product code.

- **Lane:** R3 (complete-rebuild) · **Surface:** S3 catalog/admin CRUD (REBUILD-MAP §3 Phase B, 5th strangler)
- **Date:** 2026-07-04 · **Source commit:** `fix/audit-remediation@b28b1764` (working tree)
- **Census SSOT:** `inventory/10-api-realtime-jobs.md` owner-route census rows 1–113 (this packet
  covers the **four 🔴 concerns** below; the non-🔴 CRUD port runs as a parallel build lane — §2).
- **Governing ADRs:** ADR-0004 (owner-token revocation / P-d per-route re-read), ADR-0014 (allergen
  provenance / `source` flag), the money-newtype council (`docs/design/rust-money-newtype-phase-a/`),
  the RLS-reliability council (`docs/design/audit-fix-rls-reliability/` — the anonymizer N1/GDPR-worker
  wrong-GUC-family CRITICAL precedent this packet re-raises on the owner path).
- **Parity oracle:** the 174-spec Playwright net + the invariant-cluster tests. No behavior change is
  real without a red→green test (Mandatory Proof Rule). Cutover DoD in §8.

---

## 1. Port objective and the load-bearing seam

S1 was read-only (`PgRepo` reads `pools.operational` directly, no txn). S2 wrote **auth** tables only.
**S3 is the first Rust surface that writes tenant catalog data** — products, categories,
modifier-groups, location pricing, themes, availability, allergen confirmation — through the
tenant-scoped write path. The single load-bearing seam is therefore the **tenant-scoped WRITE
contract**: every owner write must run inside one transaction that has seated the correct
per-request GUC *before* the write, and no tenant table may ever be touched on a context-free
pooled connection.

**The seam fact the current Rust scaffold gets wrong (sharpest finding — see §3 and Q1 🔴):**
The live owner path uses `withTenant(pool, userId, …)` = `BEGIN → set_config('app.user_id', userId,
true) → work → COMMIT` (`packages/platform/src/auth/tenant.ts:3-21`). Owner-path RLS policies
resolve the tenant from `app.user_id` **→ memberships** via the `app_member_location_ids()` definer
(~34 policy sites, inventory/12 §2/§7). The existing Rust helper `crates/api/src/db.rs::with_tenant`
sets a **different GUC** — `app.current_tenant` — from a `TenantId` (a *location_id*, not a user id).
`app.current_tenant` is the **courier/worker/service** root (~102 sites), not the owner root. Reusing
`with_tenant` for S3 writes seats the wrong GUC family with the wrong value; today BYPASSRLS masks it,
post-B3-flip **every owner catalog write matches 0 rows / fails `WITH CHECK`** — the exact
anonymizer-N1 / GDPR-worker break (`audit-fix-rls-reliability/breaker-findings.md:105-111`) replayed
on the owner surface. S3 must add a **`with_user(pool, UserId, …)` combinator** and route owner writes
through it; `with_tenant`/`TenantId` stay reserved for the S6/S7 courier/service surfaces.

## 2. Scope — the four 🔴 concerns (and the parallel lane)

**In this packet (🔴 only):**
1. **First Rust tenant-scoped WRITES** — the `with_user` write-pattern contract (§3). Governs the
   whole owner-write surface; the parallel CRUD lane builds *to this contract*.
2. **`locations.ts` PATCH** (census row 113) — 🔴 money-adjacent pricing inputs (§4).
3. **`menu-confirm.ts`** (census row 112) — 🔴 food-safety/liability allergen gate (§5).
4. **`menu-import.ts` commit** (census rows 90–92) — 🔴 bulk-edit + PII-in-uploads; surface-placement
   recommendation (§6).

**Parallel build lane, OUT of packet except where the §3 write-pattern touches it:** the non-🔴 CRUD
port — `products.ts` (14), `categories.ts` (8), `modifier-groups.ts` (7), `themes.ts` (3),
`menu-availability.ts` (4, incl. the R2-1 IDOR-fixed schedules POST). These build against the §3
contract but carry no independent red-line; their disposition is `PORT` in the matrix. **Non-goals:**
no error-envelope normalization (defer to post-Astro FE-lockstep, cf. S2 Q4), no new features, no
schema change (DB frozen), no order-total computation (that is S5 — S3 only *stores* the pricing
inputs, §4).

**Back-of-envelope (why boring wins here).** Catalog writes are human-driven owner edits, not an order
hot-path: at the target scale (tens of active locations, low tens of owners editing concurrently
during onboarding/menu edits, near-zero steady state) owner-write QPS is single-digit. The Rust
operational pool (20 conns, `db.rs:87`) + session pool (3) is already oversized for this. The ONE
long-held transaction in the whole surface is `menu-import commit` — a bulk upsert loop over N
categories × M products × K modifiers inside a single txn holding a connection for its duration
(§6) — which is the connection-budget and blast-radius argument for isolating it into its own slice.
Every other S3 write is a single-statement (or single-digit-statement) `with_user` txn: negligible.

---

## 3. Concern 1 — First Rust tenant-scoped WRITES · the write-pattern contract

**Contract (each clause a red→green test vector at DoD):**

1. **Correct GUC family.** Owner writes seat `app.user_id = <owner userId>` via a `with_user`
   combinator (new), mirroring `withTenant` verbatim: `pool.begin() → set_config('app.user_id', $1,
   true) → f(&mut txn) → commit/rollback`. **Do NOT reuse `db.rs::with_tenant`** (it seats
   `app.current_tenant`, the wrong root — Q1 🔴, Q7). Value bound is the owner's **user id**, not a
   location id; RLS derives the location set from memberships.
2. **Transaction-local, `is_local = true`.** `set_config(..., true)` inside the `BEGIN` — the GUC
   resets at COMMIT/ROLLBACK and never leaks across pool reuse. This is already pinned by the
   `db.rs` unit test `set_tenant_statement_is_pinned` (latent-GUC-bug class 1); `with_user` gets the
   same pinned-SQL test.
3. **No context-free connection touches a tenant table.** The raw pool stays unreachable from route
   code (`Pools` fields are `pub(crate)`, `db.rs:19-23`); `with_user`/`with_tenant` are the ONLY
   sanctioned paths. This is the structural answer to the anonymizer N1 CRITICAL ("the order read is
   left context-free"): a `pool.acquire()` in a catalog handler must not compile-through review.
4. **Belt AND suspenders.** Every live owner write ALSO carries an explicit `WHERE location_id = $N`
   (and folds child-ownership into INSERT…SELECT / same-tx pre-checks — `products.ts:307-335`,
   `categories.ts:163-176`). **Carry these verbatim** — they are defense-in-depth that holds
   *independent of* which pool role is live (the sweep's "identity-split × RLS-reliance" root). The
   S3 DoD extends `rls-adversarial.test.ts` "privileged pool queries have WHERE location_id" from
   `workers/` to `routes/owner/**`.
5. **P-d per-route membership re-read.** Owner-write routes bind the S2 `OwnerAt<Loc>` extractor (the
   live `status='active'` membership re-read, ADR-0004 P-d) — a removed owner is blocked immediately,
   not at ≤24h token expiry. The JWT-alias routes (`/api/owner/menu/*`, `getOwnerLocationId`) and the
   `getLocationId()` DB-membership helper resolve location from a **live** memberships row, never a
   trusted JWT claim (census cross-cutting note #3) — port 1:1.

**Latent GUC-leak classes NEVER copy (inventory/12 §7, all masked today by BYPASSRLS, detonate at B3):**
- **Class 1** `set_config(..., false)` (session-scoped) on the txn-pooled pool with no RESET
  (`owner/onboarding.ts:75`) — cross-tenant leak on connection reuse. `with_user` uses `true`, always.
- **Class 2** `set_config(..., true)` with **no BEGIN** (dies in its own autocommit txn:
  `owner/couriers.ts:152`, `owner/signals.ts:207`). `with_user` owns the BEGIN; a bare set_config
  cannot happen.
- **Class 3** `Pool.query()` set_config then `Pool.query()` data on **different physical connections**
  under txn pooling (`courier/settlements.ts:25…`). `with_user` runs both on the same `&mut txn`.
S3 touches none of these files, but the *pattern* is the thing to refuse — the Rust win is exactly
"there is ONE way to touch a tenant table."

**What holds TODAY (BYPASSRLS) vs POST-FLIP (NOBYPASSRLS live):**
- *Today:* RLS is bypassed; the explicit `WHERE location_id` predicate is the **only** live tenant
  boundary. A `with_user` that seated the wrong/no GUC would still pass (masked) — which is precisely
  why this is latent and dangerous.
- *Post-flip:* RLS is authoritative. `SELECT/UPDATE/DELETE` need `app.user_id` seated to see the row;
  **`INSERT` needs `app.user_id` seated to satisfy `WITH CHECK`** (products/categories INSERTs). If
  the GUC family is wrong, INSERT/UPDATE match 0 rows or raise — a total, silent owner-write outage.
  S3 must be provably correct under NOBYPASSRLS **before** the flip (Q2 🔴), and must not *rely* on
  the flip to fix a missing predicate. Two B3-council rows gate this and are named here: the
  `app_member_location_ids()` unpinned-`search_path` pin (sweep #3) and the GUC-always-seated
  invariant (sweep #2) — S3's DoD asserts the port sets `app.user_id` on every tenant-table txn.

---

## 4. Concern 2 — `locations.ts` PATCH · money-adjacent (census row 113)

`PATCH /api/owner/locations/:locationId` (`locations.ts:9-66`) writes `tax_rate`,
`delivery_fee_flat`, `free_delivery_threshold`, `min_order_value` — the **direct upstream inputs** to
the S5 order-total calc. It is not itself a transaction over money; it stores the parameters a money
transaction later reads. Port contract:

- **Validation parity, exactly (`locations.ts:17-31`, `.strict()`):** `delivery_fee_flat`
  `int ≥ 0`; `min_order_value` / `free_delivery_threshold` `int ≥ 0` **nullish** (nullable-clearable);
  `delivery_radius_km` `≥ 0` nullish (float); `tax_rate` `≥ 0 ≤ 100` — a **percent float, NOT integer
  money** (stored as `numeric`, `repo.rs::LocationInfoRow.tax_rate: Option<f64>`). Integer-money
  invariant (`domain::Lek`, i64 minor units) applies to the three flat-fee fields; `tax_rate` stays a
  rate. lat/lng bounded; `default_locale` must be in `supported_locales` **only when both are in the
  same request body** (`:42-46`) — a request setting only `default_locale` against the *stored*
  supported set is not re-checked (carried gap).
- **Empty-body → `400 VALIDATION_FAILED`** (`:39`); envelope via `sendError`. Carry.
- **Dynamic SET-clause** built from `Object.entries(updates)` (`:48-62`, `local/no-raw-sql`-disabled):
  column names come from the Zod-validated allowlist (injection-safe), values parameterized. Port as
  a **fixed column allowlist match** (enum → column), never string interpolation (Q6) — behavior-
  identical, structurally safer.
- **Interaction with money invariants (`crates/domain`):** S3 only validates+persists; **the server
  remains authoritative on price/total at order time** (`money.rs:11-14` — "not the price the server
  actually charges"). No total is computed in S3. The E2E delta to prove: a fee/tax edit changes the
  next order's total via the S5 path, not via S3.

## 5. Concern 3 — `menu-confirm.ts` · food-safety/liability gate (census row 112)

`POST /api/owner/locations/:locationId/products/:productId/confirm-allergens`
(`menu-confirm.ts:10-27`) is the ADR-0014 CC3 owner-authored allergen confirmation. The
**load-bearing invariant**: it flips `allergens_confirmed = true` **ONLY** and **never touches
`source`** (`:20`) — `source` preserves the AI-vs-owner provenance/liability audit and gates the C2
storefront read-gate (which strips allergens until confirmed). Port contract:

- **FIX-proof the column set.** The UPDATE mutates exactly `{allergens_confirmed}`. This is CARRY
  behavior with a **mandatory guardrail test** (assert the emitted SQL / the write set is exactly that
  one column) — corrupting `source` here would leak AI-authored allergens to the public storefront
  (threat S3-T5). 🔴.
- **Ownership fold-in.** `WHERE id = $1 AND location_id = $2` (`:21`) is the tenant scope; 0 rows → 404.
  Carry, under the §3 `with_user` seam.
- **Inconsistent error envelope (census row 112):** returns bare `reply.code(404).send({error:
  'PRODUCT_NOT_FOUND'})` and `reply.code(200).send({confirmed:true})` — not the `sendError`
  convention. **Decision: CARRY** (default; cf. S2 Q4 — divergent shapes carry to a post-Astro
  FE-lockstep pass). Flagged inline-fix candidate, not a port fix.
- **No FE caller found** (census: grep for `confirm-allergens`/`allergens_confirmed` in `apps/web/src`
  hit nothing — likely unwired despite the data model shipping). This makes the route's S3 scope a
  question (Q4 🔴): port-dark-with-guardrail vs defer-until-FE-wired vs RETIRE-pending. It reads
  `request.user.sub` (not `.userId`) for the owner id — the only owner route that does; `sub===userId`
  on owner tokens (S2 §5), so the extractor must expose a canonical owner id (Q8). Carry.

## 6. Concern 4 — `menu-import.ts` commit · bulk-edit + PII-in-uploads (census rows 90–92)

`menu-import.ts` is three routes: `/preview` (OWNER), `/anonymous` (**PUBLIC, unauthenticated
OCR/LLM front door**), `/commit` (OWNER, 🔴 bulk-edit). The commit
(`menu-import.ts:231-589`) is a single large `withTenant` bulk upsert over
categories→products→modifier-groups→modifiers→links, with a `mode==='replace'` arm that
**mass-DELETEs** rows not in the draft (guarded 409 if `order_items` reference a to-be-deleted product,
`:442-451`).

**Recommendation: DEFER menu-import to its OWN slice (post-S4 media), NOT S3.** Rationale:
1. **Heavy pipeline, wrong crate.** Preview/anonymous run the AI-OCR/LLM parser + storage put; the
   media/OCR stack (libvips FFI, Tesseract sidecar, pdfium) is the `media-worker` image, built in
   **S4** (REBUILD-MAP §2/§3). Porting commit in S3 either drags the media pipeline forward or ports a
   half-surface whose preview lives elsewhere.
2. **The nested-transaction quirk is a real port hazard.** The commit does outer `withTenant` (BEGIN +
   set_config) **then an inner explicit `client.query('BEGIN')`** and manual `COMMIT`/`ROLLBACK`
   inside the callback (`:254,265,509`). In Postgres the nested BEGIN is a warn-and-noop; the inner
   `COMMIT` commits the OUTER txn and resets the `is_local` GUC. This does **not** map onto the sqlx
   `Transaction` RAII guard `with_user` uses — porting it needs a deliberate single-txn rewrite
   (behavior-identical result, structurally different), which is exactly the kind of change that wants
   its own council, not a rider on the first-writes surface.
3. **Public unauthenticated OCR = a DoS/PII surface** (census 🔴 row 91): `/anonymous` takes a ≤10MB
   file with no auth, parses it through AI-OCR, and stashes PII (extracted name/address/phone) in
   Redis. That is a distinct threat model (rate-limit, claim-check, abuse) better owned with the media
   slice than bootstrapped into catalog CRUD.

If the council insists menu-import lands in S3, it must come with: the single-txn rewrite (Q5),
the replace-mode `order_items` guard verbatim, the `42703 → 500 MISSING_COLUMN` and `23505/23503/23502`
mappings (`:574-587`), and the public-OCR threat rows. Default recommendation stands: **DEFER (own
slice), matrix disposition `DEFER-FLAG`**, with the parity E2E (`flow-onboarding-parsing.spec.ts`)
staying green on Node until then.

---

## 7. Quirk register — carry-vs-fix (default = CARRY-VERBATIM)

**FIX-IN-PORT only for 🔴 security-correctness or a build-correctness bug in the Rust scaffold, each
with an explicit test/E2E delta.** Everything else CARRIES; shape-migration rows defer to post-Astro.

| ID | Quirk (source) | Disposition |
|---|---|---|
| Q-GUC-FAMILY | owner path seats `app.user_id`=userId (`tenant.ts:11`); Rust `db.rs::with_tenant` seats `app.current_tenant` from a `TenantId` (wrong root+value for owner writes) | **FIX (build-correctness):** add `with_user(UserId)`; do NOT reuse `with_tenant` for S3. Pinned-SQL test + NOBYPASSRLS integration test |
| Q-IS-LOCAL | `set_config(..., true)` txn-local (`tenant.ts:11`) | **CARRY** — port verbatim; pinned-SQL test (latent class 1) |
| Q-EXPLICIT-PREDICATE | every owner write also carries `WHERE location_id=$N` + child-ownership fold-in (belt) | **CARRY verbatim** — holds independent of pool role; rls-adversarial `routes/owner/**` gate |
| Q-SUB-VS-USERID | `menu-confirm.ts:17` reads `user.sub`; all others read `user.userId` (`sub===userId` on owner tokens) | **CARRY** — extractor exposes one canonical owner id (Q8) |
| Q-DUAL-GUC | some non-catalog owner routes also set legacy `app.current_tenant`; catalog tables use `app.user_id` only | **CARRY**; confirm at port no S3-touched policy reads `app.current_tenant` (Q7) — else dead weight |
| Q-DYNAMIC-SET | `locations`/`products`/`categories` build dynamic `SET k=$n` from validated keys (`locations.ts:48-62`) | **CARRY behavior; FIX mechanism** — fixed column allowlist match, no interpolation (Q6) |
| Q-TAX-RATE-FLOAT | `tax_rate` is percent `0-100` float, not integer money; flat fees are int minor units (`locations.ts:23-27`) | **CARRY** — `tax_rate: f64`/numeric; fees `Lek`/i64 |
| Q-LOCALE-PARTIAL | `default_locale∈supported_locales` checked only when both in the same body (`locations.ts:42-46`) | **CARRY** — known gap, parity |
| Q-CONFIRM-ENVELOPE | menu-confirm bare `{error:'PRODUCT_NOT_FOUND'}`/`{confirmed:true}` non-envelope (row 112) | **CARRY** — post-Astro FE-lockstep; inline-fix candidate |
| Q-SOURCE-PROVENANCE | confirm flips `allergens_confirmed` only, never `source` (ADR-0014) | **CARRY behavior + FIX-proof:** guardrail asserts write-set = `{allergens_confirmed}` (🔴 S3-T5) |
| Q-NESTED-BEGIN | menu-import commit nests `BEGIN` inside `withTenant` + manual COMMIT/ROLLBACK (`menu-import.ts:254,509`) | **FIX (structural)** on port to a single sqlx txn — reason to DEFER (Q5) |
| Q-REPLACE-MASSDELETE | `mode=replace` mass-DELETEs non-draft rows, 409 if `order_items` exist (`:432-473`) | **CARRY verbatim** — the historical-order guard is load-bearing (🔴 S3-T6) |
| Q-ANON-PUBLIC-OCR | `/anonymous` PUBLIC unauth AI-OCR + PII→Redis (row 91) | **DEFER** to media slice — distinct threat model (Q5, 🔴 S3-T7) |
| Q-IMPORT-ERRCODES | `42703→500 MISSING_COLUMN`, `23505/23503/23502` maps, `410 EXPIRED`, `422 LOW_CONFIDENCE` (`:574-587,281-299`) | **CARRY** with menu-import whenever it ports |
| Q-AUTOBRAND | post-commit best-effort website-fetch auto-brand outside the txn, swallows errors → `branding_generated` (`:533-569`) | **CARRY** — ports with menu-import |

## 8. Cutover DoD (REBUILD-MAP §3, this surface)

Catalog E2E slice green (as-is specs — `flow-ui-owner-crud`, `flow-modifiers-promotions`,
`flow-ingredients`, `ui-improvements`, `flow-ui-admin-branding`) · `openapi-diff` empty for the S3
namespace · invariant-cluster red→green: **`with_user` seats `app.user_id` (not `app.current_tenant`)
under a live NOBYPASSRLS probe** · owner-write blocked for a removed owner (P-d) · `routes/owner/**`
WHERE-`location_id` rls-adversarial gate green · menu-confirm write-set = `{allergens_confirmed}` ·
locations PATCH validation vectors (int fees, float tax, empty-body 400, locale rule) · map-coverage
zero-diff for the S3 namespaces · **council sign-off + rollback plan** (flag flip back to Node behind
the proxy). No 🔴 S3 row builds before this packet is APPROVED and the 🔴 questions are operator-signed.

---

**council seats to run: breaker, counsel** (architect authored; human operator signs 🔴 Q1/Q2/Q4/Q5).
**packet-status: 🟡 DRAFT.**
