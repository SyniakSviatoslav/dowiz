import test from 'node:test';
import assert from 'node:assert/strict';

// ADR brand-extractor-ssrf: the auto-branding website fetch must re-validate EVERY
// redirect hop, not just the initial URL — otherwise an attacker host can 302 to cloud
// metadata (169.254.169.254), loopback, or an internal address and the server follows.
// We drive extractFromWebsite with a mocked global fetch and a PUBLIC IP-literal entry
// (so assertPublicUrl takes the no-DNS path) and assert internal redirects are refused.

const realFetch = globalThis.fetch;

function resp(status: number, location: string | null, body = ''): any {
  return {
    ok: status >= 200 && status < 300,
    status,
    headers: { get: (k: string) => (k.toLowerCase() === 'location' ? location : null) },
    body: undefined,            // forces the res.text() path in fetchText
    text: async () => body,
  };
}

const PUBLIC = 'http://93.184.216.34/';   // public IP literal — no DNS lookup
const METADATA = 'http://169.254.169.254/latest/meta-data/';

test('brand-extractor SSRF redirect guard', async (t) => {
  const { extractFromWebsite } = await import('../src/lib/brand-extractor.js');

  await t.test('a redirect to the cloud-metadata IP is REFUSED (not followed)', async () => {
    globalThis.fetch = (async (u: any) => {
      const url = String(u);
      if (url.includes('169.254.169.254')) throw new Error('SSRF: metadata must never be fetched');
      return resp(302, METADATA);
    }) as any;
    await assert.rejects(() => extractFromWebsite(PUBLIC), /internal address|internal host/i);
  });

  await t.test('a redirect to loopback is REFUSED', async () => {
    globalThis.fetch = (async (u: any) => {
      const url = String(u);
      if (url.includes('127.0.0.1')) throw new Error('SSRF: loopback must never be fetched');
      return resp(302, 'http://127.0.0.1:8080/');
    }) as any;
    await assert.rejects(() => extractFromWebsite(PUBLIC), /internal address|internal host/i);
  });

  await t.test('more than 3 redirect hops is refused', async () => {
    globalThis.fetch = (async () => resp(302, PUBLIC)) as any; // infinite public redirect loop
    await assert.rejects(() => extractFromWebsite(PUBLIC), /Too many redirects/i);
  });

  await t.test('a direct 200 (no redirect) still works — no regression', async () => {
    globalThis.fetch = (async () => resp(200, null, '<html><title>Acme</title></html>')) as any;
    const sig = await extractFromWebsite(PUBLIC);
    assert.ok(sig, 'a public 200 should yield a brand signal');
  });

  globalThis.fetch = realFetch;
});
