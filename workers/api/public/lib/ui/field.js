// Form fields: a label, a control, an optional hint and an optional error, wired
// together so assistive technology reads them as one thing.
//
//   * the <label for> always points at the control (a placeholder is not a label)
//   * hint and error are joined into aria-describedby
//   * an error sets aria-invalid and is role="alert", so it is announced
//   * the control is 16px or larger (ui.css) -- iOS zooms on anything smaller,
//     and the old fix for that took pinch-zoom away from everyone
//
// `money: true` gives the field the tabular mono face of `.money` and a numeric
// keypad; the VALUE is still an integer in minor units, parsed by the caller.
import { esc, cx, attrs, merge, icon, label, text, i18nAttr, uid, tone } from './core.js';

const TYPES = ['text', 'email', 'tel', 'password', 'number', 'search', 'url'];

/// @param o { id, label, type='text', value, placeholder, hint, error, rows
///            (textarea when set), money, required, disabled, autocomplete,
///            inputmode, enterkeyhint, maxlength, minlength, pattern, cls, attrs }
export function field(o = {}){
  const id = o.id || uid('f');
  const type = o.type || 'text';
  if (!o.rows && !TYPES.includes(type)) throw new Error(`ui.field: unknown type ${type}`);
  const hintId = o.hint ? `${id}-hint` : null;
  const errId = o.error ? `${id}-err` : null;
  const describedBy = [hintId, errId].filter(Boolean).join(' ') || null;
  const control = merge(
    { id, name: o.name, class: cx('ui-input', o.money && 'ui-input--money money', o.controlCls),
      required: !!o.required, disabled: !!o.disabled,
      autocomplete: o.autocomplete, inputmode: o.inputmode ?? (o.money ? 'numeric' : null),
      enterkeyhint: o.enterkeyhint, maxlength: o.maxlength, minlength: o.minlength,
      pattern: o.pattern ?? (o.money ? '[0-9]*' : null),
      autocapitalize: o.autocapitalize, spellcheck: o.spellcheck == null ? null : String(o.spellcheck),
      'aria-describedby': describedBy, 'aria-invalid': o.error ? 'true' : null },
    i18nAttr('placeholder', o.placeholder),
    o.attrs,
  );
  const el = o.rows
    ? `<textarea${attrs(merge(control, { rows: o.rows }))}>${esc(o.value ?? '')}</textarea>`
    : `<input${attrs(merge({ type }, control, { value: o.value == null ? null : String(o.value) }))}>`;
  const lab = o.label ? `<label class="ui-label" for="${esc(id)}">${label(o.label)}</label>` : '';
  const hint = o.hint ? `<p class="ui-hint" id="${esc(hintId)}">${label(o.hint)}</p>` : '';
  const err = o.error ? `<p class="ui-field-err" id="${esc(errId)}" role="alert">${icon('alert-circle')}${label(o.error)}</p>` : '';
  return `<div class="${cx('ui-field', o.error && 'ui-field--invalid', o.cls)}">${lab}${el}${hint}${err}</div>`;
}

/// A field with a trailing action in one row: the ask box and its send button.
/// `action` is markup (usually `iconButton(...)`); the input keeps its label
/// visually hidden when `hideLabel` is set -- it is still read.
export function inputRow(o = {}){
  const id = o.id || uid('f');
  const a = merge(
    { id, type: o.type || 'text', class: 'ui-input', autocomplete: o.autocomplete ?? 'off',
      enterkeyhint: o.enterkeyhint, 'aria-label': o.label ? text(o.label) : null },
    i18nAttr('placeholder', o.placeholder),
    o.attrs,
  );
  return `<div class="${cx('ui-inputrow', o.cls)}"><input${attrs(a)}>${o.action || ''}</div>`;
}

/// An inline message that is not attached to one field: a failed sign-in, a
/// refused request. `tone` danger is announced (role=alert); others are status.
export function alert(o = {}){
  const tn = tone(o.tone, 'danger');
  const role = tn === 'danger' ? 'alert' : 'status';
  const ic = o.icon ?? (tn === 'danger' ? 'alert-circle' : tn === 'success' ? 'circle-check' : 'info-circle');
  return `<div class="${cx('ui-alert', `ui-alert--${tn}`, o.cls)}" role="${role}">${icon(ic)}${label(o.label)}</div>`;
}
