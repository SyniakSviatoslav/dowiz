// node --test workers/api/public/store/taste-held.test.mjs
// W-SNN: the held top-3s in the guest's own export, by name, escaped; nothing when nothing is held.
import test from 'node:test';
import assert from 'node:assert/strict';
import { heldLine } from './taste-held.js';

test('held: both lists by name (id when unnamed), escaped, labelled', () => {
  const html = heldLine({ current: [{ id: 'a', name: 'Maki' }, { id: 'b', name: '<i>x</i>' }], network: [{ id: 'z', name: '' }], temporary: true });
  assert.ok(html.includes('data-t="vk_held"') && html.includes('data-vk-held="1"'));
  assert.ok(html.includes('Maki, &lt;i&gt;x&lt;/i&gt; / z'), html);
});

test('held: nothing held, nothing drawn', () => {
  assert.equal(heldLine(null), '');
  assert.equal(heldLine(undefined), '');
  assert.ok(heldLine({ current: [], network: [] }).includes('- / -'));
});
