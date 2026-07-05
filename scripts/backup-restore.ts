/**
 * Backup Restore Script — Dry-run and full restore
 *
 * Usage:
 *   pnpm backup:restore --snapshot=<backupId>                    # Full restore
 *   pnpm backup:restore --dry-run --snapshot=<backupId>          # Dry-run (verify only, via DB)
 *   pnpm backup:restore --list                                   # List recent snapshots (via DB)
 *   pnpm backup:restore --r2-key=<r2Key>                         # DISASTER path: locate + verify
 *                                                                 # the artifact straight from R2,
 *                                                                 # with NO DB round-trip
 *   pnpm backup:restore --list-r2                                # List snapshots straight from R2
 *
 * The --r2-key / --list-r2 paths exist because in a real disaster the DB (backup_metadata) is the
 * thing you've lost — you cannot ask it where the artifact is. The R2 manifest JSON self-describes
 * everything needed (iv/authTag/keyId/checksum/rowCounts), so these paths never touch the DB.
 *
 * Environment variables required:
 *   DATABASE_URL_MIGRATIONS       — target database (restore destination)
 *   BACKUP_ENCRYPTION_KEY / BACKUP_KEYRING — key(s) for decryption (resolved per manifest keyId)
 *   R2_ENDPOINT, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_BUCKET
 */

import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import { S3Client, GetObjectCommand, ListObjectsV2Command } from '@aws-sdk/client-s3';
import { createDecryptionStream, resolveBackupKey } from '../apps/api/src/workers/backup/encrypt.js';
import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import { createWriteStream, createReadStream } from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { createHash } from 'node:crypto';

const env = loadEnv();

interface Manifest {
  backupId: string;
  type: string;
  createdAt: string;
  checksumSha256: string;
  r2Key: string;
  encryption: { algorithm: string; iv: string; authTag: string; keyId: string };
  rowCounts: Record<string, number>;
}

function getS3Client() {
  return new S3Client({
    region: 'auto',
    endpoint: env.R2_ENDPOINT,
    credentials: {
      accessKeyId: env.R2_ACCESS_KEY_ID || '',
      secretAccessKey: env.R2_SECRET_ACCESS_KEY || '',
    },
  });
}

async function downloadFromR2(key: string, destPath: string): Promise<void> {
  const client = getS3Client();
  const command = new GetObjectCommand({
    Bucket: env.R2_BUCKET,
    Key: key,
  });
  const response = await client.send(command);
  const writeStream = createWriteStream(destPath);
  await new Promise<void>((resolve, reject) => {
    (response.Body as any).pipe(writeStream);
    writeStream.on('finish', resolve);
    writeStream.on('error', reject);
  });
}

async function downloadManifest(backupId: string): Promise<Manifest> {
  const db = createSessionPool();
  try {
    const res = await db.query(
      `SELECT r2_key FROM backup_metadata WHERE id = $1 AND status = 'completed'`,
      [backupId]
    );
    if (res.rowCount === 0) {
      throw new Error(`Backup ${backupId} not found or not completed`);
    }
    const r2Key = res.rows[0].r2_key;
    const manifestKey = r2Key.replace('.enc.parts', '.manifest.json');

    const tempDir = path.join(process.cwd(), '.tmp', 'restore');
    await fs.mkdir(tempDir, { recursive: true });
    const manifestPath = path.join(tempDir, `${backupId}.manifest.json`);
    await downloadFromR2(manifestKey, manifestPath);
    const content = await fs.readFile(manifestPath, 'utf-8');
    return JSON.parse(content);
  } finally {
    await db.end();
  }
}

async function downloadManifestByR2Key(r2Key: string): Promise<Manifest> {
  // Disaster-recovery path: the DB may be GONE. Locate the manifest purely from R2. Accept either
  // the artifact key (ending .enc.parts) or the manifest key (ending .manifest.json).
  const manifestKey = r2Key.endsWith('.manifest.json')
    ? r2Key
    : r2Key.replace('.enc.parts', '.manifest.json');

  const tempDir = path.join(process.cwd(), '.tmp', 'restore');
  await fs.mkdir(tempDir, { recursive: true });
  const manifestPath = path.join(tempDir, `${path.basename(manifestKey)}`);
  await downloadFromR2(manifestKey, manifestPath);
  const content = await fs.readFile(manifestPath, 'utf-8');
  return JSON.parse(content);
}

async function listSnapshotsR2(): Promise<void> {
  // Enumerate manifests straight from R2 — no DB. Single page is plenty for an operator triage.
  const client = getS3Client();
  const prefix = `dowiz-backups/${env.NODE_ENV}/`;
  const res = await client.send(
    new ListObjectsV2Command({ Bucket: env.R2_BUCKET, Prefix: prefix, MaxKeys: 1000 }),
  );
  const manifests = (res.Contents || [])
    .map((o) => o.Key || '')
    .filter((k) => k.endsWith('.manifest.json'))
    .sort();

  console.log(`\n=== R2 Backup Manifests (prefix ${prefix}) ===\n`);
  if (manifests.length === 0) {
    console.log('  (none found)');
  } else {
    for (const key of manifests) console.log(`  ${key}`);
  }
  if (res.IsTruncated) console.log('\n  … (list truncated at 1000 keys)');
  console.log(`\n${manifests.length} manifest(s) found. Verify one with: pnpm backup:restore --r2-key=<r2Key>`);
}

async function decryptBackup(manifest: Manifest, encryptedPath: string, decryptedPath: string): Promise<void> {
  // LC7 fix 7 — resolve the key from the manifest's keyId via the keyring (fail loud on unknown).
  const keyBase64 = resolveBackupKey(manifest.encryption.keyId);

  const decipherStream = createDecryptionStream(keyBase64, manifest.encryption.iv, manifest.encryption.authTag);
  const readStream = createReadStream(encryptedPath);
  const writeStream = createWriteStream(decryptedPath);

  await new Promise<void>((resolve, reject) => {
    readStream.pipe(decipherStream).pipe(writeStream);
    writeStream.on('finish', resolve);
    writeStream.on('error', reject);
    decipherStream.on('error', reject);
  });
}

async function verifyChecksum(filePath: string, expectedSha256: string): Promise<boolean> {
  return new Promise((resolve, reject) => {
    const hash = createHash('sha256');
    const stream = createReadStream(filePath);
    stream.on('error', reject);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex') === expectedSha256));
  });
}

async function runDryRun(manifest: Manifest, opts: { compareLiveDb: boolean }) {
  console.log(`\n=== Restore Dry-Run: ${manifest.backupId} ===\n`);
  const tempDir = path.join(process.cwd(), '.tmp', 'restore');
  await fs.mkdir(tempDir, { recursive: true });

  // 1. Manifest already resolved (via DB --snapshot, or straight from R2 --r2-key)
  console.log('[1/5] Manifest loaded');
  console.log(`  Type: ${manifest.type}, Created: ${manifest.createdAt}`);
  console.log(`  Key ID: ${manifest.encryption.keyId}`);
  console.log(`  Tables in manifest: ${Object.keys(manifest.rowCounts).length}`);

  // 2. Download encrypted backup
  console.log('[2/5] Downloading encrypted backup from R2...');
  const encryptedPath = path.join(tempDir, `${manifest.backupId}.enc`);
  await downloadFromR2(manifest.r2Key, encryptedPath);
  const encryptedStat = await fs.stat(encryptedPath);
  console.log(`  Size: ${(encryptedStat.size / 1024 / 1024).toFixed(2)} MB`);

  // 3. Decrypt
  console.log('[3/5] Decrypting backup...');
  const decryptedPath = path.join(tempDir, `${manifest.backupId}.dump`);
  await decryptBackup(manifest, encryptedPath, decryptedPath);
  const decryptedStat = await fs.stat(decryptedPath);
  console.log(`  Decrypted size: ${(decryptedStat.size / 1024 / 1024).toFixed(2)} MB`);

  // 4. Verify checksum — hashes the DECRYPTED/plaintext dump (matches the writer's plaintext hash)
  console.log('[4/5] Verifying checksum...');
  const checksumOk = await verifyChecksum(decryptedPath, manifest.checksumSha256);
  if (!checksumOk) {
    throw new Error('CHECKSUM_MISMATCH: backup data corrupted or wrong encryption key');
  }
  console.log('  ✓ Checksum matches');

  // 5. Row counts — only meaningful against a live DB; skipped on the R2 disaster path.
  if (opts.compareLiveDb) {
    console.log('[5/5] Comparing manifest row counts against the live DB...');
    const db = createSessionPool();
    try {
      for (const [table, expectedCount] of Object.entries(manifest.rowCounts)) {
        const res = await db.query(`SELECT COUNT(*) as c FROM "${table}"`);
        const actualCount = parseInt(res.rows[0].c, 10);
        const match = actualCount === expectedCount;
        console.log(`  ${match ? '✓' : '⚠'} ${table}: manifest ${expectedCount}, live ${actualCount}`);
      }
    } finally {
      await db.end();
    }
  } else {
    console.log('[5/5] Skipping live-DB row-count comparison (--r2-key disaster path: no DB to compare against).');
  }

  // Cleanup
  await fs.unlink(encryptedPath).catch(() => {});
  await fs.unlink(decryptedPath).catch(() => {});
  console.log('\n✓ Dry-run completed successfully. Backup is valid and decryptable.');
}

async function listSnapshots() {
  const db = createSessionPool();
  try {
    const res = await db.query(`
      SELECT id, type, created_at, status, size_bytes, checksum_sha256 IS NOT NULL as has_checksum
      FROM backup_metadata
      WHERE status = 'completed'
      ORDER BY created_at DESC
      LIMIT 20
    `);
    console.log('\n=== Recent Backup Snapshots ===\n');
    for (const row of res.rows) {
      console.log(`  ${row.id}  ${row.type.padEnd(8)}  ${row.created_at}  ${row.status}`);
    }
  } finally {
    await db.end();
  }
}

function getFlagValue(args: string[], flag: string): string | undefined {
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  if (eq) return eq.slice(flag.length + 1);
  const idx = args.indexOf(flag);
  const next = idx !== -1 ? args[idx + 1] : undefined;
  if (next && !next.startsWith('--')) return next;
  return undefined;
}

async function main() {
  const args = process.argv.slice(2);
  const dryRunIdx = args.indexOf('--dry-run');

  // ── DISASTER paths: locate / verify straight from R2 with NO DB round-trip ──
  if (args.includes('--list-r2')) {
    await listSnapshotsR2();
    return;
  }
  const r2KeyArg = getFlagValue(args, '--r2-key');
  if (r2KeyArg) {
    const manifest = await downloadManifestByR2Key(r2KeyArg);
    await runDryRun(manifest, { compareLiveDb: false });
    return;
  }

  // ── DB paths ──
  if (args.includes('--list')) {
    await listSnapshots();
    return;
  }

  const backupId = getFlagValue(args, '--snapshot');
  if (!backupId) {
    console.error('Usage:');
    console.error('  pnpm backup:restore --snapshot=<backupId>');
    console.error('  pnpm backup:restore --dry-run --snapshot=<backupId>');
    console.error('  pnpm backup:restore --list');
    console.error('  pnpm backup:restore --r2-key=<r2Key>   (disaster path, no DB)');
    console.error('  pnpm backup:restore --list-r2          (disaster path, no DB)');
    process.exit(1);
  }

  const isDryRun = dryRunIdx !== -1;
  const manifest = await downloadManifest(backupId);
  if (isDryRun) {
    await runDryRun(manifest, { compareLiveDb: true });
  } else {
    console.log('Full restore not yet implemented — use --dry-run for verification.');
    console.log('For production restore, use pg_restore manually:');
    console.log(`  pg_restore -d <DATABASE_URL> --clean --if-exists <decrypted_dump>`);
    process.exit(1);
  }
}

main().catch(err => {
  console.error('\n❌ Restore failed:', err.message);
  process.exit(1);
});
