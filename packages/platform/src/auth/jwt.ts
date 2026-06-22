import { SignJWT, jwtVerify, decodeProtectedHeader } from 'jose';
import { AuthToken } from '@deliveryos/shared-types';
import { loadEnv } from '@deliveryos/config';
import crypto from 'crypto';

let _env: ReturnType<typeof loadEnv> | null = null;
function getEnv() {
  if (!_env) _env = loadEnv();
  return _env;
}
function getKid() { return getEnv().JWT_KID; }

function getPrivateKey(): crypto.KeyObject {
  const raw = getEnv().***REDACTED***;
  if (!raw) throw new Error('***REDACTED*** environment variable is required for RS256 signing');
  const pem = raw.replace(/\\n/g, '\n');
  return crypto.createPrivateKey(pem);
}

function getPublicKey(): crypto.KeyObject {
  const raw = getEnv().***REDACTED***;
  if (!raw) throw new Error('***REDACTED*** environment variable is required for RS256 verification');
  const pem = raw.replace(/\\n/g, '\n');
  return crypto.createPublicKey(pem);
}

// ── Dev-kid segregation (ADR-0003) ──
// Dev/mock tokens are signed under JWT_DEV_KID with the dev keypair so the prod verifier
// — which neither holds the dev public key nor accepts the dev kid — rejects them
// cryptographically. These return null when the dev keypair is absent (i.e. on prod).
function getDevPrivateKey(): crypto.KeyObject | null {
  const raw = getEnv().JWT_DEV_PRIVATE_KEY;
  if (!raw) return null;
  return crypto.createPrivateKey(raw.replace(/\\n/g, '\n'));
}

function getDevPublicKey(): crypto.KeyObject | null {
  const raw = getEnv().JWT_DEV_PUBLIC_KEY;
  if (!raw) return null;
  return crypto.createPublicKey(raw.replace(/\\n/g, '\n'));
}

type SignablePayload = Omit<AuthToken, 'iat' | 'exp' | 'kid' | 'sub'> & { sub?: string, kid?: string };

// Sign with an explicit (kid, privateKey) pair so the protected-header kid and the
// signing key can never diverge (the C.1 invariant — a dev kid signed with the prod
// key would be unverifiable). The body-claim kid is kept consistent with the header.
async function signWith(payload: SignablePayload, expiresIn: string, kid: string, privateKey: crypto.KeyObject): Promise<string> {
  const subValue = payload.sub || (('userId' in payload) ? (payload as any).userId : crypto.randomUUID());
  const jwtPayload = {
    ...payload,
    sub: subValue as string,
    kid,
  };
  return new SignJWT(jwtPayload as any)
    .setProtectedHeader({ alg: 'RS256', kid })
    .setIssuedAt()
    .setExpirationTime(expiresIn)
    .sign(privateKey);
}

/** Production/normal token signing — always the env kid + env private key. */
export async function signAuthToken(payload: SignablePayload, expiresIn: string): Promise<string> {
  return signWith(payload, expiresIn, getKid(), getPrivateKey());
}

/**
 * Dev/mock token signing (ADR-0003) — signs under JWT_DEV_KID with the dev keypair so
 * the token is cryptographically rejected by a prod verifier. Throws if the dev keypair
 * is not configured, so a dev-mint site can NEVER silently fall back to prod-key signing
 * (which would re-create the backdoor). Only reachable behind the ALLOW_DEV_LOGIN gate.
 */
export async function signDevToken(payload: SignablePayload, expiresIn: string): Promise<string> {
  const env = getEnv();
  const devKey = getDevPrivateKey();
  if (!env.JWT_DEV_KID || !devKey) {
    throw new Error('signDevToken requires JWT_DEV_KID + JWT_DEV_PRIVATE_KEY (non-prod only)');
  }
  return signWith(payload, expiresIn, env.JWT_DEV_KID, devKey);
}

export async function verifyAuthToken(token: string): Promise<AuthToken> {
  const env = getEnv();
  // Select the trusted verification key by the (unverified) header kid BEFORE checking
  // the signature — standard JWKS pattern. The signature check below still gates
  // acceptance; the header is used only to pick which trusted key to verify against.
  const header = decodeProtectedHeader(token);
  // The dev kid is accepted ONLY in non-prod AND only when a dev keypair is present.
  // On prod NODE_ENV short-circuits this to false, so a dev-kid token is rejected
  // regardless of JWT_DEV_KID — and prod holds no dev public key anyway (defence in depth).
  const acceptDevKid = env.NODE_ENV !== 'production' && !!env.JWT_DEV_KID;

  let key: crypto.KeyObject;
  if (header.kid === getKid()) {
    key = getPublicKey();
  } else if (acceptDevKid && header.kid === env.JWT_DEV_KID) {
    const devPub = getDevPublicKey();
    if (!devPub) throw new Error('Invalid Key ID');
    key = devPub;
  } else {
    throw new Error('Invalid Key ID');
  }

  // Reject alg=none by not passing it as an allowed algorithm; jose rejects alg=none.
  const { payload, protectedHeader } = await jwtVerify(token, key, {
    algorithms: ['RS256']
  });

  if (protectedHeader.alg !== 'RS256') {
    throw new Error('Invalid algorithm — only RS256 accepted');
  }

  // Parse and strict-validate claims via Zod
  return AuthToken.parse(payload);
}

export async function issueCustomerToken(params: {
  orderId: string;
  locationId: string;
  customerId: string;
}): Promise<string> {
  // P0-PII: never embed the customer phone in the JWT. Phone is PII and the token
  // is a long-lived (7d) bearer credential held client-side; consumers must look
  // the phone up server-side via orderId / sub. The token's authority is the
  // (orderId, locationId, customerId) tuple, which is sufficient for scoping.
  return signAuthToken({
    role: 'customer',
    orderId: params.orderId,
    locationId: params.locationId,
    sub: params.customerId,
  } as any, '7d');
}
