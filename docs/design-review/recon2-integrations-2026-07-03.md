# Recon #2 — Integrations · AI/LLM · Crypto-payments (READ-ONLY)

**Date:** 2026-07-03 · **Scope:** menu-import (AI/LLM), voice engine, Plisio crypto, Telegram, R2/storage, outbound fetch.
**Method:** static read of source; no code executed, no edits. Ranked by severity.
**Out of scope (already covered by recon #1, NOT re-reported):** crypto refund black hole, telegram-webhook secret-in-URL, brand-extractor SSRF DNS-rebind, menu-import file DoS.

Legend: 🔴 HIGH · 🟠 MEDIUM · 🟡 LOW/INFO · ✅ verified-good. "dark" = behind a default-off flag; "live" = reachable in prod today.

---

## 🔴 H1 — Crypto webhook flips `paid` on Plisio's word alone; no amount/currency reconciliation (and the comment lies about it)
**File:** `apps/api/src/routes/payments-webhook.ts:57-70` · `apps/api/src/lib/payments/plisio.ts:88-94`
**Status:** dark (`PAYMENTS_CRYPTO_ENABLED`), but latent money bug before launch.

On a `completed` event the route sets `payments.status='paid'`, `captured_amount_minor=amount_minor`, and `orders.payment_status='paid'` **without ever comparing the webhook's `source_amount`/`source_currency` against the charged `payments.amount_minor`/`currency_code`.** `event.amountMinor` is only *stored* (line 52), never *validated*. The whole under/over-payment defense is delegated to Plisio's own `status=mismatch` classification (`plisio.ts:28`), which is provider-side and never independently checked. Plisio's comment (`plisio.ts:89-90`) explicitly claims "the route validates against payments.amount_minor" — **that validation does not exist anywhere**, creating false confidence in a money path.
**Scenario:** a Plisio classification bug, a partial-confirmation edge, or a leaked `PLISIO_SECRET_KEY` (needed to sign) that emits `status=completed` with a `source_amount` below the charge → order marked paid + released to fulfillment for less than owed. Currency mismatch (paid in a different coin/fiat than charged) is likewise never rejected.
**Fix:** in the `completed` branch, gate the status flip on `event.amountMinor >= payments.amount_minor AND event.currencyCode = payments.currency_code`; on shortfall route to `mismatch`/owner-review instead of `paid`, and delete the misleading comment.

## 🔴 H2 — Forged Telegram update → **live** owner order actions (confirm/reject) via non-secret chat-id identity + backward-compat header bypass
**File:** `apps/api/src/routes/telegram-webhook.ts:48-61` (auth), `:98-267` (authz), `:275-311/405-439` (order.confirm/reject)
**Status:** **live in prod** — order.* actions are NOT flag-gated (only `store.*`/`pref.*` are, `:184-185`).

The only request authenticity is the bot secret embedded in the URL path (`/webhook/telegram/${telegramBotSecret}`). The defense-in-depth header check is **disabled by a backward-compat branch**: a *wrong* `x-telegram-bot-api-secret-token` → 401, but a *missing* one → "process the request anyway" (`:57-60`). Owner identity is then bound solely to `callback_query.from.id` → `owner_notification_targets.address` (chat id). **Telegram chat ids are not secrets** and are attacker-supplied in a forged body.
**Scenario:** anyone who learns the webhook URL secret (it is in logs / setWebhook config / referer per recon #1) POSTs a synthetic `callback_query` with a victim owner's chat id and `data="order.confirm:<orderId>"` or `order.reject_reason_1:<orderId>` → the membership check passes (it is the *real* owner's row) → the attacker confirms or rejects that restaurant's real orders. `order.*` has no nonce (unlike `store.confirm`), and status guards only stop double-apply, not the initial forged action.
**Fix:** make the secret-token header **mandatory** when configured (drop the missing-header branch); treat the URL path as an endpoint, not an auth factor; consider a short-lived per-callback nonce for state-changing order actions.

---

## 🟠 M1 — Plisio `parseEvent` hardcodes `minorUnit=2` → ledger amount 100× wrong for the platform's primary currency
**File:** `apps/api/src/lib/payments/plisio.ts:91-92`
`amountStringToMinor(String(payload['source_amount']), 2)` is hardcoded to 2 decimals. The platform default currency is **ALL / `currency_minor_unit=0`** (`migrations/1780338982014_location_commerce.ts:6-7`). So `"800"` is recorded as `80000` minor units in `payment_events.amount_minor` — 100× inflated for the main market (and 10× for 3-decimal currencies like KWD/BHD). It does not break the status flip (that uses `payments.amount_minor`), but it corrupts the append-only money ledger used for reconciliation/audit, and it would silently defeat any future amount-check (H1) built on the event amount.
**Fix:** thread `payments.currency_minor_unit` (or the location's) through `parseEvent` instead of the literal `2`.

## 🟠 M2 — Unauthenticated LLM cost + OCR CPU abuse (`/menu/import/anonymous` and `/preview`)
**File:** `apps/api/src/routes/owner/menu-import.ts:173-215` (public anon) · `apps/api/src/lib/ai-ocr-parser.ts:456,539-582` (unbounded prompt, no `max_tokens`)
`/menu/import/anonymous` is **public** (no account). It runs heavy OCR (Tesseract/PaddleOCR, 120 s subprocess) on a 10 MB image, then sends the **entire, un-truncated** redacted OCR/PDF text into the LLM prompt (`...${redactedText}`) with **no length cap and no `max_tokens`** on any provider call, on the *operator's* OpenRouter/OpenAI key. The only bound is `rateLimit {max:5, timeWindow:'1 minute'}` keyed per-IP — trivially rotated. There is no per-tenant token budget.
**Scenario:** attacker rotates IPs uploading dense 10 MB menus → sustained OCR CPU exhaustion + unbounded LLM token spend (financial DoS) against the shared API key. The 24 h response cache (`llmCache`) does not help — each novel body misses.
**Fix:** cap `redactedText` length before the prompt (e.g. truncate to N KB), set a provider `max_tokens`, and add a per-day anonymous-import quota independent of IP.

## 🟠 M3 — Allergen hallucination stored to `products.attributes`; grounding check ignores allergens and is dark
**File:** `apps/api/src/lib/ai-ocr-parser.ts:569` (prompt), `:53` (`attributesJson: z.record(z.unknown())`), commit `menu-import.ts:337-354` · grounding `:630-650`
The prompt instructs the model to **infer allergens from ingredient names** ("allergens: dairy, eggs, nuts, soy, gluten, shellfish, fish"). Zod accepts `attributesJson` as opaque `z.record(z.unknown())` — no validation of the allergen content — and it lands in `products.attributes`, the single source that drives dietary filtering (per the allergen-safety memory ADR-0014). The only hallucination guard (`computeGrounding`) checks **price only** and is **dark by default** (`MENU_GROUNDING_ENABLED`). A model that omits "nuts" from a nut dish (or invents "gluten-free") produces a safety-relevant error with no automated flag; the commit `force` gate keys off OCR confidence, not allergen provenance.
**Fix:** never let LLM-inferred allergens auto-populate the safety field — either drop allergens from the extraction, or mark them unverified/require explicit owner attestation per item before they can filter.

## 🟠 M4 — Fabricated prices commit with no warning and no `force` when OCR confidence is high
**File:** `apps/api/src/lib/ai-ocr-parser.ts:568` (prompt), `:624-628` (low-confidence = OCR-derived) · commit gate `menu-import.ts:296-299`
The prompt says: *"If a price appears in the text but is unclear, provide your best estimate. NEVER omit price."* → the model is instructed to fabricate. With grounding dark (M3), `low_confidence_count` is derived from **OCR confidence**, not price grounding. So on a clean scan (OCR ≥ 0.6) a hallucinated price yields `low_confidence_count=0`, the commit needs no `force`, and no `PRICE_NOT_GROUNDED` warning is emitted. The owner reviews, but nothing draws attention to the invented number → wrong menu prices ship silently.
**Fix:** enable grounding by default (or hard-block commit on ungrounded prices), and stop instructing the model to guess prices.

## 🟠 M5 — Public `/media/*` + `/images/*` serve private customer entry-photos by unguessable key, 1-year immutable public cache, zero per-object authz
**File:** `apps/api/src/routes/spa-proxy.ts:158-210` (serve), `:263-293` (`entry-photos/<uuid>.webp` upload)
Entry-anchor photos are **photos of the customer's home/door/building** (a privacy-sensitive artefact — cf. the P0-privacy memory). They are stored in the same bucket and served through the **unauthenticated** `/media/*` endpoint, protected only by an unguessable UUID key and served with `Cache-Control: public, max-age=31536000, immutable`. There is no check that the requester is the assigned courier or even authenticated. Any leak of the key (courier device, browser history, proxy/CDN logs, screenshot) makes the photo **permanently, publicly** cacheable. The traversal guard is present and correct; the gap is authorization, not path safety.
**Fix:** serve `entry-photos/*` (and import blobs) through an authenticated, order-scoped route (courier-of-record only), short/no public cache; keep only truly-public menu media on the open path.

---

## 🟡 L1 — Raw uploaded menu retrievable by key and never deleted
`menu-import.ts:111-112` stores the raw upload as `import_<uuid>.csv` (regardless of real type) via `storage.put`; it is a valid `/images/*` key (no `..`) so `GET /images/import_<uuid>.csv` returns the original file, and nothing ever deletes it → orphaned objects + retrievable-by-key (unguessable UUID mitigates). Add cleanup + keep import blobs off the public serve path.

## 🟡 L2 — RAG "memories" injected verbatim into the extraction prompt (2nd-order injection)
`ai-ocr-parser.ts:482-527` splices retrieved memory snippets into the LLM prompt. If the memory store is ever shared across tenants or fed from prior (attacker-influenced) imports, this is a stored-prompt-injection channel. `memoryService` defaults to `null` (`:298`) so it is inert in prod today; gate/scope memories per-tenant before enabling.

## 🟡 L3 — Prompt injection into menu-import is bounded but non-zero
Untrusted menu text is appended to the prompt (`:573`), but output is Zod-validated (`:601`) → injection can only yield schema-conformant menu data, written to the uploader's **own** tenant, into a draft the owner reviews (`import_sessions`, force gate). Worst case in `merge` mode: `ON CONFLICT (location_id, external_key) DO UPDATE` could overwrite the owner's existing products if injected `externalKey`s collide (LLM emits `prod1…` so collision is unlikely). No SQL/exec reachable. `attributesJson` is unbounded jsonb (bloat vector). Low.

## 🟡 L4 — Telegram `/start login_<token>` binding race
`telegram-webhook.ts:526-555`: a forged update (needs the URL secret + a live pending `telegram_login_tokens` UUID) could authenticate a victim's pending web-login session against the **attacker's** `telegram_user_id`. Requires two secrets; narrow window. Note for the auth model, not urgent.

---

## ✅ V1 — Voice engine "read-only" claim holds
`packages/voice/*` + `apps/api/src/routes/public/voice-config.ts`. The engine emits `IntentProposal` **data** only; `ConfirmationGate` is the sole sink and lives in `apps/web` (handlers never cross the engine boundary — `confirmation-gate.ts:5-21`). `capability-table.ts` is a fail-closed exhaustive `Record<IntentKind,…>` (unknown/money/dietary → `REJECT`); the only `STATEFUL` intent (`ADD_TO_CART`) is held pending an explicit human confirm; dietary category selection is force-downgraded to REJECT (`:47-61`). `voice-config` is a `no-store`, no-DB, no-auth global boolean kill-switch. **No server-side voice write endpoint exists.** Zero write capability confirmed.

## ✅ V2 — Crypto webhook idempotency + monotonic status are sound
`payments-webhook.ts:45-85`: insert-wins on `(provider, provider_payment_id, type)` + `rowCount===1` guard kills same-status replays; transitions are monotonic (`status NOT IN ('refunded','paid')`, `status='pending'`) so a replayed/late event cannot regress `paid→pending` or double-credit. The flag-gate is genuinely dark: route 404s unless `PAYMENTS_CRYPTO_ENABLED` + provider resolves to a keyed `plisio` adapter (`registry.ts:12-23`). Charge-creation amount is server-authoritative (`orders.ts:646-660`). The gap is purely the missing *amount reconciliation* on ingest (H1/M1), not idempotency.

## ✅ V3 — R2 presign/confirm tenant-scoping is correct
`product-media.ts:133-171,208-233`: presigned PUT keys are server-built from membership `locId` (`${locId}/${productId}/…`), confirm rejects any `storageKey` outside `${locId}/${productId}/`, plus mime allow-list, per-file size ceiling, 150 MB/location budget, and magic-byte re-sniff. Client cannot sign into another tenant's prefix. (Read-path authz is the separate M5 issue.)
