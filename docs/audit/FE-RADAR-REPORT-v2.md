# FE-RADAR-REPORT-v2.md — a11y + Throttled-Network Findings

> Generated: 2026-06-12 · Target: `dowiz.fly.dev` (staging)
> Method: Playwright × 7 surfaces × 2 viewports (390/1280) × 2 net profiles (fast/slow-3g) = 84 tests
> Added: axe-core a11y scan, touch-target audit, form-label audit, aria-live check, throttled-network loading check

---

## Executive Summary

| Metric | Count |
|---|---|
| **Tests run** | 84 |
| **✅ Passed** | 84 |
| **🔴 Issues (total)** | **19** |
| **🔴 Critical** | 5 |
| **🟠 Serious** | 12 |
| **🟡 Moderate** | 2 |
| **Surfaces with issues** | 7/7 |

---

## 🔴 Critical Issues

### C1. Admin settings: 17 form elements lack labels
| | |
|---|---|
| **Surface** | `admin-settings` (mobile) |
| **Rule** | `axe-label` — every form element must have a label |
| **Count** | 17 violations |
| **Impact** | Screen reader users cannot identify form fields. All settings inputs (name, phone, address, fee, radius, hours) are unlabeled. |
| **Evidence** | Axe violation: `Ensure every form element has a label` |
| **Severity** | 🔴 — blocks settings form usage for assistive tech |
| **Hypothesis** | Settings form renders inputs without `<label>` tags or `aria-label` attributes. Fields are likely styled as placeholder-only. |

### C2. Admin dashboard: button without accessible name
| | |
|---|---|
| **Surface** | `admin-dashboard` (mobile) |
| **Rule** | `axe-button-name` |
| **Count** | 1 violation |
| **Impact** | Button has no text/label — screen reader cannot announce it |
| **Evidence** | Axe violation |
| **Severity** | 🔴 — action button invisible to assistive tech |
| **Hypothesis** | Icon-only button missing `aria-label` (e.g., search, filter, or export button with only an icon) |

### C3. Admin dashboard: select without accessible name
| | |
|---|---|
| **Surface** | `admin-dashboard` (mobile) |
| **Rule** | `axe-select-name` |
| **Count** | 1 violation |
| **Impact** | Sort/filter dropdown has no accessible name |
| **Evidence** | Axe violation |
| **Severity** | 🔴 — sort/filter control invisible to screen readers |
| **Hypothesis** | Status filter `<select>` or sort dropdown missing associated label |

### C4. Menu: 88 ARIA role parent violations
| | |
|---|---|
| **Surface** | `menu` (`/s/demo`, mobile) |
| **Rule** | `aria-required-parent` |
| **Count** | 88 violations |
| **Impact** | ARIA roles like `tab`, `listitem`, `option` used without required parent roles |
| **Evidence** | Axe violation |
| **Severity** | 🔴 — screen readers may not correctly navigate product/category lists |
| **Hypothesis** | Category tabs or product cards use ARIA `tab`/`listitem` roles but are not wrapped in `role="tablist"` or `role="list"` containers |

### C5. Menu: 110 touch targets below 44px on mobile 390
| | |
|---|---|
| **Surface** | `menu` (mobile 390) |
| **Rule** | Manual touch-target audit |
| **Count** | 110 elements |
| **Impact** | Small buttons/tabs are hard to tap accurately on mobile — critical for couriers in motion |
| **Evidence** | `checkTouchTargets()`: 110 elements with `width < 44px || height < 44px` |
| **Severity** | 🔴 — WCAG 2.5.5 Target Size failure, critical for mobile users |
| **Hypothesis** | Product quantity +/- buttons, category tabs, and filter chips are smaller than 44px minimum touch target |

---

## 🟠 Serious Issues

### S1. Color contrast — 35 violations across 6 surfaces
| Surface | Count |
|---|---|
| menu | 28 |
| admin-dashboard | 3 |
| checkout | 1 |
| order-status | 1 |
| admin-login | 1 |
| courier-login | 1 |
| **Total** | **35** |

**Hypothesis:** Brand colors on certain backgrounds fail WCAG AA contrast ratio (4.5:1 for text, 3:1 for large text). Likely `--brand-primary` on light backgrounds, or muted text (`--brand-text-muted`) on `--brand-surface`.

### S2. Menu: no loading indicator on slow-3g (mobile + desktop)
| | |
|---|---|
| **Surface** | `menu` (`/s/demo`) |
| **Profile** | slow-3g (400ms delay per request) |
| **Expected** | Skeleton/spinner visible while menu data loads |
| **Actual** | No loading indicator found (page shows blank/empty while waiting for API) |
| **Evidence** | `[class*="spinner"], [class*="skeleton"], [class*="loading"]` count = 0 on both mobile and desktop |
| **Hypothesis** | MenuPage renders immediately without checking if data has loaded. No suspense/loading boundary around the fetch call. |

### S3. Admin login: 2 inputs without labels
| | |
|---|---|
| **Surface** | `admin-login` (mobile) |
| **Evidence** | `Missing label for input: owner@restaurant.com` (email field) and password field (placeholder text used as visual label but no programmatic association) |
| **Severity** | 🟠 — affects all users with screen readers |

### S4. Admin login: 4 touch targets too small
| | |
|---|---|
| **Surface** | `admin-login` (mobile 390) |
| **Count** | 4 elements below 44px |
| **Evidence** | `checkTouchTargets()` |
| **Severity** | 🟠 — "Sign in with Google" link and/or other small controls |

### S5. Checkout: link without discernible text
| | |
|---|---|
| **Surface** | `checkout` (mobile) |
| **Rule** | `axe-link-name` |
| **Count** | 1 violation |
| **Severity** | 🟠 — link has no accessible name |

---

## 🟡 Moderate Issues

### M1. Missing aria-live regions (2 surfaces)
| Surface | Issue |
|---|---|
| `order-status` | No `aria-live` or `role="alert"/"status"` — status changes not announced |
| `admin-dashboard` | No `aria-live` — new orders not announced to screen readers |

**Severity:** 🟡 — dynamic content updates are invisible to assistive tech. Users won't know when order status changes or new orders arrive without manual refresh.

---

## Cluster by Root Cause

| Cluster | Issues | Surfaces affected | Shared cause | Estimate |
|---|---|---|---|---|
| **Form labels** | C1, S3, S5 | admin-settings(17), admin-login(2), checkout(1) | `<input>` elements without `<label>` or `aria-label` | 2-3h |
| **Color contrast** | S1 | menu(28), dashboard(3), checkout, order-status, login, courier-login | CSS variable colors fail WCAG AA ratio | 2-3h |
| **Touch targets** | C5, S4 | menu(110), admin-login(4) | Icons, chips, quantity buttons below 44px | 1-2h |
| **ARIA structure** | C4 | menu(88) | List/tab roles missing parent containers | 1h |
| **Loading states** | S2 | menu (mobile + desktop) | No skeleton/spinner during slow data fetch | 1-2h |
| **Button names** | C2, C3 | admin-dashboard | Icon-only buttons and selects missing accessible names | 1h |
| **Live regions** | M1 | order-status, admin-dashboard | No aria-live for dynamic updates | 1h |

---

## Backlog (ordered severity → effort)

1. **🔴 Fix 17 unlabeled form inputs** in admin settings — add `<label>` or `aria-label` to each field
2. **🔴 Fix 88 ARIA role parent violations** on menu — wrap category tabs in `role="tablist"`, product lists in `role="list"`
3. **🔴 Fix 110 touch targets < 44px** — increase icon/button/chip min size on mobile
4. 🔴 **Add accessible names** to dashboard buttons and selects
5. 🟠 **Fix color contrast** across 6 surfaces — adjust brand/muted colors to meet WCAG AA
6. 🟠 **Add loading skeleton** to menu page during data fetch
7. 🟡 **Add aria-live regions** to order-status and admin-dashboard for dynamic updates
