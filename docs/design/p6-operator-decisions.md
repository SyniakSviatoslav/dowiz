# P6 — Operator decisions on the Council verdict (recorded ETHICAL-STOP resolution)

**Date:** 2026-06-27 · Resolves the gate in `p6-provisioning-council-verdict.md`. The operator is the final
authority; the ETHICAL-STOP is friction requiring a recorded decision — this is that record.

| # | Question | Decision | Notes / residual risk accepted |
|---|----------|----------|--------------------------------|
| 1 | Public unconsented shadow store vs private/consented preview | **PUBLIC** | Operator overrides counsel's reshape. Accepts the residual risk counsel flagged (passing-off / one-owner C&D / "means contradict ends"). **Harm-reduction guards that STILL apply** (non-negotiable, they cap the blast radius): honest "**preview mockup built from your public site — not a live store**" labeling everywhere it renders · **instant owner kill/delete**, no dark-pattern · noindex **+ sitemap exclusion** (architect) · never-orderable (decision 3) · provenance own-site/GBP-Places-API-only, never Maps-scrape · conservative/removable trademark+logo · honor robots.txt. |
| 2 | Pre-claim allergen inference | **AGREE — do NOT infer allergens pre-claim** | No allergen data generated until an authenticated owner confirms each field post-claim. `allergens_confirmed=false` remains for any later owner-entered data. |
| 3 | Add a real `status` reject in `POST /orders` | **APPROVE** | Designs out the breaker's HARD BLOCKER. This touches a 🔴 untested-hotspot (order route) → its own red→green guardrail + the closed-tenant-rejects-public-order test before it's "done". |
| 4 | Facts-only extraction vs full | **EXTRACT EVERYTHING; owner manually approves the autofilled data at claim** | Rejects facts-only. The **owner-manual-approval-at-claim** is the accuracy + authority safeguard: nothing the pipeline autofills (descriptions, etc.) is authoritative/live until a human owner reviews + approves it post-claim. **Carve-out that remains BINDING:** zero-PII-in-AI (ADR-0011) is NOT waived by "extract everything" — PII redaction + a free-text/name guard still run **before the AI boundary** (extracting the full menu ≠ sending owner phone/staff names to a 3rd-party model). `place_raw` minimized + hard-delete-on-request (unconsented third-party data). **If the operator intends to also waive PII-redaction-before-AI, that is a SEPARATE binding-invariant override to confirm at P6-3** — not assumed here. |

**Net:** PUBLIC shadow store, never-orderable (status reject), no pre-claim allergens, full extraction
gated by **owner-manual-approval-at-claim**, PII-redaction-before-AI stays binding, honest-labeled +
instant-kill + noindex/sitemap-excluded. Provenance: own-site / official Places API within ToS only.
→ Proceed to ФАЗА A (recon), then STOP for GO on P6-1.
