// Shared test env stub. Several modules call loadEnv() at import time, which throws
// if required vars (REDIS_URL, DATABASE_URL_*, JWT_*, …) are absent — crashing
// otherwise-PURE unit tests at module load in a no-infra environment.
//
// Import this FIRST (`import './_env-stub.js';`) in any test that statically imports
// server/platform code: ES-module imports evaluate depth-first in order, so this
// module's side effect sets the env BEFORE the loadEnv-triggering import is evaluated.
// It only fills MISSING vars, so a real CI env (with real infra) is never overridden.
import crypto from 'node:crypto';

const STUB: Record<string, string> = {
  NODE_ENV: 'test',
  APP_BASE_URL: 'http://localhost:3000',
  DATABASE_URL_OPERATIONAL: 'postgres://u:p@localhost:5432/db',
  DATABASE_URL_SESSION: 'postgres://u:p@localhost:5432/db',
  DATABASE_URL_MIGRATIONS: 'postgres://u:p@localhost:5432/db',
  REDIS_URL: 'redis://localhost:6379',
  JWT_KID: 'test',
  GOOGLE_CLIENT_ID: 'test',
  GOOGLE_CLIENT_SECRET: 'test',
  VAPID_PUBLIC_KEY: 'test',
  VAPID_PRIVATE_KEY: 'test',
  IP_HASH_SALT: 'test',
};
for (const [k, v] of Object.entries(STUB)) if (!process.env[k]) process.env[k] = v;

// RS256 JWT keys must be a REAL keypair (a dummy string fails the crypto decoder),
// so sign/verify in auth tests actually works. Generate an ephemeral pair once.
if (!process.env.JWT_PRIVATE_KEY) {
  const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
    modulusLength: 2048,
    publicKeyEncoding: { type: 'spki', format: 'pem' },
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
  });
  process.env.JWT_PRIVATE_KEY = privateKey;
  process.env.JWT_PUBLIC_KEY = publicKey;
}
