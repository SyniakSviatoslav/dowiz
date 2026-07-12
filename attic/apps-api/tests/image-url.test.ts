import test from 'node:test';
import assert from 'node:assert/strict';
import { getImageUrl } from '../src/lib/image-url.js';

// The S3 API endpoint (R2_ENDPOINT) is private — a browser URL built from it
// 400s without SigV4 signing. With a private bucket, images must resolve to the
// app's /images/* proxy; only an explicit public bucket/CDN (R2_PUBLIC_URL)
// yields a direct URL.
test('getImageUrl', async (t) => {
  const save = { pub: process.env.R2_PUBLIC_URL, ep: process.env.R2_ENDPOINT, bk: process.env.R2_BUCKET, ab: process.env.APP_BASE_URL };
  const restore = (k: string, v: string | undefined) => v === undefined ? delete (process.env as any)[k] : (process.env[k] = v);
  t.after(() => { restore('R2_PUBLIC_URL', save.pub); restore('R2_ENDPOINT', save.ep); restore('R2_BUCKET', save.bk); restore('APP_BASE_URL', save.ab); });

  await t.test('null/empty → null', () => {
    assert.equal(getImageUrl(null), null);
    assert.equal(getImageUrl(undefined), null);
  });

  await t.test('absolute / data URLs pass through', () => {
    assert.equal(getImageUrl('https://x.com/a.jpg'), 'https://x.com/a.jpg');
    assert.equal(getImageUrl('data:image/png;base64,AAAA'), 'data:image/png;base64,AAAA');
  });

  await t.test('private bucket (R2 set, no public URL) → app /images/* proxy, NOT a direct R2 URL', () => {
    delete process.env.R2_PUBLIC_URL;
    process.env.R2_ENDPOINT = 'https://acc.r2.cloudflarestorage.com';
    process.env.R2_BUCKET = 'dowiz-images';
    process.env.APP_BASE_URL = 'https://dowiz.fly.dev';
    const url = getImageUrl('loc1/p1.webp');
    assert.equal(url, 'https://dowiz.fly.dev/images/loc1/p1.webp');
    assert.ok(!url!.includes('r2.cloudflarestorage.com'), 'must not build a direct private-R2 URL');
  });

  await t.test('public bucket/CDN (R2_PUBLIC_URL) → direct URL', () => {
    process.env.R2_PUBLIC_URL = 'https://cdn.dowiz.app';
    assert.equal(getImageUrl('loc1/p1.webp'), 'https://cdn.dowiz.app/loc1/p1.webp');
  });
});
