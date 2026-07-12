import test from 'node:test';
import assert from 'node:assert';
import { CsvMenuParser } from '../src/lib/csv-parser.js';

test('CsvMenuParser - basic parsing', async () => {
  const parser = new CsvMenuParser();
  const csv = `category_key,category_name,product_key,product_name,price,currency
pizzas,Pizza,margherita,Margherita,8.50,EUR
pizzas,Pizza,pepperoni,Pepperoni,10.00,EUR`;

  const res = await parser.parse({
    kind: 'csv',
    bytes: Buffer.from(csv),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  assert.strictEqual(res.issues.length, 0);
  assert.strictEqual(res.summary.valid, 2);
  assert.strictEqual(res.draft.categories.length, 1);
  assert.strictEqual(res.draft.products.length, 2);
  assert.strictEqual(res.draft.products[0].price, 850);
});

test('CsvMenuParser - csv injection guard', async () => {
  const parser = new CsvMenuParser();
  const csv = `category_key,category_name,product_key,product_name,price,currency
pizzas,Pizza,hack,=1+1+cmd|' /C calc'!A0,8.50,EUR`;

  const res = await parser.parse({
    kind: 'csv',
    bytes: Buffer.from(csv),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  assert.strictEqual(res.issues.length, 1);
  assert.strictEqual(res.issues[0].code, 'POTENTIALLY_UNSAFE_VALUE');
  assert.strictEqual(res.summary.valid, 0);
});

test('CsvMenuParser - currency mismatch', async () => {
  const parser = new CsvMenuParser();
  const csv = `category_key,category_name,product_key,product_name,price,currency
pizzas,Pizza,margherita,Margherita,8.50,ALL`;

  const res = await parser.parse({
    kind: 'csv',
    bytes: Buffer.from(csv),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  assert.strictEqual(res.issues.length, 1);
  assert.strictEqual(res.issues[0].code, 'CURRENCY_MISMATCH');
  assert.strictEqual(res.summary.valid, 0);
});

test('CsvMenuParser - empty rows skipped', async () => {
  const parser = new CsvMenuParser();
  const csv = `category_key,category_name,product_key,product_name,price,currency

pizzas,Pizza,margherita,Margherita,8.50,EUR
`;

  const res = await parser.parse({
    kind: 'csv',
    bytes: Buffer.from(csv),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  assert.strictEqual(res.issues.length, 0);
  assert.strictEqual(res.summary.valid, 1);
});

test('CsvMenuParser - modifier groupings', async () => {
  const parser = new CsvMenuParser();
  const csv = `category_key,category_name,product_key,product_name,price,currency,modifier_group_key,modifier_group_name,modifier_key,modifier_name,modifier_price_delta
pizzas,Pizza,margherita,Margherita,8.50,EUR,size,Size,small,Small,0
pizzas,Pizza,margherita,Margherita,8.50,EUR,size,Size,large,Large,3.00`;

  const res = await parser.parse({
    kind: 'csv',
    bytes: Buffer.from(csv),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  assert.strictEqual(res.issues.length, 0);
  assert.strictEqual(res.draft.products.length, 1);
  assert.strictEqual(res.draft.modifierGroups.length, 1);
  assert.strictEqual(res.draft.modifiers.length, 2);
  assert.strictEqual(res.draft.links.length, 1);
  assert.strictEqual(res.draft.modifiers[1].priceDelta, 300);
});
