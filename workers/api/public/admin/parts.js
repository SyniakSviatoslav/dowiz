// The console's shared pieces on top of /lib/ui: the fragments every admin
// screen repeats (a button named by an i18n key, a labelled field, a select, a
// status pill, a tappable row, a pick chip, the empty and loading states, the
// one checkbox). PURE: options in, escaped HTML out -- no DOM, no clock, no
// network -- so every piece renders in node (`parts.test.mjs`).
//
// WORDS. The console writes its words as `data-t` keys that `retranslate()`
// fills; a component given `k('save')` renders the translated text AND
// `data-t="save"`, so a language switch rewrites it in place as before.
//
// THE CONTRACT with the handlers and the e2e walks is the ids and the `data-*`
// set (`data-act`, `data-o`, `data-open`, ...): every helper passes `id` and
// `data` through unchanged. `tour` becomes `data-tour`, the learning engine's
// anchor (docs/design/BLUEPRINT-LEARNING-VIDEOS-WIKI-2026-09-24.md section 5.1),
// listed in docs/learn/anchors-admin.txt.
//
// Relative import, so node loads the same file the browser does.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.
import * as ui from '../lib/ui/index.js';

export { ui };

/// The i18n key reference the components render with `data-t`.
export const k = key => ({ t: key });

/// Extra classes after a literal `ui-` class (the literal is what the
/// ui-adoption gate reads, so it stays in the template, not in `attrs`).
const more = c => (c ? ' ' + ui.esc(c) : '');

/// A label that may be a key (`key`) or text already in words (`label`).
const words = o => (o.key ? k(o.key) : o.label);

/// `attrs` for a component: the caller's data-* and the anchor.
export function dataOf(o = {}){
  const data = { ...(o.data || {}) };
  if (o.tour) data.tour = o.tour;
  return Object.keys(data).length ? { ...(o.attrs || {}), data: { ...((o.attrs || {}).data || {}), ...data } } : o.attrs;
}

/// A button. `key` is an i18n key, `label` plain words; variant as ui.button
/// (default `secondary`: one `primary` per screen is the caller's decision).
export function btn(o = {}){
  return ui.button({ id: o.id, type: o.type, icon: o.icon, iconEnd: o.iconEnd, label: words(o), variant: o.variant,
    size: o.size, block: o.block, disabled: o.disabled, pressed: o.pressed, href: o.href, target: o.target,
    ariaLabel: o.ariaKey ? k(o.ariaKey) : o.ariaLabel, title: o.title, cls: o.cls, attrs: dataOf(o) });
}

/// An icon-only control (delete on a row, a stepper). `ariaKey` names it.
export function iconBtn(o = {}){
  return ui.iconButton({ id: o.id, icon: o.icon, ariaLabel: o.ariaKey ? k(o.ariaKey) : o.ariaLabel, variant: o.variant,
    disabled: o.disabled, pressed: o.pressed, text: o.text, cls: o.cls, attrs: dataOf(o) });
}

/// A labelled input or textarea. `key` labels it; everything else is ui.field.
/// Types ui.field does not take (date, time, color, datetime-local) go through
/// `input()` below, which wears the same classes.
export function field(o = {}){
  return ui.field({ ...o, label: o.label ?? (o.key ? k(o.key) : undefined), placeholder: o.phKey ? k(o.phKey) : o.placeholder,
    hint: o.hintKey ? k(o.hintKey) : o.hint, attrs: dataOf(o) });
}

const INPUT_TYPES = ['date', 'time', 'color', 'datetime-local', 'month', 'file'];
/// A date, time, colour or file input with the field's label and classes -- the
/// types `ui.field` refuses today (see the lane's hand-back). `hidden` keeps a
/// file picker that a button opens out of the layout.
export function input(o = {}){
  const type = o.type;
  if (!INPUT_TYPES.includes(type)) throw new Error(`admin.input: use field() for ${type}`);
  const id = o.id || ui.uid('f');
  const a = ui.attrs({ id, type, value: o.value == null ? null : String(o.value),
    min: o.min, max: o.max, step: o.step, accept: o.accept, multiple: !!o.multiple, hidden: !!o.hidden,
    required: !!o.required, disabled: !!o.disabled, ...(dataOf(o) || {}) });
  const lab = (o.key || o.label) ? `<label class="ui-label" for="${ui.esc(id)}">${ui.label(words(o))}</label>` : '';
  return `<div class="${ui.cx('ui-field', o.cls)}">${lab}<input class="ui-input${more(o.controlCls)}"${a}></div>`;
}

/// A select, styled as the system's input. `options`: [{ value, label | key }].
/// A key renders `data-t` on the option so `retranslate()` keeps it current.
export function select(o = {}){
  const id = o.id || ui.uid('f');
  const opts = (o.options || []).map(x => {
    const on = String(x.value) === String(o.value);
    const a = ui.attrs({ value: String(x.value), selected: on, disabled: !!x.disabled, 'data-t': x.key || null });
    return `<option${a}>${ui.esc(x.key ? ui.tr(x.key) : x.label)}</option>`;
  }).join('');
  const a = ui.attrs({ id, name: o.name, required: !!o.required, disabled: !!o.disabled,
    'aria-label': (!o.key && !o.label && o.ariaLabel) || null, ...(dataOf(o) || {}) });
  const lab = (o.key || o.label) ? `<label class="ui-label" for="${ui.esc(id)}">${ui.label(words(o))}</label>` : '';
  return `<div class="${ui.cx('ui-field', o.cls)}">${lab}<select class="ui-input${more(o.controlCls)}"${a}>${opts}</select></div>`;
}

/// THE CHECKBOX, once. `ui` has no checkbox or switch yet (hand-back): this is
/// the console's only definition, drawn as the switch the console always had.
/// `hintKey` adds the small line under the words.
export function check(o = {}){
  const a = ui.attrs({ type: 'checkbox', id: o.id, name: o.name, value: o.value, checked: !!o.checked, disabled: !!o.disabled,
    required: !!o.required, ...(dataOf(o) || {}) });
  const hint = o.hintKey ? `<small data-t="${ui.esc(o.hintKey)}">${ui.esc(ui.tr(o.hintKey))}</small>` : '';
  return `<label class="${ui.cx('switch', o.cls)}"><input${a}><span class="switch-k"></span><span class="t">${ui.label(words(o))}${hint}</span></label>`;
}

/// The console's old three-tone pill, as a badge: ok / warn / bad / (none).
const TONE = { ok: 'success', warn: 'warning', bad: 'danger', info: 'info', accent: 'accent' };
export function pill(tone, o = {}){
  return ui.badge({ tone: TONE[tone] || 'neutral', label: words(o), icon: o.icon, id: o.id, cls: o.cls, attrs: dataOf(o) });
}

/// A pill that is also a switch (promo on/off, a key revoked): a chip button.
export function pillBtn(tone, o = {}){
  return ui.chip({ as: 'button', tone: TONE[tone] || 'neutral', label: words(o), icon: o.icon, id: o.id, selected: o.selected,
    cls: o.cls, ariaLabel: o.ariaKey ? k(o.ariaKey) : o.ariaLabel, attrs: dataOf(o) });
}

/// "Nothing here, and why". `alert` for a failure.
export function empty(icon, o = {}){
  return ui.emptyState({ icon, title: o.key ? k(o.key) : o.title, body: o.bodyKey ? k(o.bodyKey) : o.body,
    alert: o.alert, status: o.status, action: o.action, id: o.id, cls: o.cls, attrs: dataOf(o) });
}

/// Before the first answer: the shape of what is coming.
export const loading = (count = 1, shape = 'row') => ui.skeleton({ shape, count, label: k('loading') });

/// A tappable row that OPENS something (a dish, a courier, a supply). `ui.row`
/// offers a selectable row (aria-pressed + a radio mark) or a link; a console
/// row is neither, so it is composed here from the row's own classes, as the
/// room app does (see its HAND-BACK for `row({ act })`).
///   title: text (escaped) or `{ t }`; sub, leading, trailing: MARKUP.
export function rowBtn(o = {}){
  const a = ui.attrs({ type: 'button', id: o.id, disabled: !!o.disabled,
    'aria-label': o.ariaLabel || null, ...(dataOf(o) || {}) });
  const lead = o.leading ? `<span class="ui-row-lead">${o.leading}</span>` : '';
  const title = ui.label(o.title, 'span', 'ui-row-title ui-row-title--strong');
  const sub = o.sub ? `<span class="ui-row-sub">${o.sub}</span>` : '';
  const trail = o.trailing ? `<span class="ui-row-trail">${o.trailing}</span>` : '';
  return `<button class="ui-row ui-row--action${more(o.cls)}"${a}>${lead}<span class="ui-row-body">${title}${sub}</span>${trail}</button>`;
}

/// A row that is NOT a control: the same look, a div. Its actions go in
/// `trailing` (markup, usually buttons).
export function rowDiv(o = {}){
  const a = ui.attrs({ id: o.id, role: 'listitem', ...(dataOf(o) || {}) });
  const lead = o.leading ? `<span class="ui-row-lead">${o.leading}</span>` : '';
  const title = ui.label(o.title, 'span', 'ui-row-title ui-row-title--strong');
  const sub = o.sub ? `<span class="ui-row-sub">${o.sub}</span>` : '';
  const trail = o.trailing ? `<span class="ui-row-trail">${o.trailing}</span>` : '';
  return `<div class="ui-row${more(o.cls)}"${a}>${lead}<span class="ui-row-body">${title}${sub}</span>${trail}</div>`;
}

/// One choice among several, as a selectable row (radio mark, aria-pressed).
export function choice(o = {}){
  return ui.row({ select: true, pressed: !!o.pressed, title: words(o), sub: o.subKey ? k(o.subKey) : o.sub, id: o.id,
    cls: o.cls, attrs: dataOf(o) });
}

/// A few choices as toggle chips; the chosen one is pressed. `values`:
/// [{ value, label | key }]; `attr` is the data-* name the handler reads.
export function chips(o = {}){
  const list = (o.values || []).map(v => ui.chip({ as: 'button', selected: String(v.value) === String(o.value),
    label: v.key ? k(v.key) : v.label, icon: v.icon, attrs: { data: { [o.attr || 'v']: String(v.value), tour: o.tour } } })).join('');
  const lab = o.labelKey ? ` data-t-attr="aria-label:${ui.esc(o.labelKey)}" aria-label="${ui.esc(ui.tr(o.labelKey))}"` : '';
  return `<div class="chips"${o.id ? ` id="${ui.esc(o.id)}"` : ''} role="group"${lab}>${list}</div>`;
}

/// Mark `b` as the chosen one among `all` (chips: aria-pressed).
export function press(all, b){ for (const x of all) x.setAttribute('aria-pressed', String(x === b)); }
