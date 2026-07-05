# S3-CATALOG Port — Council Packet · THREAT MODEL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Adversarial input to the S3 council. Assets, trust
> boundaries, and the failure modes the Rust port must not silently introduce. Read alongside
> `proposal.md`. Docs only; no code.

- **Method:** STRIDE-lite over the S3 owner-write surface + fold-in of the RLS-reliability council
  (`audit-fix-rls-reliability/`, the anonymizer N1 / GDPR-worker wrong-GUC CRITICAL) + inventory/12 §7
  latent GUC-bug classes + the money-newtype council.
- **Scope note:** the B3 (NOBYPASSRLS) flip and the `app_member_location_ids()` search_path pin are
  **B3-council fixes**; their *disposition* is recorded here because they change what S3 must hold, but
  their *fix* is in that council.

---

## 1. Assets

| ID | Asset | Where it lives | Why it matters |
|---|---|---|---|
| C1 | Catalog rows (products/categories/modifiers/links, prices, allergens) | tenant tables, RLS-scoped | The menu the storefront serves; wrong-tenant write = cross-tenant menu corruption |
| C2 | Location pricing inputs (`tax_rate`, `delivery_fee_flat`, `free_delivery_threshold`, `min_order_value`) | `locations` | Direct upstream of every S5 order total; tampering shifts real charges |
| C3 | Allergen provenance (`allergens_confirmed`, `source`) | `products` | ADR-0014 liability audit; corrupting `source` leaks AI-authored allergens to the public menu |
| C4 | Import draft + extracted venue PII (name/address/phone) | `import_sessions` (auth), Redis (anon) | Bulk menu overwrite; unauthenticated PII intake |
| C5 | `menu_version` / `menu_confirmed_at` publish gate | `locations` | Draft→live transition; commit stamps the human-review gate |

## 2. Trust boundaries

- **TB-1 owner → location** — ADR-0004 P-d live `status='active'` membership re-read on write routes
  (the `OwnerAt<Loc>` extractor + the `getLocationId()`/`getOwnerLocationId` DB-recheck helpers). The
  JWT `activeLocationId` claim is **never trusted** as authority (census note #3); the live memberships
  row is.
- **TB-2 app `WHERE location_id` predicate → RLS** — every owner write carries the explicit predicate
  **and** relies on RLS. Today (BYPASSRLS) the predicate is the only live boundary; post-flip RLS is
  authoritative *iff* `app.user_id` is seated. Belt AND suspenders — neither alone.
- **TB-3 request → GUC seat** — the `with_user` combinator is the boundary that turns a bearer identity
  into an `app.user_id` GUC inside one txn. A context-free connection (no txn / wrong GUC / leaked GUC)
  dissolves TB-2's RLS arm entirely.
- **TB-4 public upload → parser** (menu-import `/anonymous`) — unauthenticated bytes into AI-OCR;
  the request body IS the authority; only IP rate-limit + size cap gate it (deferred surface, §S3-T7).

## 3. Port-specific failure scenarios (Rust)

| # | Scenario | Trigger in the port | Mitigation to prove red→green |
|---|---|---|---|
| **S3-T1** | **Wrong GUC family** — owner write seats `app.current_tenant` (or a location id) instead of `app.user_id`=userId | Reusing `db.rs::with_tenant`/`TenantId` for owner writes (Q1) | Add `with_user`; NOBYPASSRLS integration probe asserts `current_setting('app.user_id')`=userId inside the write txn, and that an owner INSERT/UPDATE affects exactly the intended rows. Post-flip a wrong family = 0 rows / `WITH CHECK` fail = total silent owner-write outage (anonymizer N1 replayed) |
| **S3-T2** | **Context-free connection** — a tenant table touched via `pool.acquire()` with no txn+set_config | A catalog handler reaching the raw pool | Raw pool unreachable from route code (`Pools` `pub(crate)`); `with_user`/`with_tenant` the only paths. Guardrail: no `pool.acquire()` in `routes/owner/**`. This is the anonymizer N1 CRITICAL ("order read left context-free") structurally refused |
| **S3-T3** | **Latent GUC leak copied** — `set_config(false)` / no-BEGIN / different-physical-conn ports forward | Copying an inventory/12 §7 class-1/2/3 site verbatim | `with_user` uses `is_local=true` inside its own BEGIN on one `&mut txn`; pinned-SQL test. S3 touches none of the flagged files; the *pattern* is refused by construction |
| **S3-T4** | **Money-input tampering** — a crafted `locations` PATCH sets an out-of-range fee/tax to shift order totals | Lenient validation drift (missing `.strict()`, wrong bounds, float where int) | Port validation exactly: fees `int ≥ 0` (nullish for min/threshold), `tax_rate` `0-100` float, empty-body 400, locale rule. Server stays authoritative on the *total* at order time (S5, `money.rs:11-14`); S3 only stores. E2E: a fee edit changes the next order total via S5, not S3 |
| **S3-T5** | **Allergen provenance corruption** — confirm route (or a shared UPDATE helper) touches `source` | Broadening the write set beyond `{allergens_confirmed}` | Guardrail asserts the emitted write-set is exactly one column; `source` untouched. Corrupting it flips the C2 read-gate → AI-authored allergens reach the storefront (food-safety) |
| **S3-T6** | **Destructive replace with live orders** — `mode=replace` mass-DELETEs a product referenced by historical `order_items` | Dropping the `order_items` existence guard (`menu-import.ts:442-451`) on port | Carry the guard verbatim: 409 `REPLACE_BLOCKED_BY_HISTORICAL_ORDERS` before any DELETE; the whole delete arm is inside the seated txn |
| **S3-T7** | **Public-OCR DoS / PII intake** — unauthenticated `/anonymous` floods the OCR/LLM path or exfiltrates via crafted uploads | Porting the public front door into S3 without its threat model | **Deferred with menu-import to its own slice (Q5).** Its rate-limit/claim-check/abuse model is owned there, not bootstrapped into catalog CRUD |
| **S3-T8** | **Cross-tenant child-write via external_key/group_id** — an owner links a foreign category/group/product | Dropping the child-ownership fold-in (INSERT…SELECT WHERE location_id / same-tx pre-check) | Carry the fold-ins verbatim (`products.ts:324-335`, category/group existence bounded by `location_id`; menu-availability R2-1 IDOR fix). A foreign key inserts 0 rows → 400/404, never a cross-tenant link |

## 4. What the B3 RLS flip changes for S3

- **Today (BYPASSRLS):** RLS is bypassed; the explicit `WHERE location_id` is the only live boundary.
  A `with_user` that seated the wrong/no GUC still "works" — the danger is invisible.
- **Post-flip (NOBYPASSRLS):** RLS is authoritative. Owner `SELECT/UPDATE/DELETE` need `app.user_id`
  seated to see the row; **`INSERT` needs it to pass `WITH CHECK`**. Two B3-council fixes gate this and
  are named here (not fixed here): the `app_member_location_ids()` unpinned-`search_path` pin (sweep
  #3) — S3 CALLS this definer transitively via every member-keyed policy — and the GUC-always-seated
  invariant (sweep #2). **S3's rule: correct independent of which pool role is live.** The DoD adds a
  NOBYPASSRLS probe so the port cannot false-green under BYPASSRLS masking.

## 5. Residual risks (summary for the human)

- **≤24h leaked-owner-access write window** (ADR-0004 accepted) — a removed owner is blocked
  immediately by P-d on write routes, but a still-valid access token writes until expiry only on the
  ≤24h window; unchanged by the port.
- **menu-confirm unwired** (no FE caller) — the 🔴 gate exists in the API but nothing calls it yet
  (Q4); porting it dark with a guardrail keeps the invariant machine-enforced.
- **Public-OCR PII surface deferred** (S3-T7 / Q5) — carried on Node until its own slice; not
  regressed, not advanced.
- **`tax_rate` as float** — a rate, not `Lek` money; the `2^53` JSON-precision limitation
  (`money.rs:15-22`) applies to the i64 fee fields at any browser-facing boundary, owned by S5.

None of these is introduced by the rewrite; each is a *current* property. The port's job is to carry
them **visibly** (matrix row + test) and to let the council promote any to a FIX-with-E2E-delta on the
record. **No ETHICAL-STOP surface identified in S3** (catalog CRUD; no military/harm capability, no new
PII egress beyond the deferred public-OCR intake already carried on Node).
