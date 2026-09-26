// dowiz UI — the core every component is built from.
//
// ONE RULE ABOVE THE OTHERS: every component is a PURE FUNCTION from options to
// an HTML string. The surfaces already build their screens as template literals
// assigned to `innerHTML`, so adopting a component is replacing a fragment of a
// template with a call -- mechanical, reviewable, and testable in node without a
// browser. Behaviour (toasts, sheets, keyboard on a radio group) lives in small
// `bind*`/`create*` functions that take an element and are tested with a shim.
//
// WHAT THIS FILE OWNS
//   esc      -- the one HTML escaper (four byte-identical copies existed)
//   cx       -- class names from parts, falsy parts dropped
//   attrs    -- an attribute string from an object, every value escaped
//   icon     -- the mask icon from /lib/icons.css, name validated
//   label    -- text that may be an i18n KEY: `{ t:'key' }` renders `data-t`
//   useTranslator -- the surface hands its own `t()` in once
//
// CSP: nothing here writes a `style=` attribute or a <style> element. A value
// that must be computed at run time goes through CSSOM at the call site.

export const esc = s => String(s ?? '').replace(/[&<>"']/g, c =>
  ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

/// Class list from parts; `false`, `null`, `undefined` and '' are dropped.
export const cx = (...parts) => parts.flat().filter(Boolean).join(' ');

// ── i18n ─────────────────────────────────────────────────────────────────────
// Each surface has its own dictionary and its own `t(key, vars)`. The system
// does not own words; it owns the HOOK. A component given `{ t:'signIn' }`
// renders the translated text AND `data-t="signIn"`, so the surface's existing
// `retranslate()` rewrites it in place on a language switch. A plain string is
// taken as already translated.
let translate = k => k;
export function useTranslator(fn){ translate = typeof fn === 'function' ? fn : (k => k); }
export const tr = (k, vars) => translate(k, vars);

/// Is `v` an i18n key reference?
const isKey = v => v && typeof v === 'object' && typeof v.t === 'string';

/// Resolve a label to plain text (for attributes, aria-labels, titles).
export function text(v){
  if (v == null || v === false) return '';
  if (isKey(v)) return String(translate(v.t, v.vars) ?? v.t);
  return String(v);
}

/// A label as markup: escaped text, carrying `data-t` when it is a key without
/// variables (a key with variables cannot be re-translated by `data-t`, so it
/// is rendered once and the caller re-renders on a language change).
export function label(v, tag = 'span', cls = ''){
  if (v == null || v === false || v === '') return '';
  const c = cls ? ` class="${esc(cls)}"` : '';
  if (isKey(v) && !v.vars) return `<${tag}${c} data-t="${esc(v.t)}">${esc(text(v))}</${tag}>`;
  return `<${tag}${c}>${esc(text(v))}</${tag}>`;
}

/// An attribute whose value may be an i18n key: returns both the attribute and,
/// for a key, the `data-t-attr` pair that lets `retranslate()` refresh it.
/// Collected by `attrs()` through the `i18n` channel below.
export function i18nAttr(name, v){
  if (v == null || v === false || v === '') return {};
  const out = { [name]: text(v) };
  if (isKey(v) && !v.vars) out['data-t-attr'] = `${name}:${v.t}`;
  return out;
}

// ── attributes ───────────────────────────────────────────────────────────────
// Names are checked, values escaped. `true` renders the bare attribute,
// `false`/`null`/`undefined` renders nothing, `data: { sel:'x' }` renders
// `data-sel="x"`. `style` is REFUSED: `style-src 'self'` drops the attribute in
// the browser, silently, and that is how 62 of them were lost once.
const NAME = /^[a-zA-Z_:][a-zA-Z0-9_:.-]*$/;
export function attrs(obj = {}){
  let out = '';
  const tAttr = [];
  for (const [k, v] of Object.entries(obj)) {
    if (v == null || v === false) continue;
    if (k === 'style') throw new Error('ui: style= is blocked by the CSP; use a class');
    if (k === 'data' && typeof v === 'object') {
      for (const [dk, dv] of Object.entries(v)) {
        if (dv == null || dv === false) continue;
        const name = 'data-' + dk.replace(/[A-Z]/g, m => '-' + m.toLowerCase());
        if (!NAME.test(name)) throw new Error(`ui: bad attribute name ${name}`);
        out += dv === true ? ` ${name}` : ` ${name}="${esc(dv)}"`;
      }
      continue;
    }
    if (!NAME.test(k)) throw new Error(`ui: bad attribute name ${k}`);
    if (/^on/i.test(k)) throw new Error('ui: inline handlers are blocked by the CSP; bind in JS');
    if (k === 'data-t-attr') { tAttr.push(v); continue; }
    out += v === true ? ` ${k}` : ` ${k}="${esc(v)}"`;
  }
  if (tAttr.length) out += ` data-t-attr="${esc(tAttr.join(' '))}"`;
  return out;
}

/// Merge attribute objects; `data-t-attr` values are concatenated, not replaced.
export function merge(...objs){
  const out = {};
  for (const o of objs) for (const [k, v] of Object.entries(o || {})) {
    if (k === 'data-t-attr' && out[k]) out[k] += ' ' + v;
    else if (k === 'data' && out.data) out.data = { ...out.data, ...v };
    else out[k] = v;
  }
  return out;
}

// ── icons ────────────────────────────────────────────────────────────────────
// The name is validated rather than escaped: an icon is a class, and a class
// name with a quote in it is a defect, not a string to be made safe.
const ICON = /^[a-z0-9-]+$/;
export function icon(name, cls = ''){
  if (!name) return '';
  if (!ICON.test(name)) throw new Error(`ui: bad icon name ${name}`);
  return `<i class="${cx('ti', 'ti-' + name, 'ui-i', cls)}" aria-hidden="true"></i>`;
}

// ── ids ──────────────────────────────────────────────────────────────────────
let seq = 0;
/// A unique id for wiring a label to its field when the caller gave none.
export const uid = (p = 'ui') => `${p}-${(++seq).toString(36)}`;

/// The one tone vocabulary. Anything else is refused, so a typo is loud.
export const TONES = ['neutral', 'accent', 'success', 'warning', 'danger', 'info'];
export function tone(v, fallback = 'neutral'){
  if (v == null) return fallback;
  if (!TONES.includes(v)) throw new Error(`ui: unknown tone ${v}`);
  return v;
}

// ── words for codes ──────────────────────────────────────────────────────────
// A person reads "Pending" and "#0001", never `RSST_PENDING` or `#ord_0001`.
// Both leaks were on screen on 2026-09-24: a booking status whose key the
// surface did not have fell through `t()` as the key itself, and an order id
// was drawn with its storage prefix.

/// The short reference a person says out loud: the id without its storage
/// prefix (`ord_`, `ebills:`), first `n` characters, with a `#`.
/// orderRef('ord_0001abcdef') -> '#0001abcd'; orderRef('glovo-7781') -> '#7781'.
export function orderRef(id, n = 8){
  const raw = String(id ?? '').trim();
  if (!raw) return '';
  const bare = raw.replace(/^[a-z]+[_:-](?=[A-Za-z0-9])/, '');
  return '#' + bare.slice(0, n);
}

/// The translated word for an enum CODE: `t(prefix + code)` when the surface
/// has the key, otherwise the code made readable ("NO_SHOW" -> "No show").
/// A `t()` that returns the key unchanged is how a missing word is detected.
export function codeWord(t, prefix, code){
  const c = String(code ?? '');
  if (!c) return '';
  const key = prefix + c;
  const w = typeof t === 'function' ? t(key) : key;
  if (w && w !== key) return w;
  const s = c.toLowerCase().replace(/_+/g, ' ').trim();
  return s.charAt(0).toUpperCase() + s.slice(1);
}
