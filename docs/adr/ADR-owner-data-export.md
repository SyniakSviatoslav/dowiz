# ADR — Owner Data Export to AI ("bring your own agent")

- **Status:** 🟡 **DRAFT — Triadic Council CONVERGED; 🔴 ETHICAL-STOP on PII export.** Aggregate (no-PII) export SHIPPED. No production code for the PII/automated paths; NO migrations.
- **Date:** 2026-06-30
- **Red-line:** 🔴 RAW PII · 🔴 PII EGRESS · 🔴 RLS (tenant isolation / non-human principal) · 🔴 SUBPROCESSOR/DPIA · 🔴 HONEST-UI (no dark pattern)
- **Brief:** `docs/design/owner-data-export-ai-council-brief.md`
- **Council seats:** system-architect · system-breaker · counsel (all returned; this ADR synthesizes them)
- **Bound by / extends:** `docs/adr/0004-owner-token-revocation.md` (24h access, **no long-lived bearer**, `status='active'` on owner-writes — note: the brief's `ADR-0004-…` path was wrong), `docs/adr/0013-courier-realtime-authz.md` (tenant-scoped client + `BEGIN…set_config('app.current_tenant')…COMMIT` under FORCE RLS), `ADR-pg-privilege-hardening.md` (NOBYPASSRLS `deliveryos_operational_user`), `ADR-p0-privacy-hardening.md`, `0001-queue-in-postgres.md` (connection budget).

## Decision (council-converged)

1. **CONDUIT, not custodian — RATIFIED.** dowiz is a pipe the owner points at *their own* destination/model; dowiz never holds a third-party AI credential. Custodian rejected (new secret class + new subprocessor + we'd call an AI on PII the customer never consented to). *Counsel dissent on record:* custodian is the only model where dowiz could enforce redaction/erasure post-egress — CONDUIT trades customer-protection-after-egress for platform-liability-reduction; the ADR names this rather than pretending CONDUIT is strictly safer.

2. **🔴 ETHICAL-STOP (friction, human-releasable) on ALL PII-bearing export — Tier 0 included.** The live checkout notice (`apps/web/src/pages/client/CheckoutPage.tsx:~1173`) promises customers: *"We never sell or share your information with third parties or advertisers — it's used only to fulfil your order."* The published policy (`compliance/policies/privacy-policy-v1.md`) is a **stub**, so that checkout sentence is the operative representation. Any PII export to an owner's AI vendor makes that live sentence **false** — a dark-pattern red-line breach, not merely the owner's compliance gap. STOP holds until the representation and the behaviour agree.

3. **Tier 0 is NOT a clean slate — it is a live, un-gated PII leak to RETROFIT.** The existing client `exportCSV` already ships PII today: `CRMPage.tsx:180` exports `phone` (a plain key the `_`-prefix strip does not catch); Couriers/Dashboard export name/phone raw. Client-side filtering is **structurally incapable** of "export without phone." Fix = a **server export endpoint** whose **SELECT column projection** is the redaction boundary (the "no-phone" profile never selects the column), default-redacted, PII only behind explicit attested opt-in. Provable by test on the response body (`not.toContain(phone)`), red→green.

4. **Tier 1 webhook (push) — RATIFIED council-gated, default-OFF.** Authenticates by **outbound HMAC-SHA256** over the body with a per-tenant rotatable secret — a **message-auth key, not an inbound bearer** → it grants zero read capability and therefore does **not** reintroduce the long-lived bearer ADR-0004 eliminated. Config CRUD reuses the owner session (P-d `status='active'` evicts a removed owner). Export worker runs on the **NOBYPASSRLS operational role** with `app.current_tenant` GUC inside `BEGIN…COMMIT` under FORCE RLS — **no new RLS principal, no BYPASSRLS, B3/B4-safe**. Outbox decouples enqueue from delivery; per-endpoint timeout + bounded retry + circuit breaker so a dead owner endpoint can't back-pressure order traffic. Reuse the pg-boss pool (+0 connections); dedicated export lane only behind a staging load-test scaling-gate.

5. **Tier 2 MCP / direct-key — DEFER (ratified).** It is the *only* path that reintroduces a long-lived **inbound** bearer (in a third-party desktop app, surviving offboarding) and it mismatches the persona. Promotion trigger = a real power-user/contractual demand.

6. **Each tier is a SEPARATE ethical decision, not sequencing.** Tier-0-redacted can satisfy the Charter; the ADR does **not** pre-authorize the Tier-1/Tier-2 capability ratchet (occasional export → continuous PII stream into third-party models). Each tier reconvenes.

## Blockers from the Breaker (must be closed before any PII/Tier-1 flag flip)

- **[CRITICAL] Cross-tenant leak substrate.** The repo's only batch-over-PII precedent — `apps/api/src/lib/anonymizer/index.ts` on the shared worker pool (`bootstrap/workers.ts:91`) — sets **no** `app.current_tenant` and scopes by a `location_id IS NULL OR …` WHERE clause; a NULL/bug = all-tenant scan. An export worker built the idiomatic way would inherit this. **Mandatory:** GUC + FORCE RLS isolation (Decision 4), guardrail red→green that the export read returns zero other-tenant rows under a NOBYPASSRLS test role (and RED without `set_config` / without `BEGIN`).
- **[HIGH] Erasure cannot propagate → BLOCKER the moment Tier 1 ships.** `CUSTOMER_ANONYMIZED` / `ORDER_ANONYMIZED` bus events have **zero subscribers**; a customer erased after export is already downstream and unrecallable. For Tier 0 (owner is controller of their copy) = DPIA-documented residual; for Tier 1 (dowiz performed the egress) = an accepted-but-unhonourable Right → blocker. Only mitigation: minimize-what-leaves (default-redact) + explicit DPIA hole with a named owner.
- **[HIGH] Prompt-injection via mandatory `delivery_instructions`.** Required customer free-text (`CheckoutPage.tsx:460`) flows verbatim into the owner's agent; we can flag/quarantine, not prevent. Must be named in the DPIA + owner-facing warning.
- **[HIGH] Credential survival past offboarding.** No machine-principal eviction exists; ADR-0004 is human-session-only. Webhook secret in a config row → instant server-side disable/rotate + drain queued outbound jobs for that `location_id` on disable.
- **[MEDIUM] Cadence vs pool starvation** (3-conn worker pool shared with dispatch/anonymize) and **[MEDIUM] audit recursion** (per-export audit row must record `row_count`/`redaction_profile`/`attestation_id` but **no PII values**, RLS-FORCE'd, retained, and itself reachable by erasure).

## NEEDS-HUMAN (legal — hard pre-reqs; do not close silently)

- There is **no published privacy policy** (stub) and **no storefront policy generator in code** (grep = zero). Before any PII/Tier-1 flag flip, legal must author: (a) real policy text, (b) owner **attestation** copy ("I am the controller; my AI vendor is my subprocessor; I have a lawful basis"), (c) `compliance/subprocessors.md` + `data-map.md` + **DPIA** entries (incl. the erasure-hole + prompt-injection residuals), and (d) reconcile/replace the live checkout "never share" sentence.
- **Field-level redaction = required default, not owner-opt-in** (Architect + Counsel recommend; final call legal).
- **Give the customer a "no"** — document why a customer cannot opt their PII out of AI export, or make default-redact moot the question.

## What shipped clean (outside the STOP)

Aggregate **Analytics** JSON/JSONL export (stat cards, top products, consumption) — no PII, explicitly cleared by all three seats. `exportJSON`/`exportJSONL` added to `apps/web/src/lib/exportCSV.ts` (preserving the `_`-prefix strip); JSON button beside CSV on the three Analytics surfaces; e2e download-proof in `flow-ui-analytics-supplies.spec.ts`. (Landed in working tree; see provenance note — committed via a concurrent session's commit `89d1652f`.)

## Scope guard

This ADR ratifies design only. NO PII-export code, NO webhook, NO migrations, NO flag flips until the NEEDS-HUMAN legal pre-reqs land and the [CRITICAL]/[HIGH] guardrails are red→green. Next executable step is **none** without human/legal sign-off — by design.
