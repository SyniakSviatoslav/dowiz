# Particle-Cloud Execution Plan (2026-07-11)

> **Basis:** `docs/research/2026-07-11-particle-cloud-interaction-analysis.md` **minus** the kills
> and **plus** the corrections in `REVIEW.md` (same directory). Every phase carries a falsifiable
> VbM proof (GREEN + the RED twin) and a CI size gate per the repo's standing rules.
> **Review-driven changes vs the analysis's P1–P4:** tilt becomes the customer-touchless primary
> (camera flow demoted to opt-in "wave mode"); voice becomes push-to-talk with a deterministic
> fixture-corpus accuracy gate and is narrowed to ADR-0015's active scope (owner/courier stateful
> voice needs its own council); the old P4 (kiosk) is parked behind a product decision — no kiosk
> surface exists; battery throttle, context restore, and singleton-canvas rules are added to P1;
> the G05 gate-mode signature becomes a named dependency for the Astro port.

---

## 1. Placement — where this slots in the master plan

- **This work must NOT preempt Wave 0 or Wave 1** (MASTER-EXECUTION-PLAN: the GDPR-trio/G03/`/claim`
  prod vehicle) or the Wave-2 validation week. It is discretionary polish; the G07 arbiter ranking
  (validate-first; then Sovereign MVP > rebuild > bebop > OSS) does not contain it. **Earliest
  start: parallel with Wave 4, and only on an explicit operator funding decision (D-PC1 below).**
  Nothing here rides the Wave-1 PR.
- **P1 targets the React 18 admin SPA (`apps/web`), not the Astro rebuild.** The owner dashboard
  lives only there today (~234 kB gz context, G05 §2.2); the rebuild cutover defaults to mothball
  (G04 Path B) and Astro FE-1+ is arbiter-contingent (G05 funds FE-0 only). Building P1 anywhere
  else would target vapor. The particle core is vanilla TS precisely so this costs nothing later.
- **The customer-surface phase (P2) runs on the live React storefront** (`/s/:slug` track/status
  pages in `apps/web`) for the same reason. An Astro port (P2b) exists as a thin, cheap follow-on
  **gated on two externals:** (a) the arbiter funding G05 FE-1+, and (b) the **FE-0.1 budget
  signature explicitly deciding how deferred decoration chunks count** (maplibre-exclusion
  precedent). Until (b) is signed, assume the chunk counts against 25/35 kB → P2b is NO-GO.
- **Dependency on G05 FE-0:** the CI gz-size gate infrastructure this plan's size gates reuse is an
  FE-0 deliverable. If FE-0 hasn't landed when P1 starts, P1 ships its own minimal
  esbuild+gzip chunk gate (same RED-provable pattern) scoped to `packages/particle-cloud`.

---

## 2. Phases

### P1 — Owner dashboard: ambient field + full event vocabulary (React admin)

**Scope.** The sovereign module + the event store + the React mount, wired to the existing
authenticated `location:<id>:dashboard` WebSocket (attach to the dashboard's **existing** socket —
no second connection), rendering the §4.2 owner vocabulary (order.created burst→glyph,
pending_aging agitation, dispatch_failed turbulence, delivered gold bloom, degradation
desaturation, etc.) with the coalescer (1 burst/1.5 s token bucket, ×N collapse, transactional >
operational > quality via `getEventCategory`), reduced-motion crossfade mode, and snapshot
reconcile on `visibilitychange`/reconnect.

**Module layout** (`packages/particle-cloud/`, vanilla TS, zero deps):

| File | Contents |
|---|---|
| `core/sim.ts` | WebGL2 TF sim + draw (the measured module, productionized: **+ context-restore re-init, + idle frame-rate throttle (~24–30 fps at idle energy, full rate on event/interaction), + adaptive particle-count guard, + settle-to-static after N idle min**) |
| `core/targets.ts` | glyph/text → point-set sampler, cached per glyph |
| `core/palette.ts` | Warm Cosmo-Noir stop sets per audience incl. daylight variant |
| `vocab.ts` | event-kind → (target, palette, energy, transient\|sustained) pure table; unknown kind → ambient (fail-safe) |
| `store/event-stream.ts` | framework-neutral store core: WS attach, reconnect + REST-snapshot reconcile, coalescer token bucket, sustained-state derivation; exposes a serializable `visualState` |
| `store/react-adapter.ts` | `useSyncExternalStore` wrapper (P1 host) |
| `mounts/ParticleCloud.tsx` | thin mount; **singleton canvas per page** (hard rule from review §6.4); sets `data-cloud-state` on its wrapper from the store (the VbM observability hook) |

**Size gate.** CI fails if the particle-cloud chunk (core+vocab+store+mount, esbuild+gzip -9)
exceeds **7,000 B gz**. RED proof committed alongside: a fixture importing a heavy dep turns the
gate red; reverting returns green.

**VbM proof.** GREEN: Playwright vs staging admin — inject a synthetic `order.created` for a
fixture order (staging API) → assert `[data-cloud-state]` transitions `idle → burst:order.created`
within 5 s (real DOM assertion), and a sustained event (`ops.degradation_changed`) sets
`mode:degraded` until cleared. Coalescer unit test: 10 `order.created` in 1 s ⇒ **exactly 1** burst
state + `×10` counter. Vocab table test enumerates all 21 `EVENT_REGISTRY` keys ⇒ every key maps to
a defined tuple (a new registry event without a mapping goes RED). RED twins: flag off / store
detached ⇒ the Playwright assertion fails; bucket bypassed ⇒ the exactly-1 assertion fails.

**Effort:** 2–3 sessions (review-corrected). Feature-flagged, default off.

### P2 — Customer track/status: touch + tilt-primary touchless (live React storefront)

**Scope.** `inputs/pointer.ts` (press-hold attract, flick impulse, pinch — the pieces the review
found outside the measured bytes), `inputs/tilt.ts` (**primary** touchless: DeviceOrientation
gravity-vector steering + shake scatter; iOS one-tap enable flow), `order:<id>` wire + status
reconcile, courier-bearing flow bias, static-gradient no-WebGL2 fallback. **Camera flow
(`inputs/motion.ts`, the 0.8 kB module) ships behind an explicit opt-in "wave mode" tap — never
auto-prompted** — and only after D-PC3 (permission UX copy + Permissions-Policy stance) is decided.

**Size gate.** Inputs (pointer+tilt) ≤ **+1,500 B gz** on the particle chunk; `motion.ts` is a
separate lazy chunk ≤ **1,500 B gz**, loaded only on the opt-in tap.

**VbM proof.** GREEN: Playwright — dispatch synthetic `deviceorientation` events ⇒ store's gravity
vector changes (asserted via `data-cloud-*`/exposed debug state); `pointerdown`+`pointermove` ⇒
pointer-force active; status transition fixture (`CONFIRMED`) ⇒ check-glyph state. Camera path:
Chromium `--use-fake-device-for-media-stream` with a **moving** y4m fixture ⇒ motion energy >
threshold; **static** y4m fixture ⇒ energy < threshold (the RED twin is a real input, not a mock).
Exit gate (unchanged from the analysis, it was right): on-device frame-time check on a
Durrës-class Android before flag-on.

**Effort:** 2–3 sessions. **P2b (Astro island port):** +1–1.5 sessions, **blocked on** arbiter
funding of FE-1+ **and** the FE-0.1 gate-mode signature (see §1); the island is `client:idle`, SSR
renders canvas + CSS-gradient fallback.

### P3 — Voice tier: ambient first, push-to-talk commands second (gated)

Split per the review's accuracy verdict:

- **P3a — ambient audio reactivity (no recognition).** Mic-level RMS/band energy → cloud
  energy/rhythm. No vocabulary, no matching, no false-positive surface; the cheapest wow in the
  program. Still needs mic permission ⇒ still behind D-PC4's UX copy. ~1 session.
- **P3b — enrolled-word commands, push-to-talk only.** Tap-to-arm a ~4 s listening window (the
  review kills always-on listening in noise); mel/MFCC+DTW per-user enrollment (2–3 templates ×
  5–15 commands); reject-quietly; **fix the prototype's framing bug (frames from the ring buffer,
  not per-chunk); add pre-emphasis + cepstral-mean normalization**; claps/whistles as
  speaker-independent read-only extras. **Scope = ADR-0015's active scope: customer-surface
  read-only/navigation intents plus the existing gate's confirmed add-to-cart. Owner/courier ⚠
  stateful commands (accept/confirm/reject/pause) are OUT until the deferred admin/courier voice
  council convenes (red-line marker below).** Proposals flow through the existing
  `packages/voice` ConfirmationGate as an `AsyncIterable<IntentProposal>` source (no callback
  surface — ADR-0015 M3/R2-F).

**Size gate.** DSP core chunk ≤ **4,000 B gz**; enrollment UI island ≤ **4,000 B gz**; both lazy.

**VbM proof (the accuracy gate ships before any UI).** A committed WAV fixture corpus (own
recordings: 3 words × sq/uk/en × 2 enrollment + N test takes, + 50 distractor utterances, + copies
noise-mixed at 10/5/0 dB) drives a deterministic offline test: **quiet accept ≥ 90%, distractor
false-accept ≤ 2%, and a printed accuracy-vs-SNR table** (precision/recall with defined thresholds —
VbM by construction). RED twin: shuffled templates ⇒ accept collapses below 20%, proving the metric
can fail. Playwright: enrollment flow completes; a voice-proposed stateful intent **never** executes
without the tap (reuses the packages/voice gate test). i18n: new keys added to the **existing**
sq/en/uk catalog (no new locale — review §4.11).

**Effort:** P3a 1 session · P3b 3–4 sessions incl. a human-with-microphone tuning session.
**Go/no-go:** if the 0–5 dB rows of the SNR table are unusable, P3b ships as quiet-environment
customer feature only — pre-committed, not renegotiated after the fact.

### P4 — Hand tracking (PARKED — conditional)

No kiosk surface exists in the product (review §4.12). P4 is **parked** until an operator decision
commissions a kiosk/desktop surface. Preconditions recorded now so nothing is relearned later:
self-hosted wasm + `.task` with SW Cache-Storage (second-visit-0-bytes proof via Playwright CDP
transferSize), **CSP `script-src` needs `'wasm-unsafe-eval'`** (current policy blocks MediaPipe
WASM), Permissions-Policy camera allowance for the kiosk origin/route, inference in a Web Worker,
degradation ladder to motion-flow. Landmark→force mapping is unit-testable against recorded
landmark fixtures. **Effort if commissioned:** 2–3 sessions + kiosk product definition (not ours).

---

## 3. Operator decision points & red lines

| # | Decision | Phase gate | Notes |
|---|---|---|---|
| D-PC1 | Fund the program at all (vs. arbiter's validate-first ranking) | before P1 | Never rides Wave 0/1; earliest = Wave-4-parallel |
| D-PC2 | FE-0.1 budget signature: do deferred decoration chunks count against 25/35/60? | before P2b only | Maplibre-exclusion precedent; unsigned ⇒ P2b NO-GO |
| D-PC3 | Camera "wave mode": opt-in UX copy + Permissions-Policy stance (`headers.ts:41` declares `camera=()`) | before P2 camera flag-on | **Red-line-adjacent: any edit to `security/headers.ts`/CSP goes through the security lane** |
| D-PC4 | Mic permission UX copy (both P3a/P3b) + on-device privacy line | before P3 | "Analyzed on this device, never recorded or sent" — matches repo doctrine |
| D-PC5 | Owner/courier stateful voice (accept/confirm/reject/pause) | separate council | **RED LINE:** ADR-0015 removed admin/courier voice from active scope; DTW-instead-of-Whisper does not reopen it. Any money/state-mutating voice command anywhere = ConfirmationGate + explicit tap, no exceptions |
| D-PC6 | Kiosk surface: commission or keep parked | unparks P4 | Includes device class, shared-device enrollment policy, camera consent signage |

Standing red lines inherited: money-adjacent events (`cash.reconcile_discrepancy`) are
visualization-only; the cloud is always a redundant channel (never the sole carrier of any signal);
flash rate stays ≤ 1 burst/1.5 s as a **stated WCAG 2.3.1 invariant**, not a tunable.

## 4. Effort summary (review-corrected)

| Phase | Sessions | Status |
|---|---|---|
| P1 owner dashboard (React admin) | 2–3 | ready after D-PC1 |
| P2 customer touch + tilt (+ opt-in camera) | 2–3 | after P1; camera part after D-PC3 |
| P2b Astro island port | 1–1.5 | blocked: arbiter FE-1+ + D-PC2 |
| P3a ambient audio | 1 | after D-PC4 |
| P3b push-to-talk words | 3–4 | after P3a gate corpus GREEN |
| P4 hand tracking | 2–3 | PARKED (D-PC6) |
| **Funded core (P1+P2)** | **4–6** | |
| **Full program** | **≈ 10–13** | vs the analysis's 7–9 |

## 5. Skip list (carried forward + review additions)

Carried forward intact from the analysis (all re-verified): three.js/Threlte/OGL for this feature ·
WebGPU/TSL compute path · server-side particle rendering · always-on CV on customer phones · Web
Speech API + Whisper-as-primary (Chrome's 17 on-device langs have no sq/uk — re-verified) ·
pitchfinder (GPL) · meyda as dependency · WebNN until ≥2027 · SDF morphing, physics engines,
`gesture_recognizer.task` on phones.

**Added by this review:** continuous open-mic recognition in any environment (push-to-talk only) ·
camera-permission-primary touchless on customer phones (tilt is primary) · P4/kiosk until a kiosk
surface exists · multiple particle canvases per page (singleton rule) · any voice binding for
owner/courier stateful actions absent a new council (ADR-0015 scope) · treating `client:idle`
chunks as budget-free before the FE-0.1 gate-mode signature.
