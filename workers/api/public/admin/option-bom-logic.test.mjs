// node --test workers/api/public/admin/option-bom-logic.test.mjs
// The option recipe section (R13, W-LOST), from an answer shaped exactly
// like the hub's (`services/catalogue/option_bom/tests.rs`,
// catalog.modifier_bom.v1): what is drawn, what a save sends, what is refused.
import test from 'node:test';
import assert from 'node:assert/strict';
import { section, collect, bodyOf, dishOf, optionBomPath, lineRow } from './option-bom-logic.js';

const view = {
  contract: 'catalog.modifier_bom.v1', product: 'roll',
  options: [{ group: 'extra', groupName: 'Extras', id: 'xsalmon', name: 'Extra <i>salmon</i>', bom: [{ supply: 'salmon', qty: 20 }] },
    { group: 'extra', groupName: 'Extras', id: 'wasabi', name: 'Wasabi', bom: [] }],
  supplies: [{ id: 'salmon', name: 'Salmon', unit: 'g' }, { id: 'rice', name: 'Rice', unit: 'g' }],
};

test('every option is drawn with its lines, a save button and the anchors', () => {
  const html = section(view);
  for (const k of ['ob_title', 'ob_hint', 'ob_supply', 'ob_qty', 'ob_add', 'ob_save']) assert.ok(html.includes(`data-t="${k}"`), k);
  assert.ok(html.includes('data-tour="dish.optionBom"') && html.includes('data-tour="dish.optionBomSave"'));
  assert.ok(html.includes('data-obsave="xsalmon"') && html.includes('data-obsave="wasabi"'));
  assert.ok(html.includes('value="20"'), 'the stored quantity');
  assert.ok(/<option[^>]*value="salmon"[^>]*selected/.test(html), 'the stored supply is chosen');
  assert.ok(!html.includes('<i>salmon</i>'), 'an option name is text');
  assert.ok(lineRow(view.supplies, {}, 'wasabi', 1).includes('ob-s-wasabi-1'));
});

test('a dish without options says so', () => {
  const html = section({ ...view, options: [] });
  assert.ok(html.includes('ob_none') && !html.includes('data-obsave'));
});

test('a save sends whole positive quantities and skips an empty row', () => {
  assert.deepEqual(collect([{ supply: 'salmon', qty: '20' }, { supply: '', qty: '' }]), { bom: [{ supply: 'salmon', qty: 20 }] });
  assert.deepEqual(collect([]), { bom: [] }, 'no line clears the recipe');
  for (const bad of ['0', '-1', '1.5', 'x', '']) assert.deepEqual(collect([{ supply: 'salmon', qty: bad }]), { error: 'ob_badQty' }, bad);
  assert.deepEqual(bodyOf('v1', 'xsalmon', [{ supply: 'salmon', qty: 20 }]), { location_id: 'v1', option: 'xsalmon', bom: [{ supply: 'salmon', qty: 20 }] });
  assert.equal(optionBomPath('a b'), '/owner/products/a%20b/option-bom');
});

test('the open sheet is the tapped dish, or the one dish with its title', () => {
  const products = [{ id: 'a', name: 'Roll' }, { id: 'b', name: 'Roll' }, { id: 'c', name: 'Maki' }];
  assert.equal(dishOf(products, 'b', 'Roll'), 'b');
  assert.equal(dishOf(products, null, 'Roll'), null, 'two dishes, one title, no tap: unknown');
  assert.equal(dishOf(products, 'a', 'Maki'), 'c', 'a stale tap does not override the title');
  assert.equal(dishOf(products, null, 'Nope'), null);
});
