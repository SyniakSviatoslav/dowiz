# Voice Control — Breaker Findings (adversarial, design-time)

> Target: `docs/design/voice-control/proposal.md` + `docs/adr/0015-voice-control.md`.
> Register: System Breaker. Job = prove it breaks. No fixes. Ranked, most severe first.
> Grounding checks run READ-ONLY against the working tree (CSP, setters, Dockerfile flags).

---

## CRITICAL

### C1 · B-SEC/B-CONSIST · Allergen filter auto-applies with NO confirm → false safety signal, and the safety gate is blind to it by construction
- **Where:** §6 classifies `setFilterAllergen` as `READ_ONLY` → "may auto-apply … no confirm".
  Verified: `setFilterAllergen` is a real storefront setter (`MenuPage.tsx`, the
  `dos_menu_prefs` allergen filter). The §4.2 **dangerous-misfire** metric is *defined* as
  "fraction that resolve to a wrong **STATEFUL** intent" — allergen filter is `READ_ONLY`, so a
  mis-transcribed allergen command is **excluded from the only safety number in the gate**.
- **Break scenario:** an allergic `sq` user says *"shfaq pa arra"* (show without nuts) /
  *"pa gluten"*. Whisper mis-transcribes the negation or the allergen token (negations and short
  function words are the WEAKEST part of small-model ASR). The gate auto-applies the **wrong**
  allergen (or no) filter, the UI now shows a "filtered" state the user reads as *nut-free*. User
  orders a dish containing the allergen → health event. No confirm tap ever gated it.
- **Regression weight:** project MEMORY records STEP-0 fixed **2 LIVE allergen false-negatives**
  (`menu-characteristics-model-2026-06-29`). Auto-applying an allergen filter from a noisy voice
  channel **re-opens exactly that false-negative class** through a new input device.
- **Violated invariant:** "worst case = a reversible wrong filter" (R-D) is false for allergen
  filters — a wrong allergen filter is a *false safety assertion*, not a reversible annoyance. The
  dangerous-misfire ≤2% gate cannot bound a risk it defines out of scope.

### C2 · B-SEC (privacy) · The Phase-0 gate cannot be computed without persisting a labeled audio+transcript PII corpus — it directly contradicts the "zero persistence/egress" invariant
- **Where:** §8 / decision 4 assert "zero audio/transcript egress **or persistence**". §4.2 / §10
  require measuring IRA ≥85% and dangerous-misfire ≤2% on "~100–150 **real-device** `sq`
  utterances (incl. Gheg/AL accents) … recorded in a noisy and a quiet condition."
- **Break scenario:** computing IRA and dangerous-misfire **requires** (a) capturing each real
  utterance's audio, (b) the produced transcript, (c) a human-labeled ground-truth `{action,
  target}`, and (d) retaining all of it to re-run after matcher tuning (AMBER → "re-measure").
  That is, by construction, a **retained, labeled dataset of identifiable Albanian speakers'
  voice recordings** — the exact artifact the invariant forbids. The `VITE_VOICE_TRANSCRIBE_DEBUG`
  overlay (§9) exists precisely to surface transcripts, on a deployed staging `/s/:slug`.
- **Violated invariant:** "Audio is PII … never written to disk, never sent to the server, never
  logged." The proposal's own success criterion is methodologically incompatible with it. No RoPA
  entry, retention policy, consent basis, or storage location for this corpus is named — yet it is
  a prerequisite of the gate. "No new subprocessor / RoPA entry" (§8, ADR Consequences) is asserted
  while the gate manufactures the dataset that would require them.

---

## HIGH

### H1 · B-ANTIPATTERN (verification) · MockProvider proof and the §4.2 gate measure DISJOINT halves — CI green is a false-green for the actual risk
- **Where:** §10 "Deterministic proof": `MockProvider` injects a **scripted transcript**, Playwright
  asserts cart/filter DOM. The unit corpus drives the **IntentMatcher** with text. Neither runs
  real audio through Whisper.
- **Break:** the entire risk surface — *does Whisper actually turn "shto sufllaqe" into the right
  token on a real mid Android in a noisy diner* — is the **transcription** stage, which MockProvider
  **replaces with a constant**. A 100% green MockProvider + corpus suite proves grammar→handler
  wiring and matcher fuzz logic; it says **nothing** about IRA or dangerous-misfire. The one number
  that gates safety (§4.2) is provable ONLY via the non-deterministic manual corpus → it is **not**
  CI-runnable and **not** deterministic, contradicting "proof must be deterministic" (ADR Context).
- **Untested by construction:** the real-audio → matcher integration. A passing pipeline can ship
  with arbitrarily bad real-world transcription and a fully green test board.

### H2 · B-SCALE/B-FAIL · The WASM-threads "fallback" rung does not exist on the storefront — it's WebGPU-or-hidden, and the dominant social-webview entry has neither
- **Where:** §2.2 / §7 present a 3-rung ladder: WebGPU → "WASM SIMD + **threads**" → WASM
  single-thread. The middle rung's RTF≈3–8× assumes WASM **threads**.
- **Break (threads):** WASM threads need `SharedArrayBuffer`, which needs **cross-origin
  isolation** (`COOP: same-origin` + `COEP: require-corp/credentialless`). The storefront loads
  multiple **cross-origin** resources — verified in the live CSP `connect-src`/`script-src`:
  `cdn.jsdelivr.net`, `tiles.openfreemap.org`, `router.project-osrm.org`, `plausible.io`,
  `cdn.tailwindcss.com`, Google Fonts. Turning on COOP+COEP to get threads **breaks** those
  (map tiles, fonts, Tailwind/Plausible) unless every one ships CORP/credentialled-correctly →
  it won't. So the realistic fallback collapses to **WASM single-thread** ("worse still", §2.2)
  → 15–40s+ → below the capability floor → **mic hidden**.
- **Break (WebGPU reach):** restaurant links are opened from social. Instagram/TikTok/Facebook
  **in-app WebViews** (the majority mobile-social entry) frequently lack WebGPU **and** lack
  COOP/COEP isolation. Result for the dominant real traffic: WebGPU absent + threads absent →
  capability floor → **mic hidden**. The "WebGPU is the design target, WASM degrades gracefully"
  story is really "WebGPU or invisible," and invisible for most mobile-social opens.
- **Violated assumption:** §2.2's WASM-threads middle path and §7's "fall back to WASM" row are
  largely fictional in the storefront's own cross-origin context.

### H3 · B-CONSIST/B-SEC (parity) · `en`/`uk` ship "first-class" but are UNGATED — the §4.2 accuracy + dangerous-misfire gate is `sq`-only
- **Where:** §1 "`en`/`uk` are also first-class"; §10 Phase 1 ships all three. §4.2 / ADR decision 4
  define the GO/NO-GO corpus, IRA, and dangerous-misfire threshold **only for `sq`**.
- **Break:** there is **no measured dangerous-misfire bound for `en` or `uk`**. A Ukrainian or
  English allergen/filter/add misfire is never counted anywhere. Compounded with C1, a `uk`
  allergen mis-transcription is **doubly invisible** (READ_ONLY + ungated language). The i18n
  parity gate covers **string presence**, not transcription accuracy — it gives false comfort.
- **Violated invariant:** "sq/en/uk parity"/"first-class" while only one language passes a safety
  gate is parity in label only.

### H4 · B-CONSIST/B-FAIL · Phases 3/4 bind voice to order-status and courier completion (cash settlement) with NO accuracy gate + confirm-fatigue — and "no voice binding to money, ever" is contradicted
- **Where:** §10 Phase 3 `handleUpdateStatus`, Phase 4 courier accept/reject/**arrived** →
  "highest intrinsic value." §6: "Money/checkout/place-order has no voice binding ever." §4.2 gate
  is `sq` **storefront** only — there is **no defined gate** for the admin/courier intent packs.
- **Break (money binding):** per the cash-as-proof spine (`deliver-v2-cash-as-proof`,
  ADR-0009/0013), courier **completeDelivery / arrived** is the trigger that **settles cash** —
  i.e., a money-state transition. Phase 4 binds a **voice** intent to that handler. "Money has no
  voice binding ever" is therefore false: it is voice→cash-settlement behind a confirm tap, not
  no binding.
- **Break (confirm-fatigue):** a busy operator/courier doing status-by-voice taps confirm
  reflexively. Whisper hears *"reject order 5"* as *"accept order 5"* (both in-grammar, both
  STATEFUL, both plausible) → the confirm sheet reads "Accept #5?", the fatigued human taps yes →
  wrong stateful mutation. This is the **dangerous-misfire** class — gated at ≤2% for storefront-sq
  only, **unmeasured** for the admin/courier vocabularies that actually move orders and cash.
- **Violated invariant:** confirm-then-execute does not bound dangerous-misfire under fatigue; the
  ≤2% safety budget is never applied to the phases with the highest blast radius.

---

## MEDIUM

### M1 · B-ANTIPATTERN (arithmetic) · n=150 cannot resolve a 2% threshold — the gate's precision is fictional and cherry-pickable
- **Recompute:** dangerous-misfire ≤2% measured on ~150 utterances = **≤3 events**. The 95% CI for
  a 2% rate at n=150 is ≈ **0.4%–5.7%** (Wilson). You can **pass** the gate with a true rate of 5%
  or **fail** with a true rate of 1% purely from sampling noise. "≤2%" is not measurable to the
  precision claimed. Likewise IRA ±~6pp at n=150 — an 85% cutoff is inside the noise band.
- **Gameability:** the corpus is hand-recorded → quiet room, clear speakers who pronounce menu
  items the way the matcher expects, demo menu only. A small, self-collected, self-labeled corpus
  is a weak oracle for a safety gate.
- **Violated assumption:** §4.2 treats the thresholds as deterministic pass/fail; the sample size
  cannot support that resolution.

### M2 · B-SCALE · The 300–450 MB memory floor undercounts the host page and the WebGPU GPU copy → OOM is more frequent, and the capability floor can't be evaluated where it matters
- **Recompute:** §2.2 peak = 130 MB weights + 2–3× arena = 300–450 MB. This ignores: (a) the
  **already-loaded storefront tab** — the demo menu has **49 products** with photos (MEMORY:
  42 real R2 dish photos); decoded images + React tree are easily 100–200 MB already; (b) the
  **WebGPU path uploads the model to GPU buffers** — another ~130 MB that, on integrated/shared-
  memory Android, counts against the **same** RAM ceiling. Realistic peak on a 3–4 GB device with
  the menu open ≈ **500–700 MB**, well into tab-crash territory the proposal calls "borderline."
- **Floor is unmeasurable:** the capability floor uses a "low-RAM heuristic". `navigator.deviceMemory`
  is **absent on iOS Safari entirely** and **coarse/capped at 8 (often reports 4)** on Android →
  the heuristic cannot be evaluated on the exact low-end/iOS devices it's meant to protect.
- **Violated assumption:** the memory budget that *defines* the capability floor omits the page it
  runs inside and the GPU copy of the happy path.

### M3 · B-SEC (DI boundary) · "Cannot mutate by construction" is enforced by an import-grep, not the type system — a runtime-injected callback bypasses it
- **Where:** §6 enforcement #1 = "eslint-plugin-local + grep: `packages/voice` imports no mutating
  API client / no CartProvider mutator / no status handler."
- **Break:** the invariant is "zero write capability by DI boundary," but the guardrail only checks
  **static imports**. Nothing stops a future refactor from passing a **callback** into
  `packages/voice` — e.g. `engine.onResult(cb)` / a MockProvider hook — where `cb` closes over
  `addItem` or `handleUpdateStatus`. The closure mutates from *inside* `packages/voice` while
  importing nothing → the grep stays green. "By construction cannot mutate" is weaker than claimed;
  it's "by-convention-plus-import-grep."
- **Violated invariant:** the structural guarantee (§4.5, §6) relies on a property the listed
  guardrail does not actually verify (no injected write-capable function reference).

### M4 · B-CONSIST · Default classification for an unlisted/forgotten intent kind is unspecified — a forgotten Phase-3 kind could auto-execute
- **Where:** §6 "classifies via a **static capability table**" — `READ_ONLY` vs `STATEFUL`. The
  proposal never states the **default** for a `kind` that is in the grammar but absent/mismarked in
  the table.
- **Break:** Phase 3 adds an admin intent (e.g. `cancel_order`). A dev adds the grammar + handler
  but forgets the table row, or the classifier is `kind === 'STATEFUL_X' ? confirm : autoApply`.
  An unlisted/mismarked stateful kind then takes the **auto-apply** branch → a stateful order
  mutation executes with no confirm. The guardrail test (§6 #3) only asserts *known* kinds and the
  *absence* of `place_order/pay/checkout` — it does not assert "unknown kind → reject."
- **Violated invariant:** confirm-then-execute holds only if "unclassified ⇒ STATEFUL/reject" is
  fail-closed; the design leaves it fail-open-shaped and unspecified.

### M5 · B-OPS · Kill-switch is a VITE build-arg → no runtime hot-kill; "instant rollback" is false
- **Where:** §9 "Kill-switch / rollback: rebuild with the flag `false` (**instant**)"; runtime
  hot-kill is only "**consider**." Verified: voice would follow the existing `VITE_*` build-arg
  pattern (`Dockerfile`), baked at image build.
- **Break:** if shipped voice causes field OOM/tab-crashes (M2) on real low-end devices, the only
  mitigation as designed is a **Docker rebuild + Fly deploy** (minutes to tens of minutes,
  remote-only), during which affected users keep crashing. "Instant" is false for a baked flag.
  There is no server-served runtime switch to disable the mic without a redeploy.
- **Violated assumption:** "<1 min visibility + controlled rollback" — rollback is a full build
  cycle, not a flip.

### M6 · B-SEC/gate-validity · The matcher is tuned and measured on the demo tenant's menu, then shipped to all tenants
- **Where:** §4.2 corpus is "against the **demo menu** vocabulary"; AMBER tuning = "decoding bias
  toward **the** tenant vocabulary." The shipped matcher/decoding config is a static asset.
- **Break:** IRA ≥85% on the demo menu does **not** transfer to a tenant selling different items
  (different item names, different `sq` spellings, different ambiguity ties). A decoding bias toward
  demo vocabulary actively *mis-fits* other tenants. The gate validates one tenant and launches all.
  (No cross-tenant data *leak* — the per-tab vocab is the tenant's own — but the **gate result is
  not valid** for non-demo tenants.)
- **Violated assumption:** "tenant isolation by construction" addresses leakage but not that the
  one measured tenant's gate is generalized to all.

---

## LOW

### L1 · B-SEC (CSP) · The locked storefront CSP does NOT currently allow the model origin — "reuses dowiz-images / stays narrow" understates the change
- **Verified:** live CSP (`apps/api/src/lib/spa-shell.ts:150`) has R2 only in **`img-src`**
  (`r2ImgSrc`). `connect-src` is `'self' https://cdn.jsdelivr.net https://tiles.openfreemap.org
  https://router.project-osrm.org https://en.wikipedia.org https://plausible.io` — **no R2**.
- **Break:** a `fetch()` of the 130 MB model from R2 is governed by **`connect-src`** and is
  currently **blocked**; the ONNX-runtime worker needs `worker-src` (today `'self' blob:`) and
  WASM needs `script-src 'wasm-unsafe-eval'`/`'unsafe-eval'` (present). So launching voice requires
  widening a deliberately locked CSP, not merely "reusing the dowiz-images pattern" (that pattern
  is image-display only). R-F flags "verify CSP" but the proposal frames it as a check, not a
  required widening of a hardened policy.

### L2 · B-OPS/B-ANTIPATTERN · "Zero bundle delta — verified, not asserted" has no verification listed
- **Where:** §9 "Zero bundle/critical-path delta … this is verified, not asserted." §10's proof
  list = MockProvider Playwright + unit corpus + capability test. **None** is a bundle-size /
  critical-path-import regression assertion.
- **Break:** a single eager `import` of `packages/voice` (e.g. a type import a bundler can't
  tree-shake, or a non-dynamic re-export) pulls worker/WASM glue into the main storefront chunk →
  the dark feature taxes the critical path. The claim "true dark" depends on the dynamic-import-only
  property, for which **no listed proof exists**. The dark claim is itself unproven.

### L3 · B-SCALE (arithmetic) · §2.1 q8 byte count is internally inconsistent and feeds the memory floor
- **Recompute:** §2.1 states whisper-base raw int8 ≈ **74 MB**, then claims the q8 transferred
  bundle = encoder 42 MB + decoder_merged 85 MB ≈ **130 MB** — i.e. ~**1.75×** the raw int8 size,
  and a per-component decoder (85 MB for a ~55M-param decoder) larger than that decoder's **fp16**
  would be. The "q8" label and the byte count don't reconcile; the artifact is mixed-precision, not
  q8. The proposal hedges this as "a measurement output," but the **unhedged** §2.2 memory floor
  (300–450 MB) is built on the 130 MB figure → if the real artifact is heavier, M2's OOM case
  worsens. The arithmetic should not be presented as q8 when the bytes say otherwise.

---

## Regression note vs the load-bearing claims
- "Confirm-then-execute reaches no money/stateful mutation without a human confirm" — **breached**
  for the READ_ONLY allergen path (C1, a safety mutation of displayed safety info), **unspecified**
  for unlisted kinds (M4), and **contradicted** for courier cash-settlement (H4).
- "Zero audio/transcript egress or persistence" — **contradicted by the gate's own methodology**
  (C2) and the debug overlay on a deployed route.
- "Deterministic proof" — the deterministic suite proves the matcher, not transcription (H1); the
  safety gate is non-deterministic and statistically under-powered (M1).
- "WebGPU-first, WASM degrades gracefully, else hide" — the WASM-threads rung is unavailable in the
  storefront's cross-origin context; effectively WebGPU-or-hidden, hidden for most social-webview
  traffic (H2).
- "Server-scaling impact nil / 0 connections" — holds for Phase 0/1 transcription; not attacked.

---

## ROUND 2 (re-attack — regression + fix-verification)

> Target: revised `proposal.md` + `resolution.md` + ADR-0015. Grounding re-run READ-ONLY against the working tree (`spa-shell.ts`, `MenuPage.tsx`, `apps/api/public/sw.js`). Round 1 stands above; this is additive.

### Round-1 dispositions — verification verdict
- **C1 — CLOSED (core).** `setFilterAllergen` removed from voice-reachable setters; no allergen/dietary `kind`; dangerous-misfire redefined to count a "state read as a dietary/safety assertion"; table-test guardrail specified. **But exclusion is setter-specific, not class-specific** — see R2-B.
- **C2 — CLOSED (methodology), residual on enforcement.** Consented research regime (§8.1) separated from production runtime; logic sound. **Production zero-egress on deployed routes is now the weak link** — see R2-C.
- **H1, H3, H4 — CLOSED.** CI-proves-only-wiring honest; per-locale gate real; cash-settling `arrived`/`completeDelivery` excluded by construction; "ever" narrowed to Phase 0/1/2.
- **H2, M1, M2, L3 — CLOSED.** WebGPU-or-hidden 2-state; Wilson upper-CI ≤2% at n≥300; 500–700 MB peak + warmup probe; mixed-precision relabel.
- **M4 — CLOSED.** `default → REJECT` fail-closed at runtime regardless of `never` build-check bypass.
- **M5 — NOT CLOSED.** Runtime hot-kill defeated by cache-first SW — see R2-A (HIGH).
- **M3 — mostly closed, guardrail inconsistency** — see R2-F (LOW).
- **L1 — closed in intent, new coupling gap** — see R2-E (MEDIUM).

### [HIGH] R2-A · B-OPS/B-FAIL · M5 runtime hot-kill rides a cache-first service worker → not instant, fail-open for returning visitors
The kill signal sits on the bootstrap payload (`/public/locations/${slug}/menu` MenuPage.tsx:378, `/info` :437). `apps/api/public/sw.js` intercepts every GET not under `/api/`|`/ws/` and serves **cache-first** (`caches.match(req).then(a => a || fetch(req))`), ignoring HTTP cache semantics. Returning visitors who loaded the menu once get the **stale pinned payload** (kill=not-set) and never hit network → MicFab keeps activating and crashing. Cache clears only on `UPDATE_CACHE_VERSION` (a redeploy) — the exact thing M5 claimed to avoid. Fail-open variant: kill-as-new-field reads `undefined` → "not set" on pre-field cached payloads. Violated: "runtime-instant disable, no rebuild."

### [HIGH] R2-C · B-SEC (privacy) · Production zero-egress is policy-enforced, not by construction; debug overlay has no guardrail; staging /s/:slug is publicly reachable
§8.1 forbids `VITE_VOICE_TRANSCRIBE_DEBUG` on public deployed `/s/:slug` by **prose**. Unlike C1/M3/M4 it gets **no guardrail** in §6/§10: nothing asserts the deployed build has the flag false or strips the overlay path. Research recording "on real devices" naturally deploys a debug build to staging `/s/:slug` — which is publicly reachable and also serves the demo tenant, E2E traffic, and live claimable demos (non-consented traffic exposed to spoken transcripts). The strongest privacy claim is the only one without a gate.

### [MEDIUM] R2-B · C1 excluded the allergen *setter*, not the *class* — a dietary-named category still auto-applies via setSelectedCategory with no confirm
`setSelectedCategory` (MenuPage.tsx:243/474) stays in READ_ONLY auto-apply. Categories are tenant-defined free text. A tenant category "Pa gluten"/"Vegan" + user "shfaq pa gluten" → fuzzy match → auto-applies, narrows menu, user reads as allergen-safe. Same false-safety-read C1 was CRITICAL for, via a still-open setter; auto-apply with no checkpoint is worse than the confirm path C1 rejected. Bounded ≤2% only if corpus includes such near-misses.

### [MEDIUM] R2-D · §8.1 corpus is identifiable-voice PII outside RLS and outside automated enforcement
≈900+ identifiable voice recordings + transcripts in an encrypted off-tenant store with "no RLS surface"; 90-day deletion + access control are manual/policy. compliance-gate documents existence, does not enforce deletion or detect breach. Controller unassigned (NEEDS-HUMAN). Lapsed copies (AMBER tuning past 90 days) have no automated tripwire — unlike every other PII surface (ENABLE+FORCE RLS + gate).

### [MEDIUM] R2-E · CSP connect-src widening gates on a "server-visible voice flag" that doesn't exist; build/server desync
§8/L1 gates the connect-src R2 addition on a server-visible flag, but the launch flag is `VITE_VOICE_CONTROL_ENABLED` (client build-arg, invisible to spa-shell.ts:150). Needs a NEW synced server env var. Desync: (1) client ON / server unset → model fetch blocked → voice silently dies; (2) server set / client dark → hardened CSP widened to R2 while dark. CSP tied to build/server flag not the runtime hot-kill → after R2-A kill, connect-src stays open.

### [LOW] R2-F · M3 "no function-typed parameter" guardrail contradicts the same section's "event stream" API
An event-stream consumed via `on(event, cb)` is a function-typed param → the guardrail forbids the offered API; an options-bag callback (`subscribe({onResult})`) slips a top-level-only rule. No-write invariant holds regardless (engine emits readonly data, holds no mutator). Fix: pick the async-iterator form; the rule + invariant become coherent.

### Round-2 regression summary
- M5 fix introduced R2-A (HIGH): hot-kill rides SW-pinned payload → not instant, fail-open for crash population.
- C2 fix residual R2-C (HIGH): production zero-egress is the only major safety property without a guardrail.
- C1 fix is setter-scoped not class-scoped (R2-B).
- New surfaces: corpus store (R2-D), two-flag CSP coupling (R2-E) — MEDIUM, enforcement-by-policy gaps.
- No new CRITICAL. Round-1 C1/C2/H1–H4/M1/M2/M4/M6/L3 verified closed.

---

## ROUND 3 (FINAL re-attack — tight verification + regression on new surfaces)

> Target: twice-revised `proposal.md` + `resolution.md` "## ROUND 2 RESOLUTION" + ADR-0015.
> Grounding re-run READ-ONLY: `apps/api/public/sw.js` (SW exempts `/api/`,`/ws/`,`sw.js`,`.webmanifest`;
> all else GET → cache-first), `apps/api/src/lib/spa-shell.ts:144-152` (CSP built server-side per
> request, `connect-src` has NO R2, `r2ImgSrc` is img-src-only, header set before shadow/tenant
> branches). Round 1+2 stand above; this is the disposition-verification pass only.

### Round-2 fix verification

- **R2-A — CLOSED.** SW grounding confirms `e.pathname.startsWith("/api/")` short-circuits the
  cache-first branch, so `GET /api/public/voice-config` is never SW-cached → instant for returning
  visitors. Fail-closed is total: design rejects on {fetch-reject, `!res.ok`, non-JSON, `enabled`
  absent/`!== true`}; a hung fetch (no resolve) also yields OFF (mic never activates). Browser HTTP
  cache cannot serve a stale `enabled:true` because the request is `cache:'no-store'`. Build-arg ON +
  endpoint unreachable ⇒ fetch-reject ⇒ OFF. No fail-open path survives.
  - *Residual (LOW, named): response-side Cache-Control unspecified.* The design mandates the
    **request** mode `no-store` but does not state the **server response** carries
    `Cache-Control: no-store`. A shared intermediary (Fly edge / proxy) could cache a 200
    `{enabled:true}` and delay an emergency `VOICE_KILL` by its TTL. Trivially closeable at build
    time; does not reopen R2-A. Not a blocker.

- **R2-C — CLOSED (as design).** Three machine-checks (bundle-absence grep, public-deploy-arg
  assertion, separate non-public research host) make "serving real customers" a predicate on the
  deploy matrix, not prose. A debug build cannot reach a public `/s/:slug` without failing the
  deploy-arg or env-matrix check. Honestly labelled specified-not-built (design-time). No surviving
  path to a transcript-surfacing overlay on a public lane.

- **R2-B — CLOSED (class), residual accepted.** Denylist is a single-source sq/en/uk stem list
  matched against the **category name** (not the utterance), evaluated across all three locales
  regardless of active locale; `setSelectedCategory` is dropped on a match. Other READ_ONLY auto-apply
  paths checked for a safety-read: `setSearchQuery` (C1-accepted, reversible, counted),
  `setMacroLens` (macro nutrition, not allergen-safety), `setSortBy`/`toggleCompare` (no safety read)
  → no remaining auto-apply path lands on a dietary-safety state outside the counted set.
  - *Residual (accept-risk, named, bounded by phasing): a safety-implying category name evading the
    stems* (e.g. `"Free From"` — a real allergen-aisle term matching no stem) auto-applies as a
    safety read. Bounded because Phase 1 ships the demo/pilot tenant ONLY (its categories are
    hand-checkable against the denylist) and broad multi-tenant launch is gated on the R-N
    re-measurement; the redefined dangerous-misfire metric counts it. Confirmed, not a new HIGH.

- **R2-E — CLOSED, claim-wording nuance.** spa-shell builds the CSP per request server-side
  (grounded), so a single server authority `VOICE_CONTROL_ENABLED && !VOICE_KILL` driving both the
  config endpoint and the `r2ConnectSrc` widening is viable, and a hot-kill returns `enabled:false`
  AND narrows connect-src on the next *server-rendered* document. The desync cases (client-on/server-off,
  widen-while-dark) are removed because the client gates activation on the fresh, fail-closed config
  endpoint — not on the CSP.
  - *Nuance (LOW, inert): the "closes connect-src on next document load" claim is not literally true
    for the SW-cached returning population.* The `/s/:slug` document is a non-`/api/` GET → the SW
    caches the Response **including its CSP header** cache-first, so a returning visitor keeps the
    OLD (R2-widened) CSP until `UPDATE_CACHE_VERSION`, despite the server's `no-store`. This is
    **security-inert**: the client still calls the SW-exempt config endpoint, gets `enabled:false`,
    and never issues the model fetch — an open connect-src with no fetch is zero egress. The CSP is
    defense-in-depth, the config endpoint is the functional gate. Wording is mildly overstated;
    no hole. Not a residual HIGH.

- **R2-D — CLOSED as fix-specified.** Scheduled deletion job keyed to per-item expiry manifest +
  expiry-audit tripwire (fails `compliance-gate`) + deletion-proof log; RLS honestly accepted as the
  wrong tool for a non-tenant artifact. Operates off-tenant, no production attack surface. Correctly
  labelled gate-on-recording, not built; controller is NEEDS-HUMAN. Honest.

- **R2-F — CLOSED.** Emission surface is a typed `AsyncIterable<IntentProposal>` pulled via
  `for await`; no `on(event,cb)`/`subscribe({onResult})`. The no-function-typed-param guardrail and
  the public API are coherent; no injectable write-capable closure surface remains.

- **Counsel C-1/C-2/C-3/C-4 — CLOSED.** Actor-anonymous telemetry-schema test (no
  `courier_id`/`user_id`/latency), decline visually-equal-affordance assertion, WebGPU-rate as a
  required gate-artifact field, and the corpus consent conditions (non-coercive, explicitly
  not-our-workforce, fair pay, withdrawal, protocol-scoped, vulnerable-pop safeguard) are all baked
  as guardrails/exit-criteria. Honestly accept/defer where human-gated.

### Regression on the NEW surfaces (focused — CRITICAL/HIGH only)

- **`/api/public/voice-config` endpoint.** Public unauthenticated read-only GET returning a **global**
  `{enabled}` boolean (the predicate is env-global, slug is near-vestigial). No tenant data → no
  cross-tenant leak; no PII; no write; no new RLS surface; design specifies an **env read, no DB
  query** → zero pool pressure / no N+1. Only the LOW response-Cache-Control gap (above). **No new
  CRITICAL/HIGH.**
- **Per-request CSP predicate in `spa-shell.ts`.** Adds a `process.env` read to an already-per-request
  CSP build (no DB). Header is emitted for shadow + tenant + fallback paths alike; widening
  `connect-src` to first-party R2 (already in `img-src`) is inert for shadow tenants and for
  dark/per-locale-dark tenants (client-gated). Negligible static-surface delta. **No new CRITICAL/HIGH.**
- **Scheduled corpus-deletion job.** Operates on the off-tenant research store only; no production DB,
  no RLS surface, no new runtime attack surface. Existence is fix-specified-not-built and honestly
  flagged. **No new CRITICAL/HIGH.**

### Round-3 verdict
- R2-A, R2-C, R2-B, R2-D, R2-E, R2-F, Counsel C-1..C-4 — all **CLOSED** (R2-A/R2-B/R2-E carry named,
  bounded, security-inert LOW residuals; nothing reopens).
- New surfaces (config endpoint, per-request CSP predicate, deletion cron) opened **no** new
  CRITICAL/HIGH.
- Honest design-time caveat stands: the R2-A/R2-C/R2-D guardrails + the `/api/public/voice-config`
  endpoint + the single-flag CSP are **specified as required exit criteria, not built** — these must
  go red→green before the first deployed phase; that is correct for a no-production-code proposal and
  is not a breaker finding.

RESIDUAL CRITICAL/HIGH: none
