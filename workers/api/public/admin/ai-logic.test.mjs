// node --test workers/api/public/admin/ai-logic.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as L from './ai-logic.js';

test('save writes the switch, the provider, the endpoint and the model, and the key only when typed', () => {
  const w = L.saveWrites({ enabled: true, mode: 'own', endpoint: ' https://openrouter.ai/api/v1 ', model: 'm', key: '' });
  assert.deepEqual(w, [
    { key: 'ai.enabled', value: '1' }, { key: 'ai.provider', value: 'own' },
    { key: 'ai.endpoint', value: 'https://openrouter.ai/api/v1' }, { key: 'ai.model', value: 'm' },
  ]);
  const k = L.saveWrites({ enabled: false, mode: 'nonsense', endpoint: '', model: '', key: ' sk-or-1 ' });
  assert.equal(k[1].value, 'auto', 'a word outside the closed set is never sent');
  assert.deepEqual(k[4], { key: 'ai.token', value: 'sk-or-1' });
});

test('the OpenRouter preset fills the address and a free model, and never a key', () => {
  const f = L.withOpenRouter({ enabled: true, mode: 'workers', endpoint: '', model: '', key: '' });
  assert.equal(f.endpoint, 'https://openrouter.ai/api/v1');
  assert.match(f.model, /:free$/);
  assert.equal(f.mode, 'own', 'workers-only would ignore the key the owner is about to paste');
  assert.equal(f.key, '');
  assert.equal(L.withOpenRouter({ mode: 'auto' }).mode, 'auto');
});

test('the meter', () => {
  assert.deepEqual(L.meter({ used: 150, cap: 300 }), { used: 150, cap: 300, left: 150, pct: 50 });
  assert.deepEqual(L.meter({ used: 900, cap: 300 }), { used: 900, cap: 300, left: 0, pct: 100 });
  assert.deepEqual(L.meter(null), { used: 0, cap: 0, left: 0, pct: 0 });
});

test('a test state and a skip reason each have words; an unknown one is a failure, never ok', () => {
  assert.equal(L.stateKey('ok'), 'ai_state_ok');
  assert.equal(L.stateKey('needs-key'), 'ai_state_needs_key');
  assert.equal(L.stateKey('something new'), 'ai_state_failed');
  assert.equal(L.whyKey('budget-spent'), 'ai_why_budget');
});

test('a number leads to its source route and each day of records, as console paths', () => {
  const tr = L.trail({ source: '/api/owner/analytics?days=7', trace: ['/api/owner/analytics?trace=2026-09-28'] });
  assert.deepEqual(tr, { source: '/owner/analytics?days=7', days: ['/owner/analytics?trace=2026-09-28'] });
  assert.deepEqual(L.trail(null), { source: '', days: [] });
});

test('a question is trimmed and bounded', () => {
  assert.equal(L.clean('  what   sold?  '), 'what sold?');
  assert.equal(L.clean(''), null);
  assert.equal(L.clean('x'.repeat(401)), null);
});
