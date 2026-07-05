# P6 — Operator decisions on the Council verdict (recorded ETHICAL-STOP resolution)

**Date:** 2026-06-27 · Resolves the gate in `p6-provisioning-council-verdict.md`. The operator is the final
authority; the ETHICAL-STOP is friction requiring a recorded decision — this is that record.

| # | Question | Decision | Notes / residual risk accepted |
|---|----------|----------|--------------------------------|
| 1 | Public unconsented shadow store vs private/consented preview | **PUBLIC** | Operator overrides counsel's reshape. Accepts the residual risk counsel flagged (passing-off / one-owner C&D / "means contradict ends"). **Harm-reduction guards that STILL apply** (non-negotiable, they cap the blast radius): honest "**preview mockup built from your public site — not a live store**" labeling everywhere it renders · **instant owner kill/delete**, no dark-pattern · noindex **+ sitemap exclusion** (architect) · never-orderable (decision 3) · provenance own-site/GBP-Places-API-only, never Maps-scrape · conservative/removable trademark+logo · honor robots.txt. |
| 2 | Pre-claim allergen inference | **AGREE — do NOT infer allergens pre-claim** | No allergen data generated until an authenticated owner confirms each field post-claim. `allergens_confirmed=false` remains for any later owner-entered data. |
| 3 | Add a real `status` reject in `POST /orders` | **APPROVE** | Designs out the breaker's HARD BLOCKER. This touches a 🔴 untested-hotspot (order route) → its own red→green guardrail + the closed-tenant-rejects-public-order test before it's "done". |
| 4 | Facts-only extraction vs full | **EXTRACT EVERYTHING; owner manually approves the autofilled data at claim** | Rejects facts-only. The **owner-manual-approval-at-claim** is the accuracy + authority safeguard: nothing the pipeline autofills (descriptions, etc.) is authoritative/live until a human owner reviews + approves it post-claim. **Carve-out that remains BINDING:** zero-PII-in-AI (ADR-0011) is NOT waived by "extract everything" — PII redaction + a free-text/name guard still run **before the AI boundary** (extracting the full menu ≠ sending owner phone/staff names to a 3rd-party model). `place_raw` minimized + hard-delete-on-request (unconsented third-party data). **If the operator intends to also waive PII-redaction-before-AI, that is a SEPARATE binding-invariant override to confirm at P6-3** — not assumed here. |

## Decision 1b — shadow write-authority (resolves the ФАЗА-A UNCERTAIN)
**Mint a one-time provisioning token so the shadow write goes THROUGH RLS, not around it.** Reject the
BYPASSRLS-pool and the blanket-SECURITY-DEFINER options. Design (P6-2): a single-use, short-TTL provisioning
grant that sets an RLS context (`SET LOCAL app.provision_token = <one-time>`) which a NEW, narrow provisioning
RLS policy on the shadow-writable tables (`organizations`/`locations`/`products`) honors **only** for
`owner_id IS NULL` + `status='closed'` shadow rows — so RLS stays ENFORCED and provisioning is an explicit,
auditable, single-use carve-out (not a standing bypass). The token is minted per-acquisition, consumed once,
never reused. 🔴 RLS red-line → its own migration + red→green RLS test (provisioning policy admits the shadow
write; a non-token write under the same role is still rejected). This is a **P6-2** concern; P6-1 is unaffected.

**Net:** PUBLIC shadow store, never-orderable (status reject), no pre-claim allergens, full extraction
gated by **owner-manual-approval-at-claim**, PII-redaction-before-AI stays binding, honest-labeled +
instant-kill + noindex/sitemap-excluded. Provenance: own-site / official Places API within ToS only.
→ Proceed to ФАЗА A (recon), then STOP for GO on P6-1.

---

## P6-3 decisions (2026-06-28 · resolves the gate in `p6-3-extraction-render-council-verdict.md`)

| # | Question | Decision | Notes / residual risk accepted |
|---|----------|----------|--------------------------------|
| D-render | Public pre-claim render scope: names+prices-only vs full verbatim descriptions | **FULL DESCRIPTIONS** | Operator **overrides counsel's ETHICAL-STOP + the binding "no verbatim creative copy" guard.** Accepts the residual risk: passing-off / copyright C&D on scraped descriptions + the GDPR Art-14 incidental-PII-in-descriptions surface on a page the restaurant never saw. **Carve-outs that REMAIN BINDING (not waived by this):** (1) **PII-redaction-before-AI (C1)** — the menu-region allowlist + name-guard + redaction-recall fixture still run before the AI boundary; descriptions are rendered from AI output fed *redacted* input. (2) **allergen no-inference (C2 / decision #2)** — write-strip + read-gate, server-authoritative. (3) descriptions stay **non-authoritative until owner approval at claim** (decision #4) — rendered as a labeled preview, not as the restaurant's live word. (4) Art-14 notice attaches to the **claim-invite outreach**. (5) still `noindex` + sitemap-excluded + never-orderable + instant-kill + **generic OG** (H3 — real name never in unfurl metadata). No scraped photos. |
| D-public | Re-confirm PUBLIC vs switch to a private authenticated sales-preview | **KEEP PUBLIC** | Re-confirmed knowingly after counsel's "question nobody asked" (given noindex+sitemap-exclusion, public buys ~nothing over private while carrying most of the liability). Operator is final. |

**Net (P6-3):** PUBLIC + full verbatim descriptions rendered pre-claim (recorded override of no-verbatim-copy),
behind the full guard-stack — C1 PII-before-AI, C2 allergen no-inference, never-orderable (`published_at` NULL),
generic OG, noindex/sitemap-excluded, instant-kill, owner-approval-at-claim, Art-14 notice at claim-invite. The
2 CRITICAL + 4 HIGH **technical** conditions in the verdict are build prerequisites (not operator-waivable).
