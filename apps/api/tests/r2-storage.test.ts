import test from 'node:test';
import assert from 'node:assert/strict';
import { R2StorageProvider } from '../src/lib/r2-storage.js';

// The provider must fail fast on misconfiguration rather than silently writing
// nowhere, AND its put/get/delete must map content types, coerce NoSuchKey/404
// to null, and re-throw every other error. A live R2 round-trip (real bucket +
// credentials) is covered separately — see needs_staging in the task report.

const ENV_KEYS = ['R2_BUCKET', 'R2_ENDPOINT', 'R2_ACCESS_KEY_ID', 'R2_SECRET_ACCESS_KEY'] as const;

function setR2Env() {
  process.env.R2_BUCKET = 'dowiz-images';
  process.env.R2_ENDPOINT = 'https://acc.r2.cloudflarestorage.com';
  process.env.R2_ACCESS_KEY_ID = 'test-key';
  process.env.R2_SECRET_ACCESS_KEY = 'test-secret';
}

function snapshotEnv() {
  return Object.fromEntries(ENV_KEYS.map((k) => [k, process.env[k]])) as Record<string, string | undefined>;
}

function restoreEnv(orig: Record<string, string | undefined>) {
  for (const k of ENV_KEYS) {
    if (orig[k] === undefined) delete process.env[k];
    else process.env[k] = orig[k];
  }
}

type SentCommand = { constructor: { name: string }; input: Record<string, unknown> };

// Inject a fake S3 client so put/get/delete never touch the network. The real
// @aws-sdk command objects are still constructed inside the provider, so the
// Bucket/Key/ContentType mapping is exercised end-to-end.
function withFakeClient(provider: R2StorageProvider, send: (cmd: SentCommand) => unknown) {
  (provider as unknown as { clientPromise: Promise<unknown> }).clientPromise = Promise.resolve({ send });
}

function s3Error(props: { name?: string; httpStatusCode?: number; message?: string }) {
  const e = new Error(props.message ?? 'r2 error') as Error & { $metadata?: { httpStatusCode?: number } };
  if (props.name) e.name = props.name;
  if (props.httpStatusCode) e.$metadata = { httpStatusCode: props.httpStatusCode };
  return e;
}

test('R2StorageProvider config guards', async (t) => {
  const orig = snapshotEnv();
  t.after(() => restoreEnv(orig));

  await t.test('throws without R2_BUCKET', () => {
    setR2Env();
    delete process.env.R2_BUCKET;
    assert.throws(() => new R2StorageProvider(), /R2_BUCKET/);
  });

  await t.test('throws without R2_ENDPOINT', () => {
    setR2Env();
    delete process.env.R2_ENDPOINT;
    assert.throws(() => new R2StorageProvider(), /R2_ENDPOINT/);
  });

  await t.test('throws without R2_ACCESS_KEY_ID', () => {
    setR2Env();
    delete process.env.R2_ACCESS_KEY_ID;
    assert.throws(() => new R2StorageProvider(), /R2_ACCESS_KEY_ID/);
  });

  await t.test('throws without R2_SECRET_ACCESS_KEY', () => {
    setR2Env();
    delete process.env.R2_SECRET_ACCESS_KEY;
    assert.throws(() => new R2StorageProvider(), /R2_SECRET_ACCESS_KEY/);
  });

  await t.test('constructs a real instance carrying bucket state when fully configured', () => {
    setR2Env();
    const p = new R2StorageProvider();
    assert.ok(p instanceof R2StorageProvider);
    assert.equal((p as unknown as { bucket: string }).bucket, 'dowiz-images');
  });
});

test('R2StorageProvider object operations', async (t) => {
  const orig = snapshotEnv();
  setR2Env();
  t.after(() => restoreEnv(orig));

  await t.test('put maps the file extension to the S3 ContentType', async () => {
    const sent: SentCommand[] = [];
    const p = new R2StorageProvider();
    withFakeClient(p, (cmd) => {
      sent.push(cmd);
      return {};
    });
    await p.put('img/a.webp', Buffer.from('x'));
    await p.put('img/b.png', Buffer.from('x'));
    await p.put('img/c.jpg', Buffer.from('x'));
    await p.put('img/d.bin', Buffer.from('x'));
    assert.deepEqual(
      sent.map((c) => c.constructor.name),
      ['PutObjectCommand', 'PutObjectCommand', 'PutObjectCommand', 'PutObjectCommand'],
    );
    assert.equal(sent[0].input.ContentType, 'image/webp');
    assert.equal(sent[1].input.ContentType, 'image/png');
    assert.equal(sent[2].input.ContentType, 'image/jpeg');
    assert.equal(sent[3].input.ContentType, 'application/octet-stream');
    assert.equal(sent[0].input.Bucket, 'dowiz-images');
    assert.equal(sent[0].input.Key, 'img/a.webp');
  });

  await t.test('put does NOT forward the ttl argument to S3 (documents current no-expiry behavior)', async () => {
    const sent: SentCommand[] = [];
    const p = new R2StorageProvider();
    withFakeClient(p, (cmd) => {
      sent.push(cmd);
      return {};
    });
    await p.put('img/a.webp', Buffer.from('x'), 60);
    assert.equal(sent[0].input.Expires, undefined);
    assert.equal('ttlSeconds' in sent[0].input, false);
  });

  await t.test('get returns the object bytes as a Buffer on success', async () => {
    const p = new R2StorageProvider();
    withFakeClient(p, () => ({
      Body: { transformToByteArray: async () => new Uint8Array([1, 2, 3]) },
    }));
    const out = await p.get('img/a.webp');
    assert.ok(out instanceof Buffer);
    assert.deepEqual([...(out as Buffer)], [1, 2, 3]);
  });

  await t.test('get returns null when the response has no Body', async () => {
    const p = new R2StorageProvider();
    withFakeClient(p, () => ({}));
    assert.equal(await p.get('img/a.webp'), null);
  });

  await t.test('get coerces a NoSuchKey error to null', async () => {
    const p = new R2StorageProvider();
    withFakeClient(p, () => {
      throw s3Error({ name: 'NoSuchKey' });
    });
    assert.equal(await p.get('img/missing.webp'), null);
  });

  await t.test('get coerces a 404 metadata status to null', async () => {
    const p = new R2StorageProvider();
    withFakeClient(p, () => {
      throw s3Error({ httpStatusCode: 404 });
    });
    assert.equal(await p.get('img/missing.webp'), null);
  });

  await t.test('get re-throws non-404 errors instead of swallowing them', async () => {
    const p = new R2StorageProvider();
    withFakeClient(p, () => {
      throw s3Error({ name: 'AccessDenied', httpStatusCode: 403, message: 'denied' });
    });
    await assert.rejects(() => p.get('img/a.webp'), /denied/);
  });

  await t.test('delete sends a DeleteObjectCommand for the exact key', async () => {
    const sent: SentCommand[] = [];
    const p = new R2StorageProvider();
    withFakeClient(p, (cmd) => {
      sent.push(cmd);
      return {};
    });
    await p.delete('img/a.webp');
    assert.equal(sent[0].constructor.name, 'DeleteObjectCommand');
    assert.equal(sent[0].input.Bucket, 'dowiz-images');
    assert.equal(sent[0].input.Key, 'img/a.webp');
  });
});

// TODO(needs_staging): live R2 round-trip — put → get (bytes match) → delete → get (null)
// against a real bucket + credentials. Requires R2_* secrets; cannot run in-sandbox.
