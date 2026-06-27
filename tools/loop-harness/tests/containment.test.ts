import { test } from 'node:test';
import assert from 'node:assert/strict';
import { findCredentials, checkCredentialIsolation, assertCredentialIsolation, isTrustedSource } from '../src/containment.js';

test('findCredentials — flags secret-shaped names with non-empty values; ignores benign + empty', () => {
  const env = { DATABASE_URL: 'postgres://x', JWT_PRIVATE_KEY: 'k', FLY_API_TOKEN: 't', PATH: '/usr/bin', EMPTY_SECRET: '' };
  assert.deepEqual(findCredentials(env), ['DATABASE_URL', 'FLY_API_TOKEN', 'JWT_PRIVATE_KEY']);
});

test('findCredentials — allow-list excludes known-safe stubs', () => {
  const env = { DATABASE_URL: 'x', DEV_AUTH_SECRET: 'stg-e2e-secret' };
  assert.deepEqual(findCredentials(env, ['DEV_AUTH_SECRET']), ['DATABASE_URL']);
});

test('checkCredentialIsolation — clean env is ok', () => {
  assert.deepEqual(checkCredentialIsolation({ PATH: '/bin', NODE_ENV: 'test' }), { ok: true, present: [] });
});

test('checkCredentialIsolation — secrets present is not ok and lists the offending names', () => {
  assert.deepEqual(checkCredentialIsolation({ JWT_PRIVATE_KEY: 'k' }), { ok: false, present: ['JWT_PRIVATE_KEY'] });
});

test('assertCredentialIsolation — throws with count + offending names when secrets present', () => {
  assert.throws(
    () => assertCredentialIsolation({ JWT_PRIVATE_KEY: 'k' }),
    /CONTAINMENT \(§4\).*1 credential-shaped env var\(s\) in context: JWT_PRIVATE_KEY\./s,
  );
});

test('assertCredentialIsolation — clean env does not throw', () => {
  assert.doesNotThrow(() => assertCredentialIsolation({ NODE_ENV: 'test' }));
});

test('isTrustedSource — only allowlisted mechanical detectors; web/LLM is untrusted', () => {
  assert.equal(isTrustedSource('config-tune detector (operator-declared tunable)'), true);
  assert.equal(isTrustedSource('web research'), false);
  assert.equal(isTrustedSource('llm-suggested patch'), false);
});

test('isTrustedSource — exact match only; a prefix/substring of a trusted source is untrusted', () => {
  // Allowlist must be injection-resistant: no startsWith/includes leniency.
  assert.equal(isTrustedSource('config-tune detector'), false);
  assert.equal(isTrustedSource('config-tune detector (operator-declared tunable) + llm patch'), false);
  assert.equal(isTrustedSource('detector'), false);
});
