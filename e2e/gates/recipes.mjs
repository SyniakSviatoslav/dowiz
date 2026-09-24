// F1 — RECIPE COVERAGE, as a report. READ-ONLY.
//
// The first coverage number this platform produced measured nothing
// (memory `dowiz-stock-ledger-works-but-is-off`, trap 2: it read the public
// menu, which never carries a recipe, and so said 0 whatever was stored). This
// reads the OWNER routes that carry what is stored: `GET /api/owner/products`
// (every dish with its `bom`) and `GET /api/owner/stock` (every supply with
// its kind and kcal), and prints
//
//   dishes 165 · with recipe N · supplies M · dishes whose every food line has kcal K
//
// A REPORT, NOT A RATCHET, until the operator says which number a venue must
// reach before stock is switched on for it (BLUEPRINT-LAUNCH-GAPS §1.3).
// A read that fails is exit 2 with the status -- never a 0 that looks measured.
//
//   HOST=qa-durres.dowiz.org node e2e/gates/recipes.mjs
//   (OWNER_EMAIL / OWNER_PASSWORD from the environment, else /root/.dowiz_owner)
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const FOOD = new Set(['food_ingredient', 'condiment']);

/// The four numbers, from the two owner reads. PURE.
export function coverage(products, supplies) {
  const byId = new Map(supplies.map(s => [s.id, s]));
  const withRecipe = products.filter(p => Array.isArray(p.bom) && p.bom.length > 0);
  // A food line's kcal comes from its supply as it is NOW (the line's own
  // snapshot is what the dish read at save time). Packaging is not eaten and
  // does not count; a dish with no food line at all is not "complete".
  const kcalComplete = withRecipe.filter(p => {
    const food = p.bom.filter(l => FOOD.has((byId.get(l.supply) || l).kind || 'food_ingredient'));
    return food.length > 0 && food.every(l => byId.get(l.supply)?.kcalPer100 != null);
  });
  return { dishes: products.length, withRecipe: withRecipe.length, supplies: supplies.length, kcalComplete: kcalComplete.length };
}

export const line = c =>
  `dishes ${c.dishes} · with recipe ${c.withRecipe} · supplies ${c.supplies} · dishes whose every food line has kcal ${c.kcalComplete}`;

function creds() {
  if (process.env.OWNER_EMAIL && process.env.OWNER_PASSWORD) return { email: process.env.OWNER_EMAIL, password: process.env.OWNER_PASSWORD };
  const f = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8').split('\n')
    .filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));
  return { email: f.OWNER_EMAIL, password: f.OWNER_PASSWORD };
}

async function main() {
  const host = process.env.HOST;
  if (!host) { console.error('recipes: HOST is required (e.g. HOST=qa-durres.dowiz.org); there is no default venue'); process.exit(2); }
  const base = `https://${host.includes('.') ? host : `${host}.dowiz.org`}`;
  const get = async (path, opts = {}) => {
    const r = await fetch(base + path, opts);
    const body = await r.text();
    if (!r.ok) { console.error(`recipes: ${path} answered ${r.status}: ${body.slice(0, 200)}`); process.exit(2); }
    return JSON.parse(body);
  };
  const login = await get('/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(creds()) });
  if (!login.access_token) { console.error('recipes: no access token from login'); process.exit(2); }
  const auth = { headers: { authorization: `Bearer ${login.access_token}` } };
  const [p, s] = [await get('/api/owner/products', auth), await get('/api/owner/stock', auth)];
  if (!Array.isArray(p.products) || !Array.isArray(s.supplies)) { console.error('recipes: a read did not carry its list'); process.exit(2); }
  console.log(`${host}: ${line(coverage(p.products, s.supplies))}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
