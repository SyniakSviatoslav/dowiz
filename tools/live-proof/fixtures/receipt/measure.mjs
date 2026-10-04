// node tools/live-proof/fixtures/receipt/measure.mjs [TESSERACT_JS_DIR]
// Reads every fixture PNG with tesseract.js (`sqi+eng`, LSTM) using the
// traineddata VENDORED under workers/api/public/lib/ocr -- the same files the
// console loads -- parses the text with receipt-photo-logic.js, matches the
// lines with no aliases, and prints line accuracy per image. Then the
// edit reduction: invoice 2 of the same supplier read cold vs. with the
// aliases invoice 1's confirmation would have stored.
//
// tesseract.js for node is NOT vendored (the console's copy is the browser
// build); pass its install dir or set TESSERACT_JS. Output: one JSON line per
// image and a summary, to stdout. Exit 1 if any image fails to read.
import { readFileSync, existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as R from '../../../../workers/api/public/admin/receipt-photo-logic.js';
import { INVOICES, SKEWED, SUPPLIES } from './invoices.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const lib = join(here, '../../../../workers/api/public/lib/ocr');
const dir = process.argv[2] || process.env.TESSERACT_JS;
if (!dir) { console.error('measure: pass the dir holding node_modules/tesseract.js'); process.exit(2); }
const require = createRequire(join(dir, 'x.js'));
const { createWorker } = require('tesseract.js');

const ids = [...INVOICES.map(i => i.id), SKEWED.id];
const worker = await createWorker(['sqi', 'eng'], 1, { langPath: lib, gzip: true, cacheMethod: 'none' });
const read = {};
let failed = 0;
try {
  await worker.setParameters({ preserve_interword_spaces: '1', tessedit_pageseg_mode: '6' });
  for (const id of ids) {
    const png = join(here, `${id}.png`);
    if (!existsSync(png)) { console.error('missing', png); failed++; continue; }
    const t0 = Date.now();
    const { data } = await worker.recognize(png);
    read[id] = { text: data.text, ms: Date.now() - t0 };
  }
} finally { await worker.terminate(); }

const score = (id, known) => {
  const want = JSON.parse(readFileSync(join(here, `${id}.expected.json`), 'utf8'));
  const p = R.parseText(read[id].text);
  const got = p.lines.map(l => ({ ...l, item: R.match(l, known, SUPPLIES).item }));
  // Read correctly = qty, unit price and total all right (the item is the match, scored apart).
  const fieldsOk = want.lines.filter(w => got.some(g => g.qty && g.qty.v / 10 ** g.qty.d === w.qty && g.unit_price === w.unit_price && g.total === w.total)).length;
  return { id, ms: read[id].ms, lines: want.lines.length, parsed: p.lines.length, readOk: fieldsOk, nipt: p.nipt === want.nipt, doc: p.doc === want.doc, ...R.edits(got, want.lines) };
};

const rows = ids.filter(id => read[id]).map(id => score(id, {}));
for (const r of rows) console.log(JSON.stringify(r));
const sum = k => rows.reduce((s, r) => s + r[k], 0);
console.log(JSON.stringify({ summary: 'line read accuracy (qty+price+total all right)', ok: sum('readOk'), of: sum('lines'), pct: Math.round(100 * sum('readOk') / sum('lines')) }));

// Edit reduction: invoice 1 confirmed as expected -> its aliases -> invoice 2.
const inv1 = JSON.parse(readFileSync(join(here, 'inv1-peshku-sq.expected.json'), 'utf8'));
const p1 = R.parseText(read['inv1-peshku-sq']?.text || '');
const known = {};
for (const l of p1.lines) {
  const w = inv1.lines.find(x => x.total === l.total);
  if (w) known[l.key] = w.item;
}
const cold = score('inv2-peshku-sq', {}), warm = score('inv2-peshku-sq', known);
console.log(JSON.stringify({ summary: 'second invoice, same supplier', coldEdits: cold.edits, warmEdits: warm.edits, reductionPct: cold.edits ? Math.round(100 * (cold.edits - warm.edits) / cold.edits) : 0, aliases: Object.keys(known).length }));
if (process.env.SHOW) for (const id of ids) console.log(`----- ${id}\n${read[id]?.text}`);
process.exit(failed ? 1 : 0);
