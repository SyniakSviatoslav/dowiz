# AI Governance Policy

DeliveryOS uses AI exclusively for administrative productivity features (Menu Parsing, Menu Translation). To protect user trust and comply with regional privacy regulations, we enforce strict governance over all AI integrations.

## 1. Zero-PII Policy
No Personally Identifiable Information (PII) may ever be transmitted to any LLM or translation service. 
- **Tooling:** All textual data must pass through `PiiRedactor` (`lib/pii-redactor.ts`) before transmission.
- **Redaction Patterns:** Emails, Phone numbers, Credit Cards, IBANs, and URLs are aggressively matched and replaced with `[REDACTED]`.
- **Audit:** Any redaction triggers a `POTENTIALLY_UNSAFE_VALUE` ParseIssue, maintaining visibility for the restaurant owner.

## 2. Self-Host First (and Zero-Cost by Default)
We prioritize self-hosted, CPU-friendly models in the development and default staging environments to ensure complete data residency. **No menu-parsing path depends on a paid API** — the system runs end-to-end with zero keys configured.
- **OCR:** `tesseract.js` running locally inside the Node.js process (WASM). Optional `paddle` engine for higher accuracy (self-hosted subprocess) — see `docs/menu-ocr-engines.md`.
- **LLM (menu structuring):** **the default is the in-process heuristic structurer** (pure code, no network, no key) — selected automatically when no LLM provider is configured. A real LLM is *optional* and only used when explicitly wired: `ollama` (self-host, e.g. `llama3.1:8b-instruct`) or a BYO-key provider (`GROQ_API_KEY` / `OPENAI_API_KEY`). There is **no Anthropic/Claude vision path** — an earlier Claude-vision parser was removed; the canonical seam is OCR-text → (heuristic | optional LLM).
- **Translation:** `libretranslate`.
- **Training Restrictions:** `OLLAMA_NOPULL=1` and `LT_NO_LEARN=true` are enforced to prevent models from learning on our data.

## 3. Human-in-the-Loop (Guardrails)
No AI-generated draft is written directly to the database without explicit user confirmation.
- **Preview First:** All AI operations must first generate a preview (`ParseResult`).
- **Low Confidence Flagging:** The system dynamically assigns a confidence score. If `confidence < 0.6`, a `LOW_CONFIDENCE` warning is raised.
- **Force Commit:** Committing a low-confidence draft requires an explicit `force=true` query parameter, representing the user clicking "Commit anyway".
- **Provenance Logging:** The `import_sessions` draft explicitly records `_provenance` (engine used, inference duration, raw text hash, and if forced).

## 4. Vendor Swap Capability
All AI calls must go through Dependency Injected Interfaces (`MenuParserProvider`, `TranslationProvider`). There shall be no `if (provider === 'openai')` scattered in the business logic. Changing a model vendor requires only environment variable changes (`LLM_PROVIDER`, `OCR_PROVIDER`, `TRANSLATION_PROVIDER`).

## 5. Degradation & Rate Limiting
- **Translations:** If a provider fails 3 times consecutively, it enters a degraded state returning the original strings, avoiding application crashes.
- **Timeouts:** Hard timeouts are strictly enforced on LLM calls to prevent hanging worker processes.
- **Rate Limit:** Automated translations are limited to 1 call per minute per location.
