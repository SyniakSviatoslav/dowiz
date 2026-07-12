import test from 'node:test';
import assert from 'node:assert/strict';
import {
  sniffMime,
  magicBytesMatch,
  isAllowedMime,
  isAllowedPosterMime,
  checkBudget,
  checkFrameCount,
  mediaServingAllowed,
  LOCATION_BUDGET_BYTES,
  SPIN_MIN_FRAMES,
  SPIN_MAX_FRAMES,
} from '../src/lib/product-media-validation.js';

// Build a buffer with a given magic-byte signature.
const webp = () => {
  const b = Buffer.alloc(16);
  b.write('RIFF', 0, 'ascii');
  b.write('WEBP', 8, 'ascii');
  return b;
};
const jpeg = () => Buffer.from([0xff, 0xd8, 0xff, 0xe0, 0, 0, 0, 0, 0, 0, 0, 0]);
const mp4 = () => {
  const b = Buffer.alloc(16);
  // size(4) then 'ftyp' at offset 4
  b.write('ftyp', 4, 'ascii');
  return b;
};
const svg = () => Buffer.from('<svg xmlns="http://www.w3.org/2000/svg"></svg>', 'utf8');
const exe = () => Buffer.from([0x4d, 0x5a, 0x90, 0x00, 0, 0, 0, 0, 0, 0, 0, 0]); // MZ

test('magic-byte detector', async (t) => {
  await t.test('webp RIFF/WEBP → image/webp', () => {
    assert.equal(sniffMime(webp()), 'image/webp');
    assert.ok(magicBytesMatch(webp(), 'image/webp'));
  });

  await t.test('jpeg FFD8FF → image/jpeg', () => {
    assert.equal(sniffMime(jpeg()), 'image/jpeg');
    assert.ok(magicBytesMatch(jpeg(), 'image/jpeg'));
  });

  await t.test('mp4 ftyp → video/mp4', () => {
    assert.equal(sniffMime(mp4()), 'video/mp4');
    assert.ok(magicBytesMatch(mp4(), 'video/mp4'));
  });

  await t.test('svg → null (rejected; active-content vector)', () => {
    assert.equal(sniffMime(svg()), null);
    assert.equal(magicBytesMatch(svg(), 'image/svg+xml'), false);
    assert.equal(magicBytesMatch(svg(), 'image/webp'), false);
  });

  await t.test('exe (MZ) → null (rejected)', () => {
    assert.equal(sniffMime(exe()), null);
  });

  await t.test('mislabelled bytes do not match claim', () => {
    // jpeg bytes claimed as webp must NOT pass.
    assert.equal(magicBytesMatch(jpeg(), 'image/webp'), false);
  });

  await t.test('too-short buffer → null', () => {
    assert.equal(sniffMime(Buffer.from([0xff, 0xd8])), null);
  });
});

test('mime allow-list', async (t) => {
  await t.test('webp/jpeg/mp4 allowed; svg never', () => {
    assert.ok(isAllowedMime('image/webp'));
    assert.ok(isAllowedMime('image/jpeg'));
    assert.ok(isAllowedMime('video/mp4'));
    assert.equal(isAllowedMime('image/svg+xml'), false);
    assert.equal(isAllowedMime('application/octet-stream'), false);
  });

  await t.test('poster is raster-only — webp/jpeg ok, mp4/svg not', () => {
    assert.ok(isAllowedPosterMime('image/webp'));
    assert.ok(isAllowedPosterMime('image/jpeg'));
    assert.equal(isAllowedPosterMime('video/mp4'), false);
    assert.equal(isAllowedPosterMime('image/svg+xml'), false);
  });
});

test('per-location budget check (150MB)', async (t) => {
  const MB = 1024 * 1024;

  await t.test('under budget → ok', () => {
    const r = checkBudget(100 * MB, 40 * MB);
    assert.equal(r.ok, true);
    assert.equal(r.total, 140 * MB);
    assert.equal(r.limit, LOCATION_BUDGET_BYTES);
  });

  await t.test('exactly at budget → ok', () => {
    assert.equal(checkBudget(150 * MB, 0).ok, true);
  });

  await t.test('over 150MB → reject', () => {
    const r = checkBudget(140 * MB, 20 * MB);
    assert.equal(r.ok, false);
    assert.equal(r.total > r.limit, true);
  });

  await t.test('incoming alone over budget → reject', () => {
    assert.equal(checkBudget(0, 200 * MB).ok, false);
  });
});

test('spin frame-count range [12, 72]', async (t) => {
  await t.test('11 → reject', () => assert.equal(checkFrameCount(SPIN_MIN_FRAMES - 1).ok, false));
  await t.test('12 → ok', () => assert.equal(checkFrameCount(SPIN_MIN_FRAMES).ok, true));
  await t.test('72 → ok', () => assert.equal(checkFrameCount(SPIN_MAX_FRAMES).ok, true));
  await t.test('73 → reject', () => assert.equal(checkFrameCount(SPIN_MAX_FRAMES + 1).ok, false));
  await t.test('non-integer → reject', () => assert.equal(checkFrameCount(12.5).ok, false));
});

test('tier/flag serving gate (returns [] when off)', async (t) => {
  await t.test('flag off → not allowed (even on business plan)', () => {
    assert.equal(mediaServingAllowed(false, 'business'), false);
  });
  await t.test('flag on but free plan → not allowed', () => {
    assert.equal(mediaServingAllowed(true, 'free'), false);
  });
  await t.test('flag on but null/unknown plan → not allowed', () => {
    assert.equal(mediaServingAllowed(true, null), false);
    assert.equal(mediaServingAllowed(true, undefined), false);
  });
  await t.test('flag on + business plan → allowed', () => {
    assert.equal(mediaServingAllowed(true, 'business'), true);
  });
});
