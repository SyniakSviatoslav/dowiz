// Rasterise the brand mark into the PNG sizes an install needs.
//
// WHY A BROWSER. There is no rasteriser on this box — no PIL, no rsvg, no
// inkscape — and the mark is a single 70x80 path with 400 segments, so it is
// not something to re-draw by hand into a PNG encoder. Chromium is already in
// the tree for the regression gates and renders the exact same SVG the app
// does, which is the point: the installed icon and the in-app logo cannot drift.
//
//   node workers/api/design/make-icons.mjs
//
// Outputs to public/kit/img/. Re-run when the mark changes; the files are
// committed, so a deploy never depends on this script.

import { chromium } from 'playwright';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const pub = join(here, '..', 'public', 'kit');
const BRAND = '#fb5b21';
const mark = readFileSync(join(pub, 'icon', 'logo-union.svg'), 'utf8');

// `any` fills the square; `maskable` must keep its content inside the 80%
// safe circle, so the mark is drawn smaller and the brand colour carries the
// bleed the platform crops into whatever shape it likes.
const VARIANTS = [
  { file: 'icon-192.png',        size: 192, scale: 0.70, radius: 0.22 },
  { file: 'icon-512.png',        size: 512, scale: 0.70, radius: 0.22 },
  { file: 'icon-maskable.png',   size: 512, scale: 0.52, radius: 0 },
  { file: 'apple-touch-icon.png', size: 180, scale: 0.66, radius: 0 },
];

const page = (size, scale, radius) => `<!doctype html><meta charset="utf-8">
<style>
 html,body{margin:0;padding:0;background:transparent}
 .c{width:${size}px;height:${size}px;background:${BRAND};
    border-radius:${Math.round(size * radius)}px;
    display:flex;align-items:center;justify-content:center;overflow:hidden}
 .c svg{width:auto;height:${Math.round(size * scale)}px;display:block}
</style><div class="c">${mark}</div>`;

const browser = await chromium.launch();
for (const v of VARIANTS) {
  const p = await browser.newPage({ viewport: { width: v.size, height: v.size },
                                    deviceScaleFactor: 1 });
  await p.setContent(page(v.size, v.scale, v.radius));
  const shot = await p.locator('.c').screenshot({ omitBackground: true });
  writeFileSync(join(pub, 'img', v.file), shot);
  console.log(`${v.file}  ${v.size}x${v.size}  ${shot.length} bytes`);
  await p.close();
}
await browser.close();
