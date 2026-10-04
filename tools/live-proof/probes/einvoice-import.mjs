// LIVE PROBE stock.einvoice_import.v1 (W-OCR P11 fallback) on qa-durres. Main
// runs it after the deploy; a lane never runs it against production.
//
//   node tools/live-proof/probes/einvoice-import.mjs
//
// The UBL parse runs in the owner's browser; what is live is what it feeds.
// 1. the self-made UBL 2.1 sample (fixtures/receipt/einvoice-peshku.xml) parsed by
//    the console's own einvoice-logic.js validates against the contract;
// 2. a supplier card; the first import's lines received + the NIPT remembered;
// 3. GET /api/owner/stock: cardFor() now finds the card BY NIPT, and every line
//    is pre-matched by alias -- the second e-invoice needs no matching at all;
// 4. clean-up.
// A REAL CIS-issued e-invoice needs a real NIPT: that step is NEEDS-KEY (rc 3)
// unless EINVOICE_REAL points at a QA-owned file.
import fs from 'node:fs';
import path from 'node:path';
import { LOC, RUN, own, reporter, validate, shelf } from './stock-lib.mjs';
import * as R from '../../../workers/api/public/admin/receipt-photo-logic.js';
import * as E from '../../../workers/api/public/admin/einvoice-logic.js';

const HERE = path.dirname(new URL(import.meta.url).pathname);
const C = JSON.parse(fs.readFileSync(path.join(HERE, '../contracts/feature-einvoice-import.json'), 'utf8'));
const P = JSON.parse(fs.readFileSync(path.join(HERE, '../contracts/feature-receipt-photo.json'), 'utf8'));
const { step, schema, verdict } = reporter('einvoice-import');
const SUP = `${RUN} Peshku EI`;
const UNIT = { 'salmon fileto norvegjeze': 'g', 'ton i freskët': 'g', 'fletë nori & alga': 'unit' };
let card = null; const made = [];

try {
  const inv = E.parseInvoice(fs.readFileSync(path.join(HERE, '../fixtures/receipt/einvoice-peshku.xml'), 'utf8'));
  schema('the parsed sample matches the contract', inv, C.input_file);
  step('supplier NIPT and three lines', inv.nipt === 'K12345678A' && inv.lines.length === 3);

  const c = await own('/api/owner/stock/supplier', { card: { name: SUP, days: [2], leadDays: 1 } });
  card = c.body?.card?.id || null;
  step('a supplier card', c.status === 200 && !!card, `${c.status} ${c.text.slice(0, 120)}`);
  const confirmed = [];
  for (const l of inv.lines) {
    const id = `${RUN.toLowerCase()}-ei-${made.length}`;
    const s = await own('/api/owner/supplies', { id, name: `${RUN} ${l.text}`, unit: UNIT[l.key], kind: 'food_ingredient', location_id: LOC });
    if (s.status === 200) made.push(id);
    const b = R.receivedBody(l, id, UNIT[l.key], SUP, inv.doc);
    const bad = validate(b.body || {}, P.requests['POST /api/owner/stock/received']);
    step(`"${l.text}" body valid`, !b.error && !bad.length, b.error || bad.join('; '));
    const r = await own('/api/owner/stock/received', b.body);
    step(`"${l.text}" received`, r.status === 200, `${r.status} ${r.text.slice(0, 120)}`);
    confirmed.push({ ...l, item: id });
  }
  const a = await own('/api/owner/stock/alias', R.aliasBody(card, inv.nipt, confirmed, {}));
  step('NIPT + 3 aliases written', a.status === 200 && a.body?.written === 4, `${a.status} ${a.text.slice(0, 200)}`);

  const st = await shelf(own, LOC);
  const found = E.cardFor({ ...inv, supplier: 'renamed by the supplier' }, st.body?.supplierCards, st.body?.supplierAliases);
  step('the next e-invoice finds its card by NIPT alone', found.id === card && found.by === 'nipt', JSON.stringify(found));
  const known = st.body?.supplierAliases?.[card]?.lines;
  step('every line arrives matched', inv.lines.every((l, i) => R.match(l, known, st.supplies).item === confirmed[i].item), JSON.stringify(known));
  const salmon = st.supplies.find(s => s.id === confirmed[0].item);
  step('2.500 kg read as 2500 g, not 2 500 000', salmon?.onHand === 2500, String(salmon?.onHand));

  if (process.env.EINVOICE_REAL) {
    const real = E.parseInvoice(fs.readFileSync(process.env.EINVOICE_REAL, 'utf8'));
    schema('a real CIS e-invoice parses and matches the contract', real, C.input_file);
  } else step('a real CIS-issued e-invoice (needs a real NIPT)', 'NEEDS-KEY', 'set EINVOICE_REAL=<QA-owned XML>');
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  if (card) step('the card is taken off', (await own('/api/owner/stock/supplier', { card: { id: card, name: SUP, gone: true } })).status === 200);
  if (made.length) step('the probe supplies are deleted', (await own('/api/owner/supplies/delete', { ids: made, location_id: LOC, confirmUses: true })).status === 200);
  verdict();
}
