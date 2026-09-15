# Storefront design — the decisions, and what each one answers to

**Surface:** the diner's storefront served by `workers/api/public/`.
**Register:** `brand` (PRODUCT.md) — design *is* the experience here. The owner
console at `/admin` is `product` register and deliberately shares no visual
language with this; the courier app at `/courier` is its own third thing.

Every choice below cites the document it comes from. Where I departed from a
project default, the departure is stated with the measurement that forced it.

---

## 1. Colour — the tenant's, not mine

The palette is **not** a preset I picked. It is what the live deployment serves
for this restaurant at `/public/locations/28239442-.../theme.css`:

```
--brand-primary:#e11d48  --brand-primary-hover:#cd3409  --brand-secondary:#457b9d
--brand-bg:#fdf2f8       --brand-text:#212529          --brand-radius:8px
```

PRODUCT.md requires the storefront to carry *the restaurant's* identity rather
than dowiz's, and `impeccable` step 5 says identity-preservation wins wherever
committed brand colours already exist. They do. So the tenant's values win over
DESIGN.md's default "Food Dark" preset and over "Crimson Classic".

**One measured departure.** DESIGN.md's `--brand-text-muted: #6B7280` lands at
**4.43:1** on this blush ground — it fails WCAG AA for body text. `impeccable`
names this exact failure as the most common one in AI design ("muted gray body
text on a tinted near-white"). The storefront uses `#6b5f66` instead: a muted
that carries the background's own hue, measured at **5.57:1**.

Contrast measured, not eyeballed:

| Pair | Ratio | Verdict |
|---|---|---|
| `#212529` body on `#fdf2f8` | 14.13:1 | pass |
| `#6B7280` muted on `#fdf2f8` | 4.43:1 | **fails AA body** — rejected |
| `#6b5f66` muted on `#fdf2f8` | 5.57:1 | pass |
| `#e11d48` primary on `#fdf2f8` | 4.30:1 | large text only — never used for small text |
| white on `#e11d48` | 4.70:1 | pass |

**Semantic and status colours are immutable** (DESIGN.md §1) and do not move when
a tenant re-skins: `--color-success:#059669`, `--color-warning:#D97706`,
`--color-danger:#DC2626`, `--color-info:#2563EB`, plus the per-status set.

**Dark mode ships on every screen.** `deliveryos-ui` makes its absence a red
line. Tokens are redefined in three places — bare `:root`, the
`prefers-color-scheme` block guarded so an explicit light choice still wins, and
`[data-theme="dark"]` — and **no component rule lives inside a theme block**,
which is what stops a colour existing in one theme and not the other.

## 2. Type — one scale, and a global heading rule

Faces are paired on a contrast axis, not two near-identical sans: **DM Serif
Display** for headings against **DM Sans** for body, which is the default
preset's pairing in DESIGN.md §2.

The scale is the project's own (`packages/ui/src/theme/tokens.css`): `--text-2xs`
through `--text-3xl`. Nothing in the stylesheet sets a raw font size.

**`h1,h2,h3 { font-family: var(--brand-font-heading) }` is set globally.** Its
absence is finding **T1** in `docs/design/storefront-polish/CONSISTENCY-AUDIT.md`
and the reason the same dish name rendered in the heading font on the card and
the body font in the detail modal on the old build. Setting it per call-site is
how that drift starts.

Display letter-spacing is `-.02em`, above `impeccable`'s `-.04em` floor. Headings
get `text-wrap: balance`, prose `text-wrap: pretty` at a 65ch measure.

## 3. Spacing and radius — one grid, one card

Spacing is the 4px grid from DESIGN.md §3, whose rule is literal: *"Only these
values. Never 6px, 10px, 14px, 18px, 22px, 28px."* Every gap and pad is a
`--space-*` token.

There is **one** card radius. The audit's fix #3 found `12px / 16px / 24px` all
meaning "a card" across screens, and two radius families holding identical pixels
under different names. Here `--radius-card` is defined once and resolves to the
tenant's own `8px`.

## 4. The primary CTA — one signature

The audit counted **five** signatures for the same button (heights 44/46/48/56px,
two radii, two weights, three text sizes). This storefront has one `.btn`: 44px
minimum target, pill radius, bold, `--text-base`. `.btn-ghost` is the same
geometry with a different surface.

## 5. Motion — reviewed against the ten standards

Measured against `review-animations`' non-negotiables:

| Standard | How it is met |
|---|---|
| GPU-only properties | Only `transform` and `opacity` animate. The skeleton was a `background-position` sweep and is now a `transform` sweep. |
| Responsive easing | `cubic-bezier(.2,.8,.2,1)` on sheets and the cart bar; `ease-out` on state changes. No `ease-in` anywhere. |
| Sub-300ms | Sheet 260ms, cart bar 220ms, scrim 180ms, presses 120ms. |
| Press feedback | `.btn:active{transform:scale(.97)}`, `.qty button:active{scale(.92)}` — `impeccable`'s point that a button with no `:active` feels dead. |
| Hover gated | Every hover rule sits inside `@media (hover:hover) and (pointer:fine)`; an ungated hover on a phone sticks after the tap. |
| Reduced motion | Movement removed, **meaning kept**: fades and colour survive, travel does not. Zeroing every duration also removes the fade that tells you the sheet opened. |
| Interruptible | CSS transitions throughout, not keyframes that restart — the toast retargets from wherever it is. |

## 6. Iconography

Tabler (`ti ti-*`), never emoji. `deliveryos-ui` lists emoji-as-UI as a red line,
and the old standalone PWA broke it.

## 7. The empty-photo problem

PRODUCT.md names the enemy exactly: *"flat dark cards with 160px empty grey image
placeholders"*. A dish with no photo gets a crafted mark whose hue is a stable
hash of its name, so the same dish looks the same on every visit and the grid
never shows a dead rectangle. A sold-out dish stays **visible**, dimmed, with its
reason attached — hiding it loses the information that it exists at all.

## 8. Failure states name the real fallback

Every error path surfaces the venue's own phone number. The old service's design
insisted on this and it is right: when the platform cannot help, the restaurant
still can.

---

## Sources

- `PRODUCT.md` — register, users, brand personality, anti-references
- `docs/design/DESIGN.md` — colour tokens, immutable semantics, type, 4px grid, elevation, z-index
- `docs/design/storefront-polish/CONSISTENCY-AUDIT.md` (on `origin/backup-wip-2026-07-08`) — the five drift findings this file answers
- `docs/design/storefront-polish/AWWWARDS-RESEARCH.md` + refs — food-site reference shots
- `.agents/skills/impeccable` — contrast floor, type pairing, letter-spacing floor, "cards are the lazy answer"
- `.agents/skills/deliveryos-ui` — red lines, two-speed rule, component states
- `.agents/skills/deliveryos-theme` — token-only theming, presets
- `.agents/skills/review-animations` + `emil-design-eng` — the ten motion standards
- The live tenant theme at `/public/locations/:id/theme.css`
