#!/usr/bin/env node
// REV-S4-3 golden-fixture parity suite — generates ONCE from the live sharp stack, per council
// resolution (docs/design/rebuild-media-s4-council/resolution.md). NOT run at build/test time —
// a generation-time-only script (requires `sharp`, which this Rust crate never depends on). Run
// manually to REGENERATE fixtures if the sharp pipeline itself ever changes:
//
//   node crates/api/tests/fixtures/media/generate-goldens.mjs
//
// Loads sharp from the OLD-stack apps/api's node_modules (absolute path) rather than adding a
// package.json to this crate — this script is a one-off generation tool, not a dependency of the
// Rust build.
import sharp from '/root/dowiz/apps/api/node_modules/sharp/lib/index.js';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import zlib from 'node:zlib';

const DIR = dirname(fileURLToPath(import.meta.url));
const PRODUCT = { width: 800, height: 800, quality: 82 };

// Sharp's `raw` input option's `depth` field did not round-trip as 16-bit in this environment
// (verified empirically: metadata reported back `depth: "uchar"` regardless) — rather than fight
// that, hand-roll a minimal, spec-correct 16-bit RGB PNG (uncompressed-then-deflated scanlines,
// PNG color type 2 truecolor, bit depth 16) using only `node:zlib` (deflate + the built-in
// `zlib.crc32`, Node 21+). This guarantees a GENUINE 16-bit input regardless of sharp's raw-input
// quirk — sharp is then used only to DECODE this real file and produce the golden, which is
// exactly what the fixture needs to test.
function pngChunk(type, data) {
  const typeBuf = Buffer.from(type, 'ascii');
  const lenBuf = Buffer.alloc(4);
  lenBuf.writeUInt32BE(data.length, 0);
  const crcInput = Buffer.concat([typeBuf, data]);
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(zlib.crc32(crcInput) >>> 0, 0);
  return Buffer.concat([lenBuf, typeBuf, data, crcBuf]);
}

function encode16BitPng(width, height, getPixel) {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdrData = Buffer.alloc(13);
  ihdrData.writeUInt32BE(width, 0);
  ihdrData.writeUInt32BE(height, 4);
  ihdrData[8] = 16; // bit depth
  ihdrData[9] = 2; // color type: truecolor RGB
  ihdrData[10] = 0; // compression method
  ihdrData[11] = 0; // filter method
  ihdrData[12] = 0; // interlace method
  const ihdr = pngChunk('IHDR', ihdrData);

  const bytesPerPixel = 6; // 3 channels x 2 bytes/channel
  const stride = 1 + width * bytesPerPixel;
  const raw = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    const rowStart = y * stride;
    raw[rowStart] = 0; // filter type: None
    for (let x = 0; x < width; x++) {
      const [r, g, b] = getPixel(x, y);
      const off = rowStart + 1 + x * bytesPerPixel;
      raw.writeUInt16BE(r, off);
      raw.writeUInt16BE(g, off + 2);
      raw.writeUInt16BE(b, off + 4);
    }
  }
  const idat = pngChunk('IDAT', zlib.deflateSync(raw));
  const iend = pngChunk('IEND', Buffer.alloc(0));
  return Buffer.concat([sig, ihdr, idat, iend]);
}

function write(name, buf) {
  writeFileSync(join(DIR, name), buf);
  console.log(`wrote ${name} (${buf.length} bytes)`);
}

async function goldenProduct(inputBuf, label, { rotate = false } = {}) {
  let pipeline = sharp(inputBuf);
  if (rotate) pipeline = pipeline.rotate();
  const golden = await pipeline
    .resize({ width: PRODUCT.width, height: PRODUCT.height, fit: 'inside' })
    .webp({ quality: PRODUCT.quality })
    .toBuffer();
  write(`${label}-golden.webp`, golden);
}

// ── 1. Phone-style JPEG w/ EXIF orientation (a real decode+auto-orient path) ──
//
// IMPORTANT, verified empirically against this repo's pinned sharp (0.34.5): sharp does NOT
// auto-apply EXIF orientation just because `.resize()`/`.webp()` are called — it requires an
// EXPLICIT `.rotate()` (no args) in the pipeline. The OLD `spa-proxy.ts:222-226` product-image
// route (and `themes.ts:127-130` theme-logo) never calls `.rotate()` — so THEY do not correct
// orientation today either (only `spa-proxy.ts:279` entry-photo does). REV-S4-4 mandates this
// Rust port apply orientation correction on ALL THREE profiles regardless — a deliberate
// IMPROVEMENT over current Node behavior for product/logo, not a parity preservation, per the
// council resolution's "the port *improves* on parity" posture for exactly this class of
// PII/correctness issue. The golden for THIS fixture is therefore generated WITH an explicit
// `.rotate()` — it is the oracle for "does the resize+encode math match sharp, given correctly
// oriented pixels", not "does today's product-image route happen to look like this" (it does
// not; see the lane report for the empirical A/B that found this).
async function phoneJpeg() {
  // A small "photo-like" source (gradient, not flat — flat colors compress to a size that
  // isn't representative and can hide real JPEG-decode issues).
  const width = 300,
    height = 400; // portrait, as a phone photo would be BEFORE EXIF-correction
  const raw = Buffer.alloc(width * height * 3);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 3;
      raw[i] = Math.floor((x / width) * 255);
      raw[i + 1] = Math.floor((y / height) * 255);
      raw[i + 2] = 128;
    }
  }
  // orientation:6 = rotate 90 CW to display correctly — a very common phone-camera tag.
  const buf = await sharp(raw, { raw: { width, height, channels: 3 } })
    .jpeg({ quality: 90 })
    .withMetadata({ orientation: 6 })
    .toBuffer();
  write('phone-jpeg-exif6.jpg', buf);
  await goldenProduct(buf, 'phone-jpeg-exif6', { rotate: true });
}

// Coarse 20px blocks (not a per-pixel gradient) — PNG's filter+deflate compress large flat
// regions far better than a smooth gradient, keeping these fixtures genuinely "a few KB" while
// still exercising real multi-color content (not a degenerate flat field).
function blocky(x, y, width, height) {
  const bx = Math.floor((x / width) * 6);
  const by = Math.floor((y / height) * 6);
  const parity = (bx + by) % 2;
  return [parity === 0 ? 40 : 210, (bx * 40) % 256, (by * 40) % 256];
}

// ── 2. Plain PNG ──
async function plainPng() {
  const width = 240,
    height = 160;
  const raw = Buffer.alloc(width * height * 3);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 3;
      const [r, g, b] = blocky(x, y, width, height);
      raw[i] = r;
      raw[i + 1] = g;
      raw[i + 2] = b;
    }
  }
  const buf = await sharp(raw, { raw: { width, height, channels: 3 } })
    .png({ compressionLevel: 9 })
    .toBuffer();
  write('plain.png', buf);
  await goldenProduct(buf, 'plain-png');
}

// ── 3. Odd aspect ratio ──
async function oddAspect() {
  const width = 400,
    height = 24;
  const raw = Buffer.alloc(width * height * 3);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 3;
      const [r, g, b] = blocky(x, y, width, height);
      raw[i] = r;
      raw[i + 1] = g;
      raw[i + 2] = b;
    }
  }
  const buf = await sharp(raw, { raw: { width, height, channels: 3 } })
    .png({ compressionLevel: 9 })
    .toBuffer();
  write('odd-aspect.png', buf);
  await goldenProduct(buf, 'odd-aspect');
}

// ── 4. 1xN edge case ──
async function oneByN() {
  const width = 1,
    height = 300;
  const raw = Buffer.alloc(width * height * 3);
  for (let y = 0; y < height; y++) {
    const i = y * 3;
    const band = Math.floor((y / height) * 4) % 2;
    raw[i] = band === 0 ? 40 : 210;
    raw[i + 1] = 80;
    raw[i + 2] = 200;
  }
  const buf = await sharp(raw, { raw: { width, height, channels: 3 } })
    .png({ compressionLevel: 9 })
    .toBuffer();
  write('one-by-n.png', buf);
  await goldenProduct(buf, 'one-by-n');
}

// ── 5. CMYK JPEG ──
async function cmykJpeg() {
  const width = 160,
    height = 120;
  const raw = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      const [c, m] = blocky(x, y, width, height);
      raw[i] = c;
      raw[i + 1] = m;
      raw[i + 2] = 60; // Y
      raw[i + 3] = 10; // K
    }
  }
  const buf = await sharp(raw, { raw: { width, height, channels: 4 } })
    .toColourspace('cmyk')
    .jpeg({ quality: 90 })
    .toBuffer();
  write('cmyk.jpg', buf);
  await goldenProduct(buf, 'cmyk');
}

// ── 6. 16-bit PNG (hand-rolled — see the module-level comment on `encode16BitPng`) ──
async function sixteenBitPng() {
  const width = 80,
    height = 60;
  const buf = encode16BitPng(width, height, (x, y) => {
    const [r8, g8] = blocky(x, y, width, height);
    // Scale the coarse 8-bit block pattern up to the 16-bit range — keeps the file small
    // (large flat regions compress well) while genuinely exercising 16-bit sample depth.
    return [r8 * 257, g8 * 257, 30000];
  });
  const meta = await sharp(buf).metadata();
  if (meta.depth !== 'ushort' || meta.width !== width || meta.height !== height) {
    throw new Error(`hand-rolled 16-bit PNG failed sharp's own sanity check: ${JSON.stringify(meta)}`);
  }
  write('sixteen-bit.png', buf);
  await goldenProduct(buf, 'sixteen-bit');
}

// ── Orientation matrix: all 8 EXIF orientation values, same underlying "canonical after
// correction" quadrant-color arrangement, derived by hand (see rebuild lane report for the
// per-value stored-quadrant derivation) so a completely independent oracle (the EXIF standard's
// well-known transform table) — not sharp's or image's own code — decides what's correct.
const RED = [237, 28, 36];
const GREEN = [34, 177, 76];
const BLUE = [63, 72, 204];
const YELLOW = [255, 242, 0];

// stored[P00, P01, P10, P11] per orientation value 1..8 (hand-derived inverse transforms).
const STORED_BY_ORIENTATION = {
  1: [RED, GREEN, BLUE, YELLOW],
  2: [GREEN, RED, YELLOW, BLUE],
  3: [YELLOW, BLUE, GREEN, RED],
  4: [BLUE, YELLOW, RED, GREEN],
  5: [RED, BLUE, GREEN, YELLOW],
  6: [GREEN, YELLOW, RED, BLUE],
  7: [YELLOW, GREEN, BLUE, RED],
  8: [BLUE, RED, YELLOW, GREEN],
};

async function orientationMatrix() {
  const quad = 24; // px per quadrant side
  const size = quad * 2;
  for (const [value, [p00, p01, p10, p11]] of Object.entries(STORED_BY_ORIENTATION)) {
    const raw = Buffer.alloc(size * size * 3);
    const put = (x0, y0, color) => {
      for (let y = y0; y < y0 + quad; y++) {
        for (let x = x0; x < x0 + quad; x++) {
          const i = (y * size + x) * 3;
          raw[i] = color[0];
          raw[i + 1] = color[1];
          raw[i + 2] = color[2];
        }
      }
    };
    put(0, 0, p00);
    put(quad, 0, p01);
    put(0, quad, p10);
    put(quad, quad, p11);
    const buf = await sharp(raw, { raw: { width: size, height: size, channels: 3 } })
      .jpeg({ quality: 95 })
      .withMetadata({ orientation: Number(value) })
      .toBuffer();
    write(`orient-${value}.jpg`, buf);
  }
}

async function main() {
  await phoneJpeg();
  await plainPng();
  await oddAspect();
  await oneByN();
  await cmykJpeg();
  await sixteenBitPng();
  await orientationMatrix();
  console.log('done.');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
