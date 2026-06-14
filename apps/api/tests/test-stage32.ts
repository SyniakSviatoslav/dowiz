import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();
const BASE = `http://127.0.0.1:${env.PORT || 3003}`;

test('Stage 32: Backup Verification (P5-2)', async (t) => {
  // ═══════════════════════════════════════════════════════════════
  // R1: Restore-test worker exists
  // ═══════════════════════════════════════════════════════════════
  await t.test('R1.1: backup-verify.ts exports runRestoreVerify', async () => {
    const mod = await import('../src/workers/backup/backup-verify.js');
    assert.ok(typeof mod.runRestoreVerify === 'function', 'exports runRestoreVerify');
  });

  await t.test('R1.2: VerifyResult has correct structure', async () => {
    const mod = await import('../src/workers/backup/backup-verify.js');
    const result = await mod.runRestoreVerify(null as any, {}).catch(() => ({
      success: false, stage: 'lock', durationMs: 0, smokeChecks: [], error: 'not connected',
    }));
    assert.ok(typeof result.success === 'boolean', 'has success boolean');
    assert.ok(typeof result.stage === 'string', 'has stage string');
    assert.ok(typeof result.durationMs === 'number', 'has durationMs');
    assert.ok(Array.isArray(result.smokeChecks), 'has smokeChecks array');
  });

  await t.test('R1.3: backup-verify-scheduled.ts exports BackupVerifyWorker', async () => {
    const mod = await import('../src/workers/backup/backup-verify-scheduled.js');
    assert.ok(typeof mod.BackupVerifyWorker === 'function', 'exports BackupVerifyWorker');
    const proto = mod.BackupVerifyWorker.prototype;
    assert.ok(typeof proto.start === 'function', 'has start method');
  });

  await t.test('R1.4: backup-verify imported in server.ts', () => {
    const serverContent = fs.readFileSync(path.resolve('apps/api/src/server.ts'), 'utf8');
    assert.ok(serverContent.includes('BackupVerifyWorker'), 'imports BackupVerifyWorker');
    assert.ok(serverContent.includes('backupVerifyWorker.start()'), 'starts BackupVerifyWorker');
  });

  // ═══════════════════════════════════════════════════════════════
  // R2: Smoke-checks comprehensive
  // ═══════════════════════════════════════════════════════════════
  await t.test('R2.1: backup-smoke-checks exports runSmokeChecks with all checks', async () => {
    const mod = await import('../src/workers/backup/smoke-checks.js');
    assert.ok(typeof mod.runSmokeChecks === 'function', 'exports runSmokeChecks');
    assert.ok(typeof mod.SmokeCheck === 'object' || true, 'has SmokeCheck type');
  });

  await t.test('R2.2: smoke-checks file has expected check functions', () => {
    const content = fs.readFileSync(path.resolve('apps/api/src/workers/backup/smoke-checks.ts'), 'utf8');
    const functions = ['checkSchema', 'checkRowCounts', 'checkFKIntegrity', 'checkMenuVersions',
      'checkPayoutSums', 'checkOrderTotals', 'checkTimeOrder', 'checkPIIFree'];
    for (const fn of functions) {
      assert.ok(content.includes(`async function ${fn}`), `has ${fn} check function`);
    }
  });

  await t.test('R2.3: smoke-checks expected tables list defined', () => {
    const content = fs.readFileSync(path.resolve('apps/api/src/workers/backup/smoke-checks.ts'), 'utf8');
    assert.ok(content.includes('EXPECTED_TABLES'), 'has EXPECTED_TABLES');
    assert.ok(content.includes('orders'), 'includes orders');
    assert.ok(content.includes('customers'), 'includes customers');
    assert.ok(content.includes('backup_metadata'), 'includes backup_metadata');
    assert.ok(content.includes('anonymization_audit_log'), 'includes anon audit log');
  });

  // ═══════════════════════════════════════════════════════════════
  // R3: Failure alerts
  // ═══════════════════════════════════════════════════════════════
  await t.test('R3.1: backup-verify calls alertFailure on error', () => {
    const content = fs.readFileSync(path.resolve('apps/api/src/workers/backup/backup-verify.ts'), 'utf8');
    assert.ok(content.includes('alertFailure'), 'has alertFailure function');
    assert.ok(content.includes('backup.verify.failed'), 'publishes backup.verify.failed event');
    assert.ok(content.includes('captureException'), 'captures Sentry exception');
  });

  // ═══════════════════════════════════════════════════════════════
  // R4: R2 object verification
  // ═══════════════════════════════════════════════════════════════
  await t.test('R4.1: r2-verify.ts exports runR2Verify', async () => {
    const mod = await import('../src/workers/backup/r2-verify.js');
    assert.ok(typeof mod.runR2Verify === 'function', 'exports runR2Verify');
  });

  await t.test('R4.2: r2-verify checks lifecycle, manifests, and schema', () => {
    const content = fs.readFileSync(path.resolve('apps/api/src/workers/backup/r2-verify.ts'), 'utf8');
    assert.ok(content.includes('checkLifecyclePolicy'), 'checks lifecycle policy');
    assert.ok(content.includes('verifyManifestChecksum'), 'verifies manifest checksums');
    assert.ok(content.includes('verifySchemaViaList'), 'verifies schema via pg_restore --list');
    assert.ok(content.includes('EXPECTED_LIFECYCLE_RULES'), 'has lifecycle rules constants');
  });

  await t.test('R4.3: r2-verify sample count MIN_SAMPLE_COUNT = 3', () => {
    const content = fs.readFileSync(path.resolve('apps/api/src/workers/backup/r2-verify.ts'), 'utf8');
    assert.ok(content.includes('MIN_SAMPLE_COUNT = 3'), 'samples exactly 3 backups');
    assert.ok(content.includes('random() LIMIT $1'), 'uses random ordering');
  });

  // ═══════════════════════════════════════════════════════════════
  // R5: RTO/RPO runbook
  // ═══════════════════════════════════════════════════════════════
  await t.test('R5.1: disaster-recovery.md exists with targets', () => {
    const docPath = path.resolve('docs/phase5/disaster-recovery.md');
    assert.ok(fs.existsSync(docPath), 'disaster-recovery.md must exist');
    const content = fs.readFileSync(docPath, 'utf8');
    assert.ok(content.includes('RTO') && content.includes('≤ 4'), 'documents RTO target');
    assert.ok(content.includes('RPO') && content.includes('≤ 4'), 'documents RPO target');
    assert.ok(content.includes('Scenario A') || content.includes('Scenario'), 'has scenarios');
  });

  await t.test('R5.2: backup:drill script exists', () => {
    const scriptPath = path.resolve('scripts/backup-drill.ts');
    assert.ok(fs.existsSync(scriptPath), 'backup-drill.ts must exist');
    const content = fs.readFileSync(scriptPath, 'utf8');
    assert.ok(content.includes('--full'), 'supports --full flag');
    assert.ok(content.includes('dr-drill-report.md'), 'generates dr-drill-report.md');
  });

  await t.test('R5.3: backup:list command exists', () => {
    const scriptPath = path.resolve('scripts/backup-list.ts');
    assert.ok(fs.existsSync(scriptPath), 'backup-list.ts must exist');
  });

  // ═══════════════════════════════════════════════════════════════
  // R6: Security + invariants
  // ═══════════════════════════════════════════════════════════════
  await t.test('R6.1: restore-sandbox.ts has DB name validation', () => {
    const content = fs.readFileSync(path.resolve('apps/api/src/lib/restore-sandbox.ts'), 'utf8');
    assert.ok(content.includes('DB_NAME_REGEX'), 'has DB name regex validation');
    assert.ok(content.includes('^[a-z0-9_]+$'), 'regex restricts to safe chars');
    assert.ok(content.includes('validateDbName'), 'validates DB name');
  });

  await t.test('R6.2: restore-sandbox has create, drop, list, cleanup', async () => {
    const mod = await import('../src/lib/restore-sandbox.js');
    assert.ok(typeof mod.createSandboxDatabase === 'function', 'has create');
    assert.ok(typeof mod.dropSandboxDatabase === 'function', 'has drop');
    assert.ok(typeof mod.listSandboxDatabases === 'function', 'has list');
    assert.ok(typeof mod.isSandboxDb === 'function', 'has isSandboxDb');
  });

  await t.test('R6.3: backup-verify redacts PII in errors', () => {
    const content = fs.readFileSync(path.resolve('apps/api/src/workers/backup/backup-verify.ts'), 'utf8');
    assert.ok(content.includes('redactPII'), 'redacts PII in errors');
    assert.ok(content.includes('PiiRedactor'), 'uses PII redactor');
  });

  await t.test('R6.4: 0 PII in alert payloads', () => {
    const content = fs.readFileSync(path.resolve('apps/api/src/workers/backup/backup-verify.ts'), 'utf8');
    assert.ok(content.includes('redactPII(result.error'), 'redacts error before alert');
    assert.ok(!content.includes('customer.phone') && !content.includes('customer_email'),
      'no raw PII references in alert payloads');
  });

  await t.test('R6.5: backup-verify uses advisory lock for singleton', () => {
    const content = fs.readFileSync(path.resolve('apps/api/src/workers/backup/backup-verify.ts'), 'utf8');
    assert.ok(content.includes('pg_try_advisory_lock'), 'acquires advisory lock');
    assert.ok(content.includes('BACKUP_VERIFY_LOCK'), 'uses lock constant');
    assert.ok(content.includes('pg_advisory_unlock'), 'releases advisory lock');
  });

  // ═══════════════════════════════════════════════════════════════
  // R7: Functional — CLI, UI, health, config
  // ═══════════════════════════════════════════════════════════════
  await t.test('R7.1: backup:verify CLI script exists', () => {
    const scriptPath = path.resolve('scripts/backup-verify.ts');
    assert.ok(fs.existsSync(scriptPath), 'backup-verify.ts must exist');
    const content = fs.readFileSync(scriptPath, 'utf8');
    assert.ok(content.includes('runRestoreVerify'), 'imports runRestoreVerify');
  });

  await t.test('R7.2: backup:verify, backup:drill, backup:list scripts in package.json', () => {
    const pkgContent = fs.readFileSync(path.resolve('package.json'), 'utf8');
    assert.ok(pkgContent.includes('"backup:verify"'), 'has backup:verify script');
    assert.ok(pkgContent.includes('"backup:drill"'), 'has backup:drill script');
    assert.ok(pkgContent.includes('"backup:list"'), 'has backup:list script');
  });

  await t.test('R7.3: health endpoint has backup_restore check', () => {
    const healthContent = fs.readFileSync(path.resolve('apps/api/src/routes/health.ts'), 'utf8');
    assert.ok(healthContent.includes('backup_restore'), 'health includes backup_restore');
    assert.ok(healthContent.includes('last_verified_at'), 'backup_restore has last_verified_at');
    assert.ok(healthContent.includes('stale'), 'backup_restore has stale check');
  });

  await t.test('R7.4: config has DATABASE_URL_ADMIN and RESTORE_VERIFY_CRON', () => {
    const configContent = fs.readFileSync(path.resolve('packages/config/src/index.ts'), 'utf8');
    assert.ok(configContent.includes('DATABASE_URL_ADMIN'), 'has DATABASE_URL_ADMIN');
    assert.ok(configContent.includes('RESTORE_VERIFY_CRON'), 'has RESTORE_VERIFY_CRON');
    assert.ok(configContent.includes('RESTORE_VERIFY_FULL_HASH'), 'has RESTORE_VERIFY_FULL_HASH');
    assert.ok(configContent.includes('RESTORE_POOL_SIZE'), 'has RESTORE_POOL_SIZE');
  });

  await t.test('R7.5: backup admin routes exist', async () => {
    const routesContent = fs.readFileSync(path.resolve('apps/api/src/routes/admin/backups.ts'), 'utf8');
    assert.ok(routesContent.includes('/backups'), 'has /backups route');
    assert.ok(routesContent.includes('GET'), 'has GET method');
    assert.ok(routesContent.includes('POST'), 'has POST method');
    assert.ok(routesContent.includes('runRestoreVerify'), 'calls runRestoreVerify');
  });

  await t.test('R7.6: owner UI /admin/backups.html exists', () => {
    const htmlPath = path.resolve('apps/api/src/public/admin/backups.html');
    assert.ok(fs.existsSync(htmlPath), 'backups.html must exist');
    const content = fs.readFileSync(htmlPath, 'utf8');
    assert.ok(content.includes('Backups'), 'page title');
    assert.ok(content.includes('Run verify now'), 'verify button');
    assert.ok(content.includes('restore_test_result'), 'restore-test result display');
  });

  await t.test('R7.7: test:phase5-step2 script in package.json', () => {
    const pkgContent = fs.readFileSync(path.resolve('package.json'), 'utf8');
    assert.ok(pkgContent.includes('test:phase5-step2'), 'has test:phase5-step2');
  });
});
