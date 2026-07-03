# Full Voice Control of the Storefront — Design & Build Plan

> Status: DRAFT plan (design-time; no production code changes proposed here). Date: 2026-07-02.
> Goal: a customer can **find, search, and order by voice** on a storefront `/s/:slug`, and the
> assistant **replies/guides** in **Albanian (sq) / Ukrainian (uk) / English (en)**. Per-role voice
> (admin, courier) is a later, separately-gated track and is explicitly **out of active scope** here.
> Companion to `docs/adr/0015-voice-control.md`, the council set in `docs/design/voice-control/`
> (`proposal.md` · `breaker-findings.md` · `counsel-opinion.md` · `resolution.md` ·
> `ethical-decisions.md` · `ui-spec.md`), and the recap in `docs/design-review/VOICE-CONTROL-WHISPER-PLAN.md`.

This plan does **not** reinvent the engine. The safety spine, the ASR engine, the intent matcher,
and the confirmation gate already exist and are proven in `packages/voice/`. This document (a) grounds
the design in what is actually built, (b) fills the two missing pieces of a *full* voice-control loop —
**audio segmentation (VAD)** and **spoken replies (TTS)** — with concrete free/OSS choices, and (c)
phases the storefront-customer build so the smallest increment ships first, flag-dark.

---

## 0. What already exists (ground truth — do NOT rebuild)

Verified against the live tree, `packages/voice/src/*` (2026-07-02):

| Piece | File | State |
|---|---|---|
| Intent data type (pure, zero write capability) | `types.ts` — `IntentProposal`, `IntentKind`, `Capability`, `GateResult` | built |
| Fail-closed capability table (`Record<IntentKind>` exhaustiveness ratchet) | `capability-table.ts` — `classify()` | built, red→green |
| sq/en/uk dietary/allergen denylist (safety) | `dietary-denylist.ts` — `isDietaryCategory()` | built |
| Confirm-then-execute boundary (sole SINK) | `confirmation-gate.ts` — `ConfirmationGate`, `VoiceHandlers` | built, red→green |
| Deterministic intent matcher (per-locale grammar + fuzzy slot-fill) | `matcher.ts` — `matchIntent()`, `MIN_CONFIDENCE=0.6` | built, sq/en/uk corpus |
| Text normalize (NFD, strip diacritics, collapse ws) | `normalize.ts` | built |
| ASR port (object, not a callback — no write can cross in) | `transcriber.ts` — `Transcriber`, `PcmAudio` (mono 16 kHz Float32) | built |
| Real ASR (whisper-base via `@huggingface/transformers`, non-literal dynamic import → true-dark) | `transformers-transcriber.ts` — `TransformersTranscriber` | built, real-model smoke-proven |
| Voice source (audio→ASR→matcher→`IntentProposal`, holds zero handler) | `whisper-provider.ts` — `WhisperProvider` | built |
| Deterministic test double (scripted transcript→matcher→gate) | `mock-provider.ts` — `MockProvider` | built |
| Eval harness (real provider over WAV manifest → IRA + dangerous-misfire) | `scripts/audio-eval.ts` | built |

**Server control-plane already built** (`apps/api`, prior commit `8423827b`): `lib/voice-flag.ts`
`isVoiceEnabled()` = `VOICE_CONTROL_ENABLED==='true' && VOICE_KILL!=='true'` (fail-closed, single
source for both endpoint + CSP); `GET /api/public/voice-config` (SW-exempt, `no-store`); `spa-shell.ts`
widens CSP `connect-src` to the R2 model origin **only** when enabled. 4/4 tests.

**Invariants that are already law (do not weaken):**

- **Source/sink no-write closure (R2-F / M3):** the engine yields `readonly IntentProposal` data and
  holds no handler, no store dispatch, no setter. The `ConfirmationGate` (in `apps/web`) is the only
  component that can apply a proposal. Enforced by the `local/no-voice-engine-callback` ESLint rule
  (ledger #45): no function-typed parameter in any exported signature under `packages/voice/src`.
- **Fail-closed classification:** any intent kind absent from `CAPABILITY_TABLE` → `REJECT`. Money,
  checkout-write, place-order, payment, admin, courier intents have **no `IntentKind` by design**.
- **STATEFUL needs a human:** `ADD_TO_CART` is the only stateful intent; it is held `pending` and
  applies **only** on an explicit confirm tap. READ_ONLY intents auto-apply.
- **Dietary/allergen is touch-only (R2-B):** a dietary-named category match is dropped before any
  surface renders — a mis-heard "gluten free" can never narrow the menu by voice.
- **Zero audio egress:** all inference is on-device; no audio, no transcript ever leaves the tab.
- **True-dark:** the ML dep is not in the static import graph and not installed in the deploy image;
  it loads via dynamic `import()` only after flag + runtime-enable + explicit mic gesture.
- **No always-listening, no wake word** (FORBIDDEN by ADR-0015 — the anti-surveillance line).

**Active scope (operator decision 2026-06-30):** storefront customer only — menu + **checkout
READ-ONLY**. Admin (was Phase 3) and courier (was Phase 4) voice are removed from this track and
require a separate future council (they reopen worker-surveillance concerns).

---

## 1. How real voice assistants work end-to-end (research grounding)

A production voice assistant is a pipeline of narrow, swappable stages. The canonical order:

```
 (1) Activation      push-to-talk button  OR  wake word ("Hey X")
        │            → dowiz: push-to-talk ONLY (wake word is FORBIDDEN; no always-listening)
        ▼
 (2) Capture + VAD   mic → 16 kHz mono PCM → voice-activity detection segments speech from silence
        │            → decides when the utterance ended (or user releases the button)
        ▼
 (3) STT / ASR       audio segment → text transcript  (language-hinted)
        │
        ▼
 (4) NLU / intent    transcript → { intent, slots }   (item / size / qty / which action)
        │            grammar/keyword + fuzzy slot-fill for a small domain; LLM for open domain
        ▼
 (5) Dialog manager  slot-filling state machine: fill missing slots, disambiguate, CONFIRM before commit
        │            (order/money-adjacent → explicit confirm gate)
        ▼
 (6) Action          execute the (already-authz'd) app action
        │
        ▼
 (7) TTS + feedback  spoken/visual reply: acknowledge, read back, guide the next step
```

Key research takeaways that shape dowiz's choices:

- **Wake word vs push-to-talk.** OpenWakeWord (MIT) and Porcupine (free tier) make wake words cheap,
  but a wake word means *always listening* — exactly the surveillance perception ADR-0015 forbids.
  dowiz is **push-to-talk only** (explicit tap = explicit consent, and the model + mic only spin up on
  that gesture). This is a deliberate constraint, not a gap.
- **VAD is the missing capture piece.** In 2026 the three common VADs are WebRTC VAD (GMM, cheap, weak
  in noise), **Silero VAD** (deep-learning, ONNX, robust, MIT), and Cobra (Picovoice, commercial). A
  restaurant is a noisy environment, so a robust VAD matters for auto-stop — but for the **first**
  increment, push-to-talk press/release needs *no* VAD at all (the human is the endpointer).
- **Small domain ⇒ grammar beats a model.** Menu ordering is a bounded domain (tenant's dish names +
  ~20 verbs). Research on on-device NLU (micro-LMs, custom grammars, Rhasspy) is consistent: for a
  constrained command vocabulary a **deterministic grammar + fuzzy entity matching** is smaller,
  faster, fully unit-testable, and *safer* (no hallucinated intents) than an LLM. dowiz already has
  exactly this in `matcher.ts`. No new NLU dependency is warranted.
- **The transcription WER is not the bottleneck — the matcher is.** A weak `sq` transcription of
  "shto sufllaqe" still resolves if "sufllaqe" is in the tenant menu vocabulary within edit distance.
  Robustness comes from biasing resolution toward the (public) menu vocabulary, not from a bigger model.
- **TTS is the one genuinely new stage for a *full* loop.** Everything upstream of the reply exists;
  spoken replies do not. Options are the browser-native Web Speech API (`SpeechSynthesis`, free,
  zero-dep, but voice availability is user-agent-dependent) and self-hosted neural TTS (Piper VITS→ONNX,
  which *does* have Albanian and Ukrainian voices) — analysed in §2.7.

---

## 2. The dowiz storefront voice pipeline — stage by stage, with free/OSS tool per stage

Each stage names the chosen free tool, its **license**, its **on-device/privacy fit**, and whether it
is already built. The bias throughout: reuse what exists, add zero cloud dependency, add zero audio
egress, keep the ML weight out of the critical path.

### 2.1 Activation — push-to-talk (built path exists in `ui-spec.md`)
- **Choice:** a `MicFab` button; press to talk. No wake word, no idle animation (anti-surveillance).
- **Tool/license:** none — plain DOM `<button>` + `getUserMedia`. Free, native.
- **On-device fit:** perfect. Mic + model spin up **only** on the tap gesture (also the true-dark load
  trigger). First-tap shows the one-time on-device-privacy disclosure sheet (`ui-spec.md §5`).

### 2.2 Capture — Web Audio → 16 kHz mono Float32
- **Choice:** `MediaRecorder`/`AudioContext` graph, downsample to 16 kHz mono, produce `PcmAudio`
  (`Float32Array`) — exactly the `transcriber.ts` contract.
- **Tool/license:** browser-native Web Audio API. Free.
- **On-device fit:** perfect; audio never leaves the tab.

### 2.3 Endpointing / VAD — **Silero VAD** (deferred; press/release first)
- **Increment 1 needs no VAD:** push-to-talk press starts capture, release ends the utterance. The
  human is the endpointer; simplest possible, zero dependency.
- **When hands-free auto-stop is wanted:** **Silero VAD** — ONNX model, runs in-browser via
  `onnxruntime-web` (already the runtime whisper uses), **MIT** license, ~1–2 MB, robust in noise.
  Self-hosted on R2 next to the whisper weights (same asset pattern, same CSP origin).
- **License/on-device fit:** MIT, fully on-device, no egress. Alternatives rejected: WebRTC VAD (weak
  in restaurant noise), Cobra/Picovoice (commercial), OpenWakeWord/Porcupine (wake word — FORBIDDEN).

### 2.4 STT / ASR — **whisper-base via Transformers.js** (BUILT)
- **Choice:** `TransformersTranscriber` (`Xenova/whisper-base`, greedy/temp-0, language-hinted per UI
  locale). Already built and real-model smoke-proven.
- **Tool/license:** OpenAI Whisper weights + `@huggingface/transformers` — **MIT**. ~60–130 MB (q8),
  cached in IndexedDB after first load (offline thereafter).
- **On-device fit:** WebGPU → WASM, 100% on-device, **zero audio egress**. Runs in a Web Worker
  (off the main thread). Dynamic-import + dynamic model fetch only after flag+enable+gesture.
- **sq upgrade path (conditional):** `Kushtrim/whisper-small-sq` converted to ONNX, self-hosted on R2,
  locale-routed for `sq` **only if** the Phase-0 sq quality gate fails (IRA ≥85% / dangerous-misfire
  ≤2% on the menu corpus). Fast-follow, not a blocker.

### 2.5 NLU / intent — **deterministic per-locale grammar + fuzzy slot-fill** (BUILT)
- **Choice:** `matchIntent()` — per-locale trigger tables (sq/en/uk), quantity parsing, semantic args,
  `MIN_CONFIDENCE` floor, fail-safe `null` on unresolved product. No model, no network.
- **Tool/license:** first-party code, **no dependency**. This is the correct choice for a bounded
  domain per the research (§1): deterministic, unit-testable, no hallucinated intents.
- **On-device fit:** pure text function; runs anywhere, instantly.
- **Optional hardening (no new dep needed):** the current `resolveByName` token-overlap fuzzy match is
  adequate; if edit-distance robustness is later wanted, a tiny in-house Levenshtein beats adding
  Fuse.js (YAGNI / no-new-dep). Menu vocabulary is language-neutral (the tenant's real dish names),
  matched after `normalize()` — so it works across all three locales.

### 2.6 Dialog / confirm — **`ConfirmationGate` + slot-filling chips** (gate BUILT; multi-turn is new UI)
- **Choice:** READ_ONLY intents (search/sort/filter/macro/compare/read-order/go-checkout) auto-apply;
  the one STATEFUL intent (`ADD_TO_CART`) is held pending and applies only on an explicit confirm tap.
  Ambiguous matches → disambiguation chips; low confidence → "didn't catch that" re-prompt. All in
  `ui-spec.md §2–3`.
- **Tool/license:** first-party. Free.
- **On-device fit:** pure UI state; nothing leaves the tab. **This is the money-safety spine** —
  see §4.

### 2.7 TTS / spoken reply — **Web Speech API first, Piper as the guaranteed-voice fast-follow**
This is the genuinely new stage for a *full* control loop (the built engine is input-only). Two tiers:

- **Tier A — Web Speech API `SpeechSynthesis` (ship first).**
  - License: browser-native, **free**, zero dependency, zero bundle weight.
  - On-device fit: on-device on most platforms, but **voice availability is user-agent-dependent**.
    `getVoices()` reliably has `en`; `uk` is common on Chrome/Edge/Android; **`sq` (Albanian) is
    frequently absent**. Because a voice may be missing, this stage must **degrade gracefully to the
    existing text/`aria-live` read-back** (which is already the accessibility surface — `ui-spec.md §4`,
    `§7`) when no locale voice exists. Never block a reply on TTS.
  - Privacy note: modern desktop voices are on-device, but historically some `SpeechSynthesis`
    implementations synthesized server-side. We treat TTS output as **non-sensitive** (it speaks only
    the public menu / the user's own cart, never PII), and prefer voices where `voice.localService`
    is true; still, keep TTS strictly optional and off by default.
- **Tier B — Piper (VITS→ONNX) self-hosted on R2 (the guaranteed on-device sq/uk voice).**
  - Why: Piper **has both Ukrainian and Albanian voices** (unlike Web Speech, where sq is usually
    missing). Runs in-browser via `onnxruntime-web` (same runtime as whisper), 100% on-device, offline
    after first fetch. ~20–60 MB per voice — so it is **gated behind an explicit "read aloud" opt-in**
    and lazy-loaded, never on first paint.
  - **License caution (must be checked before shipping):** the original `rhasspy/piper` runtime was
    MIT but was archived (Oct 2025); active development moved to `OHF-Voice/piper1-gpl` which is
    **GPL-3.0**. To stay MIT-clean in a shipped web bundle, use an **MIT-era inference path** (the
    ONNX model + a permissive `onnxruntime-web` inference wrapper, e.g. a `piper.wasm`/`sherpa-onnx`
    style runner) and treat individual voice weights per their own license on `rhasspy/piper-voices`.
    Do **not** bundle the GPL-3.0 fork. This is an open decision (§9).
  - Rejected TTS alternative: **Coqui XTTS-v2** — its model is **Coqui Public Model License
    (non-commercial)**, disqualifying for a commercial storefront. Coqui toolkit is MPL-2.0 but the
    good multilingual weights are CPML → eliminated.

**TTS summary:** ship Web Speech `SpeechSynthesis` as the zero-cost reply for en/uk, always with a
text fallback (which is the sq path until Piper lands and the general a11y path). Add self-hosted Piper
sq/uk voices as an opt-in fast-follow when a guaranteed neural voice is required.

### Pipeline-at-a-glance (free tool + license + on-device per stage)

| Stage | Tool | License | On-device | Status |
|---|---|---|---|---|
| Activation | push-to-talk `<button>` + `getUserMedia` | native | yes | design (ui-spec) |
| Capture | Web Audio (16 kHz mono Float32) | native | yes (no egress) | design |
| VAD (auto-stop) | Silero VAD (onnxruntime-web) | MIT | yes | deferred (not needed for PTT) |
| STT | whisper-base / Transformers.js | MIT | yes (WebGPU→WASM) | **built** |
| STT sq upgrade | whisper-small-sq → ONNX (R2) | MIT | yes | conditional fast-follow |
| NLU | first-party grammar + fuzzy slot-fill | (in-house) | yes | **built** |
| Dialog/confirm | `ConfirmationGate` + chips | (in-house) | yes | gate built; multi-turn UI new |
| TTS (A) | Web Speech `SpeechSynthesis` | native | mostly (voice-dependent) | new, small |
| TTS (B) | Piper VITS→ONNX (R2), sq+uk voices | MIT-era runtime + per-voice | yes | fast-follow, opt-in |

---

## 3. The storefront voice flow (find → search → add → adjust → checkout)

The reference journey, mapped to built intents (`matcher.ts`) and the gate (`confirmation-gate.ts`):

1. **Find / search a dish** — "search pizza" / "kërko sufllaqe" / "знайти піцу"
   → `SET_SEARCH { query }` (READ_ONLY) → auto-apply → menu filters → optional spoken/text reply
   "Showing results for pizza." **This is the smallest first increment (§7).**
2. **Refine the view** — "sort cheapest" / "show macros protein" / "compare X and Y"
   → `SET_SORT` / `SET_MACRO_LENS` / `TOGGLE_COMPARE` (all READ_ONLY, auto-apply, reversible via
   toast Undo).
3. **Add to cart** — "add two sufllaqe" / "shto dy sufllaqe"
   → `ADD_TO_CART { productId, qty }` (STATEFUL) → **confirmation chip** "Add 2× Sufllaqe? [Cancel]
   [Confirm]" → applies only on confirm tap (`ui-spec.md §3`).
4. **Adjust** — change quantity / remove: expressed as another `ADD_TO_CART` (re-confirm) or handled by
   touch. Cart-line removal by voice is **not** in the active intent set (touch-only) — deliberately
   conservative; can be added later as a confirm-gated `REMOVE_FROM_CART` intent.
5. **Read back** — "read my order" → `READ_ORDER` (READ_ONLY) → reads the user's own cart lines +
   total via `aria-live` + optional TTS. No new network call, no PII egress.
6. **Go to checkout** — "go to checkout" → `NAVIGATE_CHECKOUT` (READ_ONLY, navigation only) → opens
   the checkout sheet. **No field entry by voice** (address/phone/notes are touch-only), **no
   place-order by voice, no payment by voice** — those have no voice grammar by construction.

---

## 4. Confirm-before-commit (the order/money safety spine)

Voice-placed orders are order/money-adjacent, so commitment is explicitly gated. This is already
enforced in code and must not be relaxed:

- **READ_ONLY (safe, reversible) → auto-apply + Undo.** Search, sort, macro-lens, category (non-
  dietary), compare, read-order, navigate-checkout. Reversibility (toast Undo) is what licenses
  auto-apply.
- **STATEFUL (cart write) → explicit confirm chip.** `ADD_TO_CART` never applies without a human tap.
  The chip echoes the *parsed* `{qty, item}` before any write. **Equal affordance weight** for
  Confirm/Cancel (no bright-CTA-vs-grey-ghost dark pattern — C-2/STOP-2). Timeout / Esc / outside-tap
  = **Cancel** (a safety surface defaults to *not acting*). Confirm runs the handler exactly once
  (consume-once idempotency).
- **Money / place-order / payment / checkout-field-write → no voice grammar at all.** No intent kind,
  no handler, no chip. `classify()` returns `REJECT` for anything outside the table. Voice can
  *navigate to* checkout; a human completes the transaction by touch/keyboard.
- **Dietary/allergen → touch-only.** Dropped before any surface renders (a spoken "gluten free" is a
  trusted safety assertion off a noisy channel — never honored by voice).

**Why this is safe by construction, not by discipline:** the engine holds no handler (source/sink
closure, ESLint-enforced); the gate is the only sink; the capability table is exhaustive and fail-
closed; money/dietary simply have no representable intent. A mis-hear degrades to no-op or a cancel,
never to a wrong order.

---

## 5. The three-language strategy (sq / uk / en)

One pipeline, locale-routed at three points:

- **STT:** a single multilingual `whisper-base`, `language` hinted by the active UI locale
  (`WHISPER_LANG` in `transformers-transcriber.ts`). One model download covers all three. sq is the
  default locale and the weakest on base whisper → the conditional `whisper-small-sq` fine-tune
  (§2.4) is the sq quality lever.
- **NLU:** per-locale trigger tables already in `matcher.ts` (sq/en/uk verbs). The **entity vocabulary
  is language-neutral** — it is the tenant's real dish/category names, matched after `normalize()`
  (NFD + strip diacritics), so "qumësht" ≈ "qumesht" and Cyrillic/Latin both normalize. **Known honest
  gap:** a Ukrainian utterance naming a Latin-script menu item may fail the product slot → resolves to
  `null` (fail-safe: does nothing, never a wrong item). Documented, acceptable, revisitable via a
  transliteration pass.
- **TTS:** locale-routed reply. Web Speech picks a voice matching the locale if `getVoices()` has one
  (en reliable, uk common, **sq usually absent**); when absent, **fall back to the text/`aria-live`
  read-back** (already the a11y surface). Piper sq+uk voices (fast-follow) give a guaranteed neural
  voice for the two locales Web Speech under-serves.
- **i18n:** every voice UI string is added via `scripts/i18n-add.ts` (sq/en/uk together), parity-gated
  by `scripts/i18n-parity`. A locale that has not passed its own dangerous-misfire bound ships **dark
  even though the strings exist** (per-locale launch gate).

---

## 6. Reduced friction + accessibility

- **Voice is strictly additive.** Every voice action has a pre-existing touch/keyboard equivalent;
  voice removes no path. If voice is unsupported (no WebGPU, insecure context, mic denied) the MicFab
  is simply **absent** (never a greyed disabled control that invites support load).
- **One gesture, no wake word, no idle animation** — lowest cognitive load and no "always listening"
  anxiety.
- **Every state has a visual + an `aria-live` announcement** (listening / transcribing / proposal /
  applied / error). Errors are `aria-live="assertive"`. Focus moves to the confirm chip when it
  appears and returns to the MicFab on dismiss. `prefers-reduced-motion` replaces the listening pulse
  with a static ring + "Listening…" label.
- **TTS earns real accessibility value** for low-vision / hands-busy users (spoken search results,
  spoken cart read-back) — but only as an **opt-in**, never forced, always with the text mirror.
- **No dead-ends:** every error state offers a recovery affordance and leaves touch fully working
  (`ui-spec.md §2` error matrix).

---

## 7. Phasing — storefront-customer-first, built on `packages/voice`, flag-dark

All phases: `commit → staging deploy → Playwright + unit proof` (Ship Discipline), flag
`VITE_VOICE_CONTROL_ENABLED=false` until a per-locale launch decision. Nothing ships to prod without
explicit approval.

- **Phase A — Smallest increment: push-to-talk menu SEARCH, read-only (see §8).**
  Wire the built engine to the storefront for exactly one READ_ONLY intent (`SET_SEARCH`). No cart, no
  money, no confirm chip, no VAD, no TTS. Proves the whole capture→STT→matcher→apply path on real
  devices and yields the first sq/en/uk transcript quality data (the Phase-0 gate corpus).
- **Phase B — The rest of the READ_ONLY intents.** Sort, macro-lens, category (non-dietary), compare,
  read-order, navigate-checkout. Auto-apply + toast Undo. Still no cart write, no money.
- **Phase C — The one STATEFUL intent: `ADD_TO_CART` with the confirmation chip.** Introduces the
  confirm-before-commit surface (§4). This is the first cart-write path; it is the highest-scrutiny
  phase and is where the equal-weight-affordance CI assertion lands.
- **Phase D — Spoken replies (TTS Tier A).** Web Speech `SpeechSynthesis` for en/uk with text
  fallback; opt-in, off by default. Turns the loop from voice-*input* into a two-way assistant.
- **Phase E — Guaranteed sq/uk voice + hands-free (fast-follows, conditional).** Piper sq+uk voices
  (Tier B) if a guaranteed neural voice is needed; Silero VAD for hands-free auto-stop; `whisper-
  small-sq` if the sq quality gate failed.

Per-role voice (admin/courier) is **not** in this plan — separate future council.

Gating that still blocks live customer traffic (from the council, unchanged): the ML dep is
deliberately not yet in the deploy image (deploy-weight decision — flag to operator before Phase A
ships the real engine); **R-J demand-evidence** and the **C2-consented launch corpus** (R-I) gate the
public launch; the sq quality GO/NO-GO probe needs a human speaking Albanian into a mic (not
executable headless).

---

## 8. The smallest first increment (recommended start)

**Push-to-talk menu SEARCH, read-only.** Concretely:

- A `MicFab` on `MenuPage` (per `ui-spec.md §1`), render-predicate-gated (flag + runtime-enable +
  WebGPU + secure context + user pref).
- Press to talk → capture 16 kHz mono → `TransformersTranscriber` (whisper-base, in a Web Worker) →
  `matchIntent()` → **accept only `SET_SEARCH`** for this increment (everything else is a no-op
  "didn't catch that").
- `SET_SEARCH` is READ_ONLY → `ConfirmationGate` auto-applies → calls the existing `MenuPage` search
  setter → the menu list visibly filters.

**Why this first:**
- **Zero money/cart risk** — READ_ONLY only; no confirm chip, no cart write, no checkout.
- **Exercises the entire input pipeline** end-to-end (mic → STT → matcher → gate → DOM), so it
  validates the hardest integration (the Web Worker + WebGPU + model fetch) with the lowest blast
  radius.
- **Reuses a fully-built, already-tested intent** (`SET_SEARCH` in `matcher.ts`) and the built gate —
  the only genuinely new code is the browser capture/worker glue + the `MicFab` UI.
- **Produces the Phase-0 quality corpus** (real sq/en/uk transcripts against real menu vocabulary),
  which is the evidence the sq fine-tune decision and the launch gate both need.
- **Highest-frequency discovery action** — search is what a customer reaches for first on a long menu,
  so it delivers real friction reduction immediately.

**Proof (Mandatory Proof Rule):** unit — matcher `SET_SEARCH` corpus (built); E2E — inject
`MockProvider` on staging, "speak" a search transcript, assert the filtered list via `toBeVisible()` /
`toContainText()` (no live mic in CI); capability — assert graceful absence when WebGPU unavailable and
graceful error when mic denied.

---

## 9. Open decisions (for the council / ADR follow-up)

1. **ML dep in the deploy image** — `@huggingface/transformers` + onnxruntime is heavy; confirm the
   deploy-weight tradeoff with the operator before Phase A ships the real engine (until then, the
   `MockProvider` E2E path proves everything except live transcription).
2. **Piper license path** — pin an MIT-era inference runtime + permissively-licensed sq/uk voice
   weights; explicitly avoid the GPL-3.0 `piper1-gpl` fork in the shipped bundle. Verify each voice
   weight's license on `rhasspy/piper-voices` before use.
3. **TTS default** — Web Speech first (zero-dep) with text fallback; decide the threshold at which the
   guaranteed-voice Piper tier is worth its per-voice download.
4. **sq quality gate threshold** — the menu-corpus match-rate (IRA ≥85% / dangerous-misfire ≤2%) that
   justifies skipping `whisper-small-sq`; needs the Phase-A real-device corpus + a human sq speaker.
5. **VAD adoption** — whether/when to add Silero VAD for hands-free auto-stop, or keep push-to-talk
   press/release indefinitely (simplest, and arguably the clearest consent model).

---

## Sources (research)

- Web Speech API / SpeechSynthesis browser + voice availability:
  [MDN Web Speech API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Speech_API) ·
  [Speech Synthesis API browser support (TestMu)](https://www.testmuai.com/learning-hub/speech-synthesis-api-browser-support/) ·
  [web-speech-recommended-voices](https://github.com/HadrienGardeur/web-speech-recommended-voices)
- TTS engines & licenses:
  [Local TTS & voice-cloning licenses 2026 (Piper/XTTS/Coqui)](https://www.promptquorum.com/power-local-llm/local-tts-voice-cloning-piper-coqui-xtts) ·
  [Coqui vs Piper](https://anvevoice.app/faq/coqui-tts-vs-piper-tts-for-open-source-voice) ·
  [Piper VOICES.md (rhasspy)](https://github.com/rhasspy/piper/blob/master/VOICES.md) ·
  [Piper voice samples](https://rhasspy.github.io/piper-samples/) ·
  [rhasspy/piper-voices (HF)](https://huggingface.co/rhasspy/piper-voices)
- VAD / wake word:
  [Best VAD 2026: Cobra vs Silero vs WebRTC (Picovoice)](https://picovoice.ai/blog/best-voice-activity-detection-vad/) ·
  [RealtimeSTT (VAD + wake word pipeline)](https://github.com/KoljaB/RealtimeSTT) ·
  [openWakeWord topic](https://github.com/topics/openwakeword)
- On-device NLU / grammar:
  [Rhasspy intent recognition](https://rhasspy.readthedocs.io/en/latest/intent-recognition/) ·
  [Sensory micro-LMs & custom grammars](https://sensory.com/product/micro-language-and-custom-grammar-models/)
