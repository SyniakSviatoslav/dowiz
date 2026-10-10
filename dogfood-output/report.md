# Dogfood Report — <legacy-host>

**Target:** https://<legacy-host> (live)
**Date:** 2026-06-18
**Method:** agent-browser exploratory QA (owner login, /admin, public menu /s/demo) + curl SSR inspection
**Note:** Tests the **deployed** build; the in-progress local security fixes are NOT deployed.

## Summary

| # | Severity | Area | Issue |
|---|----------|------|-------|
| 1 | Medium | Auth / routing | `/admin` renders the full owner dashboard shell to unauthenticated users (no route guard) |
| 2 | Medium | SSR / SEO | Venue name double HTML-entity encoded in all meta/OG/Twitter tags |
| 3 | Low | i18n | Login error "Login failed." hardcoded English on Albanian UI |
| 4 | Low | SSR / i18n | City name mis-cased: "Durrës" → "DurrëS" in meta description |
| 5 | Info | SEO / AEO | No JSON-LD structured data on the public menu page |

Console/JS errors: none observed on login or menu pages. No PII/data leaks found (protected APIs correctly return 401).

---

## Issue 1 — `/admin` renders without authentication (Medium)

**What:** Navigating directly to `https://<legacy-host>/admin` with no token renders the complete owner dashboard shell — sidebar (Paneli, Porosite, Menu, Furnizimet, Promocionet, Postieret, Analitika, Klientet, Brandingu, Cilesimet), the "Porosite Live" panel, order filters, CSV export, and search.

**Impact:** No data leak — `/api/owner/orders`, `/api/owner/settings`, `/api/owner/couriers` all return **401**, so the panels are empty. But the protected admin UI and full feature surface are exposed to anonymous visitors, and a logged-out user sees a broken empty dashboard instead of being redirected to `/login`. The app needs a client-side auth guard on `/admin/*` that redirects unauthenticated users to `/login`.

**Repro:**
1. Clear storage / open a fresh session.
2. Navigate to `https://<legacy-host>/admin`.
3. Observe: URL stays `/admin`, `localStorage.access_token` is null, full dashboard shell renders.

**Evidence:** `screenshots/02-admin-direct-noauth.png`, network shows `/api/owner/*` → 401.

---

## Issue 2 — Venue name double HTML-entity encoded in SSR head (Medium)

**What:** The venue "Dubin & Sushi" is HTML-escaped twice (`&` → `&amp;` → `&amp;amp;`) in the server-rendered head. 10 occurrences across `<title>`, `og:title`, `og:description`, `twitter:title`, and `meta[name=description]`.

```
<title>Dubin &amp;amp; Sushi — Order Online | Dowiz</title>
<meta property="og:title" content="Dubin &amp;amp; Sushi — Order Online | Dowiz"/>
<meta name="description" content="Order delivery from Dubin &amp;amp; Sushi at ..."/>
```

**Impact:** Browser tab and social-share cards display the literal `Dubin &amp; Sushi`. SEO/AEO degradation (the app explicitly targets SEO/AEO). Likely a meta-builder escaping an already-escaped value — fix by escaping exactly once.

**Repro:** `curl -s https://<legacy-host>/s/demo | grep -o '&amp;amp;' | wc -l` → 10.

---

## Issue 3 — Login error message not localized (Low)

**What:** Submitting invalid credentials on the Albanian (SQ) login page shows an `alert`-role message **"Login failed."** in English, while the rest of the page is Albanian ("Hyr si Pronar", "Fjalëkalimi"). `i18n.ts` is a known high-churn hotspot.

**Repro:** `/login` → fill invalid email + password → click "Hyr" → English "Login failed." appears.
**Evidence:** body text capture shows `Hyr si Pronar / Login failed. / Fjalëkalimi`.

---

## Issue 4 — City name mis-cased in SSR meta (Low)

**What:** The address city "Durrës" is rendered as **"DurrëS"** (final character uppercased) in the meta description: `...Rruga Sulejman Kadiu, DurrëS.` Suggests a capitalize/title-case transform mishandling the trailing character (possibly the `ë` multibyte boundary).

**Repro:** `curl -s https://<legacy-host>/s/demo | grep -o 'Kadiu, [A-Za-zë]*'` → `Kadiu, DurrëS`.

---

## Issue 5 — No JSON-LD on public menu page (Info)

**What:** The public menu (`/s/demo`) ships meta/OG tags but **zero** `application/ld+json` blocks, despite a `jsonld-builder` in the codebase and a stated SEO/AEO focus. Restaurant/Menu structured data would improve rich results. (May be injected client-side elsewhere — worth confirming.)

---

## Not reproduced (logged for honesty)

- During initial orientation, an invalid-credential submit appeared to land on `/admin` once. On three clean retries it correctly stayed on `/login` with no token and an error message. Treating as transient SPA state, **not** a confirmed auth bypass. The underlying exposure is covered by Issue 1.

## Environment limitations

- Video recording unavailable (`ffmpeg` not installed) — evidence is screenshots + curl output.
- Owner-authenticated areas not deeply explored (no live Google session; deployed dev-login creds not used).
