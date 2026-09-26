// ONE STATIC PAGE OF THE NUMBERS: docs/measurements/evals/latest.html.
//
//   node tools/evals/dashboard.mjs [DIR]
//
// Reads the newest JSON of each suite in DIR, embeds them, and renders the headline cards and
// every group's table with no network and no script beyond the page's own. Light and dark
// follow the system. Opens from the file system.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { fmt, rowsOf } from './report.mjs';
import { groupOf } from './rules.mjs';

export const HEADLINES = [
  ['wasm.raw', 'Worker wasm'], ['surfaces.store.gzip', 'Storefront boot (gz)'], ['gates.green', 'Gates green'],
  ['tests.rust.dowiz_core', 'Core tests'], ['live.api.menu.ttfb_ms', 'Menu TTFB'], ['ux.store.fcp_ms', 'Store FCP'],
  ['health.sushi_durres.worst_used_permille', 'Sushi Durrës worst image'],
  ['health.dubin_sushi.worst_used_permille', 'Dubin worst image'], ['cost.marginal_venue_month_micro_usd', 'µ$ / venue / month'],
];

const esc = s => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

/** The newest `<date>-<commit>-<suite>.json` per suite. */
export function latest(dir) {
  const by = {};
  for (const f of fs.readdirSync(dir).filter(x => x.endsWith('.json')).sort()) {
    const m = f.match(/-(ci|nightly)\.json$/);
    if (m) by[m[1]] = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
  }
  return by;
}

export function card(doc, id, label) {
  const r = doc[id];
  if (!r) return '';
  const cls = r.status === 'breach' ? 'bad' : r.status === 'unverified' ? 'unv' : 'ok';
  return `<div class="card ${cls}"><div class="k">${esc(label)}</div><div class="v">${esc(fmt(r.value))}</div>`
    + `<div class="u">${esc(r.unit)} · ${esc(r.status)}</div></div>`;
}

export function table(doc) {
  const groups = {};
  for (const [id, r] of rowsOf(doc)) (groups[groupOf(id)] ||= []).push([id, r]);
  return Object.keys(groups).sort().map(g => `<details${groups[g].some(([, r]) => r.status === 'breach') ? ' open' : ''}><summary>${esc(g)} `
    + `<span class="n">${groups[g].length}</span></summary><table><thead><tr><th>Indicator</th><th>Value</th><th>Baseline</th><th>Rule</th><th>Status</th><th>Note</th></tr></thead><tbody>`
    + groups[g].map(([id, r]) => `<tr class="${esc(r.status)}"><td>${esc(id)}</td><td class="num">${esc(fmt(r.value))} ${esc(r.unit)}</td>`
      + `<td class="num">${esc(fmt(r.baseline))}</td><td>${esc(r.rule)}${r.limit !== undefined ? ' ' + esc(fmt(r.limit)) : ''}</td>`
      + `<td>${esc(r.status)}</td><td>${esc(r.why || r.unverified || r.note || '')}</td></tr>`).join('')
    + '</tbody></table></details>').join('\n');
}

export function page(runs) {
  const sections = Object.entries(runs).map(([suite, doc]) => {
    const m = doc._run;
    const rows = rowsOf(doc);
    const n = s => rows.filter(([, r]) => r.status === s).length;
    return `<section><h2>${esc(suite)} <small>${esc(m.date)} · ${esc(m.commit)} · ${rows.length} indicators · `
      + `<b class="${n('breach') ? 'badt' : ''}">${n('breach')} breaches</b> · ${n('unverified')} unverified</small></h2>`
      + `<div class="cards">${HEADLINES.map(([id, l]) => card(doc, id, l)).join('')}</div>${table(doc)}</section>`;
  }).join('\n');
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>dowiz evals</title><style>
:root{--bg:#f7f6f2;--fg:#1d1c1a;--mut:#6b675f;--line:#dedad1;--card:#fff;--ok:#1f7a4d;--bad:#b3261e;--unv:#8a6d00}
@media (prefers-color-scheme:dark){:root{--bg:#141412;--fg:#ecebe6;--mut:#a29d92;--line:#2f2d29;--card:#1d1c1a;--ok:#5cc28d;--bad:#ff8a80;--unv:#e0c14d}}
body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.45 system-ui,sans-serif;padding:16px;max-width:1200px;margin:auto}
h1{font-size:20px}h2{font-size:17px;margin-top:28px}small{color:var(--mut);font-weight:400}
.cards{display:grid;grid-template-columns:repeat(auto-fill,minmax(150px,1fr));gap:8px}
.card{background:var(--card);border:1px solid var(--line);border-left:4px solid var(--ok);border-radius:6px;padding:8px}
.card.bad{border-left-color:var(--bad)}.card.unv{border-left-color:var(--unv)}.k{color:var(--mut);font-size:12px}.v{font-size:20px;font-variant-numeric:tabular-nums}.u{color:var(--mut);font-size:12px}
details{margin-top:8px;background:var(--card);border:1px solid var(--line);border-radius:6px;padding:6px 10px;overflow-x:auto}
summary{cursor:pointer;font-weight:600}.n{color:var(--mut);font-weight:400}
table{border-collapse:collapse;width:100%;margin-top:6px;font-size:13px}td,th{border-top:1px solid var(--line);padding:3px 6px;text-align:left;vertical-align:top}
td.num{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}tr.breach td{color:var(--bad)}tr.unverified td{color:var(--unv)}.badt{color:var(--bad)}
</style></head><body><h1>dowiz evals</h1><p>Newest run of each suite. Breaches open their group; UNVERIFIED is never counted as green.</p>
${sections || '<p>No runs found.</p>'}</body></html>
`;
}

export function build(dir, out = path.join(dir, 'latest.html')) {
  fs.writeFileSync(out, page(latest(dir)));
  return out;
}

/** The directory the command line names, else the repo's docs/measurements/evals. */
export function dirOf(argv) {
  return argv[2] || path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../docs/measurements/evals');
}

/* node:coverage disable */
if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  console.log(build(dirOf(process.argv)));
}
/* node:coverage enable */
