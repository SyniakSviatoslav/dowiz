#!/usr/bin/env node
// Unified token-representation router — the ONE place the "measure, don't assume; every encoding
// has a crossover" finding lives. Given a payload it MEASURES every applicable representation with
// the same ruler and returns the cheapest correct one, never a net loss:
//   - raw JSON            (always the baseline)
//   - VSA1 frame          (columnar/dict — wins on repetitive tabular data above its crossover)
//   - VSA-VIZ flat image  (scatter state — fixed ~1399 tok; wins above ~25-30 entities)
//   - VSA-VIZ fractal img (hierarchical zone→vehicle→order state — wins when hierarchy is the decision)
//
//   node tools/vsa/route.mjs <data.json>        → ranked recommendation (stderr) + best text (stdout)
//   import { route } from './route.mjs'         → { best, candidates }
//
// Image tokens are ~fixed regardless of entity count; text grows linearly — so the router's pick
// flips with SCALE, which is exactly why it must measure, not assume.

import fs from 'node:fs';
import { frameIfCheaper } from './src/codec.mjs';
import { renderState } from './src/viz.mjs';
import { renderFractal } from './src/viz-fractal.mjs';
import { countTokens } from './src/tokens.mjs';

const imageTokens = (w, h) => Math.min(1600, Math.ceil((w * h) / 750));

function looksFlat(v) {
  return v && typeof v === 'object' && (Array.isArray(v.couriers) || Array.isArray(v.orders));
}
function looksFractal(v) {
  return v && typeof v === 'object' && Array.isArray(v.zones);
}

export async function route(value, { specTokens = 0 } = {}) {
  const candidates = [];

  // raw + frame (crossover-aware)
  const framed = await frameIfCheaper(value, { specTokens });
  candidates.push({ repr: 'raw', tokens: framed.rawTok, note: 'baseline' });
  candidates.push({ repr: 'frame', tokens: framed.frameTok + specTokens, note: `VSA1 (+${specTokens} spec)` });

  // fractal image (only if the state carries a hierarchy)
  if (looksFractal(value)) {
    const { meta } = renderFractal(value);
    candidates.push({
      repr: 'viz-fractal',
      tokens: imageTokens(meta.w, meta.h),
      note: `${meta.zones}z/${meta.vehicles}v image (~fixed); hierarchy-native, decision-support`,
    });
  }
  // flat image (scatter)
  if (looksFlat(value)) {
    const { meta } = renderState(value);
    candidates.push({
      repr: 'viz-flat',
      tokens: imageTokens(meta.w, meta.h),
      note: `${(meta.couriers ?? 0) + (meta.orders ?? 0)}-entity image (~fixed)`,
    });
  }

  candidates.sort((a, b) => a.tokens - b.tokens);
  const best = candidates[0];
  // Images are decision-support, not lossless — flag when the winner is an image so the caller
  // keeps the authoritative JSON server-side and verifies the returned decision against source.
  best.lossless = best.repr === 'raw' || best.repr === 'frame';
  return { best, candidates };
}

// CLI
if (import.meta.url === `file://${process.argv[1]}`) {
  const file = process.argv[2];
  if (!file) {
    console.error('usage: route.mjs <data.json>');
    process.exit(2);
  }
  const value = JSON.parse(fs.readFileSync(file, 'utf8'));
  const { best, candidates } = await route(value, { specTokens: 90 });
  console.error('[vsa route] candidates (cheapest first):');
  for (const c of candidates) {
    console.error(`  ${c === best ? '→' : ' '} ${c.repr.padEnd(12)} ${String(c.tokens).padStart(6)} tok  ${c.note}`);
  }
  console.error(`[vsa route] PICK: ${best.repr} (${best.tokens} tok)${best.lossless ? '' : ' — decision-support image, keep JSON authoritative'}`);
  // stdout: for raw/frame emit the text; for images emit a hint (the caller renders via visionMessage*)
  if (best.repr === 'raw') process.stdout.write(JSON.stringify(value));
  else if (best.repr === 'frame') process.stdout.write((await frameIfCheaper(value)).text);
  else process.stdout.write(`# render with tools/vsa: ${best.repr === 'viz-fractal' ? 'visionMessageFractal' : 'visionMessage'}(state)\n`);
}
