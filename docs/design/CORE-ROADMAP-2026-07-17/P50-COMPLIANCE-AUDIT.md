# P50 — Legal/Compliance Audit (WAVE G)

> **This audit is agent-produced git archaeology only; it self-certifies nothing; legal-judgment rows are flagged, not resolved.**
>
> It was produced mechanically from `git` history per `BLUEPRINT-P47-P50-gap-closing-phases.md §5.2`.
> No source code was modified. It reaches no legal conclusions. Any row whose disposition depends on
> jurisdiction-specific law (retention periods, tax reporting duty, food-safety applicability, erasure
> scope, tracking-consent lawful basis) is marked **⚠ OPERATOR/COUNSEL** and left undecided.
> The standing anti-self-certification rule binds hardest here: a grep for a "compliant" self-claim
> without a counsel reference is the RED check (§5.4-B2).

- **Worktree:** `/tmp/p50-waveG` (branch `feat/p50-audit-waveG`, forked from `main` @ `76167336a`)
- **Method:** `git log --all --diff-filter=D` inventory + `git show <commit>^:<path>` content recovery
- **Scope:** deleted legal-surface files only (the old JS/TS stack under `attic/`, `apps/`, `packages/`;
  `attic/` and `apps/` are **gone from disk** — git history is the only source, per §0).

---

## 0. Summary counts

| Classification | Count |
|---|---|
| **ported** (live cite required) | **1** |
| **deliberately-dropped-with-reason** (dated reason) | **0** |
| **genuinely-missing** (tracked item) | **13** |
| **⚠ OPERATOR/COUNSEL flagged** (legal judgment required) | **13** |

- Total recovered deleted files audited: **14**. Every row has exactly one classification.
- The single **ported** row is **NOT** operator/counsel-flagged (its disposition is a factual port check, not a legal judgment).
- All **13 genuinely-missing** rows **ARE** operator/counsel-flagged, because each missing obligation is a
  jurisdiction-dependent legal judgment (erasure duty, retention period, privacy-notice/transparency duty,
  tracking-consent lawful basis, tax reporting) that this audit is forbidden from resolving.
- **deliberately-dropped-with-reason = 0** is a deliberate, honest result: the files were swept up in a
  bulk operator-directed JS/TS stack purge (`f9ab28ff1`, `79ef316f6`, `a29aa219e`), **not** retired by a
  per-obligation decision that the underlying legal duty is moot. Asserting "dropped because obsolete" would
  itself be an illegal legal conclusion — so they are classified genuinely-missing, not dropped-with-reason.

---

## 1. Inventory evidence (verbatim `git log --all --diff-filter=D` output)

Command (§5.2 step 1, exact):

```
git log --all --diff-filter=D --name-only -- '*gdpr*' '*consent*' '*anonymiz*' '*tax*' '*invoice*' '*food*'
```

Raw output (commits + deleted paths). Note `dogfood-output/screenshots/*` and `dogfood-output/report.md`
matched only because the `*food*` glob catches "**dog**food" — these are QA artifacts, **not** legal surface,
and are excluded from the audit (see §2 exclusions).

```
commit aa70d7fa6fbebbdd75661a82f6929be87211437d
Author: SyniakSviatoslav <sviatoslavsyniak@gmail.com>
Date:   Sat Jul 18 01:06:04 2026 +0000

    fix(kernel,budget): refuse NaN/negative estimate + harden lock-poison (V1 #5/#6) (DCO)

dogfood-output/report.md
dogfood-output/screenshots/01-login.png
dogfood-output/screenshots/02-admin-direct-noauth.png
dogfood-output/screenshots/03-public-menu-demo.png
dogfood-output/screenshots/04-customer-menu-polluted.png
dogfood-output/screenshots/05-cart-empty.png
dogfood-output/screenshots/06-menu-hydrated-cart.png
dogfood-output/screenshots/07-checkout-cdn.png
dogfood-output/screenshots/08-checkout-built.png
dogfood-output/screenshots/09-admin-cdn.png
dogfood-output/screenshots/10-admin-built.png
dogfood-output/screenshots/card-01-current.png
dogfood-output/screenshots/dev-01-admin-orders.png
dogfood-output/screenshots/flow-01-checkout.png
dogfood-output/screenshots/img-01-client-display.png
dogfood-output/screenshots/issue-001-invalid-login-to-admin.png
dogfood-output/screenshots/issue-001-result.png
dogfood-output/screenshots/issue-001-step-1-login.png
dogfood-output/screenshots/issue-001-step-2-filled.png
dogfood-output/screenshots/lc-admin-analytics.png
dogfood-output/screenshots/lc-admin-crm-fixed.png
dogfood-output/screenshots/lc-client-delivered.png
dogfood-output/screenshots/lc-courier-earnings.png
dogfood-output/screenshots/live-01-menu.png
dogfood-output/screenshots/live-02-cart.png
dogfood-output/screenshots/live-03-menu-mobile.png
dogfood-output/screenshots/live-04-checkout.png
dogfood-output/screenshots/live-05-checkout-fixed.png
dogfood-output/screenshots/live-06-menu-blue-theme.png
dogfood-output/screenshots/live-07-checkout-deployed.png
dogfood-output/screenshots/spa-01-branding-preview.png
dogfood-output/screenshots/spa-checkout.png

commit a29aa219ee60edab315f498b18cf1f94d0801d28
Author: SyniakSviatoslav <sviatoslavsyniak@gmail.com>
Date:   Wed Jul 15 23:02:30 2026 +0000

    engine(bridge): wire VertexBridge to GpuUploadSink + purge dead TS trees

attic/apps-api/src/public/admin/gdpr.html

commit f9ab28ff183097b3bce56df41e654405624fedd1
Author: SyniakSviatoslav <sviatoslavsyniak@gmail.com>
Date:   Wed Jul 15 16:16:12 2026 +0000

    chore: drop ALL JS/TS (per operator); rewire CI to telemetry+eqc; add bench/monitor

attic/apps-api/src/lib/anonymizer/index.ts
attic/apps-api/src/routes/owner/gdpr.ts
attic/apps-api/src/workers/anonymizer-gdpr.ts
attic/apps-api/src/workers/anonymizer-retention.ts
attic/apps-api/tests/money-tax.test.ts
attic/packages-db/migrations/1780421100060_anonymization-seam.ts

commit 79ef316f68c4fb91df9855180d8dafb3ed2dee77
Author: SyniakSviatoslav <sviatoslavsyniak@gmail.com>
Date:   Mon Jul 13 22:06:46 2026 +0000

    refactor: remove legacy JS/TS thin-layer, kernel is now sole source of truth

packages/shared-types/src/contracts/owner/gdpr.ts

commit db766de474512414d8f0c5373c9e169b98a029a2
Author: SyniakSviatoslav <sviatoslavsyniak@gmail.com>
Date:   Mon Jul 13 22:06:46 2026 +0000

    refactor: remove legacy JS/TS thin-layer, kernel is now sole source of truth

packages/shared-types/src/contracts/owner/gdpr.ts
```

### §5.2 step 1 (extend by grep) — recovered additional legal-surface hits

A second pass over **all** deleted paths, filtered by a broader compliance regex
(`privacy|retention|cookie|consent|anonym|gdpr|tax|invoice|food|legal|data.protect|optout|kyc|aml|terms|tos|right.to|erasure|data.subject|…`),
recovered these additional legal-surface deletions (the base glob missed `retention`/`privacy`/anonymous-R予LS
files). Each is a genuine legal-surface hit and is audited below:

- `attic/apps-api/src/public/admin/settings-retention.html` (del `a29aa219e`)
- `attic/apps-api/src/workers/access-request-retention.ts` (del `f9ab28ff1`)
- `attic/apps-api/tests/p0-privacy.test.ts` (del `f9ab28ff1`)
- `attic/packages-db/migrations/1780338981782_customer-anonymous-update.ts` (del `f9ab28ff1`)
- `attic/packages-db/migrations/1780338981783_anonymous_orders.ts` (del `f9ab28ff1`)
- `apps/web/src/pages/PrivacyPage.tsx` (del `79ef316f6`)

### Excluded as non-legal-surface noise (recorded for transparency)

From the extended regex pass, these matched but are **not** legal-surface obligations and are excluded:
`dogfood-output/*` (QA screenshots/report — only matched via "dog**food**"), `e2e/artifacts/.../*-no-cookies-*`
(test-run artifacts), `.claude/skills/.../cookies.js` (vendored third-party skill lib), `loops/design-convergence.yaml`,
`packages/shared-types/src/legacy.ts`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`. No obligation is served by these.

---

## 2. Audit rows (every recovered file → one classification)

Legend: 🟢 ported · 🟡 genuinely-missing · ⚠ = OPERATOR/COUNSEL legal-judgment flag.

---

### R1 — `attic/apps-api/src/routes/owner/gdpr.ts`
- **Deleted:** `f9ab28ff1` (2026-07-15, "drop ALL JS/TS per operator")
- **Obligation served:** GDPR **right to erasure** intake API — owner-initiated erasure requests
  (`POST/GET /:locationId/gdpr-requests`), dedup/cooldown logic, and retention-settings read/write
  (`GET/PUT /:locationId/settings/retention`, `retention_days` 30–2555). Maps to GDPR Art. 17 (erasure) + Art. 5(1)(e) (storage limitation).
- **New-stack status:** 🟡 **genuinely-missing**. No erasure-request API, no `retention_days` config surface exists in the
  Rust/WASM stack (verified: zero `gdpr|erasure|retention_days` hits in `kernel/`, `web/`, `engine/`).
- **⚠ OPERATOR/COUNSEL:** Whether the product must implement Art. 17 erasure + a configurable retention period,
  and what the lawful default retention window is, is jurisdiction-dependent. **Left undecided.**
- **Tracked item:** `P50-missing:gdpr-erasure-api`

---

### R2 — `attic/apps-api/src/lib/anonymizer/index.ts`
- **Deleted:** `f9ab28ff1` (2026-07-15)
- **Obligation served:** The **anonymization engine** — `AnonymizerService.anonymize()` rewrites `customers`
  (phone→`anon_<uuid>`, name→NULL, `marketing_opt_in=false`, avatar storage purged) and `orders`
  (`client_ip_hash`→NULL, `delivery_address`→NULL), writes `anonymization_audit_log`, publishes bus events;
  plus `findExpiredCustomers/Orders` retention sweep driven by `locations.retention_days`. Core execution of erasure + retention.
- **New-stack status:** 🟡 **genuinely-missing**. No anonymization service, no `anonymized_at` field, no audit-log
  writer in the new stack.
- **⚠ OPERATOR/COUNSEL:** Retention period and erasure-execution obligations are jurisdiction-dependent; the
  *correct* anonymization scope (which fields, which subjects) is a legal judgment. **Left undecided.**
- **Tracked item:** `P50-missing:anonymization-engine`

---

### R3 — `attic/apps-api/src/workers/anonymizer-gdpr.ts`
- **Deleted:** `f9ab28ff1` (2026-07-15)
- **Obligation served:** `GdprErasureWorker` — async (pg-boss) processor that turns a pending erasure request
  into an anonymization call, with retry/backoff and completion events. The operational half of Art. 17.
- **New-stack status:** 🟡 **genuinely-missing**. No erasure worker in the new stack.
- **⚠ OPERATOR/COUNSEL:** Erasure timeliness/completeness duty (Art. 17 "without undue delay") is jurisdiction-dependent. **Left undecided.**
- **Tracked item:** `P50-missing:gdpr-erasure-worker`

---

### R4 — `attic/apps-api/src/workers/anonymizer-retention.ts`
- **Deleted:** `f9ab28ff1` (2026-07-15)
- **Obligation served:** `AnonymizerRetentionWorker` — nightly cron (`0 3 * * *`) that anonymizes all
  expired customers/orders per `retention_days` and purges expired `customer_track_grants`. The automated
  storage-limitation control (GDPR Art. 5(1)(e)).
- **New-stack status:** 🟡 **genuinely-missing**. No retention cron in the new stack.
- **⚠ OPERATOR/COUNSEL:** The retention window itself is a legal judgment; the duty to auto-purge is jurisdiction-dependent. **Left undecided.**
- **Tracked item:** `P50-missing:retention-cron`

---

### R5 — `attic/apps-api/src/workers/access-request-retention.ts`
- **Deleted:** `f9ab28ff1` (2026-07-15)
- **Obligation served:** `AccessRequestRetentionWorker` — 12-month auto-erase of `access_requests`
  (the standing consent-expiry mechanism) + a reconcile cron for lost notify enqueues. Encodes consent-record
  lifecycle / retention (`ACCESS_REQUEST_RETENTION` default `12 months`).
- **New-stack status:** 🟡 **genuinely-missing**. No access-request retention worker; the consent-record model
  itself is absent (P49 identity is deferred to 5–50 real clients per §4.2).
- **⚠ OPERATOR/COUNSEL:** Consent-record retention period and whether a consent/access-request flow even applies
  to the new (deferred) identity model is a legal judgment. **Left undecided.**
- **Tracked item:** `P50-missing:access-request-retention`

---

### R6 — `attic/apps-api/tests/money-tax.test.ts`   🟢 **PORTED**
- **Deleted:** `f9ab28ff1` (2026-07-15)
- **Obligation served:** Proof that **tax is computed with integer/BigInt math, zero float drift** on the
  monetary value (`applyTax` / `computeLineTotal` integer-safe, rejects non-integer input, large-value exact).
  This is the money-correctness half of the **tax computation** obligation (accurate tax math ⇒ correct tax amounts).
- **New-stack status:** 🟢 **ported** — verified live in the Rust kernel:
  - `kernel/src/money.rs:270` — `pub fn apply_tax(subtotal: i64, tax_rate: f64, price_includes_tax: bool) -> Result<i64, String>`
    (integer `i128` math, half-up rounding, checked/range-checked per `BP-17`; header comment cites `money.ts:23` as the oracle).
  - `kernel/src/money.rs:307` — `pub fn compute_line_total(...) -> Result<i64, String>` (overflow-safe).
  - Reused by the order total: `kernel/src/domain.rs:135` — `let tax = apply_tax(subtotal, tax_rate, price_includes_tax)?;`
    inside `compute_order_total` (`domain.rs:129`), and `lib.rs:230` re-exports both.
  - Parity test exists in-kernel: `kernel/src/money.rs:567` `apply_tax_generated_parity_exact_integers`.
- **Flag:** **Not** operator/counsel-flagged — the disposition is a factual port check (the arithmetic obligation
  this file served is satisfied in the new stack). NOTE: the *tax reporting / filing / invoice-issuance* duty is a
  separate legal-judgment matter (see R14 note) and is **not** an obligation this test file served.

---

### R7 — `attic/apps-api/tests/p0-privacy.test.ts`
- **Deleted:** `f9ab28ff1` (2026-07-15)
- **Obligation served:** Privacy-hardening proof — **GPS tracking consent boundary** (no position stored unless
  the courier is on an *active* delivery; `assigned`/`delivered` withheld), the 24h GPS-purge cron, and rejection
  of new non-telegram notification targets (`owner_notification_targets_not_whatsapp`). Encodes a lawful-basis /
  data-minimisation control on location tracking.
- **New-stack status:** 🟡 **genuinely-missing**. The P0 consent boundary + GPS-purge cron are not ported; P49
  (customer tracking) is deferred, and no tracking-consent enforcement exists in the kernel.
- **⚠ OPERATOR/COUNSEL:** The lawful basis for courier/customer location tracking, the consent boundary, and the
  tracking-data retention window are jurisdiction-dependent (GDPR Art. 6 lawful basis, e-Privacy). **Left undecided.**
- **Tracked item:** `P50-missing:tracking-consent-boundary`

---

### R8 — `attic/packages-db/migrations/1780421100060_anonymization-seam.ts`
- **Deleted:** `f9ab28ff1` (2026-07-15)
- **Obligation served:** DB schema that **persists** the erasure/retention machinery — `anonymized_at` columns on
  `customers`/`orders`, `locations.retention_days` (30–2555), `gdpr_erasure_requests` table, `anonymization_audit_log`
  table, RLS tenant-isolation policies, and red-line comments ("P5-0"). The durable state backing R1–R4.
- **New-stack status:** 🟡 **genuinely-missing**. The new stack is event-sourced (fold over `DeliveryEvent`s); no
  SQL erasure/retention schema, no `anonymized_at`/`retention_days` columns, no `gdpr_erasure_requests` table.
- **⚠ OPERATOR/COUNSEL:** Whether erasure/retention must be expressible in the new event-sourced model, and the
  retention window, are legal judgments. **Left undecided.**
- **Tracked item:** `P50-missing:erasure-retention-schema`

---

### R9 — `attic/packages-db/migrations/1780338981782_customer-anonymous-update.ts`
- **Deleted:** `f9ab28ff1` (2026-07-15)
- **Obligation served:** RLS policy `anonymous_update` / `anonymous_select` on `customers` — lets an **unauthenticated**
  (anonymous) actor update/select customer rows (the anonymous-ordering seam). Part of the anonymous-identity model (P49 lineage).
- **New-stack status:** 🟡 **genuinely-missing**. No SQL RLS layer in the new stack; anonymous identity is deferred (§4.2).
- **⚠ OPERATOR/COUNSEL:** Anonymous-processing obligations (GDPR Art. 13(4) transparency for data not obtained from
  the subject; lawful basis for processing anonymous-order data) are jurisdiction-dependent. **Left undecided.**
- **Tracked item:** `P50-missing:anonymous-customer-rl`

---

### R10 — `attic/packages-db/migrations/1780338981783_anonymous_orders.ts`
- **Deleted:** `f9ab28ff1` (2026-07-15)
- **Obligation served:** RLS policy `anonymous_select` on `orders` / `order_items` — anonymous browsing of order state.
  Part of the anonymous-identity/anonymization seam.
- **New-stack status:** 🟡 **genuinely-missing**. No equivalent in the new stack.
- **⚠ OPERATOR/COUNSEL:** Same anonymous-processing transparency obligation as R9. **Left undecided.**
- **Tracked item:** `P50-missing:anonymous-order-rl`

---

### R11 — `apps/web/src/pages/PrivacyPage.tsx`
- **Deleted:** `79ef316f6` (2026-07-13)
- **Obligation served:** The **privacy notice** page — legal basis (consent), data stored, purpose, **retention
  (12 months from first contact)**, data-subject rights, and a reachable erasure contact (`privacy@dowiz.org`).
  Satisfies GDPR Art. 13/14 transparency + the "consent link must resolve" requirement.
- **New-stack status:** 🟡 **genuinely-missing**. No privacy-notice page in the new WebGPU/Rust UI (the old `web/`
  SPA is gone; P48 owner surface + P49 customer surface are not yet built).
- **⚠ OPERATOR/COUNSEL:** The notice content, the stated retention period, and the erasure-contact obligation are
  jurisdiction-dependent transparency duties. **Left undecided.**
- **Tracked item:** `P50-missing:privacy-notice`

---

### R12 — `packages/shared-types/src/contracts/owner/gdpr.ts`
- **Deleted:** `79ef316f6` (2026-07-13)
- **Obligation served:** Zod contract types for the erasure-request API (`CreateGDPRRequest`, `GDPRRequestItem`,
  `GDPRRequestListResponse`, `CreateGDPRResponse`) — the typed surface of R1.
- **New-stack status:** 🟡 **genuinely-missing**. No typed erasure-request contract in the new stack (mirrors R1).
- **⚠ OPERATOR/COUNSEL:** Same Art. 17 erasure obligation as R1. **Left undecided.**
- **Tracked item:** `P50-missing:gdpr-erasure-contract`

---

### R13a — `attic/apps-api/src/public/admin/gdpr.html`
- **Deleted:** `a29aa219e` (2026-07-15, "purge dead TS trees")
- **Obligation served:** Admin **UI** for managing GDPR erasure requests (owner surface to view/initiate erasure).
  The operator-facing half of R1/R3.
- **New-stack status:** 🟡 **genuinely-missing**. P48 owner/admin surface is not yet built.
- **⚠ OPERATOR/COUNSEL:** Same Art. 17 erasure obligation as R1. **Left undecided.**
- **Tracked item:** `P50-missing:gdpr-admin-ui`

---

### R13b — `attic/apps-api/src/public/admin/settings-retention.html`
- **Deleted:** `a29aa219e` (2026-07-15)
- **Obligation served:** Admin **UI** to configure `retention_days` per location — the operator-facing half of R4.
- **New-stack status:** 🟡 **genuinely-missing**. No retention-settings UI in the new stack.
- **⚠ OPERATOR/COUNSEL:** Retention-period configuration duty is jurisdiction-dependent (see R4). **Left undecided.**
- **Tracked item:** `P50-missing:retention-settings-ui`

---

## 3. Cross-cutting notes (no legal conclusions)

- **The one port (R6) is narrow:** only the *integer-safe tax arithmetic* obligation is ported (and re-verified
  with a parity test in `kernel/src/money.rs`). The *broader* tax compliance surface — invoice generation,
  tax-rate jurisdiction config, filing/reporting — was **never** present as a ported obligation and is **not**
  claimed here.
- **No "deliberately-dropped-with-reason" rows:** every deletion was part of the operator-directed JS/TS stack
  purge. No row carries a dated reason showing the underlying legal duty was affirmatively retired as moot.
  Mislabeling them "dropped" would be reaching a legal conclusion this audit is forbidden from making.
- **13 genuinely-missing rows = 13 tracked items**, each a legal-surface gap in the new stack. They are **not**
  silently absorbed; they are enumerated as `P50-missing:*` items above and should feed the §5.3 first-order gate
  (zero unresolved genuinely-missing *legal blockers*). Several are gated behind P48/P49 build-out (deferred).
- **Self-certification guard:** this document contains no "compliant" / "GDPR-compliant" claim. Per §5.4-B2 the
  presence of such a claim without a counsel reference is the RED check; none is made.

---

## 4. Method reproducibility

```
# 1. inventory (base glob, verbatim output in §1)
git log --all --diff-filter=D --name-only -- '*gdpr*' '*consent*' '*anonymiz*' '*tax*' '*invoice*' '*food*'

# 1b. extend by grep (broader compliance regex over all deleted paths)
git log --all --diff-filter=D --name-only --pretty=format: | sort -u \
  | grep -Ei 'privacy|retention|cookie|consent|anonym|gdpr|tax|invoice|food|legal|data.protect|...'

# 2. for each hit, recover last-live content
git show <deletion_commit>^:<path>

# 3. classify per §5.2-3 contract; flag legal-judgment rows ⚠ OPERATOR/COUNSEL
```

All 14 recovered files were recovered via `git show <commit>^:<path>` and individually inspected before classification.
No `git add -A` was used; only this document is staged/committed. No source files were modified.
