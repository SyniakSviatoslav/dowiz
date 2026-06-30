# Voice Control — RESOLVE round (Architect dispositions)

> Register: System Architect. Inputs: `proposal.md` (v1), `breaker-findings.md`
> (C1,C2 / H1-H4 / M1-M6 / L1-L3), `counsel-opinion.md` (STOP-1, STOP-2,
> accessibility-inversion, demand-evidence). Date: 2026-06-30.
> Every row = exactly one disposition (**fix** / **accept-risk** / **defer-flag (MISSING)**)
> with the concrete change, justification, or recorded gap + a named owner.
> Goes back to Breaker + Counsel for re-attack. Honest about fixed-vs-deferred-vs-accepted.
>
> Grounding re-verified this round: CSP `connect-src` at `apps/api/src/lib/spa-shell.ts:150`
> has **no R2 origin** (R2 is only in `img-src` via `r2ImgSrc` line 144-148) → L1 confirmed.
> `setFilterAllergen` is a real storefront setter (`MenuPage.tsx:227`, toggled at `:832`,
> cleared at `:944`) → C1 confirmed.

---

## CRITICAL

### C1 · Allergen auto-apply → false safety signal, invisible to the gate — **FIX**
**Change (proposal §6 capability table + §4.2 metric + ADR decision 5):**
1. **Allergen / dietary-safety filters are removed from voice ENTIRELY** — same class as
   money. There is **no `kind`** a voice intent can carry that maps to `setFilterAllergen`
   (or any future dietary/"safe-for-me" filter). `setFilterAllergen` is removed from the
   voice-reachable setter set; the allergen chips (`MenuPage.tsx:832`) stay **touch-only**.
   - *Why not merely CONFIRM-gate it:* a confirm chip that echoes the (possibly mis-parsed)
     allergen — "Show nut-free? ✓" — is **itself a safety assertion** the user trusts, and the
     chip cannot validate that the matched allergen is the one meant, nor that the negation
     ("pa arra" / "without nuts") survived ASR. Negations + short function words are the
     weakest part of small-model ASR (the C1 break). Confirm does not bound this; exclusion does.
2. **Dangerous-misfire is redefined** (closes the "blind by construction" half): it now counts
   *any* resolution to a wrong **STATEFUL** intent **OR** any resolution that produces a state a
   user could read as a **dietary/safety assertion**. Since allergen is no longer voice-settable,
   the residual is an adjacent misfire (e.g. a spoken allergen phrase fuzzy-matching a search) —
   such cases are counted and bounded, not excluded.
3. **Guardrail (red→green):** a table test asserts no allergen/dietary `kind` exists in the
   grammar and `setFilterAllergen` is **not** in the voice-reachable setter set — exactly the
   shape of the existing "no `place_order`/`pay`/`checkout` kind" test.
**Owner:** Architect (capability table) + FE (setter exclusion). **Residual:** an allergen phrase
may still land on READ_ONLY *search* — reversible, shows results, asserts no safety state. Accepted.

### C2 · The Phase-0 gate manufactures the retained PII corpus §8 forbids — **FIX** (option a)
**Change (new proposal §8.1 "Phase-0 research corpus — a separate data regime" + §4.2 + §5 + ADR):**
The contradiction is dissolved by **separating two data regimes** and naming the governance of each:
1. **Production storefront runtime** keeps the zero-egress / zero-persistence invariant
   **absolutely, unchanged.** No customer audio or transcript is ever captured, stored, or sent.
   The `VITE_VOICE_TRANSCRIBE_DEBUG` overlay is **forbidden on any public deployed `/s/:slug`
   serving real customers** — it runs only inside the consented research session below.
2. **The Phase-0 evaluation corpus is a separate, consented research artifact — NOT a storefront
   collection.** It is governed as its own data-processing activity:
   - **Legal basis:** explicit, informed, written consent from **recruited adult speakers**
     (not live customers), in `sq`/`en`/`uk`, stating purpose (ASR accuracy eval), retention,
     and deletion rights.
   - **Not personal-special-category:** ASR ≠ speaker identification → **not Art-9 biometric**
     (Counsel correction adopted); it is ordinary personal data under consent.
   - **Storage:** one access-controlled, encrypted location **outside** any tenant DB and
     **outside** the public R2 bucket; no tenant RLS surface; no production system reads it.
   - **RoPA:** the corpus **DOES** get its own RoPA entry + retention schedule + a named
     controller/owner. (Corrects §8's blanket "no new RoPA entry" — that holds for the
     *production runtime* only.)
   - **Retention / deletion:** time-boxed — deleted within **90 days** of the gate decision or on
     Phase-2 completion (whichever is first), with a **documented deletion proof**. AMBER
     re-measure happens **inside** that window or the corpus is re-consented and re-collected.
   - `scripts/compliance-gate.ts` records the corpus as a documented research dataset.
**Owner:** named human data-controller (NEEDS-HUMAN to assign before any real-device recording) +
Architect (regime separation). **This is the only correct read of the gate; nothing is deferred.**

---

## HIGH

### H1 · MockProvider/​corpus prove DISJOINT halves → CI green is a false-green — **FIX**
**Change (proposal §10 honesty label + a real-audio integration harness):**
- **Label what CI proves, honestly.** The cloud-CI lane (`MockProvider` Playwright + text unit
  corpus) proves **only** (a) grammar→handler wiring + the gate's confirm / no-write invariant,
  and (b) IntentMatcher fuzz/threshold logic **on text**. It proves **nothing** about
  transcription, IRA, or dangerous-misfire. The proposal now says this in those words.
- **Add the real-audio→matcher integration check.** A fixed set of pre-recorded WAVs (the C2
  consented corpus) is fed through the **real `WhisperProvider` → IntentMatcher** by a scripted
  `node` harness with the **pinned model + greedy/temperature-0 decode** → deterministic given
  fixed WAVs + pinned weights. It is **not** in cloud CI (needs the model + WASM/WebGPU); it runs
  as a gated local/self-hosted harness whose report **is** the §4.2 gate artifact.
- Result: "deterministic proof" is split into *CI-deterministic (matcher/wiring)* and
  *harness-deterministic (real-audio gate)* — both real, neither pretends to be the other.
**Owner:** Spike/FE. No residual hidden; the gate is explicitly a separate artifact from CI-green.

### H2 · The WASM-threads rung doesn't exist on the storefront → WebGPU-or-hidden — **FIX** + **accept-risk** (reach)
**Change (proposal §2.2 + §7 collapse the 3-rung ladder to an honest 2-state):**
- We will **NOT** enable cross-origin isolation (`COOP: same-origin` + `COEP`) on the storefront:
  it would break the existing first-class cross-origin resources verified in the live CSP
  (`cdn.jsdelivr.net`, `tiles.openfreemap.org`, `router.project-osrm.org`, `plausible.io`,
  Google Fonts). Therefore `SharedArrayBuffer` / WASM **threads are unavailable by deliberate
  decision**, and WASM-single-thread (15-40 s+) is below the floor. **Net posture: voice is
  WebGPU-gated on the storefront.** The "WASM degrades gracefully" middle rung is **removed** from
  the doc as fiction; WASM-single-thread is **not offered** (a slow mic is a latency lie — Counsel).
- **accept-risk (reach):** social in-app WebViews (Instagram/TikTok/FB) frequently lack WebGPU →
  mic **hidden** for much of the dominant social-entry traffic. Tolerable because voice is
  **strictly additive** (touch fully works) and flag-dark. **Mandatory measurement:** the Phase-0
  gate report MUST record the **real WebGPU-availability rate** across a representative
  device/WebView sample, so "who can actually use this" is a measured number (feeds the
  accessibility relabel + the demand question), not an assumption.
**Owner:** FE (posture) / Architect (no-COOP-COEP decision recorded). **Residual:** shrunken
addressable surface — explicitly accepted and cross-referenced to demand + accessibility.

### H3 · `en`/`uk` ship "first-class" but the safety gate is `sq`-only — **FIX**
**Change (proposal §1 relabel + §4.2 per-locale gate + ADR decision 4):**
- The §4.2 dangerous-misfire bound (and IRA) is now required **per shipped locale**. A locale
  launches its voice intents **only after passing its own** dangerous-misfire measurement on its
  own corpus. `sq`, `en`, `uk` each carry an independent gate; a locale that hasn't passed ships
  **dark for that locale** even if strings exist.
- "First-class" is relabelled honestly: **first-class i18n string parity** (the existing parity
  gate) **+ an independent per-locale safety gate**. String parity is not safety parity.
**Owner:** Spike (corpora) / Architect (gate). No residual — each locale is self-gated.

### H4 · Phases 3/4 bind voice to cash settlement → "money has no voice binding, ever" is false — **FIX** + **defer-flag** (3/4 thresholds)
**Change (proposal §1 + §6 + §10 + ADR decision 5):**
1. **Narrow the absolute claim to the true one:** "In **Phase 0/1/2** (customer storefront) there
   is **no** voice binding to any money or order-finalization path. Phases 3/4 are a **separate,
   independently-gated scope NOT approved by this ADR**." The unqualified "ever" is removed.
2. **Exclude cash-settling actions from voice by construction.** Per the cash-as-proof spine
   (ADR-0009/0013), courier `arrived`/`completeDelivery` **settles cash** = a money-state
   transition → it gets the **same treatment as `place_order`/`pay`/`checkout`: no `kind`, ever.**
   If Phase 4 happens, courier voice is limited to **non-settling** actions (e.g. accept/reject an
   offer, navigation toggles), each with its own gate **and** STOP-1's human decision.
3. **Phase 3/4 get their own accuracy + dangerous-misfire gate** on the admin/courier vocabularies
   (confirm-fatigue: "reject 5"↔"accept 5" are both in-grammar STATEFUL — exactly the
   dangerous-misfire class) — **no Phase 3/4 voice ships without it.** The *requirement* is fixed
   now; the *threshold values* for admin/courier vocab are Phase-3 scope → **defer-flag (MISSING):
   define the admin/courier dangerous-misfire gate before Phase 3 code.**
**Owner:** Architect. **Residual:** confirm-fatigue on non-settling admin/courier actions —
bounded by the Phase-3/4 gate (deferred) + STOP-1.

---

## MEDIUM

### M1 · n=150 can't resolve a 2% threshold → fictional precision — **FIX**
**Change (proposal §4.2 gate statistics):**
- The safety metric is gated on the **upper 95% Wilson CI bound**, not the point estimate: a
  one-sided **safety bound**. To make ≤2% upper-bound achievable, **corpus size ≥ 300 per
  locale** for the safety metric (0 events / 300 → upper-95% ≈ 1.2%; ≤1/300 → ≈ 1.7%). n=150 was
  under-powered and is corrected.
- IRA is gated on **lower 95% CI ≥ 80% AND point estimate ≥ 85%** (CI reported), not a bare point.
- Anti-gaming: corpus from **≥15 distinct speakers**, both noise conditions, a documented
  recording protocol, double-checked labels, and **deliberate adversarial near-miss pairs**
  (accept/reject, with/without, allergen-adjacent).
**Owner:** Spike. No residual — the gate now states its own statistical power.

### M2 · Memory floor undercounts host page + GPU copy; `deviceMemory` unreliable — **FIX**
**Change (proposal §2.2):**
- Corrected arithmetic: realistic peak = model weights + ONNX arena **+ the already-loaded 49-product
  menu (decoded photos + React tree ≈ 100-200 MB) + the WebGPU GPU-buffer copy (~another model
  size, shared-memory on Android)** → **≈ 500-700 MB** peak on a 3-4 GB device with the menu open.
  Stated as tab-crash territory, not "borderline."
- **The capability floor no longer relies on `navigator.deviceMemory`** (absent on iOS Safari,
  coarse/capped on Android). Replaced with a **bounded runtime warmup probe**: gate on WebGPU
  adapter presence + a guarded tiny warmup transcription under a hard time + OOM `try/catch`
  budget; **if the device cannot complete the warmup within budget, hide the mic.** Capability is
  **measured**, not predicted. `deviceMemory` is demoted to a soft hint only.
**Owner:** FE. No residual — the floor is now an actual measurement on the actual device.

### M3 · "Cannot mutate by construction" is import-grep, not types → runtime callback bypass — **FIX**
**Change (proposal §6 invariant + enforcement):**
- **Invert control so no handler ever crosses the boundary.** The engine is a **source**; the
  `ConfirmationGate` (in `apps/web`) is the **sink** that *pulls* `IntentProposal` values (typed
  async iterator / event stream of plain readonly data). The engine's public API accepts **no
  callback / function-typed parameter** that could close over `addItem`/`handleUpdateStatus`.
- **Guardrails strengthened to cover runtime callbacks, not just static imports:** (1) an
  `eslint-plugin-local` rule banning **function-typed parameters** in `packages/voice`'s exported
  signatures (no `(...) => …` handler params crossing in); (2) `IntentProposal` typed as a pure
  `readonly` data record (no methods); (3) the existing import-grep stays. The "no-write
  guarantee" now covers injected closures.
**Owner:** Architect/FE. No residual — there is no surface to inject a write-capable closure into.

### M4 · Default classification for an unlisted kind is unspecified → fail-open-shaped — **FIX**
**Change (proposal §6 gate default):**
- The capability table is a **total function via an exhaustive `switch` whose `default` REJECTS** —
  an unlisted/unmapped `kind` is **dropped** ("didn't catch that"), executed **neither** as
  auto-apply **nor** as STATEFUL-confirm. **Fail-closed.**
- Plus a **TypeScript exhaustiveness check** (`never` assertion) so a new grammar `kind` without a
  table row **fails the build**, and a **guardrail unit test**: unknown/unmapped kind → assert no
  handler called and no auto-apply.
**Owner:** Architect. No residual — fail-closed by construction, by type, and by test.

### M5 · Kill-switch is a build-arg → no runtime hot-kill; "instant rollback" false — **FIX**
**Change (proposal §9 + §7):**
- Add a **server-served runtime kill signal** piggybacked on an **already-fetched bootstrap
  payload** (the tenant/menu bootstrap the storefront already loads) — **no new endpoint** (keeps
  the §8 invariant). MicFab activates only if **build-flag true AND runtime-kill not set**; the
  **engine dynamic-import and model fetch are gated behind the same runtime check**, so a hot-kill
  also stops new fetches. An operator can disable the mic across all clients by flipping the served
  value — **runtime-instant**, no rebuild. The build-arg remains the true-dark default.
**Owner:** FE/Ops. No residual — rollback is now a runtime flip, with the build-arg as belt-and-suspenders.

### M6 · Matcher tuned/measured on the demo tenant, shipped to all — **FIX** + **defer-flag** (multi-tenant validation)
**Change (proposal §4.2 + §8 + §10):**
- **Remove any shipped static demo-vocabulary decoding bias.** Slot-fill / decoding bias is
  computed **per-tenant at runtime from that tenant's own already-loaded menu vocabulary** (which
  the per-tab matcher already holds). The AMBER tuning line "decoding bias toward *the* tenant
  vocabulary" is rewritten to "**runtime per-tenant** vocabulary biasing." No demo-fitted asset
  ships to other tenants.
- **Relabel the gate scope honestly:** the demo-menu IRA validates the **pipeline mechanics**
  (transcribe + matcher), it is **not a per-tenant launch certification.** Phase 1 ships to the
  demo/pilot tenant first.
- **defer-flag (MISSING):** a **multi-tenant (or representative multi-menu) accuracy
  re-measurement before broad multi-tenant launch** — out of single-pilot Phase-0/1 scope, recorded
  as a gap to close before broad rollout.
**Owner:** Architect. **Residual:** broad-launch accuracy unvalidated → deferred, gated.

---

## LOW

### L1 · Storefront CSP does not allow the model origin — **FIX** (exact change specified)
**Change (proposal §8 + §9 + ADR; file `apps/api/src/lib/spa-shell.ts:150`):**
- Before any **deployed** voice phase, add the R2 model origin to **`connect-src`** — derive an
  `r2ConnectSrc` from `R2_PUBLIC_URL` exactly like the existing `r2ImgSrc` (lines 144-148) and
  append it to the `connect-src` list (currently `'self' https://cdn.jsdelivr.net
  https://tiles.openfreemap.org https://router.project-osrm.org https://en.wikipedia.org
  https://plausible.io`). `worker-src 'self' blob:` already covers the worker; `script-src`
  already has `'unsafe-eval'` (covers WASM compile). **Only** the first-party R2 origin is added.
- **Gate the connect-src addition on the server-visible voice flag** so the hardened CSP is **not
  widened while voice is dark** → preserves "true dark = zero surface delta."
**Owner:** FE/Security. No residual — scoped, first-party, flag-gated.

### L2 · "Zero bundle delta — verified, not asserted" has no verification — **FIX** (relabel + named guardrail)
**Change (proposal §9 + §10):**
- The word **"verified" is removed** until proof exists; the claim is downgraded to **"intended
  true-dark, to be verified by guardrail before Phase-1 launch."**
- The named guardrail (a Phase-0/1 **exit criterion**, red→green): an import-graph / built-bundle
  assertion that the storefront **main chunk contains none** of
  `@huggingface/transformers` / `onnxruntime-web` / the `packages/voice` engine when the flag is
  off — i.e. the engine is reachable **only** via dynamic import. CI fails if an eager import leaks
  it into the critical path.
**Owner:** FE. **Honest status:** the dishonest "verified" is fixed **now**; the guardrail is a
named, required exit criterion (not yet built) — that part is correctly a Phase-0/1 exit gate, not
a claim of completion.

### L3 · §2.1 q8 byte count is internally inconsistent — **FIX** (honesty)
**Change (proposal §2.1 + propagate to §2.2):**
- Stop labelling the artifact "q8." The Transformers.js whisper-base ONNX export is
  **mixed-precision** (quantized matmuls + retained fp32 layernorm/embeddings), so ~130 MB
  transferred is expected and is **not** reconcilable with 74 MB raw int8 × 1. Relabelled as
  **"mixed-precision quantized (~130 MB *measured target*)."** The §2.2 memory floor must be
  **recomputed from the measured artifact size** (feeds M2). The byte size remains a Phase-0
  measurement output.
**Owner:** Architect. No residual.

---

## Counsel ETHICAL-STOPs

### STOP-1 · Worker/courier voice = labour-surveillance gradient — **REVISE (partial dissolve) + NEEDS-HUMAN-DECISION (Phase-3/4 entry)**
- **Dissolved now (baked as hard, load-bearing design rules, not buried prose):** Phase 3/4
  courier/worker voice, **if** ever built, **MUST** (a) retain **zero** transcript/audio (same as
  storefront), (b) emit **no** per-worker timing/latency telemetry, (c) **never** surface any
  voice-derived data to a manager/owner view, (d) be **courier-opt-in** with touch fully
  equivalent. Encoded as a **guardrail requirement** (red→green) gating Phase 3/4.
- **Residual = NEEDS-HUMAN-DECISION:** the decision to *enter* Phase 3/4 at all (a worker's mic
  under a manager's platform) is a conscious human/ethics call. Counsel does not override a human,
  but the decision must be **recorded** at the Phase-3 boundary. **Does not block Phase 0/1/2.**
- **Owner:** human (ethics/product) at the Phase-3 gate; Architect records the constraints now.

### STOP-2 · Disclosure-sheet dark-pattern (OK-only soft-confirm) — **REVISE (dissolved by design)**
- **Change (proposal §8 consent UX):** the first-run disclosure is **not** an OK-only gate. It
  offers a genuine choice — **"Use voice"** vs **"Not now / use touch"** — where "Not now" is the
  no-op default and leaves touch fully functional. It is backed by a **persistent on/off voice
  setting** in storefront prefs (like `dos_menu_prefs`), not just a dismissable sheet. Honours
  "soft-confirm-не-пастка" / honest-UI / consent-as-real-choice.
- **Guardrail:** a test asserting the disclosure's decline path leaves the mic **unactivated** and
  touch working. **STOP-2 dissolved.** **Owner:** FE/Design.

### Accessibility-inversion — **DROP the framing (honest relabel)**
- Phase 0/1 voice is WebGPU-gated (H2) and its confirm/disclosure UI is text → it routes the
  benefit **away** from the low-end / elderly / low-literacy users an accessibility claim would
  invoke. So **the accessibility justification is explicitly NOT claimed.** Phase 0/1 is relabelled
  as **"a convenience for capable (WebGPU) devices."**
- **defer-flag (MISSING):** to ever *earn* the accessibility framing requires (a) **audio confirm
  read-back (TTS)** for low-literacy, and (b) an honest reckoning with whom the WebGPU floor
  excludes (possibly the rejected server-side path for the excluded users — Counsel's secondary
  steel-man). If accessibility is ever cited as the *reason* for voice, that is **NEEDS-HUMAN** +
  this gap must be closed first. **Owner:** Design/Product.

### Demand-evidence open question — **NEEDS-HUMAN-DECISION (recorded) + accept-risk for Phase-0 only**
- Not an engineering finding a design change can close. Recorded as a hard sequencing gate:
  **before any Phase-1 engineering spend**, a human product decision must record either (a) a
  demand signal (even qualitative — pilot-tenant request, user interviews), **or** (b) explicit
  acceptance that voice is speculative/strategic, **ranked against the open launch-blockers**
  (B1 inverted-money, B2 dispatch, B3 RLS — the actual MVP NO-GO per project memory + Counsel §3).
- **accept-risk (Phase-0 only):** the Phase-0 **laptop probe** stays cheap and may proceed to
  produce the feasibility number; **no Phase-1 code** until the demand decision is recorded and the
  launch-blockers are addressed. **Owner:** human (product).

---

## Disposition tally
- **fix:** C1, C2, H1, H2(+accept), H3, H4(+defer), M1, M2, M3, M4, M5, M6(+defer), L1, L2, L3,
  STOP-2 (dissolved), accessibility (relabel/drop).
- **accept-risk:** H2 reach (FE), demand Phase-0-only (product), C1 residual search-match.
- **defer-flag (MISSING):** H4 admin/courier dangerous-misfire thresholds (pre-Phase-3);
  M6 multi-tenant accuracy validation (pre-broad-launch); accessibility-earn requirements
  (pre any accessibility claim); STOP-1 Phase-3/4 entry constraints' human decision.
- **NEEDS-HUMAN-DECISION:** STOP-1 Phase-3/4 entry; demand evidence; C2 data-controller assignment;
  accessibility-as-reason (if ever cited).
- **Nothing marked fixed that is only intended later** — L2's guardrail and the deferred gates are
  labelled as exit criteria / gaps, not completions.

---

## ROUND 2 RESOLUTION (Architect dispositions on the re-attack)

> Inputs: `breaker-findings.md` **## ROUND 2** (R2-A,R2-C HIGH; R2-B,R2-D,R2-E MED; R2-F LOW) +
> `counsel-opinion.md` **## ROUND 2** (cheap strengthenings, not vetoes). Round-1 rows stand above.
> Grounding re-verified READ-ONLY this round:
> - `apps/api/public/sw.js` — single-line SW: `fetch` handler exempts only `/api/`, `/ws/`,
>   `sw.js`, `.webmanifest`; everything else GET → `caches.match(req).then(a => a || fetch(req))`
>   (**cache-first, HTTP cache semantics ignored**). Cache name flips only on an
>   `UPDATE_CACHE_VERSION` message (a redeploy). → **R2-A confirmed.**
> - `apps/api/src/lib/spa-shell.ts:150` — CSP built **server-side per request**; `connect-src` =
>   `'self' …jsdelivr …openfreemap …osrm …wikipedia …plausible` (**no R2**); `r2ImgSrc` (`:144-148`)
>   is `img-src`-only. → **R2-E confirmed** (and the per-request build means a server flag is a
>   viable single source of truth).
> - `apps/web/src/pages/client/MenuPage.tsx:474` — `setSelectedCategory` filters by tenant
>   free-text `_catId` (`:301`); categories are tenant-defined. → **R2-B confirmed.**

| Finding | Sev | Disposition | One-line |
|---|---|---|---|
| R2-A | HIGH | **FIX** | Kill rides an `/api/`-prefixed, SW-exempt, `no-store`, **fail-closed** control-plane GET; "single external call" honestly narrowed to two. |
| R2-C | HIGH | **FIX** | Debug-overlay zero-egress becomes a red→green guardrail: bundle-absence assertion + deploy-arg assertion + separate research build target / non-public host. |
| R2-B | MED | **FIX** + accept-risk (residual) | Close the **class**: a category whose name matches a dietary/allergen token denylist (sq/en/uk) is **not** voice-auto-appliable (touch-only); denylist + test guardrail. |
| R2-D | MED | **FIX** (enforcement mechanism) + accept-risk (outside-RLS) | Scheduled deletion job keyed to a per-item expiry manifest + an expiry-audit tripwire + deletion-proof log; RLS is the wrong tool for a non-tenant research artifact (accepted, named controller owns it). |
| R2-E | MED | **FIX** | **Single source of truth = server `VOICE_CONTROL_ENABLED` (+ `VOICE_KILL`)**; the connect-src R2 widening uses the **same predicate** as the config endpoint's `enabled`, so a hot-kill also closes the R2 origin; one per-env var fans out to both client build-arg and server env. |
| R2-F | LOW | **FIX** | Engine emission surface is a typed **`AsyncIterable<IntentProposal>`** (no `on(event,cb)`/`subscribe({onResult})`); the no-function-typed-param guardrail and the API are now coherent. |
| Counsel C-1 | — | **FIX** | Actor-anonymous telemetry **schema guardrail NOW** (Phase 0/1): assert the counter record carries no `courier_id`/`user_id`/latency field. |
| Counsel C-2 | — | **FIX** (exit criterion) | Disclosure "Use voice" / "Not now" asserted **visually equal** affordance (same weight class, not primary-vs-ghost) — guardrail, not just functional. |
| Counsel C-3 | — | **FIX** | Measured WebGPU-availability rate (R-K) is a **required field** of the Phase-0 gate artifact, not a promise. |
| Counsel C-4 | — | **FIX** | §8.1 corpus consent conditions baked: non-coercive recruitment (**explicitly NOT the platform's own couriers/workforce**), fair pay, withdrawal right, protocol-scoped consent, vulnerable-population safeguard — the gate on real-device recording. |

### R2-A · Runtime hot-kill defeated by the cache-first service worker — **FIX**
**Root cause (grounded).** The M5 fix piggybacked the kill on the menu/info bootstrap payloads.
`sw.js` serves every non-`/api/` GET **cache-first** and ignores HTTP cache headers, so a returning
visitor who loaded the menu once keeps the **stale pinned payload** (kill not set) and never hits
the network → MicFab keeps activating/crashing. `cache: 'no-store'` does **not** help: the SW
intercepts *before* the network request's cache mode matters, for any non-`/api/` URL. The cache
only flips on `UPDATE_CACHE_VERSION` (a redeploy) — exactly what M5 claimed to avoid.
**Change (proposal §9 kill-switch + §7 failure table + §8 + ADR).**
1. The runtime kill is read from a **dedicated `/api/`-prefixed control-plane GET** —
   `GET /api/public/voice-config?slug=:slug` — which the SW **exempts by construction**
   (`pathname.startsWith('/api/')` → straight to network, never cached). Fetched with
   `cache: 'no-store'` as belt-and-suspenders. This is the **only** SW-reliable bypass; the
   bootstrap-piggyback is abandoned.
2. **Fail-closed default.** Voice activates **only** on an explicit `{ enabled: true }` from a
   `res.ok` response. Any of {fetch rejects, `!res.ok`, non-JSON, `enabled` absent/`undefined`/not
   `=== true`} ⇒ **voice stays OFF** (mic not rendered, engine not imported, model not fetched).
   Absence is OFF, not on. The check runs **before** the engine dynamic-import and before the
   model fetch, so a kill also stops new model fetches.
3. **Honest reconciliation of the "single external call" claim.** This adds a **control-plane
   read**, so the voice path now has **two** external calls, not one: (a) the cheap `/api/`
   `no-store` voice-config GET (fail-closed, no PII, no DB write — a read of a flag), and (b) the
   R2 model GET. The "exactly one external call / no transcription server to fall over" reliability
   property is **narrowed honestly**: there is still no audio/transcription server; the new call is
   a tiny fail-closed control-plane flag whose **failure mode is "voice OFF," never a page block or
   a cascade.** The earlier "no new endpoint (piggyback bootstrap)" claim is **withdrawn** — a real,
   minimal, read-only `/api/public/voice-config` endpoint is required (it is **not** an audio/PII or
   write surface, so the "no new RLS/auth/write surface" invariant still holds).
**Owner:** FE/Ops. **Residual:** one extra control-plane round-trip on voice bootstrap (gated by
build-flag, so zero when truly dark) — accepted; it is the price of a real hot-kill.

### R2-C · Production zero-egress is policy, not a guardrail — **FIX**
**Root cause.** §8.1 forbids `VITE_VOICE_TRANSCRIBE_DEBUG` on public deployed `/s/:slug` by **prose
only** — the single strongest privacy property is the only one without a red→green gate, while a
research "record on real devices" naturally deploys a debug build to the publicly-reachable staging
`/s/:slug` (which also serves the demo tenant, E2E traffic, live claimable demos — non-consented).
**Change (proposal §8.1 + §9 + §10 + ADR) — three machine-checks make "serving real customers"
structural:**
1. **Bundle-absence guardrail (red→green, like the §9/L2 true-dark check).** A built-bundle
   assertion that **no** transcribe-debug overlay / transcript-surfacing code path is present in
   the storefront bundle when `VITE_VOICE_TRANSCRIBE_DEBUG` is not `true`. The overlay + any
   transcript-capture function are gated behind `import.meta.env.VITE_VOICE_TRANSCRIBE_DEBUG` so
   they tree-shake out; CI greps the built chunks and **fails** if the symbol survives.
2. **Deploy-arg guardrail.** A CI assertion that every **public** deploy target (prod + the public
   staging `/s/:slug`) carries `VITE_VOICE_TRANSCRIBE_DEBUG=false`. The deploy **fails** if a
   public target sets it `true`. "Serving real customers" = "deploy target is public" = a
   machine-checkable predicate on the deploy matrix, not a sentence in a doc.
3. **Structural separation of the research build.** The debug/transcribe build is a **distinct
   build target** deployed **only to a separate non-public / throwaway research host** (never the
   public `/s/:slug`). The CI env-matrix check forbids the debug target from the prod/staging-public
   lanes. The consented research session runs there, not on customer-serving infrastructure.
**Owner:** FE/Security. **Residual:** none for the runtime invariant — it is now gated, not promised.

### R2-B · C1 closed the allergen *setter*, not the *class* — **FIX** + accept-risk (residual)
**Root cause (grounded).** `setSelectedCategory` stays READ_ONLY auto-apply, but categories are
**tenant-defined free text** filtered by `_catId`. A tenant category named *"Pa gluten"* / *"Vegan"*
+ user *"shfaq pa gluten"* → fuzzy match → **auto-applies, narrows the menu, the user reads it as
allergen-safe** — the exact false-safety read C1 was CRITICAL for, re-opened through a still-open
setter, and **worse** than the confirm path C1 rejected because there is no checkpoint at all.
**Change (proposal §6 capability table + §4.2 + ADR decision 5) — close the class.**
Category voice-navigation stays (it is useful), **but a category is not voice-auto-appliable if its
normalized name matches a dangerous dietary/allergen token denylist** — such a match is **dropped**
(treated like allergen: **touch-only**), never auto-applied. The denylist is a **single-source
stem list across sq/en/uk** (conservative/broad):
- **sq:** `pa arra`, `pa gluten`, `pa laktoz*`, `pa qumësht`, `pa veze`, `pa sheqer`, `vegan`,
  `vegjetar*`, `vege`, `bio`, `organik*`, `alergj*`, `çeliak*` / `celiac`, `halal`, `kosher`;
- **en:** `vegan`, `vegetarian`, `gluten`, `gluten-free`, `dairy-free`, `nut-free`, `lactose`,
  `allergen`, `allergy`, `bio`, `organic`, `halal`, `kosher`, `sugar-free`;
- **uk:** `веган`, `вегетаріан`, `глютен`, `без глютену`, `без лактоз*`, `безлактозн*`,
  `алерг*`, `органічн*`, `без цукру`, `халяль`, `кошер`.
Match rule: a category name is denylisted if its normalized form **contains** any stem (substring /
stemmed contains), evaluated for all three locales regardless of the active locale (a tenant may
name categories in any language). A denylisted-named category → voice match **dropped**.
**Guardrail (red→green):** the denylist is one exported constant; a unit test feeds category names
(incl. `"Pa gluten"`, `"Vegan"`, `"Без глютену"`) through the voice-category matcher and asserts a
denylisted-named category is **not** auto-applied; the §4.2 corpus **must** include such near-miss
pairs (extends the M1 adversarial set). Dangerous-misfire (already redefined in C1 to count a
"state read as a dietary/safety assertion") **counts a category auto-apply read as safety**, so any
residual is bounded by the ≤2% upper-CI gate.
**Owner:** Architect (denylist + matcher rule) / FE (category setter wiring). **Residual
(accept-risk):** a tenant could name a safety-implying category that evades the stem denylist (e.g.
a brand-shaped name). The denylist is heuristic, not total; this residual is **counted** in
dangerous-misfire (not excluded) and accepted, with the broad conservative stem list as mitigation.

### R2-D · Corpus PII outside RLS and outside automated enforcement — **FIX** (mechanism) + accept-risk (RLS)
**Root cause.** ≈900+ identifiable voice recordings + transcripts in an encrypted off-tenant store
with "no RLS surface"; 90-day deletion + access control are manual/policy; `compliance-gate`
documents existence but does not **enforce** deletion or detect a lapse — unlike every other PII
surface (ENABLE+FORCE RLS + gate). Lapsed AMBER-tuning copies past 90 days have no tripwire.
**Change (proposal §8.1) — name who/what enforces deletion automatically, not by memory.**
1. **Scheduled deletion job.** Each corpus item is written with an **expiry timestamp** in a
   manifest (recording date + retention window). A **scheduled job** (cron) deletes every item past
   its expiry and **writes a deletion-proof entry** (item id, expiry, deletion time) to an audit log.
   Deletion is automatic, keyed to data, not to a human remembering.
2. **Expiry-audit tripwire.** A scheduled assertion that **no** manifest item exceeds its retention
   expiry; on a violation it **alarms and fails `compliance-gate`** — the automated analogue of the
   "ENABLE+FORCE RLS + gate" tripwire every other PII surface has.
3. **AMBER window.** Re-measurement must occur **inside** the retention window or the corpus is
   re-consented + re-collected; the job enforces this by deleting on expiry regardless.
**Honest accept-risk (RLS).** The corpus is a **non-tenant research artifact**, so RLS — a
row-level *tenant-isolation* mechanism — is the **wrong tool**; access control + the scheduled
job + the expiry tripwire + the named controller are the right ones. The "outside RLS" structural
point is **accepted** (not a gap to close with RLS); the "enforcement by memory" gap is **fixed**
by the job + tripwire.
**Honest status of completeness.** This regime only exists **once real-device recording is
authorized** (NEEDS-HUMAN: controller + recruitment ethics, below). The job + tripwire are
specified now as a **gate on recording** (must exist before the first recording) — they are
**not "built"**, because recording has not been authorized. Marked **fix-specified**, not
fix-complete. **Owner:** named human data-controller (NEEDS-HUMAN) + Architect (mechanism spec).

### R2-E · CSP gates on a nonexistent server flag; two-flag desync — **FIX**
**Root cause (grounded).** §8/L1 gate the connect-src R2 widening on a "server-visible voice flag"
that does not exist: the launch flag is `VITE_VOICE_CONTROL_ENABLED` (a **client** build-arg,
invisible to `spa-shell.ts:150`). Desync: (1) client ON / server unset → model fetch blocked by CSP
→ voice silently dies; (2) server set / client dark → hardened CSP widened to R2 while dark; (3)
after an R2-A hot-kill, connect-src stays open.
**Change (proposal §8 CSP + §9 + ADR) — one server-side source of truth.**
- **Define the server authority:** `VOICE_CONTROL_ENABLED` (server env, read **at request time** in
  `spa-shell.ts`, matching its existing per-request CSP build) **+ `VOICE_KILL`** (the runtime
  hot-kill). The **same predicate** `VOICE_CONTROL_ENABLED && !VOICE_KILL` drives **both**:
  (a) the `/api/public/voice-config` endpoint's `enabled` (R2-A), and (b) the `connect-src` R2
  widening (`r2ConnectSrc` derived from `R2_PUBLIC_URL` like `r2ImgSrc`). Because it is the **same
  predicate**, a hot-kill (`VOICE_KILL=true`) returns `enabled:false` **and** drops the R2 origin
  from `connect-src` on the next document load → **a killed voice closes the R2 origin too.**
- **Sync mechanism (kills the desync):** one **per-environment variable** in the deploy pipeline
  fans out to **both** the client build-arg `VITE_VOICE_CONTROL_ENABLED` **and** the server env
  `VOICE_CONTROL_ENABLED` — they flip **together** in a single deploy, never independently. The
  client build-arg is **only** the true-dark tree-shake gate; the **server flag is authoritative**
  at runtime (the client always asks `/api/public/voice-config` before activating — R2-A
  fail-closed), so case (1) becomes "config says off → client never attempts the model fetch" (no
  silent death) and case (2) "widened-while-dark" is removed because the predicate also requires the
  feature to be enabled. **Single source of truth, one predicate, runtime-killable.**
**Owner:** FE/Security/Ops. **Residual:** none — the two flags become one authority + a derived
build gate.

### R2-F · async-iterator engine surface vs the no-function-param guardrail — **FIX**
**Root cause.** §6 offered an "event stream" while banning function-typed params; `on(event, cb)`
**is** a function-typed param (the guardrail forbids the offered API), and `subscribe({onResult})`
slips a top-level-only rule.
**Change (proposal §6 + §10 + ADR decision 5).** The engine's emission surface is a **typed
`AsyncIterable<IntentProposal>`** — the gate (sink) pulls via `for await (const p of
engine.results())`. **No** `on(event,cb)` / `subscribe({onResult})` / options-bag callback. The
no-function-typed-param guardrail and the public API are now **coherent**; the no-write invariant
holds the same way (the engine emits `readonly` data and holds no mutator). **Owner:** Architect/FE.
**Residual:** none.

### Counsel round-2 strengthenings (folded — cheap, done)
- **C-1 · actor-anonymous telemetry schema guardrail NOW (proposal §8 + §6 + §9 + §10 + R-O).**
  A red→green test asserts the voice counter record `{ intent_kind, matched, confidence_bucket,
  locale }` carries **no** `courier_id`, **no** `user_id`/actor id, **no** latency/timing field —
  locking the STOP-1 surveillance gradient at its cheapest point (before any worker mic exists).
  **FIX.**
- **C-2 · decline affordance visually equal (proposal §8 + STOP-2 exit criterion).** Beyond the
  functional decline test, an assertion that **"Use voice"** and **"Not now / use touch"** are the
  **same affordance weight** (same button class / not primary-CTA-vs-grey-ghost) — a passing
  functional test must not license a lopsided hierarchy. **FIX (exit criterion).**
- **C-3 · WebGPU rate is a required gate-artifact field (proposal §4.2 + §10 + R-K).** The measured
  WebGPU-availability rate across the device/WebView sample is a **required line** in the Phase-0
  gate report — "who this excludes" is a printed number, not a promise. **FIX.**
- **C-4 · corpus consent conditions baked (proposal §8.1 + R-I).** The gate on **real-device
  recording** now names: **non-coercive recruitment — explicitly NOT the platform's own couriers /
  workforce** (so STOP-1's power-asymmetry cannot collapse into the corpus), **fair compensation**,
  an explicit **withdrawal right**, **protocol-scoped consent** (specific recording protocol +
  retention window), and a documented **vulnerable-population safeguard** for the diaspora/migrant
  cohort. Gates real-device recording only — **not** the cheap Phase-0 laptop probe (architect's own
  voice) and **not** the engine build. **FIX.**

### Round-2 disposition tally
- **fix:** R2-A, R2-C, R2-B (+residual accept), R2-D (mechanism; +RLS accept), R2-E, R2-F,
  Counsel C-1/C-2/C-3/C-4.
- **accept-risk:** R2-A extra control-plane round-trip; R2-B safety-implying-name evading the
  denylist (counted in dangerous-misfire); R2-D "outside RLS" (research artifact — RLS is the wrong
  tool; controller + job + tripwire instead).
- **defer-flag (MISSING):** none new this round. (Round-1 deferrals stand: Phase-3/4 admin/courier
  gate thresholds; multi-tenant accuracy validation; accessibility-earn requirements.)
- **NEEDS-HUMAN-DECISION (unchanged, recorded):** STOP-1 Phase-3/4 entry; demand evidence; corpus
  data-controller assignment — now **carrying the C-4 recruitment-ethics conditions**.
- **Honest completeness note:** R2-C/R2-D/R2-A guardrails + the control-plane endpoint + the CSP
  single-flag are **specified as required exit criteria, not built** (this remains a design-time
  proposal, no production code). Nothing later-only is marked done.

---

## SCOPE NARROWING (operator 2026-06-30)

> Not a Breaker/Counsel finding — an **operator scope decision**, recorded here for the trail. This
> is scope bookkeeping: it **narrows** the build and adds one read-only capability. It does **not**
> redesign anything and does **not** re-open any resolved finding (C1/C2, H1–H4, M1–M6, L1–L3,
> R2-A..R2-F, Counsel C-1..C-4 stand exactly as dispositioned above).

**Decision.**
1. **Active scope = client-facing flow only: storefront menu + checkout.** **Phase 3 (admin) and
   Phase 4 (courier) are REMOVED from the active roadmap and deferred to a SEPARATE FUTURE COUNCIL —
   not part of this build at all.** Their recorded constraints (admin/courier own dangerous-misfire
   gate — R-M; courier non-settling-only with `arrived`/`completeDelivery` excluded by construction —
   H4; STOP-1's human decision — R-O) travel forward to that future council; they are not enacted
   here.
2. **Checkout voice = READ_ONLY only — exactly two intents:** "read my order" (reads the user's
   **own client-side cart + total** back to them) and "go to checkout" / navigation. **No field
   writes** (address / phone / notes are not voiceable), **no place-order, no payment, no order
   finalization.** Checkout stays inside the existing money-has-no-voice-grammar invariant and adds
   **zero** stateful/money voice grammar.
3. The menu intent pack is **unchanged** (add [confirm-gated] / filter / sort / macro / compare /
   search, with the dietary-token denylist of R2-B intact).

**No new attack surface (the security read).**
- The two checkout intents are **READ_ONLY, non-safety-read, non-money.** "Read my order" reads only
  the user's own in-tab cart state — **no new PII egress, no new server call, no cross-tenant
  surface**, no new write/RLS/auth path (the §5/§8 invariants hold unchanged).
- Removing admin/courier **reduces** surface: it drops the highest-risk classes (worker-mic
  labour-surveillance gradient, confirm-fatigue on in-grammar STATEFUL admin/courier actions) from
  the active design. The active capability table now holds **add-to-cart** (the only STATEFUL intent)
  plus READ_ONLY menu + checkout-read intents.

**Unchanged by this narrowing (explicitly):** the hardened guardrails, the fail-closed runtime
kill-switch (R2-A), the zero-egress design (R2-C), the single-flag CSP authority (R2-E), and the
back-of-envelope (§2) are all untouched. **Storefront gates still live:** R-J (demand evidence →
gates Phase-1 code) and R-I/C-4 (corpus consent → gates real-device recording). **STOP-1 is now
out of active scope** (deferred — re-opens only with a future admin/courier-voice council).

**Files updated:** `proposal.md` (§1, §6 capability table, §10 phasing), `docs/adr/0015-voice-control.md`
(Context, Decision 5, Phasing, Consequences, Status), `ethical-decisions.md` (STOP-1 row + Status),
this file.
