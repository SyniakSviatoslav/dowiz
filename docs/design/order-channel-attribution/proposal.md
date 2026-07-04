# Design Proposal — Order acquisition-attribution `channel` (write-only metadata)

- Status: **PROPOSED — RESOLVED** (design-time only; no production code in this change). Updated 2026-07-04
  after the breaker + counsel round; every finding dispositioned in
  `docs/design/order-channel-attribution/resolution.md`.
- Date: 2026-07-04 (revised post-review)
- Seat: System Architect (QR/ATTRIBUTION build lane)
- Relates: mig `1780421100057_anti-fake-signals.ts` (added `orders.metadata jsonb`); ADR-0016
  (checkout Communication); the OTP/velocity anti-abuse seam.
- ADR: `docs/adr/ADR-order-channel-attribution.md`
- Review artifacts: `breaker-findings.md`, `counsel-opinion.md`, `resolution.md` (this design reflects all
  resolved dispositions).

---

## 1. Problem + non-goals

**Problem.** We want to know *how a customer arrived* at a storefront order — scanned a table QR, tapped
an NFC tag, followed a Google Business Profile link, came from Instagram, opened the Telegram mini-app,
used an embedded widget, etc. Today the order-creation contract carries no acquisition-source field, so
every order is attributionally anonymous. Owners (and, later, our own analytics) cannot tell paid/organic
QR/social channels apart.

The ask is a single, **purely descriptive** `channel` tag captured from a `?ch=<value>` link param on the
storefront and threaded — write-only — into the row we already insert per order.

**Non-goals (explicit):**
- **NOT** a pricing/fee/tax/discount input. `channel` must never be read by the price engine.
- **NOT** a status/state-machine input, dispatch/courier-assignment input, or authz/RLS predicate.
- **NOT** a new DB column and **NOT** a migration (this lane cannot touch `packages/db/migrations/`).
- **NOT** a new npm dependency (this lane cannot touch any `package.json`).
- **NOT** an analytics/reporting build. Building a channel-breakdown UI / dedicated read surface is out of
  scope here (§9 notes the query). (Note: the owner dashboard already forwards the whole `metadata` object
  to the owner for their own orders — a benign same-tenant passthrough, §8/M1 — so `channel` becomes
  visible there day 1; that is expected, not a designed feature.)
- **NOT** a free-text field. Fixed 13-value allowlist only; no owner-authored channels in v1.
- **NOT** PII. The value is one of 13 fixed tokens, tenant-agnostic, not personal data.

---

## 2. Back-of-envelope

**Data volume.** One write per created order into an **already-written** `metadata jsonb` blob (today it
holds `{ otp_verified, client_ip_hash }`). Adding `channel:'<token>'` costs ~20–30 bytes on that same blob.

- Longest token = `apple-maps`/`telegram-tma` = ~12 chars; key `"channel":""` ≈ 12 bytes ⇒ ≤ ~26 bytes/row.
- Order rate: even a generous ceiling — say **500 locations × 2 orders/min peak = 1,000 orders/min ≈
  17 orders/s** — adds ≈ **26 B × 17/s ≈ 442 B/s** of extra jsonb payload. At 1,000 orders/min for a full
  busy hour that is **~1.5 MB/hour** across the whole fleet. Effectively free.
- **Zero extra rows, zero extra INSERTs, zero extra round-trips, zero new index.** It is one more key in a
  string we already `JSON.stringify` and bind as one existing positional parameter (`$18`).

**Connection budget.** Unchanged. No new pool consumer (API / worker / analytics / migrations). This rides
inside the existing single `db.connect()` order transaction. Net conn delta = **0**.

**Client cost.** One `URLSearchParams.get('ch')` + one `sessionStorage` read/write on shell mount
(`capture(slug, search)`), and one `read(slug)` + one extra key on the checkout POST body. Sub-millisecond,
no network. The per-slug key (`dos_channel:<slug>`) adds no measurable cost.

Conclusion: the load-bearing engineering fact is that this is *additive to an existing write*. There is no
scaling question to answer — the cheapest option that satisfies the constraint is also correct.

---

## 3. Options (≥2, with named tradeoffs + concept)

### Option A — Fold into the existing `orders.metadata` jsonb (RECOMMENDED)
Add `channel` as one more key in the object already stringified into `orders.metadata` at INSERT.

- **Concept:** *schema-rich-runtime-minimal* / "seam in the schema, don't switch on it." The `metadata`
  column is a pre-existing, RLS-FORCE'd, per-order sidecar for exactly this class of descriptive signal.
- **(+)** No migration, no new column, no new dependency — satisfies every protected-path constraint.
- **(+)** Same row, same transaction, same tenant guard — no new consistency, RLS, or blast-radius surface.
- **(+)** Reversible by omission; nothing downstream reads it, so removing it later is a no-op.
- **(−)** Not first-class/queryable-by-index; analytics must reach through `metadata->>'channel'`. Accepted:
  no analytics is being built now, and a first-class column is drafted-but-not-built for later (§5).
- **(−)** Weak typing at rest (jsonb). Mitigated: the value is **Zod-enum-validated before it ever reaches
  SQL**, and the column already carries `CHECK (jsonb_typeof(metadata) = 'object')`.

### Option B — New dedicated `orders.channel text` column + CHECK/migration
A first-class `channel text NOT NULL DEFAULT 'web-direct'` with a `CHECK (channel IN (...))`.

- **Concept:** first-class relational modelling; queryable/indexable attribution dimension.
- **(+)** Strong typing at rest, trivially indexable, self-documenting in `\d orders`.
- **(−)** **Requires a migration** — `packages/db/migrations/` is a red-line/protected path for this lane
  (operator-gated). This alone disqualifies it *for this change*.
- **(−)** ALTER on the hottest table (`orders`) for a field with no current reader — premature. `NOT NULL
  DEFAULT` on a large table is a rewrite risk to reason about; no reason to pay it now.
- **(−)** Ratchets a new enum into a CHECK constraint that then needs a migration every time the allowlist
  grows — the Zod enum is a cheaper single source of truth while the field is write-only.

### Option C — Log-structured / audit-only capture (no order-row change)
Emit the channel to an append-only attribution log / event stream keyed by order id, never touching `orders`.

- **Concept:** event-sourced / claim-check side-channel; keep the order row pristine.
- **(+)** Zero coupling to the order row; clean separation of "facts about the order" from "the order."
- **(−)** Requires a **new table (migration) or a new sink/dependency** — again hits both protected paths.
- **(−)** Introduces a second write that can succeed/fail independently of the order INSERT → a *new*
  consistency concern (dual-write, needs outbox/transactional enqueue) for a field that is trivially
  co-locatable in the same row. Over-engineered for descriptive metadata.
- **(−)** Join-to-read for the simplest question ("what channel was this order?").

---

## 4. Decision + justification (ADR-format → `docs/adr/ADR-order-channel-attribution.md`)

**Decision: Option A.** Fold `channel` into the existing `orders.metadata` jsonb, written inside the
existing order INSERT. Validate the token against a single Zod enum (`OrderChannel`) shared by client and
server; server default `'web-direct'`; unknown/garbled → `'other'`.

**Why.**
1. It is the only option that respects **both** protected paths (no migration, no new dependency) — a hard
   constraint of this lane, not a preference.
2. `orders.metadata` was purpose-built (mig `…057`) as the per-order descriptive sidecar; this is exactly
   its intent. "Boring & proven > novelty."
3. It is *strictly additive to an existing write* — no new row/tx/pool/index/consistency surface (§2, §6).
4. The Zod enum is the cheapest correct source of truth while the field is write-only; it upgrades cleanly
   to a first-class column later (§5) without a client contract change (the wire shape is identical).

Rejected: **B** (needs a migration — out of lane; premature ALTER on hottest table with no reader);
**C** (needs a new sink/table + invents a dual-write consistency problem for co-locatable metadata).

---

## 5. Data / migration notes

- **NO migration in this change.** Zero DDL. `orders.metadata` already exists (mig `…057`) with
  `NOT NULL DEFAULT '{}'::jsonb` and `CHECK (jsonb_typeof(metadata) = 'object')`. Adding a string key keeps
  the value an object → the existing CHECK still holds; **no constraint touched.**
- **Write site:** `apps/api/src/lib/order-persistence.ts` — extend the existing
  `JSON.stringify({ otp_verified, client_ip_hash })` (bound as `$18`) to
  `JSON.stringify({ otp_verified, client_ip_hash, channel })`. No new positional parameter, no INSERT column
  list change.
- **Shape at rest:** `orders.metadata = { "otp_verified": <bool>, "client_ip_hash": <hash|undefined>,
  "channel": "<one-of-13>" }`.
- **Future first-class column (deferred, operator-gated):** if attribution ever needs to be indexed/queried
  at scale, draft — **do not build** — a forward-only migration adding `orders.channel text` (with a CHECK
  mirroring the enum, `DEFAULT 'web-direct'`, backfill from `metadata->>'channel'`) under
  `docs/design/channel-hub/migration-drafts/` for operator review. That is a separate change on the
  red-line path with its own ADR. **Nothing in the present change presumes or blocks it** — the wire
  contract is identical either way, so the migration can land later with zero client change.

---

## 6. Consistency + idempotency

- **Same transaction.** The `channel` write is part of the existing single `orders` INSERT, which the caller
  wraps in `BEGIN/COMMIT/ROLLBACK` (`insertOrderWithItems` deliberately does not own tx control). No new
  write, no dual-write, therefore **no new consistency concern** — it commits atomically with the order or
  not at all.
- **Idempotency preserved.** Order creation is idempotency-keyed (`idempotency_keys`, UUID
  `idempotency_key`). A replay returns the original order; `channel` is captured once at first insert and is
  not re-derived on replay. `channel` is **not** part of the `request_hash` / idempotency identity and must
  not be — two otherwise-identical bodies differing only in `channel` are the *same* order intent; folding
  it into the hash would wrongly split them. (Note for the impl lane: thread `channel` into the persistence
  input, but do **not** add it to the request-hash computation.)
- **Default determinism.** `.optional().default('web-direct')` means `input.channel` is always defined
  post-parse, so the server writes a concrete token even when the client omits it — no `null`/`undefined`
  drift into the jsonb.
- **Wire-optional / type-required is intentional and consistent (breaker L1).** `.optional().default(...)`
  makes `channel` omittable on the wire but **non-optional in the inferred `CreateOrderInput` output type**
  (post-parse it is always present). This is **not** a defect — it is the exact same pattern already used by
  the sibling field `acknowledged_codes: z.array(z.string()).max(10).optional().default([])` in the *same*
  schema. No live constructor/snapshot break exists today (breaker grep found none); the shape shift is
  expected and consistent with established convention. No fix.

---

## 7. Failure + degradation (failure-first)

The governing rule: **attribution never blocks an order.** Worst case it silently normalizes.

- **Garbled / unknown `?ch=` (e.g. `?ch=<script>`, `?ch=DROP`, `?ch=🍕`, 5 KB junk):** the client
  normalizer validates against the `OrderChannel` allowlist → unknown ⇒ `'other'`. Never reaches the server
  as-is. Even if a hand-crafted request sends a bad `channel`, the server `CreateOrderInput.parse` rejects
  it at the enum → 400 VALIDATION_FAILED (same path as any other bad field). Because a legitimate client
  always sends a valid token or omits it, no real user hits this.
- **Missing `?ch=`:** client sends nothing; server default `'web-direct'` applies. Existing clients that
  never send `channel` are **unaffected** — the field is optional with a safe default (invariant §8).
- **`sessionStorage` unavailable (private mode quota, SSR, disabled storage):** the client helper is wrapped
  in try/catch and returns `'web-direct'` on any read/write throw. Capture failing degrades to the default;
  it **never** throws into render or checkout submit.
- **Per-slug key scoping (breaker M2 fix).** The storage key is **scoped per storefront slug** —
  `dos_channel:<slug>` — never a single session-global key. Consequently the `channel.ts` API takes `slug`
  as a required parameter: **`capture(slug, search)`** (read `?ch=` from `search`, normalize, write under
  `dos_channel:<slug>`) and **`read(slug)`** (read `dos_channel:<slug>`, default `'web-direct'`). This
  closes the cross-storefront bleed: a customer who scans restaurant **A**'s QR (`/s/restA?ch=qr`) and later,
  in the same tab/session, opens restaurant **B**'s `/s/restB` with **no** `?ch=` reads `dos_channel:restB`
  (absent) → `'web-direct'` for B, and can never inherit A's `qr`. Semantics per slug: a mount **with**
  `?ch=` writes/overwrites that slug's key; a mount **without** `?ch=` leaves that slug's key untouched
  (first-touch-per-slug; R1). Checkout reads `read(slug)` for the slug it is checking out.
- **No cascade.** `channel` has zero downstream **decision** readers (§8; the only reader is the benign
  same-tenant owner-dashboard `metadata` passthrough, which makes no decision), so a wrong/missing value
  cannot affect price, status, dispatch, notifications, or authz. The blast radius of any bug here is "one
  order is tagged `web-direct`/`other` instead of its true source" — a lossy-analytics event, not an
  incident.
- **External calls:** none introduced. No new timeout/fallback surface.

---

## 8. Security + tenant isolation

- **Decision-path read prohibition (the load-bearing invariant).** `channel` is **never read by pricing
  (subtotal/fee/tax/total), the order-status state-machine, dispatch/courier-assignment, notifications, or
  any authz/RLS decision.** That — not "no reader anywhere" — is the invariant that matters and the one the
  impl lane must hold. (Correction, per breaker M1: the earlier "written once and never consulted / no
  reader anywhere" wording was factually wrong.) There **is** one benign, same-tenant reader today:
  `apps/api/src/routes/owner/dashboard.ts:112-131` already `JSON.parse`s `orders.metadata` and returns the
  whole object verbatim in the owner-dashboard response. So on day 1 after ship, an owner's own dashboard
  call surfaces `metadata.channel` for that owner's own orders. This is **expected and acceptable** — an
  owner reading acquisition metadata about their own orders is exactly the "QR-kit" owner value this feature
  exists to seed, and it is same-tenant (no cross-tenant read, no decision gate). **Out of scope for this
  pass:** building a dedicated channel-breakdown UI, label/i18n rendering, or any owner-facing analytics
  surface — the passthrough is a raw metadata echo, not a designed feature (see counsel advice B/C, §10).
  Analytics aggregation remains out of scope; when built it reads for reporting only, never for a decision
  gate.
- **Injection / XSS surface: none.** The value is `OrderChannel` Zod-enum-validated *before* it touches SQL,
  and the INSERT is a **parameterized query** (bound as `$18` inside `JSON.stringify`) either way. It never
  concatenates into SQL and is never rendered as HTML. Max length is bounded by the enum itself (≤ ~12
  chars) — the 5-KB-junk vector is closed at the enum.
- **Single source of truth.** The 13-value allowlist lives once, in `@deliveryos/shared-types`
  (`OrderChannel`). Client (`apps/web/src/lib/channel.ts`) imports it — **no duplicated list** to drift.
- **Tenant isolation unchanged.** `channel` is tenant-agnostic and rides in the same `orders` row, under the
  same location-scoped INSERT and the same ENABLE+FORCE RLS as every other order field. **No RLS bypass, no
  cross-tenant read/write, no new policy.** It cannot be used to escape or widen a tenant boundary because
  nothing reads it to make a scoping decision.
- **No PII / no AI egress *from `channel` itself*.** A fixed enum token is not personal data; nothing here
  adds a PII→AI path, and `channel` requires **no anonymizer change** to erase it (there is nothing to
  erase). **Correction, per breaker H1 / counsel Rec #2:** the earlier claim that the sibling
  `client_ip_hash` "is already handled" is only half true. `insertOrderWithItems` writes a *second* copy of
  `client_ip_hash` **inside `orders.metadata`** (`order-persistence.ts:95`), and
  `AnonymizerService.anonymizeOrder` (`anonymizer/index.ts:210-222`) nulls the dedicated **column**
  `client_ip_hash` but issues **zero writes against `metadata`** — so after a GDPR erasure the hashed IP
  survives in `metadata->>'client_ip_hash'`. This is a **pre-existing GDPR gap NOT introduced by this
  change** (`channel` adds no PII of its own). It is **explicitly ACCEPTED-as-risk for this build lane and
  flagged as a separate follow-up ticket for the GDPR/anonymizer owner** — see R4 (§10) and
  `resolution.md`. This lane does **not** touch `apps/api/src/lib/anonymizer/*`.
- **No secrets, no cookies.** Client persistence is `sessionStorage` under a **per-slug** key
  (`dos_channel:<slug>`, session-scoped, per-tab), not a cookie and not `localStorage` — a fresh tab/session
  re-attributes, it never becomes a tracking cookie, and it never bleeds one storefront's channel onto
  another (breaker M2).

---

## 9. Operability

- **Health / degraded-vs-down:** N/A as a new subsystem — no worker, no external dependency, no new health
  signal. A failure here is invisible to liveness by design (§7 degrades to a default).
- **Observability (<1 min to answer "what channel was this order?"):** read straight off the row —
  `SELECT id, metadata->>'channel' AS channel FROM orders WHERE id = $1;` Aggregate (future analytics,
  out of scope to build):
  `SELECT metadata->>'channel' AS channel, count(*) FROM orders WHERE location_id = $1 GROUP BY 1 ORDER BY 2 DESC;`
  (a functional/expression index on `(location_id, (metadata->>'channel'))` would be part of the deferred
  first-class-column migration, not this change).
- **Rollback:** trivial and safe. Reverting the three code edits stops new writes; already-written keys are
  inert (no reader) and harmless. No data migration to unwind, no down() to run.
- **Flag / scaling-gate:** none required — the field is inert (write-only, safe default) and cannot change
  user-visible behavior, so it does not need a launch flag. (Contrast ADR-0016, which gated *behavioral*
  link-kinds.) If the operator prefers a kill-switch anyway, an env-gated pass-through in
  `apps/web/src/lib/channel.ts` (return `'web-direct'` when off) is the cheapest option — noted, not built.

---

## 10. Open / accepted risks

| # | Risk | Disposition | Owner |
|---|------|-------------|-------|
| R1 | First-touch vs last-touch attribution, **per slug**: for a given `dos_channel:<slug>` key, a mount with `?ch=` writes/overwrites; a mount without `?ch=` leaves it untouched → effectively first-touch-per-slug. A later same-slug nav carrying a different `?ch=` overwrites. | **Accepted (v1).** Spec = `capture(slug, search)` on shell mount; for a write-only analytics tag the difference is noise. Document the chosen semantics in `channel.ts`; revisit only if attribution reporting demands strict first-touch. Cross-slug bleed is closed by the per-slug key (R7). | Attribution lane |
| R2 | jsonb weak typing at rest — a future hand-written INSERT could bypass the Zod enum. | **Accepted.** All order writes go through `CreateOrderInput.parse` + `insertOrderWithItems`; no other write path exists. The deferred first-class-column CHECK (§5) closes it permanently if ever needed. | API lane |
| R3 | Enum growth (14th channel) means a shared-types release, not a migration. | **Accepted — this is the point.** Cheaper than a CHECK migration while write-only; single SoT in `@deliveryos/shared-types`. | Shared-types owner |
| R4 | **GDPR erasure gap (pre-existing).** `metadata` co-locates `client_ip_hash` (hashed IP = pseudonymized PII) with `channel`. On erasure the anonymizer nulls the `client_ip_hash` **column** but issues **zero writes against `metadata`** (`anonymizer/index.ts:210-222`), so `metadata->>'client_ip_hash'` **survives erasure**; the owner dashboard (R-M1) already forwards it. `channel` adds no PII but is the new reason engineers will read `metadata`. | **ACCEPTED-as-risk for this lane; flagged as a separate follow-up ticket — NOT fixed here, NOT dropped.** Fix belongs to the GDPR/anonymizer owner: have the anonymizer also strip the jsonb copy (`metadata = metadata - 'client_ip_hash'`). This lane does **not** touch `apps/api/src/lib/anonymizer/*`. See `resolution.md` H1. Any future analytics/AI read must project `metadata->>'channel'` **only**, never `SELECT metadata`. | **GDPR/anonymizer maintainer** (escalate via lead/operator) |
| R5 | Someone later wires `channel` into a decision (price/status/dispatch/authz), violating the decision-path-read prohibition. | **Guardable.** Recommend the impl lane promote this from prose to a deterministic guardrail (grep/lint assertion or a red→green test) that `metadata->>'channel'` acquires no reader in pricing/dispatch/authz/status/RLS modules — per counsel Rec #1, this guard *is* the ethical mechanism, not mere tidiness. Design-time flag for the impl lane. | Impl lane |
| R6 | **Second, legacy checkout client uninstrumented (breaker L2).** `apps/api/src/client/checkout/app.ts::confirmOrder` (vanilla JS, `POST /api/orders`) is a second order-entry path this pass does **not** instrument with `?ch=` capture; its orders always fall to server default `'web-direct'`. | **ACCEPTED as an explicit, documented known-gap** (honest under-attribution for that path, not a break — server default is safe; no PII, no decision impact). Not fixed in this pass, not silently dropped. See `resolution.md` L2. | Attribution lane (future, if that client stays live) |
| R7 | Cross-storefront attribution bleed within one tab (breaker M2). | **RESOLVED in design.** `sessionStorage` key is per-slug (`dos_channel:<slug>`); `capture(slug, search)`/`read(slug)` (§7). B without `?ch=` reads its own absent key → `'web-direct'`, never A's channel. | Attribution lane |

---

## 11. Counsel advisory dispositions (non-blocking; addressed-or-acknowledged)

Counsel verdict was **CLEAR TO PROCEED — no grounded ETHICAL-STOP** (§2 of the counsel opinion). No human
ethical decision is required for this design. The two proportional recommendations and five aesthetic/
strategic notes are dispositioned here (full record in `resolution.md`):

- **Rec #1 (elevate write-only invariant to a guard):** accepted as a design-time flag for the impl lane —
  folded into R5. The no-decision-path-reader rule is the *ethical mechanism*, so the impl lane should ship
  a deterministic red→green guard (grep/lint/test), not just a comment.
- **Rec #2 (anonymizer does not scrub the `metadata` copy of `client_ip_hash`):** same finding as breaker
  H1 — **ACCEPTED-as-risk, flagged as a separate follow-up ticket for the GDPR/anonymizer owner** (R4). Not
  fixed in this lane; not dropped.
- **A — `'other'` silent fallback masks a broken QR:** acknowledged; deferred to the QR-kit/analytics lane
  (separate `'unknown'` for malformed vs `'other'` for valid-but-unlisted, or treat an `'other'` spike as a
  QR-health alarm). Out of scope for this write-only pass.
- **B — i18n the channel *labels*, keep *tokens* stable / C — document the taxonomy:** acknowledged and now
  directly relevant given the owner-dashboard passthrough (§8, M1) surfaces raw tokens today. Building
  label rendering, the `al`/`en` catalog entries, and the published token→meaning map is **out of scope for
  this pass** and owned by the channel-breakdown UI lane; no raw token should be *rendered as a label* until
  then.
- **D — customer-facing transparency:** acknowledged; a one-line `/compliance` privacy-notice addition
  ("arrival source is recorded for the owner's own analytics") is the honest, commons-consistent move —
  deferred to the compliance owner, low stakes.
- **E — enum-growth as an ethical control surface:** acknowledged; the add-a-channel process should carry a
  one-line "is this token sensitive?" check (keep the allowlist to benign marketing channels) — folded into
  R3's intent.
- **Steel-man of Option B (dedicated column = legibility/auditability, DB-CHECK, PII separation):**
  accepted on the merits. `A` remains defensible for *this* lane (no-migration constraint + write-only,
  no-decision-reader field); the deferred first-class-column path (§5) is the correct hedge and lands with
  an identical wire contract. Recorded so the rejection is on merits, not silently on the lane constraint.
- **§6 "the question nobody asked" (customer standpoint):** acknowledged as a horizon marker. Holds while
  the field stays coarse, write-only, and merchant-first; any move to join `channel` to a stable
  cross-order customer identity, or to aggregate `channel` across tenants for the platform's own benefit, is
  a *different feature* requiring its own Charter/ethics review (counsel Commons/§6).

---

## Appendix — invariant checklist (must all hold in the implementing change)

- [ ] `channel` is **never read by any decision path** — no reader in pricing (subtotal/fee/tax/total),
      status/state-machine, dispatch/courier-assignment, notifications, or authz/RLS. (The benign,
      same-tenant owner-dashboard `metadata` passthrough — `dashboard.ts:112-131` — is *expected* and is not
      a decision-path reader; it is out of scope to build a channel-breakdown UI here. §8, M1.)
- [ ] **No new DB column, no migration** (`packages/db/migrations/` untouched).
- [ ] **No new npm dependency** (no `package.json` touched).
- [ ] `.strict()` behavior for every **other** `CreateOrderInput` field is unchanged; only one new optional
      field with a safe default (`'web-direct'`) is added — no existing `?ch=`-less client breaks.
- [ ] Single source of truth for the allowlist: `OrderChannel` in `@deliveryos/shared-types`, imported by
      the web client (no duplicated list).
- [ ] `channel` is **not** folded into the idempotency `request_hash`.
- [ ] Tenant-agnostic: same row, same location-scoped INSERT, same ENABLE+FORCE RLS; no bypass.
- [ ] Client persistence is `sessionStorage` under a **per-slug** key `dos_channel:<slug>` (not cookie, not
      `localStorage`), try/catch-guarded; the `channel.ts` API is `capture(slug, search)` / `read(slug)`
      (breaker M2 — no session-global key, no cross-storefront bleed).
- [ ] **Known-gap (documented, not fixed here):** the legacy vanilla-JS checkout client
      `apps/api/src/client/checkout/app.ts::confirmOrder` is **not** instrumented this pass → its orders fall
      to server default `'web-direct'` (breaker L2, R6). Honest under-attribution, not a break.
- [ ] **Flagged follow-up (not this lane):** the GDPR anonymizer does not strip the `metadata` copy of
      `client_ip_hash` — escalate to the GDPR/anonymizer owner (breaker H1, R4). This lane does not touch
      `apps/api/src/lib/anonymizer/*`.
