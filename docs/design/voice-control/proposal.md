# Voice Control — Design Proposal

> Status: PROPOSED, **RESOLVE round applied** (design-time; NO production code). Date: 2026-06-30.
> ADR: `docs/adr/0015-voice-control.md`. Supersedes the informal draft
> `docs/design-review/VOICE-CONTROL-WHISPER-PLAN.md` (kept as research context).
> Author register: System Architect — truth of engineering (does it work / scale / hold).
> Adversarial Breaker (`breaker-findings.md`) + Counsel (`counsel-opinion.md`) reviewed this in
> **two rounds**; dispositions per finding/STOP are in `resolution.md` and folded into the sections
> below (markers: C1/C2/H1-H4/M1-M6/L1-L3, STOP-1/2; **round 2: R2-A..R2-F + Counsel C-1..C-4**).
> Goes back for a **final** re-attack — §11 + `resolution.md` are honest about what is **fixed** vs
> **accepted** vs **deferred (MISSING)** vs **NEEDS-HUMAN**.

---

## 1. Problem + non-goals

**Problem.** Hands-busy / low-friction interaction on the **client-facing storefront** (menu +
checkout-read-only — the active scope per the operator 2026-06-30; admin/courier voice is deferred
to a separate future council, §10):
let a customer say *"shto sufllaqe"* / *"add a gyro"* and have the menu act, without ever
finalizing money or stateful actions by voice. Albanian (`sq`) is the default locale and the
hardest ASR target; `en`/`uk` carry **first-class i18n string parity** — but "first-class" means
string parity **plus an independent per-locale safety gate** (§4.2), not safety parity by default.
The route is the public menu (`/s/:slug/*`, `apps/web/src/main.tsx:49`).

**Who Phase 0/1 is for (honest framing — RESOLVE accessibility-inversion).** Phase 0/1 voice is
**WebGPU-gated** (§2.2, §7) and its confirm/disclosure UI is text. It therefore routes the benefit
**away** from the low-end / elderly / low-literacy users an accessibility claim would invoke →
**the accessibility justification is explicitly NOT claimed.** Phase 0/1 is **a convenience for
capable (WebGPU) devices.** Earning an accessibility framing later requires audio confirm read-back
(TTS) + an honest reckoning with whom the WebGPU floor excludes (deferred; see §11).

**Non-goals (explicit, to keep the blast radius small).**
- **No voice checkout / pay / place-order, and no voice binding to any money / order-finalization
  path in Phase 0/1/2** (customer storefront). Phases 3/4 (admin/courier) are a **separate,
  independently-gated scope NOT approved by this proposal/ADR** (§6, §10, §11). Server stays
  authoritative for price/status (canon). The earlier unqualified "no money binding *ever*" is
  corrected: courier `arrived`/`completeDelivery` *settles cash* (ADR-0009/0013) → it is excluded
  from the voice grammar by construction (§6), the same as `place_order`/`pay`/`checkout`.
- **No voice for allergen / dietary-safety filters — ever.** A voice-set allergen filter is a
  *false safety assertion* off a noisy channel (negations are the weakest part of small-model ASR).
  `setFilterAllergen` is **not** voice-reachable; allergen chips stay **touch-only** (§6, C1).
- **No dictation / free-text** (no "leave a note for the kitchen"). Voice emits *intents from a
  closed grammar*, never arbitrary text into a field.
- **No wake-word / always-listening.** Mic is an explicit per-use gesture only. Ambient
  listening is **FORBIDDEN**, not deferred (surveillance-shaped → Ethics Charter). 
- **No server-side transcription** (rejected in §3.2 on privacy + cost grounds).
- **No new auth path, no new RLS surface, no new write/PII surface** for Phase 0/1 (§5, §8). The
  runtime kill-switch (§9/R2-A) needs **one new minimal read-only endpoint**
  (`/api/public/voice-config`) — the earlier "no new endpoint" claim is **withdrawn** (the
  cache-first SW defeated the bootstrap-piggyback); it is a fail-closed flag read, **not** an
  audio/PII or write surface, so the no-new-RLS/auth/write invariant holds.
- Voice is **strictly additive**: every touch/keyboard path remains fully functional.

---

## 2. Back-of-envelope (show the arithmetic)

### 2.1 Model size (whisper-base, the PoC model) — mixed-precision, not pure-q8 (L3)
- whisper-base = **74M params**. Pure-int8 raw weights would be ~74 MB; fp16 ≈ 148 MB; fp32 ≈ 296 MB.
- **The Transformers.js ONNX artifact is mixed-precision, NOT pure-q8:** quantized matmuls **plus
  retained fp32 ops** (layernorm/embeddings) on a split encoder + decoder. So the **transferred**
  bundle is *expected* to exceed raw int8 and is **not** reconcilable with 74 MB × 1 (correcting the
  earlier "q8" label): encoder ≈ 42 MB + decoder_merged ≈ 85 MB + tokenizer/config ≈ 2 MB
  → **≈ 130 MB per cold fetch (mixed-precision quantized, *measured target*)**. A q4/q4f16 build
  (if exported) ≈ 40–65 MB. **This byte size is a Phase-0 measurement output, not an assumption** —
  record the real bytes of the chosen artifact in the gate report, and **recompute the §2.2 memory
  floor from that measured number** (it feeds the OOM case).

### 2.2 In-browser latency & memory (mid Android, the constraint) — WebGPU-gated (H2/M2)
- **The 3-rung ladder was fiction in the storefront's own context. The honest posture is 2 states:
  WebGPU → voice; otherwise → mic hidden.** Reason: WASM **threads** need `SharedArrayBuffer` →
  cross-origin isolation (`COOP: same-origin` + `COEP`), which **would break** the storefront's
  existing first-class cross-origin resources (verified in the live CSP: `cdn.jsdelivr.net`,
  `tiles.openfreemap.org`, `router.project-osrm.org`, `plausible.io`, Google Fonts). **We will NOT
  enable COOP/COEP** → threads are unavailable by deliberate decision → the only WASM path is
  single-thread (15–40 s+), which is a latency lie and is **not offered**. Net: **voice is
  WebGPU-gated on the storefront.**
  - **WebGPU** (the only shipped path): RTF < 1 on most 2022+ phones → **~1–3 s** end-to-end. OK.
  - **No WebGPU** (incl. iOS Safari today, and most Instagram/TikTok/FB in-app WebViews — the
    dominant social-entry traffic): **mic hidden** (accepted reach cost — §11 R-K; the Phase-0 gate
    report MUST record the real WebGPU-availability rate across a representative device/WebView
    sample so "who can use this" is measured, not assumed).
- **Memory (corrected — M2):** model weights ~130 MB + ONNX arena (≈2–3×) **+ the already-loaded
  storefront tab** (49 products with R2 photos → decoded images + React tree ≈ 100–200 MB) **+ the
  WebGPU GPU-buffer copy** (~another model size; integrated/shared-memory Android counts it against
  the **same** RAM ceiling) → **realistic peak ≈ 500–700 MB** on a 3–4 GB device with the menu open
  → squarely **tab-crash territory**, not merely "borderline."
- **Capability floor is MEASURED, not predicted (M2).** `navigator.deviceMemory` is **absent on iOS
  Safari** and **coarse/capped on Android** → it cannot gate the exact devices it must protect, so
  it is demoted to a **soft hint only.** The floor is a **bounded runtime warmup probe**: require
  WebGPU adapter presence, then run a **tiny warmup transcription under a hard time + OOM `try/catch`
  budget**; if the device cannot complete the warmup in budget, **hide the mic.**
- **Conclusion (drives §7):** WebGPU + a passing warmup probe = voice; anything else = mic hidden,
  rather than ship a feature that times out or OOMs.

### 2.3 R2 egress per model fetch
- ~130 MB per cold fetch. **Cloudflare R2 egress to the internet is $0** (zero egress fees;
  that is R2's whole point — reuse the existing `dowiz-images` bucket pattern). Cost is one
  Class-B GET (~$0.36 / million reads → negligible).
- Transformers.js **caches the model in IndexedDB after first load** → **one fetch per device
  per model version**, offline thereafter.
- Lifetime worst case: 10,000 unique devices × 130 MB = **1.3 TB transferred → $0 egress on R2.**
  On HF CDN this is also "free" but with no version pin / no SLA / a third-party origin in the
  storefront CSP (§ decision 2). The math is what makes R2 a no-brainer.

### 2.4 Concurrent storefront sessions & server load (the architectural win)
- Planning scale: ~50 locations × ~20 orders/hr peak; browsing sessions ≈ 5–10× orders →
  ~150 concurrent storefront sessions at peak. Voice opt-in (taps the mic) ≈ 5–10% →
  **~15 concurrent voice users.**
- Because transcription is **100% client-side**, those 15 users add **ZERO** to:
  the API process, any worker, analytics, or the migrations pool. The only server touch is the
  **static, cached, $0-egress R2 GET** of the model. **Server-side scaling impact ≈ nil.**
- **DB connection budget (API + worker + analytics + migrations, the standing constraint):**
  client-side voice consumes **0 new connections**. The rejected server-side option (§3.2) would
  add a transcription **worker role** → new pool pressure against the already-tight Supabase
  poolers (`packages/config/src/index.ts:7-9`) — a second reason to reject it.

### 2.5 Audio bandwidth IF server-side (the rejected option, costed)
- 16 kHz mono Float32 = 16000 × 4 = **64 KB/s = 512 kbps** raw; PCM16 = 256 kbps;
  Opus ≈ 16–24 kbps. A 5 s utterance: raw 320 KB, Opus ~12–15 KB.
- Bandwidth itself is survivable, but server-side needs a **transcription server**: we have **no
  GPU**; CPU whisper-base RTF on a small Fly VM ≈ 5–10× → it **cannot keep up with even a few
  concurrent users**, and it means **audio PII egress + a new stateful streaming endpoint +
  connection-pool pressure**. Privacy regression **and** infra regression → rejected (§3.2).

---

## 3. Architecture options (named concepts + tradeoffs)

### 3.1 Engine-home (open decision 1): `packages/ui` vs new `packages/voice`

| | Option A — extend `packages/ui` | **Option B — new `packages/voice` (RECOMMENDED)** |
|---|---|---|
| Concept | *Co-location in the shared UI lib* | *Quarantine boundary / bulkhead for an optional heavy subsystem* |
| Pros | i18n catalog already lives there; MicFab is a UI component; no new package wiring | Heavy + dangerous parts (the `@huggingface/transformers` dep, the Web Worker, WASM/ONNX asset config, the model fetch) isolated to one package that **only `apps/web` depends on, only via dynamic import**; the package can be built with **zero dependency on `CartProvider`/`apiClient`** → the "zero write capability" invariant (§6) is structural, not just convention |
| Cons | Pollutes a **99.9th-%ile churn** package (`packages/ui/src/lib/i18n.ts`); pulls a large optional dep into the transitive graph of every `packages/ui` consumer; Vite worker+WASM build config bleeds into the shared package | New package scaffolding; must inject i18n strings (depend on `packages/ui` catalog) rather than co-locate them |

**Recommendation: Option B (`packages/voice`).** Boring/proven says: a thin new boundary
package that can *hold zero write capability* is cheaper than untangling a heavy optional
subsystem out of a hot shared package later. This is a deliberate exception to "edit don't
create" — the architectural register: **the boundary is the design**. Split of concerns:
- `packages/voice` — engine (worker, audio, capability detect), IntentMatcher, grammar types,
  the `SpeechProvider` interface + `MockProvider`. **No import of any app store / API client.**
- `packages/ui` — the presentational **MicFab** shell + voice i18n strings (reuses the existing
  key-major catalog `packages/ui/src/lib/i18n-catalog.ts`, added via `scripts/i18n-add.ts`,
  sq/en/uk parity gate).
- `apps/web` — per-role **intent packs** that bind grammar → existing handlers. These *must* live
  in the app because they reference app handlers; they cannot be shared.

### 3.2 Transcription runtime (open decision 4 — the privacy crux): client-WASM vs server-worker

| | **Option A — client-side ONNX/WASM in a Web Worker (RECOMMENDED)** | Option B — server transcription worker |
|---|---|---|
| Concept | *On-device inference / data-stays-home* | *Centralized ASR service* |
| Audio egress | **Zero** — audio never leaves the device | Audio (PII) streamed to our server → new subprocessor-shaped surface |
| Infra | 0 new DB connections, 0 new endpoints, $0 R2 GET only (§2.4) | Needs GPU we don't have; CPU can't keep up (§2.5); new worker role + pool pressure |
| Latency | WebGPU ~1–3 s; WASM 15–40 s on low-end (§2.2) | network RTT + server queue |
| Privacy/GDPR | No audio/transcript at rest, no new RoPA/subprocessor | Audio-PII processing → DPIA, retention policy, subprocessor disclosure |
| Failure mode | degrades to "hidden" on weak device | a server outage takes voice down for everyone |

**Recommendation: Option A (client-side, Web Worker).** It is the only option consistent with
the canon "nul PII у ШІ", the claim-check pattern, and the connection budget. Web Speech API is
**also rejected** because it streams audio to Google (a subprocessor + GDPR disclosure,
Chrome/Edge-only). Engine: **OpenAI Whisper via Transformers.js / ONNX Runtime Web** (MIT),
WebGPU → WASM, dynamic-import-only.

---

## 4. Decisions on the 5 open questions (summary)

1. **Engine home →** new **`packages/voice`** (engine+matcher+MockProvider), MicFab + strings in
   `packages/ui`, intent packs in `apps/web`. (§3.1)
2. **Model delivery for the PoC →** **R2 from day one** for anything on a deployed route; HF CDN
   only for a throwaway local-laptop capability probe. (§4.1 below)
3. **Phase-0 GO/NO-GO (per locale, on CI bounds — §4.2) →** GO if IRA lower-95%-CI ≥ 80% AND
   point ≥ 85% **AND** dangerous-misfire **upper-95%-CI ≤ 2%** (one-sided safety bound), on a
   **≥300-utterance/locale** consented corpus; < 70% point IRA → NO-GO; in-between → AMBER (tune
   matcher w/ **runtime per-tenant** biasing, re-measure). `sq`/`en`/`uk` each gated independently.
4. **Privacy/runtime →** transcription **client-side in a Web Worker**; **zero audio/transcript
   egress or persistence in the production runtime**; the **Phase-0 eval corpus is a separate
   consented research regime with its own RoPA/retention (§8.1)**; logs carry coarse non-PII
   counters only; consent = explicit gesture + native getUserMedia + one-time on-device disclosure
   **with a real decline + persistent off-setting**. (§3.2, §8, §8.1)
5. **Intent→action safety →** the voice layer is a **pure producer of `IntentProposal` values with
   zero write capability** (control **inverted** so no handler crosses the boundary — M3); a single
   **ConfirmationGate** with a **total, fail-closed** capability table (unknown kind → reject — M4)
   enforces confirm-then-execute for every STATEFUL intent; **money, allergen/dietary filters, and
   cash-settling courier actions have NO voice binding** (no `kind`, by construction). No voice→money
   binding in Phase 0/1/2; Phases 3/4 are separately gated. (§6)

### 4.1 Decision 2 — model delivery: R2 from day one

| | Option A — HF CDN for the PoC | **Option B — R2 from day one (RECOMMENDED)** |
|---|---|---|
| Setup | none (Transformers.js default) | one-time convert + upload (needed for Phase 2 anyway) |
| Version pinning | none → upstream could change under us and **silently invalidate the Phase-0 gate measurement** | immutable, version-pinned path |
| CSP | must allowlist `huggingface.co` + their CDN in the locked storefront CSP (`apps/api/src/lib/spa-shell.ts`); "temporary" allowlists become permanent | first-party R2 custom domain — narrow, our origin |
| Egress | "free" but third-party | **$0 (R2)**, reuses `dowiz-images` pattern |

**Recommend R2 from day one.** Version pinning is a *correctness* requirement for the gate; the
CSP surface stays first-party; the conversion step is not wasted (Phase 2 needs it). HF CDN is
acceptable **only** for an un-deployed local spike, **never** on the deployed staging `/s/:slug`.

### 4.2 Decision 3 — the Phase-0 `sq` threshold (concrete GO/NO-GO)

Raw WER is the **wrong** gate: the product is *command matching against a closed vocabulary*,
not dictation. The gate metric is the **end-to-end** result of `transcribe → IntentMatcher`.

**The corpus is a separate, consented research artifact — see §8.1 for its data-governance
(RoPA, consent, retention, deletion). It is NOT a storefront collection and does NOT relax the
production zero-egress invariant (resolves C2).**

- **Corpus (per locale — M1 power):** **≥ 300 real-device utterances per shipped locale** for the
  safety metric, from **≥ 15 distinct speakers** (incl. Gheg/AL accents for `sq`), across all
  client intents, in a noisy **and** a quiet condition, with a documented recording protocol,
  double-checked labels, and **deliberate adversarial near-miss pairs** (accept/reject, with/without,
  allergen-adjacent, **and dietary-named-category utterances** like *"shfaq pa gluten"* against a
  *"Pa gluten"* category — R2-B). The `sq` corpus uses the demo/pilot menu vocabulary (see M6 scope
  caveat).
- **Primary metric — Intent Resolution Accuracy (IRA):** fraction resolved to the correct
  `{action, target}` after fuzzy slot-fill.
- **Safety metric — dangerous-misfire rate (redefined — C1):** fraction that resolve to a *wrong
  STATEFUL* intent a user might plausibly confirm **OR** that produce a state a user could read as a
  **dietary / safety assertion**. (Allergen filters are no longer voice-settable — §6 — so the
  residual here is an adjacent misfire, e.g. a spoken allergen phrase fuzzy-matching a search; it is
  counted, not excluded.)
- **Diagnostic (report, do not gate on):** token WER on the menu-item slot span. **Required gate-
  artifact field (Counsel C-3 — a printed number, not a promise):** the **measured WebGPU-
  availability rate** across the device/WebView sample (H2 reach). The gate report is incomplete
  without this line — it is how "who this feature excludes" stays a measurement and not a footnote.

**Decision rule (per locale — H3; on confidence bounds, not point estimates — M1):**
- **GO** (ship base for that locale, **skip** Phase-2 fine-tune): IRA **lower-95%-CI ≥ 80% AND
  point ≥ 85%**, **AND** dangerous-misfire **upper-95%-CI (Wilson) ≤ 2%** (a one-sided *safety
  bound*; ~0 events / 300 → upper ≈ 1.2%). Report the CIs, not bare points.
- **NO-GO** (schedule Phase-2 fine-tune for that locale): IRA point < **70%**.
- **AMBER** (in between): ship dark, **tune the IntentMatcher** (fuzz thresholds; **runtime
  per-tenant** vocabulary biasing — *not* a shipped static demo-vocab bias, M6) and re-measure
  **inside the corpus retention window** (§8.1); commit fine-tune spend only if matcher tuning
  can't clear the bound.
- **Per-locale gating:** `sq`/`en`/`uk` are each gated **independently**; a locale that has not
  passed its own dangerous-misfire bound ships **dark for that locale** even though strings exist.

**Gate-scope honesty (M6):** the demo/pilot-menu IRA validates the **pipeline mechanics**
(transcribe + matcher), it is **NOT a per-tenant launch certification.** Phase 1 ships to the
demo/pilot tenant first; **broad multi-tenant launch is gated on a multi-tenant (multi-menu)
re-measurement** (deferred — §11 R-N).

**Why these bounds:** below ~85% top-1 *with* a confirm step users hit "no match / did-you-mean"
more than ~1 in 7 tries → worse than typing; and n=150 could not resolve a 2% safety rate (Wilson
CI ≈ 0.4–5.7% at n=150) → the corpus size and the CI-bound rule make the gate statistically real.
The Phase-2 fine-tune spend is justified only once the base model cannot clear the bound and the
matcher can't absorb the gap.

---

## 5. Data / migrations

**Phase 0 and Phase 1: ZERO migrations, ZERO new tables, ZERO new RLS surface, ZERO new
write/PII endpoint.** The model is a static R2 GET; transcription and intent matching are
client-side; voice dispatches **existing already-authz'd handlers** (`addItem`
`apps/web/src/lib/CartProvider.tsx:89`; the menu setters `MenuPage.tsx:211-240`). Nothing is written
to a DB by the voice path. **The one new server surface (R2-A):** a **read-only** control-plane
endpoint `GET /api/public/voice-config?slug=:slug` returning `{ enabled }` (the kill flag) — no
write, no PII, no DB mutation, no new RLS table; it reads the server `VOICE_CONTROL_ENABLED` /
`VOICE_KILL` env. (The earlier "zero new endpoints" was withdrawn — the cache-first SW forced a real
hot-kill path; the surface is a flag read, not a data surface.)

**Phase 2: ZERO migrations** — the Albanian fine-tune is an R2 asset + a locale route + a flag.

**Only-if-ever (deferred, would be a RED-LINE migration):** opt-in voice telemetry as a
tenant-scoped table. If built it **must** be forward-only, atomic, idempotent, re-assert
`ENABLE + FORCE` RLS, integer where numeric, and contain **zero transcript text** (counters
only). Default dark. Not in scope now — stated so the breaker can hold us to it.

**The Phase-0 research corpus is NOT a production data store (C2):** it lives **outside** any
tenant DB and **outside** the public R2 bucket, has **no RLS surface**, and no production system
reads it. It is a consented, time-boxed research artifact with its own RoPA entry and deletion
proof — governed in §8.1, not here.

---

## 6. Consistency, idempotency, and the confirm-then-execute INVARIANT

> **UI surfaces for this section (MicFab, the state machine, the confirm chip, the disclosure
> sheet, the checkout read-back, i18n keys, a11y/motion, the Phase-0 overlay) — see
> [`ui-spec.md`](./ui-spec.md).** It renders these invariants in pixels; it adds no behaviour they forbid.

**Architectural invariant (the single most important safety property).** The voice layer is a
**pure producer of immutable `IntentProposal` values and holds zero write capability.**

```
IntentProposal = { kind, slots, confidence, transcript, id }   // immutable, no methods
```

- **Control is inverted so no write-capable reference ever crosses the boundary (M3).** The engine
  is a **source**; the `ConfirmationGate` (in `apps/web`) is the **sink** that *pulls*
  `IntentProposal` values off a typed **`AsyncIterable<IntentProposal>`** of plain `readonly` data
  (consumed via `for await (const p of engine.results())`). The emission surface is the async
  iterator **only** — **not** an `on(event, cb)` listener and **not** a `subscribe({ onResult })`
  options-bag callback (R2-F: those are function-typed params the guardrail must forbid; the
  iterator makes the API and the guardrail coherent). The engine's public API therefore accepts
  **no callback / function-typed parameter** — so there is no `cb` that could close over
  `addItem` / `handleUpdateStatus`. The engine + IntentMatcher hold **no
  reference** to any cart mutator, `setState`, `apiClient`, store dispatch, or status handler.
  **"Cannot mutate" is now structural against runtime callbacks, not just static imports.**
- The `ConfirmationGate` (the only holder of the handlers) consumes a proposal and classifies it via
  a **total, exhaustive capability table — `default` REJECTS (fail-closed, M4)**:
  - `READ_ONLY` — filter / sort / search / macro / compare **+ the two checkout-read intents
    (scope narrowing, operator 2026-06-30):** **"read my order"** (reads the user's **own
    client-side cart + total** back to them — no field write, no network call, no new PII egress,
    no cross-tenant surface; the cart is the in-tab state the user already holds) and **"go to
    checkout" / navigation** (a route change, UI-local). UI-local, reversible, no money,
    no network-write, **no safety assertion** → **may auto-apply** (calls `setSortBy` /
    `setSelectedCategory` / `setMacroLens` / `setSearchQuery` / `toggleCompare`,
    `MenuPage.tsx:211-240`). **`setFilterAllergen` is REMOVED from this set (C1)** — see below.
    **Checkout adds exactly these two READ_ONLY intents and ZERO stateful/money voice grammar**
    (address / phone / notes are **not** voiceable; place-order / payment / order-finalization
    have **no `kind`** — see the exclusion list).
    - **Category voice-match is class-constrained, not just setter-scoped (R2-B).**
      `setSelectedCategory` (`MenuPage.tsx:474`) filters by **tenant-defined free-text** category.
      A category named *"Pa gluten"* / *"Vegan"* / *"Без глютену"* auto-applied by voice narrows the
      menu and is **read by the user as a dietary-safety assertion** — the same false-safety hazard
      C1 closed, through a still-open setter, with **no checkpoint at all**. So: a category whose
      normalized name **matches the dietary/allergen token denylist** (the single-source stem list
      across **sq/en/uk** — `pa arra`/`pa gluten`/`pa laktoz*`/`vegan`/`vegjetar*`/`vege`/`bio`/
      `organik*`/`alergj*`/`çeliak*`/`halal`/`kosher` · `gluten`/`gluten-free`/`dairy-free`/
      `nut-free`/`lactose`/`allergen`/`allergy`/`vegan`/`vegetarian`/`organic` · `веган`/
      `вегетаріан`/`глютен`/`без глютену`/`без лактоз*`/`алерг*`/`органічн*`/`халяль`/`кошер`,
      matched across all three locales regardless of the active one) is **NOT voice-auto-appliable**
      — the voice match is **dropped**, the category stays **touch-only** (treated like allergen).
      Non-dietary categories (e.g. *"Pizza"*, *"Drinks"*) auto-apply normally.
  - `STATEFUL` — **add-to-cart** (the **only** STATEFUL intent in active scope) → **renders a
    confirm affordance**; the real handler runs **only on an explicit human tap**. The proposal
    never reaches the handler before confirm. **(Scope narrowing, operator 2026-06-30: the active
    capability table contains NO admin/courier stateful intent. The earlier "non-settling
    admin/courier" STATEFUL intents are REMOVED from this build and deferred to a separate future
    council — see §10.)**
  - **`default` (any unlisted / unmapped / forgotten `kind`) → REJECT** — dropped with "didn't catch
    that", executed **neither** as auto-apply **nor** as STATEFUL-confirm. A `never`-assertion
    exhaustiveness check makes a new grammar `kind` without a table row **fail the build**.
- **Excluded from the table entirely — no `kind` a voice intent can carry (by construction):**
  - **Money / checkout writes / place-order / payment / order-finalization** (storefront). The
    checkout flow exposes **only** the two READ_ONLY intents above ("read my order" + "go to
    checkout" navigation); **no checkout field (address / phone / notes) is voiceable, and there is
    no place-order / pay / finalize `kind`** — checkout stays inside the money-has-no-voice-grammar
    invariant and adds **zero** stateful/money voice grammar (scope narrowing, operator 2026-06-30).
  - **Allergen / dietary-safety filters (C1)** — `setFilterAllergen` and any future "safe-for-me"
    filter. A voice-set safety filter is a *false safety assertion* off a noisy channel; a confirm
    chip that echoes a mis-parsed allergen is itself a trusted safety assertion. Touch-only.
  - **Cash-settling courier actions (H4)** — `arrived` / `completeDelivery` *settle cash*
    (ADR-0009/0013) = a money-state transition → excluded the same as place_order. If Phase 4 ever
    ships, courier voice is limited to **non-settling** actions, separately gated + STOP-1's human
    decision.
- **Money-binding claim, corrected (H4):** there is **no voice binding to money in Phase 0/1/2.**
  "No voice binding to money *ever*" was an overclaim — Phases 3/4 are a separate scope and cash
  settlement is excluded by construction (above), not merely confirm-gated.

**Enforcement is a guardrail (red→green), not convention** — for the self-improvement ratchet:
1. An `eslint-plugin-local` rule + a grep guardrail: `packages/voice` modules import **no** mutating
   API client, **no** `CartProvider` mutator, **no** status handler — **and** export **no
   function-typed parameter** in their public API (M3: no injected write-capable closure).
2. A unit test: feed a `STATEFUL` proposal through the gate **without** a confirm tap → assert the
   handler is **NOT** called; with a confirm tap → assert it is called exactly once. **Plus an
   unknown/unmapped `kind` → assert no handler called AND no auto-apply (M4 fail-closed).**
3. A table test asserting add-to-cart is `STATEFUL`, **no `place_order`/`pay`/`checkout` kind
   exists, no allergen/dietary kind exists and `setFilterAllergen` is not voice-reachable (C1), and
   no `arrived`/`completeDelivery`/settling kind exists (H4).**
4. **A category-denylist test (R2-B):** feed category names — incl. `"Pa gluten"`, `"Vegan"`,
   `"Без глютену"` — through the voice-category matcher and assert a denylisted-named category is
   **not** voice-auto-applied; the denylist is one exported single-source constant.
5. **An actor-anonymous telemetry-schema test (Counsel C-1, locked from Phase 0/1):** assert the
   voice counter record `{ intent_kind, matched, confidence_bucket, locale }` carries **no**
   `courier_id`, **no** `user_id` / actor id, and **no** latency / timing field — the surveillance
   gradient (STOP-1) is forbidden at the column, before any worker mic exists.

**Idempotency / double-fire.** Voice introduces **no new write path** → it inherits the existing
idempotency of the touch path (add-to-cart is local state; admin/courier status changes already go
through idempotent server handlers per ADR-0007/0009/0013). Extra guard: a proposal is
**consumed-once** by the gate (marked consumed on confirm) so a stray worker re-emit can't
double-add. Server stays authoritative for price/status.

---

## 7. Failures + degradation (failure-first; every external call has timeout+fallback, zero cascade)

The voice path has **two external network calls (corrected — R2-A):** (1) a tiny `/api/`-prefixed,
`no-store`, **fail-closed control-plane GET** (`/api/public/voice-config?slug=…`, §9) that the
service worker exempts and whose only failure mode is "voice OFF" (never a page block, never a
cascade), and (2) the R2 model GET. Everything else (audio capture, inference, matching) is local.
The honest reliability property is narrowed but intact: **there is still no audio/transcription
server to fall over** — the new call is a fail-closed flag read, not a stateful streaming endpoint.
(The earlier "exactly one external call" was true only before R2-A forced a real hot-kill path.)

| Failure | Detection | Degradation (no cascade) |
|---|---|---|
| Mic permission **denied** | getUserMedia reject | MicFab shows one-line "use touch" and disables; no retry loop; touch/keyboard intact |
| getUserMedia unsupported / **insecure context** | feature detect | **hide** MicFab |
| **WebGPU unavailable** (incl. iOS Safari, most social in-app WebViews) | capability detect | **hide** MicFab — voice is WebGPU-gated; WASM-single-thread is **not offered** (no COOP/COEP → no threads; a slow mic is a latency lie, §2.2/H2) |
| **Warmup probe fails** (no WebGPU adapter, or tiny warmup transcription exceeds time/OOM budget) | bounded runtime warmup (§2.2/M2) | **hide** MicFab — never ship a feature that will OOM/timeout; `deviceMemory` is a soft hint only |
| **Runtime kill flipped** (operator hot-kill, §9/M5/R2-A) | `/api/public/voice-config` GET (`cache:'no-store'`, SW-exempt) before import/fetch | MicFab not rendered; engine never imported, model never fetched — runtime-instant, no rebuild, **and instant for returning visitors** (the `/api/` path is never SW-cached) |
| **Control-plane GET fails / offline / malformed** (R2-A fail-closed) | fetch reject / `!res.ok` / `enabled` absent or `!== true` | **voice stays OFF** — absence is OFF, not on; the page/menu/cart are untouched (the call is voice-only) |
| **Model fetch fails / offline** (first use) | fetch error / **30 s timeout** | retry affordance + "voice unavailable offline"; page never blocked; **no spinner-forever** |
| **Inference timeout** (WASM too slow) | **~8 s** cap | abort, terminate+respawn worker; "didn't catch that — try again or use touch" |
| **Empty / low-confidence transcript** | confidence < threshold | re-prompt / "did you mean" chips; **never auto-execute a STATEFUL action on low confidence** |
| **Ambiguous match** (two items within edit distance) | matcher returns ties | disambiguation chips; **never guess-execute** |
| **Worker crash** | worker error event | terminate + offer retry; **main thread unaffected** (isolation) |

---

## 8. Security + tenant isolation + consent

- **No new auth path, no new RLS surface, no new write/PII surface** (§5). Voice is a new *input
  device* for **existing guarded handlers**; it bypasses no RLS/auth and adds no privilege path. The
  one new server surface is the **read-only** `/api/public/voice-config` kill-flag endpoint (R2-A) —
  no write, no PII, no RLS table.
- **Tenant isolation by construction:** the matcher's slot-fill vocabulary is **only** the
  in-memory, already-fetched, already-authz'd data the role legitimately has (storefront menu
  vocab is public; admin order ids the admin already sees; courier tasks the courier already
  sees). Voice can't *name* a target outside the role's scope, and if it somehow did, the
  underlying handler + RLS reject it. The matcher issues **no free query**.
- **Menu-only to the ASR/matcher** — no PII fed to the model; the model is generic ASR, not an
  LLM with tenant context (claim-check spirit preserved).
- **Privacy / audio = PII (decision 4):**
  - Audio lives in an **ephemeral in-memory ring buffer**, discarded after transcription;
    **never written to disk/IndexedDB** (only the *model weights* are cached in IndexedDB).
  - The transcript string lives **only** in component state for the confirm UI; **never sent to
    the server, never logged.**
  - **Logging policy:** zero transcript/audio in any log or telemetry. Allowed telemetry =
    coarse counters only `{ intent_kind, matched:bool, confidence_bucket, locale }` — no free
    text, no menu-item names (a spoken utterance can contain incidental PII).
  - **Actor-anonymous schema is a guardrail NOW, from Phase 0/1 (Counsel C-1 — locks STOP-1 at its
    cheapest point):** a red→green test asserts the counter record carries **no** `courier_id`,
    **no** `user_id` / actor id, and **no** latency / timing field. Forbidding the per-actor column
    *before* any worker mic exists makes the labour-surveillance gradient structurally impossible to
    grow into later, rather than relying on a future human to refuse a schema addition.
  - **Audio is personal data, NOT Art-9 biometric (Counsel correction).** Whisper does ASR, not
    speaker identification; Art-9 biometric applies only when processing is *for uniquely
    identifying a person*. So no mandatory explicit-consent regime is triggered — stated accurately
    rather than inflated (inflated risk language is how a heavier control gets rationalised later).
  - **GDPR (production runtime):** no audio/transcript egress and no persistence → **no new
    server-side processing of personal data, no new subprocessor, no new RoPA entry for the
    production runtime**. Still run `scripts/compliance-gate.ts`. (Web Speech API rejected precisely
    because it *does* egress.) **The Phase-0 research corpus is a separate regime with its own RoPA
    — see §8.1; the blanket "no new RoPA" holds only for the runtime, not the corpus (C2).**
  - **Consent UX (STOP-2 dissolved):** mic is an explicit gesture (tap MicFab) → the browser's
    native getUserMedia prompt is the *mic-access* boundary; plus a one-time first-run disclosure
    sheet ("Processed on your device. Audio never leaves your phone. No recording is kept.") in
    sq/en/uk that **offers a genuine choice — "Use voice" vs "Not now / use touch"** — where "Not
    now" is the no-op default and leaves touch fully functional. It is **not** an OK-only gate. It
    is backed by a **persistent on/off voice setting** in storefront prefs (like `dos_menu_prefs`),
    not just a dismissable sheet. Guardrail: a test asserts the decline path leaves the mic
    **unactivated** and touch working. **Plus (Counsel C-2):** an assertion that **"Use voice"** and
    **"Not now / use touch"** are the **same affordance weight** (same button class — not a bright
    primary CTA against a greyed, small ghost). A passing *functional* decline test must not license
    a lopsided button hierarchy that re-introduces a soft dark-pattern. **No always-listening / no
    wake-word** (FORBIDDEN).
- **Supply chain:** `@huggingface/transformers` + `onnxruntime-web` are quarantined in
  `packages/voice`, **dynamic-import-only**, versions pinned, scanned (SkillSpector/dep gate)
  before adding — matching the existing "scan before install" rule.
- **CSP (exact change — L1).** The live storefront CSP (`apps/api/src/lib/spa-shell.ts:150`) has R2
  **only in `img-src`** (`r2ImgSrc`); `connect-src` is `'self' https://cdn.jsdelivr.net
  https://tiles.openfreemap.org https://router.project-osrm.org https://en.wikipedia.org
  https://plausible.io` — **no R2.** A 130 MB model `fetch()` is governed by `connect-src` and is
  **currently blocked.** The required change: derive an `r2ConnectSrc` from `R2_PUBLIC_URL` exactly
  like `r2ImgSrc` (lines 144-148) and append the **first-party R2 origin** to `connect-src`.
  `worker-src 'self' blob:` already covers the worker; `script-src` already has `'unsafe-eval'`
  (covers WASM compile).
  - **Single source of truth — kills the two-flag desync (R2-E).** The launch flag
    `VITE_VOICE_CONTROL_ENABLED` is a **client** build-arg, invisible to `spa-shell.ts`. So the CSP
    cannot gate on it. Define a **server authority** read at request time in `spa-shell.ts` (where
    the CSP is already built per request): **`VOICE_CONTROL_ENABLED` + `VOICE_KILL`**. The connect-src
    R2 widening uses the **same predicate as the runtime control-plane endpoint** (§9/R2-A):
    `VOICE_CONTROL_ENABLED && !VOICE_KILL`. Because it is the *same* predicate, **a hot-kill
    (`VOICE_KILL=true`) both returns `enabled:false` from `/api/public/voice-config` AND drops the
    R2 origin from `connect-src` on the next document load** — a killed voice closes the R2 origin
    too. The CSP is **not** widened while the feature is off (true-dark = zero surface delta).
  - **Sync mechanism:** one **per-environment variable** in the deploy pipeline fans out to **both**
    the client build-arg `VITE_VOICE_CONTROL_ENABLED` **and** the server env `VOICE_CONTROL_ENABLED`,
    so they flip **together**, never independently. The client build-arg is only the true-dark
    tree-shake gate; the **server flag is authoritative at runtime** (the client always asks
    `/api/public/voice-config` before activating — fail-closed). This removes both desync cases:
    client-on/server-off → config says off → the client never attempts the model fetch (no silent
    death); server-on/client-dark → the predicate also requires the feature enabled (no widen-while-
    dark). A deliberate, scoped widening of a locked policy — recorded as such, not "reusing the
    dowiz-images pattern."

### 8.1 Phase-0 research corpus — a separate data regime (resolves C2)

The §4.2 gate cannot be computed without a labelled audio+transcript corpus. That corpus does
**not** relax the production zero-egress invariant — it is a **distinct, consented, time-boxed
research artifact**, governed independently:

- **Legal basis + consent conditions (gate on real-device recording — Counsel C-4):** explicit,
  informed, **written consent** from **recruited adult speakers** (not live customers), in
  `sq`/`en`/`uk`, stating purpose (ASR accuracy eval), retention, and deletion rights. Consent must
  be **freely given**, which for the wanted diaspora/migrant-Albanian cohort is exactly where
  freeness can erode — so the following are **named conditions on the gate**, not aspirations:
  - **non-coercive recruitment** — and **explicitly NOT drawn from the platform's own couriers /
    workforce** under any implied-benefit pressure (that would collapse STOP-1's power-asymmetry
    concern into the corpus itself);
  - **fair compensation**, an explicit **withdrawal right**, and **protocol-scoped consent** (the
    *specific* recording protocol + the retention window, not a blanket grant);
  - a documented **vulnerable-population safeguard** for the diaspora/migrant cohort.
  These gate **real-device human recording only** — **not** the cheap Phase-0 laptop probe (the
  architect's own voice, no recruitment) and **not** the Phase-0/1 engine build.
- **Debug-overlay zero-egress is a guardrail, not prose (R2-C — the strongest privacy property
  finally gets a gate):** the `VITE_VOICE_TRANSCRIBE_DEBUG` overlay is **FORBIDDEN on any public
  deployed `/s/:slug` serving real customers**, enforced by **three machine-checks**:
  1. **Bundle-absence (red→green):** the overlay + any transcript-capture/surfacing code path is
     gated behind `import.meta.env.VITE_VOICE_TRANSCRIBE_DEBUG` so it tree-shakes out; CI greps the
     built storefront chunks and **fails** if the symbol survives when the flag is not `true`.
  2. **Deploy-arg assertion:** every **public** deploy target (prod + the public staging `/s/:slug`)
     must carry `VITE_VOICE_TRANSCRIBE_DEBUG=false`; the deploy **fails** if a public target sets it
     `true`. "Serving real customers" = "deploy target is public" = a machine-checkable predicate on
     the deploy matrix, not a sentence in a doc.
  3. **Separate research build target / non-public host:** the debug/transcribe build is a distinct
     build target deployed **only** to a separate non-public / throwaway research host (never the
     public `/s/:slug`); the CI env-matrix check forbids it from the prod/staging-public lanes. The
     consented research session runs there, not on customer-serving infrastructure.
- **Storage:** one access-controlled, **encrypted** location **outside** any tenant DB and
  **outside** the public R2 bucket; no RLS surface; no production system reads it.
- **RoPA + owner:** the corpus gets its **own RoPA entry** + retention schedule + a **named
  data-controller/owner** (NEEDS-HUMAN to assign before any real-device recording).
- **Retention / deletion — enforced automatically, not by memory (R2-D):** **deleted within 90
  days** of the gate decision or on Phase-2 completion (whichever first), with a **documented
  deletion proof**. Because the store is off-tenant it has **no RLS surface** — RLS is a tenant-
  isolation tool and is the **wrong** instrument for a non-tenant research artifact (accepted, not a
  gap to close with RLS). The right instruments, **required before the first recording**:
  1. **Scheduled deletion job (cron):** each corpus item is written with an **expiry timestamp** in
     a manifest (recording date + retention window); the job deletes every expired item and writes a
     **deletion-proof entry** (item id, expiry, deletion time) to an audit log. Deletion is keyed to
     data, not to a human remembering.
  2. **Expiry-audit tripwire:** a scheduled assertion that **no** manifest item exceeds its
     retention expiry; on a violation it alarms and **fails `scripts/compliance-gate.ts`** — the
     automated analogue of the "ENABLE+FORCE RLS + gate" tripwire every other PII surface has.
  3. **AMBER window:** re-measurement happens **inside** the window or the corpus is re-consented +
     re-collected; the job deletes on expiry regardless.
  **Honest status:** these are **specified now as a gate on recording**, not built — the regime only
  exists once real-device recording is authorized (NEEDS-HUMAN below); fix-specified, not
  fix-complete.
- `scripts/compliance-gate.ts` records it as a documented research dataset.

---

## 9. Operability (flag rollout, kill-switch, observability, rollback, scaling gate)

- **Flag:** `VITE_VOICE_CONTROL_ENABLED` build-arg in the Dockerfile, **default `false`**,
  matching the existing dark-flag pattern (`Dockerfile:20-36`) — the **client true-dark tree-shake
  gate**. Its **server-side authority** is `VOICE_CONTROL_ENABLED` (+ `VOICE_KILL`), read at request
  time, single-source for both `/api/public/voice-config` and the CSP widening (§8/R2-E); one
  per-env deploy variable fans out to both so they never desync. Phase-0 debug overlay behind a
  sub-flag `VITE_VOICE_TRANSCRIBE_DEBUG`, which is **forbidden on any public deploy target by
  guardrail** (bundle-absence + deploy-arg assertion + separate research build — §8.1/R2-C), not by
  prose. Per-role launch is a separate explicit flag flip.
- **True dark when off (claim corrected — L2):** MicFab never renders → the engine module is
  **never dynamically imported** → the model is **never fetched** → intended **zero
  bundle/critical-path delta**. This is **intended, to be verified by a named guardrail before
  Phase-1 launch** (the word "verified" is withdrawn until that guardrail is green): an import-graph
  / built-bundle assertion that the storefront **main chunk contains none of**
  `@huggingface/transformers` / `onnxruntime-web` / the `packages/voice` engine when the flag is off
  (engine reachable **only** via dynamic import); CI fails if an eager import leaks it.
- **Kill-switch / rollback (M5 — runtime hot-kill, not just a rebuild):**
  - **Build-arg** `VITE_VOICE_CONTROL_ENABLED=false` remains the **true-dark default** (nothing
    imported, image-baked).
  - **Runtime hot-kill (corrected — R2-A; the bootstrap-piggyback was defeated by the cache-first
    SW).** `apps/api/public/sw.js` serves **every non-`/api/` GET cache-first** (`caches.match(req)
    .then(a => a || fetch(req))`, HTTP cache semantics ignored), so a kill ridden on the menu/info
    bootstrap payload reaches a returning visitor **never** (they keep the stale pinned payload, mic
    keeps activating/crashing) until an `UPDATE_CACHE_VERSION` redeploy — exactly what the hot-kill
    was meant to avoid. `cache:'no-store'` does **not** help on a non-`/api/` URL (the SW intercepts
    first). **Fix:** the kill is read from a **dedicated `/api/`-prefixed control-plane GET**,
    `GET /api/public/voice-config?slug=:slug`, which the SW **exempts by construction**
    (`pathname.startsWith('/api/')` → straight to network, never cached) → **instant for returning
    visitors too**. Fetched `cache:'no-store'` (belt-and-suspenders). It returns
    `{ enabled: VOICE_CONTROL_ENABLED && !VOICE_KILL }` (the **same predicate** as the CSP widening,
    §8/R2-E). MicFab activates, the engine dynamic-import runs, and the model fetch happens **only**
    on an explicit `{ enabled: true }`; **fail-closed** — fetch reject / `!res.ok` / non-JSON /
    `enabled` absent or `!== true` ⇒ **voice OFF** (absence is OFF, not on). An operator flips
    `VOICE_KILL` to disable the mic across all clients **runtime-instant, no rebuild**; the build-arg
    stays the true-dark default.
  - **Honest cost (R2-A):** this is a **new, minimal, read-only `/api/public/voice-config`
    endpoint** — the earlier "no new endpoint (piggyback bootstrap)" claim is **withdrawn**. It is
    **not** an audio/PII or write surface, so the "no new RLS/auth/write surface" invariant (§8)
    still holds; it adds one fail-closed control-plane round-trip on voice bootstrap (zero when truly
    dark — the build-arg short-circuits it).
- **Observability (<1 min):** coarse non-PII counters (§8) to existing telemetry; an error counter
  for fetch-fail / inference-timeout / capability-hidden. No server health surface needed.
- **Health degraded-vs-down:** voice has **no server dependency** beyond the static model GET →
  "down" = R2 unreachable (degrades to hidden); "degraded" = WASM-only slow path. Neither affects
  the menu/cart/checkout.
- **Scaling gate:** none server-side — client-side transcription adds 0 connections / 0 worker
  load (§2.4). That *absence* of a scaling gate is itself the design payoff.

---

## 10. Phasing (each phase: commit → staging → Playwright + unit proof; flag dark per role)

> **Active scope (operator 2026-06-30): client-facing flow only — storefront menu + checkout
> (READ_ONLY).** Phases 0–2 are in scope. **Phases 3 (admin) and 4 (courier) are REMOVED from the
> active roadmap and deferred to a separate future council — OUT of this build entirely.** They are
> listed below only to record the constraints that travel with them to that future council.

- **Phase 0 — PoC / decision gate.** Engine + Web Worker + `whisper-base` (q8) from R2, a
  transcribe-only debug overlay on `/s/:slug`, flag-dark. Capture the §4.2 sq/en/uk corpus on real
  devices. **GATE = §4.2 rule** → decides whether Phase 2 is scheduled.
- **Phase 1 — Client-facing storefront (menu + checkout-read-only) — the ACTIVE BUILD SCOPE.**
  MicFab on MenuPage; client intent pack (add / filter / sort / macro / compare / search) **plus
  the two checkout READ_ONLY intents** ("read my order" → reads the user's own cart + total; "go to
  checkout" → navigation — §6); confirm-then-execute (§6); sq/en/uk i18n via
  `scripts/i18n-add.ts`. `VITE_VOICE_CONTROL_ENABLED=false`. **Checkout adds zero stateful/money
  voice grammar** (no field writes, no place-order/payment — §6).
- **Phase 2 — Albanian fine-tune (CONDITIONAL on the Phase-0 gate).** Convert a `sq` fine-tune to
  ONNX, self-host on R2, locale-route `sq` to it. Only if Phase 0 was NO-GO/AMBER-unresolved.
- **Phase 3 (Admin intent pack) and Phase 4 (Courier intent pack) — REMOVED from the active
  roadmap; deferred to a SEPARATE FUTURE COUNCIL; OUT of this build (scope narrowing, operator
  2026-06-30).** Admin order-status voice and courier accept/reject/arrived voice are **not part of
  this build at all** — they re-open only if a future, independently-convened council takes up
  admin/courier voice. The design constraints already recorded for them (admin/courier need their
  **own** dangerous-misfire gate — confirm-fatigue, "reject 5"↔"accept 5" both in-grammar STATEFUL —
  §11 R-M; courier voice limited to **non-settling** actions with `arrived`/`completeDelivery` cash
  settlement **excluded by construction** §6/H4; **STOP-1's recorded human decision** §11 R-O) are
  **carried forward to that future council**, not enacted here. No admin/courier voice ships from
  this proposal/ADR.

**Deterministic proof — and an honest label of what it does NOT prove (H1):**
- **CI lane (deterministic, but proves only the matcher + wiring, NOT transcription):**
  - `SpeechProvider` = `WhisperProvider` (real) + **`MockProvider`** (scripted transcript) →
    Playwright injects the mock, "speaks", asserts cart/filter DOM with `toBeVisible()` /
    `toContainText()`. **This proves grammar→handler wiring + the gate's confirm / no-write / fail-
    closed invariant — it replaces transcription with a constant and says NOTHING about IRA or
    dangerous-misfire.**
  - A **unit corpus** of sq/en/uk utterances drives the IntentMatcher on **text** → asserts
    `{action,target}` incl. fuzzy/typo/below-threshold cases (`node --test`). **Matcher logic only.**
  - A capability test: graceful hide when the warmup probe fails / WebGPU absent; graceful error on
    mic-denied. The fail-closed unknown-kind test (§6/M4). The decline-path test (§8/STOP-2) **plus
    the visually-equal-affordance assertion (§8/C-2)**.
  - **The category-denylist test (§6/R2-B)** — a dietary-named category is not voice-auto-applied.
  - **The actor-anonymous telemetry-schema test (§8/C-1)** — counter record carries no
    `courier_id`/`user_id`/latency field.
  - **The fail-closed control-plane test (§9/R2-A)** — `/api/public/voice-config` fetch
    reject / `!res.ok` / `enabled !== true` ⇒ mic not rendered, engine not imported, model not
    fetched.
  - **The bundle/true-dark guardrail (§9/L2)** — engine absent from the main chunk when flag off.
  - **The debug-overlay-absence + public-deploy-arg guardrails (§8.1/R2-C)** — no transcript-capture
    path in a public build; `VITE_VOICE_TRANSCRIBE_DEBUG=false` asserted on every public deploy
    target.
- **Real-audio→matcher integration harness (the actual §4.2 gate — deterministic but NOT cloud-CI,
  H1):** the §8.1 consented WAV corpus is fed through the **real `WhisperProvider` → IntentMatcher**
  by a scripted `node` harness with the **pinned model + greedy/temperature-0 decode** →
  deterministic given fixed WAVs + pinned weights, but run as a gated local/self-hosted lane (needs
  the model + WASM/WebGPU). Its report **is** the per-locale gate artifact. **Green CI ≠ IRA passed.**

---

## 11. Open + accepted risks (owner named)

| ID | Risk | Disposition | Owner |
|---|---|---|---|
| R-A | Mid/low Android too slow/heavy for on-device inference (§2.2) | **Accept** — voice is WebGPU-gated; WASM-single-thread not offered (no COOP/COEP); measured warmup-probe floor hides the mic rather than OOM/timeout | FE |
| R-B | Base `sq` quality unknown | **Open** — the Phase-0 gate (§4.2) is the resolution | Spike |
| R-C | `@huggingface/transformers` + `onnxruntime-web` supply-chain + bundle size | **Open→mitigated** — quarantine in `packages/voice`, dynamic-import-only, pin+scan | Architect |
| R-D | Mis-hear in a noisy restaurant | **Accept residual** — confirm-then-execute + disambiguation; worst case = a reversible wrong **non-safety** filter. **Allergen/dietary filters are NO LONGER voice-settable (C1)** so a wrong filter can no longer be a false safety assertion; residual allergen-phrase misfire to *search* is reversible + counted in dangerous-misfire | Design |
| R-E | Mic perceived as surveillance | **Open→mitigated** — gesture-only, no wake-word (FORBIDDEN), on-device disclosure **with a real decline path + persistent off-setting (STOP-2)**, no persistence | Counsel |
| R-F | CSP must allow the R2 model origin (`spa-shell.ts:150`) | **Fix specified (L1)** — append first-party `r2ConnectSrc` to `connect-src`, **flag-gated so it's not widened while dark**; verify before deploy | FE/Security |
| R-G | Server-side transcription ever revisited | **Defer-flag** — a NEW ADR (worker role + pool budget + audio-PII DPIA). Counsel notes it would serve the WebGPU-excluded user better (equity-of-reach trade was *made, not discovered*) | Architect |
| R-H | IndexedDB model cache eviction → re-fetch (re-download cost to user) | **Accept** — $0 R2 egress; user re-fetches on a fresh device/cleared cache | FE |
| R-I | Phase-0 research corpus = retained voice PII (C2 / R2-D) | **Fix→governed (§8.1)** — separate consented regime, own RoPA, encrypted off-tenant store; deletion now **enforced by a scheduled job + expiry-audit tripwire + deletion-proof log** (R2-D), not by memory; **RLS accepted as the wrong tool** for a non-tenant artifact. **NEEDS-HUMAN:** assign the named data-controller **carrying the C-4 recruitment-ethics conditions** (non-coercive, not-our-workforce, fair pay, withdrawal, protocol-scoped, vulnerable-pop safeguard) before recording | Human (controller) |
| R-J | No demand evidence — building feasibility for assumed desire (Counsel §5) | **NEEDS-HUMAN-DECISION** — record a demand signal **or** explicit speculative acceptance ranked vs launch-blockers (B1/B2/B3) **before any Phase-1 code**; Phase-0 laptop probe may proceed (accept-risk, Phase-0 only) | Human (product) |
| R-K | WebGPU absent on iOS Safari + most social in-app WebViews → mic hidden for much of real traffic (H2) | **Accept-risk** — voice is strictly additive + flag-dark; the measured WebGPU-availability rate is a **REQUIRED printed field of the Phase-0 gate artifact** (Counsel C-3), not a promise | FE |
| R-L | Accessibility framing not earned (Counsel) | **Dropped for Phase 0/1** ("convenience for capable devices"). **Defer-flag (MISSING):** earning it needs TTS read-back + reckoning with whom the floor excludes; if accessibility is ever *cited as the reason* → NEEDS-HUMAN + close this gap first | Design/Product |
| R-M | No accuracy/dangerous-misfire gate defined for admin/courier vocab (H4) | **Defer-flag (MISSING)** — define the Phase-3/4 admin/courier dangerous-misfire gate **before Phase-3 code**; settling actions already excluded by construction (§6) | Architect |
| R-N | Gate measured on one tenant, generalized to all (M6) | **Fix (runtime per-tenant biasing) + Defer-flag (MISSING)** — multi-tenant/multi-menu re-measurement **before broad launch**; Phase 1 ships demo/pilot tenant first | Architect |
| R-O | Worker/courier voice = labour-surveillance gradient (STOP-1) | **Constraints baked now** (zero transcript/audio/per-worker-timing, never surfaced to manager, courier-opt-in) + the **actor-anonymous telemetry-schema guardrail is live from Phase 0/1** (Counsel C-1: no `courier_id`/`user_id`/latency column — the gradient is forbidden at the column before any worker mic exists). **Residual = NEEDS-HUMAN-DECISION** at the Phase-3/4 entry boundary; does NOT block Phase 0/1/2 | Human (ethics) |
| R-P | Runtime hot-kill defeated by the cache-first SW (R2-A) | **Fix** — kill rides a SW-exempt `/api/public/voice-config` `no-store` GET, **fail-closed** (absence = OFF), instant for returning visitors; honestly adds one control-plane call + one minimal read-only endpoint ("single external call" / "no new endpoint" narrowed) | FE/Ops |
| R-Q | Two-flag CSP/launch desync (R2-E) | **Fix** — single server authority `VOICE_CONTROL_ENABLED`+`VOICE_KILL`; the connect-src widening uses the **same predicate** as the config endpoint so a kill closes R2 too; one per-env var fans out to both client+server flags | FE/Security |
| R-R | Dietary-named category auto-applies → false-safety read (R2-B) | **Fix (denylist) + accept-risk (residual)** — dietary/allergen-token-named categories are not voice-auto-appliable (touch-only), single-source denylist + test; residual = a safety-implying name that evades the stem denylist, **counted** in dangerous-misfire | Architect/FE |
| R-S | Production zero-egress was policy, not a gate (R2-C) | **Fix** — debug-overlay forbidden on public builds by **guardrail** (bundle-absence + deploy-arg assertion + separate research build/host), the strongest privacy property finally machine-checked | FE/Security |
