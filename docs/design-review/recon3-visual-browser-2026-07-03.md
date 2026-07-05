# Recon 3 — Visual Browser Recon (LIVE prod)

- **Date:** 2026-07-03
- **Target:** `https://dowiz.fly.dev` (prod — pre-audit-fix image)
- **Tool:** playwright-test MCP (real Chromium). Browser tooling **WORKED**. All evidence = real screenshots + console + computed-style measurements.
- **Viewports:** desktop 1280×800, mobile 390×844.
- **Constraint honored:** read-only. Opened product / added to cart (client-side only) / opened checkout form. **No order placed, no payment, no login with real creds, no data mutated.**

## Pages covered
| Route | Desktop | Mobile | Notes |
|---|---|---|---|
| `/s/demo` (Dubin & Sushi, orange brand) | ✓ | ✓ | full menu; product modal → cart → checkout traced |
| `/s/artepasta` | ✓ | (same not-found) | slug does not exist on prod (staging-only); not-found state |
| `/start` (landing, `/` redirects here) | ✓ | ✓ | onboarding / menu upload |
| `/admin/login` (redirects to `/login`) | ✓ | ✓ | owner login |

`/s/dubin-sushi` / `/s/sushi-durres` are the same brand as `/s/demo`, so not re-shot.

---

## Findings

### F1 — [MEDIUM] Storefront hero media 404 on every load
- **Page/viewport:** `/s/demo` (and `/s/sushi-durres`), desktop + mobile.
- **Defect:** Two hard 404s on every storefront load:
  - `GET /media/037d46db-e143-4b9a-bc39-1f31fa1804e0/hero/cover.webp` → 404
  - `GET /media/037d46db-e143-4b9a-bc39-1f31fa1804e0/hero/video.mp4` → 404
  The hero falls back to a generic dark location-map with a pin. Not a crash (graceful fallback), but the *intended* hero cover/video is missing and it throws 2 console errors on 100% of loads. A human reads the top of the page as a plain map, not a branded hero.
- **Evidence:** `demo-desktop.png`, `demo-mobile.png`, console (2 errors).

### F2 — [MEDIUM] Add-to-Cart modal price label fails WCAG AA contrast
- **Page/viewport:** `/s/demo` product detail modal, desktop.
- **Defect:** On the single orange CTA `Shto në Shportë  1400 ALL`, the two labels are colored inconsistently:
  - `"Shto në Shportë"` = near-black `rgb(15,17,23)` on orange `rgb(234,79,22)` → **5.06:1 (pass)**
  - `"1400 ALL"` (price) = near-white `rgb(226,232,240)` on the same orange → **3.03:1 (FAIL AA 4.5:1 for normal text)**
  White-on-orange for the price is below AA; the mixed light/dark text on one button also reads as inconsistent.
- **Evidence:** `demo-product-detail.png` + computed-style measurement.

### F3 — [MEDIUM] Telegram SSO button fails WCAG AA contrast
- **Page/viewport:** `/login`, desktop + mobile.
- **Defect:** `Vazhdo me Telegram` = white text `rgb(255,255,255)` on Telegram-blue `rgb(34,158,217)`, 16px / weight 400 → **3.02:1 (FAIL AA 4.5:1 for normal text)**.
- **Evidence:** `login-mobile.png`, `login-desktop.png` + measurement.

### F4 — [MEDIUM] PWA install banner shows on DESKTOP with iOS-only instructions, overlapping menu
- **Page/viewport:** `/s/demo`, desktop 1280×800 (Chromium).
- **Defect:** An `Instalo dowiz` install banner (dialog) renders on a desktop browser, floating over the bottom of the menu and covering ~2–3 product cards. Its body text is iOS-Safari-specific: *"Trokit Share, pastaj 'Add to Home Screen'"* (tap Share → Add to Home Screen) — wrong platform for a desktop user. Should be mobile/PWA-capable only, and platform-appropriate copy.
- **Evidence:** `demo-desktop.png` (banner over cards).

### F5 — [LOW-MEDIUM] Landing `/start` not vertically centered on desktop
- **Page/viewport:** `/start`, desktop.
- **Defect:** The onboarding card is horizontally centered but top-anchored; the lower ~60% of the viewport is empty. Looks unfinished/unbalanced on wide screens. Mobile is fine (content naturally fills width).
- **Evidence:** `landing-desktop.png` vs `landing-mobile.png`.

### F6 — [LOW] Console noise on checkout: WebGL GPU stall + null-number warnings
- **Page/viewport:** `/s/demo` → checkout, desktop.
- **Defect:** Opening checkout emits `GL Driver Message ... GPU stall due to ReadPixels (High)` (WebGL perf) and **6×** `Expected value to be of type number, but found null instead` from a blob worker (likely the map component receiving null coordinates). Not user-visible, but console noise + a potential perf/logic smell in the map integration.
- **Evidence:** console (12 messages: 2 errors + 10 warnings).

### F7 — [LOW] `/s/artepasta` renders storefront chrome above the not-found state
- **Page/viewport:** `/s/artepasta`, desktop.
- **Defect:** Slug does not exist on prod (`/public/locations/artepasta/menu`, `/api/public/theme/artepasta`, `/public/locations/artepasta/info` all 404). The not-found empty state (`Restoranti nuk u gjet` + `Kthehu në fillim` CTA) is well-designed, BUT the page still renders the map hero, currency switcher, language switcher, and an empty `Të gjitha (0)` category tab above it. A cleaner 404 would suppress the storefront chrome.
- **Evidence:** `artepasta-desktop.png`, console (4×404).

### F8 — [INFO] Generic `<title>` on several routes
- `/s/artepasta`, `/login`, `/start` all have `<title>Dowiz</title>` (no descriptive per-page title). `/s/demo` correctly uses `Dubin & Sushi — Order Online | Dowiz`. Minor SEO/tab-clarity.

---

## Verified NOT defects (checked and cleared)
- **Icon glyphs:** the ₩-looking marks next to cooking-time / currency are correctly-rendered `tabler-icons` glyphs (font loaded); the ₩ look was small-icon downscaling in the full-page screenshot. Zoomed currency button shows a proper coin icon.
- **Category-chip scroll-jump:** works. Clicking a chip scrolls the inner `main` panel (window.scrollY stays 0) and scroll-spy sets the active chip `aria-pressed=true`. Confirmed with a trusted click (`Nigiri & Sashimi` → its heading jumps to top).
- **Checkout CTAs `Porosit`:** dark-on-orange 5.06:1 (pass AA). Free-delivery hint 5.72:1 (pass).
- **Login `Hyr` amber button:** ~5.3:1 (pass AA).
- **Cart persistence:** item survived reload (localStorage). Toast + floating cart bar work.
- **No JS crashes / blank error screens** on any page; mobile layouts (storefront, `/start`, `/login`) are clean and well-proportioned.

## Severity counts
- HIGH: 0
- MEDIUM: 4 (F1, F2, F3, F4)
- LOW/LOW-MED: 3 (F5, F6, F7)
- INFO: 1 (F8)

## Worst user-facing visual defect actually SEEN
The **PWA "Instalo dowiz" install banner on the DESKTOP browser** (F4): it overlays the bottom product cards and tells a desktop user to *"tap Share → Add to Home Screen"* (iOS-only). It is unambiguously wrong-context and obscures menu content. Runner-up: the storefront hero throwing two 404s and rendering a plain map instead of branded hero media (F1).
