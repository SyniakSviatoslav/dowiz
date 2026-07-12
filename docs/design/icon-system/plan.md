# Icon System Plan — `lucide-react` as the unified, brandable, animated icon set

> Design-time plan. No code, no commit, **no `package.json` edit** — the dep addition is a
> protect-path staged for the operator (see §6). Author: research+plan lane, 2026-07-02.

## 0. TL;DR

Replace the current **Tabler icon webfont** (`@tabler/icons-webfont`, 423 `ti-*` usages across 59
files) with **`lucide-react`** — per-icon ES-module React components, ISC-licensed,
`currentColor`-driven strokes. One `<Icon>` atom in `packages/ui` maps every icon to the tenant
brand through the **existing `derivePalette` → CSS-var** pipeline (color via `currentColor`,
`strokeWidth` as a brand "weight" knob, a size-token scale). Animation **reuses the repo's already-
provisioned `LazyMotion`/`m`** (zero new motion dep) plus one pure-CSS keyframe for the loading spin,
with a small purposeful micro-animation set wired to the `motion.ts` / `tokens.css` motion tokens and
gated by `prefers-reduced-motion`. Migration runs Tabler→lucide in collision-free lanes:
`packages/ui` atoms → shared molecules → per-surface (`admin` / `client` / `courier`), each lane a
distinct file set.

---

## 1. Ground truth — how icons are done today

| Mechanism | Where | Count | Notes |
|-----------|-------|-------|-------|
| **Tabler webfont** `<i className="ti ti-…">` | `apps/web/src` + `packages/ui/src` | **423** `ti-` occurrences, **59** files | dep `@tabler/icons-webfont@3.31.0` in `apps/web` **and** `apps/api` package.json; CSS imported once in `apps/web/src/main.tsx:8`. |
| **Wrapper atom** `Icon.tsx` | `packages/ui/src/components/atoms/Icon.tsx` | 1 component + `ICONS` name map (49 names) | renders `<i class="ti ti-${name}">`; `size` via `SIZE_MAP` (sm/md/lg/xl → rem), `stroke` via `--ti-stroke`. **Barely adopted** — most call sites write raw `<i className="ti ti-…">` inline, not `<Icon>`. |
| **Inline `<svg>`** | 13 files | 13 | mostly *not* icons: `NomadicScene`, `PaperIllustration`, `CourierLiveMap`, `StylizedMap`, `SatelliteMap`, `DishStats`, `SwipeToComplete`, `Button` (spinner), `AnimatedCheck`. These are illustrations/maps/bespoke motion — **out of scope**, keep as-is. |
| **Emoji** | ~28 literals in tsx | 28 | scattered; not a system. Opportunistically replace where they stand in for UI icons; leave content emoji. |

Colouring today: icons inherit `currentColor` or take `style={{ color: 'var(--brand-primary)' }}` /
`text-[var(--color-success)]` etc. ad-hoc. So the brand seam already exists — lucide (`currentColor`
default) slots into it directly.

**Top-churn files to sequence carefully** (also the app hotspots per CLAUDE.md):
`apps/web/src/pages/client/MenuPage.tsx` (54), `admin/MenuManagerPage.tsx` (53),
`admin/DashboardPage.tsx` (32), `admin/SupplyLibraryPage.tsx` (24), `routes/AdminRoutes.tsx` (18),
`admin/AnalyticsPage.tsx` (18), `packages/ui/.../admin/OrderCard.tsx` (17).

### Theme system (the brand seam)

- `packages/ui/src/theme/palette.ts` → `derivePalette(input)` computes a full, contrast-safe token set
  (`primary`, `primaryHover`, `primaryReadable`, `primaryStrong`, `onPrimary`, `primaryLight`,
  `accent`, `bg`, `surface`, `surfaceRaised`, `text`, `textMuted`, `border`, `fontHeading`,
  `fontBody`) from minimal tenant input (often just a primary + bg).
- `packages/ui/src/theme/ThemeProvider.tsx` writes those onto `document.documentElement` as CSS vars
  (`--brand-primary`, `--brand-text`, `--brand-text-muted`, `--brand-primary-readable`, …). Per-tenant
  storefront fonts (`location_themes.heading_font/body_font`, mig 084) resolve through the same path.
- **Icons ride this for free**: `currentColor` = whatever `color` cascades, and every brand token is a
  CSS var. No JS bridge from theme→icon is needed; it's all CSS inheritance.

### Motion system (reuse target)

- `ThemeProvider` mounts **one** `LazyMotion features={domAnimation}` (framer-motion), so `m.*`
  components (`import { m } from 'framer-motion'`) are cheap (~4.6 KB `m` + lazy features vs ~46 KB
  full `motion`). The repo **just migrated `packages/ui` to `m` + `LazyMotion`** — icon animation must
  reuse this, not add a competing motion lib.
- `packages/ui/src/lib/motion.ts` — the motion SoT: `ease` (out/inOut/soft/bounce), `spring`
  (press/enter/bounce/gentle), `duration` (instant .08 / fast .15 / base .24 / slow .4), plus variants.
- `packages/ui/src/theme/tokens.css` — CSS mirror: `--motion-instant/fast/base/slow`,
  `--ease-out/in-out/soft`, and a `@media (prefers-reduced-motion: reduce)` block that **zeros all
  `--motion-*` durations** (state preserved). Icon CSS animations that use these vars are reduced-motion-
  safe automatically; `m` variants must gate via `useReducedMotion()` at the call site.
- Size/spacing tokens: `--space-*`, `--radius-*` exist; there is **no icon-size scale token yet** —
  this plan adds one (`--icon-*`, §2) to replace the ad-hoc `SIZE_MAP`.

### Tooling grammar it must satisfy (`TOOLING-REGISTRY.md` + tooling-decision-patterns memory)

- **license-first** → lucide is **ISC** (OSI permissive, MIT-equivalent). PASS. Animation source
  `pqoqubbw/lucide-animated` is **MIT** — we *vendor patterns*, not the dep. PASS.
- **don't-conflict-utilize** → reuse existing `LazyMotion`/`m`; do **not** add `motion`/`gsap`/
  `lottie`. PASS by design.
- **pick-one-lane** → lucide *replaces* Tabler; we do not run both webfont + SVG sets in parallel past
  the migration window. Removing `@tabler/icons-webfont` from both `apps/web` and `apps/api` is the
  closing act.
- **subtractive-first** → net effect deletes a webfont CSS payload + 423 stringly-typed class names.

---

## 2. `lucide-react` — confirmation + the pick vs alternatives

**Confirmed (sources §7):**
- **License: ISC** (permissive, commercial-safe). ✔ passes license-first gate.
- **Tree-shaking:** ES-module, **per-icon named imports** (`import { ShoppingCart } from
  'lucide-react'`); only imported icons ship — ~1 KB gzip each, not the full 1,000+ set.
- **Prop model:** `size` (number|string, default 24), `color` (default **`currentColor`**),
  `strokeWidth` (default 2), `absoluteStrokeWidth` (bool — keeps stroke a constant px regardless of
  size). All extra SVG props (incl. `className`, `aria-*`, event handlers) are forwarded to the `<svg>`.
- **Bundle gotcha to avoid:** import from the package root (`from 'lucide-react'`), which is tree-
  shaken by modern bundlers (Vite/Rollup — this repo). Do **not** deep-import per-file paths blindly,
  and be aware of the historical "repeated license header" bloat issue in some setups (lucide #3744) —
  Vite's minifier strips it; verify in the bundle-size check (§6). If tree-shaking ever misbehaves,
  `lucide-react/dynamic` or `unplugin-icons` are fallbacks (not needed initially).

**Why lucide over the alternatives (brief):**

| Set | License | Style | Verdict |
|-----|---------|-------|---------|
| **lucide-react** | ISC | 24px stroke, `currentColor`, `strokeWidth` knob, 1,000+ | **PICK** — stroke-based (matches the current Tabler look → low visual churn), the `strokeWidth` prop *is* the "brand weight" knob this plan needs, best-maintained React port, huge coverage. |
| Heroicons | MIT | outline+solid, 24/20, no stroke-width knob | good but two fixed weights, smaller set, no per-icon stroke control. |
| Phosphor | MIT | 6 weights, heavier API | more weights but larger surface + different visual language (a bigger redesign than a swap). |
| Tabler (SVG-react) | MIT | 24px stroke, ~5,900 | *closest* to today, but we're already on Tabler **webfont** — the whole point is to move off a webfont to tree-shaken SVG with a real prop model + animation story; lucide is the better-supported React-first set of the two. |

Lucide is the right pick: a **stroke-based, `currentColor`, per-icon-import** set whose native
`strokeWidth` prop gives us the brand-weight lever, with minimal visual churn from Tabler.

---

## 3. The brandable `<Icon>` wrapper (one atom, `packages/ui`)

**Placement:** extend the existing `packages/ui/src/components/atoms/Icon.tsx` (read-before-edit; keep
`ICONS` name-map exported for the migration, mapping old Tabler names → lucide component names during
transition). Barrel-export from `packages/ui/src/index.ts`.

**Design principles**
- **Color** = `currentColor` (lucide default). The wrapper sets `color` from a **semantic `tone` prop**
  that maps to the brand CSS vars already produced by `derivePalette`/`ThemeProvider` — so a tenant's
  storefront icons take their palette **automatically** via CSS inheritance. No JS read of theme
  context is required for color (it's pure CSS cascade); `useBrandTheme()` is available only if a rare
  computed case needs the raw hex.
- **Size** = a token scale (`xs/sm/md/lg/xl`) → new `--icon-*` CSS vars (replaces the ad-hoc
  `SIZE_MAP`); numeric passthrough still allowed.
- **Weight** = `strokeWidth` prop, defaulting to a brand token `--icon-stroke` (a single knob a tenant
  theme can later set to make all icons feel lighter/bolder). `absolute` prop forwards
  `absoluteStrokeWidth` so large icons keep a crisp constant stroke.
- **A11y** = decorative by default (`aria-hidden`, `focusable=false`); a `label` prop switches to
  `role="img" aria-label`. (Fixes today's mostly-missing `aria-hidden` on `<i class="ti">`.)
- **Reduced motion** = when `animate` is set, gate via `useReducedMotion()`; static render otherwise.

**API sketch**

```tsx
// packages/ui/src/components/atoms/Icon.tsx
import { type LucideIcon } from 'lucide-react';

type IconTone = 'current' | 'primary' | 'ink' | 'muted' | 'success' | 'warning' | 'danger' | 'on-primary';
type IconSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl';
type IconAnim = 'none' | 'hover' | 'tap' | 'spin' | 'success' | 'pulse'; // §4

const TONE_VAR: Record<IconTone, string> = {
  current:      'currentColor',
  primary:      'var(--brand-primary-readable, var(--brand-primary))', // AA-safe as text
  ink:          'var(--brand-text)',
  muted:        'var(--brand-text-muted)',
  'on-primary': 'var(--color-on-primary)',
  success:      'var(--color-success)',
  warning:      'var(--color-warning)',
  danger:       'var(--color-danger)',
};
const SIZE_VAR: Record<IconSize, string> = {
  xs: 'var(--icon-xs, 0.875rem)', sm: 'var(--icon-sm, 1rem)',
  md: 'var(--icon-md, 1.25rem)',  lg: 'var(--icon-lg, 1.5rem)', xl: 'var(--icon-xl, 2rem)',
};

export interface IconProps extends Omit<React.SVGProps<SVGSVGElement>, 'ref' | 'color'> {
  icon: LucideIcon;              // e.g. import { ShoppingCart } → <Icon icon={ShoppingCart} />
  size?: IconSize | number;
  tone?: IconTone;              // → color = currentColor driven by a brand token
  weight?: number;             // strokeWidth; default var(--icon-stroke)
  absolute?: boolean;          // absoluteStrokeWidth passthrough
  animate?: IconAnim;          // §4 — reuses m/LazyMotion or CSS
  label?: string;              // set → role="img" aria-label; unset → aria-hidden
}

export function Icon({ icon: Glyph, size='md', tone='current', weight, absolute, animate='none', label, style, ...rest }: IconProps) {
  const px  = typeof size === 'number' ? size : undefined;
  const dim = typeof size === 'number' ? size : SIZE_VAR[size];
  const a11y = label ? { role: 'img', 'aria-label': label } : { 'aria-hidden': true, focusable: false as const };
  const el = (
    <Glyph
      size={px}
      width={px ? undefined : dim} height={px ? undefined : dim}
      strokeWidth={weight ?? 'var(--icon-stroke, 2)'}
      absoluteStrokeWidth={absolute}
      color="currentColor"
      style={{ color: TONE_VAR[tone], ...style }}
      {...a11y} {...rest}
    />
  );
  return animate === 'none' ? el : <AnimatedIcon variant={animate}>{el}</AnimatedIcon>; // §4
}
```

New tokens to add in `tokens.css` (`:root`): `--icon-xs..xl` (mirror the scale above) and
`--icon-stroke: 2;` — a tenant theme may later override `--icon-stroke` for a brand weight, exactly
like fonts already override `--brand-font-*`.

---

## 4. Animation — chosen approach, why, sources

**Decision: reuse the repo's `LazyMotion` + `m` (framer-motion) for interactive/state animations,
plus one pure-CSS keyframe for the indefinite loading spin. Add NO new motion dependency.**

**Why:**
- The ecosystem's best animated-lucide pattern — **`pqoqubbw` / `lucide-animated`** (MIT) — is
  *copy-paste per-icon React components animated with Motion (framer-motion) variants on the SVG
  paths, triggered on hover*. Our repo **already** provides framer-motion via `LazyMotion`/`m`, so we
  adopt that exact pattern **without its npm dep** — we vendor the technique, honoring
  *don't-conflict-utilize* and *license-first* (MIT source, ISC icons).
- Alternatives rejected: adding `motion`/`lucide-animated`/`lottie` = a competing motion lib (violates
  the just-completed `m`/LazyMotion migration). Pure CSS-only for *everything* can't do path-draw /
  variant orchestration as cleanly — but it **wins for the infinite spinner** (no JS, GPU transform,
  already the repo idiom `animate-spin`), so we use CSS there.
- All motion maps to existing tokens (`motion.ts` `duration`/`ease`, `tokens.css` `--motion-*`), and
  the `@media (prefers-reduced-motion)` block already zeroes CSS durations; `m` variants gate via
  `useReducedMotion()`. Nothing gratuitous — a fixed, purposeful set only.

**The `AnimatedIcon` wrapper** applies a variant to `m.span`/the icon's `motion` props; for path-draw
variants (success check) it swaps to a small vendored per-icon `m.svg`+`m.path` component (the
pqoqubbw pattern) since generic wrapping can't animate interior paths.

**Purposeful micro-animation set** (each tied to a token, each reduced-motion-safe):

| `animate` | Trigger | Motion | Token | Purpose |
|-----------|---------|--------|-------|---------|
| `hover`   | parent hover | `scale 1→1.08` (or slight rotate for settings/refresh) | `duration.fast` + `ease.out` | affordance on interactive icon buttons — subtle. |
| `tap`     | press | `scale → 0.9` | `spring.press` | tactile feedback; mirrors existing `scaleTap`. |
| `spin`    | `loading` prop | continuous rotate | **pure CSS** `@keyframes` × `--motion` (∞) | loaders (replaces `ti-loader animate-spin`). |
| `success` | state change | path **draw-on** (checkmark) | `duration.base` + `ease.out` | confirm actions (order placed / saved) — vendored `m.path` pattern. |
| `pulse`   | live/attention | `scale/opacity` pulse | reuse `pulseDot` variant from `motion.ts` | live badges/notifications; already exists — icon reuses it. |

Constraint held: reuse `pulseDot` from `motion.ts` rather than re-authoring; loaders use CSS not JS.
No decorative/idle animation on static content icons — motion only signals interaction or state.

---

## 5. Migration inventory + phased, collision-free plan

**Inventory estimate**
- **423** `ti-*` usages across **59** files to convert.
- **~49** distinct icon names in the current `ICONS` map + more used inline; expect **~70–90 distinct
  lucide icons** after mapping (Tabler→lucide names differ, e.g. `ti-world`→`Globe`,
  `ti-dots-vertical`→`EllipsisVertical`, `ti-menu-2`→`Menu`, `ti-circle-check-filled`→`CheckCircle2`).
- **13** inline `<svg>` files — **out of scope** (illustrations/maps/bespoke motion), audited and kept.
- ~28 emoji literals — opportunistic, low priority.
- **2** package.json entries to remove at close (`apps/web`, `apps/api` — `@tabler/icons-webfont`) +
  drop the CSS import in `main.tsx:8`.

**Sequencing rule:** `packages/ui` (shared atoms/molecules) MUST land first — they're imported by all
three apps' surfaces, so converting them first prevents a surface lane from depending on an
un-migrated shared component. The `<Icon>` atom + tokens + name-map is the shared integration point
the lead lands before fan-out.

**Phase 0 (lead, no fan-out):** land `<Icon>` atom rewrite, `--icon-*`/`--icon-stroke` tokens,
`AnimatedIcon` + vendored success/spin, the Tabler→lucide name map, barrel export, and a bundle-size /
tree-shake check. This is the one hot file everything else builds on.

**Phase 1 → 3: collision-free lanes (fan-out).** Partition by directory so no two lanes touch the same
file:

| Lane | Scope (distinct file sets) | ~files | Notes |
|------|----------------------------|--------|-------|
| **L1 · ui-shared** | `packages/ui/src/components/{atoms,molecules,client,admin,courier}` (`OrderCard` 17, `ProductCard` 7, `Toast`, `StateChip`, `TourHint`, `MapWithPin`, `Textarea`, `SearchInput`, `SunlightToggle`, `CurrencySwitcher`, `I18nProvider`) | ~13 | **runs in Phase 1 alone** (dependency root); others wait for it. |
| **L2 · admin** | `apps/web/src/pages/admin/**` + `apps/web/src/components/admin/**` + `routes/AdminRoutes.tsx` (`MenuManagerPage` 53, `DashboardPage` 32, `SupplyLibraryPage` 24, `AnalyticsPage` 18, `CouriersPage` 16, `PromotionsPage`, `RecipeEditor`, `CRMPage`, `ActivationPage`, `MediaManager`, `SettingsPage`, `AllergenEditor`, `BrandingPage`, `FlowTestPage`, `LoginPage`) | ~18 | biggest lane; can split L2a pages / L2b components if parallelised further. |
| **L3 · client/storefront** | `apps/web/src/pages/client/**` + `apps/web/src/components/client/**` + `routes/ClientLayout.tsx` (`MenuPage` 54, `OrderStatusPage`, `CheckoutPage`, `MenuComparePanel`, `checkout/*` sections, `DishStats`) | ~12 | the brand-facing surface — validate per-tenant palette on `/s/:slug` here. |
| **L4 · courier** | `apps/web/src/pages/courier/**` + `routes/CourierRoutes.tsx` (`ShiftPage`, `EarningsPage`, `DeliveryPage`, `HistoryPage`, `TasksPage`) | ~6 | independent surface. |
| **L5 · misc/root** | `apps/web/src/main.tsx`, `apps/web/src/lib/messenger.ts`, `MenuFirstOnboarding.tsx`, `i18n-catalog.ts` string icons | ~4 | small; lead can absorb. |

L2/L3/L4/L5 are **mutually collision-free** (disjoint dirs) → fan out concurrently **after L1 merges**.

**Phase 4 (lead, close-out):** remove `@tabler/icons-webfont` from both package.json files + the CSS
import; grep-assert **zero** remaining `ti ti-` / `ti-` class usages; run the Mandatory-Proof E2E on
`/s/:slug` (storefront icons take tenant palette), `/admin/*`, `/courier/*` + typecheck + bundle-size
delta. The package.json removals are protect-path → staged for the operator (§6).

---

## 6. Dependency addition — protect-path + new-dep-scan (operator gate)

- `package.json` edits are a **protect-path** → this plan does **not** touch them. Adding
  `"lucide-react"` to `apps/web/package.json` (and the eventual **removal** of
  `@tabler/icons-webfont` from `apps/web` **and** `apps/api`) is **staged for the operator**.
- New dep must pass **`scripts/new-dep-scan.mjs`**: after `lucide-react` is added, run
  `node scripts/new-dep-scan.mjs` (reports it as a newcomer to reverse-engineer per the 12-rule
  grammar), reverse-engineer it (ISC ✓, no runtime side-effects, tree-shaken), then
  `node scripts/new-dep-scan.mjs --bump` to record the new baseline (`loops/runs/dep-baseline.json`).
- **Bundle proof required** (Mandatory Proof + tree-shake claim): capture the `apps/web` production
  bundle delta before/after — expect it to **shrink** (drop the Tabler webfont CSS + font files; add
  only the ~70–90 imported lucide SVGs ≈ tens of KB, tree-shaken). Confirm no full-set import leaked
  in and no repeated-license bloat (lucide #3744).
- One-per-turn edit discipline for the atom; `packages/ui` is the hot file — lead-only.

---

## 7. Sources

- Lucide — React package / tree-shaking / props: <https://lucide.dev/guide/packages/lucide-react>,
  <https://lucide.dev/guide/> , npm: <https://www.npmjs.com/package/lucide-react>
- Lucide — stroke-width & `absoluteStrokeWidth`: <https://lucide.dev/guide/react/basics/stroke-width>
- Lucide — repeated-license bundle issue (verify in bundle check): lucide-icons/lucide#3744
- Animated-lucide pattern (framer-motion variants on SVG paths, hover-triggered, copy-paste per-icon,
  **MIT**): `pqoqubbw` / **lucide-animated** — <https://lucide-animated.com/> (redirect from
  icons.pqoqubbw.dev), announce: <https://x.com/pontusab/status/1853402612877689125>
- Repo grounding: `packages/ui/src/components/atoms/Icon.tsx`, `theme/palette.ts`,
  `theme/ThemeProvider.tsx` (LazyMotion+`m`), `lib/motion.ts`, `theme/tokens.css`,
  `scripts/new-dep-scan.mjs`, `TOOLING-REGISTRY.md`.
</content>
</invoke>
