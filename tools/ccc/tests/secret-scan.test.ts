import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { buildIndex } from '../src/indexer.js';
import { isSecretPath, loadIgnore } from '../src/ignore.js';

// ADR-0012 C1 — the secret-scan MERGE GATE. ccc must consult ignore rules BEFORE reading bytes
// (B10), never index a secret, and never write into dist/. Proof is structural (the recorded
// read-set), not just output-scanning: a secret that is never READ cannot leak no matter how the
// index is later serialized. RED→GREEN: a read-then-filter walker would have `.env` in readPaths.

const SECRET_ENV = 'SUPER_SECRET_API_KEY=sk_live_zzz_do_not_index_111';
const SECRET_GITIGNORED = 'const LEAKED_TOKEN = "gitignored_secret_abc_222";';

function makeFixture(): string {
  const dir = mkdtempSync(join(tmpdir(), 'ccc-secret-scan-'));
  // A real secret on disk.
  writeFileSync(join(dir, '.env'), SECRET_ENV);
  writeFileSync(join(dir, '.env.production'), 'PROD_SECRET=sk_live_prod_333');
  // A .gitignore that hides a file carrying a secret.
  writeFileSync(join(dir, '.gitignore'), 'ignored-secret.ts\nnode_modules/\n');
  writeFileSync(join(dir, 'ignored-secret.ts'), SECRET_GITIGNORED);
  // A private key (hard-deny, independent of .gitignore).
  writeFileSync(join(dir, 'server.key'), '-----BEGIN PRIVATE KEY-----\nMIIabc\n-----END PRIVATE KEY-----');
  // A documentation example — explicitly NOT a secret, may be indexed.
  writeFileSync(join(dir, '.env.example'), 'API_KEY=your-key-here');
  // A normal source file the indexer SHOULD pick up.
  writeFileSync(join(dir, 'normal.ts'), 'export function publicHandler(x: number) { return x + 1; }\nexport class Widget {}\n');
  // A node_modules secret — must be skipped wholesale.
  mkdirSync(join(dir, 'node_modules', 'evil'), { recursive: true });
  writeFileSync(join(dir, 'node_modules', 'evil', 'index.ts'), 'const NM_SECRET = "node_modules_secret_444";');
  return dir;
}

test('ccc secret-scan merge gate', async (t) => {
  await t.test('isSecretPath hard-denies .env*/keys but allows .env.example', () => {
    assert.equal(isSecretPath('.env'), true);
    assert.equal(isSecretPath('apps/api/.env.production'), true);
    assert.equal(isSecretPath('server.key'), true);
    assert.equal(isSecretPath('certs/tls.pem'), true);
    assert.equal(isSecretPath('.fly-staging-token'), true);
    // Exercise the remaining hard-deny patterns so all 10 SECRET_DENY entries have a red→green guard.
    assert.equal(isSecretPath('certs/keystore.p12'), true);
    assert.equal(isSecretPath('certs/client.pfx'), true);
    assert.equal(isSecretPath('android/release.keystore'), true);
    assert.equal(isSecretPath('certs/tls.crt'), true);
    assert.equal(isSecretPath('certs/tls.cer'), true);
    assert.equal(isSecretPath('.ssh/id_rsa'), true);
    assert.equal(isSecretPath('home/id_ed25519'), true);
    assert.equal(isSecretPath('gcp/credentials.json'), true);
    assert.equal(isSecretPath('config/secrets.yaml'), true);
    assert.equal(isSecretPath('.env.example'), false); // documentation, not a secret
    assert.equal(isSecretPath('apps/api/src/server.ts'), false);
    assert.equal(isSecretPath('docs/keystore-howto.md'), false); // not a real keystore
  });

  await t.test('the walker never READS a secret/ignored file (B10: ignore before read)', () => {
    const dir = makeFixture();
    try {
      const { index, readPaths } = buildIndex(dir, 'test');

      // Structural proof: the secret/ignored files were never opened.
      assert.ok(!readPaths.includes('.env'), '.env was read');
      assert.ok(!readPaths.includes('.env.production'), '.env.production was read');
      assert.ok(!readPaths.includes('ignored-secret.ts'), '.gitignored file was read');
      assert.ok(!readPaths.includes('server.key'), 'private key was read');
      assert.ok(!readPaths.some((p) => p.startsWith('node_modules/')), 'node_modules was read');

      // The legitimate source file WAS indexed.
      assert.ok(readPaths.includes('normal.ts'), 'normal.ts should have been read');
      assert.ok(index.symbols.some((s) => s.name === 'publicHandler' && s.kind === 'function'));
      assert.ok(index.symbols.some((s) => s.name === 'Widget' && s.kind === 'class'));

      // Output proof: no secret string survives anywhere in the serialized index.
      const blob = JSON.stringify(index);
      assert.ok(!blob.includes('sk_live_zzz_do_not_index_111'), '.env secret leaked into index');
      assert.ok(!blob.includes('gitignored_secret_abc_222'), 'gitignored secret leaked into index');
      assert.ok(!blob.includes('node_modules_secret_444'), 'node_modules secret leaked into index');
      assert.ok(!blob.includes('BEGIN PRIVATE KEY'), 'private key leaked into index');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  await t.test('.gitignore negation re-includes a previously-ignored path', () => {
    const dir = mkdtempSync(join(tmpdir(), 'ccc-negate-'));
    try {
      // Use a dir NOT in the hardcoded DIR_SKIP (build/dist/node_modules always win) so we
      // exercise the .gitignore ordered-negation path specifically.
      // The `!.env` line is a DELIBERATE attacker negation trying to re-include the secret; the
      // hard-deny layer must still win over it (this is the actual B10 adversarial case).
      writeFileSync(join(dir, '.gitignore'), 'generated/\n!generated/keep.ts\n!.env\n!server.key\n');
      mkdirSync(join(dir, 'generated'), { recursive: true });
      const ig = loadIgnore(dir);
      assert.equal(ig.isIgnored('generated/skip.ts'), true);
      assert.equal(ig.isIgnored('generated/keep.ts'), false); // negation wins for non-secrets
      // …but a hard secret is NEVER re-includable, even by an explicit `!`-negation in .gitignore.
      assert.equal(ig.isIgnored('.env'), true);
      assert.equal(ig.isIgnored('server.key'), true);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
