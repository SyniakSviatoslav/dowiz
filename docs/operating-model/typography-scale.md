# Typography Scale (additive, non-breaking)

> Status: shipped tokens + Tailwind utilities. Component adoption is incremental
> (no mass-migration). Source of truth: `packages/ui/src/theme/tokens.css`,
> exposed via `packages/ui/tailwind.config.ts`.

## Why this exists

`tokens.css` had spacing/radius/motion/color tokens but **zero** type tokens.
Screens carried ~521 ad-hoc `text-*` and ~383 `font-*` utilities plus pixel
literals (`text-[10px]`, `text-[22px]`, …). There was no shared, coherent type
rhythm — every screen reinvented its sizes.

## The scale

A 7-step modular scale (rem, so it respects user zoom), with paired
line-heights and weights. Defined as CSS custom properties in `tokens.css`:

| Token          | Size        | Intended use                         |
|----------------|-------------|--------------------------------------|
| `--text-xs`    | 0.75rem 12px | meta, captions, badges              |
| `--text-sm`    | 0.875rem 14px | secondary body, labels             |
| `--text-base`  | 1rem 16px    | body default                        |
| `--text-lg`    | 1.125rem 18px | lead body, card titles             |
| `--text-xl`    | 1.375rem 22px | section headings                   |
| `--text-2xl`   | 1.75rem 28px  | page headings                      |
| `--text-3xl`   | 2.25rem 36px  | hero / display                     |

Line-heights (unitless): `--leading-tight` 1.15 · `--leading-snug` 1.3 ·
`--leading-normal` 1.5 · `--leading-relaxed` 1.7.

Weights: `--weight-normal` 400 · `--weight-medium` 500 · `--weight-semibold` 600
· `--weight-bold` 700.

## Breaking vs non-breaking — the decision

**Non-breaking was chosen.** We did NOT redefine Tailwind's stock `text-xs` …
`text-3xl`.

Tailwind's defaults diverge from this scale at almost every step
(default `xl`=20px, `2xl`=24px, `3xl`=30px; ours are 22/28/36) and use
different line-heights. Keying the `fontSize` extend as `xs/sm/base/...` would
**override** those defaults and silently resize **all ~521 existing `text-*`
usages** — exactly the risky mass-migration this pass forbids. The same trap
applies to `leading-tight/snug/relaxed` (~15 usages, different stock values).

So the scale ships under **new, namespaced utilities** that cannot collide:

| Utility            | Maps to                                            |
|--------------------|----------------------------------------------------|
| `text-step-xs` … `text-step-3xl` | `--text-*` + a paired `--leading-*`  |
| `leading-step-tight/snug/normal/relaxed` | `--leading-*`               |
| `font-display`     | `var(--font-display, var(--brand-font-heading))`   |

`fontWeight` (`font-normal/medium/semibold/bold`) is also wired to the
`--weight-*` tokens — this is a **no-op** because those values are identical to
Tailwind's defaults, so nothing renders differently; it only makes the weights
token-traceable.

## How to adopt incrementally

1. **New components / new screens** — reach for `text-step-*`,
   `leading-step-*`, and `font-display` from day one.
2. **Touching an existing screen for another reason** — opportunistically swap
   a hand-picked size to the nearest `text-step-*` if it visually matches.
   Don't change rendered size just to adopt the token.
3. **Pixel literals are the first migration targets.** Replace
   `text-[Npx]` with the nearest step:
   - `text-[10px]`/`text-[11px]` → `text-step-xs`
   - `text-[13px]` → `text-step-sm`
   - `text-[22px]` → `text-step-xl`
   - `text-[26px]` → `text-step-2xl`
4. **A future, deliberate pass** can alias the stock `text-*` to the scale once
   every screen has been audited for size shifts. Until then, keep the two
   systems side by side.

### Known pixel-literal hotspots (documented, NOT changed this pass)

`apps/web/src/pages/client/MenuPage.tsx` (owned by another agent — left
untouched) carries the worst cluster:

| Literal        | Count | Suggested step    |
|----------------|-------|-------------------|
| `text-[10px]`  | 7     | `text-step-xs`    |
| `text-[11px]`  | 6     | `text-step-xs`    |
| `text-[13px]`  | 1     | `text-step-sm`    |
| `text-[22px]`  | 1     | `text-step-xl`    |
| `text-[26px]`  | 1     | `text-step-2xl`   |

These are the highest-value first migration when MenuPage is next opened.
