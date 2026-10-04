// node --test workers/api/public/admin/receipt-photo-logic.test.mjs
// An invoice photo's OCR text -> lines, numbers in both conventions, whole lek,
// the cross-check, matching by alias then by name, and the requests built.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as R from './receipt-photo-logic.js';

const val = n => n && n.v / 10 ** n.d;

test('numbers: Albanian and English forms, exact, never a float', () => {
  assert.deepEqual(R.num('1.234,50'), { v: 12345, d: 1 }, 'the trailing zero carries nothing');
  assert.deepEqual(R.num('1,234.50'), { v: 12345, d: 1 });
  assert.equal(val(R.num('1 234')), 1234);
  assert.equal(val(R.num('2,5')), 2.5);
  assert.equal(val(R.num('2.5')), 2.5);
  assert.equal(val(R.num('450,00')), 450);
  // Three digits after one separator group thousands -- and keep the other reading.
  assert.equal(val(R.num('1.800')), 1800);
  assert.equal(val(R.num('2,500').alt), 2.5);
  assert.equal(R.num('abc'), null);
  assert.equal(R.num('1.2.3,4,5'), null);
  assert.equal(R.num(''), null);
});

test('money is whole lek, half up, and says when qindarka were dropped', () => {
  assert.deepEqual(R.lek(R.num('4.500')), { lek: 4500, rounded: false });
  assert.deepEqual(R.lek(R.num('120,50')), { lek: 121, rounded: true });
  assert.deepEqual(R.lek(R.num('120,49')), { lek: 120, rounded: true });
});

test('a line: words, qty, unit, unit price, total', () => {
  const l = R.parseLine('1 Salmon fileto Norvegjeze kg 2,500 1.800 4.500');
  assert.equal(l.text, 'Salmon fileto Norvegjeze');
  assert.equal(l.key, 'salmon fileto norvegjeze');
  assert.equal(val(l.qty), 2.5, 'the cross-check chose the kilogram reading');
  assert.equal(l.unit, 'kg');
  assert.equal(l.unit_price, 1800);
  assert.equal(l.total, 4500);
  assert.deepEqual(l.flags, []);
});

test('English invoice, a unit glued to the number, lek suffixes', () => {
  const a = R.parseLine('Cream cheese 1,5 kg pcs 4 1,450.00 5,800.00');
  assert.equal(a.text, 'Cream cheese 1,5 kg');
  assert.equal(a.unit, 'pcs');
  assert.equal(val(a.qty), 4);
  assert.equal(a.total, 5800);
  const b = R.parseLine('Kastravec 3kg 80 L 240 L');
  assert.equal(b.text, 'Kastravec');
  assert.equal(b.unit, 'kg');
  assert.equal(b.unit_price, 80);
  assert.equal(b.total, 240);
  const c = R.parseLine('Cola 0,33 l 24 80 1.920');
  assert.equal(c.text, 'Cola 0,33 l', 'a litre left of the numbers stays in the words');
  assert.equal(val(c.qty), 24);
});

test('the cross-check flags a misread digit', () => {
  const l = R.parseLine('Ton i freskët kg 1,2 3.200 3.940');
  assert.deepEqual(l.flags, ['mismatch']);
  const n = R.parseLine('Limon 30 750');
  assert.deepEqual(n.flags, ['no_price']);
  assert.equal(n.unit_price, null);
});

test('sums, taxes and headers are not lines', () => {
  for (const s of ['Totali pa TVSH 13.740', 'TVSH 20% 2.748', 'Gjithsej 16.488', 'NIPT: K12345678A', 'Subtotal 15,475.00', '', 'Rruga Tregtare 12, Durrës', 'Nr Përshkrimi Njësia Sasia Çmimi Vlera'])
    assert.equal(R.parseLine(s), null, s);
});

test('the whole text: NIPT, invoice number, lines', () => {
  const p = R.parseText('Peshku i Detit sh.p.k.\nNIPT: K12345678A\nFatura nr. 118/2026\nData: 02.10.2026\n1 Fletë nori copë 20 150 3.000\nGjithsej 3.600');
  assert.equal(p.nipt, 'K12345678A');
  assert.equal(p.doc, '118/2026');
  assert.equal(p.lines.length, 1);
  assert.equal(p.lines[0].unit, 'copë');
  assert.equal(R.parseLine('4 Fleté nori copé 20 150 3.000').text, 'Fleté nori', 'the OCR misreading of copë is still a unit');
  assert.equal(R.parseText('Invoice No. INV-2041').doc, 'INV-2041');
});

const SUP = [{ id: 'salmon', name: 'Salmon', unit: 'g' }, { id: 'nori', name: 'Nori sheets', unit: 'unit' }, { id: 'tuna', name: 'Tuna', unit: 'g' }];

test('matching: the alias first, then a shared word, else nothing', () => {
  const l = R.parseLine('Ton i freskët kg 1,2 3.200 3.840');
  assert.deepEqual(R.match(l, {}, SUP), { item: '', by: '' });
  assert.deepEqual(R.match(l, { 'ton i freskët': 'tuna' }, SUP), { item: 'tuna', by: 'alias' });
  assert.deepEqual(R.match(l, { 'ton i freskët': 'gone' }, SUP), { item: '', by: '' }, 'an alias to a removed supply is not used');
  assert.deepEqual(R.match(R.parseLine('Fletë nori copë 20 150 3.000'), {}, SUP), { item: 'nori', by: 'name' });
});

test('the receipt body: base units, the invoice total, the paper', () => {
  const l = R.parseLine('Salmon fileto kg 2,500 1.800 4.500');
  assert.deepEqual(R.receivedBody(l, 'salmon', 'g', 'Peshku i Detit', '118/2026').body, { item: 'salmon', qty: 2500, total: 4500, supplier: 'Peshku i Detit', doc: '118/2026' });
  assert.equal(R.receivedBody(l, 'nori', 'unit').error, 'rp_badQty', 'kilograms onto a piece supply');
  assert.equal(R.receivedBody(l, '', 'g').error, 'rp_pickSupply');
  assert.equal(R.toBase({ v: 15, d: 4 }, 'kg', 'g'), null, 'a fraction of a gram is refused, not rounded');
  assert.equal(R.toBase({ v: 20, d: 0 }, 'copë', 'unit'), 20);
});

test('the alias body carries only what the owner changed', () => {
  const confirmed = [{ key: 'salmon fileto', item: 'salmon' }, { key: 'ton i freskët', item: 'tuna' }, { key: 'x', item: '' }];
  assert.deepEqual(R.aliasBody('peshku', 'K12345678A', confirmed, { 'salmon fileto': 'salmon' }),
    { card: { supplier: 'peshku', nipt: 'K12345678A', lines: [{ text: 'ton i freskët', item: 'tuna' }] } });
  assert.deepEqual(R.aliasBody('p', '', [], {}), { card: { supplier: 'p', lines: [] } });
});

test('the alias key is the Worker rule (aliases.rs key)', () => {
  assert.equal(R.aliasKey('  Salmon   FILETO\tNorvegjeze '), 'salmon fileto norvegjeze');
  assert.equal(R.aliasKey('FLETË Nori'), 'fletë nori');
});

test('edits count what the owner must fix', () => {
  const want = [{ text: 'Ton i freskët', item: 'tuna', qty: 1.2, unit_price: 3200, total: 3840 }];
  const got = [{ ...R.parseLine('Ton i freskët kg 1,2 3.200 3.840'), item: '' }];
  assert.deepEqual(R.edits(got, want), { edits: 1, exact: 0, of: 1 });
  got[0].item = 'tuna';
  assert.deepEqual(R.edits(got, want), { edits: 0, exact: 1, of: 1 });
  assert.deepEqual(R.edits([], want), { edits: 4, exact: 0, of: 1 });
});
