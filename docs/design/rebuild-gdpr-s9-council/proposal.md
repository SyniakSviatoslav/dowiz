# S9-GDPR/COMPLIANCE Port — Council Packet · PROPOSAL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Description input to the live Triadic Council
> (system-architect + system-breaker + counsel + human). **No S9 code is ported to Rust until this
> packet is council-APPROVED, every quirk-register row (§11) is dispositioned one by one, and the
> operator signs the 🔴 open questions (`open-questions.md`).** This is the **REDDEST surface in the
> whole rebuild** — the one place where a port defect is an **irreversible erasure** (an Art.17
> "complete" that erased nothing, or an erasure of the wrong subject) and where the failure is a
> **legal-compliance** failure, not just a money or availability one. Docs only; no product code.

- **Lane:** R3 (complete-rebuild) · **Surface:** S9 GDPR/compliance (REBUILD-MAP §3 Phase B, 9th
  strangler — `S5 orders → S6 WS → S7 dispatch → S8 jobs → S9 GDPR 🔴`), **5 route-surface rows**
  (route-surface-map rows 70-74) **+ the erasure semantics** (`lib/anonymizer`) the S8 fleet pumps.
- **Date:** 2026-07-05 · **Source commit:** `fix/audit-remediation` (working tree).
- **Census SSOT:** `route-surface-map.generated.md` S9 rows 70-74
  (`POST|GET /api/owner/locations/:locationId/gdpr-requests`, `GET .../gdpr-requests/:requestId`,
  `GET|PUT .../settings/retention`, all `gdpr.ts`) + the erasure engine
  (`lib/anonymizer/index.ts`, `workers/anonymizer-gdpr.ts`, `workers/anonymizer-retention.ts`) +
  `compliance/data-map.md` (the code-grounded PII inventory that defines what "complete erasure" means).
- **Governing ADRs / prior councils (this surface inherits hard-won invariants — do not re-litigate):**
  - **audit-fix-rls-reliability RESOLVE-R2** (`docs/design/audit-fix-rls-reliability/resolution-r2.md`)
    — the **N1 CRITICAL**: `customers` has **no `app.current_tenant` policy arm** (RC4 arm is
    orders-only, `1790000000077:44-67`), the anonymizer runs on a **context-free** pool connection
    (`index.ts:131,220`), and the worker wrote `completed` **regardless of `result.skipped`** →
    post-NOBYPASSRLS+MIG-2 a context-free erasure sees ∅ and **silently false-completes** an Art.17
    request. FIX = the fail-loud data-level backstop (**ledger #61**, shipped) + the DEFINER
    `gdpr_erase_customer` (**draft `1790000000088`**, operator-gated) + a NOBYPASSRLS data-level P-proof.
  - **S4-media RESOLVE REV-S4-7 / ledger #74** — the `delivery_photo_key` erasure gap: `anonymizeOrder`
    nulled the text address but left the doorway **R2 object** alive, public-by-key, indefinitely.
    Fixed (extend the avatar-purge pattern to `delivery_photo_key`, `index.ts:260-267`). **The pattern
    this council must complete: erasure enumerates EVERY PII carrier, including object-storage keys.**
  - **audit-fix-authz LC5 / ledger #57** — the gdpr-requests **cross-tenant IDOR** (owner-A erasing
    tenant-B); FIX = the same-tenant proof + masked-404 + the `|| row.location_id` self-derived-scope
    fallback **DELETED** in the anonymizer (fail-closed, scope required, `index.ts:129-132,218-221`).
  - **safe-reversal-spine / LC7 / ledger #64** — the backup **restore-drill was false-green**
    (`checkRowCounts` a 9-table `count>base*10` heuristic; a 99%-truncated table PASSED; the smoke pool
    queried **live prod**, not the restore). FIX = strict full-set parity + `checkPIIFree` **removed**
    (Option A keeps full-PII **encrypted** dumps — a faithful restore MUST contain PII).
  - **pg-privilege-hardening / ledger #33** — the DEFINER **search_path-hijack** class
    (`read_public_menu` + 5 unpinned DEFINER fns); every DEFINER fn pins
    `SET search_path = pg_catalog, public, pg_temp`. Draft 088 already carries the pin (`:40`).
  - **owner-data-export-ai** (ETHICAL-STOP: zero PII to AI) · **p0-privacy-hardening**
    (bus claim-check — status/ids only, no name/phone/address; GPS 24h TTL).
- **Parity oracle:** the 174-spec Playwright net (load-bearing: the gdpr-requests owner slice) **plus**
  the compliance invariant cluster: the **N1 data-level erasure P-proof** (target
  `customers.anonymized_at IS NOT NULL` + `phone` tokenised under **NOBYPASSRLS+MIG-2**; negative:
  fn-absent/context-withheld ⇒ `failed`+DLQ, never `completed` — ledger #61 backstop
  `anonymizer-gdpr-backstop.test.ts`), the **erasure-carrier completeness** guardrails
  (`anonymizer-order-photo-purge.test.ts` #74 + the new carriers this packet enumerates, §3), the
  **cross-tenant erasure IDOR** guard (`gdpr-authz`, `anonymizer-fail-closed`,
  `anonymizer-gdpr-worker-provenance` — ledger #57), and the **restore-fidelity** drill
  (`backup-drill-integrity.test.ts` #64). **No behavior change is real without a red→green test**
  (Mandatory Proof Rule). Cutover DoD in §12.

---

## 1. Port objective and the load-bearing seam

S5 wrote money; S8 owns the background runtime. **S9 is the one surface whose defining operation is
DESTRUCTIVE and IRREVERSIBLE** — Art.17 erasure. There is no "undo," no "rollback plan," no cleanup
after the flip: once a row is anonymised, the pre-image is gone. Every other surface's cutover safety
net is "both stacks write the same tables, roll back the proxy flag." **S9 has no such net for the
erasure itself** — the only safe posture is to **prove erasure correctness under enforcement BEFORE
the flip** and make the flip **human-gated** (§9, Q6 🔴).

There are **three** load-bearing seams, each an independent failure mode the port must hold
simultaneously:

1. **The erasure-completeness seam (Q1 🔴).** "Complete erasure" is defined by
   `compliance/data-map.md`, not by whichever columns the current UPDATE happens to touch. The port
   must (a) **never open a context-free connection** — the N1 class: `customers` has no
   `app.current_tenant` arm, so a context-free erasure is **invisible** to `customers` RLS post-MIG-2
   and **silently no-ops**; erasure runs through the **DEFINER `gdpr_erase_customer`**
   (visibility-independent), never a raw `pool.connect()→UPDATE`; and (b) **enumerate EVERY PII
   carrier** — text fields *and* object-storage keys — and prove each is erased. This packet's
   carrier audit (§3) surfaces **three live gaps** beyond the two already fixed (metadata ip-hash #1,
   `delivery_photo_key` #2): the erasure **does not fan out to the subject's orders at all**
   (GAP-A 🔴), `orders.delivery_lat/lng` is never nulled (GAP-B), and `order_ratings.feedback` is
   never touched (GAP-C).
2. **The tenant-isolation seam (Q2 🔴).** A GDPR request is driven by a **client-supplied
   `customerId` or `phone`**; a cross-tenant `customerId` must resolve to a **masked 404** (never a
   leak, never a cross-tenant erasure), with the cross-tenant attempt **security-logged** before the
   404. The status reads must **mask** the subject id (`maskName`) so the request/audit surface is not
   itself a PII disclosure. The anonymizer's scope is **required, never self-derived** (the deleted
   `|| row.location_id` fallback).
3. **The irreversibility seam (Q3 🔴).** Erasure "proving it erased" cannot be a restore-drill
   assertion (the restore-drill proves **fidelity** — rows come *back*; erasure needs the **opposite**
   — a row *stays gone*). Erasure correctness is the **N1 data-level re-read gate**
   (`anonymized_at IS NOT NULL`) under NOBYPASSRLS, not a false-green. And the port must name the
   **restore-resurrection** hazard: a pre-erasure encrypted backup still contains the PII, so a
   restore of it **resurrects** an erased subject unless outstanding erasures are re-applied.

**The sharpest compliance fact (see §3, Q1 🔴):** today, a GDPR erasure of a **customer** anonymises
**only the `customers` row** — `anonymizer-gdpr.ts:62-65` calls
`anonymize({scope:'gdpr', subject:{customerId, locationId}})`, which enters the `customerId` branch
(`index.ts:83-88` → `anonymizeCustomer`) and **never calls `anonymizeOrder`**. The subject's
**orders** — carrying their home `delivery_address`, `delivery_instructions`, `receiver_name`,
`receiver_handle`, and the **doorway `delivery_photo_key`** (more identifying than the address) — are
anonymised **only** by the time-based **retention** sweep (`retention_days`, default **365**). So an
Art.17 "erase me now" request leaves the subject's address and doorway photo live for **up to a
year**. The `delivery_photo_key` purge that S4/REV-S4-7 (ledger #74) just landed lives in
`anonymizeOrder` — **which the GDPR path never reaches**. This is the reddest single finding in the
packet and the most likely breaker escalation.

## 2. Scope — what is S9, what is explicitly NOT

**In this packet (S9):**
1. **`POST /api/owner/locations/:locationId/gdpr-requests`** (`gdpr.ts:33`) — create an erasure
   request: resolve `customerId` from `phone` if needed → the **same-tenant IDOR proof** for a
   client-supplied `customerId` (masked-404 + security-log, `:63-86`) → the **active-request** guard
   (409, `:88-97`) → the **24h completed cooldown** (429, `:99-107`) → insert `pending` → enqueue
   `anonymizer.gdpr`. fastify rate-limit **30/min** (`:34-36`).
2. **`GET .../gdpr-requests`** (`:139`) + **`GET .../gdpr-requests/:requestId`** (`:199`) — the owner
   status surface; **subject id masked** (`maskName`), the `anonymization_audit_log` trail returned
   with masked `subjectId`/`actorId`. Cursor pagination.
3. **`GET|PUT .../settings/retention`** (`:257`/`:272`) — read/set `locations.retention_days`
   (Zod-bounded **30–2555** days; drives the retention sweep's per-record TTL).
4. **The erasure ENGINE semantics** (`lib/anonymizer/index.ts`) — `anonymizeCustomer` /
   `anonymizeOrder`, the carrier null-set, the avatar/`delivery_photo_key` R2 purges, the
   audit-log provenance stamps, and the **fail-closed scope** contract. **This is the code S9 owns**;
   the DEFINER `gdpr_erase_customer` (draft 088) is the visibility-independent port target.
5. **The completion contract** (`workers/anonymizer-gdpr.ts` — the N1 data-level re-read gate + the
   LC4 retry-reset) as the **semantics** the runtime must honour, even though the **runtime plumbing**
   (the `FOR UPDATE SKIP LOCKED` claim loop, the global singleton, the cron) is S8-owned (§9).

**NOT S9 (explicit boundary — each owned elsewhere):**
- **The background-work RUNTIME** — the `anonymizer.gdpr` / `anonymizer.retention` **queue + cron +
  single-flight + at-least-once** plumbing is **S8** (S8 §2/§3.7 explicitly: "the GDPR erasure LOGIC …
  is S9; S8 owns the `anonymizer.gdpr` / retention cron plumbing, never the erasure semantics"). S9
  defines *what a correct erasure is*; S8 defines *when/how the worker is pumped*. The **cross-surface
  coupling** (a Rust-S9 create-request producing into a queue drained by a possibly-Node S8 fleet) is
  §9, Q6 🔴.
- **The DEFINER money/refund folds, settlement, dispatch** — S5/S7. S9 touches no money.
- **The backup/restore PIPELINE** (`pg_dump`/encrypt/upload/`restore-sandbox.ts`/`smoke-checks.ts`) —
  backup/DR council + the ops-binary sidecar (REBUILD-MAP §8). S9 **consumes** the restore-fidelity
  property (LC7) and names the **restore-resurrection** hazard (§5, Q3), but does not port the pipeline.
- **`POST .../orders/:orderId/reveal-customer-contact`** (route 109, S5) and **`GET /api/owner/customers*`**
  (rows 181-182, UNMAPPED, CRM PII) — PII-disclosure-adjacent but **path-owned elsewhere**; named here
  only so the port does not silently pull them into S9 (§5 residual).
- **No schema change** — the DB is frozen. Draft 088 (`gdpr_erase_customer`) is a **DEFINER function**
  (additive, `CREATE OR REPLACE`), **not** a business-table change; it is a `packages/db/migrations/`
  red-line, operator-placed-verbatim, staging-first, forward-only — **S9 does not author or apply it**,
  it **calls** it (§10). Any carrier-completeness fix (GAP-A/B/C) is an **app-side** null-set/fan-out
  change plus (if fanning erasure out to orders under FORCE-RLS) reuse of the existing `orders` RC4
  `app.current_tenant` arm — **no new policy arm on `customers`** (rejected: N2 blast radius on the
  primary PII table).

**Back-of-envelope (why boring wins, and where the real ceiling is).**
- **Scale:** target **N ≈ 10–50 locations**, low-hundreds. **GDPR erasure requests are RARE** — an
  SMB restaurant fields **≈0–5 erasure requests/month system-wide**; even a 100× pessimistic surge is
  a few requests/hour. The **retention sweep** is a nightly (`0 3 * * *` UTC, env-overridable —
  **not** monthly; `anonymizer-retention.ts:25`) batch-of-100 pass per location. There is **no
  throughput surface here at all.**
- **Per-erasure cost:** ONE operational connection for a bounded per-request tx (the DEFINER fn takes a
  `FOR UPDATE` row lock on one `customers` row; the worker batches ≤10 pending requests). The retention
  sweep holds one connection + `pg_try_advisory_lock(4)` for its nightly pass. **Negligible against
  the 20-conn operational pool.**
- **The real ceiling is NOT connections — it is CORRECTNESS and LEGAL COMPLETENESS.** A silent
  non-erasure (N1), an incomplete erasure (GAP-A/B/C), or an erasure of the wrong subject is a
  **legal-red-line** failure at any scale. The `anonymizer.gdpr` **global singletonKey** (at most ONE
  erasure in-flight system-wide, S8 Q-GDPR-GLOBAL-SINGLETON) is a **queueing-theory ceiling only if
  erasure volume grows** — at N≈10-50 with ≈5 req/month it is irrelevant; flagged for the far future,
  not built for.
- **Conclusion:** boring wins — no new runtime; erasure runs on the S8 fleet's worker; the **DEFINER
  fn does the visibility-independent erase**; the carrier null-set is a plain SQL UPDATE. The
  engineering risk is entirely **erasure completeness + tenant isolation + irreversibility + legal
  basis**, not throughput.

---

## 3. Concern 1 — Erasure completeness + the anonymizer context (Q1 🔴)

### 3.1 The N1 context class — never a context-free connection

`customers` has **no `app.current_tenant` policy arm** — `tenant_isolation USING (location_id IN
(SELECT app_member_location_ids()))` (member-only, `1780310074262:76-77`); the RC4
`app.current_tenant` arm exists **only** on `orders`/`delivery_trace`/`courier_cash_ledger`
(`1790000000077:44-67`) — **confirmed** in RESOLVE-R2 §0. The anonymizer runs on its **own
context-free** pool connection (`index.ts:131,220`: `this.pool.connect()→BEGIN→…→COMMIT`, **no
`set_config` anywhere**). Under **NOBYPASSRLS+MIG-2** that connection is invisible to `customers` RLS
→ the `SELECT … FOR UPDATE` matches **0 rows** → the erasure **silently no-ops**, while the worker
would (pre-#61) write `completed`. Adding a `customers` `app.current_tenant` arm is **REJECTED**
(RESOLVE-R2 N2: it hands every courier-shift/webhook principal SELECT/UPDATE on **all** customers at
their location — a confidentiality hole on the primary PII table).

**Port contract (three layers, all carried):**
1. **Structural success — the DEFINER `gdpr_erase_customer(p_customer, p_location)`** (draft 088). It
   runs as the function **owner** (RLS-visibility-independent); scoping is the fn's **own**
   `WHERE id = p_customer AND location_id = p_location` predicate — the same discipline the anonymizer
   already uses. The Rust worker **calls the fn** (`SELECT * FROM gdpr_erase_customer($1,$2)`); it
   **never** replicates the context-free `pool.connect()→UPDATE customers`. App-side side effects a
   SQL fn cannot do (the avatar **R2 `storage.delete`**, the bus publish) stay in the worker, keyed
   off the fn's returned `out_avatar_key`. This is the "tiny auditable DEFINER ingress-resolver"
   convention (`gdpr_claim_due`, `app_member_location_ids()`).
2. **Fail-loud data-level backstop (ledger #61, carry verbatim).** `completed` + audit + event fire
   **only** when a **re-read confirms `customers.anonymized_at IS NOT NULL`** (credits the idempotent
   already-anonymised case; rejects the no-effect case) — else `status='failed'` +
   `ANONYMIZER_GDPR_FAILED` bus signal, **never `completed`**. The Rust worker ports this gate
   byte-for-byte (`anonymizer-gdpr.ts:74-94`). This makes silent false-completion **structurally
   impossible** independent of (1).
3. **LC4 retry-reset (ledger #61, carry verbatim).** A retryable failure resets `status='pending'`
   (the scan only re-selects `pending`), **never** leaves it `in_progress` (which would strand a
   legally-mandated erasure forever); exhausted retries → `failed` (`:144-164`).

### 3.2 The PII-carrier enumeration — every carrier, text AND object-storage

**"Complete erasure" = every carrier in `compliance/data-map.md` for the subject.** The matrix below
is the port's erasure contract; each row is a red→green assertion at DoD. **CARRIED** = erased today;
**GAP** = a completeness gap this council must disposition (the anonymizer-completeness pattern, #1/#2
already fixed → #3/#4/#5 surfaced here).

| Carrier (table.field / object) | Data-map | Erased today? | Where | Disposition |
|---|---|---|---|---|
| `customers.phone` | #1 | ✅ tokenised `anon_<uuid>` | `index.ts:150-158` / 088 `:65-71` | **CARRY** |
| `customers.name` | #1 | ✅ NULL | same | **CARRY** |
| `customers.marketing_opt_in` | #3 | ✅ false | same | **CARRY** |
| `customers.avatar_key` → R2 object | (avatar) | ✅ object `storage.delete` (column not nulled) | `index.ts:161-177` / 088 returns `out_avatar_key` | **CARRY** (note: the *column* retains a key to a now-deleted object — low risk; flag for tidy-up) |
| `customers.no_show_count / completed_count / last_no_show_at / loyalty_points` | #2/#3 | ❌ retained | — | **ACCEPT-RISK (Q1c)** — pseudonymised counters, LI (fraud/loyalty) once name/phone gone; **must be explicit + legal basis**, not silent |
| `orders.client_ip_hash` | #7 | ⚠️ only via `anonymizeOrder` | `index.ts:242` | **GAP-A** (never reached by GDPR path) |
| `orders.delivery_address / delivery_instructions` | #4 | ⚠️ only via `anonymizeOrder` | `index.ts:243-244` | **GAP-A** |
| `orders.customer_messenger_handle / receiver_name / receiver_handle / receiver_messenger_kind` | #4 | ⚠️ only via `anonymizeOrder` | `index.ts:245-248` | **GAP-A** |
| `orders.delivery_photo_key` → R2 object | #5 | ⚠️ only via `anonymizeOrder` (S4 REV-S4-7 / #74) | `index.ts:249,260-267` | **GAP-A** (the #74 fix lives in the branch the GDPR path never calls) |
| `orders.delivery_lat / delivery_lng` | #4 [HIGH-RISK] | ❌ **never nulled** (not in the `anonymizeOrder` UPDATE) | — | **GAP-B 🔴** — precise home GPS survives even the retention anonymise; data-map documents intent "anonymized_at NULLs" that the code does not fulfil |
| `order_ratings.feedback / rating / customer_id` | #8 | ❌ never touched by either path | — | **GAP-C** — customer free-text (self-identifying) survives erasure |
| `orders.metadata` (ip-hash / channel) | #14/#7 | ✅ (gap #1, historically addressed) | — | **CARRY** — verify no PII copy re-introduced |
| `orders.cash_pay_with` | #6 | ❌ retained | — | **ACCEPT-RISK** — financial record, LI (dispute/tax); explicit |
| `MessageBus payload` | #14 | ✅ claim-check (status/ids only) | p0-privacy | **CARRY** — no name/phone/address on the bus |

**GAP-A (🔴, the reddest): GDPR erasure does not fan out to the subject's orders.**
`anonymize({scope:'gdpr', subject:{customerId, locationId}})` enters the `customerId` branch **only**
(`index.ts:83-88`); it never enumerates that customer's orders → **every** order-carried PII of the
data subject (address, instructions, receiver name/handle, IP hash, **doorway photo**) survives the
Art.17 request until the **retention TTL** (default 365 days) sweeps the *order* by age. **Options**
(Q1a): **(a)** FIX-IN-PORT — the GDPR erasure fans out: after `gdpr_erase_customer`, the worker
selects the subject's orders (tenant-scoped) and runs `anonymizeOrder` for each **under the existing
`orders` RC4 `app.current_tenant=locationId` arm** (or a companion DEFINER `gdpr_erase_order`), so the
#74 photo purge is actually reached; **(b)** ACCEPT-RISK — orders are retained as transaction records
under LI (tax/dispute) with **address/photo erased on the retention TTL**, recorded as an **explicit,
owned, legally-justified accepted-risk** with a documented "undue delay" position — **not** a silent
omission. **Recommendation: (a)** — a doorway photo + home address are the subject's personal data;
"erase me" that leaves them for a year is not a defensible Art.17 completion. This is the fix the S4
council's carrier-completeness pattern points at.

**GAP-B: `orders.delivery_lat/lng` never nulled.** Even the retention `anonymizeOrder` UPDATE
(`index.ts:240-253`) nulls `delivery_address`/`delivery_instructions` but **not** `delivery_lat`/`lng`
— a precise home GPS coordinate survives. **FIX-IN-PORT:** add `delivery_lat=NULL, delivery_lng=NULL`
to the `anonymizeOrder` null-set (confirm the columns exist first — a Phase-0 `ci-schema-drift` read;
else (b) trades a gap for a 500). Owner: S9 lead + operator.

**GAP-C: `order_ratings.feedback` never touched.** Customer free-text feedback (potentially
self-identifying) + `customer_id` survives both paths. **Disposition (Q1b):** fan the erasure out to
`order_ratings` (null `feedback`, or re-key `customer_id`) **or** ACCEPT-RISK with legal basis.
Recommend fan-out under the same GAP-A mechanism.

**Failure-first:** the avatar/`delivery_photo_key` `storage.delete` is **tolerated-and-reported**
(catch + log, **never rethrown, never rolls back** the anonymisation, `index.ts:170-176,261-266`) —
an R2 outage must not block the DB erasure; carry verbatim. But a **repeatedly-failing** object purge
must surface (an ops signal + a re-drive), never a silent orphan — the port adds a purge-failure
counter to the audit metadata (already partially present via `storagePurged`).

## 4. Concern 2 — The gdpr-requests IDOR + status masking (Q2 🔴)

**The cross-tenant erasure IDOR (ledger #57, carry verbatim).** A client-supplied `customerId` is
**unverified** — it must prove same-tenant membership before it can drive an irreversible erasure
(`gdpr.ts:63-86`): `SELECT 1 FROM customers WHERE id=$customerId AND location_id=$locationId`; 0 rows →
`{notOwned:true}` → **404 `NOT_FOUND`** (a plain 404, never distinguishing nonexistent from
cross-tenant to the caller); but a **cross-tenant** attempt (the id exists at *another* location) is
**`request.log.warn({event:'cross_tenant_attempt', …})`** first (`:74-83`) so it stays detectable.
**Port contract:** carry the masked-404 + the security-log verbatim; the classification
(nonexistent-vs-cross-tenant) is **server-side only**. The whole flow runs inside
`withTenant(db, user.userId)` (the owner root, `app.user_id`→memberships) — the S3 REV-10
non-confusable-type combinator.

**The anonymizer scope is REQUIRED, never self-derived (ledger #57, carry verbatim).** The
`|| row.location_id` fallback was **DELETED** — `anonymizeCustomer`/`anonymizeOrder` **throw** if
`options.subject.locationId` is absent (`index.ts:129-132,218-221`), fail-closed. A caller that omits
the scope gets a throw, not a silent same-row "self-proof." Port as a required parameter (a Rust type
that cannot be constructed without the `TenantId`).

**Rate-limit / cooldown (carry verbatim).** fastify `30/min` (`:34-36`); an **active** (pending/
in_progress) request → **409 `CONFLICT`** (`:88-97`, backed by the unique index on those statuses); a
**completed within 24h** → **429 `RATE_LIMIT`** (`:99-107`). Carry all three codes/messages.

**Status reads are not a PII disclosure (Q2, carry + verify).** `GET .../gdpr-requests` and
`.../:requestId` return the subject id **masked** (`maskName(row.customer_id)`, `:184,237`) and the
`anonymization_audit_log` trail with **masked** `subjectId`/`actorId` (`:247-250`). **Port contract:**
the Rust status handlers mask before egress; a guardrail asserts no un-masked `customer_id`/phone
appears in any gdpr-requests response. **Note the asymmetry:** the *request row* stores
`subject_phone` in plaintext (`gdpr.ts:110-115`, data-map #13) — the erasure request itself is a PII
carrier that must live under the same RLS + eventual-retention as the subject; it is **not** returned
un-masked by any read path (verify the port keeps it so).

## 5. Concern 3 — Irreversibility + the safe-reversal boundary (Q3 🔴)

**Erasure "proving it erased" is the N1 data-level gate, NOT a restore-drill.** The restore-drill
(`smoke-checks.ts`) proves **fidelity** — every manifest row comes **back** (LC7 strict full-set
parity, ledger #64). Erasure needs the **opposite** polarity — the row **stays gone**. These must not
be conflated: a restore that brings a row back is a **success** for the drill and would be a
**failure** for an erased subject. **Port contract:** erasure correctness is proven by the **N1
data-level P-proof** (`customers.anonymized_at IS NOT NULL` + `phone` tokenised, under
**NOBYPASSRLS+MIG-2**; negative: fn-absent ⇒ `failed`+DLQ, never `completed`) — a **red→green ledger
row on a legal red-line** — **before** the flip, never a restore false-green.

**The restore-drill's own false-green history (LC7 / #64) is the cautionary tale.** `checkRowCounts`
was a 9-table `count>base*10` heuristic that **passed on a 99%-truncated table** and queried **live
prod** instead of the sandbox; `selectBackup` read a nonexistent column (latent crash-RED that a
naive fix would flip to false-GREEN). **Port lesson carried into S9:** every erasure proof must be
able to go **RED on the real defect** (test-integrity rule) — a proof that asserts status/audit-count
but not the **data-level end-state** is exactly the P5 false-green RESOLVE-R2 rejected.

**Restore-resurrection (Q3, the named residual).** Backups keep **full-PII encrypted** dumps
(Option A / BRK-5 — `checkPIIFree` was **removed** because a faithful restore MUST contain PII). So a
backup taken **before** an erasure still contains the subject's PII; restoring it **resurrects** an
erased subject. **This is a genuine compliance property, not a bug** — but it must be **named and
owned**: (a) backups age out via **R2 lifecycle expiry**, so erasure is "eventually complete" once all
pre-erasure backups expire (the erasure is durable in the live DB immediately; the backup window is
bounded); **and** (b) the operator's **restore runbook must include a re-erase pass** — after any
restore, re-apply all `gdpr_erasure_requests` with `status='completed'` whose `completed_at` precedes
the backup, so a restore cannot silently un-erase a subject. **Recommendation:** document (a) as the
compliance position (bounded backup window + lifecycle expiry) **and** add (b) to the restore runbook
as an operator gate. Owner: operator + counsel (the "undue delay" + backup-retention position).

## 6. Concern 4 — DEFINER search_path (Q4 🔴)

**Draft 088 (`gdpr_erase_customer`) already pins the search_path** —
`SET search_path = pg_catalog, public, pg_temp` (`:40`), closing the DEFINER-hijack class (ledger #33:
`read_public_menu` + 5 unpinned DEFINER fns let a `public`-schema object shadow a called
function/table inside a DEFINER body). Because the fn body references `customers`, `gen_random_uuid()`,
and `now()`, the pin ensures `gen_random_uuid` resolves from `pg_catalog` and `customers` from
`public`, with `pg_temp` **last** (so a temp object can never shadow). **Port contract:**
- **Who runs it:** `REVOKE ALL … FROM PUBLIC` + `GRANT EXECUTE … TO dowiz_app` (`:78-79`). The Rust
  worker connects as **`dowiz_app`** (the operational role) and calls the fn; the fn executes as its
  **owner** (RLS-visibility-independent). No principal outside `dowiz_app` can invoke it.
- **The pin is a named DoD gate, not a detail.** A P8-hygiene assertion (RESOLVE-R2 pattern): **every**
  DEFINER fn the S9 port depends on (`gdpr_erase_customer`, and any companion `gdpr_erase_order` if
  GAP-A(a) is taken) carries a pinned `search_path`; a guardrail greps the migration for an
  unpinned `SECURITY DEFINER`. This inherits the pg-privilege-hardening class — do **not** author a
  new unpinned DEFINER.
- **Fail-closed contract of the fn (carry).** 0 rows (subject not at this tenant) → **empty result** →
  the caller **must** treat it as "no effect" → `failed`, never `completed` (`088:53-55`, dovetails
  with the §3.1 backstop). Already-anonymised → returns the existing timestamp (idempotent success,
  `:57-63`).

## 7. Concern 5 — Retention: legal basis + retained-vs-erased (Q5 🔴)

**The retention sweep** (`anonymizer-retention.ts`) runs **nightly** (`0 3 * * *` UTC,
`ANONYMIZER_RETENTION_CRON`-overridable — **not** monthly; the task's "monthly" is imprecise) under
`pg_try_advisory_lock(4)` single-flight. It: purges expired `customer_track_grants`
(`:45-48`); batch-deletes `funnel_events` older than 90 days (bounded LIMIT-loop, `:54-68`); then per
location runs `anonymize({scope:'retention', locationId})` → `findExpiredCustomers`/`findExpiredOrders`
(older than `retention_days`, `index.ts:303-327`) → `anonymizeCustomer`/`anonymizeOrder`.

**Counsel's live flag: "retention needs a legal basis."** The knobs:
- **`locations.retention_days` is owner-set, 30–2555 days** (`gdpr.ts:272-287` — **up to 7 years**).
  A 7-year retention of `customers.phone`/`name` + order delivery PII needs an **explicit legal basis**
  per location (contract / LI / a statutory record-keeping obligation). Today the code lets an owner
  set any value in-range with **no basis captured**. **Q5 disposition options:** **(a)** ACCEPT-RISK —
  retention is the owner's (controller's) decision under their own legal basis, DeliveryOS is the
  processor; record the accepted-risk + the DPA obligation to document a basis (data-map #1/#4/#13);
  **(b)** capture a basis field / justification at set-time. **Recommend (a) + a documented DPA
  clause** — retention policy is the controller's; the platform must not silently default to the
  **maximum**. **Confirm the default is 365** (`gdpr.ts:268` fallback), a defensible SMB default, not
  2555.
- **What is retained vs erased under retention.** Retention **anonymises** (not deletes)
  customers/orders by **age** — the same carrier null-set as GDPR (§3.2), so it inherits **GAP-B**
  (`delivery_lat/lng` never nulled → precise GPS survives the retention anonymise too) and **GAP-C**
  (`order_ratings.feedback`). The retained pseudonymised **counters** (`no_show_count`, etc.) and the
  **financial** fields (`cash_pay_with`) are LI/statutory — explicit accepted-risk (§3.2).
- **The `funnel_events` 90-day purge + track-grant purge are storage-limitation (Art-5(e))** with a
  clear basis (analytics / expired tracking codes) — carry verbatim; the **best-effort** funnel sweep
  (a failure is non-fatal, `:66-68`) is correct (retention must never wedge the nightly pass).

**Port contract:** carry the sweep semantics verbatim; **fix GAP-B/C** in the shared null-set (§3.2);
surface `retention_days`'s legal-basis position as an **explicit operator/counsel decision** (Q5), not
a silent max. The advisory-lock id (`4`) joins the **lock-id registry** the S8 port introduces
(S8 Q10 — never a raw reused small int).

## 8. Tenancy — how the erasure writes under FORCE RLS

**Three seats, spelled out because the port must not "fix" them into a broken flow:**
- **`customers` erasure → the DEFINER `gdpr_erase_customer`** (no `app.current_tenant` arm on
  `customers`; adding one is REJECTED — §3.1). The fn is visibility-independent; scope is its own
  predicate. **This is the only correct mechanism for `customers` post-flip.**
- **`orders` erasure (GAP-A fan-out) → seat `app.current_tenant=locationId`** — `orders` **has** the
  RC4 arm (`1790000000077:44-67`), so `anonymizeOrder` under a seated `app.current_tenant` passes
  FORCE-RLS `WITH CHECK`. Today `anonymizeOrder` runs context-free (`index.ts:220`, `pool.connect()`,
  no `set_config`) — so post-flip the **retention** order-anonymise **also** silently no-ops (the same
  N1 class on orders). **FIX-IN-PORT:** route `anonymizeOrder` through the seated
  `with_tenant(app.current_tenant=locationId)` combinator (or a companion DEFINER `gdpr_erase_order`),
  with a NOBYPASSRLS probe. **Both** the GDPR fan-out **and** the existing retention order-anonymise
  need this seat — it is not new work for GAP-A alone.
- **The two GDPR bookkeeping tables** (`gdpr_erasure_requests`, `anonymization_audit_log`) — RESOLVE-R2
  N2 **rejected** adding a `app.current_tenant` arm; the worker's terminal writes + audit INSERT run
  through the member-context owner route (creates) and the worker (finalise). The port carries the
  member-only policies; if the worker needs to write them under FORCE-RLS, it uses a DEFINER
  `gdpr_finalize` (RESOLVE-R2 N2 recommendation) — **no arm on the PII/append-only tables**. The audit
  log is **append-only** (documented, `1780421100060:70`) — the port must never UPDATE/DELETE it.
- **Provenance stamps (carry verbatim).** Every audit row stamps the **subject's TRUE tenant**
  (`row.location_id`, read back from the locked row, never trusted blind) + `actor_location_id` /
  `subject_location_id` / `request_id` — the STOP-1 forensic trail (`index.ts:183-196`;
  `anonymizer-gdpr.ts:109-132`, ledger #57 R2-5). Two audit rows for one erasure must never disagree
  on tenant.

## 9. Cutover concurrency — the flip with no cleanup (Q6 🔴)

**Erasure is irreversible; the flip has NO cleanup plan (there is nothing to clean up — it's done).**
The failure classes and controls:

1. **Double-erasure across stacks — bounded to idempotent-safe.** During the S9/S8 overlap, if both
   Node and Rust run the `anonymizer.gdpr` worker, could an erasure double-fire? **No harm:** the
   effect is **idempotent by the `anonymized_at IS NOT NULL` guard** (`index.ts:145,234` / 088
   `:57-63`) — the second run is a no-op success, and the §3.1 backstop confirms `anonymized_at IS NOT
   NULL` → both write `completed` for the same (correct) end-state. **The real cross-stack single-flight
   is the shared-table `gdpr_erasure_requests … WHERE status='pending' FOR UPDATE SKIP LOCKED`**
   (`anonymizer-gdpr.ts:26-33`) — a **database-global** row lock, stack-agnostic, so each request row
   is claimed by exactly one worker regardless of stack or queue. **Control:** carry the shared-table
   `FOR UPDATE SKIP LOCKED` claim as the cross-stack single-flight, and rely on the idempotent guard —
   **not** on the pg-boss singletonKey (which is per-stack). **Note the current claim's TOCTOU** (§11
   Q-CLAIM-TOCTOU): the run() `COMMIT`s **before** marking `in_progress`, so the lock is released and
   two workers could both select the same still-`pending` row; idempotency covers correctness, but the
   port should adopt the **claim-before-work CAS** (RESOLVE-R2 F7 / S8 access-request gold standard) so
   the claim is exclusive, not just idempotent-safe.
2. **The producer/consumer split — the sharpest S9 cutover fact.** The **5 S9 HTTP routes** (create/
   status/retention) flip with the **owner surface**; the **erasure WORKER** is part of the **S8
   background fleet** (S8 §2). So a **Rust-S9 create-request** may **enqueue into a queue drained by a
   still-Node S8 fleet** (or vice versa). **Control:** the create-request writes the **shared business
   table** `gdpr_erasure_requests` (a `pending` row) — the enqueue (`queue.send('anonymizer.gdpr')`,
   `gdpr.ts:131-133`) is a **latency optimisation**, not the source of truth: the S8 fleet's
   **retention/erasure worker recovers any `pending` row from the table directly** (the scan is
   `WHERE status='pending'`, table-driven, stack-agnostic) even if the cross-stack enqueue is dropped.
   So a Rust-created request is **never lost** — worst case it waits for the next worker scan. Confirm
   the Rust-S9 create writes the shared `gdpr_erasure_requests` row (not only a queue job). Owner: S9
   lead + S8 lead + operator (Q6b).
3. **No rollback of the erasure itself.** A proxy flag-flip back to Node leaves the **DB rows already
   anonymised** — correct on either stack (a Rust-erased customer is a normal anonymised row Node
   reads). The rollback is a proxy flag for the **routes**; the **erasure is durable** either way. The
   only irreversible act is the erasure, and it is idempotent + data-level-verified, so a rollback
   mid-overlap leaves no fork.
4. **Prove correctness BEFORE the flip; the flip is human-gated.** Because there is no cleanup for a
   wrong erasure, the DoD is inverted vs other surfaces: **the N1 data-level P-proof + the carrier
   completeness proofs + the cross-tenant IDOR probe must be GREEN under NOBYPASSRLS BEFORE the S9
   flip**, and the flip is a **separate explicit operator go/no-go** (not folded into the S8 fleet
   flip, not automatic). Draft 088 must be **landed before the flip** (§10). The S9 flip runs
   **alongside S5** as one of the two irreversible-surface flips — both demand a human gate.
5. **Connection budget — negligible (§2).** The erasure worker is single-stack (S8 fleet ownership),
   so the overlap does **not** double the worker connection draw; the surface is not connection-bound.

**Cutover DoD gates specific to S9 (in addition to §12):** the N1 data-level erasure P-proof (target
+ negative `failed`/DLQ) under **NOBYPASSRLS+MIG-2** · draft 088 landed (search_path-pinned) · the
carrier-completeness suite (GAP-A/B/C dispositioned + proven) · the cross-tenant erasure IDOR probe
(owner-A `customerId`→404 + security-log) · the status-masking guardrail · the shared-table
`FOR UPDATE SKIP LOCKED` cross-stack single-flight probe · **the flip is a human go/no-go, alongside
S5**.

## 10. Migration-draft interaction (088 + MIG-2 sequencing) (Q7 🔴)

- **Draft `1790000000088` (`gdpr_erase_customer` DEFINER) — land it BEFORE the S9 flip (cutover
  asset).** It is the **structural** post-flip visibility fix (RESOLVE-R2 N1.1); the Rust worker keys
  `completed` off the fn's returned `out_anonymized_at`. Visibility-independent, so it works **pre- and
  post-MIG-2**. Assumes `customers.avatar_key` exists in the target env (the app anonymizer references
  it under a `columnExists` guard) — a **Phase-0 `ci-schema-drift` read** confirms it, else drop
  `avatar_key` from the fn's `RETURNS TABLE` (draft header N). `packages/db/migrations/` red-line,
  operator-placed-verbatim, staging-first, forward-only — **S9 does not author or apply it**.
- **MIG-2 (NOBYPASSRLS anon-policy scoping) — the flip that makes N1 *matter*.** N1's silent
  false-completion is a **flip + MIG-2 latent** (RESOLVE-R2 §0 nuance): pre-MIG-2 the unscoped
  `anonymous_*` policies still admit the context-free erasure; post-MIG-2 they require
  `app.current_tenant` → the context-free `customers` erasure sees ∅. So the **DEFINER 088 must be
  live before MIG-2 + NOBYPASSRLS reach the environment the S9 worker runs in** — sequencing the
  operator owns. The fail-loud backstop (#61, already shipped) makes the **interim safe** (a
  no-effect lands `failed`+DLQ, never a false `completed`).
- **GAP-A(a) / GAP-B fan-out (if taken) — app-side + the existing `orders` RC4 arm.** No new policy
  arm; `anonymizeOrder` seats `app.current_tenant=locationId` (already-present arm) or a companion
  DEFINER `gdpr_erase_order` (search_path-pinned). If a companion DEFINER is authored, it is a **new
  red-line migration** the operator gates (Q4/Q7).
- **Ordering summary:** S9 **code** builds independent of the migration; the **flip** requires **088
  landed + the N1 P-proof green** (§9.4). MIG-2/NOBYPASSRLS is the B3-council's flip — S9 **inherits**
  its timing. Owner: operator (088 placement + MIG-2 sequencing) + S9 lead + B3-council.

## 11. Quirk register — carry-vs-fix (default = CARRY-VERBATIM)

**FIX-IN-PORT only for a 🔴 security/correctness/legal issue or a build-correctness bug, each with an
explicit test/E2E delta.** Everything else CARRIES.

| ID | Quirk (source) | Disposition |
|---|---|---|
| Q-N1-CONTEXT | anonymizer runs on a **context-free** pool conn (`index.ts:131,220`); `customers` has no `app.current_tenant` arm → post-MIG-2 silent ∅ no-op | **FIX-IN-PORT** → erase via the DEFINER `gdpr_erase_customer` (088, visibility-independent); NEVER a raw context-free UPDATE (🔴 T-NONERASURE) |
| Q-N1-BACKSTOP | worker gates `completed` on a **data-level re-read** `anonymized_at IS NOT NULL`, else `failed`+DLQ (ledger #61, `anonymizer-gdpr.ts:74-94`) | **CARRY verbatim** — the fail-loud gate; makes silent false-completion structurally impossible (🔴 T-NONERASURE) |
| Q-LC4-RETRY | retryable failure resets `status='pending'` (scan re-selects); never left `in_progress` (`:144-156`) | **CARRY verbatim** — a stranded `in_progress` never re-runs (legal-mandate liveness) |
| Q-ERASE-FANOUT | GDPR-by-customer calls `anonymizeCustomer` **only** — orders' PII (address/photo/receiver) survives to retention TTL (`anonymizer-gdpr.ts:62-65`) | **FIX-IN-PORT (🔴 Q1a, GAP-A)** — fan the erasure out to the subject's orders (reach the #74 photo purge); or explicit legal accepted-risk. E2E: erase → subject's orders anonymised, `delivery_photo_key` object deleted |
| Q-ERASE-LATLNG | `anonymizeOrder` UPDATE nulls address/instructions but **not** `delivery_lat/lng` (`index.ts:240-253`) | **FIX-IN-PORT (Q1, GAP-B)** — add `delivery_lat/lng=NULL` to the null-set (confirm columns exist first); precise home GPS survives otherwise |
| Q-ERASE-RATINGS | `order_ratings.feedback` (customer free-text) never touched by either path (data-map #8) | **FIX-IN-PORT (Q1, GAP-C)** — fan out to `order_ratings`; or explicit accepted-risk with basis |
| Q-PHOTO-PURGE | `delivery_photo_key` R2 purge, tolerated-and-reported (S4 REV-S4-7 / #74, `index.ts:260-267`) | **CARRY verbatim** — object `storage.delete`, never rethrown/rolled-back; the pattern GAP-A must actually reach |
| Q-IDOR-MASK404 | client `customerId` proved same-tenant → masked **404** + `cross_tenant_attempt` security-log (`gdpr.ts:63-86`, ledger #57) | **CARRY verbatim** — no leak; classification server-side only (🔴 T-XTENANT) |
| Q-SCOPE-FAILCLOSED | anonymizer `locationId` **required**, the `\|\| row.location_id` self-derive **DELETED**, throws if absent (`index.ts:129-132`) | **CARRY verbatim** — fail-closed scope; port as a non-constructible-without-`TenantId` type |
| Q-STATUS-MASK | status reads mask `customerId`/`subjectId`/`actorId` via `maskName` (`gdpr.ts:184,237,247-250`) | **CARRY verbatim + GUARDRAIL** — the status surface is not a PII disclosure (🔴 Q2) |
| Q-COOLDOWN | active→409, completed-within-24h→429, fastify 30/min (`gdpr.ts:88-107,34-36`) | **CARRY verbatim** — the abuse/replay guards |
| Q-DEFINER-PIN | draft 088 pins `search_path = pg_catalog, public, pg_temp`; `REVOKE PUBLIC`+`GRANT dowiz_app` (`:40,78-79`) | **CARRY + GUARDRAIL** — no unpinned `SECURITY DEFINER`; the pg-privilege-hardening class (🔴 Q4) |
| Q-DEFINER-FAILCLOSED | 088 returns **empty** on 0 rows → caller treats as no-effect → `failed`, never `completed` (`:53-55`); already-anon → idempotent success (`:57-63`) | **CARRY verbatim** — dovetails with the §3.1 backstop |
| Q-AUDIT-APPEND | `anonymization_audit_log` append-only + subject-true-tenant + STOP-1 provenance (`index.ts:183-196`, ledger #57 R2-5) | **CARRY verbatim** — never UPDATE/DELETE the audit log; two rows for one erasure agree on tenant |
| Q-RETENTION-BASIS | `retention_days` owner-set **30–2555** (up to 7yr), no basis captured; default 365 (`gdpr.ts:272-287,268`) | **ACCEPT-RISK + DPA clause (🔴 Q5)** — controller's decision; don't default to max; record basis obligation |
| Q-RETENTION-CRON | nightly `0 3 * * *` UTC, `pg_try_advisory_lock(4)` single-flight, funnel-90d + track-grant purge best-effort (`anonymizer-retention.ts`) | **CARRY verbatim** — UTC cron; advisory-lock `4` joins the S8 lock-id registry; funnel sweep non-fatal |
| Q-CLAIM-TOCTOU | run() `COMMIT`s the `FOR UPDATE SKIP LOCKED` select **before** marking `in_progress` → two workers can select the same pending row (`anonymizer-gdpr.ts:26-40`) | **FIX-IN-PORT (recommend)** — claim-before-work CAS (RESOLVE-R2 F7 / S8 gold standard); idempotency covers correctness today but the claim is not exclusive (🔴 Q6) |
| Q-GDPR-SINGLETON | `anonymizer.gdpr` global singletonKey = at most ONE erasure in-flight system-wide (S8 Q-GDPR-GLOBAL-SINGLETON) | **CARRY (intentional serialization)** — a queueing-theory ceiling only if volume grows; recommend CAS over global singleton (Q6) |
| Q-RESTORE-RESURRECT | full-PII encrypted backups (Option A / BRK-5) → restoring a pre-erasure backup resurrects an erased subject | **ACCEPT-RISK + RUNBOOK (🔴 Q3)** — bounded backup window + R2 lifecycle expiry; restore runbook re-applies completed erasures |
| Q-RESTORE-DRILL | the restore-drill proves **fidelity** (rows come back, LC7 strict parity #64), NOT erasure | **CARRY (do not repurpose)** — erasure proof is the N1 data-level P-proof, opposite polarity (🔴 Q3) |
| Q-AVATAR-COL | avatar object deleted but the `customers.avatar_key` **column** not nulled (088 returns it, doesn't null it) | **CARRY + TIDY-FLAG** — column holds a dead key to a deleted object; low risk; null it in the fan-out fix |

## 12. Cutover DoD (REBUILD-MAP §3, this surface)

gdpr-requests owner E2E slice green · `openapi-diff` empty for the S9 namespace · invariant-cluster
red→green:
- **Silent-non-erasure (N1)** — under **NOBYPASSRLS+MIG-2**: drive one erasure end-to-end → target
  `customers.anonymized_at IS NOT NULL` + `phone` tokenised; **negative**: DEFINER absent / context
  withheld → `status='failed'` + `ANONYMIZER_GDPR_FAILED`, **never `completed`** (the ledger #61
  backstop suite, extended to the data level). A **legal-red-line red→green ledger row**.
- **Erasure completeness** — the carrier matrix (§3.2): every CARRY row proven erased; **GAP-A**
  (subject's orders anonymised, `delivery_photo_key` object deleted — the #74 purge reached),
  **GAP-B** (`delivery_lat/lng` nulled), **GAP-C** (`order_ratings.feedback`) — each dispositioned
  (fix + proof, or explicit owned accepted-risk).
- **Cross-tenant erasure IDOR (Q2)** — owner-A `customerId` at tenant-B → **404** + `cross_tenant_attempt`
  log; own → erasure; the `\|\| row.location_id` fail-open stays deleted (`anonymizer-fail-closed`).
- **Status masking (Q2)** — no un-masked `customer_id`/phone in any gdpr-requests response (guardrail).
- **DEFINER (Q4)** — 088 landed, search_path-pinned; a guardrail greps for any unpinned
  `SECURITY DEFINER` the S9 port depends on; only `dowiz_app` may `EXECUTE`.
- **Retention (Q5)** — the sweep anonymises by TTL (GAP-B/C fixed in the shared null-set); the
  `retention_days` legal-basis position recorded (default 365, not max); the funnel/track-grant purges
  carried.
- **Irreversibility (Q3)** — the erasure proof is the **data-level** gate (not a restore false-green);
  the restore-resurrection runbook (re-apply completed erasures post-restore) recorded.
- **Cutover (Q6)** — the shared-table `FOR UPDATE SKIP LOCKED` cross-stack single-flight probe;
  a Rust-S9 create writes the shared `gdpr_erasure_requests` row (worker-recoverable if the enqueue
  drops); **the flip is a separate human go/no-go, alongside S5**.

map-coverage zero-diff for the S9 namespace · **council sign-off + human go/no-go** (no auto-flip;
erasure has no cleanup — correctness is proven BEFORE the flip). **No 🔴 S9 row builds before this
packet is APPROVED and the 🔴 questions (Q1–Q7) are operator-signed.**

---

**council seats to run: breaker, counsel** (architect authored; human operator signs 🔴 Q1
erasure-completeness/N1 + the orders-fan-out gap / Q2 cross-tenant IDOR + status masking / Q3
irreversibility + restore-resurrection / Q4 DEFINER search_path / Q5 retention legal basis / Q6
cutover human-gate + cross-stack single-flight / Q7 draft-088 + MIG-2 sequencing).
**packet-status: 🟡 DRAFT.**
