# Particle-Cloud Interaction Analysis — Touch · Camera-Motion · Voice-DSP · Live Events (Rust + Astro + Svelte)

> **Date:** 2026-07-11 · **Method:** builds on `docs/research/2026-07-11-design-libraries-research.md`
> (its verified verdicts inherited, not re-derived) + web survey with independent verification
> (HTTP HEAD on model files, npm tarball measurement, esbuild `--bundle --minify` + `gzip -9` on
> three prototype modules written for this analysis in `/root/.claude/jobs/c6a4c73f/tmp/measure/`)
> + in-repo evidence (event registry, WS fan-out, rebuild PgListener, brand tokens, voice engine).
> Every load-bearing number is cited or marked UNVERIFIED. **Research only — no repo file outside
> this doc was touched.**
>
> **The ask:** real-time multicolored particle clouds that (a) morph on touch, (b) react to distant
> hand/finger movement via camera, (c) react to every state change/event worth surfacing to courier,
> customer, or venue owner — an ambient reactive status display, not decoration. Scope additions
> mid-run (operator): phone touchless interaction is REQUIRED (not opt-in); voice tier is REQUIRED
> and must be **pure WebAudio DSP, zero AI**, launching sq/uk/en.

---

## 0. Executive summary — the recommended stack in 5 lines

1. **Render:** one hand-rolled **WebGL2 transform-feedback** particle module (vanilla TS, framework-agnostic core + thin Svelte/React mounts) — **measured 3.6 kB gz** complete with morph-targets, pointer forces, palette/energy API; 10k–100k particles; canvas2D/static-gradient fallback; no library (OGL only if it ever grows meshes; three/TSL never for this).
2. **Inputs, one force bus:** touch (pointer events, ~0 kB) · phone touchless = **64×48 frame-diff global optical flow, measured 0.8 kB gz** (primary) + DeviceOrientation tilt (fallback, ~0.3 kB) · kiosk/desktop opt-in **MediaPipe HandLandmarker** lazy layer (11.1 MB one-time, SW-cached, never on customer phones by default).
3. **Voice = zero-AI DSP, measured 2.3 kB gz:** mel/MFCC + DTW template matching against **per-user enrolled words** (language-independent → sq/uk/en solved by design) + clap/whistle acoustic commands + ambient mic-level drive; all on-device, nothing uploaded; mutating commands always need visual-confirm + tap.
4. **Events:** one Svelte-5 runes event-store fed by today's Node WS rooms (`order:`/`location:*:dashboard`/`courier:*`), later the rebuild's axum PgListener + `Resync`/seq envelope; a fixed **event→visual vocabulary** (burst/condense/stream/desaturate) grounded in Warm Cosmo-Noir tokens, coalesced under burst load, reduced-motion-safe, never the only signal.
5. **Budgets hold:** full stack (particles+store+touch+motion+voice) ≈ **8–10 kB gz**, lazy-loaded outside the critical path — fits inside G05's 25/35/60 kB route classes with room to spare; the only >60 kB item (MediaPipe) is a user-triggered opt-in download, not route JS.

**Phases:** P1 owner-dashboard ambient+events (1.5–2 sessions) → P2 customer touch + REQUIRED phone touchless (1.5–2) → P3 voice-DSP with enrollment (2–3) → P4 kiosk/desktop hand-tracking (2).

---

## 1. Particle rendering tier

### 1.1 Technique comparison (10k–500k multicolored particles, real-time morphing, WebGL2 baseline)

| Technique | How | Count ceiling | Fit for this product |
|---|---|---|---|
| **(a) Transform feedback GPGPU** | vertex shader integrates pos/vel into ping-pong `ARRAY_BUFFER`s, rasterizer discarded; second pass draws points | ~1M on desktop demos; mobile ceiling UNVERIFIED, adaptive 15k–100k realistic | ⭐ **The pick.** Simplest WebGL2-native GPGPU; no float-texture extensions needed; particle state never touches the CPU ([gpfault WebGL2 particles](https://gpfault.net/posts/webgl2-particles.txt.html), [webgl2fundamentals GPGPU](https://webgl2fundamentals.org/webgl/lessons/webgl-gpgpu.html)) |
| **(b) FBO ping-pong position textures** | positions in float textures, fragment shader simulates, vertex shader reads texture (classic curl-noise clouds) | similar | Equivalent power; needed only when particles must read *neighbor* state or the sim result must be a texture. Requires `EXT_color_buffer_float` rendering support. The pre-WebGL2 classic ([barradeau FBO particles](https://barradeau.com/blog/?p=621), [Codrops UntilLabs living-particles 2025-12-10](https://tympanus.net/codrops/2025/12/10/simulating-life-in-the-browser-creating-a-living-particle-system-for-the-untillabs-website/)) |
| **(c) WebGPU compute (TSL / raw WGSL)** | compute shaders update storage buffers; three r185 TSL examples run 500k+ ([webgpu_compute_particles](https://threejs.org/examples/webgpu_compute_particles.html), [webgpu_tsl_compute_attractors_particles](https://threejs.org/examples/webgpu_tsl_compute_attractors_particles.html)) | millions | Progressive enhancement **not worth it here**: three's WebGPU/TSL path measured ~207 kB gz and WebGPURenderer is still officially experimental (prior report §2.3); WebGPU covers 83.63 % global ([caniuse, fetched live in prior report §2.5](https://caniuse.com/webgpu)) so WebGL2 fallback stays mandatory — and our counts (≤100k) don't need compute. Raw WGSL dual-path = 2× shader maintenance for zero visible gain at this tier. Revisit only if a "full scene" tier ever exists. |
| **(d) CPU fallback (canvas2D)** | few hundred particles, JS loop | ~300–800 | For the rare no-WebGL2 client; below that, the 0 kB static gradient + `feTurbulence` grain already in the skin (BRAND-BIBLE §8). |

### 1.2 Shape morphing — how particles become glyphs/icons

- **Precomputed target-point sets ⭐ (the pick):** draw text/SVG/icon to an offscreen 2D canvas, sample opaque pixels into an `RG32F` target texture, each particle springs to its own texel (`v += (tgt−p)·k·dt`). Cheap (one canvas rasterize per event kind, cacheable), deterministic, works for order numbers, check-marks, arrows, digits. This is the technique behind every "particles form text/face" piece surveyed ([Codrops Phantom.land face-particle system 2025-06-30](https://tympanus.net/codrops/2025/06/30/invisible-forces-the-making-of-phantom-lands-interactive-grid-and-3d-face-particle-system/)).
- **SDF-target attraction:** encode the shape as a signed-distance field, particles descend the gradient. Smoother edge flow, but needs SDF generation (extra pass or asset) and buys little over point sets at 15k–60k particles. **Skip for v1**; note as upgrade if morph quality ever disappoints.
- **Attractor/flow fields:** curl-ish noise field for the *ambient* state (this is the idle look), attractor points for pointer/courier-direction bias. Included in the measured module below.
- **Spring-to-target:** the integration scheme used on top of point sets (critically damped spring = no oscillation overshoot). Included.

### 1.3 Library fit + honest gz accounting (against G05 budgets 25/35/60 kB gz)

G05 ratified: `/s/[slug]` critical-path JS ≤25 kB gz (read parity), ≤35 kB with the checkout island, hard ceiling 60 kB; measured base today 21.6 kB, Svelte-5 floor 14.3 kB (`docs/design/gap-blueprints-2026-07-11/G05-astro-fe-parity.md` §2.1/§3-Option-A).

| Option | gz cost (evidence) | Verdict |
|---|---|---|
| **Hand-rolled WebGL2 TF module** | **3,586 B gz measured** this session — a *complete* representative module: TF ping-pong pos/vel, hash-noise flow field, pointer attract/repel, morph-target texture + glyph sampling from canvas text, 3-stop palette + desaturation uniform, energy/pulse event API, DPR resize, IntersectionObserver pause, reduced-motion freeze, context-loss cleanup (`/root/.claude/jobs/c6a4c73f/tmp/measure/particles-tf.js`, esbuild+gzip -9; reference: prior report's raw-hero 1,473 B re-measured same run) | ⭐ **Adopt.** Fits any budget; zero deps; the prior report's §2.6 rung-1 conclusion extends cleanly from quad-shader to particle system |
| OGL subset | ~14–20 kB gz, Unlicense (prior report §2.2, verified) | Step-up only if meshes/render-targets/post passes arrive. Not needed for points |
| three r185 (WebGL) | ~129 kB gz minimal measured (prior §2.1) | 5× the landing budget — skip for this feature |
| three WebGPU/TSL | ~207 kB gz measured (prior §2.3) | Skip (see 1.1c) |
| Threlte 8 wrapper | three's floor + wrapper (130–210 kB band, prior §2.4) | Right shape for hypothetical full scenes; irrelevant here. **Note:** the particle island must be vanilla-core anyway because today's owner dashboard is React 18 (`apps/web/package.json`) while the storefront rebuild is Astro+Svelte — one core, two 10-line mounts |

### 1.4 Recommendation ladder per surface

| Surface | Particle budget | Config |
|---|---|---|
| Customer phone (`/s/:slug`, track page — mid-range Android, Durrës) | 3.6 kB module, lazy (`client:idle`) | 10k–20k particles, DPR cap 2, `powerPreference:'low-power'`, adaptive count (halve on >20 ms frames) |
| Courier phone (PWA, outdoors) | same module + daylight palette | 8k–15k; `[data-daylight]` remaps to ink-on-paper high contrast (BRAND-BIBLE §3 daylight variant); density replaces glow |
| Owner dashboard (React admin today) | same vanilla core, React mount | 30k–60k; desktop/tablet GPU |
| Venue kiosk / tablet | same + CV layer (§3) | 60k–100k; the flagship look |

Particle-count ceilings on specific mid-range hardware: **UNVERIFIED** — no trustworthy published numbers; ship the adaptive-count guard (measure `dt`, halve N until <16.7 ms) instead of trusting any benchmark.

---

## 2. Touch input tier

- **Pointer Events only** (`pointermove/down/up/cancel`, `{ passive: true }`): unifies mouse/touch/pen; the sim reads a pointer uniform — no per-event work beyond storing NDC coords, so listener latency is a non-issue. Never call `preventDefault` on move (keeps scroll native — the storefront canvas must not eat scrolling; use `touch-action: pan-y` on the canvas so vertical scroll passes through and horizontal drags drive the cloud).
- **Forces:** press-hold = attract (particles pool under the finger), flick = directional impulse from pointer velocity, second pointer = swirl (angular force around the midpoint), pinch (two-pointer distance delta) = condense/expand the morph target scale. All are uniform updates — trivial.
- **What needs care on mid-range Android:** (1) `getCoalescedEvents()` for smooth force trails at 60 Hz+ input on 120 Hz-touch devices (cheap, optional); (2) never resize the canvas on keyboard-open viewport changes (listen to `visualViewport` and ignore height-only changes); (3) battery — pause simulation on `document.visibilitychange` and off-screen (`IntersectionObserver`, already in the measured module); (4) reduced-motion freeze (already in module). Multi-touch pinch-morph is the only piece above trivial: ~30 lines of two-pointer bookkeeping.
- Cost: included in the 3.6 kB measurement (pointer wiring was in the measured module).

---

## 3. CV hand-tracking tier

### 3.1 Full hand tracking, 2026 state of the art (kiosk / owner-desktop tier)

**MediaPipe Tasks HandLandmarker** (`@mediapipe/tasks-vision` 0.10.35) remains the production option; TF.js `hand-pose-detection` is a wrapper over the same MediaPipe stack and its last release is ~a year stale ([socket.dev health note](https://socket.dev/npm/package/@tensorflow-models/hand-pose-detection), [tfjs-models repo](https://github.com/tensorflow/tfjs-models/tree/master/hand-pose-detection)) — no reason to add the wrapper. No lighter production-grade browser hand model emerged by mid-2026 (searched; the "lighter" energy went to NPU/WebNN, below).

**Real download sizes (measured this session, npm tarball 0.10.35 + HTTP HEAD on Google's model CDN):**

| Asset | Raw | Over-wire (gz) |
|---|---|---|
| `vision_bundle.mjs` (JS API) | 137.0 kB | **39.7 kB** |
| `vision_wasm_internal.js` loader | 322 kB | 77.5 kB |
| `vision_wasm_internal.wasm` (SIMD) | 11.15 MB | **3.28 MB** |
| `hand_landmarker.task` (float16 — the only published variant; no "lite" exists) | **7.82 MB** (7,819,105 B via HEAD) | ~7.8 MB (already-quantized weights barely compress) |
| **Total first visit** | | **≈ 11.2 MB** |
| (`gesture_recognizer.task` alternative — adds canned open-palm/fist/thumbs-up classification) | 8.37 MB (8,373,440 B via HEAD) | swap, not add |

Sources: [hand landmarker model docs](https://developers.google.com/edge/mediapipe/solutions/vision/hand_landmarker) (model card table: HandLandmarker (full), float16, Pixel 6 latency **17.12 ms CPU / 12.27 ms GPU** — native benchmark, not browser); [web guide](https://developers.google.com/edge/mediapipe/solutions/vision/hand_landmarker/web_js) (`FilesetResolver`, `detectForVideo()` **blocks the main thread — run it in a Web Worker**); the shipped typings expose `baseOptions.delegate?: "CPU" | "GPU"` (verified in `vision.d.ts` from the tarball — the web GPU delegate is WebGL-based inference; the web_js options table omits it but the API carries it). Browser FPS on mid-range hardware: the only published browser datapoint found is ~20 fps on a Pixel 3 tracking 2 hands (2021-era, [Towards Data Science](https://towardsdatascience.com/exquisite-hand-and-finger-tracking-in-web-browsers-with-mediapipes-machine-learning-models-2c4c2beee5df/)); 2026 mid-range browser FPS and init time: **UNVERIFIED — measure on the actual kiosk tablet before committing**. Battery/thermal drain numbers: **UNVERIFIED** (no credible public data found; continuous camera+inference is assumed heavy — another reason this tier is kiosk/desktop-only).

**WebNN status:** Chromium-only, origin-trial M147–M149, explicitly not production-viable cross-browser in early 2026 ([webstatus.dev/features/webnn](https://webstatus.dev/features/webnn), [ddevtools Jan 2026 overview](https://www.ddevtools.com/updates/2026-01-webgpu-webnn-browser-ai), [MS WebNN overview](https://learn.microsoft.com/en-us/windows/ai/directml/webnn-overview)). NPU offload would eventually fix the battery objection — watch, don't build.

**Load choreography (this tier blows every route budget by design — so it never rides a route):** user taps "wave mode" (or kiosk boots into it) → dynamic-import a `hands.ts` chunk (~40 kB gz JS) → `FilesetResolver` pointed at **self-hosted** wasm + `.task` (same-origin → Service-Worker `Cache Storage` caches all of it; second visit = 0 bytes; the Cache API stores any same-origin response including wasm and the model — [MDN Cache API](https://developer.mozilla.org/en-US/docs/Web/API/Cache)) → progress UI on the 11 MB first fetch → landmarks stream from a worker at camera FPS → mapped to the same force bus as touch.

**Gesture vocabulary (landmark-derived, deterministic post-processing — no gesture model needed):** open palm (5 extended fingers) = scatter · pinch (thumb-index distance < threshold) = condense/grab the morph target · palm-swipe velocity = directional wave · two hands apart/together = expand/contract. Index-fingertip position = the same attractor uniform touch uses.

**Privacy stance (state it in UI copy):** all inference on-device in WASM/WebGL inside the browser; **camera frames never leave the page — no frame, landmark, or derived data is transmitted or stored.** This matches the repo's privacy-maximalist doctrine (GPS handling, hub review §4.5).

**Degradation ladder:** no camera hardware / permission denied / `available()` fails → touch-only (everything still works) · sustained <15 FPS inference → auto-drop to §3.2 motion-flow (it's 0.8 kB and already loaded on phones) · no WebGL2 → static gradient + CSS grain.

### 3.2 CV on the customer phone: what actually fits (operator-mandated re-examination — and now a REQUIRED tier)

The "no CV on customer phones" reflex assumed CV = ML model. Re-examined honestly, three phone-viable options exist, and the first is now the **required** touchless interaction for the customer surface:

**1. Model-free optical flow / frame differencing ⭐ PRIMARY (measured 0.8 kB gz).**
Downscale the front camera to 64×48, frame-difference + solve one global Lucas-Kanade normal-equation system per tick → `{energy, dx, dy, centroid}` driving the particle flow field. A prototype written and measured this session bundles to **777 B gz** (`/root/.claude/jobs/c6a4c73f/tmp/measure/motion-flow.js`). Compute cost: 64×48 = 3,072 px × ~15 ops at 15 fps ≈ **0.7 M ops/s — negligible on any 2020+ phone** (deterministic arithmetic, no model, no allocation per frame). The technique is the classic webcam-interactive-art approach; the reference JS implementation is anvaka's **oflow** (block-matching optical flow, webcam demo + ping-pong game — [github.com/anvaka/oflow](https://github.com/anvaka/oflow)), measured 3.6 kB gz from its npm dist but last modified 2022-06 — the hand-rolled 0.8 kB version wins on size and freshness; oflow stands as evidence the approach works live in browsers. **Gesture limits (honest):** motion energy + direction + rough position only — wave/swipe/approach; **no pinch, no finger poses, no hand shape.** For "wave at your order and the cloud ripples back," that is exactly enough. Camera permission is still required (a soft ask with visible value the moment it's granted). FPS claim basis: arithmetic cost analysis above + oflow's real-time demos; on-device verification on a Durrës-class Android: cheap to do in P2, flagged as the P2 exit check.
**2. Device motion/orientation sensors — FALLBACK (~0.3 kB, no camera at all).**
Tilt steers the flow field's gravity vector; shake = scatter pulse. `deviceorientation`/`devicemotion` fire without any permission on Android Chrome; iOS 13+ requires `DeviceOrientationEvent.requestPermission()` from a user-gesture tap in a secure context ([dev.to guide](https://dev.to/li/how-to-requestpermission-for-devicemotion-and-deviceorientation-events-in-ios-13-46g2), [Lee Martin/Medium](https://leemartin.dev/how-to-request-device-motion-and-orientation-permission-in-ios-13-74fc9d6cd140)) — one "enable motion" tap on iOS, zero taps on Android. This is the no-camera/no-permission fallback so "touchless reacts" is true on essentially every phone.
**3. Cached opt-in full tracking — NOT default on phones.** The full HandLandmarker stack (≈11.2 MB first load, §3.1) is cacheable via SW so repeats are free, but first-load weight + battery on mid-range customer hardware + zero incremental gesture value over flow (at phone screen distance the finger is ON the screen — touch already wins) keep it kiosk/desktop-only. A hidden "wave mode" easter-egg toggle on the track page is acceptable later; not a phase commitment.

**Ranked for the customer phone: (1) frame-diff optical flow (primary, in the required P2 scope) → (2) tilt/shake sensors (fallback + iOS-no-camera path) → (3) HandLandmarker (skip on phones).** Camera-permission denial-rate data: **UNVERIFIED** (no published per-market data found) — mitigated by the sensor fallback and by asking only after showing the cloud.

---

## 4. Event-reactive tier (the product core)

### 4.1 The real event landscape (all verified in code)

- **Owner notification registry — 21 events** (`apps/api/src/notifications/event-registry.ts:16-167`): `order.created` (confirm/reject), `order.confirmed`, `order.rejected`, `order.delivered`, `order.substitution_needs_human`, `order.dwell_escalation`, `order.timeout_cancelled`, `order.dispatch_failed`, `order.pending_aging`, `order.ready_for_pickup`, `cash.reconcile_discrepancy`, `delivery.flag_raised`, `rating.low_received`, `courier.assigned`, `shift.started/closed/close_reminder`, `ops.worker_liveness`, `ops.backup_failed`, `ops.degradation_changed`, `test` — with the category doctrine (transactional never suppressed; reversibility-of-consequence, `event-registry.ts:175-194`).
- **Bus vocabulary — ~46 channels** (`apps/api/src/lib/registry.ts` `BUS_CHANNELS`): the order lifecycle (`order.created/confirmed/rejected/picked_up/in_delivery/delivered/cancelled/dispatch_failed/assignment_created/cancelled.customer_after_dispatch`), courier (`courier.position_updated`, `courier.stale_heartbeat`), shifts, settlements, backups, dwell, menu-import, GDPR/anonymizer, worker liveness, signals (`signal.created/acknowledged/dismissed`).
- **Order state machine — 10 statuses** (`packages/shared-types/src/contracts/owner/orders.ts:4`): `PENDING, CONFIRMED, PREPARING, READY, IN_DELIVERY, DELIVERED, REJECTED, CANCELLED, SCHEDULED, PICKED_UP`.
- **WS rooms actually fanned out** (`apps/api/src/websocket.ts`, publish sites in routes/workers): `order:<id>` (customer + bound courier + owner), `location:<id>:dashboard` (owner live feed: `offer_sent/offer_declined/assignment_aborted/binding_changed`, dwell alerts, signals, GDPR notices), `courier:<id>` (task_offered…), `courier:<id>:shift`. Frames are `{room, data:{type,…}}` (`websocket.ts:218`), every member re-authorized on the broadcast path (courier + owner relay guards).
- **Degrade/error classes worth visualizing:** `ops.degradation_changed`, `courier.stale_heartbeat`, `worker.*`, WS `Health::Degraded` from the rebuild's heartbeat monitor (`rebuild/crates/api/src/ws/pg_fanout.rs` — the "connected-but-mute" detector), checkout 422 contract errors (G03).

### 4.2 Event → visual vocabulary

Grammar first — every event maps to a tuple **(shape-target, palette-shift, motion-energy, transient|sustained)**, rendered by the §1 module's four knobs (`setTargetsFromText/scatter`, colors, `pulse(energy)`, `setMode({morph,sat,energy})`). Palettes are the Warm Cosmo-Noir tokens verbatim (`docs/design/dowiz-brand/BRAND-BIBLE.md` §3): field lives in `--void/--hull` dark; **amber `#E8A544` = life/action, teal `#46B0A4` = success/alive-signal, rust `#B26850` = warm warning, blood `#E0543E` = danger, gold `#C79675` = completion, magenta `#C8438F` = rare anomaly accent; one saturated accent per view** (color law 2), and motion follows the "low-power idle → sharp engage" doctrine (§6: `--ease-jazz-snap` 180 ms engage, 3–4 s ambient breathing).

**Owner dashboard (`location:*:dashboard` + registry events):**

| Event | Visual |
|---|---|
| `order.created` | **warm burst** — amber pulse (energy spike) → particles **condense into the order-number glyph** ~4 s → release to ambient |
| `order.pending_aging` / `order.dwell_escalation` | **sustained agitation** — rust drift, slowly tightening ring; persists until the order is actioned (state-derived, not event-derived) |
| `order.dispatch_failed` / `delivery.flag_raised` | blood-tinted turbulence + field desaturation + "!" glyph morph (paired with the existing Telegram/push alert — never color-only, WCAG 1.4.1 / color law 4) |
| `offer_sent` → `courier.assigned` | **directional teal stream** toward screen edge; assignment lands = brief teal condense |
| `order.ready_for_pickup` | teal condense pulse |
| `order.delivered` | gold bloom → slow settle (completion breath) |
| `cash.reconcile_discrepancy` | magenta flicker + hold (the rare-accent budget spent only on money anomalies) |
| `shift.started/closed` | perimeter ring forms/disperses, low energy |
| `ops.degradation_changed` / `worker.*` / WS degraded | **global sustained desaturation** (`uSat`↓) + turbulence↑ — the room itself feels "off" until healthy |
| `rating.low_received` | brief cold contraction (default-off quality category, mirrors prefs) |

**Customer (`order:<id>`, storefront/track):** `CONFIRMED` = amber bloom → check-mark glyph · courier accepted/assigned = directional stream + courier glyph · `PICKED_UP/IN_DELIVERY` = travelling wave; live courier bearing (from `order.courier_updated` polyline/position data) biases the flow field toward the courier's direction — the cloud literally leans toward your food · `DELIVERED` = gold+amber celebration burst → settle · `REJECTED/timeout_cancelled` = desaturate + slow fall (with the existing text message) · checkout 422/network error = grain turbulence + desaturation behind the error toast.

**Courier (`courier:<id>`, daylight skin):** `task_offered` = hard condense into order-number glyph + shrinking countdown ring (the accept window made visible) · `binding_changed/assignment_aborted` = scatter + desaturate · `shift.close_reminder` = pulsing perimeter ring. Daylight variant (BRAND-BIBLE §3): ink-density on paper, no glow — sun-readable.

### 4.3 Idle, burst load, reduced motion, accessibility

- **Idle/ambient:** the "2 a.m. bar" state — 0.15 energy, slow hash-noise drift, amber-in-noir 90/10 grading, pilot-light breathing (3–4 s). Idle IS the brand asset; events are deviations from it.
- **Queuing/coalescing (dinner rush ≠ strobe):** transient bursts go through a token bucket — **max 1 burst per ~1.5 s**; N `order.created` inside a window collapse into one burst with a "×N" glyph; priority = transactional > operational > quality (reuse `getEventCategory`, `event-registry.ts:190`). Sustained states are **derived from store state, not replayed events** — 40 events during a hidden tab reconcile to one final state, never 40 animations.
- **`prefers-reduced-motion`:** simulation `dt`=0 (frozen positions — already in the measured module); events map to **opacity/palette crossfades only** (color + the static glyph outline still communicate); matches BRAND-BIBLE §6's reduced-motion law.
- **Accessibility (hard rule):** the cloud is a **redundant, ambient channel** — every event it visualizes is already delivered by the existing notification system (Telegram/push/badges/ARIA live regions). It accompanies; it never replaces. No information exists only in the cloud; glyph morphs pair shape with color so no state is color-only.

---

## 5. Transport / state wiring

- **Today (Node):** `PgMessageBus` (LISTEN/NOTIFY on a dedicated session-mode client, 7,800 B payload cap with claim-check truncation — `packages/platform/src/message-bus.ts`) → `setupWebSocket` room fan-out with per-frame re-authz (`apps/api/src/websocket.ts`). The island authenticates via message-auth (`{type:'auth',token}` — the URL-token path is deprecated, `websocket.ts:179`), subscribes its room, receives `{room,data}`. **No seq, no replay:** a hidden/offline tab misses frames silently. Mitigation now: on `visibilitychange`→visible or WS reconnect, **re-fetch the REST snapshot** (order status / dashboard orders) and reconcile sustained state; missed *transients* are deliberately dropped (coalescing policy §4.3 makes this correct by design).
- **Rebuild (axum):** `PgListener` port with the two upgrades the visual layer wants: an **active self-NOTIFY heartbeat** that detects the connected-but-mute blackout (`Health::Degraded` → map directly to the §4.2 desaturation state — a truthful "system off" visual for free) and a typed **`ControlFrame::Resync{entity,id}`** for truncated payloads → an explicit refetch contract (`rebuild/crates/api/src/ws/pg_fanout.rs`, `ws/protocol.rs`).
- **Does the 1.4 envelope carry enough?** Yes. `order_events` rows carry `(order_id, seq, at, cause_hash, payload=canonical event bytes, content_hash)` (GRAND-PLAN §1.2–1.4) — event kind lives in the payload; `seq`+`at` give ordering and staleness; the 1.3 port's `read_since(scope, seq)` is precisely the missed-events replay the tab-hidden case wants. **No new fields needed**; when 1.2/1.3 land, swap the REST-snapshot reconcile for `read_since` catch-up (transients still coalesced).
- **One shared store:** `event-stream.svelte.ts` — a Svelte 5 runes module (`$state` connection health, last-event, per-entity sustained state; `$derived` visual-state tuple) with the coalescing token bucket inside. The particle island and any UI badge/toast consume the same store, so visuals and chrome can never disagree. The core is plain TS (runes in a `.svelte.ts` module); for the React admin (P1 host), the same core is wrapped with `useSyncExternalStore` — one wire protocol, two 15-line adapters.
- **Astro integration:** the cloud is its own island — `client:idle` for the ambient background layer (it must not compete with MenuBrowser hydration; it's decoration until an event arrives), `client:visible` for below-fold instances. SSR safety: module top-level has zero `window` references (all wiring inside mount/`onMount`); server renders the `<canvas>` + the CSS static-gradient fallback behind it, so no-JS and pre-hydration states are the brand gradient — the same degradation terminus as no-WebGL2. Astro client directives: [docs.astro.build/en/reference/directives-reference](https://docs.astro.build/en/reference/directives-reference/#client-directives).

---

## 6. Recommendation + phased build path

### 6.1 Architecture (file-level sketch, descriptions not code)

```
packages/particle-cloud/            (vanilla TS, zero deps — the sovereign module)
  core/sim.ts          WebGL2 TF sim + draw (the measured 3.6 kB module, split for tests)
  core/targets.ts      glyph/SVG → point-set sampler (offscreen canvas, cached per glyph)
  core/palette.ts      Warm Cosmo-Noir stop sets per audience incl. daylight variant
  vocab.ts             event-kind → (target, palette, energy, transient|sustained) table (§4.2)
  inputs/pointer.ts    touch/pointer forces (passive, coalesced, pinch)
  inputs/motion.ts     64×48 frame-diff flow (the measured 0.8 kB module)
  inputs/tilt.ts       DeviceOrientation/Motion fallback (+ iOS permission tap)
  inputs/hands.ts      lazy MediaPipe layer (dynamic chunk; kiosk/desktop; SW-cached)
  inputs/voice/        §7: dsp.ts (mel+DTW+YIN+clap — the measured 2.3 kB), enroll.ts, vocab-{sq,uk,en}.ts
  store/event-stream.svelte.ts   runes store: WS client, reconnect+snapshot reconcile, coalescer
  store/react-adapter.ts         useSyncExternalStore wrapper for apps/web
  mounts/ParticleCloud.svelte · mounts/ParticleCloud.tsx   (thin)
```

### 6.2 Phases, size, effort

| Phase | Scope | Added gz (route) | Effort |
|---|---|---|---|
| **P1** Owner dashboard: ambient + full event vocabulary | sim+targets+palette+vocab+store, React mount, `location:*:dashboard` wire, coalescer, reduced-motion | ~6 kB (irrelevant against the React app's ~234 kB; G05 §2.2) | 1.5–2 sessions |
| **P2** Customer storefront/track: touch + **REQUIRED phone touchless** | pointer forces, `order:<id>` wire + status reconcile, **motion-flow primary + tilt fallback**, Astro `client:idle` island, static-gradient fallback | +~6.5 kB lazy (21.6 base untouched at critical path; worst-case in-budget vs 25/35) | 1.5–2 sessions (+on-device FPS check = exit gate) |
| **P3** Voice-DSP tier (all surfaces, opt-in mic) | §7: dsp core, enrollment UX (sq/uk/en), clap/whistle, confirm-gated mutations | +~5 kB lazy (2.3 core + UI island) | 2–3 sessions (threshold tuning needs a human + mic) |
| **P4** Kiosk / owner-desktop hand tracking | hands.ts chunk, SW caching of self-hosted wasm+task, gesture mapping, degradation to motion-flow | ~40 kB JS chunk + 11.2 MB one-time opt-in assets (outside route budgets by design) | 2 sessions |

Cross-check vs budgets: customer critical path stays 21.6 kB; everything above is lazy `client:idle`/user-triggered; even counted worst-case, 21.6+6.5 = 28.1 kB < 35 kB post-checkout target, ≪60 ceiling (G05 §3-A).

### 6.3 Skip list

| Skip | Why |
|---|---|
| three.js / Threlte / OGL for this feature | 129–210 kB vs a measured 3.6 kB hand-rolled module; OGL only if meshes ever arrive (prior report §2.6) |
| WebGPU compute path now | ~207 kB TSL path, experimental renderer, 16 % of users lack WebGPU, and ≤100k particles don't need compute (§1.1c) |
| Server-side particle rendering | The Rust side's job is the event log/envelope, not pixels; SSR canvas is a contradiction — server ships the CSS gradient fallback only |
| Always-on CV on customer phones | 11.2 MB + battery for gestures touch already does better at phone distance; motion-flow (0.8 kB) delivers the touchless feel instead (§3.2) |
| Web Speech API + the existing whisper engine as voice primary | Operator: zero-AI. Also independently disqualified: Chrome's on-device recognition supports 17 languages — **no sq, no uk** ([explainer list](https://github.com/WebAudio/web-speech-api/blob/main/explainers/on-device-speech-recognition.md)); default mode ships audio to servers; Firefox disabled 22–155, global support 87.24 % *partial* ([caniuse](https://caniuse.com/speech-recognition)); Safari's engine can't do sq either. The in-repo `@deliveryos/voice` transcriber is Whisper via Transformers.js (~130 MB q8, `packages/voice/src/transformers-transcriber.ts`) = a local ML model → fails "no AI"; **its deterministic safety spine (IntentProposal/ConfirmationGate/matcher) is transcript-agnostic and IS reused** (§7.5) |
| pitchfinder npm | GPL-3.0 ([npm](https://www.npmjs.com/package/pitchfinder)); YIN is ~40 lines hand-rolled (measured inside the 2.3 kB) |
| meyda as a dependency | MIT but 556 kB unpacked and stale-stable since 2024-04 ([repo](https://github.com/meyda/meyda)); our whole DSP core measured 2.3 kB — meyda remains the reference for validating MFCC output |
| WebNN / NPU offload | Chromium-only origin trial (M147–149), not production mid-2026 (§3.1) — re-check in 2027 |
| SDF-morphing, matter-js/physics engines, gesture_recognizer.task on phones | Point-set targets + springs cover the need at 0 extra kB; physics engines eat a route budget for effects a flow field gives free |

---

## 7. Voice command tier — pure WebAudio DSP, zero AI (operator-mandated design)

### 7.1 Speaker-dependent isolated-word recognition: MFCC/mel features + DTW template matching

The pre-neural classical recognizer, deliberately: per-frame **mel-band log-energies/MFCC** (Davis & Mermelstein 1980 — [IEEE](https://ieeexplore.ieee.org/document/1163420)) compared against **per-user enrolled templates** by **dynamic time warping** with a Sakoe–Chiba band (Sakoe & Chiba 1978 — [IEEE](https://ieeexplore.ieee.org/document/1163055)); this was the canonical isolated-word/digits architecture of the late-70s/80s (Rabiner et al.'s connected-digit work grew from it). Properties that fit exactly: **deterministic** (same PCM → same distance, VbM-testable), **no training data, no model download**, and **language-independent** — the user enrolls *their own words in their own language/accent*, so **Albanian works as well as English by construction; the sq-support question that kills every native recognizer (§6.3) is moot here.**

- **Mechanics:** mic → `AudioWorkletNode` (inline processor; `ScriptProcessor` fallback) → 16 kHz mono frames (512/256 hop) → Hann + radix-2 FFT + 20-mel filterbank + DCT → 13 MFCCs; energy-gated endpointing (adaptive noise floor + hangover) segments an utterance (0.25–3 s ≈ 8–200 frames); DTW distance to every stored template; accept = best distance under an absolute threshold **and** ≥15 % margin over the runner-up, else reject-quietly.
- **Enrollment:** say each command 2–3× → 2–3 templates stored per command.
- **Verified feasibility/cost:** the complete prototype (FFT, MFCC, DTW, endpointing, template store, clap detector, YIN, worklet plumbing) written this session measures **2,260 B gz** (`/root/.claude/jobs/c6a4c73f/tmp/measure/voice-dsp.js`). CPU: FFT-512 ≈ 4.6k butterflies × 62.5 fps ≈ well under 1 % of a phone core; DTW on a 60×60-frame pair with a bandwidth-8 corridor is ~10⁳–10⁴ distance evaluations per utterance — imperceptible. Building blocks exist if wanted (meyda MIT for reference MFCCs — [meyda.js.org](https://meyda.js.org/); `dynamic-time-warping` npm, MIT, frozen 1.0.0) but the hand-rolled core is smaller than either.
- **Honest accuracy expectations:** speaker-dependent isolated-word DTW on a small vocabulary is the one regime where classical DSP genuinely worked — high-90s % in quiet conditions on ≤10-word vocabularies per the classical literature (order-of-magnitude, condition-dependent: **treat as design target, UNVERIFIED until the P3 mic-in-hand tuning session**). It degrades with: background noise at enrollment≠use time, other speakers (it's speaker-*dependent* — a feature for personal devices, a limitation for shared kiosks), and near-homophones in one vocabulary. **Realistic vocabulary: 5–15 commands/user.** Failure mode is designed to be silence (reject-quietly), never a wrong action.

### 7.2 Non-speech acoustic commands (the robust complement)

- **Claps:** broadband transient detection (RMS rise > 6× a slow-EMA noise floor, 120 ms refractory, 450 ms grouping window) → single/double/triple = three distinct commands. Excellent in noise — a clap's onset slope survives kitchens.
- **Whistles:** deterministic pitch tracking — YIN (cumulative-mean-normalized difference; de Cheveigné & Kawahara 2002 — [JASA/PDF](http://audition.ens.fr/adc/pdf/2002_JASA_YIN.pdf)) or plain autocorrelation on 400–4000 Hz; classify the pitch contour into **up-glide / down-glide / flat** = three more commands. Whistles sit in a narrow band where kitchen clatter is weak.
- **Robustness per context:** owner kitchen (loud, hands dirty) → claps/whistles primary, enrolled words at the pass-station ok · **courier on a moving bike: honest verdict — spoken commands into wind at 25 km/h are impractical**; wind is broadband low-frequency noise that wrecks both endpointing and enrollment match → whistle (band-pass survives wind better) or nothing while moving; enrolled words work stopped at pickup/dropoff · customer at home (quiet) → enrolled words work best there.

### 7.3 Ambient audio-energy reaction (no commands)

`AnalyserNode`/worklet RMS + band energies → particle energy/rhythm modulation. ~1–2 kB class, already inside the measured core (`onLevel`). Pure visualization input; zero recognition; the kiosk breathing with the room's noise level is the cheapest wow in the whole program.

### 7.4 Enrollment UX + storage + the three launch languages

- **Templates live on-device only:** `localStorage`/IndexedDB per device (measured store uses localStorage; ~10–20 kB per command set). **Nothing is uploaded; raw audio is never persisted anywhere** — PCM exists only inside the AudioContext callback. Re-enroll = re-record (3 taps); clear-all provided; templates are per-device by design (a shared kiosk enrolls its owner's voice, or uses claps/whistles which are speaker-independent).
- **sq/uk/en (operator addendum):** the recognizer is language-agnostic — the 3-language requirement lands on the **default suggested vocabularies + enrollment UI strings**, not the DSP. Suggested sets below are chosen for **acoustic distinctness within each set** (different syllable counts, distinct vowel skeletons — near-homophones in one set are the #1 DTW failure). Users may enroll any word they like; these are the suggested defaults:

| Audience / intent | sq | uk | en |
|---|---|---|---|
| **Courier** accept task ⚠ | «pranoj» (2 syl) | «беру» (2) | “accept” |
| picked up | «e mora» (3) | «забрав» (2, brav-coda) | “pickup” |
| delivered | «dorëzova» (4) | «доставив» (3) | “delivered” |
| **Owner** confirm order ⚠ | «konfirmo» (3) | «підтверди» (3) | “confirm” |
| reject ⚠ | «refuzo» (3, fricative onset) | «відхили» (3, distinct vowels) | “reject” |
| ready for pickup | «gati» (2) | «готово» (3) | “done” (avoid “ready”/“reject” re- collision) |
| pause store ⚠ | «pauzë» (2, diphthong) | «пауза» (2) | “pause” |
| **Customer** show status | «statusi» | «статус» | “status” |
| scroll menu | «menuja» | «меню» | “menu” |

- Non-speech commands (claps/whistles) are inherently language-free — instruction text only. i18n note: enrollment/confirm UI strings need al+en per the repo rule **plus Ukrainian as a NET-NEW catalog language** — ~10–15 keys, flag the small addition, not a catalog redesign (the voice matcher in-repo already models sq/en/uk locales, `packages/voice/src/matcher.ts`).

### 7.5 Command semantics, safety, and the shared visual bus

Voice emits the **same intent/event vocabulary** as touch, CV, and server events — one `vocab.ts`, one visual system. Reuse the existing deterministic safety spine from `packages/voice` (council-approved ADR-0015, built 2026-07-01): `IntentProposal` pure data → fail-closed capability table → **ConfirmationGate** — it is transcript-source-agnostic by construction (the `Transcriber` port is just an object with `transcribe()`; memory: `voice-engine-build-2026-07-01.md`), so the DTW matcher slots in where Whisper would have. **Red-line discipline:** every ⚠ command above (mutates money/state — confirm/reject/accept/pause) triggers only a **particle-cloud confirm posture (glyph + amber hold) + an explicit tap** to commit; a spurious clap or false DTW match can therefore *never* mutate state — worst case is a 4-second visual that dissolves. Read-only/ambient reactions (status pulse, menu scroll, energy) are unrestricted. Clap commands map to read-only intents only (double-clap = replay last event pulse; triple = toggle ambient mode).

### 7.6 Platform reality + degradation

`AudioWorklet` is Baseline-supported in all modern engines ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorklet)); iOS Safari requires `AudioContext.resume()` inside a user gesture ([MDN autoplay guide](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Autoplay#the_web_audio_api)) — start() is always tap-invoked anyway (mic permission needs the same tap). Background tabs: worklets keep running but the sim is paused — stop the mic graph on `visibilitychange` to save battery and to be honest about listening (mic indicator discipline). Degradation: no mic permission / no worklet → touch/CV/server-event tiers fully functional; the voice tier is additive everywhere. Privacy line for UI copy: **"processing happens on this device; the microphone stream is analyzed in the page and never recorded or sent anywhere."**

---

## 8. Verification method + unresolved items

- **Measured first-hand this session:** three prototype modules bundled with the repo's esbuild (`--bundle --minify` + `gzip -9`): particles-tf **3,586 B**, voice-dsp **2,260 B**, motion-flow **777 B**; raw-hero re-measured 1,473 B (consistency check with the prior report). MediaPipe: npm tarball 0.10.35 unpacked + per-file gzip; model files via HTTP HEAD Content-Length (7,819,105 / 8,373,440 B). `delegate:"CPU"|"GPU"` verified in shipped `vision.d.ts`. Repo evidence read directly (files cited in-line).
- **Prototype caveat:** the measured modules are representative, compile-verified bundles — **not** run against a live camera/mic/GPU in this sandbox; treat sizes as solid, behavior as designed-not-proven (P1–P3 exit gates prove behavior per the repo's VbM rule).
- **UNVERIFIED left standing:** HandLandmarker browser FPS/init on 2026 mid-range Android (only 2021 Pixel-3-browser ≈20 fps + native Pixel 6 17.12/12.27 ms exist); battery/thermal drain for continuous camera inference (no credible public data); camera-permission denial rates (no published data); TF particle-count ceilings per device (adaptive guard ships instead); DTW accuracy % (classical-literature regime, tune in P3); Safari dictation language list absence of sq (inferred from Apple's published ~30-language sets, not point-verified — moot under DSP-only).
