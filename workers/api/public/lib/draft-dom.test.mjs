// node --test workers/api/public/lib/draft-dom.test.mjs
// The browser half of drafts (W-SESSION TZ 7) on the ui dom-shim: typing is
// kept, a form that reopens on the same record gets it back with a note and a
// discard button, another record does not, a save forgets it, a sign-out
// forgets them all.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { render, fire } from './ui/dom-shim.mjs';
import { mountDrafts, draftSaved, forgetAllDrafts, discardDraft } from './draft.js';
import { PREFIX } from './draft-logic.js';

const fakeStorage = () => { const m = new Map(); return { getItem: k => m.has(k) ? m.get(k) : null, setItem: (k, v) => m.set(k, String(v)), removeItem: k => m.delete(k), keys: () => [...m.keys()], m }; };
const T0 = 1_700_000_000_000;
const dishForm = (name, price) => `<div class="sheet-in"><label>n</label><input id="dname" value="${name}"><input id="dprice" type="number" value="${price}">
  <input id="dpass" type="password" value=""><input id="dmail" type="email" value="a@b.c"><textarea id="dnote"></textarea><button id="save" type="button">Save</button></div>`;
const type = (root, id, v) => { const el = root.querySelector('#' + id); el.value = v; fire(el, 'input'); };
const opts = s => ({ app: 'admin', venue: 'v1', scope: 'menu', lang: 'en', storage: s, now: () => T0 });

test('typing is kept under the registered key, and only the fields that moved', () => {
  const s = fakeStorage();
  const { root } = render(dishForm('Nori', '300'));
  mountDrafts(root, opts(s));
  type(root, 'dname', 'Nori roll');
  type(root, 'dpass', 'hunter2');
  type(root, 'dmail', 'x@y.z');
  const keys = s.keys();
  assert.equal(keys.length, 1);
  assert.ok(keys[0].startsWith(PREFIX + 'admin/v1/menu/'), keys[0]);
  const book = JSON.parse(s.getItem(keys[0]));
  const entry = Object.values(book)[0];
  assert.deepEqual(entry.fields, { dname: 'Nori roll' }, 'the password and the e-mail are never kept');
});

test('the same record reopened gets the draft back, with the note and a discard button; another record does not', () => {
  const s = fakeStorage();
  const a = render(dishForm('Nori', '300'));
  mountDrafts(a.root, opts(s));
  type(a.root, 'dname', 'Nori roll');
  type(a.root, 'dnote', 'with sesame');
  // The sheet is torn down by a 401 and re-rendered after sign-in with the server's values.
  const b = render(dishForm('Nori', '300'));
  const restored = mountDrafts(b.root, opts(s));
  assert.equal(restored.length, 1);
  assert.equal(b.root.querySelector('#dname').value, 'Nori roll');
  assert.equal(b.root.querySelector('#dnote').value, 'with sesame');
  const note = b.root.querySelector('.draft-note');
  assert.ok(note, 'a visible note');
  assert.match(note.textContent, /unsaved draft was restored/i);
  // Another dish of the same shape opens clean.
  const c = render(dishForm('Tuna', '400'));
  assert.equal(mountDrafts(c.root, opts(s)).length, 0);
  assert.equal(c.root.querySelector('#dname').value ?? c.root.querySelector('#dname').getAttribute('value'), 'Tuna');
  // Discard: the base values come back, the note goes, the draft is gone for good.
  fire(note.querySelector('[data-draft-discard]'), 'click');
  assert.equal(b.root.querySelector('#dname').value, 'Nori');
  assert.equal(b.root.querySelector('.draft-note'), null);
  const d = render(dishForm('Nori', '300'));
  assert.equal(mountDrafts(d.root, opts(s)).length, 0, 'discarded');
});

test('a successful save after the form\'s own button forgets its draft; an unrelated write does not', () => {
  const s = fakeStorage();
  const { root } = render(dishForm('Nori', '300'));
  mountDrafts(root, opts(s));
  type(root, 'dname', 'Nori roll');
  draftSaved();
  assert.equal(Object.keys(JSON.parse(s.getItem(s.keys()[0]))).length, 1, 'no button pressed here: kept');
  fire(root.querySelector('#save'), 'click');
  draftSaved();
  assert.equal(Object.keys(JSON.parse(s.getItem(s.keys()[0]))).length, 0, 'saved: forgotten');
});

test('typing back to the opening values leaves no draft; a form or field marked data-nodraft is never kept', () => {
  const s = fakeStorage();
  const { root } = render(dishForm('Nori', '300'));
  mountDrafts(root, opts(s));
  type(root, 'dname', 'Nori roll');
  type(root, 'dname', 'Nori');
  assert.equal(Object.keys(JSON.parse(s.getItem(s.keys()[0]))).length, 0);
  const off = render(`<form data-nodraft><input id="q" value=""></form>`);
  mountDrafts(off.root, opts(s));
  type(off.root, 'q', 'secret');
  assert.equal(s.keys().length, 1, 'the opted-out form wrote nothing');
  const fld = render(`<form><input id="ok" value=""><input id="no" data-nodraft value=""></form>`);
  mountDrafts(fld.root, opts(s));
  type(fld.root, 'no', 'x'); type(fld.root, 'ok', 'y');
  const book = JSON.parse(s.getItem(s.keys().find(k => k !== s.keys()[0])));
  assert.deepEqual(Object.values(book)[0].fields, { ok: 'y' });
});

test('a sign-out forgets every draft on the device and nothing else', () => {
  const s = fakeStorage();
  s.setItem('dw_lang', 'uk');
  const { root } = render(dishForm('Nori', '300'));
  mountDrafts(root, opts(s));
  type(root, 'dname', 'x');
  assert.equal(s.keys().length, 2);
  forgetAllDrafts(s);
  assert.deepEqual(s.keys(), ['dw_lang']);
  // discardDraft on a form with nothing to discard is a no-op, not a throw.
  discardDraft(root);
});

test('TZ7: one sheet element re-filled for a second record keeps the SECOND record\'s typing (not an empty book under the first key)', () => {
  const s = fakeStorage();
  const { root } = render(dishForm('Nori', '300'));
  mountDrafts(root, opts(s));
  // The console's #sheetIn is re-filled in place for the next dish and mounted again.
  root.innerHTML = dishForm('Tuna', '400').replace('class="sheet-in"', 'class="sheet-in"');
  mountDrafts(root, opts(s));
  type(root, 'dnote', 'spicy');
  const books = s.keys().map(k => [k, JSON.parse(s.getItem(k))]);
  const kept = books.filter(([, b]) => Object.values(b).some(e => e?.fields?.dnote === 'spicy'));
  assert.equal(kept.length, 1, `the typing is kept under one key (books: ${JSON.stringify(books)})`);
  assert.ok(books.every(([, b]) => Object.keys(b).length > 0), `no empty book is written (books: ${JSON.stringify(books)})`);
  // Reopened on the same record, the text comes back.
  root.innerHTML = dishForm('Tuna', '400');
  const restored = mountDrafts(root, opts(s));
  assert.equal(restored.length, 1);
  assert.equal(root.querySelector('#dnote').value, 'spicy');
});
