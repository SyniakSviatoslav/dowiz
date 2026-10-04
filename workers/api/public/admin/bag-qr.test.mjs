// RED PROOF 6.4 (W-QR): THE PRINTED BAG CODE OPENS EXACTLY THE VENUE'S OWN HOST.
//
// The Worker draws the code (`orders::room::table_qr::svg`, qrcodegen). The Rust
// route test `bag_routes::tests::the_printed_code_is_the_decoded_fixture` pins
// what the route draws for venue `alpha`, campaign `spring`, to the fixture file
// read here; this test DECODES that file with an independent decoder written
// from the QR standard (ISO/IEC 18004: format bits, function patterns, zigzag
// placement, mask, block de-interleave, byte/alphanumeric/numeric segments)
// and demands exactly the landing URL. No error correction is applied: a clean
// code needs none, and a decoder that corrected would hide a wrong module.
//
//   node workers/api/public/admin/bag-qr.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FIXTURE = path.join(HERE, '../../src/services/loyalty/bag_routes/qr-alpha-spring.svg');
const BORDER = 4;
const WANT = 'https://alpha.dowiz.org/?src=bag&c=spring';

// ── the SVG back to a module matrix ────────────────────────────────────────
export function modules(svg){
  const dim = Number(/viewBox="0 0 (\d+) \d+"/.exec(svg)[1]);
  const size = dim - 2 * BORDER;
  const m = Array.from({ length: size }, () => new Array(size).fill(false));
  for (const [, x, y] of svg.matchAll(/M(\d+),(\d+)h1v1h-1z/g)) m[Number(y) - BORDER][Number(x) - BORDER] = true;
  return m;
}

// ── the standard's tables (versions 1-10; index 0 unused) ─────────────────
const ECC_PER_BLOCK = { L: [0, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18], M: [0, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26],
  Q: [0, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24], H: [0, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28] };
const BLOCKS = { L: [0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4], M: [0, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5],
  Q: [0, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8], H: [0, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8] };
const ECC_OF_FORMAT = ['M', 'L', 'H', 'Q'];
const MASKS = [
  (x, y) => (x + y) % 2 === 0, (x, y) => y % 2 === 0, (x) => x % 3 === 0, (x, y) => (x + y) % 3 === 0,
  (x, y) => (Math.floor(x / 3) + Math.floor(y / 2)) % 2 === 0, (x, y) => (x * y) % 2 + (x * y) % 3 === 0,
  (x, y) => ((x * y) % 2 + (x * y) % 3) % 2 === 0, (x, y) => ((x + y) % 2 + (x * y) % 3) % 2 === 0,
];

function alignment(ver){
  if (ver === 1) return [];
  const n = Math.floor(ver / 7) + 2;
  const step = Math.floor((ver * 8 + n * 3 + 5) / (n * 4 - 4)) * 2;
  const out = [6];
  for (let pos = ver * 4 + 10; out.length < n; pos -= step) out.splice(1, 0, pos);
  return out;
}

function rawCodewords(ver){
  let r = (16 * ver + 128) * ver + 64;
  if (ver >= 2) { const n = Math.floor(ver / 7) + 2; r -= (25 * n - 10) * n - 55; if (ver >= 7) r -= 36; }
  return Math.floor(r / 8);
}

function functionMap(size, ver){
  const f = Array.from({ length: size }, () => new Array(size).fill(false));
  const box = (x0, y0, w, h) => { for (let y = y0; y < y0 + h; y++) for (let x = x0; x < x0 + w; x++) if (x >= 0 && y >= 0 && x < size && y < size) f[y][x] = true; };
  box(0, 0, 9, 9); box(size - 8, 0, 8, 9); box(0, size - 8, 9, 8);
  box(6, 0, 1, size); box(0, 6, size, 1);
  const a = alignment(ver);
  for (const cx of a) for (const cy of a) {
    if ((cx === 6 && cy === 6) || (cx === 6 && cy === a[a.length - 1]) || (cx === a[a.length - 1] && cy === 6)) continue;
    box(cx - 2, cy - 2, 5, 5);
  }
  if (ver >= 7) { box(size - 11, 0, 3, 6); box(0, size - 11, 6, 3); }
  return f;
}

export function decode(m){
  const size = m.length, ver = (size - 17) / 4;
  assert.ok(Number.isInteger(ver) && ver >= 1 && ver <= 10, `size ${size}`);
  // Format: 15 bits beside the top-left finder, as qrcodegen drawFormatBits places them.
  const at = [];
  for (let i = 0; i <= 5; i++) at.push([8, i]);
  at.push([8, 7], [8, 8], [7, 8]);
  for (let i = 9; i < 15; i++) at.push([14 - i, 8]);
  let bits = 0;
  at.forEach(([x, y], i) => { if (m[y][x]) bits |= 1 << i; });
  const data = (bits ^ 0x5412) >> 10;
  const ecc = ECC_OF_FORMAT[data >> 3], mask = MASKS[data & 7];
  // The codewords, in the zigzag, unmasked.
  const fn = functionMap(size, ver), stream = [];
  for (let right = size - 1; right >= 1; right -= 2) {
    if (right === 6) right = 5;
    for (let vert = 0; vert < size; vert++) for (let j = 0; j < 2; j++) {
      const x = right - j, up = ((right + 1) & 2) === 0, y = up ? size - 1 - vert : vert;
      if (!fn[y][x]) stream.push(m[y][x] !== mask(x, y));
    }
  }
  const raw = rawCodewords(ver), words = [];
  for (let i = 0; i < raw; i++) { let b = 0; for (let k = 0; k < 8; k++) b = (b << 1) | (stream[i * 8 + k] ? 1 : 0); words.push(b); }
  // De-interleave the data codewords.
  const nb = BLOCKS[ecc][ver], per = ECC_PER_BLOCK[ecc][ver];
  const short = nb - raw % nb, shortLen = Math.floor(raw / nb);
  const lens = Array.from({ length: nb }, (_, b) => shortLen - per + (b < short ? 0 : 1));
  const blocks = lens.map(() => []);
  let k = 0;
  for (let i = 0; i < Math.max(...lens); i++) for (let b = 0; b < nb; b++) if (i < lens[b]) blocks[b].push(words[k++]);
  const dataBits = blocks.flat().flatMap(b => [7, 6, 5, 4, 3, 2, 1, 0].map(s => (b >> s) & 1));
  // Segments.
  let p = 0;
  const read = n => { let v = 0; for (let i = 0; i < n; i++) v = (v << 1) | dataBits[p++]; return v; };
  const ALNUM = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:';
  const bytes = []; let text = '';
  while (p + 4 <= dataBits.length) {
    const mode = read(4);
    if (mode === 0) break;
    if (mode === 4) { const n = read(8); for (let i = 0; i < n; i++) bytes.push(read(8)); text += Buffer.from(bytes.splice(0)).toString('utf8'); }
    else if (mode === 2) { let n = read(9); for (; n >= 2; n -= 2) { const v = read(11); text += ALNUM[Math.floor(v / 45)] + ALNUM[v % 45]; } if (n) text += ALNUM[read(6)]; }
    else if (mode === 1) { let n = read(10); for (; n >= 3; n -= 3) text += String(read(10)).padStart(3, '0'); if (n === 2) text += String(read(7)).padStart(2, '0'); if (n === 1) text += String(read(4)); }
    else throw new Error('mode ' + mode);
  }
  return { text, ver, ecc };
}

test('the bag code decodes to exactly the venue host and the two parameters', () => {
  const svg = fs.readFileSync(FIXTURE, 'utf8');
  const { text, ver, ecc } = decode(modules(svg));
  assert.equal(ecc, 'M', 'qrcodegen QrCodeEcc::Medium');
  assert.equal(text, WANT, `version ${ver}`);
  const u = new URL(text);
  assert.equal(u.host, 'alpha.dowiz.org');
  assert.equal(u.pathname, '/');
  assert.deepEqual([...u.searchParams.keys()], ['src', 'c'], 'nothing per guest, no id, no token');
});

test('control: one data module flipped is NOT the URL (the decoder reads, it does not guess)', () => {
  const m = modules(fs.readFileSync(FIXTURE, 'utf8'));
  const size = m.length;
  m[size - 1][size - 1] = !m[size - 1][size - 1]; // the first data bit in the zigzag
  let text = '';
  try { text = decode(m).text; } catch (e) { text = 'threw: ' + e.message; }
  assert.notEqual(text, WANT);
});
