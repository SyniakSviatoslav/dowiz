# Voice Control (Whisper, on-device) — Integration Plan

> Status: DRAFT plan (pre-council, pre-code). Date: 2026-06-30.
> Goal: voice-driven **actions** (not dictation) on the storefront first, then admin & courier.
> Engine: **OpenAI Whisper via Transformers.js**, 100% on-device (WebGPU → WASM fallback), MIT.
> Languages: **sq (default) / en / uk** — single engine, all three.

---

## 0. Why Whisper on-device (recap of the research gate)

- **Vosk** — no Albanian model → eliminated (sq is the default locale).
- **Web Speech API** — free but streams audio to Google (subprocessor + GDPR disclosure), Chrome/Edge-only.
- **Whisper + Transformers.js** — MIT, runs in the browser via ONNX Runtime Web, **zero audio egress**,
  covers sq+en+uk. Albanian quality on the base model is unmeasured/weak, but our domain is a
  *constrained command vocabulary* (menu items + ~20 verbs), so the **intent matcher**, not raw WER,
  is what determines usability. Albanian fine-tunes exist (`Kushtrim/whisper-large-v3-turbo-shqip`,
  `Kushtrim/whisper-small-sq`) if the PoC shows base `sq` is insufficient.

Key consequence: **voice is only the input layer.** The value and the robustness live in the
intent→action layer, which reuses action functions that already exist and already enforce authz.

---

## 1. Architecture

```
🎤 mic gesture (FAB)
  → MediaRecorder / Web Audio  (downsample to 16 kHz mono Float32)
  → ring buffer + VAD (silence-gated chunking)
  → whisper.worker.ts  (Web Worker — first worker in apps/web)
        pipeline('automatic-speech-recognition', MODEL,
                 { device: 'webgpu'|'wasm', dtype: 'q8'|'q4', language: <locale> })
  → transcript string (sq/en/uk)
  → IntentMatcher  (LOCAL — no LLM, no network)
        • per-locale command grammar  (verbs → action templates)
        • fuzzy slot-fill against this role's vocabulary (menu items / order ids / statuses)
        → { action, target, confidence }
  → CommandRegistry dispatch  → EXISTING action functions
        (addItem / setMacroLens / setFilterAllergen / toggleCompare …;
         admin handleUpdateStatus; courier handleAccept …)
  → confirm-then-execute for anything stateful; auditory cue via existing useSound()
```

### Why a Web Worker
`apps/web` has **no workers today**. Whisper inference (even base) blocks the main thread for
hundreds of ms; it must run off-thread. Vite supports
`new Worker(new URL('./whisper.worker.ts', import.meta.url), { type: 'module' })`.

### Why the engine must be lazy / dynamic-import only
`@huggingface/transformers` is a large JS payload and the model is **60–200 MB**. It must **never**
touch the storefront critical path or the bundle-perf gate. Load rule:
**dynamic `import()` the engine + fetch the model only when (a) the flag is on AND (b) the user taps
the mic** — explicit gesture. Cached in IndexedDB by Transformers.js after first load (offline after).

---

## 2. Model strategy (the crux)

| Phase | Model | dtype | ~size | Covers |
|---|---|---|---|---|
| v1 (all roles) | `Xenova/whisper-base` (multilingual) | q8 (WASM) / fp16 (WebGPU) | ~60–80 MB | en good, uk good, **sq passable in a constrained vocab** |
| v1.5 sq upgrade *(gated on PoC)* | `Kushtrim/whisper-small-sq` → converted to ONNX | q8 | ~120 MB | sq strong (incl. Gheg dialect for the AL/Kosovo market) |

- Start with **one multilingual model** keyed by the current UI locale (`language: locale`). Simplest,
  one download, good enough for en/uk immediately.
- **Albanian fine-tune is a fast-follow, not a blocker.** It is NOT pre-converted for Transformers.js,
  so it needs an offline ONNX step:
  `python -m scripts.convert --quantize --model_id Kushtrim/whisper-small-sq --task automatic-speech-recognition`
  → host the ONNX artifacts ourselves.
- **Self-host the ONNX weights on R2** (reuse the `dowiz-images`/asset-serving pattern) and point
  Transformers.js at it (`env.remoteHost` / `env.allowLocalModels`). Avoids a hard HF-Hub runtime
  dependency, gives us version pinning and a privacy-clean origin.

**Robustness lever (more important than model size):** bias decoding toward our domain and let the
intent matcher fuzzy-match. A weak `sq` transcription of "shto sufllaqe" still resolves if "sufllaqe"
is in the tenant's menu vocabulary within edit-distance.

---

## 3. Package / module layout (role-agnostic engine + per-role intent packs)

New shared engine (role-agnostic) — put in `packages/ui` (or a small new `packages/voice`):

```
packages/.../voice/
  engine/whisper.worker.ts     # pipeline init + transcribe; webgpu→wasm capability detect
  engine/audio.ts              # mic capture, 16kHz resample, VAD chunking
  engine/useVoice.ts           # React hook: state machine idle→listening→thinking→result→error
  intent/matcher.ts            # transcript → {action,target,confidence}; fuzzy slot-fill
  intent/grammar.ts            # CommandRegistry type + per-locale verb tables (from i18n)
  ui/MicFab.tsx                # shared FAB + live transcript + confirm chips + a11y
```

Each **role** registers its own intent pack (command registry) + binds to its existing handlers:

- **Client / storefront** (FIRST): `add {dish} [qty]`, `filter {allergen|category}`, `sort cheapest`,
  `macro {protein|kcal}`, `compare {dishA} {dishB}`, `search {text}`
  → `addItem`, `setFilterAllergen`, `setSelectedCategory`, `setSortBy`, `setMacroLens`, `toggleCompare`
  (all in `MenuPage.tsx` / `CartProvider.tsx`). **Never** voice-places an order or pays.
- **Admin**: `accept order {id}`, `mark {id} ready|delivered`, `reject {id}`
  → `handleUpdateStatus(id, status)` (`DashboardPage.tsx`).
- **Courier** (highest *intrinsic* value — hands-busy/driving, but client-first per scope):
  `accept` / `reject` / `arrived` / `message {preset}`
  → `handleAccept` / `handleReject` / `handleSendMessage` (`TasksPage.tsx` / `DeliveryPage.tsx`).

**Security invariant:** voice dispatches ONLY existing, already-authz'd action functions — it adds
**no new endpoints** and bypasses no RLS/auth. It is a new *input device* for guarded actions, never a
new privilege path.

---

## 4. Phasing

- **Phase 0 — PoC / decision gate (spike).** Engine + worker + `whisper-base`, transcribe-only debug
  overlay on `/s/:slug`, flag-dark. Capture real-device sq/en/uk transcripts.
  **GATE:** is base `sq` usable against the menu vocabulary? → if no, schedule Phase 2 fine-tune.
- **Phase 1 — Client storefront (ships first).** `MicFab` on `MenuPage`, client intent pack, confirm
  step for `add`. Flag `VITE_VOICE_CONTROL_ENABLED=false`. i18n sq/en/uk for all voice strings.
- **Phase 2 — Albanian fine-tune (conditional).** Convert `whisper-small-sq` → ONNX, self-host on R2,
  locale-route `sq` to it. Only if Phase 0 gate failed.
- **Phase 3 — Admin intent pack** (order status).
- **Phase 4 — Courier intent pack** (accept/reject/arrived/message) — the hands-free win.

Each phase: commit → staging deploy → Playwright + unit proof (Ship Discipline). Flag stays dark
until a launch decision per role.

---

## 5. Proof strategy (Mandatory Proof Rule — deterministic, no live mic in CI)

The `SpeechProvider` is an interface with two implementations: `WhisperProvider` (real) and
`MockProvider` (emits a scripted transcript). This decouples proof from the model:

- **Unit** — `IntentMatcher` against a corpus of sq/en/uk utterances → asserts `{action,target}`
  (incl. fuzzy/typo/near-miss and below-threshold "did you mean" cases). `node --test`.
- **E2E (Playwright vs staging)** — inject `MockProvider`, "speak" a transcript, assert the resulting
  DOM/cart/filter state with `toBeVisible()` / `toContainText()`. No microphone, fully deterministic.
- **Capability** — a test that asserts graceful hide when WebGPU+WASM both unavailable and graceful
  error when mic permission denied.

---

## 6. Task-Exit enrichment (states / errors / edges / security / i18n)

- **States:** idle · requesting-mic · downloading-model(%) · listening · thinking · matched(confirm) ·
  executed · no-match("did you mean" chips) · error. Every state has copy in sq/en/uk.
- **Error matrix:** mic denied · WebGPU+WASM unavailable (hide) · model fetch fail/offline (retry) ·
  empty/low-confidence transcript (re-prompt) · ambiguous match (disambiguate, never guess-execute).
- **Edges:** noisy restaurant → **confirm-then-execute** for any stateful action; numbers/quantities
  parsed per-locale; barge-in/cancel; mixed-language utterance (use UI locale as the `language` hint).
- **Security/privacy:** zero audio/transcript egress; no recording persisted; menu vocab is public;
  admin/courier voice calls the SAME guarded handlers (no new route, no authz bypass); voice **never**
  finalizes an order or money action without explicit on-screen confirm.
- **a11y:** voice is strictly additive — keyboard/touch paths remain fully functional; respects
  reduced-motion for any mic animation; live region announces recognized intent.
- **Bundle/perf:** engine is dynamic-import-only behind flag+gesture; zero delta to storefront
  first-paint and the bundle gate.
- **i18n:** all voice strings + the per-locale command grammar added via `scripts/i18n-add.ts`
  (sq/en/uk parity gate must pass).

---

## 7. Open decisions for the council / ADR

1. **Engine home** — `packages/ui` vs a new `packages/voice` (worker + WASM asset handling under Vite).
2. **Model hosting** — self-host ONNX on R2 from day one vs HF-Hub CDN for the PoC only.
3. **Phase-0 gate threshold** — what sq match-rate on the menu corpus justifies skipping the fine-tune.
4. **Confirm policy** — which client intents auto-execute (filter/sort/search) vs require confirm
   (add-to-cart). Money/checkout: always manual, never voice.

This is a new subsystem touching a worker, a model asset pipeline, and three roles → it qualifies for
the Triadic Council (architect + breaker + counsel) + an ADR **before** Phase 1 code.
