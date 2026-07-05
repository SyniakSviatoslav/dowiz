# ADR-0015 — Voice Control (on-device Whisper, intent-not-dictation)

- **Status:** PROPOSED (design-time; no production code). Date: 2026-06-30.
- **Slug:** `voice-control`
- **Design doc:** `docs/design/voice-control/proposal.md`
- **Research context:** `docs/design-review/VOICE-CONTROL-WHISPER-PLAN.md` (informal draft).
- **Relates to / does not contradict:** ADR-001 (monolith-first — voice adds no service),
  ADR-0010 (error-envelope — voice adds no new endpoint to envelope), ADR-0014
  (menu-characteristics — voice reuses its setters), ADR-p0-privacy-hardening / claim-check
  (no PII to AI), the dark-flag pattern (`Dockerfile:20-36`), the DB connection budget
  (`packages/config/src/index.ts:7-9`).

## Context

We want hands-busy / low-friction interaction on the **client-facing storefront** (`/s/:slug/*`,
`apps/web/src/main.tsx:49`) — **menu + checkout (read-only)**. **Scope (operator 2026-06-30): the
active build is the client-facing flow only; admin and courier voice are REMOVED from this build and
deferred to a separate future council.** Albanian (`sq`) is the default and hardest
ASR target; `en`/`uk` are first-class. Hard rules are pre-baked: **voice never finalizes an order
or any money/stateful action** (confirm-then-execute for anything stateful), and proof must be
**deterministic** (a scripted `MockProvider` for Playwright + a unit corpus for the intent
matcher). Audio is PII. Web Speech API streams audio to Google (subprocessor + GDPR, Chrome/Edge
only) and Vosk has no Albanian model → both eliminated. The product is *command matching against a
closed vocabulary*, not dictation — so the **intent matcher**, not raw WER, governs usability.

## Decision

Build voice as an **on-device, client-side input layer** that emits **immutable intent proposals
into a confirmation gate which holds all write capability**, in flag-dark phases.

1. **Engine home →** a **new `packages/voice`** package (Web Worker engine, audio capture +
   capability detect, IntentMatcher, grammar types, the `SpeechProvider` interface and
   `MockProvider`). It imports **no** app store / API client → the "zero write capability"
   invariant is structural. MicFab presentation + voice strings live in `packages/ui` (existing
   key-major i18n catalog, `scripts/i18n-add.ts`, sq/en/uk parity gate). Per-role **intent packs**
   live in `apps/web` (they bind to existing handlers, so cannot be shared). *Deliberate exception
   to "edit don't create": a quarantine boundary for a heavy optional subsystem is the design.*

2. **Transcription runtime →** **client-side ONNX/WASM in a Web Worker** (Whisper via
   Transformers.js / ONNX Runtime Web, MIT, WebGPU → WASM, dynamic-import-only). **Zero audio
   egress.** Server-side transcription is **rejected** (no GPU; CPU can't keep up; audio-PII
   egress; new worker role → pool pressure).

3. **Model delivery →** **self-host the ONNX weights on R2 from day one** (reuse the
   `dowiz-images` pattern; $0 egress; version-pinned; first-party CSP). HF CDN is allowed **only**
   for an un-deployed local capability probe, never on a deployed route.

4. **Phase-0 GO/NO-GO (per locale, on confidence bounds — RESOLVE M1/H3/C2/M6) →** measured
   through the full `transcribe → IntentMatcher` pipeline on a **≥300-utterance-per-locale**,
   **≥15-speaker** corpus that is a **separate consented research artifact (own RoPA/retention —
   §8.1 of the proposal), NOT a storefront collection** (resolves C2): **GO** if IRA **lower-95%-CI
   ≥ 80% AND point ≥ 85%** AND **dangerous-misfire upper-95%-CI (Wilson) ≤ 2%** (a one-sided safety
   bound — n=150 was under-powered); **NO-GO** if IRA point < 70%; **AMBER** in between → tune the
   matcher with **runtime per-tenant** vocabulary biasing (no shipped static demo-vocab bias — M6)
   and re-measure. `sq`/`en`/`uk` are each gated **independently** (string parity ≠ safety parity —
   H3). The demo-menu result validates **pipeline mechanics, not per-tenant launch** — broad
   multi-tenant launch needs a multi-menu re-measurement (deferred). Raw WER + measured
   WebGPU-availability rate are reported, not gated on.

5. **Intent→action safety →** the voice layer is a **pure producer of `IntentProposal` values with
   zero write capability**, with **control inverted so no handler crosses the boundary** — the
   engine is a source emitting `readonly` data over a typed **`AsyncIterable<IntentProposal>`**
   (pulled via `for await`; **not** an `on(event,cb)`/`subscribe({onResult})` callback surface —
   R2-F), the gate is the sink that *pulls* it; the engine's public API takes **no
   callback/function-typed param** (closes the runtime-callback bypass — M3; the iterator makes the
   API and the no-function-param guardrail coherent).
   A single **`ConfirmationGate`** classifies each proposal via a **total, exhaustive capability
   table whose `default` REJECTS (fail-closed; unknown/forgotten kind → dropped, never executed —
   M4)**: `READ_ONLY` (filter/sort/search/macro/compare **+ the two checkout-read intents — "read my
   order" reading the user's own client-side cart + total, and "go to checkout" navigation — no
   field write, no PII egress, no cross-tenant surface; scope narrowing 2026-06-30** — UI-local
   reversible, **no safety assertion**, may auto-apply; **but a category whose name matches the
   sq/en/uk dietary/allergen token denylist is NOT voice-auto-appliable — touch-only — closing the
   class not just the setter, R2-B**) vs `STATEFUL` (**add-to-cart — the only STATEFUL intent in
   active scope**; admin/courier stateful intents are REMOVED from this build — confirm affordance;
   handler runs only on an explicit human tap). **Excluded from the table entirely (no
   `kind`, by construction):** money / checkout writes / place-order / payment / order-finalization
   (no checkout field — address/phone/notes — is voiceable; checkout adds **zero** stateful/money
   voice grammar), **allergen/dietary-safety filters
   (`setFilterAllergen` — a voice-set safety filter is a false safety assertion off a noisy channel,
   C1)**, and **cash-settling courier actions (`arrived`/`completeDelivery` settle cash per
   ADR-0009/0013 — H4).** **No voice→money binding in Phase 0/1/2** (the unqualified "ever" was an
   overclaim — Phases 3/4 are a separate scope, NOT approved here, each needing its own gate).
   Enforced by guardrails (red→green): eslint-local/grep (voice imports no mutator/API
   client/status handler **and exports no function-typed param**), a gate unit test (STATEFUL
   without confirm → not called; **unknown kind → reject**), a `never`-exhaustiveness build check,
   a table test (no `place_order`/`pay`/`checkout`, **no allergen/dietary kind +
   `setFilterAllergen` not voice-reachable, no settling kind**), **a category-denylist test
   (dietary-named category not auto-applied — R2-B), an actor-anonymous telemetry-schema test (no
   `courier_id`/`user_id`/latency column — Counsel C-1), a fail-closed control-plane test
   (`/api/public/voice-config` failure ⇒ voice OFF — R2-A), and the debug-overlay-absence +
   public-deploy-arg guardrails (zero-egress on public builds machine-checked — R2-C).**

**Data / migrations:** **none** for Phase 0/1/2 production runtime (model is a static R2 GET;
transcription + matching are client-side; voice dispatches existing already-authz'd handlers —
`addItem` `CartProvider.tsx:89`, the menu setters `MenuPage.tsx:211-240` **excluding
`setFilterAllergen`**, non-settling admin/courier handlers). The **Phase-0 research corpus is a
separate consented regime** (own RoPA/retention/deletion, encrypted off-tenant store, no RLS
surface — resolves C2), **not** a production data store. Its 90-day deletion is **enforced
automatically** by a scheduled deletion job + an expiry-audit tripwire (fails `compliance-gate`) +
a deletion-proof log — **not by memory** (R2-D; RLS accepted as the wrong tool for a non-tenant
artifact). Real-device recording is gated on named **consent conditions** — non-coercive
recruitment (explicitly **not** the platform's own couriers/workforce), fair pay, withdrawal right,
protocol-scoped consent, vulnerable-population safeguard (Counsel C-4); the cheap laptop probe and
the engine build are not gated by this. Any future opt-in telemetry would be a
separate RED-LINE forward-only migration, `ENABLE+FORCE` RLS, **zero transcript text**, default dark.

**CSP + single source of truth (RESOLVE L1 / R2-E):** the live storefront `connect-src`
(`apps/api/src/lib/spa-shell.ts:150`, built **server-side per request**) has **no R2 origin** → a
model `fetch()` is currently blocked. The launch flag `VITE_VOICE_CONTROL_ENABLED` is a **client**
build-arg invisible to the server, so the CSP cannot gate on it (R2-E). **Single authority:** a
server env `VOICE_CONTROL_ENABLED` (+ `VOICE_KILL`) read at request time drives **both** the
`connect-src` R2 widening (`r2ConnectSrc` from `R2_PUBLIC_URL`, like `r2ImgSrc`) **and** the runtime
config endpoint, via the **same predicate** `VOICE_CONTROL_ENABLED && !VOICE_KILL` → a hot-kill also
closes the R2 origin. One per-environment deploy variable fans out to both the client build-arg and
the server env so they never desync; the hardened policy is not widened while dark.

**Kill-switch (RESOLVE M5 / R2-A — the cache-first SW defeats the bootstrap-piggyback):**
`apps/api/public/sw.js` serves every non-`/api/` GET **cache-first** (HTTP cache semantics ignored),
so a kill ridden on the menu/info bootstrap reaches returning visitors only on a redeploy — fail-open
for the crash population. **Fix:** the build-arg `VITE_VOICE_CONTROL_ENABLED` is the true-dark
default **plus** a **server-served runtime hot-kill** read from a **dedicated `/api/`-prefixed,
SW-exempt control-plane GET** (`GET /api/public/voice-config?slug=…`, `cache:'no-store'`) gating
MicFab + engine dynamic-import + model fetch. **Fail-closed:** fetch reject / `!res.ok` /
`enabled !== true` ⇒ voice OFF (absence = OFF). Runtime-instant disable without a rebuild, **instant
for returning visitors too**. **Honest cost:** this is a **new minimal read-only endpoint** (the "no
new endpoint" claim is withdrawn — it is not an audio/PII/write surface, so "no new RLS/auth/write
surface" still holds) and the voice path now has **two** external calls (the fail-closed config GET +
the R2 model GET), not one — there is still no audio/transcription server.

**Phasing (active scope = client-facing flow only — operator 2026-06-30):** 0 PoC (gate) → 1 client
storefront, **menu + checkout-read-only** (demo/pilot tenant first) → 2 conditional fine-tune.
**Phases 3 (admin) and 4 (courier) are REMOVED from this build and deferred to a SEPARATE FUTURE
COUNCIL** — not approved here and not part of the active roadmap; the constraints recorded for them
(own admin/courier dangerous-misfire gate; cash-settling actions excluded by construction; STOP-1
human decision) travel forward to that future council. Each in-scope phase: commit → staging →
Playwright (`MockProvider`, proves wiring+gate only) +
unit (text corpus, matcher only) + the bundle/true-dark guardrail; the **real-audio→matcher
integration harness** (real `WhisperProvider`, pinned model, temp-0) is the §4.2 gate artifact and
runs in a self-hosted lane, **not** cloud CI — **green CI ≠ IRA passed** (resolves H1/L2). Flag-dark
per role (`VITE_VOICE_CONTROL_ENABLED=false`).

## Consequences

**Positive.**
- **Server-side scaling impact ≈ nil:** client-side transcription adds **0 DB connections, 0
  worker load, 0 new endpoints** — only a static $0-egress R2 GET (back-of-envelope §2.4).
- **Strong reliability property (narrowed honestly — R2-A):** the voice path has **two** external
  calls — a fail-closed `/api/public/voice-config` control-plane GET and the R2 model GET — and
  **still no audio/transcription server to fall over.** The control-plane call's only failure mode
  is "voice OFF" (never a page block or cascade).
- **Privacy by construction, now machine-checked:** no audio/transcript egress or persistence → no
  new subprocessor / RoPA entry for the runtime; satisfies "nul PII у ШІ" / claim-check. The
  zero-egress invariant is enforced by **guardrail** on public builds (debug-overlay bundle-absence
  + public-deploy-arg assertion — R2-C), and the telemetry counter is actor-anonymous **by test**
  (no `courier_id`/`user_id`/latency — Counsel C-1), not by prose.
- **Safety by construction:** the engine cannot mutate anything (DI boundary + inverted control, no
  injected callback — M3); the gate is **fail-closed** (unknown kind rejected — M4); money,
  allergen/dietary filters, and cash-settling courier actions have **no voice binding** in Phase
  0/1/2; confirm-then-execute is a guardrail, not a convention.
- **True dark when off:** engine never imported, model never fetched → zero storefront
  critical-path / bundle-gate delta; instant flag rollback with zero data change.
- **Narrowed scope = smaller blast radius (operator 2026-06-30):** removing admin/courier voice from
  this build drops the highest-risk surfaces (worker-mic labour-surveillance gradient, confirm-fatigue
  on in-grammar STATEFUL admin/courier actions) from the active design. The active capability table
  holds **add-to-cart** (the only STATEFUL intent) plus READ_ONLY menu + checkout-read intents;
  checkout adds **zero** stateful/money voice grammar. No new attack surface vs the prior design — the
  two checkout intents are READ_ONLY, non-safety-read, non-money, and read only the user's own
  client-side cart.

**Negative / costs.**
- **Voice is WebGPU-gated on the storefront (RESOLVE H2/M2):** WASM threads need COOP/COEP, which
  would break existing cross-origin map tiles/fonts/Plausible/OSRM → **we will NOT enable it** →
  WASM-single-thread is not offered. So no-WebGPU (iOS Safari, most social in-app WebViews) = **mic
  hidden** for much of real traffic — an accepted reach cost; the **accessibility framing is dropped
  for Phase 0/1** ("convenience for capable devices" — Counsel). Realistic peak memory with the menu
  open + GPU copy ≈ **500–700 MB**; the capability floor is a **measured warmup probe**, not
  `deviceMemory`.
- Adds a heavy optional dep (`@huggingface/transformers` + `onnxruntime-web`) — quarantined,
  dynamic-import-only, pinned, scanned. The model is **mixed-precision quantized (~130 MB measured
  target), not pure-q8** (L3).
- Base quality is unmeasured → Phase 0 is a real per-locale gate, not a formality; the gate requires
  a consented research corpus with its own data-governance (C2). Phase 2 fine-tune is contingent
  owner/eng work (ONNX conversion + R2 hosting + a second locale-keyed model).
- One-time R2 model-hosting + a scoped, flag-gated CSP `connect-src` widening (`spa-shell.ts:150`)
  before the first deployed phase.
- **No demand evidence yet (Counsel §5):** feasibility is rigorously gated, *desire* is unmeasured →
  a human product decision (demand signal or explicit speculative acceptance, ranked vs the open
  launch-blockers B1/B2/B3) is required before any Phase-1 code.

## Alternatives considered

- **Server-side transcription worker** — rejected: no GPU, CPU can't keep up, audio-PII egress,
  new worker role → pool pressure. (Revisiting it = a new ADR.)
- **Web Speech API** — rejected: streams audio to Google (subprocessor + GDPR disclosure),
  Chrome/Edge-only.
- **Vosk** — rejected: no Albanian model (sq is the default locale).
- **Engine in `packages/ui`** — rejected: pollutes a 99.9th-%ile-churn package and leaks a heavy
  optional dep + worker/WASM build config into every UI consumer; can't structurally guarantee
  zero write capability.
- **HF CDN for the PoC** — rejected on deployed routes: no version pin (would silently invalidate
  the Phase-0 gate measurement) + widens the storefront CSP to a third-party origin.
- **Raw WER as the Phase-0 gate** — rejected: the closed-vocabulary intent matcher, not dictation
  accuracy, determines usability; IRA + dangerous-misfire is the correct gate.
- **Auto-execute stateful voice intents** — rejected: confirm-then-execute is a hard rule; money,
  allergen/dietary filters, and cash-settling actions have no voice binding at all.
- **Do NOT build voice now — text-first, ship the launch-blockers (Counsel steel-man §4)** —
  **not rejected, recorded as the binding sequencing constraint:** Albanian food-delivery's binding
  constraint is trust/payment-correctness/dispatch (the open NO-GO B1/B2/B3), not input friction.
  Phase-0 stays a cheap laptop probe; **no Phase-1 engineering** until a human records a demand
  signal (or explicit speculative acceptance) **and** the launch-blockers are addressed. The
  cleanest design may be the feature not yet built; this ADR designs it dark so it is *ready*, not
  *prioritised*.
- **Voice for the WebGPU-excluded via the rejected server path (Counsel steel-man §4)** — the
  client-side choice optimises privacy + infra **at the cost of equity-of-reach** (it routes around
  the cheap-phone / elderly user); that trade was **made, not discovered**, and is recorded (R-G/R-L).

## Status

**SCOPE NARROWED (operator 2026-06-30):** active build = **client-facing flow only (storefront menu
+ checkout READ_ONLY)**. **Phase 3 (admin) and Phase 4 (courier) are REMOVED from the active roadmap
and deferred to a separate future council** — not part of this build. Consequently STOP-1
(worker/courier voice surveillance) is **out of active scope** (re-opens only if that future council
takes up admin/courier voice); the Phase-3/4 admin/courier dangerous-misfire defer-flags travel to
that future council. The storefront gates — R-J demand evidence (still gates Phase-1 code) and
R-I/C-4 corpus consent (still gates real-device recording) — are **unchanged**. This narrowing
introduces **no new attack surface**: the two checkout intents are READ_ONLY, non-safety-read,
non-money, reading only the user's own client-side cart; removing admin/courier reduces surface.

**PROPOSED — RESOLVE round 2 applied (pending Breaker + Counsel FINAL re-attack).** Breaker
(`breaker-findings.md`, rounds 1+2) and Counsel (`counsel-opinion.md`, rounds 1+2) findings are
dispositioned in `docs/design/voice-control/resolution.md` (round 1: C1/C2, H1–H4, M1–M6, L1–L3 +
STOP-1/2 + accessibility + demand; round 2: R2-A..R2-F + Counsel C-1..C-4) and folded into the
proposal + this ADR. **Fixed (R1):** C1, C2, H1, H2-honesty, H3, H4-claim, M1–M6, L1–L3, STOP-2,
accessibility-relabel. **Fixed (R2):** R2-A (SW-exempt fail-closed control-plane hot-kill), R2-C
(debug-overlay zero-egress guardrail), R2-B (category-class denylist), R2-E (single server flag
authority), R2-F (async-iterator surface), Counsel C-1 (telemetry-schema guardrail now) / C-2
(decline visually-equal) / C-3 (WebGPU rate a required gate field) / C-4 (corpus consent
conditions). **Accept-risk:** H2-reach, Phase-0 demand probe, R2-A extra control-plane round-trip,
R2-B safety-implying name evading the denylist (counted in dangerous-misfire), R2-D corpus
outside-RLS (research artifact — controller + scheduled-deletion job + audit tripwire instead).
**Defer-flag (MISSING):** Phase-3/4 admin/courier gate thresholds, multi-tenant accuracy
validation, accessibility-earn requirements (no new deferrals in round 2). **NEEDS-HUMAN-DECISION
(recorded, does not block Phase 0/1/2):** STOP-1 worker-voice entry, demand evidence, the corpus
data-controller assignment (now carrying the C-4 recruitment-ethics conditions). **Specified as
required exit criteria, NOT built** (design-time, no production code): the R2-A/R2-C guardrails +
the `/api/public/voice-config` endpoint + the single-flag CSP + the corpus deletion job.

ADR sign-off + the human demand decision + the launch-blockers (B1/B2/B3) are required **before
Phase-1 code**. Phase-0 may spike behind `VITE_VOICE_TRANSCRIBE_DEBUG` **only in the consented
research session (§8.1), never on a public deployed `/s/:slug`** — to produce the §4.2 gate
measurement.
