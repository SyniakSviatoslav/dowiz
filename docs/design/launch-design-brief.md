# dowiz Launch Design Brief — taste orchestration (v1)

> The bar for v1 launch UI is **stunning, anti-AI-slop, impeccable**. Technical correctness is the floor.
> FE/designer agents: read this first, then pull only the skills that fit the surface you're building.
> (To promote this into a first-class `dowiz-launch-design` skill under `.claude/skills/`, approve the protect-paths gate — the content is ready.)

## Vendored design suite (from Owl-Listener/designer-skills, now in `.claude/skills/`)

Interaction: `interfaces-that-feel` · `animation-principles` · `micro-interaction-spec` · `loading-states` · `feedback-patterns` · `onboarding-design` · `error-handling-ux`
UI: `aesthetic-usability` · `visual-hierarchy` · `typography-scale` · `color-system` · `spacing-system` · `layout-grid` · `illustration-style` · `responsive-design`
Critique (anti-slop gate): `critique-visual-hierarchy` · `critique-brand-consistency` · `critique-composition` · `critique-typography` · `critique-color` · `critique-affordance` · `critique-information-density` → run all 7 via the `/critique-screen` command.
Design systems: `motion-system` · `theming-system` · `accessibility-audit`
Repo's own (compose freely): `impeccable` · `taste-skill` · `design-taste-frontend` · `frontend-design` · `stop-slop` · `web-design-guidelines` · `wcag-accessibility-audit`.

## Non-negotiable project constraints (override generic design advice)

- **Brand tokens only** on customer surfaces: `var(--brand-primary)`, `var(--brand-bg)`, `var(--brand-text)`, `var(--brand-border)`, `var(--brand-font-heading)`. Never hardcode hex on `/s/:slug` or any client surface — the owner's theme drives it. Owner admin UI may use the app's own palette.
- **Mobile-first**, tap targets **≥48px**, thumb-reachable primary actions.
- **Glyphs**: Albanian ë/ç must render (fonts load `subset=latin-ext`).
- **Launch swan hero = lightweight CSS/SVG/Motion**, NOT WebGL. The Three.js cinematic is an explicit fast-follow tracked separately. No Three.js/GSAP in the launch path.
- **Money is integer ALL** — use the locale formatter, never invent decimals.
- **Offline is a designed state** — the order-status offline banner surfaces the restaurant phone as a `tel:` action.
- **Embed safety**: no `position:fixed` inside `?embed=true` storefronts.

## Surface → skills map

| Surface | Pull |
|---|---|
| `/start` swan hero + value strip | `illustration-style`, `animation-principles`, `motion-system`, `interfaces-that-feel`, `aesthetic-usability`, `visual-hierarchy` |
| Menu import / onboarding | `onboarding-design`, `loading-states`, `feedback-patterns`, `error-handling-ux`, `micro-interaction-spec` |
| Key-fields + live preview (ActivationPage) | `feedback-patterns`, `micro-interaction-spec`, `theming-system`, `layout-grid` |
| Public menu `/s/:slug` + cart | `visual-hierarchy`, `typography-scale`, `spacing-system`, `color-system`, `responsive-design`, `theming-system` |
| Checkout (map pin) | `feedback-patterns`, `error-handling-ux`, `loading-states` |
| Order status (WS + offline) | `loading-states`, `feedback-patterns`, `error-handling-ux`, `interfaces-that-feel` |

## Workflow for every launch UI task

1. Read the brief; pull only fitting skills (no mechanical rule-application).
2. Build against brand tokens + the constraints above.
3. **`/critique-screen`** on a real screenshot; resolve every **P1** before "done".
4. **Prove it**: Playwright E2E vs `https://dowiz.fly.dev` (or `VITE_BASE_URL`) with `toBeVisible()`/`toContainText()` on real DOM. Typecheck/build ≠ UI proof.

## Anti-slop tells to kill on sight

Gradient-on-card cliché · everything equally weighted (no focal point) · default shadow stacks · centered-everything · emoji-as-icon · lorem cadence in copy · motion that eases nothing · spinners with no mood · error copy that blames the user. The swan is the brand's one moment of delight — make it feel **authored, not stock**.
