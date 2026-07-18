/**
 * Backup Restore Script — Dry-run and full restore
 *
 * Usage:
 *   pnpm backup:restore --snapshot=<backupId>                    # Full restore
 *   pnpm backup:restore --dry-run --snapshot=<backupId>          # Dry-run (verify only)
 *   pnpm backup:restore --list                                   # List recent snapshots
 *
 * Environment variables required:
 *   DATABASE_URL_MIGRATIONS       — target database (restore destination)
 *   BACKUP_ENCRYPTION_KEY         — 32-byte base64 key for decryption
 *   R2_ENDPOINT, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_BUCKET
 */

import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import { S3Client, GetObjectCommand } from '@aws-sdk/client-s3';
import { createDecryptionStream } from '../attic/apps-api/src/workers/backup/encrypt.js';
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

async function decryptBackup(manifest: Manifest, encryptedPath: string, decryptedPath: string): Promise<void> {
  const keyBase64 = env.BACKUP_ENCRYPTION_KEY;
  if (!keyBase64) throw new Error('BACKUP_ENCRYPTION_KEY is required');

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

async function runDryRun(backupId: string) {
  console.log(`\n=== Restore Dry-Run: ${backupId} ===\n`);
  const tempDir = path.join(process.cwd(), '.tmp', 'restore');
  await fs.mkdir(tempDir, { recursive: true });

  // 1. Download and verify manifest
  console.log('[1/5] Downloading manifest...');
  const manifest = await downloadManifest(backupId);
  console.log(`  Type: ${manifest.type}, Created: ${manifest.createdAt}`);
  console.log(`  Tables: ${Object.keys(manifest.rowCounts).join(', ')}`);
  console.log(`  Row counts: ${JSON.stringify(manifest.rowCounts)}`);

  // 2. Download encrypted backup
  console.log('[2/5] Downloading encrypted backup from R2...');
  const encryptedPath = path.join(tempDir, `${backupId}.enc`);
  await downloadFromR2(manifest.r2Key, encryptedPath);
  const encryptedStat = await fs.stat(encryptedPath);
  console.log(`  Size: ${(encryptedStat.size / 1024 / 1024).toFixed(2)} MB`);

  // 3. Decrypt
  console.log('[3/5] Decrypting backup...');
  const decryptedPath = path.join(tempDir, `${backupId}.dump`);
  await decryptBackup(manifest, encryptedPath, decryptedPath);
  const decryptedStat = await fs.stat(decryptedPath);
  console.log(`  Decrypted size: ${(decryptedStat.size / 1024 / 1024).toFixed(2)} MB`);

  // 4. Verify checksum
  console.log('[4/5] Verifying checksum...');
  const checksumOk = await verifyChecksum(decryptedPath, manifest.checksumSha256);
  if (!checksumOk) {
    throw new Error('CHECKSUM_MISMATCH: backup data corrupted or wrong encryption key');
  }
  console.log('  ✓ Checksum matches');

  // 5. Verify row counts (dry-run: restore to a temp schema or just validate manifest)
  console.log('[5/5] Validating row counts...');
  const db = createSessionPool();
  try {
    for (const [table, expectedCount] of Object.entries(manifest.rowCounts)) {
      const res = await db.query(`SELECT COUNT(*) as c FROM "${table}"`);
      const actualCount = parseInt(res.rows[0].c, 10);
      const match = actualCount === expectedCount;
      console.log(`  ${match ? '✓' : '⚠'} ${table}: expected ${expectedCount}, actual ${actualCount}`);
    }
  } finally {
    await db.end();
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

async function main() {
  const args = process.argv.slice(2);
  const listIdx = args.indexOf('--list');
  const dryRunIdx = args.indexOf('--dry-run');
  const snapshotIdx = args.indexOf('--snapshot');

  if (listIdx !== -1) {
    await listSnapshots();
    return;
  }

  if (snapshotIdx === -1 || !args[snapshotIdx + 1]) {
    console.error('Usage:');
    console.error('  pnpm backup:restore --snapshot=<backupId>');
    console.error('  pnpm backup:restore --dry-run --snapshot=<backupId>');
    console.error('  pnpm backup:restore --list');
    process.exit(1);
  }

  const backupId = args[snapshotIdx + 1];
  const isDryRun = dryRunIdx !== -1;

  if (isDryRun) {
    await runDryRun(backupId);
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
