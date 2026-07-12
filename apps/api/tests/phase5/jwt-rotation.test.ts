import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { importJWK, exportJWK, SignJWT, jwtVerify, generateKeyPair, exportSPKI } from 'jose';

// inline (file is under apps/api/tests/, not e2e/) — crypto.randomUUID() emits v4
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const expectUuid = (v: unknown, label: string) =>
  assert.match(String(v), UUID_RE, `${label} should be a UUID, got: ${String(v)}`);

test('H2: JWT rotation test', async (t) => {
  const kidV1 = 'v1_test_' + crypto.randomUUID().slice(0, 8);
  const kidV2 = 'v2_test_' + crypto.randomUUID().slice(0, 8);

  // Generate two RSA key pairs
  const { publicKey: pubV1, privateKey: privV1 } = await generateKeyPair('RS256', { modulusLength: 2048 });
  const { publicKey: pubV2, privateKey: privV2 } = await generateKeyPair('RS256', { modulusLength: 2048 });

  const payload = { sub: 'test_user', role: 'owner', locationId: crypto.randomUUID() };

  await t.test('R1: Sign with kid=v1, verify with v1', async () => {
    const token = await new SignJWT(payload)
      .setProtectedHeader({ alg: 'RS256', kid: kidV1 })
      .setIssuedAt()
      .setExpirationTime('1h')
      .sign(privV1);

    const { payload: verified, protectedHeader } = await jwtVerify(token, pubV1, {
      algorithms: ['RS256'],
    });
    assert.strictEqual(protectedHeader.kid, kidV1);
    assert.strictEqual(verified.sub, 'test_user');
    // tenant claim must survive the round-trip as a real UUID (truthy-on-id guard)
    expectUuid(verified.locationId, 'verified.locationId');
    assert.strictEqual(verified.locationId, payload.locationId);
  });

  await t.test('R2: Rotate to kid=v2 as active signer', async () => {
    const token = await new SignJWT(payload)
      .setProtectedHeader({ alg: 'RS256', kid: kidV2 })
      .setIssuedAt()
      .setExpirationTime('1h')
      .sign(privV2);

    const { protectedHeader } = await jwtVerify(token, pubV2, {
      algorithms: ['RS256'],
    });
    assert.strictEqual(protectedHeader.kid, kidV2);
  });

  await t.test('R3: Old token kid=v1 still verifiable with v1 key in key set', async () => {
    const token = await new SignJWT(payload)
      .setProtectedHeader({ alg: 'RS256', kid: kidV1 })
      .setIssuedAt()
      .setExpirationTime('1h')
      .sign(privV1);

    // Simulate key set with both v1 and v2
    const keySet = new Map<string, CryptoKey>();
    keySet.set(kidV1, pubV1);
    keySet.set(kidV2, pubV2);

    const keyLookup = async (protectedHeader: any) => {
      const key = keySet.get(protectedHeader.kid);
      if (!key) throw new Error('key not found');
      return key;
    };

    const { protectedHeader } = await jwtVerify(token, keyLookup, {
      algorithms: ['RS256'],
    });
    assert.strictEqual(protectedHeader.kid, kidV1);
  });

  await t.test('R4: Remove kid=v1, old tokens are rejected', async () => {
    const token = await new SignJWT(payload)
      .setProtectedHeader({ alg: 'RS256', kid: kidV1 })
      .setIssuedAt()
      .setExpirationTime('1h')
      .sign(privV1);

    const keySet = new Map<string, CryptoKey>();
    keySet.set(kidV2, pubV2); // v1 removed

    const keyLookup = async (protectedHeader: any) => {
      const key = keySet.get(protectedHeader.kid);
      if (!key) throw new Error('key not found');
      return key;
    };

    // assert.rejects cannot swallow its own failure (prior try/catch caught its own assert.fail)
    await assert.rejects(
      jwtVerify(token, keyLookup, { algorithms: ['RS256'] }),
      (err: any) =>
        err.message.includes('key not found') || err.code === 'ERR_JOSE_JWK_KEY_MISMATCH',
    );
  });

  await t.test('R5: alg-confusion — none and HS256(pubkey-as-secret) are rejected', async () => {
    // 1) alg:'none' unsecured token must be refused by an RS256-only verifier
    const noneToken =
      Buffer.from(JSON.stringify({ alg: 'none', kid: kidV1 })).toString('base64url') +
      '.' +
      Buffer.from(JSON.stringify(payload)).toString('base64url') +
      '.';
    await assert.rejects(
      jwtVerify(noneToken, pubV1, { algorithms: ['RS256'] }),
      (err: any) => err.code === 'ERR_JOSE_ALG_NOT_ALLOWED',
    );

    // 2) classic RS->HS downgrade: sign HS256 using the RSA public key bytes as the HMAC secret
    const pubSpki = await exportSPKI(pubV1);
    const hsSecret = new TextEncoder().encode(pubSpki);
    const hsToken = await new SignJWT(payload)
      .setProtectedHeader({ alg: 'HS256', kid: kidV1 })
      .setIssuedAt()
      .setExpirationTime('1h')
      .sign(hsSecret);
    await assert.rejects(
      jwtVerify(hsToken, pubV1, { algorithms: ['RS256'] }),
      (err: any) => err.code === 'ERR_JOSE_ALG_NOT_ALLOWED',
    );
  });

  await t.test('R6: expired token (exp in the past) is rejected', async () => {
    const past = Math.floor(Date.now() / 1000) - 3600;
    const expiredToken = await new SignJWT(payload)
      .setProtectedHeader({ alg: 'RS256', kid: kidV1 })
      .setIssuedAt(past)
      .setExpirationTime(past + 60) // exp ~59min in the past
      .sign(privV1);
    await assert.rejects(
      jwtVerify(expiredToken, pubV1, { algorithms: ['RS256'] }),
      (err: any) => err.code === 'ERR_JWT_EXPIRED',
    );
  });

  // TODO(needs_staging): claim-level cross-tenant rejection (token w/ locationId-A presented to a
  // verifier/endpoint scoped to locationId-B) cannot be proven here — jose only validates signature/
  // alg/exp, not tenant authorization. Real IDOR proof requires the app's auth middleware + a 2nd
  // real tenant on https://dowiz-staging.fly.dev. See findings #2 (app key-store) and #4 (IDOR).
});
