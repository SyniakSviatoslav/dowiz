import test from 'node:test';
import assert from 'node:assert/strict';
import { R2StorageProvider } from '../src/lib/r2-storage.js';

// The provider must fail fast on misconfiguration rather than silently writing
// nowhere. (Full put/get/delete round-trip is verified live once an R2 bucket +
// credentials exist — it delegates to @aws-sdk/client-s3.)
test('R2StorageProvider config guards', async (t) => {
  const origBucket = process.env.R2_BUCKET;
  const origEndpoint = process.env.R2_ENDPOINT;
  t.after(() => {
    origBucket === undefined ? delete process.env.R2_BUCKET : (process.env.R2_BUCKET = origBucket);
    origEndpoint === undefined ? delete process.env.R2_ENDPOINT : (process.env.R2_ENDPOINT = origEndpoint);
  });

  await t.test('throws without R2_BUCKET', () => {
    delete process.env.R2_BUCKET;
    process.env.R2_ENDPOINT = 'https://acc.r2.cloudflarestorage.com';
    assert.throws(() => new R2StorageProvider(), /R2_BUCKET/);
  });

  await t.test('throws without R2_ENDPOINT', () => {
    process.env.R2_BUCKET = 'dowiz-images';
    delete process.env.R2_ENDPOINT;
    assert.throws(() => new R2StorageProvider(), /R2_ENDPOINT/);
  });

  await t.test('constructs when both are set', () => {
    process.env.R2_BUCKET = 'dowiz-images';
    process.env.R2_ENDPOINT = 'https://acc.r2.cloudflarestorage.com';
    assert.ok(new R2StorageProvider());
  });
});
