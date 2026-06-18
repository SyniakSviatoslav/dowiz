# UI/Design Skillset Plan — removing AI-slop from dowiz

**Date:** 2026-06-18 · **Goal:** a curated, mandated skillset that takes the dowiz customer UI from "templated dark AI-slop" to crafted, on-brand, accessible production design. **`taste-skill` and `impeccable` are mandatory on every UI surface.**

---

## 1. The AI-slop problem in dowiz today (grounded in the live dogfood)

The deployed customer UI has the classic AI-slop signature — functional but generic, "designed by default":
- **Flat dark cards** with 160px **empty grey image placeholders** (cards are ~70% dead space).
- **No motion** — no entrances, no micro-interactions, no reduced-motion handling.
- **No visible focus states**; tap targets under 44px (add 32×32, lang links 35×30).
- **Washed-out muted text**, plain typographic hierarchy, no font-pairing intent.
- **Cards-everywhere** layout, sparse on desktop (800px centered, lots of dead margin).
- **Plain-text language switcher**, generic empty/error states ("Error loading menu").
- Menu buried in **e2e test data** (content slop, separate process fix).

This is exactly what `impeccable` and `taste-skill` exist to kill: default aesthetic, low-contrast "elegance," lazy cards, no committed design choices.

---

## 2. Skills already present (inventory by role)

The project is well-stocked. Roles:

| Role | Skill(s) present | Notes |
|------|------------------|-------|
| **Anti-slop design (craft)** | `taste-skill`, `design-taste-frontend`, `frontend-design` | `taste-skill`/`design-taste-frontend` are landing/redesign-focused; overlapping — pick `taste-skill` as the mandated one |
| **Interface-rules compliance** | `web-design-guidelines` | Vercel web-interface-guidelines; already run once this session |
| **Accessibility** | `wcag-accessibility-audit` | WCAG 2.1/2.2 POUR (installed this session) |
| **UX review** | `ui-ux-reviewer` | flow/usability review (installed this session) |
| **Project design system** | `deliveryos-ui`, `deliveryos-theme` | the dowiz tokens/brand — **identity-preservation source of truth** |
| **Build** | `component-builder`, `screen-builder` | component/screen scaffolding |
| **Frontend perf** | `vercel-react-best-practices` | React/Next perf for the `/admin` SPA |
| **Copy de-slop** | `stop-slop` | removes AI tells from UX copy/microcopy |

## 3. What to add

| Add | Why | Status |
|-----|-----|--------|
| **`impeccable` (`pbakaus/impeccable`, 53.8K installs)** | The heavyweight anti-slop engine — `craft`/`shape`/`audit`/`polish` sub-commands, opinionated rules on contrast, typography (line length, font-pairing, letter-spacing floors), "cards are the lazy answer," semantic z-index, mandatory reduced-motion, OKLCH palettes. Covers what the lighter taste skills only gesture at. | ✅ **installed** (mandated) |
| _(optional)_ `dylantarre/animation-principles@micro-interactions` (704) | A dedicated micro-interaction pack — **but `impeccable` already covers motion deeply**, so add only if motion becomes a dedicated workstream. | deferred |
| _(skip)_ design-system/token skills | Low-install, and **`deliveryos-theme`/`deliveryos-ui` already own the project's tokens** — identity-preservation wins. | skip |

**Verdict:** the bench is complete with `impeccable` + the existing skills. No further installs needed; adding more would be slop in itself.

---

## 4. Mandated anti-slop workflow (how the skills compose)

Every UI surface goes through this pipeline. **`taste-skill` + `impeccable` are required gates; the rest are supporting passes.**

**Phase A — Context (prerequisite, once).**
`impeccable` requires a root **`PRODUCT.md`** (it runs `scripts/context.mjs`; reports `NO_PRODUCT_MD` and stops otherwise). The project has `docs/design/DESIGN.md` but **no `PRODUCT.md`** → create one (register = `product` for app UI / `brand` for the public menu storefront). This is the single blocking prerequisite. → `impeccable` `reference/init.md`.

**Phase B — Audit (diagnose the slop).**
Run in parallel against the target surface:
- **`impeccable audit`** — the primary craft/anti-pattern audit.
- **`web-design-guidelines`** — interface-rule violations (this session's run is the first cut).
- **`wcag-accessibility-audit`** — POUR/contrast/focus/targets.
- **`ui-ux-reviewer`** — flow & cognitive-load.

**Phase C — Redesign (commit design choices).**
- **`impeccable craft`/`shape`** drives the redesign (live browser iteration, screenshots).
- **`taste-skill`** as the anti-slop read/gate — "read the room," kill default aesthetic.
- **`deliveryos-theme`/`deliveryos-ui`** — preserve brand identity; extend tokens, don't reinvent (OKLCH, existing `--brand-*` vars).

**Phase D — Build & harden.**
- **`component-builder` / `screen-builder`** — scaffold the new components/screens.
- **`vercel-react-best-practices`** — perf pass for the `/admin` React SPA.
- **`impeccable polish`** — final craft pass.

**Phase E — Copy.**
- **`stop-slop`** on all UX copy/microcopy/empty-error states.

**Exit gate:** no surface ships until `impeccable` + `taste-skill` sign off, with Playwright/screenshot proof (the project's Mandatory Proof Rule).

---

## 5. Apply to dowiz surfaces (priority order)

1. **Public menu (`/s/:slug`)** — the storefront; `register: brand`. Kill the empty-image slop (collapse media or branded fallback), real card design, motion on add-to-cart + FAB, 44px targets, focus rings, typographic hierarchy. → `impeccable craft` + `taste-skill` + `deliveryos-theme`.
2. **Cart/checkout (`/s/:slug/checkout`)** — now renders (Phase 0 fix); `register: product`. Polish the cart view, totals, the order form (labels/`autocomplete`/`inputmode`), empty/error states. → `impeccable polish` + `wcag` + `stop-slop`.
3. **Admin SPA (`/admin`)** — `register: product`; the high-churn `MenuManagerPage`/`CheckoutPage`. → `impeccable audit` + `vercel-react-best-practices`.

---

## 6. Prerequisites & next step
- **Blocking:** author a root **`PRODUCT.md`** (register + product/brand context) so `impeccable` runs. `docs/design/DESIGN.md` already exists and feeds the design context.
- **Then:** start Phase B (audit) on the public menu — the highest-visibility slop.

**Recommended kickoff:** create `PRODUCT.md`, then run `impeccable audit` + `taste-skill` on `/s/:slug` and bring back a concrete redesign proposal before writing code.
