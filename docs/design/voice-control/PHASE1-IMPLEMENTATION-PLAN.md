# Voice Control — Phase-1 Implementation Plan (client storefront)

> Status: PLAN (design-first; **no product code in this document**). Date: 2026-07-03.
> Branch: `fix/audit-remediation`. Binding architecture: `docs/adr/0015-voice-control.md`.
> Companion specs: `docs/design/voice-control/proposal.md`, `.../ui-spec.md`, `.../resolution.md`,
> `.../breaker-findings.md`, `.../counsel-opinion.md`.
>
> **Read this first if you have not read the ADR.** The operator's ask is *"a voice agent on the
> client storefront that can make any action needed based on voice input; sq/en/uk from the start;
> best-practice design."* ADR-0015 is binding and it draws one hard line: **voice never finalizes an
> order or any money/payment/checkout-write; there is NO voice→money binding; stateful actions use
> confirm-then-execute (the real handler runs only on an explicit human tap); the ConfirmationGate is
> fail-closed (unknown/forgotten intent kind → REJECT).** "Any action" is therefore realized as *voice
> may PROPOSE or INITIATE any in-scope action, and money/order actions execute only after a visible
> confirmation tap.* Section 7 states exactly where "any action" cannot mean auto-execution and why
> confirm-then-execute satisfies the intent safely. Do not plan to bypass the gate. Anything that would
> auto-execute a money/order action off voice is **REQUIRES-COUNCIL** (Phase 3/4 territory, not approved).

---

## 1. Current-state inventory (what exists on this branch)

The **entire safety core, matcher, ASR runtime, and control plane are already built and unit-proven.**
Phase-1 is almost entirely *browser wiring + UI + strings + the remaining machine-checks* — not new
engine logic. Verified against the live tree 2026-07-03.

### 1.1 BUILT — `packages/voice/src/*` (the engine; imports no app store / API client)

| File | What it provides | Key symbols (file:line) |
|---|---|---|
| `types.ts` | The closed `IntentKind` union (8 kinds), `Capability`, immutable `IntentProposal`, `GateStatus`, `GateResult`. | `IntentKind` `types.ts:14-22`; `IntentProposal` `types.ts:32-38` |
| `capability-table.ts` | `classify(kind)` — fail-closed `Record<IntentKind, …>` table; unknown kind → `REJECT`; adding a kind without a row **fails the build** (exhaustiveness ratchet). | `CAPABILITY_TABLE` `capability-table.ts:9-18`; `classify` `:25-27` |
| `confirmation-gate.ts` | `ConfirmationGate` — the **sole write sink**: READ_ONLY auto-applies, STATEFUL is held `#pending` and applies **only** on `confirm()`, REJECT is dropped, dietary category downgraded to REJECT before any apply. `VoiceHandlers` port is owned by apps/web and injected here (engine never sees it). | `ConfirmationGate` `confirmation-gate.ts:30`; `submit` `:44-73`; `confirm` `:76-84`; `cancel` `:87-89`; `VoiceHandlers` `:12-21` |
| `dietary-denylist.ts` | `isDietaryCategory(name)` + single-source `DIETARY_TOKENS` (sq/en/uk). Over-matches on purpose (safe direction). | `DIETARY_TOKENS` `:10-53`; `isDietaryCategory` `:61-64` |
| `matcher.ts` | `matchIntent(transcript, locale, menu)` — deterministic closed-vocab matcher (NOT dictation). Per-locale `TRIGGERS` (sq/en/uk), number-word parsing, sort/macro detection, fuzzy product/category resolution. `MIN_CONFIDENCE = 0.6`. Emits **semantic** args (`{ by:'price' }`), never imports apps/web. | `matchIntent` `matcher.ts:174-235`; `TRIGGERS` `:25-56`; `MenuContext` `:12-15`; `MIN_CONFIDENCE` `:22` |
| `normalize.ts` | NFD diacritic-strip + lowercase + whitespace-collapse, shared by matcher + denylist. | `normalize` `:6-13` |
| `mock-provider.ts` | `MockProvider(transcripts, locale, menu)` — scripted `AsyncIterableIterator<IntentProposal>` for Playwright + unit tests (no mic, no model). | `MockProvider.intents` `:24-29` |
| `whisper-provider.ts` | `WhisperProvider(transcriber, locale, menu)` — production source: `intents(utterances: AsyncIterable<PcmAudio>)` transcribe→match→yield, fail-quiet, one bad segment can't end the stream. `once(pcm)` = eval-harness path. | `WhisperProvider` `:24`; `intents` `:42-54`; `once` `:57-61` |
| `transcriber.ts` | `Transcriber` **port** (object with `transcribe(PcmAudio):Promise<string>`, NOT a bare function — M3/R2-F) + `PcmAudio = Float32Array` (mono 16 kHz). | `Transcriber` `:14-21` |
| `transformers-transcriber.ts` | `TransformersTranscriber` — real ASR via **dynamic-imported** `@huggingface/transformers` (`Xenova/whisper-base`, `WHISPER_LANG` sq/en/uk), WebGPU default, greedy/temp-0 decode, `warmup()`. Non-literal import specifier so `packages/voice` builds without the heavy dep. | `WHISPER_LANG` `:17`; `warmup` `:80-82`; `transcribe` `:84-98` |
| `index.ts` | Public surface (exports the above). | `index.ts:6-18` |
| `scripts/audio-eval.ts` | The §4.2 real-audio gate harness (fixed WAV corpus → real WhisperProvider → IRA / dangerous-misfire / fail-quiet). Runs self-hosted, **not** cloud CI. | header `:1-24` |
| `fixtures/` | `smoke-manifest.json` + `jfk-en-16k.wav` (pipeline smoke, not a launch gate). | — |
| `__tests__/*` | Unit proofs (see 1.4). | — |

### 1.2 BUILT — control plane / server (the hot-kill + CSP)

- `apps/api/src/lib/voice-flag.ts` — `isVoiceEnabled()` = `VOICE_CONTROL_ENABLED === 'true' && VOICE_KILL !== 'true'`. **Single source of truth** for both the config endpoint and the CSP. Default OFF. (`voice-flag.ts:11-13`)
- `apps/api/src/routes/public/voice-config.ts` — `GET /api/public/voice-config` → `{ enabled }`, `Cache-Control: no-store`, no DB/auth/PII. Lives under `/api/` so the cache-first SW (`apps/api/public/sw.js`) never pins the kill signal (R2-A). (`voice-config.ts:11-15`)
- **Registered** at `apps/api/src/bootstrap/routes.ts:157` (`fastify.register(publicVoiceConfigRoutes)`), imported `:65`. **This is live.**
- `apps/api/src/lib/spa-shell.ts:151-158` — CSP `connect-src` widened to the R2 origin **only when `isVoiceEnabled()`** (breaker R2-E; a `VOICE_KILL` closes the origin too). While dark the CSP is byte-unchanged. Full CSP string at `spa-shell.ts:159`.

### 1.3 BUILT — guardrails already proven (red→green)

- **`no-voice-engine-callback`** eslint-local rule — bans any function-typed parameter in `packages/voice` exported signatures (no injected write-capable closure — M3/R2-F). Rule `tools/eslint-plugin-local/src/index.js:681-720`; tests `tools/eslint-plugin-local/__tests__/rules.test.ts:109-133`; fixtures `__fixtures__/{good,bad}-voice-engine-callback.ts`.
- **Fail-closed control-plane unit test** — `apps/api/tests/voice-flag.test.ts` (OFF when unset, no truthy coercion, `VOICE_KILL` overrides).
- **Gate + matcher unit tests** — `packages/voice/src/__tests__/`:
  - `confirmation-gate.test.ts` — READ_ONLY auto-applies; STATEFUL not applied until `confirm()`; `cancel()` discards; excluded money/checkout kinds REJECT; **unknown kind REJECT fail-closed**; dietary-named `SELECT_CATEGORY` rejected; `classify()` maps known + fail-closes the rest.
  - `matcher.test.ts` — sq/en/uk corpus; asserts the corpus covers every active-scope kind.
  - `mock-provider.test.ts` — READ_ONLY auto-apply + dietary REJECT end-to-end; ADD held pending, applies only after confirm.
  - `whisper-provider.test.ts` — transcribe→match→yield; fail-quiet; one bad segment doesn't end the stream; pure readonly data crosses the boundary; drives the SAME gate outcomes as the mock.

### 1.4 DEAD / ABSENT

- **`use-voice-order` — does not exist anywhere in the tree** (grep across all `.ts/.tsx` returns nothing; `packages/ui/src` has no `*voice*` file). The audit flag refers to a hook that was already removed / never landed on this branch. **Nothing to delete** — note it in the PR so the audit item can be closed.

### 1.5 NOT BUILT — what ADR-0015 lists as "specified but NOT built" and remains for Phase-1

ADR §Status: *"Specified as required exit criteria, NOT built."* Concretely, none of the browser-facing
layer exists yet:

1. **The MicFab + the whole UI state machine** (`packages/ui`) — no `MicFab` component anywhere (grep finds only the eslint test). Missing: MicFab, listening/transcribing visuals, confirmation chip, disclosure sheet, read-back panel, disambiguation chips, partial-transcript pill, the "Voice" preference toggle.
2. **The web-side engine bootstrap + intent pack + mic capture** (`apps/web`) — no consumer of `@deliveryos/voice` exists (grep for `@deliveryos/voice` / `packages/voice` outside the package returns nothing). Missing: the render predicate, the runtime-config fetch, the WebGPU warmup probe, `getUserMedia` + `AudioContext`@16 kHz + VAD segmentation producing `AsyncIterable<PcmAudio>`, the `VoiceHandlers` implementation binding semantic args to the real `MenuPage`/`CartProvider` setters, and locale wiring.
3. **The i18n `voice.*` strings** — the 22 keys in ui-spec §6 are **not** in `packages/ui/src/lib/i18n-catalog.ts` yet (grep returns nothing). Must be added via `scripts/i18n-add.ts` in sq/en/uk.
4. **Remaining machine-checks** (proposal §6 / §10): the *no-mutator-import* half of guardrail #1, a dedicated capability-table `never`-exhaustiveness/table test, the **actor-anonymous telemetry-schema test**, the **client-side fail-closed control-plane test** (config-fail ⇒ mic absent), the **true-dark bundle guardrail**, and the **debug-overlay-absence + public-deploy-arg** guardrails.
5. **The actor-anonymous telemetry counter** `{ intent_kind, matched, confidence_bucket, locale }`.
6. **The Playwright MockProvider E2E** (wiring + gate proof).
7. **R2 model hosting** — the pinned ONNX whisper weights self-hosted on R2 (ops/deploy step).
8. **(Research regime, gated on human decisions — not Phase-1 code):** the C2-consented corpus + its scheduled 90-day deletion job + expiry tripwire.

---

## 2. The action catalog (every storefront voice intent)

Classification is **already encoded** in `capability-table.ts` + `dietary-denylist.ts`; this table is
the human-readable mirror an implementer wires the adapter against. Trigger phrases are the built
`TRIGGERS` (`matcher.ts:25-56`). "Bound handler" is the `VoiceHandlers` method (`confirmation-gate.ts:12-21`)
→ the real apps/web setter it must map to.

| Intent kind | Class | Trigger phrases — sq / en / uk | `VoiceHandlers` method → real setter | Confirmation UX |
|---|---|---|---|---|
| `ADD_TO_CART` | **STATEFUL** | sq `shto`, `shtoje`, `me shto` · en `add`, `i want`, `put in cart` · uk `додай`, `додати`, `хочу` (+ number-word/qty parsing) | `addToCart(args)` → build a `CartItem`, then `CartProvider.addItem` (`apps/web/src/lib/CartProvider.tsx:89`, `addItem` type `:45`) | **Confirm chip** (ui-spec §3): "Add {qty}× {item}?", equal-weight Confirm/Cancel, 12 s timeout = Cancel, Esc/outside = Cancel. Runs `addItem` **once** on tap only. |
| `SET_SORT` | READ_ONLY | sq `rendit`/`sipas cmimit`/`me te lira`/`sipas popullaritetit`/`sipas emrit` · en `sort`/`sort by price`/`cheapest first`/`by popularity`/`by name` · uk `сортувати`/`за ціною`/`найдешевші`/`за популярністю`/`за назвою` | `setSort({by})` → `setSortBy` (`MenuPage.tsx:227`). **Arg mismatch to resolve (see §2.1)** | Auto-apply + toast w/ Undo |
| `SET_MACRO_LENS` | READ_ONLY | sq `trego makro`/`sipas proteinave`/`proteina`/`kalori` · en `show macros`/`by protein`/`calories` · uk `покажи макрос`/`за білком`/`калорії` | `setMacroLens({lens})` → `setMacroLens` (`MenuPage.tsx:223`; gated by `FILTER_LENSES_ENABLED` `:515`) | Auto-apply + toast w/ Undo |
| `SELECT_CATEGORY` | READ_ONLY **(dietary-named → REJECT)** | sq `kategoria`/`trego kategorine` · en `category`/`show category` · uk `категорія`/`покажи категорію` (+ resolved category name) | `selectCategory({categoryId})` → `setSelectedCategory` (`MenuPage.tsx:251`, applied at `:513`) | Auto-apply + toast; **dietary-named category dropped before apply** (`confirmation-gate.ts:49-61`) |
| `SET_SEARCH` | READ_ONLY | sq `kerko`/`gjej`/`kerko per` · en `search`/`search for`/`find` · uk `знайти`/`пошук`/`шукати` (+ query slot) | `setSearch({query})` → `setSearchQuery` (`MenuPage.tsx:241`) | Auto-apply + toast w/ Undo |
| `TOGGLE_COMPARE` | READ_ONLY | sq `krahaso` · en `compare` · uk `порівняй`/`порівняти` (+ resolved product) | `toggleCompare({productId})` → `toggleCompare` (`MenuPage.tsx:191`) | Auto-apply + toast w/ Undo |
| `READ_ORDER` | READ_ONLY (checkout-read) | sq `lexo porosine`/`porosia ime`/`cfare kam ne shporte` · en `read my order`/`what is in my cart`/`my order` · uk `прочитай замовлення`/`що в кошику`/`мій кошик` | `readOrder({})` → render read-back panel from own cart (`ClientLayout.tsx:187-257`) | Auto-apply → `aria-live` read-back panel (ui-spec §4). **No PII egress, own cart only** |
| `NAVIGATE_CHECKOUT` | READ_ONLY (checkout-read) | sq `te arka`/`shko te arka`/`vazhdo te arka`/`paguaj` · en `go to checkout`/`proceed to checkout`/`check out` · uk `до кошика`/`оформити`/`до оплати` | `navigateCheckout({})` → open checkout sheet (`setCheckoutOpen(true)`, `ClientLayout.tsx:271`) | Auto-apply → route/sheet change. **Navigation only — no field write, no place-order** |

### 2.1 EXCLUDED by construction (no `IntentKind`, REJECTed fail-closed — never build a handler)

- **Money / checkout writes / place-order / payment / order-finalization** — no `kind` exists; checkout adds **zero** stateful/money voice grammar. No address/phone/notes voice field. (proposal §6; `types.ts:8-13`)
- **Allergen / dietary-safety filters (`setFilterAllergen`, `MenuPage.tsx:234/964`)** — a voice-set safety filter is a false safety assertion off a noisy channel (C1). Never voice-reachable; there is deliberately no `VoiceHandlers.setFilterAllergen`.
- **Dietary/allergen-*named categories*** — even though `SELECT_CATEGORY` is READ_ONLY, a resolved category whose name hits `DIETARY_TOKENS` is **dropped before apply** (R2-B; `confirmation-gate.ts:49-61`). Closes the class, not just the setter.
- **Cash-settling courier actions (`arrived`/`completeDelivery`)** and all admin/courier stateful voice — removed from this build, deferred to a future council (H4; ADR §Status).

### 2.2 Two adapter arg-mismatches to resolve during wiring (inline-fix, flag with the implementer)

The matcher emits **semantic** args; the real setters have a narrower vocabulary. The web adapter must
map, and where the target vocabulary is missing, **drop rather than guess** (fail-quiet):

1. **Sort:** matcher emits `by: 'price' | 'popularity' | 'name'` (`matcher.ts:17`,`:143-158`). `MenuPage.sortBy` is `'default' | 'price-asc' | 'price-desc' | 'name'` (`MenuPage.tsx:227`) — there is **no `popularity` sort**, and `price` must resolve to a direction. Map `price → 'price-asc'` (matches the "cheapest first"/`me te lira` triggers), `name → 'name'`, `popularity →` **no-op + no-match toast** (do not silently pick a wrong sort). Consider adding a `price-desc` trigger later; not now (YAGNI).
2. **Macro lens:** matcher emits `lens: 'protein' | 'calories' | 'carbs' | 'fat'` (`matcher.ts:18`). Confirm `MenuPage`'s `MacroLens` union covers all four; map any uncovered lens to no-op. The lens surface is itself behind `FILTER_LENSES_ENABLED` (`MenuPage.tsx:515`) — if that flag is off, `SET_MACRO_LENS` must degrade to a no-match toast, not a dead apply.

`ADD_TO_CART` args are `{ productId, productName, qty }` (`matcher.ts:226-231`) but `CartProvider.addItem` takes a full `CartItem` (`CartProvider.tsx:45`). The adapter must **resolve the full product** (price, modifiers, currency) from the already-loaded menu data by `productId` and construct the `CartItem` — the matcher deliberately carries only id+name+qty.

---

## 3. Best-practice UX design (the pixels — grounded in ui-spec.md)

The UI is fully specified in `docs/design/voice-control/ui-spec.md`; the below is the decision summary
an implementer needs, with the *why* for each best-practice choice.

### 3.1 Push-to-talk, NOT wake-word (RECOMMEND — this is mandated, not optional)

**Recommendation: explicit push-to-talk (tap the MicFab to start one utterance).** No wake-word, no
always-listening. Reasons: (a) ADR/proposal §1 make wake-word **FORBIDDEN** (`proposal.md:45`, ui-spec §5)
— an always-listening mic is the exact surveillance perception R-E warns of; (b) it is universal
voice-commerce best practice (Alexa/Google confirm before acting; none auto-listen for purchases in a
web tab); (c) it makes consent a per-use gesture, aligning with "audio is PII." The idle FAB is a
**calm, static** button — **no idle pulse animation** (a pulsing idle mic reads as "always listening",
ui-spec §1). Motion happens only *after* a tap.

### 3.2 Visual states (one finite machine, no dead-ends — ui-spec §2)

`IDLE → (first tap) DISCLOSURE SHEET → PERMISSION-REQUEST → LISTENING → TRANSCRIBING →
INTENT-PROPOSAL → {APPLIED | CONFIRMING | ERROR} → IDLE`. Per-state visuals (all token-based, dark +
paper-skin auto, reduced-motion-safe):

- **idle** — static `ti-microphone` FAB, `--brand-primary`, `--tap-critical` (56px), bottom-right (`z-sticky` 200), unmounted while any modal is open.
- **listening** — expanding ring (`color-mix` 35% `--brand-primary`) + a **live partial-transcript pill** above the FAB (`aria-live="polite"`); reduced-motion → static filled ring + "Listening…" text.
- **transcribing** — indeterminate spinner + frozen words + "…".
- **proposal (READ_ONLY)** — menu visibly updates + **toast w/ Undo**.
- **proposal (STATEFUL)** — **confirmation chip** above the FAB; menu unchanged until confirm.
- **confirming/applied** — brief check pulse, chip/toast dismisses; focus returns to the FAB.
- **error** — inline pill with recovery affordance (see 3.6).

### 3.3 Barge-in

Push-to-talk is single-utterance, so "barge-in" here = **a new tap always pre-empts** any in-flight
listening/transcribing session and **any pending STATEFUL proposal is cancelled** (fail-safe to no
write) before a new session starts. The gate's `#pending` is *last-proposal-wins* already
(`confirmation-gate.ts:70-72`); the UI must call `gate.cancel()` on re-tap so a stale chip can never be
confirmed against a new utterance. No audio overlap — the previous `AudioContext`/worker session is torn
down first.

### 3.4 Partial feedback

Show incremental partial transcript in the listening pill (`aria-live="polite"`, non-interruptive). The
built `WhisperProvider` yields per completed utterance; for *live partials* the browser mic layer may
surface interim decode text if available, else the pill simply shows "Listening…". Partial text is
**ephemeral component state, never logged, never sent** (ui-spec §8).

### 3.5 The confirmation affordance for stateful intents (safety-critical — ui-spec §3, C-2/STOP-2)

A non-modal chip at `z-toast` (500) echoing the **parsed** intent before any write: "Add 2× Sufllaqe?".
**Equal affordance weight is a hard assertion:** Confirm and Cancel share the *same button class* —
identical size (`--tap-min` 44px), `--brand-surface-raised` background, `--brand-border`, `--brand-text`.
Neither is a bright CTA against a grey ghost (a lopsided hierarchy is a soft dark-pattern). A glyph
(`ti-check`/`ti-x`) may differ; color/size may not. **Confirm runs `addItem` exactly once; Cancel writes
nothing; timeout (~12 s)/Esc/outside-tap = Cancel.** Focus moves to the chip container on appear; neither
button is a default-Enter primary (a STATEFUL write must be deliberate). The chip is the **only** path
from a STATEFUL proposal to `addItem`.

### 3.6 Error / no-match handling (nothing dead-ends — ui-spec §2 error matrix)

| Condition | User sees | Recovery |
|---|---|---|
| Mic denied | pill `voice.err.mic_denied` | no re-prompt loop; touch intact; tap re-prompts once |
| Model fetch fails / offline (30 s) | pill `voice.err.model_offline` + **Retry** | Retry re-attempts R2 GET; no spinner-forever |
| Low-confidence / no-match | pill `voice.err.no_match` + optional did-you-mean chips | tap FAB to retry; **never auto-executes a STATEFUL action on low confidence** (`MIN_CONFIDENCE 0.6`) |
| Ambiguous (ties) | disambiguation chips | tap to pick; never guess-executes |
| Worker crash / inference timeout (~8 s) | pill `voice.err.try_again`; worker respawned | tap to retry; main thread unaffected |
| Dietary-named category / excluded kind | silently dropped → `no_match` copy | (never a "we ignored a safety request" message — that itself implies safety handling) |

### 3.7 WebGPU-unavailable fallback (mic hidden — not greyed)

The MicFab **renders `null`** (never a disabled grey button) unless ALL hold (ui-spec §1):
1. build flag `import.meta.env.VITE_VOICE_CONTROL_ENABLED === 'true'` (else engine never in bundle),
2. `GET /api/public/voice-config` returned `{ enabled: true }` (`cache:'no-store'`; reject/`!ok`/`enabled!==true` ⇒ absent — R2-A),
3. **WebGPU adapter present AND the bounded warmup probe passed** in its time/OOM budget (`TransformersTranscriber.warmup()` `transformers-transcriber.ts:80`; no WebGPU on iOS Safari + most in-app WebViews ⇒ absent),
4. user pref ≠ `off`,
5. secure context + `getUserMedia` supported.
Voice is **additive** — every voice action has a pre-existing touch/keyboard equivalent (ui-spec §7). No
COOP/COEP, so WASM-threads/single-thread is **not** offered (would break map tiles/fonts/Plausible/OSRM,
proposal §2.2).

### 3.8 Language detection / selection for the 3 locales

- **Source of truth = the active UI locale** `getLocale()` (`packages/ui/src/lib/i18n.ts:57`), one of `'sq'|'en'|'uk'`, persisted in `dos_locale`, **default `sq`**. The storefront already exposes a language switcher (`getLocales()` `i18n.ts:63`).
- Per-tenant default: `MenuPage` carries `default_locale` (`MenuPage.tsx:100`) — seed the locale from it on first load if the user has no stored preference (existing behaviour).
- **The engine/Whisper language hint is that same locale, mapped 1:1** by `WHISPER_LANG` (`transformers-transcriber.ts:17`: `sq→albanian, en→english, uk→ukrainian`) and passed as `WhisperProvider`'s `Locale`. **No auto-detect** — the user's chosen locale is authoritative (avoids sq/uk confusion on short commands). Changing the UI language re-instantiates the engine with the new locale + `TRIGGERS` set.

---

## 4. Multilingual intent-matching (sq / en / uk first-class from the start)

- **Closed-vocabulary matcher, not dictation** (ADR §Context). Each locale has its own `TRIGGERS` map
  (`matcher.ts:25-56`) already populated for **all three** locales; `NUMBER_WORDS` (`:59-67`) covers
  sq/en/uk quantity words (incl. Cyrillic). Sort/macro keyword detection is per-locale
  (`detectSortKey` `:143-158`, `detectMacroLens` `:160-166`). uk is **not** a bolt-on — it is a
  first-class key in every per-locale structure today.
- **Matching flow:** `normalize()` (NFD diacritic-strip, so `qumësht`~`qumesht`, Cyrillic preserved) →
  specific/phrasal intents first, generic `ADD` verb last → semantic args → `IntentProposal` or `null`
  below `MIN_CONFIDENCE`/unresolved slot (fail-quiet). Product/category slots use fuzzy name overlap
  (`resolveByName` `:117-141`, threshold 0.5) against the tenant `MenuContext`.
- **Per-locale safety independence (H3):** string parity ≠ safety parity. Each locale is gated
  **independently** by the §4.2 audio harness (IRA lower-95%-CI ≥ 80% + dangerous-misfire upper-bound
  ≤ 2%). A locale that has strings but has not passed its bound **ships dark even though the strings
  exist** (ui-spec §6). The dietary denylist carries **all three** locales' tokens and matches
  regardless of the active locale (`dietary-denylist.ts:10-53`) — defence in depth.
- **String parity gate (sq/en/uk):** all `voice.*` UI strings are added via
  `pnpm exec tsx scripts/i18n-add.ts <key> "<en>" "<sq>" "<uk>"` (`scripts/i18n-add.ts`), which writes one
  key with all locales into `packages/ui/src/lib/i18n-catalog.ts`; any locale left as a `TODO:` draft
  **fails the parity gate** (`scripts/i18n-parity`). Author the **sq** strings first (hardest,
  default locale — ui-spec §6). The 22 keys are listed in ui-spec §6.
- **Extending vocabulary** is a matcher-only, unit-testable change: add phrases to `TRIGGERS[locale]`
  and a corpus row to `matcher.test.ts`. Runtime per-tenant vocabulary biasing (menu names) is already
  the mechanism (the `MenuContext` slot resolution); no shipped static demo-vocab bias (M6).

---

## 5. The guardrails ADR-0015 mandates (enumerated as concrete red→green artifacts)

Legend: ✅ built · ⬜ to build in Phase-1. Each ⬜ is a deterministic test/lint that must go
**red before green** and get a `docs/regressions/REGRESSION-LEDGER.md` row.

| # | Guardrail (ADR §6/§8/§9/§10) | Status | Where it lives / what it asserts |
|---|---|---|---|
| G1a | Engine exports **no function-typed param** (no injected write-capable closure — M3/R2-F) | ✅ | `no-voice-engine-callback` (`eslint-plugin-local/src/index.js:681`) + tests |
| G1b | `packages/voice` imports **no** mutating API client / `CartProvider` mutator / status handler | ⬜ | New eslint-local rule **or** a grep guardrail (CI) over `packages/voice/src/**` — RED on any import of `apps/web`, a fetch/api-client, or a `Cart*` mutator; GREEN on the current pure engine. (proposal §6 guardrail #1, second half) |
| G2 | Gate: STATEFUL without confirm → handler **not** called; with confirm → called **once**; **unknown kind → REJECT, no handler, no auto-apply** | ✅ | `confirmation-gate.test.ts` |
| G3 | **`never`-exhaustiveness build check** — a new `IntentKind` with no table row fails the build | ✅ (structural) / ⬜ (explicit) | Structural via `Record<IntentKind,…>` (`capability-table.ts:9`). Add an **explicit `never`-assertion** in a dedicated capability-table test so the invariant is asserted, not just implied. |
| G4 | Capability-table **table test** — add-to-cart is STATEFUL; **no** `place_order`/`pay`/`checkout` kind; **no** allergen/dietary kind and `setFilterAllergen` not voice-reachable (C1); **no** `arrived`/`completeDelivery`/settling kind (H4) | ⬜ (partial in gate test) | Promote the excluded-kind assertions into a dedicated `capability-table.test.ts` iterating the full `IntentKind` union + asserting the forbidden strings classify to `REJECT` and no `VoiceHandlers` key exists for them. |
| G5 | **Category-denylist test (R2-B)** — a dietary-named category (`Pa gluten`, `Vegan`, `Без глютену`) is **not** voice-auto-applied; denylist is one exported constant | ✅ | `confirmation-gate.test.ts` (dietary-named case) + `dietary-denylist.ts`. Extend with the exact ui-spec token list if any are missing. |
| G6 | **Actor-anonymous telemetry-schema test (Counsel C-1)** — the voice counter record `{ intent_kind, matched, confidence_bucket, locale }` carries **no** `courier_id`, **no** `user_id`/actor id, **no** latency/timing field | ⬜ | Build the telemetry emitter (§6 below) + a red→green test asserting the record's key set is exactly the 4 allowed keys. Locked from Phase-1 (forbids the surveillance gradient at the column before any worker mic exists). |
| G7 | **Fail-closed control-plane test (R2-A)** — `/api/public/voice-config` reject / `!res.ok` / `enabled !== true` ⇒ mic **not rendered**, engine **not imported**, model **not fetched** | ✅ (server) / ⬜ (client) | Server side proven by `voice-flag.test.ts`. Add the **client-side** proof: a Playwright test that routes the config GET to fail/`{enabled:false}` and asserts `[data-testid=voice-mic-fab]` is **not** visible and no `@huggingface/transformers` request fired. |
| G8 | **True-dark bundle guardrail (L2)** — with `VITE_VOICE_CONTROL_ENABLED` unset, the engine + `@huggingface/transformers` are **absent** from the main storefront chunk | ⬜ | CI: build the storefront with the flag off, grep the emitted chunks for the voice engine / transformers symbols → RED if present. Proves zero bundle delta when dark. |
| G9 | **Debug-overlay-absence guardrail (R2-C)** — no transcript-surfacing symbol survives a build where `VITE_VOICE_TRANSCRIBE_DEBUG` is not `true` | ⬜ | CI greps built chunks for the overlay/transcript-render symbol → RED if present when the flag is off. |
| G10 | **Public-deploy-arg guardrail (R2-C)** — every public deploy target carries `VITE_VOICE_TRANSCRIBE_DEBUG=false`; the deploy fails if a public target sets it true | ⬜ | CI env-matrix assertion over the deploy configs (prod + public staging `/s/:slug`). |
| G11 | **Decline-path + visually-equal-affordance (STOP-2 / C-2)** — the disclosure "Not now" never imports the engine / requests the mic / fetches the model; Confirm & Cancel have identical computed `background`/`border-width`/`min-height`/`font-weight` | ⬜ | Playwright: assert engine-never-imported after decline (touch still works) + a computed-style equality assertion on the two chip buttons (ui-spec §3/§5). |

---

## 6. Phased build order (each PR flag-dark, each with its proof)

Global flags (both required to activate; default OFF): client `VITE_VOICE_CONTROL_ENABLED=false`,
server `VOICE_CONTROL_ENABLED` (+ `VOICE_KILL`). Deploy dark to verify; **launching is a separate,
explicit act**. Every PR: commit (feature branch) → staging deploy → proof pasted (Mandatory Proof
Rule). PRs are ordered so each has runnable proof and nothing ships a half-wired mic.

**Prerequisite (NEEDS-HUMAN, does not block dark code):** ADR §Status requires a recorded **demand
signal** (R-J) before Phase-1 *launch*, ranked against launch-blockers B1/B2/B3. Building dark is fine;
flipping the flag on is the gated act.

| PR | Scope | Touches | Proof | Red-line? |
|---|---|---|---|---|
| **PR-0** | Close the audit item: confirm `use-voice-order` is absent; note it in the PR. Add the **missing guardrails that need no UI**: G1b (no-mutator-import), G3 (explicit `never`), G4 (capability-table test). | `packages/voice/src/__tests__/`, `tools/eslint-plugin-local` | `pnpm --filter @deliveryos/voice test` + eslint rule red→green | safe-direct (tests only) |
| **PR-1** | **i18n strings** — add the 22 `voice.*` keys (sq/en/uk) via `scripts/i18n-add.ts`. | `i18n-catalog.ts` | `scripts/i18n-parity` green; `pnpm typecheck` | safe-direct |
| **PR-2** | **Web adapter (headless) + MockProvider E2E** — the `VoiceHandlers` impl mapping semantic args → real setters (§2/§2.1), `MenuContext` builder, gate construction. **No mic, no model yet** — driven by `MockProvider`. | `apps/web/src/**` (new intent-pack module), imports `@deliveryos/voice` | **Playwright** on staging `/s/:slug`: inject `MockProvider`, "speak" sq/en/uk transcripts, assert cart/filter/sort DOM via `toBeVisible()`/`toContainText()`; assert STATEFUL held until confirm tap; assert dietary category dropped. (proposal §10 CI lane) | safe-direct (READ_ONLY setters + confirm-gated add; no money binding) |
| **PR-3** | **MicFab + UI state machine** (`packages/ui`) — FAB, render predicate, disclosure sheet, confirm chip, toast/Undo, read-back panel, disambiguation, partial pill, "Voice" pref toggle. Wired to PR-2's gate. | `packages/ui/src/**` | **Playwright** (still MockProvider-fed): full state machine visible; **G7 client** (config-fail ⇒ FAB absent); **G11** (decline never imports engine + equal-affordance computed-style). Reduced-motion + keyboard/Esc paths. | safe-direct |
| **PR-4** | **Mic capture + WebGPU warmup probe** — `getUserMedia` + `AudioContext`@16 kHz + VAD segmentation → `AsyncIterable<PcmAudio>` feeding the real `WhisperProvider`; the bounded warmup probe gating the FAB (clause 3). Dynamic-import `@huggingface/transformers`. | `apps/web/src/**` | Capability test: FAB hidden when WebGPU absent / warmup fails / mic denied (ui-spec §2). **G8** true-dark bundle guardrail. Manual on a WebGPU device. | safe-direct |
| **PR-5** | **Telemetry counter** — actor-anonymous `{ intent_kind, matched, confidence_bucket, locale }` to existing telemetry. | `apps/web` + counter sink | **G6** telemetry-schema test (record has exactly the 4 keys; no actor id/latency). | safe-direct (no PII, no new RLS/write surface) |
| **PR-6** | **Zero-egress deploy guardrails** — G9 (debug-overlay-absence) + G10 (public-deploy-arg `VITE_VOICE_TRANSCRIBE_DEBUG=false`). | CI config | Guardrail red→green in CI. | safe-direct |
| **PR-7 (ops)** | **R2 model hosting** — self-host the pinned ONNX whisper weights on R2 (reuse `dowiz-images` pattern); confirm CSP `connect-src` widening (`spa-shell.ts:151`) resolves the origin when `isVoiceEnabled()`. | R2 bucket + deploy args | Model GET succeeds from the widened CSP with voice enabled on staging; 404/blocked when dark. | safe-direct (static asset host) |
| **Phase-0 gate (separate lane, gated on human decisions — NOT a launch PR)** | Run `scripts/audio-eval.ts` over the **C2-consented ≥300/locale, ≥15-speaker** corpus → per-locale IRA + dangerous-misfire report. | `packages/voice/scripts/` | The §4.2 report **is** the launch gate; **green CI ≠ IRA passed**. Corpus needs consent regime + 90-day deletion job (research data-controller = NEEDS-HUMAN). | gated: corpus consent (R-I/C-4) |

**REQUIRES-COUNCIL (do NOT build in this plan):** any admin (Phase 3) or courier (Phase 4) voice; any
intent that would auto-execute a money/order/place-order/payment action off voice; any `setFilterAllergen`
or "safe-for-me" voice filter; any voice telemetry that persists a transcript or an actor id. These are
excluded by construction and re-open only via a separately-convened council (ADR §Status, proposal §10).

---

## 7. Explicit conflict callout (for the operator)

**The ask:** "a voice agent that can make *any action needed* based on voice input."

**Where "any action" cannot mean auto-execution, and why that is correct:**

1. **Money / order finalization has NO voice grammar at all — by construction, not by a toggle.** There
   is no `place_order`, `pay`, `checkout-field-write`, or `order-finalization` `IntentKind`
   (`packages/voice/src/types.ts:14-22`), so the matcher cannot even *produce* such a proposal, and if
   one somehow appeared the fail-closed gate `REJECT`s it (`capability-table.ts:25-27`,
   `confirmation-gate.ts:63-64`). This is universal voice-commerce best practice — Alexa/Google/Siri all
   confirm (and never silently place) a purchase — and it is ADR-0015's binding hard rule.

2. **Add-to-cart (the one stateful action) requires an explicit human confirmation tap.** Voice
   *proposes* "Add 2× Sufllaqe?"; the item enters the cart **only** when the human taps Confirm on an
   equal-weight chip (`confirmation-gate.ts:76-84`; ui-spec §3). Voice initiates; the human commits.

3. **Allergen / dietary-safety selections are voice-*proposable* but touch-only to *apply*** — a
   mis-heard "gluten-free" must never render as an allergen-safe menu (a false safety assertion off a
   noisy channel), so dietary-named categories are dropped and `setFilterAllergen` is unreachable by
   voice (`dietary-denylist.ts`, `confirmation-gate.ts:49-61`).

**How confirm-then-execute satisfies the operator's intent safely:** the voice agent **can drive every
in-scope action of the storefront** — search, sort, macro-lens, category, compare, add-to-cart, read the
cart, and navigate to checkout — in sq/en/uk from day one. It *reaches* checkout and *reads back* the
order by voice; the customer completes the money step with a deliberate tap. So "any action" is realized
as **"voice can propose or initiate any action, and the human commits the irreversible/money ones with
one tap"** — the maximum capability that does not turn a noisy channel into an unconfirmed money
transaction. **Auto-executing a money/order action off voice is out of scope here and is
REQUIRES-COUNCIL (Phase 3/4).** If the operator wants that, it is a new, separately-gated decision — this
plan will not silently cross that line.
