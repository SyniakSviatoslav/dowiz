# Ethical Decisions — `error-contract-parse-token-economy`

Records of human decisions on grounded ETHICAL-STOPs raised by Counsel. Counsel is
friction, not veto; a conscious human decision is authoritative and recorded here.

---

## ETHICAL-STOP-1 · zero-PII-in-AI — OCR text → LLM prompt

**Raised by:** Counsel (counsel-opinion.md, STOP-1), grounded in live code.

**Red line:** §"zero-PII-in-AI" / GDPR data-minimisation. The menu-parse path concatenates
**raw OCR text** into the LLM prompt (`apps/api/src/lib/ai-ocr-parser.ts:515` uses `rawText`;
the redacted copy at `:399` is used only for the provenance hash). A real menu **photo** can
contain incidental **third-party PII** (staff name/phone, handwritten note, a face/contact)
that the owner's consent does **not** cover, which would egress to an external model
(OpenRouter / OpenCode Zen).

**Options presented to human:**
1. Redact-by-default (privacy-first) — feed `redactedText` into the prompt; if onboarding
   pre-fill regresses, add a *separate consented* venue-contact extraction path.
2. Reclassify venue contact as business data — send raw OCR; keeps pre-fill but still egresses
   incidental third-party PII (does NOT clear the red line).
3. Defer Area B entirely; ship only A + C now.

**HUMAN DECISION:** **Option 1 — Redact-by-default.**

**Rationale (human):** Incidental third-party PII must not reach an external model; the
owner's consent covers only their own data. Privacy-first by default; the venue's own contact
is recovered, if needed, through a separate consented path — never by sending raw PII to the
model.

**Binding consequences for implementation (Area B / B3):**
- The LLM prompt input becomes `piiRedactor.redact(ocrText)` (i.e. `redactedText`, not
  `rawText`) at `ai-ocr-parser.ts:515`. Confirm `redactedText` is computed/populated before
  that point (Round-2 breaker to verify ordering vs `:399`).
- The "venue-own-contact = business-data-not-PII" reclassification is **NOT** adopted as a
  blanket exception. If onboarding pre-fill regresses because the redactor also strips the
  venue's own phone, the remedy is a **separate, consented** venue-contact extraction path —
  not reverting to raw PII into the model.
- Vision-review PII masking + OpenRouter subprocessor registration in `compliance-gate`
  (seed only) remain as specified in B3.

**Date:** 2026-06-26
**Owner:** Parse owner (implementation) · sviatoslavsyniak@gmail.com (decision)
**Status:** RESOLVED — design revised to redact-by-default; this gate is cleared.
