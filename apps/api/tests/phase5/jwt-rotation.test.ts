import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { importJWK, exportJWK, SignJWT, jwtVerify, generateKeyPair } from 'jose';

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

    try {
      await jwtVerify(token, keyLookup, { algorithms: ['RS256'] });
      assert.fail('Should have thrown: key not found');
    } catch (err: any) {
      assert.ok(err.message.includes('key not found') || err.code === 'ERR_JOSE_JWK_KEY_MISMATCH',
        `Expected key-not-found error, got: ${err.message}`);
    }
  });
});
