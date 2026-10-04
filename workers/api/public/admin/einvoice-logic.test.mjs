// node --test workers/api/public/admin/einvoice-logic.test.mjs
// A UBL 2.1 e-invoice file (self-made sample, tools/live-proof/fixtures/receipt)
// -> the same lines the photo sheet confirms; refusals for what it is not.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as E from './einvoice-logic.js';

const SAMPLE = readFileSync(new URL('../../../../tools/live-proof/fixtures/receipt/einvoice-peshku.xml', import.meta.url), 'utf8');

test('the sample: supplier, NIPT, number, date, three exact lines', () => {
  const inv = E.parseInvoice(SAMPLE);
  assert.equal(inv.supplier, 'Peshku i Detit');
  assert.equal(inv.nipt, 'K12345678A', 'the SUPPLIER NIPT, not the buyer one');
  assert.equal(inv.doc, '118/2026');
  assert.equal(inv.date, '2026-10-02');
  assert.equal(inv.lines.length, 3);
  const [a, b, c] = inv.lines;
  assert.deepEqual([a.text, a.key, a.qty, a.unit, a.unit_price, a.total], ['Salmon fileto Norvegjeze', 'salmon fileto norvegjeze', { v: 25, d: 1 }, 'kg', 1800, 4500]);
  assert.deepEqual(b.qty, { v: 12, d: 1 }, 'UBL "1.200" is 1.2: a UBL decimal has no thousands separator');
  assert.equal(c.text, 'Fletë nori & alga', 'entities and UTF-8 survive');
  assert.equal(c.unit, 'copë');
  assert.deepEqual(inv.lines.map(l => l.flags), [[], [], []]);
});

const wrap = body => `<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="c" xmlns:cac="a">${body}</Invoice>`;
const line = (q, unit, amt, name, price = '') => `<cac:InvoiceLine><cbc:InvoicedQuantity unitCode="${unit}">${q}</cbc:InvoicedQuantity><cbc:LineExtensionAmount currencyID="ALL">${amt}</cbc:LineExtensionAmount><cac:Item><cbc:Name>${name}</cbc:Name></cac:Item>${price}</cac:InvoiceLine>`;

test('refused: not an invoice, a credit note, another currency, broken XML, an entity bomb', () => {
  assert.throws(() => E.parseInvoice('<Order/>'), /ei_notInvoice/);
  assert.throws(() => E.parseInvoice('<CreditNote></CreditNote>'), /ei_credit/);
  assert.throws(() => E.parseInvoice(wrap(`<cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode>${line('1', 'KGM', '10.00', 'x')}`)), /ei_currency/);
  assert.throws(() => E.parseInvoice('<Invoice><cbc:ID>1</cbc:IDX></Invoice>'), /ei_bad/);
  assert.throws(() => E.parseInvoice('<?xml version="1.0"?><!DOCTYPE x [<!ENTITY a "aaaa">]><Invoice/>'), /ei_bad/);
  assert.throws(() => E.parseInvoice(wrap('')), /ei_line/);
  assert.throws(() => E.parseInvoice(wrap(line('', 'KGM', '10.00', 'x'))), /ei_line/);
});

test('qindarka are rounded and flagged; an unknown unit code is flagged; a per-10 price is not a unit price', () => {
  const inv = E.parseInvoice(wrap(line('3', 'XYZ', '100.50', 'Limon', '<cac:Price><cbc:PriceAmount>335.00</cbc:PriceAmount><cbc:BaseQuantity>10</cbc:BaseQuantity></cac:Price>')));
  assert.equal(inv.lines[0].total, 101);
  assert.deepEqual(inv.lines[0].flags, ['rounded', 'unit']);
  assert.equal(inv.lines[0].unit_price, null);
});

test('a PDF that embeds the XML uncompressed gives it back; one without gives nothing', () => {
  const latin1 = Buffer.from(`%PDF-1.7\n1 0 obj<</Type/EmbeddedFile>>stream\n${SAMPLE}\nendstream\nendobj\n%%EOF`, 'utf8').toString('latin1');
  const x = E.xmlInPdf(latin1);
  assert.ok(x.startsWith('<?xml'));
  assert.equal(E.parseInvoice(x).lines[1].text, 'Ton i freskët', 'the UTF-8 bytes are decoded, not read as latin-1');
  assert.equal(E.xmlInPdf('%PDF-1.4 just a picture %%EOF'), '');
});

test('the supplier card: by the NIPT an earlier import stored, else by name', () => {
  const inv = E.parseInvoice(SAMPLE);
  const cards = [{ id: 'peshku', name: 'Peshku' }, { id: 'pd', name: 'peshku i detit' }];
  assert.deepEqual(E.cardFor(inv, cards, { peshku: { nipt: 'K12345678A', lines: {} } }), { id: 'peshku', by: 'nipt' });
  assert.deepEqual(E.cardFor(inv, cards, {}), { id: 'pd', by: 'name' });
  assert.deepEqual(E.cardFor(inv, [], {}), { id: '', by: '' });
});
