// canvas-frame's MACHINE CALIBRATION (W-RGQUICK, R-GRAPH §3.3): a first-frame mark in milliseconds
// is a statement about a machine, and the same build read 81-135 ms on this box on 2026-10-06, 332-570
// on 2026-10-09 and passes on the x86 CI runner. A gate that is red for the machine measures nothing.
//
// THE REFERENCE is a fixed page this harness serves (no file, no network): a fresh context at the
// same viewport and dpr navigates to it, it runs a fixed integer loop and a fixed Canvas2D raster the
// size of the board, and reports performance.now() -- milliseconds from navigation start, exactly the
// clock the board's first frame is read on. So it pays what the board pays and the board's code does
// not: context creation, navigation, HTML parse, script start, a full-screen raster.
//
// THE MARK = first_ms x min(ref_median / ref_nominal_ms, cap)   (canvas-frame.baseline)
//   i.e. pass iff board/reference <= first_ms/ref_nominal_ms (2.0) on EVERY machine.
//   - NO floor at first_ms. The first version held a fast machine to the absolute 300 ms, and on CI
//     (board 61.6 ms, reference 52.8 ms, run 38068929237) a first frame twice as slow read ~123 ms,
//     PASSED, and canvas-frame.prove went red: an absolute mark on a machine 5x under it measures
//     nothing. As a ratio: CI 1.17, this box 1.52-1.56, a 2x slower frame ~2.3 on CI and ~3.1 here.
//   - past `cap` the machine is too slow to judge -> exit 2, never a pass and never a red.
// Paired with the board's navigations (one reference before each), so load on a shared box slows
// both sides of the ratio together.

import { readFileSync } from 'node:fs';

export const REF_PAGE = `<!doctype html><meta charset=utf-8><meta name=viewport content="width=device-width">
<body style="margin:0"><canvas id=c style="width:100vw;height:100vh;display:block"></canvas><script>
const c = document.getElementById('c'), d = devicePixelRatio;
c.width = Math.round(innerWidth * d); c.height = Math.round(innerHeight * d);
const g = c.getContext('2d'); g.scale(d, d);
let h = 0x811c9dc5; for (let i = 0; i < 1500000; i++) { h ^= i & 255; h = Math.imul(h, 16777619) >>> 0; }
g.font = '14px sans-serif';
for (let i = 0; i < 600; i++) { g.fillStyle = 'hsl(' + (i * 37 % 360) + ',55%,' + (30 + i % 40) + '%)';
  g.fillRect((i % 10) * 39, ((i / 10) | 0) * 14, 38, 13); g.fillStyle = '#111'; g.fillText('ref ' + i, (i % 10) * 39 + 2, ((i / 10) | 0) * 14 + 11); }
const px = g.getImageData(0, 0, c.width, c.height).data; let s = 0; for (let i = 0; i < px.length; i += 4096) s = (s + px[i]) | 0;
window.__ref = { ms: performance.now(), h, s };
</script>`;

/// One reference navigation in a fresh context; milliseconds from navigation start to the end of the
/// script. Same viewport/dpr/mobile flags as canvas.mjs's `page`.
export async function refRun(browser, base) {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
  try {
    const p = await ctx.newPage();
    await p.goto(`${base}/__ref.html`, { waitUntil: 'load' });
    const r = await p.evaluate(() => window.__ref);
    if (!r || !(r.ms > 0) || r.s === 0) throw new Error('reference page did not run: ' + JSON.stringify(r));
    return r.ms;
  } finally { await ctx.close(); }
}

/// `key=value` lines of canvas-frame.baseline; a missing file or key is a harness failure, not a default.
export function readCalib(path) {
  const kv = {};
  for (const l of readFileSync(path, 'utf8').split('\n')) { const m = l.match(/^(\w+)=([\d.]+)\s*$/); if (m) kv[m[1]] = Number(m[2]); }
  for (const k of ['first_ms', 'ref_nominal_ms', 'cap']) if (!(kv[k] > 0)) throw new Error(`${path}: no ${k}=`);
  return kv;
}

/// The mark for this run, from its paired reference times (pure: tested in canvas-calib.test.mjs).
export function calibrate(refs, { first_ms, ref_nominal_ms, cap }) {
  const sorted = refs.slice().sort((a, b) => a - b), ref = sorted[Math.floor(sorted.length / 2)];
  const ratio = ref / ref_nominal_ms, factor = Math.min(ratio, cap);
  const out = { refs: sorted, ref, ref_nominal_ms, ratio: Math.round(ratio * 1000) / 1000, factor: Math.round(factor * 1000) / 1000, mark: Math.round(first_ms * factor) };
  if (ratio > cap) out.refused = `machine reads ${out.ratio}x the nominal reference (cap ${cap}x): too slow to judge a first frame`;
  return out;
}
