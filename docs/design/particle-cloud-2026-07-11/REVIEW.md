# Adversarial Review — Particle-Cloud Interaction Analysis (2026-07-11)

> **Reviewer lane:** independent of the author lane. **Subject:**
> `docs/research/2026-07-11-particle-cloud-interaction-analysis.md`.
> **Method:** re-ran every measurement myself (repo esbuild 0.28.0 `--bundle --minify` + `gzip -9`
> on the author's prototypes in `/root/.claude/jobs/c6a4c73f/tmp/measure/`), read every prototype
> line-by-line, re-verified every cited repo file at the cited line, live-re-HEAD'd the MediaPipe
> model CDN, and web-verified the external claims (Chrome speech explainer, DTW/MFCC noise
> literature). Verdicts: **CONFIRMED / WEAKENED / REFUTED** per claim.
>
> **Roll-up: 10 CONFIRMED · 7 WEAKENED · 2 REFUTED · 9 material omissions.**
> The size engineering is honest and reproduces to within 8 bytes. The kills are elsewhere:
> the voice tier's noise-accuracy story, the camera-primary choice for customer phones, the
> "lazy JS doesn't count" budget assumption, an ADR-0015 scope conflict, a nonexistent kiosk
> surface, and an unmentioned repo security posture that denies camera+mic.

---

## 1. Measurements re-verified (this session)

| Module | Claimed gz | Re-measured gz | Delta |
|---|---|---|---|
| `particles-tf.js` | 3,586 B | **3,594 B** | +8 B (esbuild version drift) |
| `voice-dsp.js` | 2,260 B | **2,268 B** | +8 B |
| `motion-flow.js` | 777 B | **785 B** | +8 B |
| MediaPipe `vision_bundle.mjs` | 137.0 kB / 39.7 gz | **136,993 B / 39,652 B** | exact |
| `vision_wasm_internal.wasm` | 11.15 MB / 3.28 MB gz | **11,153,617 B / 3,277,471 B** | exact |
| `vision_wasm_internal.js` | 322 kB / 77.5 gz | **322,044 B / 77,486 B** | exact |
| `hand_landmarker.task` | 7,819,105 B | **7,819,105 B** (live HEAD re-run) | exact |

**Sizes: CONFIRMED.** The numbers are real, reproducible, and were not massaged.
(Nit: exec summary says "11.1 MB one-time", §3.1 says "≈11.2 MB" — the over-wire sum is 11.22 MB.)

### 1.1 Prototype honesty audit — is each module a real proxy or a hollow stub?

**`particles-tf.js` (3.6 kB) — mostly honest, one overclaim.** Read line-by-line: it genuinely
contains the TF ping-pong sim (pos+vel+seed, `SEPARATE_ATTRIBS`), hash-noise flow field, morph-target
RG32F texture + real glyph sampling from canvas text (`setTargetsFromText`), 3-stop palette +
desaturation uniform, `pulse`/`setMode` event API, DPR resize via ResizeObserver, IntersectionObserver
pause, reduced-motion freeze, and context-loss *cleanup*. Not a stub. **But** §2 of the analysis claims
press-hold attract, **flick impulse, second-pointer swirl, and pinch** as touch forces and says "Cost:
included in the 3.6 kB measurement" — the measured module contains **only single-pointer
attract/repel**. Flick/swirl/pinch (§2 itself estimates ~30 lines for pinch alone), context
*restore* (only cleanup is handled), the adaptive-count guard promised in §1.4, `touch-action`/
`getCoalescedEvents` wiring, the canvas2D fallback, and both mount adapters are all outside the
measured bytes. **Realistic production core: ~5–7 kB gz** — which the analysis's own P1 (~6 kB) and
P2 (+6.5 kB) route figures largely absorb, so the budget conclusion survives; the word "complete"
does not. → **WEAKENED** (size honest, completeness overstated).

**`voice-dsp.js` (2.3 kB) — all claimed components present, but the pipeline is broken as written.**
It really contains radix-2 FFT, 20-band mel filterbank + DCT-II → 13 MFCCs, DTW with Sakoe–Chiba band
+ absolute-threshold + 15%-margin rejection, localStorage template enroll (2–3 per command), YIN,
whistle glide classifier, clap detector, energy endpointing, and worklet+ScriptProcessor plumbing.
Not hollow. **However — a real functional bug:** frame extraction is
`for (off = 0; off + FRAME <= chunk.length; off += HOP)` with `FRAME = 512`, but AudioWorklet
delivers **128-sample** chunks → the loop body never executes in the worklet path → **zero MFCC
frames are ever produced**; the 3 s ring buffer is written but never read. (The ScriptProcessor
fallback at 1024 samples "works" but drops cross-chunk continuity.) The fix (frame from the ring
buffer) is roughly size-neutral, so 2.3 kB stands as a size proxy — and the analysis's §8 caveat
("compile-verified, behavior designed-not-proven") is accurate — but "complete prototype" oversells
what was proven. A robust production version also wants pre-emphasis, cepstral-mean normalization,
and tuned endpointing: **+0.5–1 kB gz**, total ~3 kB. → **WEAKENED** (small size delta, real bug found).

**`motion-flow.js` (0.8 kB) — honest and complete** for what it claims: 64×48 grayscale
downscale, frame differencing, one global Lucas–Kanade normal-equation solve, energy/centroid.
The compute-cost arithmetic (~0.7 M ops/s at 15 fps) checks out. → **CONFIRMED.**

---

## 2. The riskiest claim: MFCC+DTW accuracy (attacked)

The analysis is candid that accuracy is UNVERIFIED and its safety design (reject-quietly,
confirm+tap for mutations) is correct. But its framing — "high-90s in quiet conditions … treat as
design target" — materially understates the noise problem, and the interaction design (continuous
listening after one tap-to-start) doesn't survive the analysis's own target environments.

What the literature actually supports:

- **Quiet, speaker-dependent, small vocabulary: yes.** Classical and modern replications of
  MFCC+DTW isolated-word recognition report high-90s in clean conditions (e.g. 98.4% with
  MFCC+DTW+KNN; the 1978 Sakoe–Chiba regime). The claim's clean-case anchor is fine.
- **Noise mismatch is brutal for MFCC.** Even *denoised* MFCC front-ends on the Aurora-2 noisy-digits
  benchmark fall to ≈93% at 5 dB SNR, **≈75% at 0 dB, ≈37% at −5 dB** — and that is with far
  stronger back-ends than bare DTW. The existence of PNCC-class features (≈8.7% better than MFCC in
  noise) is the field's admission that MFCC degrades badly under enrollment≠use noise mismatch. A
  working kitchen runs ~70–85 dBA; a phone at arm's length there sees roughly 0–10 dB SNR.
  Extrapolation: **50–80% accept accuracy with elevated false-accepts — not high-90s.**
- **Endpointing dies first.** Adaptive-noise-floor energy gating in *non-stationary* clatter
  (pans, chopping, shouting) triggers continuously; every speech segment from any speaker in range
  is then DTW-scored. DTW template matching has no garbage/filler model — out-of-vocabulary
  rejection rests entirely on the absolute threshold + margin, which is the classical weak point.
  In a loud kitchen, an always-listening mic will produce a stream of near-threshold candidate
  matches. The false-positive budget is only survivable because mutations need a tap — but the
  *visual* false-positive rate (spurious confirm-posture glyphs blooming during dinner rush) would
  itself discredit the feature.
- **Bike/wind: the analysis already concedes this honestly** (§7.2 — spoken commands at 25 km/h
  impractical). Confirmed as conceded.
- **Claps in kitchens:** "excellent in noise" is half-true. The *detector* survives (onset slope is
  robust); the *false-trigger rate* does not — dropped pans and chopping are exactly broadband
  transients with >6× RMS rise. Harmless only because claps map to read-only intents.

**Verdict: WEAKENED — and the honest design consequence must be stated plainly:** enrolled-word
voice is a **push-to-talk feature** (tap-to-arm a listening window of a few seconds) **plus
confirm-gate**, viable in quiet-to-moderate environments (customer at home; owner at the pass
during lulls); it is **not** an always-on kitchen interface. Ambient level-reactivity (§7.3, no
recognition) is unaffected and remains the best value/kB in the voice tier. The plan (PLAN.md P3)
adopts push-to-talk as a hard design change and adds a deterministic fixture-corpus accuracy gate
(quiet ≥90% accept, distractor false-accept ≤2%, with a RED case) before any UI ships.

---

## 3. Phone touchless reality: permission choreography (position taken)

The analysis makes 64×48 frame-diff optical flow the **primary** phone touchless tier and tilt the
fallback, with a "soft ask… after showing the cloud" and an admitted absence of denial-rate data.
Three problems:

1. **The conversion-risk argument that killed HandLandmarker applies to the permission, not the
   megabytes.** A camera-permission prompt on a food ordering/tracking page — plus the browser's
   persistent "camera in use" indicator while the customer watches their order — is a trust ask the
   customer never expected on this surface. The 0.8 kB download was never the issue.
2. **The repo's own declared security posture denies camera.**
   `apps/api/src/lib/security/headers.ts:41` sets
   `Permissions-Policy: camera=(), geolocation=(self), microphone=(), payment=(self)`. Today that
   header only lands on API/JSON responses (the SSR HTML path passes `isSsr: true`, which skips it —
   verified in `securityHeadersPlugin`), so `getUserMedia` works on the storefront **by accident of
   plumbing, against the declared intent**. Any security pass that extends the header to document
   responses (G10 is exactly such a pass) silently kills the camera tier. The analysis never
   mentions Permissions-Policy at all.
3. **Tilt already satisfies the operator mandate.** "Phone touchless REQUIRED" is met by
   DeviceOrientation: **zero permission on Android Chrome, one in-context tap on iOS 13+**, no
   privacy indicator, no battery cost beyond the sensor, works in pockets of the population that
   would never grant camera. The analysis verified this itself (§3.2.2) and still ranked it second.

**Position (adopted in PLAN.md): INVERT the ranking.** Tilt/shake = **primary** customer touchless
(the default, no prompt); frame-diff camera flow = **opt-in "wave mode"** behind an explicit
user tap with visible value framing, never auto-prompted, and gated on an operator-approved
permission UX copy + an explicit decision about the Permissions-Policy stance. The 0.8 kB module
itself is fine and stays. → **WEAKENED** (technique confirmed, choreography and default flipped).

---

## 4. Claim spot-checks against ground truth

| # | Claim | Ground truth found | Verdict |
|---|---|---|---|
| 4.1 | ADR-0015 "safety spine … council-approved" | `docs/adr/0015-voice-control.md` exists; `packages/voice` (capability-table, ConfirmationGate, matcher with `Locale = 'sq'\|'en'\|'uk'`) is real and transcript-agnostic — reuse is plausible. **But** the ADR's status is **PROPOSED, "pending Breaker + Counsel FINAL re-attack"** — not council-approved — and the operator scope-narrowing of 2026-06-30 **REMOVED admin and courier voice from the active build**, deferring them "to a separate future council". The analysis's ⚠ vocabulary (courier «pranoj»/accept, owner «konfirmo»/confirm, reject, pause) sits squarely in that removed scope. Changing the recognizer from Whisper to DTW does not re-open the scope. Also: ADR-0015's engine surface is a **no-callback `AsyncIterable<IntentProposal>`** (M3/R2-F); the prototype's `onCommand` callback API would need reshaping to slot into the gate. | **WEAKENED** — spine exists and is reusable; "council-approved" is wrong and the owner/courier command set requires a council the analysis didn't flag |
| 4.2 | 21-event inventory from `event-registry.ts:16-167` | Counted: **exactly 21 keys**, names match the analysis list one-for-one; category doctrine (`getEventCategory`, reversibility-of-consequence) at lines ~175–196 as cited | **CONFIRMED** |
| 4.3 | "~46 BUS_CHANNELS" | `apps/api/src/lib/registry.ts`: **44** channel entries counted | CONFIRMED (minor imprecision, not load-bearing) |
| 4.4 | 10 order statuses at `orders.ts:4` | Exactly 10, verbatim, at the cited line | **CONFIRMED** |
| 4.5 | GRAND-PLAN 1.4 envelope: `(order_id, seq, at, cause_hash, payload, content_hash)` + 1.3 `read_since(scope, seq)` replay | `docs/design/sovereign-core-mvp/GRAND-PLAN.md` §1.2 (lines 203–205: the exact column list, `signature` NULLable), §1.3 (line 224: `read_since(scope, seq)`), 0b-2 `Envelope { seq, at, cause }` — all exact | **CONFIRMED** |
| 4.6 | Chrome on-device speech: 17 langs, no sq/uk | Explainer fetched live: exactly 17 locales listed (de, en-US, es, fr, hi, id, it, ja, ko, pl, pt-BR, ru, th, tr, vi, zh-CN, zh-TW); **no sq, no uk** | **CONFIRMED** |
| 4.7 | MediaPipe 11.2 MB / component sizes / model HEAD | All six numbers reproduce exactly (see §1); `hand_landmarker.task` re-HEAD'd live = 7,819,105 B | **CONFIRMED** |
| 4.8 | WS frame `{room,data}` at `websocket.ts:218`; `?token=` deprecated ~:179; per-member re-authz | All three verified at the cited lines (deprecation logger + ADR-0013 guard comment on the fan-out) | **CONFIRMED** |
| 4.9 | Rebuild `pg_fanout.rs`: active self-NOTIFY heartbeat, `Health::Degraded`, `ControlFrame::Resync{entity,id}` | All present (REV-S6-1, REV-S6-6, `HEARTBEAT_CHANNEL`, line 132 `Resync { entity, id }`) | **CONFIRMED** |
| 4.10 | Hub review §4.5 privacy-maximalist GPS | §4.5 "Location/tracking — VERIFIED, privacy-maximalist" — GPS only during active delivery, purge cron | **CONFIRMED** |
| 4.11 | "Ukrainian as a **NET-NEW** catalog language … flag the small addition" | `packages/ui/src/lib/i18n.ts:3,72`: `SUPPORTED_LOCALES = ['sq','en','uk']` — **uk is already a first-class catalog locale today** (the analysis even cites `matcher.ts` modelling sq/en/uk two sentences earlier, and G05 §2.2 counts a "full 3-locale i18n catalog" in the React bundle). No net-new language, no catalog addition — only ordinary new keys in three existing locales | **REFUTED** (in the plan's favor — cheaper than claimed, but factually wrong) |
| 4.12 | P4 kiosk tier: "Venue kiosk / tablet … the flagship look", 60–100k particles, kiosk boots into wave mode | **No kiosk surface exists anywhere in the repo.** `kiosk` appears only as a channel-attribution enum value (`apps/web/src/lib/channel.ts:27`) and a landing-page marketing line. No route, no app, no build, no hardware. P4 is a phase against a surface that would first have to be commissioned, specced, and built | **REFUTED as a phase premise** (the CV research itself is sound; the phase is conditional on a product decision the analysis presents as settled) |
| 4.13 | Coalescing (1 burst/1.5 s, ×N collapse) + tab-hidden/reconnect handling | Design is present and correct: sustained states derived from store state, not replayed events; `visibilitychange`/reconnect → REST snapshot reconcile; transients deliberately dropped; rebuild path gets `read_since` catch-up. Bonus not claimed: the 1/1.5 s bucket also keeps flash frequency under the WCAG 2.3.1 three-flashes threshold. Residual gaps are listed in §6 (multi-island, context restore) | **CONFIRMED** (design level) |

---

## 5. Budget math + phase estimates

**G05 numbers: verified.** Ratified 25/35/60 kB gz route classes, measured 21.6 kB base, 14.3 kB
Svelte floor, React oracle ~234 kB — all match G05 §2.1/§2.2/§3.1. The arithmetic "21.6 + 6.5 =
28.1 < 35" is correct *as arithmetic*.

**But the comparison is wrong twice → "budgets hold" is WEAKENED:**

1. **"Lazy doesn't count" is an assumption, not a ratified rule.** G05's gate command measures the
   route's **full transfer set** (`<script type="module" src>` entries + `modulepreload` hrefs), and
   the authoritative Lane-B budget is "**total hydrated JS** on `/s/[slug]`". A `client:idle` island
   is downloaded seconds after load — it is part of total hydrated JS. There is precedent for an
   explicit exclusion ("excl. lazily-loaded maplibre"), but that's precisely what it is: an
   explicit, signed exclusion. Whether decoration chunks count is an **FE-0.1 signature item**, not
   something the analysis may assume. (Conversely, a runtime `import()` chunk is invisible to the
   gate script as written — evading the gate that way would be gaming the metric.)
2. **28.1 < 35 double-spends the checkout headroom.** The 35 kB class exists to absorb read-parity
   growth (+ up to 3.4 kB) *and* the CartCheckout island (~10 kB implied headroom). Fully spent:
   21.6 + 3.4 + 10 + 6.5 ≈ **41.5 kB > 35** if the particle stack counts. The honest statement:
   the stack fits **only if** the FE-0 budget signature explicitly excludes deferred decoration
   chunks (maplibre-style) or budgets them separately. PLAN.md makes that a named dependency.

Also of note: the 25/35/60 budgets bind the **Astro rebuild storefront**, which per
MASTER-EXECUTION-PLAN Wave 4 is funded only to FE-0, with FE-1+ **arbiter-contingent** and the
rebuild cutover defaulting to **mothball (Path B)**. The surface P2's budget story is written for
may not exist in production for months. (See PLAN.md §2 for the placement consequence.)

**Phase estimates (7–9 sessions): WEAKENED — optimistic by ~30–40%.** Calibration against the
repo's own rates: G05 prices FE-1 — *porting existing, fully-specified components* — at 4–6
sessions. The analysis prices P1 — a novel GPU particle system productionized + runes/React store +
vocab + coalescer + reduced-motion + Playwright VbM + staging ship discipline (commit → staging
deploy → falsifiable proof per the repo's rules) — at 1.5–2 sessions. Each phase here also carries
the task-exit-rule overhead (exit list, RED case, ledger). Realistic: **P1 2–3 · P2 2–3 · P3 3–4
(plus a human mic session) · P4 2–3 ⇒ 9–13 sessions** for the full program.

---

## 6. What the analysis missed entirely

1. **The repo's declared camera/mic security posture.** `headers.ts:41` denies `camera=()` and
   `microphone=()`; the storefront works today only because the SSR HTML path skips that header.
   P2-camera/P3/P4 all require a deliberate Permissions-Policy decision (and P4's WASM requires
   adding `'wasm-unsafe-eval'` to a CSP that currently ships `script-src 'self'` — MediaPipe's
   WASM will not instantiate under the current policy). These are security-review-lane changes,
   nowhere flagged. **Biggest single omission.**
2. **Battery cost of the continuous on-screen sim.** The analysis pauses off-screen/hidden tabs —
   but the *core use case* is a customer watching the track page for 20–40 minutes with the canvas
   fully visible: 10–20k particles × 60 fps × additive blending on a mid-range Android is real
   battery + thermal drain. No idle throttle is designed. Cheap fix (adopted in PLAN.md): idle-state
   frame-rate cap (~24–30 fps when energy < threshold, full rate on interaction/event) + settle-to-
   static after N idle minutes.
3. **GPU context-loss recovery.** The module handles `webglcontextlost` (stop) but has no
   `webglcontextrestored` re-init path. Android WebViews/Chrome shed WebGL contexts under memory
   pressure routinely; without restore, the ambient layer dies permanently until reload —
   indistinguishable from the "system degraded" visual, which corrupts the truthfulness of §4.2's
   desaturation vocabulary.
4. **Multiple simultaneous islands.** §5 suggests `client:visible` "below-fold instances"; every
   instance is a full WebGL context + rAF loop. Mobile context caps (~8) and battery say: one
   context per page, period (singleton canvas, multiple logical regions if ever needed).
5. **Kiosk hardware assumptions** — no kiosk exists (§4.12 above); P4's "the flagship look" sizes a
   surface with zero product definition (device class, mounting, who grants the camera permission,
   who re-enrolls voice templates on a shared device).
6. **`AudioContext({ sampleRate: 16000 })` on iOS Safari** — hardware-rate quirks are known; a
   fallback resample path (or accepting the context's native rate into the mel front-end) needs a
   line item in P3.
7. **`prefers-reduced-motion` is sampled once at mount** in the prototype (no `change` listener) —
   trivial, but the a11y story quotes it as a hard guarantee.
8. **Photosensitivity never mentioned** — happily, the 1-burst/1.5 s coalescer keeps the design
   under WCAG 2.3.1's three-flashes-per-second line; this should be stated as a design invariant
   (it currently holds by coincidence, and a future "make it punchier" tweak could break it).
9. **WS session sharing on the React admin** — the dashboard already holds an authenticated
   `location:*:dashboard` socket; the store must attach to the existing connection, not open a
   second one per island (double fan-out + double re-authz cost). Minor, but P1-relevant.

---

## 7. Verdict roll-up

| Verdict | Claims |
|---|---|
| **CONFIRMED (10)** | All prototype + MediaPipe + model-CDN sizes (re-measured); motion-flow module honesty; 21-event registry (exact); 10 order statuses (exact); GRAND-PLAN envelope + `read_since` (exact); Chrome 17-langs-no-sq/uk (live-fetched); WS frame/deprecation/re-authz lines; rebuild heartbeat/`Resync`/`Degraded`; hub §4.5 privacy stance; coalescing/tab-hidden/reconnect design |
| **WEAKENED (7)** | "3.6 kB **complete** incl. pointer forces" (flick/swirl/pinch + restore + adaptive guard not in the bytes; realistic core 5–7 kB); "2.3 kB complete voice prototype" (all parts present but worklet path functionally broken as written; +0.5–1 kB for robustness); **MFCC+DTW accuracy in noise** (kitchen ≈50–80% not high-90s; endpointing collapses; push-to-talk required); camera-flow as customer **primary** (permission choreography + declared PP posture ⇒ invert to tilt-primary); "budgets hold / lazy doesn't count" (FE-0.1 gate-mode signature required; 28.1<35 double-spends checkout headroom); ADR-0015 "council-approved" + owner/courier voice vocabulary (ADR is PROPOSED; admin/courier voice explicitly removed from active scope, needs its own council); 7–9 session total (realistic 9–13) |
| **REFUTED (2)** | "Ukrainian as a NET-NEW catalog language" (`SUPPORTED_LOCALES = ['sq','en','uk']` already — packages/ui/src/lib/i18n.ts:72); **P4 kiosk phase premise** (no kiosk surface exists in the product — only a channel-enum string and landing copy) |

**What survives, cleanly:** the hand-rolled WebGL2-TF render tier and its skip-list (three/OGL/
WebGPU all correctly rejected on re-verified numbers), the event→visual vocabulary grounded in a
real 21-event registry, the transport/store design incl. the rebuild's `read_since` upgrade path,
tilt + frame-diff as the phone touchless techniques (order inverted), ambient audio reactivity,
and the confirm-gated safety architecture. The execution plan built on this — minus the kills —
is in `PLAN.md` alongside this file.
