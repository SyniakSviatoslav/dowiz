# S3-CATALOG Port — Council Packet · OPEN QUESTIONS

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Everything the live Triadic Council (architect / breaker /
> counsel / human) must decide before S3 catalog is ported. Each question has options + a lane-R3
> recommendation — a *starting position for friction*, not a decision. Docs only.

Legend: **[SEC]** security-correctness · **[CONTRACT]** shape/parity · **[INFRA]** topology ·
**[SCOPE]** surface placement · **[SAFETY]** food-safety/liability. 🔴 = red-line, operator sign-off required.

---

### Q1 🔴 [SEC] The owner-write GUC family — add `with_user`, do NOT reuse `with_tenant`
Owner-path RLS keys on `app.user_id`→memberships (~34 sites). The existing Rust `db.rs::with_tenant`
seats `app.current_tenant` from a `TenantId` (a location id) — the **courier/service** root (~102
sites). Reusing it for S3 seats the wrong GUC family with the wrong value.
- **(a)** Add a `with_user(pool, UserId, …)` combinator (BEGIN → `set_config('app.user_id',$1,true)`
  → work → commit); route all owner writes through it; keep `with_tenant`/`TenantId` for S6/S7. *(recommend)*
- **(b)** Generalize into `with_ctx(GucCtx::User|Tenant, …)` one combinator, two variants.
- **(c)** Reuse `with_tenant` and pass the userId as a `TenantId` — **rejected** (wrong root; masked
  today by BYPASSRLS, silent total owner-write break post-flip).

**R3 recommendation:** (a). Two distinct roots, two named combinators, raw pool unreachable from route
code. (b) is acceptable if the seam is one type with a compiler-checked variant; (c) is the trap.

### Q2 🔴 [INFRA/SEC] B3-flip readiness — prove S3 writes correct under NOBYPASSRLS *before* the flip
Post-flip, `INSERT` needs `app.user_id` seated to satisfy `WITH CHECK` (products/categories);
`UPDATE/DELETE/SELECT` need it to see the row. Two B3-council rows gate correctness: the
`app_member_location_ids()` unpinned-`search_path` pin (sweep #3) and GUC-always-seated (sweep #2).
- **(a)** Build S3 belt-AND-suspenders (explicit `WHERE location_id` + seated `app.user_id`), proven by
  a NOBYPASSRLS integration probe in the invariant cluster, independent of when B3 actually flips. *(recommend)*
- **(b)** Defer NOBYPASSRLS correctness to the B3 council; S3 relies on the explicit predicate only.

**R3 recommendation:** (a). S3 must not *rely* on the flip to fix a missing predicate, nor ship a
port that only works pre-flip. The DoD names a live NOBYPASSRLS probe that asserts `app.user_id` is
seated on every tenant-table txn. Owner: S3 lead + B3-council for the search_path pin dependency.

### Q3 [CONTRACT] menu-confirm error envelope — carry or normalize (census row 112)
Bare `{error:'PRODUCT_NOT_FOUND'}` / `{confirmed:true}` vs the `sendError` envelope everywhere else.
- **(a)** Carry verbatim; normalize in a post-Astro FE-lockstep pass. *(recommend)*
- **(b)** Normalize to the envelope now.

**R3 recommendation:** (a). Same posture as S2 Q4 — divergent shapes carry; migrating during the port
couples two risky changes. Inline-fix candidate, not a port fix.

### Q4 🔴 [SAFETY] menu-confirm scope — port-dark, defer, or RETIRE-pending? (no FE caller found)
The route is 🔴 food-safety but has **zero FE callers** (census grep miss) despite the ADR-0014 data
model shipping. The load-bearing invariant (flip `allergens_confirmed` only, never `source`) still matters.
- **(a)** Port dark in S3 with the write-set guardrail (assert only `allergens_confirmed` mutates),
  ready for FE wiring. *(recommend)*
- **(b)** Defer until the FE actually calls it (matrix `DEFER-FLAG`).
- **(c)** RETIRE-pending with proof-of-deadness — **rejected**: the data model is live and the
  read-gate depends on the flag; the route is the only way to set it.

**R3 recommendation:** (a). Low surface, high liability — port it correctly with the guardrail now so
the invariant is machine-enforced, rather than leaving an unwired 🔴 to be re-derived later.

### Q5 🔴 [SCOPE] menu-import.ts placement — S3 now, S4 media, or its own slice?
Heavy AI-OCR/LLM pipeline (S4 `media-worker` image), a nested-transaction quirk that needs a
single-txn rewrite, and a PUBLIC unauthenticated OCR/PII front door.
- **(a)** Its own slice, post-S4 media; keep Node serving it (parity E2E green) until then. *(recommend)*
- **(b)** Fold into S4 with the media pipeline.
- **(c)** Port in S3 now — carries the media stack forward + rides the first-writes surface with a
  txn rewrite; not recommended.

**R3 recommendation:** (a). Matrix `DEFER-FLAG`. If overridden to (c), it must come with the single-txn
rewrite (Q-NESTED-BEGIN), the replace-mode `order_items` guard verbatim, and the public-OCR threat rows.

### Q6 [CONTRACT] Dynamic SET-clause port mechanism (locations/products/categories PATCH)
Live builds `SET k=$n` from validated keys (injection-safe; `local/no-raw-sql`-disabled).
- **(a)** Fixed column allowlist match (enum → column), values parameterized — no interpolation. *(recommend)*
- **(b)** A runtime query builder that interpolates validated keys (closer to the JS shape, weaker guarantee).

**R3 recommendation:** (a). Behavior-identical, structurally injection-proof by construction; the
Zod-key allowlist becomes a Rust enum the compiler checks.

### Q7 [SEC] The legacy `app.current_tenant` on catalog tables — read anywhere, or dead weight?
Some non-catalog owner routes also set `app.current_tenant`. Confirm whether any RLS policy on the
S3-touched tables (products/categories/modifier_groups/modifiers/product_modifier_groups/
product_translations/location_themes/menu_schedules/import_sessions) reads `app.current_tenant`.
- **(a)** Confirm at port from the live policy catalog (Phase-0 `ci-schema-drift` diff); if none, S3
  seats `app.user_id` only. *(recommend)*
- **(b)** Seat both GUCs defensively on every owner write (belt), accepting one extra `set_config`.

**R3 recommendation:** (a). Seat exactly what the policies read; over-seating hides a real
misconfiguration. If any dual-keyed policy is found, that table's writes seat both.

### Q8 [CONTRACT] Canonical owner-id claim field — `.sub` vs `.userId`
menu-confirm reads `user.sub`; every other owner route reads `user.userId`; they are equal on owner
tokens (S2 §5).
- **(a)** The `Claims<Owner>` extractor exposes one canonical owner-id accessor; both call sites use it. *(recommend)*
- **(b)** Port each call site reading its literal field name.

**R3 recommendation:** (a). One accessor removes a class where a future edit reads the wrong field; the
S2 type-state extractor already narrows to owner, so a single `owner_id()` is natural.

---

## Decision-ordering note for the council
**Q1** (`with_user` GUC family) and **Q2** (B3 NOBYPASSRLS readiness) are **port-blocking** — no S3
write builds before they are settled, because they define the write seam itself. **Q4** (menu-confirm
scope) and **Q5** (menu-import placement) are **scope-blocking** for those two rows but not the whole
surface. Q3/Q6/Q7/Q8 are build-detail decisions that can settle at build time without blocking approval.
