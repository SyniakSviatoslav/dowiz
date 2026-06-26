# ADR-0011 — Menu-parse eval harness + hallucination-grounding + injection-safe prompt (Area B)

- Status: **Proposed** (design-time)
- Date: 2026-06-26
- Deciders: DeliveryOS Triadic Council
- Relates: `apps/api/src/lib/ai-ocr-parser.ts`, `import_sessions`
  (`packages/db/migrations/1780338982025_import_sessions.ts`), compliance subprocessor register
  (`scripts/compliance-gate.ts:44`). **Explicitly NOT RAG** — no vector DB, reranking, multi-hop, chunking.

## Context

A real cascade parser exists (zen→groq→openai→openrouter→heuristic, with degradation,
`ai-ocr-parser.ts:80–104,517–523`), money is already integer minor units (`:756`), and import already
lands as a **draft** in `import_sessions` requiring explicit owner-publish (activation-gate). But model
swaps are **unguarded** (no regression detector), no field carries provenance/confidence, and OCR text
is concatenated straight into the prompt (`:515`) — an injection surface.

## Decision

1. **B1 — Parse-eval harness.** `tests/menu-parse/fixtures/*.{pdf,csv,jpg}` + `expected/*.json`,
   field-level scorer. **Price = EXACT integer-minor-unit match (zero tolerance)**; items = recall;
   modifier-groups = structure. Run deterministically. Thresholds explicit + version-controlled
   (item-recall ≥ 0.95, modifier-structure ≥ 0.90; fixture set grows on each real-world miss). **Blocks a
   measured cascade-swap regression on the committed set; does NOT replace human review** (B9 — 15
   self-authored fixtures, independent oracle is future work).
2. **B2 — Hallucination grounding (B7-corrected).** Every parsed field carries `{ value, confidence,
   grounded }` (inside `draft_json` jsonb — **no migration**). `grounded` compares the parsed minor-unit
   value against OCR price-tokens **via the same `priceOf` normalizer** (`ai-ocr-parser.ts:745–756`) —
   **not** a substring (a substring false-flags `"1.200 Lek"` and false-passes `price:1`). A price with no
   normalized OCR match → `grounded:false` → flagged. **Never auto-publish** — the existing draft +
   owner-publish gate stands. **The GROUNDING ENRICHMENT is GATED on B5/B12** (below); B3-privacy is not.
3. **B3 — Injection-safe + PII redaction (B8-honest; ETHICS gate CLOSED).** OCR text is **UNTRUSTED data,
   not instructions**: data-delimited block + system prompt (defense-in-depth). **Schema validation is NOT
   a backstop** — `safeParse` (`:536`) constrains shape, not truth (`{price:1}` passes); the real floor is
   `grounded` flags + the pre-existing **human review of the draft**. **ETHICS GATE CLOSED — redact-by-
   default is BINDING** (`ethical-decisions.md`): run the existing `piiRedactor.redact()` over OCR text
   **before** the prompt (`ai-ocr-parser.ts:515` currently uses raw `rawText`; the redacted copy at `:399`
   feeds only the hash). **This ships INDEPENDENT of the B5/B12 RLS gate** — it touches only prompt
   construction, is the **earliest-shipping part of B**, and is not held hostage by the `import_sessions`
   FORCE defer. Keep OpenRouter in the compliance subprocessor register.

## Consequences

- (+) Cascade swaps blocked from a measured regression on the committed set.
- (+) Hallucinated prices caught by normalizer-parity grounding, not silently published.
- (+) Privacy hardening (OCR redaction) ships first, independent of the RLS gate.
- (−) `{value,confidence,grounded}` enriches the draft payload (jsonb, bounded).
- (−) Honest: prompt-injection has **no hard automated backstop**; human review is the floor.
- **Money:** exact integer, zero tolerance.
- **GATE (B5/B12) — grounding only:** `import_sessions` is RLS **ENABLE-only** (`1780338982025:26`),
  **absent** from `1780421100051_force-rls.ts`; AND the `verify:rls` gate was a no-op (omitted the table,
  checked only ENABLE — B12). **B2-grounding MUST NOT ship** until the **strengthened** `verify:rls`
  (fails on missing FORCE; includes `import_sessions`) is green AND a forward-only FORCE migration has
  landed (separate red-line change, DB owner).
- **ETHICS gate (CLOSED):** redact-by-default is the binding posture; the venue-own-contact decision is
  recorded in `ethical-decisions.md`.

## Proof (STOP-DESIGN-B obligations)

Eval: price 100% exact on 15 fixtures, recall ≥ 0.95, structure ≥ 0.90, blocks a measured swap regression.
Grounding (B7): false-flag fixture (`1.200 Lek` → grounded) + false-pass fixture (`price:1` → flagged),
never auto-published. Injection fixture: directive-in-OCR → prices unchanged, **plus** a note the
schema-valid-but-wrong class is caught only by grounding + human review. **Redaction-recall fixture
(Counsel a):** seeded third-party PII (Albanian name / handwritten / non-Latin) does **not** reach the
prompt. B5/B12 gate (grounding only): strengthened `verify:rls` green + FORCE migration landed before
B2-grounding ships; **B3-redaction ships independently**.
