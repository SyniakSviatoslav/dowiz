# Council Brief — Owner Data Export to AI ("bring your own agent")

- **Status:** 🟡 **DRAFT — to convene Triadic Council** (design-time; NO production code, NO migrations)
- **Date:** 2026-06-30
- **Red-line:** 🔴 RAW PII (customer/courier names, phones, addresses) · 🔴 PII EGRESS (new outbound flow) · 🔴 RLS (tenant isolation / non-human principal) · 🔴 SUBPROCESSOR (compliance-gate / RoPA / DPIA) · 🔴 AUTH (new credential class if webhook/MCP)
- **Trigger:** owner-facing usability — let a restaurant owner send *their own* tenant data to *their own* AI agent (Claude / Codex / other) instead of manual CSV→PDF shuffling between tools.
- **Bound by / extends:** `ADR-p0-privacy-hardening.md` (PII minimization, active-delivery GPS guard), `ADR-0004-owner-token-revocation.md` (24h access + per-request `status='active'`), `ADR-soft-access-gate.md`, the B3 NOBYPASSRLS+GUC dependency (launch-blocker councils), `/compliance` SoT (data-map / RoPA / subprocessors / privacy-gate).

## Context

Today's export is **client-side only**: `apps/web/src/lib/exportCSV.ts` (16 lines) serializes
already-loaded page rows to a CSV download, used by 4 admin pages — Analytics (aggregate, no PII),
Dashboard (orders → customer PII), CRM (customers → name/phone/LTV), Couriers (name/phone). There is
**no server export endpoint**. A user-initiated download of your own tenant data is **GDPR data
portability** (a right we owe anyway), not a subprocessor egress — this is the safe anchor.

**2026 agentic context (validates the demand, not the risk):**
- Token economics favor **files/CLI over MCP** for repetitive reporting (~200 tok vs 32k–82k/op) —
  i.e. a JSON/CSV export is the *right* primitive for "owner's agent ingests data daily", not MCP.
- **Data portability / anti-lock-in** is a named 2026 enterprise buyer demand (Constellation; Celonis-v-SAP).
- The persona now *has somewhere to send it*: NRA *State of the Restaurant Industry 2026* (Feb) — **26%**
  of operators use AI tools; back-office agents (e.g. Loop "Samantha") ingest order/financial data via API.

Sources: firecrawl.dev/blog/agentic-ai-trends · bitmovin.com/blog/understanding-mcp-agentic-ai-data-access ·
constellationr.com (2026 enterprise trends) · restaurantdive.com (NRA 26%, Feb 2026).

## Already decided / out of scope for council

- **JSON/JSONL serialization util** (`exportJSON` / `exportJSONL` in `exportCSV.ts`) — BUILT, dark,
  no PII of its own; preserves the `_`-prefix internal-field strip so JSON can't leak fields CSV hides.
- **Analytics JSON export** (aggregate, no PII) — shippable WITHOUT council via the normal proof loop.
- This brief is **only** about exporting **PII-bearing** surfaces (orders / customers / couriers) and
  any **automated outbound** path (webhook / MCP). The post-edit red-line gate already blocks wiring
  JSON onto those pages — that block is the reason this brief exists.

## The central decision: CONDUIT vs CUSTODIAN

> Is dowiz a **pipe** the owner points at their own destination (owner brings key/endpoint, owner picks
> the model), or a **custodian** that holds owner credentials and calls the model on their behalf?

Recommended default to ratify/amend: **CONDUIT**, sequenced lowest-risk first.

1. **Tier 0 — owner-initiated file download (CSV + JSON/JSONL) of PII surfaces.** Owner clicks, gets a
   file. Their next action (feed to their agent) is theirs, not our pipe. Legally = portability.
   Gate concern = the file *contains* customer PII the customer never consented to share with the
   owner's AI vendor → requires **owner attestation** + **field-level redaction toggles** (export
   orders/revenue without phone) + an **audit row** per export.
2. **Tier 1 — outbound webhook (push).** Owner-configured endpoint + HMAC signature + rotatable secret
   (reuse ADR-0004 revocation / `status='active'`), runs in our job under proper tenant context
   (no foreign RLS principal), outbox so a slow endpoint can't back-pressure order traffic. The "no
   manual action" payoff. Council-gated.
3. **Tier 2 — per-tenant MCP / direct-key connector.** DEFER. Persona mismatch (owners don't configure
   MCP), long-lived credential in a third-party desktop app, new non-human RLS principal. Power-user
   only, later.

## What the council must attack (seat by seat)

**Architect** — the credential & isolation model: how does an outbound/automated path authenticate
without (a) creating a long-lived bearer that breaks the 24h/`status='active'` model, (b) introducing a
non-human RLS principal that can confuse tenant binding (B3/B4/BOLA class)? Where does redaction live
(server-side, provable by test) so "export without PII" is enforced, not cosmetic?

**Breaker** — demonstrate the failures (ranked):
- **Erasure can't propagate** past the send — a customer erased after export is already downstream. Is
  this an acceptable, DPIA-documented hole, or a blocker?
- **Prompt-injection via customer free-text** (order notes / names / reviews) flows into the owner's
  agent — we can flag/quarantine but not prevent. Harm to our owner; reputational to us.
- **Cross-tenant leak** if any future server export runs under a service role.
- **Audit recursion** — logging PII access generates more PII-access records.
- **Pull/push cadence vs operational pool** (prior pool-starvation incidents) during dinner rush.

**Counsel (ETHICAL-STOP candidates)** — the owner consents, but the *customer* did not agree their
phone/address goes to the owner's AI vendor. Does enabling this make us complicit in the owner's
compliance gap unless we force attestation + update the storefront privacy-policy generator? Charter:
"AI built on collective knowledge, never turned against the people it learned from" — does a redacted/
consented design satisfy it?

## Red lines to preserve (non-negotiable inputs)

- Integer money; RLS FORCE on any new table; no BYPASSRLS for an export path.
- No new PII class without `/compliance` RoPA + subprocessor registry entry + **DPIA**.
- No credential that survives owner offboarding / manager removal without instant server-side revocation
  + live-connection eviction (courier-WS C1 precedent).

## NEEDS-HUMAN

- Does the owner's (and our storefront-generated) privacy policy actually cover "data shared with the
  owner's AI vendor"? If not, attestation copy + policy-generator update is a hard pre-req.
- Is field-level redaction *required* for Tier 0, or owner-opt-in? (Counsel + legal.)

## Scope guard

NO production code, NO migrations, NO flag flips from this brief. Output = Triadic verdict → ADR
(`docs/adr/ADR-owner-data-export.md`) with the conduit/custodian decision, the credential model, the
redaction/consent/audit requirements, and the explicit DPIA hole list — *then* build behind a default-OFF
flag.
