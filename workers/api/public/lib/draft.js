// DRAFTS IN THE BROWSER (W-SESSION, operator TZ 7, 2026-10-12): the DOM half of
// `draft-logic.js`. An app's bootstrap calls `mountDrafts(root, …)` after it
// renders a screen or a sheet; from then on every keystroke in that root is
// kept on the device, a form that reopens on the same record gets the draft
// back with a visible note and a discard button, the form's own save forgets
// it, and an explicit sign-out forgets them all (a 401 does NOT: that is the
// whole point). What is never kept is `draft-logic.js`'s rule.
//
// GENERIC BY DESIGN. Nothing here knows a dish from a supplier: the form is
// whatever holds the fields (a <form>, a [data-draft] block, or the root the
// app rendered into), the record is the values it opened with. A screen opts
// out of all of it with `data-nodraft` on the form, a field with the same.

import * as L from './draft-logic.js';
import { DRAFT_WORDS } from './draft-words.js';

/// localStorage throws in private mode / blocked-site-data: never let that
/// break the page. Same shape as store/storage.js, kept here so this file
/// has no app import.
const safe = {
  getItem(k) { try { return localStorage.getItem(k); } catch { return null; } },
  setItem(k, v) { try { localStorage.setItem(k, v); } catch {} },
  removeItem(k) { try { localStorage.removeItem(k); } catch {} },
  keys() { try { return Object.keys(localStorage); } catch { return []; } },
};

const FIELDS = 'input,textarea,select';
/// What the app told us about each mounted form.
const mounted = new WeakMap();
/// The form whose own button was pressed last: the one a successful write saves.
let armed = null;

const isCheck = el => el.tagName === 'INPUT' && /^(checkbox|radio)$/i.test(el.getAttribute('type') || '');
function read(el) {
  if (isCheck(el)) return (el.checked ?? el.hasAttribute('checked')) ? 'on' : '';
  if (el.tagName === 'TEXTAREA') return String(('value' in el ? el.value : null) ?? el.textContent ?? '');
  return String(('value' in el ? el.value : null) ?? el.getAttribute('value') ?? '');
}
function write(el, v) {
  if (isCheck(el)) { el.checked = v === 'on'; return; }
  el.value = v;
  if (el.tagName === 'TEXTAREA') el.textContent = v; else el.setAttribute('value', v);
}

/// The fields a draft may hold, each with the name it is kept under.
export function fieldsIn(container) {
  const out = [];
  container.querySelectorAll(FIELDS).forEach((el, i) => {
    if (el.closest('[data-nodraft]') && el.closest('[data-nodraft]') !== container.closest('[data-nodraft]')) return;
    const ok = L.collectable({ tag: el.tagName, type: el.getAttribute('type'), autocomplete: el.getAttribute('autocomplete'), nodraft: el.hasAttribute('data-nodraft') });
    if (!ok) return;
    out.push({ el, name: el.getAttribute('name') || el.getAttribute('id') || '#' + i });
  });
  return out;
}
const values = fields => Object.fromEntries(fields.map(f => [f.name, read(f.el)]));

function note(container, lang, onDiscard) {
  const w = DRAFT_WORDS[lang] || DRAFT_WORDS.en;
  const doc = container.ownerDocument || document;
  const n = doc.createElement('div');
  n.className = 'ui-alert ui-alert--info draft-note';
  n.setAttribute('role', 'status');
  n.innerHTML = `<i class="ti ti-history" aria-hidden="true"></i><span class="draft-note-text"></span>` +
    `<button type="button" class="ui-btn ui-btn--ghost ui-btn--sm" data-draft-discard></button>`;
  n.querySelector('.draft-note-text').textContent = w.draftRestored;
  const b = n.querySelector('[data-draft-discard]');
  b.textContent = w.draftDiscard;
  b.addEventListener('click', e => { e.preventDefault?.(); onDiscard(); });
  if (container.firstChild && container.insertBefore) container.insertBefore(n, container.firstChild);
  else container.appendChild(n);
  return n;
}

/// Mount every form under `root`. Returns the containers whose draft came back.
export function mountDrafts(root, { app, venue = '', scope = '', lang = 'en', storage = safe, now = () => Date.now() } = {}) {
  if (!root) return [];
  let containers = [...root.querySelectorAll('form,[data-draft]')];
  if (!containers.length) containers = [root];
  const restored = [];
  for (const c of containers) {
    if (c.hasAttribute('data-nodraft') || c.closest('[data-nodraft]') || mounted.has(c)) continue;
    const fields = fieldsIn(c);
    if (!fields.length) continue;
    const key = L.keyOf(app, venue, c.getAttribute('data-draft') || scope || c.id || '', L.shapeOf(fields.map(f => f.name)));
    const base = values(fields), baseHash = L.hashOf(base);
    const m = { key, base, baseHash, fields, storage, now, note: null };
    mounted.set(c, m);
    const saved = L.recall(L.parseBook(storage.getItem(key)), baseHash, now());
    if (saved) {
      for (const f of fields) if (f.name in saved) write(f.el, saved[f.name]);
      m.note = note(c, lang, () => discardDraft(c));
      restored.push(c);
    }
    const save = () => {
      const d = L.diff(m.base, values(m.fields));
      const book = L.parseBook(storage.getItem(key));
      storage.setItem(key, JSON.stringify(d ? L.remember(book, baseHash, d, now()) : L.forget(book, baseHash)));
    };
    c.addEventListener('input', save);
    c.addEventListener('change', save);
    c.addEventListener('click', e => {
      const t = e.target;
      if (!t?.closest) return;
      if (t.closest('[data-draft-discard]')) return;
      if (t.closest('button,[type="submit"],[data-act]')) armed = c;
    });
    c.addEventListener('submit', () => { armed = c; });
  }
  return restored;
}

/// The form's own values come back and the draft is gone for good.
export function discardDraft(container) {
  const m = mounted.get(container);
  if (!m) return;
  m.storage.setItem(m.key, JSON.stringify(L.forget(L.parseBook(m.storage.getItem(m.key)), m.baseHash)));
  for (const f of m.fields) if (f.name in m.base) write(f.el, m.base[f.name]);
  if (m.note) { m.note.remove(); m.note = null; }
}

/// The app's write landed (a 2xx on a POST): the form whose button sent it
/// has nothing left to keep. Called by each app's fetch wrapper.
export function draftSaved() {
  const c = armed;
  armed = null;
  if (!c || !mounted.has(c)) return;
  const m = mounted.get(c);
  m.storage.setItem(m.key, JSON.stringify(L.forget(L.parseBook(m.storage.getItem(m.key)), m.baseHash)));
  if (m.note) { m.note.remove(); m.note = null; }
}

/// An EXPLICIT sign-out: the next person at this device is not the author.
export function forgetAllDrafts(storage = safe) {
  const keys = typeof storage.keys === 'function' ? storage.keys() : Object.keys(storage);
  for (const k of keys) if (String(k).startsWith(L.PREFIX)) storage.removeItem(k);
}

/// Does this device hold any draft for this app and venue? (A toast after
/// a re-login says so.)
export function hasDrafts(app, venue, storage = safe, now = () => Date.now()) {
  const head = L.keyOf(app, venue, '', '').replace(/\/+$/, '/');
  const keys = typeof storage.keys === 'function' ? storage.keys() : Object.keys(storage);
  return keys.some(k => String(k).startsWith(head) && Object.values(L.parseBook(storage.getItem(k))).some(e => e && now() - (e.at || 0) < L.KEEP_MS));
}
