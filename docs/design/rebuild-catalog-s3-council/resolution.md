# S3-CATALOG Port — Council RESOLVE

> **Verdict: PROCEED-WITH-REVISIONS.** No ETHICAL-STOP (counsel). Packet-status: **🟢
> COUNCIL-APPROVED — operator signed all §4 🔴 items in-session 2026-07-04 ("do this by yourself",
> direct reply to the enumerated §4 list).** Seats: architect (packet author), breaker
> (2 CRIT / 4 HIGH / 3 MED / 2 LOW), counsel (PROCEED-WITH-REVISIONS), lead (this RESOLVE).
> Build lane note: the parallel S3 build lane was course-corrected in-flight to REV-1/REV-2/REV-4
> and the Q1(a) `with_user` seam; SSG gate PASS ×2 (guardian 8/8 high-conf, sentinel 0 CRIT/HIGH);
> domain money/order_status/tenant byte-identity lead-verified (0-byte diff vs b28b1764).

## 1. Frozen revision set (REV-1..REV-10 — must fold before COUNCIL-APPROVED)

- **REV-1 (breaker C1+H4 → CRIT).** The per-request membership authorization read (old
  `requireLocationAccess`/`getLocationId`, `apps/api/src/plugins/auth.ts:148-151`) must run **inside
  the seated `with_user` transaction** with the explicit predicate (`user_id AND location_id AND
  role='owner' AND status='active'`), never on the raw pool. S2's `OwnerClaimsExt` is role-narrow
  only and provides no membership re-read — S3 adds an explicit `require_membership` step per op.
  Silent drop would regress ADR-0004.
- **REV-2 (breaker C2 → CRIT).** `themes` is a sanctioned **FIX-IN-PORT divergence**: the old raw-pool
  GUC-less `db.connect()` path is one of the never-copy leak classes — Rust themes ops go through
  `with_user` + membership like every other op, with a `CARRY-DIVERGENCE` note. `theme_versions`
  carries a third GUC family (`request.jwt.claim.sub` `TO authenticated` — PostgREST-style, dormant
  for our roles): recorded in the quirk register; its post-flip semantics join the B3-flip checklist.
  Q7 coverage must enumerate **all** S3-touched tables including `theme_versions`.
- **REV-3 (breaker H1 → HIGH).** Packet claim corrected: tables with `public_select FOR SELECT
  USING(true)` (products/categories/locations/menu_schedules) make owner **reads** RLS-free — SELECT
  visibility is NOT a valid B3 probe signal there. Belt-and-suspenders for reads = explicit `WHERE`
  + membership check (app-level). The probe targets `WITH CHECK` on writes and row-visibility only
  on tables without a public-select arm.
- **REV-4 (breaker H2 → HIGH).** For top-level `INSERT … VALUES` there is no `WHERE`; the enforced
  invariant is: in-txn membership precondition (REV-1) **and** `location_id` bound from the validated
  path param, never from the body. The DoD's "every write carries WHERE location_id" gate is reworded
  accordingly (it was vacuous on exactly the statements where `WITH CHECK` is the sole post-flip boundary).
- **REV-5 (breaker H3 → HIGH).** The NOBYPASSRLS probe cannot impersonate the real flip subject
  (`dowiz_app` is BYPASSRLS, out-of-band). Probe role = a NOBYPASSRLS substitute with **documented
  grant/`TO` divergences**; what the probe proves is (i) `app.user_id` is seated on every tenant-table
  txn and (ii) writes satisfy `WITH CHECK` under a policy-enforcing role — NOT production-role
  equivalence. Final flip readiness remains owned by the B3 council (dependency: the
  `app_member_location_ids()` search_path pin). Q2 wording softened from "provably correct" to
  "probe-verified within stated divergences".
- **REV-6 (breaker M1 → MED).** `locations` PATCH tri-state nullability (absent ≠ null ≠ value; the
  live dynamic-SET keeps vs clears) must be modeled explicitly in the deferred locations op spec
  (double-`Option`/presence tracking) so clear-vs-keep parity holds. Applies when that 🔴 op builds.
- **REV-7 (breaker M3+L2, counsel #2 → cutover).** Two-writer window: menu-import stays on Node
  (Q5 defer RATIFIED) while Rust owns catalog writes — the `external_key=NULL` bulk-delete hazard and
  S1 menu TTL-cache staleness are both **cutover DoD items**: read-after-write invalidation story +
  the replace-mode guard verbatim on the Node side. Cutover flip = a separate explicit operator
  go/no-go, flipped **atomically per-surface** (catalog is an edit-session surface, not stateless —
  inverts the S2 per-request canary).
- **REV-8 (counsel #1, Q4).** menu-confirm guardrail proves the **consequence, not the SQL text**:
  post-state assertion that after confirm, `source` is byte-identical and the write-effect is exactly
  `{allergens_confirmed}` — run under the NOBYPASSRLS probe. The pre-existing "confirm-of-blank
  publishes owner-warranted safe" hole is registered (owner: S3 lead; trigger: FE wires confirm) —
  carried, not a port-blocker.
- **REV-9 (counsel #3, Q5).** Defer compensating control = regression pins on the Node menu-import
  while it lives on: preview TTL ≤ 30 min, rate-limit present (5/min preview, 1/min anon), provider
  stays **local** (a swap to a cloud LLM = red-line crossing requiring its own council).
- **REV-10 (counsel #4, Q1).** `UserId` and `TenantId` are non-confusable types with **no conversion
  path** — a wrong-context call must be a compile error, not a post-flip 0-rows. The tenancy-GUC
  contract (which GUC family per surface family) is grafted into an ADR so S4–S7 inherit it without
  re-litigating.

## 2. Breaker disposition table

| # | Sev | Disposition |
|---|-----|-------------|
| C1 | CRIT | ACCEPTED → REV-1 (folded into build lane in-flight) |
| C2 | CRIT | ACCEPTED → REV-2 (fix-in-port sanctioned; theme_versions to quirk register + B3 checklist) |
| H1 | HIGH | ACCEPTED → REV-3 (packet overclaim corrected; probe design amended) |
| H2 | HIGH | ACCEPTED → REV-4 (INSERT invariant = membership + path-bound location_id) |
| H3 | HIGH | ACCEPTED-WITH-SCOPE → REV-5 (probe semantics corrected; flip readiness stays with B3 council) |
| H4 | HIGH | ACCEPTED → REV-1 (no existing extractor provides P-d; explicit membership step) |
| M1 | MED | ACCEPTED → REV-6 (locations op spec, builds only after 🔴 sign-off) |
| M2 | MED | NOTED-CARRY — repo layer keeps bare `Option<i64>` per S1 precedent; `Lek` stays a domain-boundary type; any change belongs to the S5 money council |
| M3 | MED | PREMISE-CORRECTED (GUC-family claim false) / hazard ACCEPTED → REV-7 |
| L1 | LOW | NOTED — `.sub`≡`.userId` by construction; Q8(a) single accessor stands |
| L2 | LOW | ACCEPTED → REV-7 (cache read-after-write gate joins cutover DoD) |

## 3. Question resolutions

- **Q1 → (a)** `with_user` combinator + REV-10 type separation. 🔴 operator.
- **Q2 → (a)** as amended by REV-3/REV-5. 🔴 operator.
- **Q3 → (a)** carry the divergent menu-confirm envelope verbatim.
- **Q4 → (a)** port dark with the REV-8 post-state guardrail. 🔴 operator.
- **Q5 → (a)** menu-import defers to its own slice post-S4; Node keeps serving it under REV-9 pins. 🔴 operator.
- **Q6 → (a)** fixed column-allowlist enum, values parameterized, zero interpolation.
- **Q7 → (a)** amended by REV-2: enumerate ALL S3-touched tables incl. `theme_versions`; seat exactly
  what the live policy catalog reads; dual-keyed tables (if found) seat both.
- **Q8 → (a)** one canonical `owner_id()` accessor on the owner-narrowed claims.

## 4. 🔴 OPERATOR SIGN-OFF REQUIRED (blocks COUNCIL-APPROVED → commit of S3 write code)

1. **Q1(a)** — `with_user(app.user_id)` as the owner-write seam (REV-10 types; ADR to follow).
2. **Q2(a)** — belt-and-suspenders + NOBYPASSRLS probe with REV-5's stated scope limits.
3. **Q4(a)** — port menu-confirm dark with the REV-8 food-safety guardrail.
4. **Q5(a)** — defer menu-import to its own slice; Node serves it under REV-9 pins meanwhile.
5. **Cutover posture (REV-7)** — per-surface atomic flip, separate operator go/no-go, cache +
   two-writer DoD items.

## 5. Build/cutover DoD deltas (added by this RESOLVE)

- NOBYPASSRLS probe in the invariant cluster asserting GUC-seating on every tenant-table txn (REV-5 scope).
- menu-confirm post-state guardrail test (REV-8).
- Node menu-import regression pins (REV-9).
- Cutover: read-after-write cache invalidation gate + two-writer hazard items (REV-7).
- ADR: tenancy-GUC contract per surface family (REV-10).
