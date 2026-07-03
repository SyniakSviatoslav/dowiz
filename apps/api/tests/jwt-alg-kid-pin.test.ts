import './_env-stub.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { SignJWT } from 'jose';
import { signAuthToken, verifyAuthToken } from '@deliveryos/platform';

// ── B3 P0-5 · W1-adjacent — JWT alg/kid PIN lock (ADR-0003) ──
//
// The real verifier (`packages/platform/src/auth/jwt.ts`) is already hardened: it selects the
// trusted key by header `kid`, then verifies with `algorithms: ['RS256']` and re-asserts
// `protectedHeader.alg === 'RS256'`. This locks that posture against the two classic JWT breaks
// so a future refactor that loosens the verifier goes RED:
//   1. alg=none         — the unsigned-token bypass.
//   2. alg=HS256        — the RS256→HS256 alg-confusion attack (HMAC the token with the *public*
//                          key as the secret; a verifier that treats the RSA public key as an HMAC
//                          key would accept it).
//   3. missing/wrong kid — the verifier must refuse to pick a key when kid is absent or unknown.
//
// These drive the REAL exported `verifyAuthToken` — not a mock jose keyset — so the guardrail
// tracks the shipped verifier, not a re-implementation of it.

const KID = process.env.JWT_KID as string;           // 'test' (from _env-stub)
const PUBLIC_PEM = process.env.JWT_PUBLIC_KEY as string;
const PRIVATE_PEM = process.env.JWT_PRIVATE_KEY as string;

const ownerClaims = { role: 'owner', userId: '11111111-1111-1111-1111-111111111111' };

function unsignedNoneToken(): string {
  // Hand-craft an alg=none token: header.payload. (no signature segment content).
  const b64 = (o: unknown) => Buffer.from(JSON.stringify(o)).toString('base64url');
  const header = b64({ alg: 'none', typ: 'JWT', kid: KID });
  const payload = b64({ ...ownerClaims, sub: ownerClaims.userId, iat: Math.floor(Date.now() / 1000) });
  return `${header}.${payload}.`;
}

test('JWT verifier pins alg=RS256 + requires a known kid (alg-confusion lock)', async (t) => {
  await t.test('control: a valid RS256 token with the env kid VERIFIES', async () => {
    const good = await signAuthToken(ownerClaims as any, '1h');
    const decoded = await verifyAuthToken(good);
    assert.equal(decoded.role, 'owner');
  });

  await t.test('alg=none unsigned token is REJECTED', async () => {
    await assert.rejects(() => verifyAuthToken(unsignedNoneToken()));
  });

  await t.test('alg=HS256 confusion (HMAC with the RSA public key) is REJECTED', async () => {
    // The attacker signs HS256 using the PUBLIC key bytes as the shared secret.
    const forged = await new SignJWT({ ...ownerClaims, sub: ownerClaims.userId })
      .setProtectedHeader({ alg: 'HS256', kid: KID })
      .setIssuedAt()
      .setExpirationTime('1h')
      .sign(new TextEncoder().encode(PUBLIC_PEM));
    await assert.rejects(() => verifyAuthToken(forged));
  });

  // NOTE: these forged payloads are made fully Zod-VALID (kid body-claim + uuid sub) on
  // purpose — so the header-kid selection in the verifier is the SOLE gate. If it were the
  // downstream `AuthToken.parse` rejecting them, this guardrail would pass even with the
  // kid-pin removed (a false green). Isolating the pin is what gives it teeth.
  await t.test('a token with an UNKNOWN kid is REJECTED (no key selected)', async () => {
    const privKey = crypto.createPrivateKey(PRIVATE_PEM);
    const forged = await new SignJWT({ ...ownerClaims, sub: ownerClaims.userId, kid: 'attacker-kid' })
      .setProtectedHeader({ alg: 'RS256', kid: 'attacker-kid' })
      .setIssuedAt()
      .setExpirationTime('1h')
      .sign(privKey);
    await assert.rejects(() => verifyAuthToken(forged), /Invalid Key ID/);
  });

  await t.test('a token with NO kid is REJECTED (kid is required)', async () => {
    const privKey = crypto.createPrivateKey(PRIVATE_PEM);
    const forged = await new SignJWT({ ...ownerClaims, sub: ownerClaims.userId, kid: KID })
      .setProtectedHeader({ alg: 'RS256' })
      .setIssuedAt()
      .setExpirationTime('1h')
      .sign(privKey);
    await assert.rejects(() => verifyAuthToken(forged), /Invalid Key ID/);
  });
});
