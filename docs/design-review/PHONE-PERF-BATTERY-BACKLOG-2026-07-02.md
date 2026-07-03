# Phone-first perf / battery / usability backlog (2026-07-02)

Real-world levers for dowiz given the scope (food-delivery web app, **mostly phone**, mobile data,
courier battery during delivery). Ranked by **usability impact** first; **weirdness** = how
unusual/experimental/support-risky the technique is. Grounded in the current code, not generic advice.

Scores: Usability + Weirdness on 1–5. Effort S/M/L.

## Findings baseline (what's already here)
- ✅ `prefers-reduced-motion` well covered (128 refs) · ✅ `loading="lazy"` on images · ✅ WS hook handles
  `visibilitychange` (`useWebSocket.ts:140`) · ✅ MediaGallery neighbour prefetch.
- ⚠️ Menu images = single 800×800 raster served **through the Node API** (`spa-proxy.ts:206`, `/images/*`),
  no responsive/AVIF/CDN. ⚠️ Courier GPS = `enableHighAccuracy:true` + `maximumAge:0` (`DeliveryPage.tsx:96`).
  ⚠️ Service worker = naive cache-first-forever, `/api` excluded (`apps/api/public/sw.js`).

---

## Tier 1 — Boring, highest-impact (do first). Usability 5 · Weirdness 1
| # | Lever | Usab | Weird | Effort | Grounding / what to do |
|---|-------|------|-------|--------|------------------------|
| 1 | **Responsive image pipeline** | 5 | 1 | M | Today one 800px raster serves both a 96px thumbnail and full view, through the Node API. Generate width variants (e.g. 160/400/800) + **AVIF/WebP** on upload (`sharp` already in `spa-proxy`), emit `srcset`+`sizes`, serve from an **edge/CDN (finish the R2 move)** not the origin API. Biggest single win for data + speed + battery on the storefront. |
| 2 | **Adaptive courier GPS** | 5 | 2 | M | `enableHighAccuracy:true` + `maximumAge:0` is the max-drain combo. Allow cached fixes (`maximumAge` 5–15s), drop high-accuracy when stationary, throttle server posts by distance/interval (send on ~25m move or ~10s), coalesce. Cuts courier battery + upload volume materially with no UX loss. |
| 3 | **Real service-worker strategy** | 4 | 2 | M | Replace cache-first-forever with: precache the shell (versioned), **stale-while-revalidate for the public menu** (so `/s/:slug` opens instantly and offline-ish), navigation preload, and *don't* blanket-exclude `/api`. Fixes stale-asset bugs + makes repeat menu opens near-instant on mobile. |
| 4 | **Ship less JS to phones** | 4 | 1 | S–M | Route-level code-splitting for admin/courier (customers shouldn't download admin bundles) + **`LazyMotion`+`m`** for the 33 `framer-motion` imports (~46KB→~4.6KB initial). Enforce with the existing `.size-limit.json` gate. |

## Tier 2 — Standard, high usability. Usability 4 · Weirdness 1–2
| # | Lever | Usab | Weird | Effort | Grounding / what to do |
|---|-------|------|-------|--------|------------------------|
| 5 | **`content-visibility:auto` + list virtualization** | 4 | 2 | S–M | Long menus / order lists render + paint every row. `content-visibility:auto` on off-screen menu sections is a near-free paint win; virtualize the admin order list. Cheap phones feel it most. |
| 6 | **Web Push for order status** | 4 | 2 | M | `VAPID_PRIVATE_KEY` exists → extend push so customers get "order accepted / courier assigned / arriving" **without keeping the tab + WS open**. Closing the socket when backgrounded is a direct battery + data saver. |
| 7 | **Optimistic UI + skeletons on cart/checkout** | 4 | 1 | S | Perceived speed: reflect add-to-cart / place-order instantly, reconcile on server truth. Pairs with the `loading-states` skill. |
| 8 | **Preconnect + font subset** | 3 | 1 | S | `preconnect`/`dns-prefetch` to the image origin; subset the per-tenant Google fonts (egress-safe loader already exists) to the glyphs actually used — cuts first-paint KB. |

## Tier 3 — Modern web APIs, moderately weird, good ROI. Usability 3–4 · Weirdness 3
| # | Lever | Usab | Weird | Effort | Grounding / what to do |
|---|-------|------|-------|--------|------------------------|
| 9 | **Background Sync for order submit** | 4 | 3 | M | Queue the place-order POST in the SW and flush on reconnect → orders survive a dropped mobile connection at checkout instead of failing. High reliability value for flaky networks. |
| 10 | **View Transitions API** | 3 | 3 | S | Native cross-document/route transitions (menu→item→checkout) that feel app-native and are cheaper than JS animating layout. Progressive-enhance (fallback = instant nav). |
| 11 | **Speculation Rules (prerender)** | 3 | 3 | S | Prerender/prefetch the likely next step (checkout from menu, order-status after placing) so it opens instantly. Guard with Save-Data/data-saver. |
| 12 | **Wake Lock for courier navigation** | 4 | 2 | S | Keep the screen awake during an active delivery so the courier doesn't lose the map mid-ride. Usability win; scope tightly to active-delivery to bound the battery cost. |
| 13 | **Vibration API haptics** | 3 | 2 | S | Cheap phone-native feel: subtle haptic on add-to-cart / order-confirmed / courier "arrived". Gated behind a user setting. |

## Tier 4 — Adaptive / niche. Usability 3 · Weirdness 3–4
| # | Lever | Usab | Weird | Effort | Grounding / what to do |
|---|-------|------|-------|--------|------------------------|
| 14 | **Network Information API adaptive loading** | 3 | 3 | S | Respect `Save-Data` + `effectiveType`: on 2G/3G/data-saver serve smaller images, skip the landing WebGL, defer non-critical fetches. Realistic for the target market's connectivity. |
| 15 | **Web Share / Share Target** | 3 | 2 | S | Native share of a storefront/menu link; optionally register as a share target. Growth + phone-native. |
| 16 | **Local-first menu mirror (IndexedDB)** | 3 | 4 | M | Cache the last-seen menu in IndexedDB so a returning customer sees it instantly and offline, revalidate in background. Overlaps #3 — do only if SWR isn't enough. |

## Tier 5 — Weird / experimental / low ROI (know they exist; mostly don't build). Usability 1–2 · Weirdness 5
| # | Lever | Usab | Weird | Effort | Verdict |
|---|-------|------|-------|--------|---------|
| 17 | **On-device voice control (in build)** | 2 | 4 | L | `packages/voice` whisper is CPU/battery-heavy. If launched, gate inference to Wi-Fi/charging and keep it opt-in; never auto-run on mobile data. Flag, not a perf *win*. |
| 18 | **WebGPU landing scene** | 2 | 5 | M | Marginal over the existing three.js/WebGL; support still uneven on the phones we target. Skip unless the landing becomes GPU-bound. |
| 19 | **Periodic Background Sync** | 2 | 5 | M | Chrome-only, needs an installed PWA + engagement heuristics. Fragile; not worth it for menu freshness that SWR already covers. |
| 20 | **Contact Picker API** | 2 | 4 | S | Autofill phone at checkout — Chrome-Android only, permission-gated. Minor convenience, narrow support. |
| 21 | **Battery Status API degradation** | 1 | 5 | S | Tempting ("dim animations on low battery") but the API is removed/deprecated in most browsers + privacy-flagged. **Don't build** — use `prefers-reduced-motion` + Save-Data instead. |

---

## If you do nothing else
**#1 (responsive images off the origin) + #2 (adaptive GPS) + #3 (real SW)** are the three that move real
numbers for real users on real phones — data cost, courier battery, and repeat-open speed. Everything in
Tier 3+ is polish or resilience on top. All perf work must clear the existing `.size-limit.json`,
reduced-motion, and `no-arbitrary-tailwind` gates, and follow Ship Discipline (staging + proof) before prod.
