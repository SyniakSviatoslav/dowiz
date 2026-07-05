# S9-GDPR/COMPLIANCE Port — Council Packet · OPEN QUESTIONS

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Everything the live Triadic Council (architect / breaker /
> counsel / human) must decide before S9 GDPR/compliance is ported. Each question has options + a
> lane-R3 recommendation — a *starting position for friction*, not a decision. This is the **REDDEST
> surface in the rebuild** (irreversible erasure + a **legal-compliance** failure mode); **every Q is
> 🔴** by design. Docs only.

Legend: **[ERASE]** erasure-completeness/correctness · **[SEC]** tenant-isolation/leak · **[LEGAL]**
compliance basis · **[INFRA]** cutover/topology · **[MIGRATION]** DB-draft interaction. 🔴 = red-line,
operator sign-off required.

---

### Q1 🔴 [ERASE] Erasure completeness + the anonymizer context (N1) — and the orders fan-out gap
Two coupled facts. **(i) The N1 context class:** `customers` has **no `app.current_tenant` arm** (RC4
is orders-only); the anonymizer runs **context-free** (`index.ts:131,220`) → post-NOBYPASSRLS+MIG-2 the
erasure sees ∅ and **silently no-ops**. **(ii) Completeness:** the carrier audit (proposal §3.2)
surfaces **GAP-A** (GDPR erasure calls `anonymizeCustomer` **only** — the subject's orders' address /
`delivery_photo_key` / receiver name survive to the retention TTL, so the #74 photo purge is **never
reached**), **GAP-B** (`orders.delivery_lat/lng` never nulled — precise home GPS survives), **GAP-C**
(`order_ratings.feedback` untouched).
- **(a) FIX the context:** erase `customers` via the DEFINER `gdpr_erase_customer` (088,
  visibility-independent — NEVER a context-free UPDATE) + carry the fail-loud data-level backstop
  (`anonymized_at IS NOT NULL` → else `failed`+DLQ, ledger #61) + the N1 data-level P-proof. *(recommend)*
- **(a1) [GAP-A] FIX the fan-out:** the GDPR erasure fans out to the subject's **orders** (reach the
  #74 photo purge) under the existing `orders` RC4 `app.current_tenant` arm (or a companion DEFINER
  `gdpr_erase_order`). *(recommend)* — **(a2) ACCEPT-RISK:** orders retained as transaction records
  under LI, address/photo erased on the retention TTL — **explicit, owned, legal-basis'd** accepted-risk
  with an "undue delay" position, never silent.
- **[GAP-B/C] FIX** `delivery_lat/lng` (confirm columns exist via Phase-0 `ci-schema-drift`) +
  `order_ratings.feedback` in the shared null-set. *(recommend)*
- **(b) Add a `customers` `app.current_tenant` arm** — **rejected** (RESOLVE-R2 N2): hands every
  courier-shift/webhook principal SELECT/UPDATE on all customers at their location — a confidentiality
  hole on the primary PII table.

**R3 recommendation:** (a) + (a1) + GAP-B/C fixes. **🔴** — a silent non-erasure is a false Art.17
completion; an incomplete erasure ("erase me" that leaves the home address + doorway photo for a year)
is not a defensible completion. GAP-A is **the most likely breaker escalation** in this packet. Owner:
S9 lead + operator + counsel (the "undue delay" position if (a2)).

### Q2 🔴 [SEC] The gdpr-requests IDOR + status masking
A client-supplied `customerId`/`phone` drives an **irreversible** erasure. (i) A cross-tenant
`customerId` → masked **404** + a `cross_tenant_attempt` security-log (`gdpr.ts:63-86`, ledger #57);
the anonymizer scope is **required** (the `|| row.location_id` self-derive **deleted**). (ii) Status
reads mask `customerId`/`subjectId`/`actorId` (`maskName`).
- **(a) CARRY verbatim:** the masked-404 + security-log + fail-closed scope + status masking; port the
  scope as a non-constructible-without-`TenantId` type; a guardrail asserts no un-masked
  `customer_id`/phone in any gdpr-requests response. *(recommend)*
- **(b) Distinguish 404-nonexistent from 403-cross-tenant to the caller** — **rejected**: a
  cross-tenant 403 confirms the id exists at another tenant (enumeration leak); the classification
  stays server-side only.

**R3 recommendation:** (a). **🔴** — a cross-tenant erasure is the worst class (irreversible +
cross-tenant); the status surface must not itself leak PII. Owner: S9 lead + operator.

### Q3 🔴 [LEGAL] Irreversibility + safe-reversal (the restore boundary)
Erasure is IRREVERSIBLE. The restore-drill proves **fidelity** (rows come **back**, LC7 strict parity
#64) — the **opposite** polarity of erasure (rows **stay gone**). And full-PII **encrypted** backups
(Option A / BRK-5) mean a pre-erasure restore **resurrects** an erased subject.
- **(a) Erasure correctness = the N1 data-level P-proof** (`anonymized_at IS NOT NULL` under
  NOBYPASSRLS+MIG-2; negative → `failed`/DLQ) — **never** a restore false-green (the restore-drill is
  not repurposed as an erasure oracle). **+ (a1)** name restore-resurrection as an **accepted-risk +
  runbook**: bounded backup window + R2 lifecycle expiry, and the restore runbook **re-applies all
  completed erasures** post-restore. *(recommend)*
- **(b) Treat the restore-drill as the erasure oracle** — **rejected**: opposite polarity; a restore
  that brings a row back would falsely "pass" an erasure check.
- **(c) Purge/scrub PII from backups so a restore cannot resurrect** — **rejected** (BRK-5): a
  faithful restore MUST contain PII; a pii-free assertion fails every real restore. Protection is
  encryption-at-rest + lifecycle expiry + the re-erase-on-restore runbook, not scrubbing the dump.

**R3 recommendation:** (a) + (a1). **🔴** — "proving it erased" must be able to go RED on the real
defect (test-integrity, legal red-line); the backup window is a real, bounded compliance property the
operator + counsel own, not a silent hole. Owner: operator + counsel (undue-delay + backup-retention).

### Q4 🔴 [MIGRATION/SEC] DEFINER search_path + who runs it
The DEFINER `gdpr_erase_customer` (draft 088) pins `SET search_path = pg_catalog, public, pg_temp`
(`:40`), `REVOKE PUBLIC` + `GRANT EXECUTE TO dowiz_app` (`:78-79`) — closing the DEFINER-hijack class
(ledger #33). The Rust worker (as `dowiz_app`) calls it; the fn runs as owner (RLS-visibility-independent).
- **(a) CARRY the pin + grant; a guardrail asserts no unpinned `SECURITY DEFINER`** the S9 port
  depends on (incl. any companion `gdpr_erase_order` if Q1(a1) is taken); only `dowiz_app` may EXECUTE.
  *(recommend)*
- **(b) Author the fan-out (Q1a1) as an app-side seated UPDATE, no companion DEFINER** — acceptable
  (orders HAS the RC4 arm); then only 088 is a DEFINER dependency.

**R3 recommendation:** (a); prefer (b)'s app-side seat for orders (reuse the existing arm) over a new
DEFINER, minimising the DEFINER surface. **🔴** — an unpinned DEFINER on the erasure path is a
privilege-escalation vector; the pin + grant are a named DoD gate. Owner: operator (migration red-line)
+ S9 lead.

### Q5 🔴 [LEGAL] Retention — legal basis + retained-vs-erased
`locations.retention_days` is owner-set **30–2555 days (up to 7yr)** with **no basis captured**
(`gdpr.ts:272-287`); the nightly sweep anonymises customers/orders by age (inheriting GAP-B/C). Counsel
flagged: "retention needs a legal basis."
- **(a) ACCEPT-RISK + DPA clause:** retention policy is the **controller's** (owner's) decision under
  their own basis; DeliveryOS is the processor. Record the accepted-risk + the DPA obligation to
  document a basis; **confirm the default is 365** (a defensible SMB default, `gdpr.ts:268`), never
  silently the 2555 max. *(recommend)*
- **(b) Capture a justification/basis field at set-time** — heavier; a product decision; defer to a
  retention-policy council if the DPA requires it.
- Retained-vs-erased: pseudonymised counters (`no_show_count`…) + financial (`cash_pay_with`) are LI/
  statutory — **explicit** accepted-risk (proposal §3.2), not silent.

**R3 recommendation:** (a) + fix GAP-B/C in the shared null-set. **🔴** — a 7-year retention of PII
needs a basis; the platform must not default to the maximum or silently retain carriers the data-map
marks HIGH-RISK. Owner: operator (controller policy) + counsel (DPA clause).

### Q6 🔴 [INFRA] Cutover — the flip with no cleanup; cross-stack single-flight
The **5 S9 routes** flip with the owner surface; the **erasure worker** is part of the **S8 fleet** —
a producer/consumer **split** across surfaces. Erasure is irreversible → **no cleanup plan** (it's
done). Double-erasure is idempotent-safe (`anonymized_at` guard); the real cross-stack single-flight is
the shared-table `FOR UPDATE SKIP LOCKED`, not the pg-boss singletonKey.
- **(a) Human-gated flip, alongside S5:** prove correctness (N1 P-proof + carrier completeness +
  IDOR) under NOBYPASSRLS **BEFORE** the flip; a Rust-S9 create writes the **shared `gdpr_erasure_requests`
  row** (worker-recoverable if the cross-stack enqueue drops); the flip is a **separate explicit
  operator go/no-go**, not folded into the S8 fleet flip. Adopt the **claim-before-work CAS** over the
  current COMMIT-then-mark TOCTOU. *(recommend)*
- **(b) Auto-flip with the S8 fleet** — **rejected**: erasure has no cleanup; a wrong erasure post-flip
  is unrecoverable — the flip must be human-gated.
- **(c) Rely on the pg-boss global singletonKey for cross-stack single-flight** — **rejected**: the
  singletonKey is per-stack; the shared-table row lock (`FOR UPDATE SKIP LOCKED`) is the database-global
  guard.

**R3 recommendation:** (a). **🔴** — the reddest cutover (irreversible, no cleanup, alongside S5); the
human go/no-go and the "prove-before-flip" inversion are non-negotiable. Owner: architect + operator +
S8 lead + breaker (attack the cross-stack claim + the producer/consumer split).

### Q7 🔴 [MIGRATION] Draft-088 + MIG-2 sequencing
- **088 (`gdpr_erase_customer` DEFINER)** — land **before the S9 flip** (visibility-independent, works
  pre/post MIG-2); assumes `customers.avatar_key` exists (Phase-0 `ci-schema-drift` confirm, else drop
  it from the fn's `RETURNS TABLE`). `packages/db/migrations/` red-line, operator-placed.
- **MIG-2 (NOBYPASSRLS anon-policy scoping)** — the flip that makes N1 *matter*; N1 is a **flip+MIG-2
  latent** (RESOLVE-R2 §0). The DEFINER 088 must be live before MIG-2 reaches the S9 worker's env; the
  #61 backstop makes the interim safe (no-effect → `failed`, never false `completed`).
- **(a) S9 code builds independent; land 088 before the flip; MIG-2/NOBYPASSRLS timing inherited from
  the B3-council; surface the 088-before-MIG-2 sequencing as an operator gate.** *(recommend)*
- **(b) Block S9 on MIG-2 landing first** — **rejected**: over-couples; the code is dark-verifiable
  first, the P-proof needs a NOBYPASSRLS+MIG-2 rehearsal DB before the flip, not before the build.

**R3 recommendation:** (a). **🔴** on the **088-before-MIG-2 sequencing** (else a post-MIG-2 worker
without 088 fails-loud on every erasure — safe but broken). S9 authors/applies no migration. Owner:
operator (088 placement + MIG-2 sequencing) + S9 lead + B3-council.

---

## Decision-ordering note for the council
**Q1 (erasure completeness + N1)**, **Q2 (IDOR + masking)**, and **Q4 (DEFINER)** are **port-blocking**
— no S9 erasure write builds before all three are settled, because they define correctness (the erasure
actually runs + reaches every carrier), isolation (never the wrong tenant), and the privilege boundary
(the DEFINER pin). Decide them first. **Q1's GAP-A (orders fan-out)** is the single sharpest decision —
it changes the erasure's shape (customer-only → subject-graph).

**Q3 (irreversibility)**, **Q5 (retention basis)**, **Q6 (cutover human-gate)**, and **Q7 (088/MIG-2
sequencing)** are **cutover/policy-blocking, not build-blocking** — the Rust code can be built +
dark-verified before they settle, but the **flip** cannot happen until the N1 data-level P-proof + the
carrier-completeness proofs are green under NOBYPASSRLS, 088 has landed, and the operator signs the
**separate human go/no-go** (alongside S5). Q5 is a policy decision that can lag the build but must
precede a production retention change.

**The single most likely breaker escalation:** **GAP-A (Q1a1)** — the GDPR erasure never fans out to
the subject's orders, so the just-landed `delivery_photo_key` purge (#74) is never reached and the
home address + doorway photo survive an Art.17 request for up to a year. A silent, structural
under-erasure on the reddest surface.

**The single most likely counsel flag:** the **retention legal basis (Q5)** + the **restore-resurrection
window (Q3)** — a 7-year retention with no captured basis, and a pre-erasure encrypted backup that can
resurrect an erased subject, are both defensible **only** as explicit, owned, documented positions
(controller policy + a re-erase-on-restore runbook), never by silence.
