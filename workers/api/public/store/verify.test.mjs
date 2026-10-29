// T-LV-PCD-03: the storefront's price proof. The JS PRODUCER (`certFor`) must emit, byte for byte,
// the gate's fixture certificate (crates/bebop-wasm/fixtures/verify/good.cert, cut by
// oracle_verify.py and read by four readers), and the promoted reader (verify.wasm, bw_verify) must
// accept 100% of a fixture menu's lines and refuse a moved price and one named path byte.
// Run: node --test workers/api/public/store/verify.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { pricesOf, certFor, verifyWith, qrWith, unb64url, b64url, readCert, verifyLink, hex } from './verify.js';

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const FIX = here('../../../../crates/bebop-wasm/fixtures/verify/good.cert');
const WASM = here('./verify.wasm');

// The gate's catalogue: p_000..p_164 at 100 + 7 i, split over two categories (one dish twice).
const menu = { categories: [
  { products: Array.from({ length: 100 }, (_, i) => ({ id: `p_${String(i).padStart(3, '0')}`, price: 100 + 7 * i })) },
  { products: Array.from({ length: 66 }, (_, j) => { const i = 99 + j; return { id: `p_${String(i).padStart(3, '0')}`, price: 100 + 7 * i }; }) },
] };

test('prices: id byte order, a dish in two categories once, two prices for one id refused', () => {
  const rows = pricesOf(menu);
  assert.equal(rows.length, 165);
  assert.deepEqual(rows[0], ['p_000', 100]);
  assert.equal(pricesOf({ categories: [{ products: [{ id: 'a', price: 1 }, { id: 'a', price: 2 }] }] }), null);
  assert.deepEqual(pricesOf({ categories: [{ products: [{ id: 'b', price: 2 }, { id: 'a', price: 1 }] }] }), [['a', 1], ['b', 2]], 'positive twin');
});

test('the JS producer emits the gate fixture byte for byte', async () => {
  const c = await certFor(menu, 'p_007', 149);
  assert.equal(hex(c), hex(readFileSync(FIX)));
  assert.equal(await certFor(menu, 'p_007', 150), null, 'a price the menu does not hold has no certificate');
  assert.equal(await certFor(menu, 'p_999', 1), null, 'nor a dish it does not list');
});

test('codecs: base64url round trip, the link, the line read back', async () => {
  const c = await certFor(menu, 'p_164', 100 + 7 * 164);
  assert.deepEqual(unb64url(b64url(c)), c);
  const link = verifyLink('https://dubin.dowiz.org', 'dubin', c);
  assert.match(link, /^https:\/\/dubin\.dowiz\.org\/store\/verify\.html#s=dubin&c=[A-Za-z0-9_-]+$/);
  const r = readCert(c);
  assert.equal(r.id, 'p_164');
  assert.equal(r.price, 1248);
  assert.equal(r.root, '9d64ce278dfc1b4282ef478ea3361e6f9ded6cfc8ac8717d3d2c234e8481879b');
});

test('the promoted reader accepts every line and refuses a moved price and a named byte', { skip: !existsSync(WASM) && 'verify.wasm not promoted (crates/bebop-wasm/verify.sh --promote)' }, async () => {
  const { instance } = await WebAssembly.instantiate(readFileSync(WASM), {});
  const x = instance.exports;
  for (const [id, price] of pricesOf(menu)) {
    const v = verifyWith(x, await certFor(menu, id, price));
    assert.equal(v.status, 0, id);
    assert.equal(v.root, '9d64ce278dfc1b4282ef478ea3361e6f9ded6cfc8ac8717d3d2c234e8481879b');
  }
  const good = await certFor(menu, 'p_007', 149);
  const moved = good.slice();
  moved[good.length - 32 * 8] ^= 1; // byte 0 of the first sibling: the gate's named byte
  assert.equal(verifyWith(x, moved).status, 51);
  const lie = await certFor({ categories: [{ products: [...menu.categories[0].products.slice(0, 7), { id: 'p_007', price: 150 }, ...menu.categories[0].products.slice(8), ...menu.categories[1].products] }] }, 'p_007', 150);
  const forged = good.slice();
  forged.set(lie.subarray(0, 1 + 8 + 5 + 16), 0); // the receipt's price says 150, the path is the real one
  assert.equal(verifyWith(x, forged).status, 51, 'a moved price climbs elsewhere');
  assert.equal(verifyWith(x, good.subarray(0, good.length - 1)).status, 50);
  const q = qrWith(x, verifyLink('https://dubin.dowiz.org', 'dubin', good));
  assert.ok(q && q.side >= 21 && q.cells.length === q.side * q.side, 'the receipt link fits a QR code');
});
