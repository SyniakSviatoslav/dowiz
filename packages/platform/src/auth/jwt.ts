import { SignJWT, jwtVerify } from 'jose';
import { AuthToken } from '@deliveryos/shared-types';
import { loadEnv } from '@deliveryos/config';
import crypto from 'crypto';

const env = loadEnv();
const kid = env.JWT_KID;

function getPrivateKey(): crypto.KeyObject {
  const raw = env.JWT_PRIVATE_KEY;
  if (!raw) throw new Error('JWT_PRIVATE_KEY environment variable is required for RS256 signing');
  const pem = raw.replace(/\\n/g, '\n');
  return crypto.createPrivateKey(pem);
}

function getPublicKey(): crypto.KeyObject {
  const raw = env.JWT_PUBLIC_KEY;
  if (!raw) throw new Error('JWT_PUBLIC_KEY environment variable is required for RS256 verification');
  const pem = raw.replace(/\\n/g, '\n');
  return crypto.createPublicKey(pem);
}

export async function signAuthToken(payload: Omit<AuthToken, 'iat' | 'exp' | 'kid' | 'sub'> & { sub?: string, kid?: string }, expiresIn: string): Promise<string> {
  const subValue = payload.sub || (('userId' in payload) ? (payload as any).userId : crypto.randomUUID());
  const jwtPayload = {
    ...payload,
    sub: subValue as string,
    kid: payload.kid || kid
  };

  const jwt = new SignJWT(jwtPayload as any)
    .setProtectedHeader({ alg: 'RS256', kid })
    .setIssuedAt()
    .setExpirationTime(expiresIn);

  return jwt.sign(getPrivateKey());
}

export async function verifyAuthToken(token: string): Promise<AuthToken> {
  // Reject alg=none by not passing it as an allowed algorithm
  // jose automatically rejects alg=none
  const { payload, protectedHeader } = await jwtVerify(token, getPublicKey(), {
    algorithms: ['RS256']
  });

  if (protectedHeader.alg !== 'RS256') {
    throw new Error('Invalid algorithm — only RS256 accepted');
  }

  if (protectedHeader.kid !== kid) {
    throw new Error('Invalid Key ID');
  }

  // Parse and strict-validate claims via Zod
  return AuthToken.parse(payload);
}

export async function issueCustomerToken(params: {
  orderId: string;
  locationId: string;
  phone: string;
  customerId: string;
}): Promise<string> {
  return signAuthToken({
    role: 'customer',
    orderId: params.orderId,
    locationId: params.locationId,
    phone: params.phone,
    sub: params.customerId,
  } as any, '7d');
}
