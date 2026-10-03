// node --test workers/api/public/admin/order-list-logic.test.mjs
// The order a supplier receives, in the supplier's language, and Share with
// copying as the fallback.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as O from './order-list-logic.js';
import { WORDS } from './start-stock-i18n.js';

const w = lang => k => WORDS[lang][k];

test('quantities as a supplier reads them', () => {
  assert.equal(O.qtyText(2000, 'g'), '2 kg');
  assert.equal(O.qtyText(1500, 'g'), '1,5 kg');
  assert.equal(O.qtyText(500, 'ml'), '500 ml');
  assert.equal(O.qtyText(5000, 'ml'), '5 l');
  assert.equal(O.qtyText(12, 'unit'), '12');
});

test('the text, in the supplier language, with whole packs spelled out', () => {
  const lines = [
    { id: 'pack-salmon', name: 'Salmon (fileto)', unit: 'g', qty: 2000, pack: { name: '1 kg', qty: 1000 } },
    { id: 'pack-nori', name: 'Nori (fletë)', unit: 'unit', qty: 0 },
    { id: 'pack-soy', name: 'Salcë soje', unit: 'ml', qty: 1500, pack: { name: '1 l', qty: 1000 } },
  ];
  const sq = O.orderText({ venue: 'Sushi Durrës', supplier: 'Peshku', lines }, w('sq'));
  assert.equal(sq, 'Përshëndetje Peshku!\nPorosi nga Sushi Durrës:\n- Salmon (fileto): 2 kg (2 x 1 kg)\n- Salcë soje: 1,5 l\nFaleminderit!');
  assert.ok(O.orderText({ venue: 'X', supplier: 'Y', lines }, w('uk')).startsWith('Добрий день Y!\nЗамовлення від X:'));
  assert.ok(O.orderText({ venue: 'X', supplier: 'Y', lines }, w('ru')).endsWith('Спасибо!'));
  assert.deepEqual(O.toSend(lines), [{ item: 'pack-salmon', qty: 2000 }, { item: 'pack-soy', qty: 1500 }], 'a zero line is not ordered');
});

test('share where the phone can, copy where it cannot, and say which', async () => {
  const said = [];
  assert.equal(await O.shareOrCopy('t', { share: async o => said.push(o.text) }), 'shared');
  assert.deepEqual(said, ['t']);
  const abort = Object.assign(new Error('no'), { name: 'AbortError' });
  assert.equal(await O.shareOrCopy('t', { share: async () => { throw abort; } }), 'cancelled', 'the owner closed the sheet: nothing marked');
  const copied = [];
  assert.equal(await O.shareOrCopy('t', { clipboard: { writeText: async x => copied.push(x) } }), 'copied');
  assert.equal(await O.shareOrCopy('t', { share: async () => { throw new Error('denied'); }, clipboard: { writeText: async () => {} } }), 'copied', 'a failed share falls back');
  assert.equal(await O.shareOrCopy('t', {}), 'failed');
});
