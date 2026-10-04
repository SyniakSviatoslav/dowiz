// LIVE PROBE stock.receipt_from_photo.v1 (W-OCR P10) on qa-durres. Main runs
// it after the deploy; a lane never runs it against production.
//
//   node tools/live-proof/probes/receipt-photo.mjs
//
// The OCR runs in the owner's browser, so the live surface is:
// 1. the reader's files are SERVED: every file under /lib/ocr answers 200 with
//    the sha256 pinned in workers/api/public/lib/ocr/README.md, and the console's
//    CSP carries 'wasm-unsafe-eval' (without it the core cannot compile);
// 2. a fixture invoice's OCR text (the text tesseract read off
//    fixtures/receipt/inv1-peshku-sq.png) parsed by the console's own
//    receipt-photo-logic.js into receipt bodies that validate against the contract;
// 3. a supplier card, then each confirmed line through POST /api/owner/stock/received,
//    then POST /api/owner/stock/alias; GET /api/owner/stock shows the priced
//    receipt (supplier, doc) and the aliases;
// 4. the same alias request again writes nothing; an unknown field is refused (strict body);
// 5. clean-up: the card gone, the probe supplies deleted.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { HOST, LOC, RUN, own, reporter, validate, shelf } from './stock-lib.mjs';
import * as R from '../../../workers/api/public/admin/receipt-photo-logic.js';

const HERE = path.dirname(new URL(import.meta.url).pathname);
const C = JSON.parse(fs.readFileSync(path.join(HERE, '../contracts/feature-receipt-photo.json'), 'utf8'));
const { step, schema, verdict } = reporter('receipt-photo');
const OCR = path.join(HERE, '../../../workers/api/public/lib/ocr');
const SUP = `${RUN} Peshku`;
const ITEMS = { salmon: `${RUN.toLowerCase()}-rp-salmon`, nori: `${RUN.toLowerCase()}-rp-nori` };
// What the reader returns for two lines of the fixture (kept as text so the probe needs no OCR).
const TEXT = `${SUP}\nNIPT: K12345678A\nFatura nr. ${RUN}-118\n1 Salmon fileto Norvegjeze kg 2,500 1.800 4.500\n2 Fletë nori copë 20 150 3.000\nGjithsej 9.000`;
let card = null; const made = [];

try {
  // ── 1. the reader's files, as served ──
  const pinned = Object.fromEntries([...fs.readFileSync(path.join(OCR, 'README.md'), 'utf8').matchAll(/^\| ([\w.-]+\.(?:js|wasm|gz)) \|.*\| ([0-9a-f]{64}) \|$/gm)].map(m => [m[1], m[2]]));
  step('the README pins the reader files', Object.keys(pinned).length >= 8, Object.keys(pinned).join(' '));
  for (const [f, sha] of Object.entries(pinned)) {
    const r = await fetch(`${HOST}/lib/ocr/${f}`);
    const buf = Buffer.from(await r.arrayBuffer());
    step(`/lib/ocr/${f} is served as pinned`, r.status === 200 && crypto.createHash('sha256').update(buf).digest('hex') === sha, `${r.status} ${buf.length} B`);
  }
  const page = await fetch(`${HOST}/admin/`);
  const csp = page.headers.get('content-security-policy') || '';
  step("the console's CSP lets WebAssembly compile", /script-src[^;]*'wasm-unsafe-eval'/.test(csp), csp.slice(0, 160));

  // ── 2. the parse ──
  const p = R.parseText(TEXT);
  step('two lines and the paper are read', p.lines.length === 2 && p.nipt === 'K12345678A' && p.doc === `${RUN}-118`, JSON.stringify(p).slice(0, 300));

  // ── 3. card, supplies, receipt, aliases ──
  const c = await own('/api/owner/stock/supplier', { card: { name: SUP, days: [1], leadDays: 1 } });
  card = c.body?.card?.id || null;
  step('a supplier card', c.status === 200 && !!card, `${c.status} ${c.text.slice(0, 120)}`);
  for (const [k, id] of Object.entries(ITEMS)) {
    const s = await own('/api/owner/supplies', { id, name: `${RUN} ${k}`, unit: k === 'nori' ? 'unit' : 'g', kind: 'food_ingredient', location_id: LOC });
    if (s.status === 200) made.push(id);
    step(`a supply ${k}`, s.status === 200, `${s.status} ${s.text.slice(0, 100)}`);
  }
  const confirmed = [{ ...p.lines[0], item: ITEMS.salmon }, { ...p.lines[1], item: ITEMS.nori }];
  for (const l of confirmed) {
    const b = R.receivedBody(l, l.item, l.item === ITEMS.nori ? 'unit' : 'g', SUP, p.doc);
    const bad = validate(b.body || {}, C.requests['POST /api/owner/stock/received']);
    step(`the receipt body for "${l.text}" matches the contract`, !b.error && !bad.length, b.error || bad.join('; '));
    const r = await own('/api/owner/stock/received', b.body);
    step(`"${l.text}" is received`, r.status === 200, `${r.status} ${r.text.slice(0, 160)}`);
  }
  const ab = R.aliasBody(card, p.nipt, confirmed, {});
  schema('the alias request matches the contract', ab, C.requests['POST /api/owner/stock/alias']);
  const a = await own('/api/owner/stock/alias', ab);
  step('the aliases are written (nipt + 2 lines = 3 notes)', a.status === 200 && a.body?.written === 3, `${a.status} ${a.text.slice(0, 200)}`);
  schema('the alias answer validates', a.body, C.responses['POST /api/owner/stock/alias 200']);
  const st = await shelf(own, LOC);
  schema('GET /api/owner/stock carries the fields', st.body, C.responses['GET /api/owner/stock 200 (fields this feature reads)']);
  const known = st.body?.supplierAliases?.[card];
  step('the Stock screen reads the aliases back', known?.nipt === 'K12345678A' && known?.lines?.['salmon fileto norvegjeze'] === ITEMS.salmon && known?.lines?.['fletë nori'] === ITEMS.nori, JSON.stringify(known));
  const sal = st.supplies.find(s => s.id === ITEMS.salmon), nor = st.supplies.find(s => s.id === ITEMS.nori);
  step('2500 g salmon and 20 nori are on the shelf', sal?.onHand === 2500 && nor?.onHand === 20, JSON.stringify([sal?.onHand, nor?.onHand]));
  step('the next invoice from this supplier arrives matched', p.lines.every(l => R.match(l, known?.lines, st.supplies).by === 'alias'));

  // ── 4. idempotent, strict ──
  const again = await own('/api/owner/stock/alias', ab);
  step('the same aliases again write nothing', again.status === 200 && again.body?.written === 0, again.text.slice(0, 120));
  const strict = await own('/api/owner/stock/alias', { card: { ...ab.card, rating: 5 } });
  step('an unknown field is refused (400)', strict.status === 400, `${strict.status}`);
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  if (card) step('the card is taken off', (await own('/api/owner/stock/supplier', { card: { id: card, name: SUP, gone: true } })).status === 200);
  if (made.length) step('the probe supplies are deleted', (await own('/api/owner/supplies/delete', { ids: made, location_id: LOC, confirmUses: true })).status === 200);
  verdict();
}
