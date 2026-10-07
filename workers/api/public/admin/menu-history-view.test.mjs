// menu-history-view.js renders the hub's answer (W-PITR): a dish edit with its
// restore button, the journal's own marks as words, and the empty sentence.
// Run: node --test workers/api/public/admin/menu-history-view.test.mjs
import test from 'node:test';
import assert from 'node:assert/strict';
import { drawHistory, row, who, change } from './menu-history-view.js';
import { WORDS } from './menu-history-words.js';

const edit = { seq: 7, at: 1790000000000, by: 'usr_owner', key: 'product:futomaki', kind: 'product', id: 'futomaki', name: 'Futomaki', price: 1200, available: true, added: false, removed: false, restorable: true };

test('a dish edit draws its name, price, editor and the restore button with its seq and key', () => {
  const html = row(edit, { money: n => `${n} L`, when: () => 'today' });
  assert.match(html, /Futomaki/);
  assert.match(html, /1200 L/);
  assert.match(html, /usr_owner/);
  assert.match(html, /data-mh-seq="7"/);
  assert.match(html, /data-mh-key="product:futomaki"/);
  assert.match(html, /data-t="mh_changed"/);
});

test('an edit the hub does not offer to restore has no button', () => {
  assert.doesNotMatch(row({ ...edit, restorable: false }), /data-mh-seq/);
});

test('the journal marks read as words, a removal says removed', () => {
  assert.deepEqual(who('?'), { key: 'mh_unseen' });
  assert.deepEqual(who('baseline'), { key: 'mh_baseline' });
  assert.equal(change({ removed: true, added: false }), 'mh_removed');
  assert.match(row({ ...edit, by: '?' }), /data-t="mh_unseen"/);
});

test('a name with markup is escaped, never drawn', () => {
  assert.doesNotMatch(row({ ...edit, name: '<img src=x onerror=1>' }), /<img/);
});

test('no edits draws the empty sentence; every key exists in every language', () => {
  assert.match(drawHistory({ edits: [] }), /data-t="mh_none"/);
  const keys = Object.keys(WORDS.en);
  for (const l of Object.keys(WORDS)) assert.deepEqual(Object.keys(WORDS[l]).sort(), [...keys].sort(), l);
  const used = new Set([...drawHistory({ edits: [edit, { ...edit, kind: 'location', by: 'baseline', added: true }] }, {}).matchAll(/data-t="([^"]+)"/g)].map(m => m[1]));
  for (const k of used) if (k !== 'tabMenu') assert.ok(k in WORDS.en, k);
});
