# S9-GDPR/COMPLIANCE Port — Council Packet · THREAT MODEL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Adversarial input to the S9 council. Assets, trust boundaries,
> and the failure modes the Rust port must not silently introduce — on the one surface where a defect
> is a **legal-compliance** failure (a false Art.17 completion, an incomplete erasure, an erasure of
> the wrong subject) and the operation is **irreversible**. Read alongside `proposal.md`. Docs only.

- **Method:** STRIDE-lite over the S9 GDPR/compliance surface + fold-in of the N1 CRITICAL
  (audit-fix-rls-reliability RESOLVE-R2), the LC5 cross-tenant erasure IDOR (ledger #57), the S4
  REV-S4-7 / #74 carrier-completeness pattern, the LC7 restore false-green (ledger #64), the DEFINER
  search_path class (ledger #33), and `compliance/data-map.md` as the code-grounded definition of
  "complete erasure."
- **Scope note:** the B3 (NOBYPASSRLS) flip + MIG-2 anon-policy scoping are **B3-council** changes;
  recorded here because they change what S9 must hold (they turn the N1 latent into a certainty), but
  their *fix* lives in that council. The **erasure RUNTIME** (queue/cron/single-flight) is **S8**; S9
  owns the erasure **semantics**. The **backup/restore pipeline** is the backup/DR council; S9 consumes
  its fidelity property + names the restore-resurrection hazard.

---

## 1. Assets

| ID | Asset | Where it lives | Why it matters |
|---|---|---|---|
| G1 | The **erasure completion** (`gdpr_erasure_requests.status='completed'` + `anonymization_audit_log`) | `gdpr_erasure_requests` / audit log (tenant, RLS) | The legal record that an Art.17 request was honoured; a **false `completed`** is a silent legal-compliance failure (N1) |
| G2 | The **subject's PII carriers** — `customers.phone/name/avatar_key`, `orders.delivery_address/lat/lng/instructions/receiver_*/delivery_photo_key/client_ip_hash`, `order_ratings.feedback` | `customers`/`orders`/`order_ratings` (tenant, RLS) + **R2 objects** (avatar, doorway photo) | The data that must actually be gone; a carrier missed (GAP-A/B/C) = an **incomplete erasure** — the subject's home + face survive (data-map #1/#4/#5/#8) |
| G3 | The **tenant boundary on erasure** (the same-tenant `customerId` proof) | `gdpr.ts:63-86` + RLS | An unverified client `customerId` could drive an **irreversible cross-tenant** erasure (the worst class) |
| G4 | The **erasure request itself** (`gdpr_erasure_requests.subject_phone/customer_id/requested_by_owner_id`) | `gdpr_erasure_requests` (tenant, RLS) | The request row is a PII carrier (plaintext `subject_phone`, data-map #13); the status surface must not leak it |
| G5 | The **DEFINER `gdpr_erase_customer`** (owner-privileged erase) | `pg_proc` (owner-owned fn) | Runs as owner (RLS-independent); an unpinned search_path or a wider grant is a privilege-escalation / hijack vector (ledger #33) |
| G6 | The **retention policy** (`locations.retention_days`) | `locations` (tenant, RLS) | Drives time-based anonymisation; a 7-year retention with no basis is a storage-limitation (Art-5(e)) failure |
| G7 | The **audit provenance** (subject-true-tenant + actor/subject/request stamps) | `anonymization_audit_log` (append-only) | The forensic trail that an erasure was scoped to the right subject/tenant (STOP-1); a forged/erased audit row destroys accountability |
| G8 | The **backup dumps** (full-PII, encrypted) | R2 (Option A / BRK-5) | A pre-erasure backup contains the erased PII; a restore can **resurrect** an erased subject |

## 2. Trust boundaries

- **TB-1 owner → erasure request (`POST .../gdpr-requests`)** — authenticated owner (verifyAuth +
  requireRole(owner) + requireLocationAccess). The request body carries a **client-supplied
  `customerId`/`phone`** — the input that selects **whose** data is irreversibly erased. The tenant is
  the URL `:locationId` (membership-checked); the `customerId` is proven same-tenant (G3) before it can
  drive the erasure.
- **TB-2 request → tenant GUC seat (the erasure write plane)** — the anonymizer's write. **The N1
  boundary:** `customers` has **no `app.current_tenant` arm**, so a context-free connection is
  **invisible** to `customers` RLS post-MIG-2. The erasure must cross this boundary via the **DEFINER
  fn** (owner-privileged, visibility-independent), never a context-free UPDATE. Order erasure (GAP-A
  fan-out) crosses via the `orders` RC4 `app.current_tenant` arm (seated).
- **TB-3 worker → completion write** — the worker turns a `pending` request into `completed`/`failed`.
  The boundary the N1 backstop guards: the completion must be **keyed off the data-level end-state**
  (`anonymized_at IS NOT NULL`), never off "the call returned."
- **TB-4 owner → status read (`GET .../gdpr-requests*`)** — the read surface returns the request +
  audit trail; the boundary is **masking** (`maskName`) — the status of an erasure must not itself
  disclose the (un-erased or erased) subject id/phone.
- **TB-5 caller → DEFINER fn** — only `dowiz_app` may `EXECUTE gdpr_erase_customer`; the fn runs as
  owner. The boundary is the `search_path` pin (no `public`-schema shadow) + the `REVOKE PUBLIC`.
- **TB-6 stack → stack (cutover)** — during the S9/S8 overlap, a Rust-S9 create-request and a
  (possibly Node) S8 erasure worker act on the **shared `gdpr_erasure_requests` table**; the trust is
  mediated by the shared-table `FOR UPDATE SKIP LOCKED` row lock + the idempotent `anonymized_at` guard,
  not by any per-stack queue token.
- **TB-7 backup → restore (irreversibility)** — a restore re-materialises rows from a pre-erasure
  dump; the boundary between "erased" and "resurrected" is the R2 lifecycle window + the
  re-erase-on-restore runbook.

## 3. Port-specific failure scenarios (Rust)

| # | Scenario | Trigger in the port | Mitigation to prove red→green |
|---|---|---|---|
| **S9-T1** | **Silent non-erasure (N1)** — the worker writes `completed` but the row was never erased (post-MIG-2 the context-free `customers` erasure sees ∅) | Carrying the context-free `pool.connect()→UPDATE customers` (`index.ts:131`); writing `completed` off the call, not the data | Erase via the DEFINER `gdpr_erase_customer` (088, visibility-independent); the **data-level backstop** (`anonymized_at IS NOT NULL` → else `failed`+DLQ, ledger #61); the **N1 data-level P-proof** under NOBYPASSRLS+MIG-2 (target + negative `failed`/DLQ) |
| **S9-T2** | **Incomplete erasure (GAP-A/B/C)** — the subject's orders (address / doorway photo / receiver name), GPS, or feedback survive an Art.17 request | GDPR path calls `anonymizeCustomer` **only** (`anonymizer-gdpr.ts:62-65`) — never `anonymizeOrder`; `delivery_lat/lng` + `order_ratings.feedback` in no null-set | Fan the erasure out to the subject's orders (reach the #74 photo purge) under the `orders` RC4 arm; add `delivery_lat/lng` + `order_ratings.feedback` to the null-set; a **carrier-completeness suite** asserts each carrier gone after an erasure (or an explicit owned accepted-risk) |
| **S9-T3** | **Cross-tenant PII leak / cross-tenant erasure** — owner-A's `customerId` reads or **irreversibly erases** tenant-B's subject | Skipping the same-tenant `customerId` proof; distinguishing 404-vs-403 to the caller; the `\|\| row.location_id` self-derive | Carry the masked-**404** + `cross_tenant_attempt` security-log (`gdpr.ts:63-86`); the anonymizer scope **required** (fail-closed, `\|\| row.location_id` deleted); E2E owner-A `customerId`@B → 404 + log; own → erase |
| **S9-T4** | **DEFINER hijack / privilege escalation** — a `public`-schema object shadows a name inside the DEFINER body, or a wider grant lets a non-worker invoke the erase | An unpinned `search_path` (ledger #33 class); a `GRANT … TO PUBLIC` | Carry 088's pin `SET search_path = pg_catalog, public, pg_temp` + `REVOKE PUBLIC` + `GRANT dowiz_app`; a guardrail greps for any unpinned `SECURITY DEFINER` on the S9 erasure path; only `dowiz_app` may EXECUTE |
| **S9-T5** | **Erasure of the wrong subject** — a `phone`/`customerId` resolves to a different customer, or a stale phone re-resolves nothing | Phone re-resolution `WHERE location_id AND phone` against a tokenised/rotated phone; a customerId that was hard-deleted then reused | Bind the request to the resolved `customer_id` at create-time (tenant-proven); the worker's re-resolve is tenant-scoped; a no-match → `failed 'Customer not found'` (`anonymizer-gdpr.ts:54-59`), never a wrong-row erase; the DEFINER's own `WHERE id=p_customer AND location_id=p_location` predicate |
| **S9-T6** | **Restore false-green** — an erasure "proof" passes because it asserts status/audit-count, not the data end-state; or the restore-drill is repurposed as an erasure oracle | A P5-style proof (status/audit only, RESOLVE-R2-rejected); conflating restore-fidelity (rows come back) with erasure (rows stay gone) | The erasure proof is the **data-level** re-read (`anonymized_at IS NOT NULL`), able to go RED on the real defect; the restore-drill (LC7 strict parity) proves **fidelity only** and is never the erasure oracle (opposite polarity) |
| **S9-T7** | **Restore-resurrection** — a restore of a pre-erasure encrypted backup un-erases a subject | Restoring an Option-A full-PII backup taken before the erasure; no re-erase pass | Named accepted-risk + runbook: bounded backup window + R2 lifecycle expiry; the restore runbook **re-applies all `status='completed'` erasures** whose `completed_at` precedes the backup |
| **S9-T8** | **Retention without legal basis** — a 7-year `retention_days` retains PII the data-map marks HIGH-RISK, with no basis | Defaulting to / allowing the 2555 max silently; no basis captured (`gdpr.ts:272-287`) | Accept-risk + DPA clause (controller's decision); confirm the default is **365** not the max; fix GAP-B/C in the retention null-set so the sweep does not silently retain GPS/feedback |
| **S9-T9** | **Double-erasure across stacks** — both Node + Rust workers process the same request | Two stacks scanning `gdpr_erasure_requests`; a COMMIT-then-mark TOCTOU (`anonymizer-gdpr.ts:26-40`) | Shared-table `FOR UPDATE SKIP LOCKED` (database-global row lock) as the cross-stack single-flight; the idempotent `anonymized_at` guard makes a double-run a no-op success; **adopt claim-before-work CAS** so the claim is exclusive, not just idempotent-safe |
| **S9-T10** | **Audit-trail forgery / loss** — a courier/webhook principal forges or erases an `anonymization_audit_log` row, or an audit row disagrees with its erasure on tenant | Adding a `FOR ALL`/DELETE `app.current_tenant` arm to the append-only tables (RESOLVE-R2 N2); trusting the request's tenant for the audit stamp | Route the worker's audit write through a DEFINER `gdpr_finalize` (no arm on the PII/append-only tables); stamp the **subject-true-tenant** read back from the row; append-only preserved (P8: no `FOR ALL`/DELETE arm on either GDPR table) |
| **S9-T11** | **Status-surface PII disclosure** — the gdpr-requests read returns an un-masked `customer_id`/phone | Dropping `maskName` on `customer_id`/`subjectId`/`actorId`; returning the plaintext `subject_phone` | Carry the masking (`gdpr.ts:184,237,247-250`); a guardrail asserts no un-masked `customer_id`/phone/`subject_phone` in any gdpr-requests response |
| **S9-T12** | **Stranded / lost erasure (LC4)** — a legally-mandated erasure never reaches a terminal state | A retryable failure leaving `status='in_progress'` (the scan only re-selects `pending`) | Carry the LC4 reset-to-`pending` on retryable failure (`:144-156`); exhausted → `failed`; a guardrail asserts no path leaves a request `in_progress` without a re-enqueue |
| **S9-T13** | **Object-storage orphan** — the DB erases but the R2 avatar/doorway object survives (or a repeated purge failure goes silent) | An R2 outage swallowed (correctly) but never re-driven; a dead `avatar_key` column left pointing at a deleted object | Carry the tolerated-and-reported purge (never rethrown); add a purge-failure counter + ops signal + re-drive; null the `avatar_key`/`delivery_photo_key` columns in the erasing UPDATE (not just the object) |

## 4. What the B3 RLS flip changes for S9

- **Today (BYPASSRLS):** RLS is bypassed; the anonymizer's context-free connection "works," the
  explicit `WHERE id=$ AND location_id=$` predicates are the only live boundary, and a non-erasure is
  masked. The danger is **invisible** — the exact N1 masking on the reddest surface.
- **Post-flip (NOBYPASSRLS) + MIG-2:** RLS is authoritative. `customers` has **no `app.current_tenant`
  arm** → the context-free `customers` erasure sees **∅** and no-ops (S9-T1); the retention/GDPR
  `orders` erasure needs `app.current_tenant` seated to pass FORCE-RLS (S9-T2 order fan-out). **The
  DEFINER `gdpr_erase_customer` is the visibility-independent crossing** for `customers`; the seated
  `orders` RC4 arm is the crossing for orders. N1's silent false-completion is a **flip+MIG-2 latent**
  (RESOLVE-R2 §0) — the DEFINER 088 must be live before MIG-2 reaches the worker's env; the #61
  backstop makes the interim safe (no-effect → `failed`, never false `completed`).
- **S9's rule:** the erasure is correct **independent of which pool role is live** — the DEFINER fn is
  owner-privileged (role-independent), the completion is data-level-verified (not call-return-trusted),
  and the scope is required (never self-derived). The B3 flip and the Node→Rust flip are two
  orthogonal, independently-reversible events — **except the erasure itself, which is irreversible and
  must be proven correct under NOBYPASSRLS BEFORE the S9 flip** (Q6).

## 5. Residual risks (summary for the human)

- **GAP-A — GDPR erasure never fans out to the subject's orders (S9-T2 / Q1a1).** The single reddest
  finding: the just-landed `delivery_photo_key` purge (#74) lives in `anonymizeOrder`, which the
  GDPR-by-customer path **never calls** — the subject's home address + doorway photo survive an Art.17
  request up to `retention_days` (default 365). **The most likely breaker escalation.** Fixable
  (fan-out under the `orders` RC4 arm) or an explicit legal accepted-risk — never silent. Owner: S9
  lead + operator + counsel.
- **Retention legal basis (S9-T8 / Q5)** — a 7-year `retention_days` with no captured basis; the
  platform must not default to the max or silently retain HIGH-RISK carriers (GPS, feedback). **A
  likely counsel flag** — defensible only as the controller's documented decision + a DPA clause.
- **Restore-resurrection (S9-T7 / Q3)** — a pre-erasure encrypted backup can un-erase a subject.
  Defensible only as a bounded backup window + R2 lifecycle expiry + a re-erase-on-restore runbook —
  an explicit, owned position, not a silent hole. Owner: operator + counsel.
- **The claim TOCTOU (S9-T9 / Q6)** — the worker's COMMIT-then-mark leaves the `pending` row selectable
  by a second worker; idempotency covers correctness but the claim is not exclusive. Adopt the
  claim-before-work CAS (RESOLVE-R2 F7 / S8 gold standard). Owner: S9 lead + S8 lead.
- **The erasure request row is a PII carrier (G4)** — `subject_phone` is plaintext in
  `gdpr_erasure_requests`; it lives under RLS + eventual retention and is never returned un-masked, but
  the port must not accidentally widen its exposure (e.g. logging it, or returning it in an admin read).
  Owner: S9 lead.

**None of G1–G8's failure modes is *introduced* by the rewrite** — each (the erasure running under
enforcement, reaching every carrier, scoped to the right tenant, verified at the data level, proven
before the flip) is a **current** property the port must carry **visibly** (matrix row + test). The
rewrite's *new* risks are the **cutover** (S9-T9, TB-6/TB-7 — two stacks + a restore window on an
irreversible operation) and the **discipline of completeness** (S9-T2 — a from-scratch port that copies
"anonymize the customer" without the S4 carrier-completeness lesson would re-ship GAP-A/B/C). **Breaker-
escalation candidate: GAP-A (the orders fan-out, S9-T2).** **Counsel-flag candidates: the retention
legal basis (S9-T8) + the restore-resurrection window (S9-T7)** — each acceptable only as an explicit,
owned position, never by silence.

---

**council seats: breaker, counsel** · **packet-status: 🟡 DRAFT.**
