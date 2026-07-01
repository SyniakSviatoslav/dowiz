#!/usr/bin/env node
// demo-builder · extraction layer — Wolt venue menu → structured items.
//
// Wolt venue pages (wolt.com/.../restaurant/<slug>) are client-rendered, but the FULL menu ships inline
// in the HTML as JSON: each item object carries "checksum" … "description" … "images":[{url}] … "name" …
// "price" (minor units ÷100 = local currency). This parses that inline JSON — no API key, no browser.
//
// Fetch the HTML from a NON-blocked egress first (this repo's note: the staging Fly EU-IP fetches Wolt
// with HTTP 200 where the sandbox IP is rate-limited). Then:
//   node tools/demo-builder/wolt-menu-extract.mjs <wolt.html>            # → JSON on stdout
//   node tools/demo-builder/wolt-menu-extract.mjs <wolt.html> --pretty
//
// Output: [{ name, all (price in local units), sq_desc (verbatim description), image }] deduped by name+price.
// It does NOT translate or categorise — that's the loop's normalize step (translate → English, parse the
// description into display ingredients). Respect Wolt's ToS + this repo's scraping-conduct gate before use.

import { readFileSync } from 'node:fs';

const unescapeJson = (s) =>
  s == null ? s : s
    .replace(/\\u([0-9a-fA-F]{4})/g, (_, h) => String.fromCharCode(parseInt(h, 16)))
    .replace(/\\"/g, '"')
    .replace(/\\\//g, '/');

/** Parse a Wolt venue HTML string into structured menu items. */
export function extractWoltMenu(html, { minAll = 100, maxAll = 5000 } = {}) {
  const items = [];
  const seen = new Set();
  // Each item object begins with "checksum": — split on it and read the fields within each segment.
  for (const seg of html.split('"checksum":')) {
    const name = (seg.match(/"name":"((?:[^"\\]|\\.)*)"/) || [])[1];
    const price = +((seg.match(/"price":(\d{3,7})/) || [])[1] || 0);
    const desc = (seg.match(/"description":"((?:[^"\\]|\\.)*)"/) || [])[1] || null;
    const image = (seg.match(/"images":\[\{"url":"([^"]+)"/) || [])[1] || null;
    if (!name || price <= 0) continue;
    const all = price / 100;
    if (all < minAll || all > maxAll) continue;
    const key = `${name}|${all}`;
    if (seen.has(key)) continue;
    seen.add(key);
    items.push({ name: unescapeJson(name), all, sq_desc: unescapeJson(desc), image });
  }
  return items;
}

if (process.argv[1] && import.meta.url.endsWith(process.argv[1].split('/').pop())) {
  const file = process.argv.find((a) => !a.startsWith('--') && a.endsWith('.html'));
  if (!file) { console.error('usage: wolt-menu-extract.mjs <wolt.html> [--pretty]'); process.exit(2); }
  const items = extractWoltMenu(readFileSync(file, 'utf8'));
  console.error(`extracted ${items.length} items`);
  console.log(JSON.stringify(items, null, process.argv.includes('--pretty') ? 2 : 0));
}
