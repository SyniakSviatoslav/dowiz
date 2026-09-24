// The storefront's pieces on top of /lib/ui (docs/design/DESIGN-SYSTEM.md).
//
// THE VENUE'S SKIN STAYS THE STOREFRONT'S. A component renders the markup,
// the accessibility and the i18n hook; the storefront's own class on it
// (`.btn` -- the foil primary, `.btn-ghost` -- the glass secondary, `.seg-b`,
// `.qty`, `.card-add`, `.dback`, `.rail-chip`) keeps the venue's look, drawn in
// store.css from the venue's `--brand-*` tokens. store.css loads after ui.css,
// so on a tie the skin wins; the colour ROLES under the components read the
// same `--brand-*` tokens (lib/tokens.css), so a venue's accent reaches both.
//
// PURE: options in, escaped HTML out. Relative imports only, so node loads
// this file and store/parts.test.mjs renders every helper for real.
//
// `data-tour` is the learning engine's anchor (§5.1 of
// docs/design/BLUEPRINT-LEARNING-VIDEOS-WIKI-2026-09-24.md), listed in
// docs/learn/anchors-store.txt: an attribute only, it changes no behaviour.
import * as ui from '../lib/ui/index.js';

export { ui };

/// The i18n key reference the components render with `data-t`, which the
/// storefront's own `retranslate()` rewrites on a language switch.
export const k = key => ({ t: key });

/// `attrs` with the anchor merged into `data`.
const withTour = (attrs = {}, tour) => (tour ? { ...attrs, data: { ...(attrs.data || {}), tour } } : attrs);

/// THE primary: one per sheet, in the venue's foil.
export const cta = ({ tour, attrs, cls, ...o }) => ui.button({ variant: 'primary', size: 'lg', block: true, ...o,
  cls: ui.cx('btn', cls), attrs: withTour(attrs, tour) });

/// A second action, in the venue's glass.
export const ghost = ({ tour, attrs, cls, ...o }) => ui.button({ variant: 'secondary', block: true, ...o,
  cls: ui.cx('btn', 'btn-ghost', cls), attrs: withTour(attrs, tour) });

/// One choice of a few (delivery or pickup, now or later), as a pressed chip
/// in the storefront's segment skin. `on` is the chosen one; the binder
/// toggles `on` and `aria-pressed` together, as it always did.
export const seg = ({ on, tour, attrs, cls, ...o }) => ui.chip({ as: 'button', selected: !!on, ...o,
  cls: ui.cx('seg-b', on && 'on', cls), attrs: withTour(attrs, tour) });

/// The quantity stepper: minus, the count, plus. `minus`/`plus` are the two
/// buttons' attributes (their `data-*` and ids are the binder's contract).
export function stepper({ value, valueId, minus = {}, plus = {}, tour }) {
  const count = valueId ? `<span id="${ui.esc(valueId)}">${ui.esc(value)}</span>` : `<span>${ui.esc(value)}</span>`;
  return `<span class="qty"${tour ? ` data-tour="${ui.esc(tour)}"` : ''}>${ui.iconButton({ icon: 'minus', ariaLabel: '−', ...minus })}${count}${ui.iconButton({ icon: 'plus', ariaLabel: '+', ...plus })}</span>`;
}

/// "Nothing here, and why", as a whole sheet's body.
export const emptySheet = ({ icon, title, body, action }) => ui.emptyState({ icon, title: k(title), body: body ? k(body) : null, action });

/// Before an answer: the shape of what is coming, labelled for a screen reader.
export const rows = (n, label) => ui.skeleton({ shape: 'row', count: n, label });

/// A pick-one list (language, currency): selectable rows, the chosen one
/// pressed, the code on the trailing edge. `data` carries the binder's key.
export function choiceList(items, { label, cls = 'choices' } = {}) {
  return ui.list(items.map(it => ui.row({ select: true, pressed: !!it.on, title: it.title, data: it.data,
    trailing: `<span class="${ui.cx('choice-code', it.money && 'money')}">${ui.esc(it.code)}</span>`, cls: it.on ? 'on' : '' })),
  { label, cls });
}
