#!/usr/bin/env node
// demo-from-wolt — turn a Wolt venue slug into a demo-builder-ready prospect JSON, so building a demo for a
// new venue is one repeatable command instead of hand-assembly. Fetches the Wolt venue page, extracts the
// menu (authentic Albanian — no LLM translation needed for an Albanian venue), keyword-categorises it, and
// writes loops/prospects/<slug>.json ({slug,name,cuisine,phone?,primary?,bg?, menu:[{name,category,price,
// description,image_url,description_sq}]}). Then: node scripts/demo-builder.mjs loops/prospects/<slug>.json
//
// Usage: node scripts/demo-from-wolt.mjs <wolt-slug> --slug <demo-slug> --name "<Name>" --cuisine <c> [--phone +355…] [--city durres]
// Public business data only. Respect Wolt ToS + the repo scraping-conduct gate.
import { writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i >= 0 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--') ? process.argv[i + 1] : d; };
const woltSlug = process.argv[2] && !process.argv[2].startsWith('--') ? process.argv[2] : null;
const slug = arg('slug', woltSlug);
const name = arg('name', woltSlug);
const cuisine = arg('cuisine', 'restaurant');
const phone = arg('phone', null);
const city = arg('city', 'durres');
if (!woltSlug) { console.error('usage: demo-from-wolt.mjs <wolt-slug> --slug <demo-slug> --name "<Name>" --cuisine <c> [--phone …]'); process.exit(1); }

// keyword categoriser (Albanian + Italian food words) — reliable ≥2 categories without fragile Wolt parsing.
const CATS = [
  ['Pije', /\b(çaj|caj|ujë|uje|coca|cola|soda|fanta|sprite|birr|birra|verë|vere|wine|beer|kafe|coffee|lemonade|juice|lëng|leng|pije|water|spritz|aperol|prosecco|cocktail|shake)\b/i],
  ['Ëmbëlsira', /\b(ëmbëlsir|embelsir|tiramisu|dessert|tort|akullore|gelato|cake|panna|cheesecake|brownie|profiterol|crema)\b/i],
  ['Pizza', /\b(pizz)/i],
  ['Pasta', /\b(pasta|spaghetti|linguine|penne|rigatoni|tagliatelle|lasagne|lasagna|gnocchi|risotto|ravioli|fettuccine|carbonara|tortellini)\b/i],
  ['Antipasti & Sallata', /\b(antipast|sallat|salad|karpaçio|karpacio|bruschetta|supë|supe|soup|meze|starter|bruskete|prosciutto)\b/i],
  ['Peshk & Detare', /\b(peshk|levrek|koran|oktapod|karkalec|midhje|fish|seafood|frutti di mare|calamar|kallamar|salmon|shrimp|lobster|aragost)\b/i],
  ['Mish & Grill', /\b(mish|biftek|steak|fillet|filetto|pulë|pule|chicken|grill|qofte|brek|tavë|tave|kebab|hamburger|burger|bërxoll|berxoll|viçi|vici)\b/i],
];
const categorise = (n, d) => { const hay = `${n} ${d || ''}`; for (const [c, re] of CATS) if (re.test(hay)) return c; return 'Të tjera'; };

// Cuisine → palette seed (mirrors demo-builder CUISINE_SEEDS; owner refines on claim).
const SEED = { italian: ['#3f7d4f', '#fbf6ee'], pizzeria: ['#c1352b', '#fbf6ee'], seafood: ['#1f6f8b', '#f2f6f7'], mediterranean: ['#2f7d76', '#f7f3ea'], traditional: ['#8a5a2b', '#f6efe4'], restaurant: ['#b5471f', '#faf4e8'] };

(async () => {
  const { extractWoltMenu: extract } = await import('../tools/demo-builder/wolt-menu-extract.mjs');
  const res = await fetch(`https://wolt.com/en/alb/${city}/restaurant/${woltSlug}`, { headers: { 'user-agent': 'Mozilla/5.0 Chrome/126 Safari/537.36' } });
  if (!res.ok) throw new Error(`Wolt fetch ${res.status}`);
  const items = extract(await res.text());
  const menu = items.map((it, i) => ({
    name: it.name,
    category: categorise(it.name, it.sq_desc),
    price: Math.round(it.all), // ALL, 0 minor units (matches demo-builder ArtePasta-style prices)
    ...(it.sq_desc ? { description: it.sq_desc, description_sq: it.sq_desc } : {}),
    ...(it.image ? { image_url: it.image } : {}),
    sort_order: i,
  }));
  const cats = [...new Set(menu.map((m) => m.category))];
  const withDesc = menu.filter((m) => m.description).length;
  const [primary, bg] = SEED[cuisine] || SEED.restaurant;
  const prospect = { slug, name, cuisine, ...(phone ? { phone } : {}), primary, bg, menu };
  const dir = resolve('loops/prospects'); mkdirSync(dir, { recursive: true });
  const out = resolve(dir, `${slug}.json`);
  writeFileSync(out, JSON.stringify([prospect], null, 1));
  console.error(`[demo-from-wolt] ${slug}: ${menu.length} items · ${cats.length} cats [${cats.join(', ')}] · ${Math.round(100 * withDesc / menu.length)}% described → ${out}`);
  console.log(JSON.stringify({ slug, items: menu.length, categories: cats.length, describedPct: Math.round(100 * withDesc / menu.length), gateOK: menu.length >= 6 && cats.length >= 2 }));
})().catch((e) => { console.error('[demo-from-wolt] FATAL:', e.message); process.exit(1); });
