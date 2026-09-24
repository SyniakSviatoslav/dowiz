// The room's shared pieces on top of /lib/ui: the few fragments every view of
// the room app repeats (the back bar, a tappable card row, a pick chip, the
// empty and loading states). PURE: options in, escaped HTML out -- no DOM, no
// clock, no network -- so every view that uses them renders in node.
//
// THE CONTRACT with the binders and the e2e walks is the `data-*` set:
// `data-act` (what a tap does), `data-id` / `data-line` / `data-v` (what it
// acts on), `data-form` and the control `name`s. Every helper here passes them
// through unchanged. `data-tour` is the learning engine's anchor
// (docs/design/BLUEPRINT-LEARNING-VIDEOS-WIKI-2026-09-24.md §5.1): an
// attribute only, listed in docs/learn/anchors-room.txt.
//
// Relative import, so node loads the same file the browser does.
import * as ui from '../lib/ui/index.js';

export { ui };

/// The i18n key reference the components render with `data-t`.
export const k = key => ({ t: key });

/// `data` for ui components: the act, and the anchor when one is given.
export const act = (name, extra = {}, tour) => ({ data: { act: name, ...extra, tour } });

/// The bar every sub-view starts with: back on the left, extras on the right.
export function backBar(extra = '') {
  return `<div class="bar">${ui.button({ variant: 'ghost', icon: 'arrow-left', label: k('back'), attrs: act('back', {}, 'nav.back') })}<span class="sp"></span>${extra}</div>`;
}

/// A tappable card: a real button element that NAVIGATES (a table, a round, a dish
/// to add). `ui.row` offers a selectable row (aria-pressed + a radio mark) or a
/// link; a room card is neither, so it is composed here from the row's own
/// classes -- see the lane's HAND-BACK for the `row({ act })` it asks for.
/// `sub` and `trailing` are MARKUP (a status, an amount); `title` is text.
export function actionRow(o) {
  const a = ui.attrs({ type: 'button', disabled: !!o.disabled, 'aria-label': o.ariaLabel || null, ...(o.attrs || {}) });
  const title = `<span class="ui-row-title ui-row-title--strong">${ui.esc(o.title)}</span>`;
  const sub = o.sub ? `<span class="ui-row-sub">${o.sub}</span>` : '';
  const trail = o.trailing ? `<span class="ui-row-trail">${o.trailing}</span>` : '';
  return `<button class="ui-row ui-row--action${o.cls ? ' ' + ui.esc(o.cls) : ''}"${a}><span class="ui-row-body">${title}${sub}</span>${trail}</button>`;
}

/// One choice among a few, as toggle chips that keep the binder's
/// `data-act` and value attribute (`data-v`, or `key`). The group names
/// itself; the chosen chip is pressed.
export function pickChips(name, label, values, current, word, tour, key = 'v') {
  const chips = values.map(v => ui.chip({ as: 'button', selected: v === current, label: word(v),
    attrs: { data: { act: name, [key]: v, tour } } })).join('');
  return `<div class="chips" role="group" aria-label="${ui.esc(label)}">${chips}</div>`;
}

/// Money on screen: the formatted STRING from logic.money, never a number.
export const amt = (s, o) => ui.amount(String(s), o);

/// Before the first answer: the shape of what is coming.
export const loading = (t, shape = 'row', count = 3) => ui.skeleton({ shape, count, label: t('loading') });
