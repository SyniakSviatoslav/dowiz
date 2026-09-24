# dowiz design system — `/lib/ui`

2026-09-24. One set of components for every surface (storefront, room, owner
console, courier). The courier app is the reference migration; the other three
follow, one lane each, turning the `ui-adoption` ratchet down.

- Code: `workers/api/public/lib/ui/` (one module per component, `index.js` re-exports them all)
- Styles: `lib/tokens.css` (tokens and colour roles), `lib/ui/ui.css` (components, built only on tokens)
- Tests: `lib/ui/*.test.mjs`, run with `node --test workers/api/public/lib/ui/`
- Living style guide: `/ui-gallery/` (every component in every state, sq/en/uk, light/dark, made-up data)
- Gates: `tools/gates/ui-adoption.sh` (hand-rolled UI per surface, ratchet), `tools/gates/sw-shell.sh`
  (every offline shell lists every module its page imports), each with a `.prove.sh`

## Principles

1. **A component is a pure function: options in, escaped HTML string out.** The surfaces already
   build screens as template literals assigned to `innerHTML`. Adopting a component means
   replacing a piece of a template with a call. You can review it line by line, and node can test
   it without a browser.
2. **Behaviour is a separate, small binder.** `bindSegmented(el, fn)`, `bindTabs(el, fn)`,
   `createToaster(host)`, `openSheet(opts)`, `setBusy(btn, text)`. The surface keeps its state and
   re-renders; the system never owns application state.
3. **Every string is escaped.** `esc` in `core.js` is the one escaper; four byte-identical copies
   existed before it. `attrs()` refuses `style=` (the CSP drops it silently) and `on*=` (the CSP
   blocks it).
4. **Words belong to the surface; the hook belongs to the system.** Pass `{ t: 'key' }` and the
   component renders the translated text plus `data-t="key"` (or `data-t-attr="aria-label:key"`),
   so the surface's existing `retranslate()` rewrites it in place when the language changes.
   A plain string counts as already translated.
5. **Accessibility is built into the markup, not added afterwards.** An icon-only button with no
   `ariaLabel` throws. Fields wire `label[for]`, `aria-describedby` and `aria-invalid`. Errors
   are `role=alert`. The segmented control is a radiogroup, tabs are tabs, and a sheet is a modal
   dialog that traps focus and returns it.
6. **Mistakes fail loudly.** An unknown variant, tone, size, shape, field type or icon name throws.
   A raw number handed to `amount()` throws, because 1500 lek was once drawn as $15.00.
7. **Phone first, thumb sized.** Nothing tappable is under `--tap` (44px); primary actions on the
   courier are `--tap-lg` (56px). Inputs are 16px or larger, so iOS does not zoom on focus and
   pinch-zoom can stay enabled.
8. **Motion carries meaning.** Durations are tokens, and all of them drop to 0 under
   `prefers-reduced-motion`. Loops (spinner, pulse, shimmer) stop, and money never animates.

## Tokens (`lib/tokens.css`)

Every existing token name still works; the 2026-09-24 additions only add.

| Group | Tokens |
|---|---|
| Space (4px grid) | `--space-1..6, 8, 10, 12, 16` |
| Type | `--text-base` (1rem default; a surface may set its own), ratio `--text-ratio` 1.2 → `--text-xs, sm, lg, xl, 2xl, 3xl`; `--leading-tight/normal`; `--weight-normal/medium/semibold/bold`; `--tracking-tight`; `--font-sans` |
| Radius | `--radius-sm 4, md 8, lg 12, xl 16, 2xl 24, full` |
| Elevation | `--elevation-1..3`, `--ui-shadow-sheet` |
| Tap | `--tap` 44, `--tap-md` 48, `--tap-lg` 56 |
| Motion | `--motion-press 120ms, enter 200ms, exit 140ms, sheet 260ms`; `--ease-snap, enter, exit, spring, tide` |
| Stacking | `--z-sticky 50, --z-overlay 80, --z-toast 90` |
| Status hues | `--st-PENDING … --st-COMPENSATED_REFUND`, the same under every brand; a hue colours a DOT, never the word |

**Colour roles.** A component asks for a role, never a hex value or a surface's private name.
Each role reads the venue's `--brand-*` token when the surface defines one, and otherwise falls
back to dowiz's own chrome:

`--ui-bg, --ui-surface, --ui-surface-2, --ui-fg, --ui-fg-muted, --ui-line, --ui-accent,
--ui-on-accent, --ui-accent-edge, --ui-focus, --ui-scrim, --ui-float`, tone inks
`--ui-success-ink / warning / danger / info` (the tone mixed toward the page's own ink, so they
stay readable in both themes), `--ui-on-tone`, `--ui-on-danger`.

Dark mode re-points the same roles under `@media (prefers-color-scheme: dark)` (guarded by
`:root:not([data-theme="light"])`) and under `:root[data-theme="dark"]`. A surface forces a theme
by setting `data-theme` on `<html>`.

`lib/ui/tokens.test.mjs` measures the contrast of every fallback pair. In light mode: fg/surface
17.81, muted/surface 7.26, on-accent/accent 7.25, every tone ink ≥ 6.54. In dark mode every pair
is ≥ 5.47. White text on the gold accent is 2.46:1 and fails, which is why `--ui-on-accent` is ink.

**Knobs** (`lib/ui/ui.css` `:root`): these are the only values a surface should override:
`--ui-radius`, `--ui-radius-btn`, `--ui-font`, `--ui-font-heading`, `--ui-btn-h`, `--ui-btn-h-lg`,
`--ui-toast-bottom`. For example, the courier sets pill buttons, 48px secondaries, and a toast that
sits above its sheet.

## Component API

`import * as ui from '/lib/ui/index.js'` (from a surface), or with a relative path from a node test.
Call `ui.useTranslator(t)` once, passing the surface's `t(key, vars)`.

| Call | Renders | Notes |
|---|---|---|
| `button({ label, icon, iconEnd, variant, size, block, busy, busyLabel, disabled, pressed, href, target, ariaLabel, id, type, cls, attrs })` | `<button class="ui-btn ui-btn--{variant}">` or `<a>` when `href` | variants `primary` (one per screen), `success`, `secondary`, `ghost`, `danger`; sizes `md` 44 / `lg` 56; no `sm` |
| `iconButton({ icon, ariaLabel, variant:'plain', pressed, text })` | round 44px control | `ariaLabel` required |
| `setBusy(btn, text) → undo` | spinner + text, `disabled`, `aria-busy` | `undo()` restores the exact markup |
| `badge({ label, tone, dot, icon, live })` | non-interactive pill | tones `neutral accent success warning danger info` |
| `status({ status, label, pulse })` | FSM status: hue dot + ink word | unknown status → `UNKNOWN`, not a throw |
| `chip({ label, as:'span'|'button', selected, tone, dot, floating, live })` | readout or toggle | a `<span>` never pretends to be tappable |
| `field({ id, label, type, value, placeholder, hint, error, rows, money, … })` | label + control + hint + error | `rows` → textarea; `money` → numeric keypad, `.money` face |
| `inputRow({ id, label, placeholder, action })` | input with a trailing action | label is `aria-label` |
| `alert({ label, tone })` | inline message | `danger` → `role=alert`, others `role=status` |
| `segmented({ id, name, label, options:[{value,label,icon}], value })` + `bindSegmented(el, onChange)` | radiogroup | arrows/Home/End; roving tabindex |
| `tabs({ id, label, items:[{id,label,icon,panel,badge}], active })` + `bindTabs(el, onSelect)` | tablist | toggles `hidden` on the panels |
| `row({ title, sub, leading, trailing, select, pressed, href, data })`, `list(rows, { label, inset })` | the product's basic unit | `select` → `<button aria-pressed>` with a radio mark drawn in CSS |
| `emptyState({ icon, title, body, reason, action, alert, status, tone })` | "nothing here, and why" | `alert:true` for a failure |
| `skeleton({ shapes:[…] } | { shape, count }, label)` | `aria-busy` placeholder | shapes `line title block button row card` |
| `toastHost(id)`, `createToaster(host, { ms, timers }) → { show(msg, { icon, tone, ms }), hide }` | one polite live region per page | the host must be in the HTML before the first notice |
| `sheet(o)`, `openSheet(o) → { el, close }`, `confirmSheet(o) → Promise<bool>` | modal bottom sheet | focus trap, Esc, scrim, `data-ui-close="value"` |
| `amount(formatted, { size, tone, sign, strong })`, `stat({ label, value, hint, emphasis })` | money display | takes the STRING from `/lib/money.js`; never tweens |
| `card({ title, eyebrow, body, tone })`, `section({ title, sub, back:{id,label}, action, level })`, `para(text, { hint, center })` | grouping | `section` + `back` is the panel header |
| `when(ms, { style:'time'|'date'|'datetime', locale, timeZone })`, `mmss(seconds)` | `<time datetime>` | the caller passes the instant; the component never reads the clock |
| `esc, cx, attrs, icon, label, text, uid, TONES` | primitives | `icon(name)` validates the name |

Every component accepts `id`, `cls` and `attrs` (an object whose values are escaped; `data:{…}`
becomes `data-*`).

## Do / don't

| Do | Don't |
|---|---|
| `ui.button({ variant:'primary', label:{t:'take'} })` | `` `<button class="cta">${esc(t('take'))}</button>` `` |
| One `primary` (or `success` for a completing action) per screen | Two main actions side by side |
| `ui.amount(money(o.total))` | `` `<span class="money">${o.total / 100}</span>` `` |
| `ui.status({ status:o.status, label })` | colour the status word with the hue (IN_DELIVERY blue fails as text) |
| `ui.emptyState({ …, alert:true })` for a failure | a bare `<p>` with an error string |
| `ui.skeleton(…)` before the first answer | "You are offline" before the hub has answered |
| Change the look with a knob in the surface's sheet | Copy a `.ui-*` rule into a surface sheet and edit it |
| Check an icon name against `/lib/icons.css` (`css.test.mjs` does this) | Use a Tabler name that the sprite does not have: it renders nothing |
| Compute a size at run time with CSSOM (`el.style.x = …`) | Write `style="…"` in markup (the CSP drops it) |

## Migration recipe (for the admin, room and store lanes)

The courier migration (`courier/screens.js`, `courier/app.js`) is the worked example. For each
surface:

1. **Load the layer.** In the surface's `index.html`, add
   `<link rel="stylesheet" href="/lib/ui/ui.css">` after `components.css` and before the
   surface's own sheet. In the entry module, add `import * as ui from '/lib/ui/index.js'` and
   call `ui.useTranslator(t)`.
2. **Move screens out as pure functions.** Put them in `<surface>/screens.js` (or one file per
   area, each under 300 lines) with the signature `(data, ctx) => html`, where
   `ctx = { t, money, lang, langs }`. Import `../lib/ui/index.js` by relative path so node can load
   it. The ids that `app.js` binds and the e2e journeys drive are a contract: list them in the
   file header and keep every one.
3. **Replace fragments, one pattern at a time.** Before and after:

   ```js
   // before
   `<button class="cta go" id="retry" type="button">${icon('refresh')}${esc(t('retry'))}</button>`
   // after
   ui.button({ id:'retry', variant:'success', size:'lg', block:true, icon:'refresh', label:{ t:'retry' } })

   // before
   `<label for="em">${esc(t('emailOrPhone'))}</label><input id="em" autocomplete="username">`
   // after
   ui.field({ id:'em', label:{ t:'emailOrPhone' }, autocomplete:'username' })

   // before
   `<div class="empty" role="alert">${icon('plug-connected-x')}<b>${esc(t('noLink'))}</b></div>`
   // after
   ui.emptyState({ icon:'plug-connected-x', title:{ t:'noLink' }, alert:true })

   // before: a hand-rolled toast with its own timer
   el.innerHTML = `${icon(ic)}<span>${esc(m)}</span>`; el.hidden = false; setTimeout(…)
   // after: once per page, the host stays in index.html
   const toaster = ui.createToaster($('#toast')); toaster.show(m, { icon: ic, tone: 'danger' });

   // before: button busy state written by hand in every handler
   const had = b.innerHTML; b.disabled = true; b.innerHTML = …; /* on failure */ b.innerHTML = had;
   // after
   const restore = ui.setBusy(b, t('saving')); /* on failure */ restore();
   ```
4. **Delete what the components replaced from the surface's CSS.** Keep only what belongs to that
   surface alone (the courier's map, sheet, swipe track and voice read-back). The courier's sheet
   shrank from 487 lines to 196.
5. **Test the screens in node.** Follow `courier/screens.test.mjs`: every bound id is present, one
   main action per screen, every server string escaped (`injected()` from `lib/ui/dom-shim.mjs`),
   money goes through the formatter it is handed.
6. **Update the service worker** if the surface has one. Add `screens.js` and every `/lib/ui/*`
   module to the shell list and bump `SHELL_CACHE`; `sw-shell.sh` refuses the change until you do.
7. **Turn the ratchet.** Run `sh tools/gates/ui-adoption.sh`, then lower that surface's lines in
   `tools/gates/ui-adoption.baseline` to the new counts in the same commit.
8. **Run the checks:** `bash scripts/design-gate.sh`, `sh tools/gates/tap-size.sh`,
   `sh tools/gates/sw-shell.sh`, `node --test workers/api/public/lib/ui/ workers/api/public/<surface>/`,
   and parse-check each changed `.js` (copy it to `.mjs`, then `node --check`).

## What the gallery is for

`/ui-gallery/` is the review surface. Look at a component change there in both themes and all three
languages before it reaches a surface. The page is static, uses no real data, and needs no login.
