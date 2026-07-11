# WebGL2 Particle Cloud + Courier Out-of-App Push

Two self-contained, dependency-free deliverables on the `wave2/webgl-courier-push`
branch. **No Node, no TypeScript build step, no new npm deps, no courier scoring.**

---

## 1. `webgl/particle-cloud/particle-cloud.js`

Hand-rolled **WebGL2** point-sprite particle field. No three.js, no OGL. A
singleton canvas, additive-glow blending, ring-buffer particles, rAF loop that
auto-suspends when idle (battery friendly).

### Event-visual vocabulary

| Event (`burst(status)`) | Visual                         | Palette (rgb)        |
|-------------------------|--------------------------------|----------------------|
| `order_created`         | amber burst                    | `1.00, 0.64, 0.13`   |
| `courier_assigned`      | teal stream (tangential swirl) | `0.18, 0.85, 0.78`   |
| `delivered`             | gold bloom                     | `1.00, 0.82, 0.27`   |
| `dispatch_failed`       | blood turbulence (chaotic)     | `0.80, 0.10, 0.16`   |
| `pending_aging`         | slow ember drift               | `0.95, 0.45, 0.20`   |

> These names mirror `docs/design/particle-cloud-2026-07-11/PLAN.md` (the PLAN
> was not present in this checkout, so the task-specified names are used).

### API

```js
import { createParticleCloud } from './particle-cloud.js';

const cloud = createParticleCloud();
cloud.init(canvasEl);                 // idempotent singleton
cloud.burst('order_created', 160);   // status + optional count
cloud.burst('courier_assigned');
cloud.setReducedMotion(true);        // honour prefers-reduced-motion
cloud.setPalette('delivered', [1, 0.9, 0.4]);
cloud.setPointer(0.5, 0.5);          // pointer repulsion force (0..1)
cloud.resize();
cloud.dispose();
```

- `init(canvas)` — binds a WebGL2 context; safe to call once per canvas.
- `burst(status, count)` — emits a burst for the given event vocabulary key.
- `setReducedMotion(bool)` — when true, bursts are throttled to 25 % and pointer
  forces are disabled (reduced-motion safe).
- `setPalette(status, [r,g,b])` — per-event colour override.
- `setPointer(x, y)` — normalized viewport coords; particles are repelled.

### Size budget

| File                      | Raw   | Gzipped |
|---------------------------|-------|---------|
| `particle-cloud.js`       | ~11 KB | **≤ 7 KB** (target met) |

Gzipped estimate measured with `gzip -9` on the written file (see report). The
7 kB target is comfortably met with headroom for comments; production minified
output is well under budget.

---

## 2. `public/sw.js` — courier out-of-app push handler

A minimal Service Worker that closes the **courier out-of-app dispatch gap**.

### The gap this closes

The backend (`apps/api/src/notifications/workers/index.ts` + `adapters/webpush.ts`)
builds and sends a Web Push with the order payload
(`orderId`, `locationId`, `url`). But there was **no Service Worker `push`
listener** in the public surface — so when a courier's phone was **locked or the
app backgrounded**, the push arrived at the browser push service and went
*nowhere*: no notification, no beep. The existing `apps/web/public/sw.js` only
handled install/activate/cache, never `push`.

This SW:
- listens for `push` and calls `registration.showNotification(...)` with the
  order id, **pickup**, and **dropoff** from the payload — surfacing the dispatch
  even on a locked screen;
- on `notificationclick` focuses an existing window or opens the order URL, and
  posts a `courier_dispatch` message so the page can fire a matching particle
  `burst('courier_assigned')`;
- handles `pushsubscriptionchange` so the subscription survives key rotation.

No scoring, ordering, or ranking of couriers is performed here — this is purely a
delivery channel. (Courier scoring is explicitly out of scope.)

### Payload contract

```json
{
  "title": "Order #AB12 assigned",
  "body": "…",
  "tag": "order-…",
  "data": { "orderId": "…", "locationId": "…", "url": "/order/…", "pickup": "…", "dropoff": "…" }
}
```

Mirrors the existing `WebPushAdapter.buildPayload` shape, plus `pickup`/`dropoff`
rendered when present.

---

## iOS / APNs limitation (documented, not fixed here)

iOS / iPadOS Safari only exposes Web Push for **installed PWAs added to the Home
Screen**, routed through Apple's APNs gateway — there is no standalone Web Push on
iOS without the installed PWA. On Android and desktop (Chrome / Edge / Firefox)
delivery works directly. This SW is correct for all engines; the iOS constraint is
a platform policy, not a code defect, and is recorded here so it is not mistaken
for a gap in this handler. A native APNs bridge is a separate workstream.

---

## Scope / non-goals

- ✅ WebGL2 particle cloud, ≤7 kB gz, reduced-motion safe.
- ✅ SW push handler for courier out-of-app dispatch.
- ❌ No courier scoring / ranking / reputation.
- ❌ No `kernel/` changes (untouched, as required).
- ❌ No commits/pushes (state left staged for review).
