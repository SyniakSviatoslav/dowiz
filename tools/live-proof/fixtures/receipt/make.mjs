// node tools/live-proof/fixtures/receipt/make.mjs
// Renders every invoice in invoices.mjs to <id>.html and <id>.png (headless
// Chromium through the repo's Playwright), plus <id>.expected.json. Idempotent.
import { writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { INVOICES, SKEWED, money, want } from './invoices.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT || '/root/dowiz/node_modules/playwright');

const W = {
  sq: { title: 'FATURË', no: 'Fatura nr.', date: 'Data', nipt: 'NIPT', buyer: 'Blerësi', cols: ['Nr', 'Përshkrimi', 'Njësia', 'Sasia', 'Çmimi', 'Vlera'], sub: 'Totali pa TVSH', vat: 'TVSH 20%', tot: 'Gjithsej' },
  en: { title: 'INVOICE', no: 'Invoice No.', date: 'Date', nipt: 'NIPT', buyer: 'Buyer', cols: ['#', 'Description', 'Unit', 'Qty', 'Price', 'Amount'], sub: 'Subtotal', vat: 'VAT 20%', tot: 'Total' },
};

function html(inv, { rotate = 0, blur = 0 } = {}){
  const w = W[inv.lang], f = inv.fmt;
  const sub = inv.lines.reduce((s, l) => s + l.total, 0), vat = sub / 5;
  const rows = inv.lines.map((l, i) => `<tr><td>${i + 1}</td><td>${l.text}</td><td>${l.unit}</td><td class=n>${l.qty}</td><td class=n>${money(l.price, f)}</td><td class=n>${money(l.total, f)}</td></tr>`).join('');
  return `<!doctype html><meta charset=utf-8><style>
body{margin:0;background:#fff;font:17px/1.5 "DejaVu Sans",Arial,sans-serif;color:#111}
.p{width:760px;padding:36px;transform:rotate(${rotate}deg);filter:blur(${blur}px)}
h1{font-size:26px;margin:0 0 6px}table{border-collapse:collapse;width:100%;margin:18px 0}
th,td{border-bottom:1px solid #999;padding:6px 8px;text-align:left}.n{text-align:right}.s{margin-left:auto;width:320px}</style>
<div class=p><h1>${inv.supplier}</h1><div>${w.nipt}: ${inv.nipt}</div><div>Rruga Tregtare 12, Durrës</div>
<h2>${w.title}</h2><div>${w.no} ${inv.doc}</div><div>${w.date}: ${inv.date}</div><div>${w.buyer}: Restorant Shembull</div>
<table><tr>${w.cols.map((c, i) => `<th${i > 2 ? ' class=n' : ''}>${c}</th>`).join('')}</tr>${rows}</table>
<table class=s><tr><td>${w.sub}</td><td class=n>${money(sub, f)}</td></tr><tr><td>${w.vat}</td><td class=n>${money(vat, f)}</td></tr><tr><td><b>${w.tot}</b></td><td class=n><b>${money(sub + vat, f)}</b></td></tr></table></div>`;
}

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 840, height: 900 }, deviceScaleFactor: 1.5 });
  const jobs = [...INVOICES.map(inv => [inv.id, inv, {}]), [SKEWED.id, INVOICES.find(i => i.id === SKEWED.of), SKEWED]];
  for (const [id, inv, look] of jobs) {
    const h = html(inv, look);
    writeFileSync(join(here, `${id}.html`), h);
    await page.setContent(h);
    await page.screenshot({ path: join(here, `${id}.png`), fullPage: true });
    writeFileSync(join(here, `${id}.expected.json`), JSON.stringify({ supplier: inv.card, nipt: inv.nipt, doc: inv.doc, lines: want(inv) }, null, 1) + '\n');
    console.log('made', id);
  }
} finally { await browser.close(); }
