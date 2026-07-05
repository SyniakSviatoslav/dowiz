import type { Pool } from 'pg';
import { type S3Client, GetObjectCommand, GetBucketLifecycleConfigurationCommand } from '@aws-sdk/client-s3';
import { createHash } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { getS3Client } from './upload.js';
import type { R2Config } from './upload.js';
import { createDecryptionStream, resolveBackupKey } from './encrypt.js';

const SAMPLE_DAYS_BACK = 7;
const MIN_SAMPLE_COUNT = 3;

interface R2Manifest {
  backupId: string;
  type: string;
  checksumSha256: string;
  r2Key: string;
  encryption: { iv: string; authTag: string; algorithm: string; keyId: string };
  rowCounts: Record<string, number>;
}

interface R2VerifyResult {
  passed: boolean;
  manifestsChecked: number;
  manifestsValid: number;
  lifecycleDrift: boolean;
  errors: string[];
}

const EXPECTED_LIFECYCLE_RULES: Array<{ prefix: string; days: number }> = [
  { prefix: 'dowiz-backups/production/hourly/', days: 1 },
  { prefix: 'dowiz-backups/production/daily/', days: 30 },
  { prefix: 'dowiz-backups/production/weekly/', days: 90 },
  { prefix: 'dowiz-backups/production/monthly/', days: 2555 },
];

function getR2Config(): R2Config {
  return {
    accountId: '',
    endpoint: process.env.R2_ENDPOINT || '',
    accessKeyId: process.env.R2_ACCESS_KEY_ID || '',
    secretAccessKey: process.env.R2_SECRET_ACCESS_KEY || '',
    bucket: process.env.R2_BUCKET || '',
  };
}

async function downloadManifest(s3: S3Client, config: R2Config, key: string): Promise<R2Manifest> {
  const command = new GetObjectCommand({ Bucket: config.bucket, Key: key });
  const response = await s3.send(command);
  const body = await response.Body?.transformToString();
  if (!body) throw new Error(`Empty manifest at ${key}`);
  return JSON.parse(body);
}

// LC7 fix 1 — the writer stores sha256 of the PLAINTEXT dump, so the artifact must be DECRYPTED
// before hashing. Download + decrypt ONCE; the checksum and the pg_restore --list schema check
// both run on the resulting plaintext dump. (The old verifyManifestChecksum hashed the CIPHERTEXT
// → always mismatched → `continue` skipped the schema check entirely.)
async function downloadAndDecrypt(
  s3: S3Client,
  config: R2Config,
  manifest: R2Manifest,
  encryptedPath: string,
  decryptedPath: string,
): Promise<void> {
  const response = await s3.send(new GetObjectCommand({ Bucket: config.bucket, Key: manifest.r2Key }));
  const writeStream = createWriteStream(encryptedPath);
  await new Promise<void>((resolve, reject) => {
    (response.Body as unknown as NodeJS.ReadableStream).pipe(writeStream);
    writeStream.on('finish', resolve);
    writeStream.on('error', reject);
  });

  // LC7 fix 7 — resolve the key via the keyring from the manifest's keyId (fail loud on unknown).
  const key = resolveBackupKey(manifest.encryption.keyId);
  const decipher = createDecryptionStream(key, manifest.encryption.iv, manifest.encryption.authTag);
  const readStream = createReadStream(encryptedPath);
  const decWriteStream = createWriteStream(decryptedPath);
  await new Promise<void>((resolve, reject) => {
    readStream.pipe(decipher).pipe(decWriteStream);
    decWriteStream.on('finish', resolve);
    decWriteStream.on('error', reject);
    decipher.on('error', reject);
  });
}

async function sha256File(filePath: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const hash = createHash('sha256');
    const stream = createReadStream(filePath);
    stream.on('error', reject);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
  });
}

async function pgRestoreList(decryptedPath: string): Promise<boolean> {
  return new Promise((resolve, reject) => {
    const child = spawn('pg_restore', ['--list', decryptedPath], { stdio: ['ignore', 'pipe', 'pipe'] });
    let output = '';
    child.stdout?.on('data', (d: Buffer) => { output += d.toString(); });
    child.on('close', (code) => resolve(code === 0 && output.length > 0));
    child.on('error', reject);
  });
}

async function checkLifecyclePolicy(s3: S3Client, config: R2Config): Promise<boolean> {
  try {
    const command = new GetBucketLifecycleConfigurationCommand({ Bucket: config.bucket });
    const response = await s3.send(command);
    const rules = response.Rules || [];

    for (const expected of EXPECTED_LIFECYCLE_RULES) {
      const match = rules.find(
        (r: any) =>
          r.Filter?.Prefix === expected.prefix &&
          r.Status === 'Enabled' &&
          r.Expiration?.Days === expected.days,
      );
      if (!match) return false;
    }
    return true;
  } catch (err: any) {
    console.warn('[r2-verify] lifecycle policy check failed:', err?.message);
    return false;
  }
}

export async function runR2Verify(pool: Pool): Promise<R2VerifyResult> {
  const errors: string[] = [];
  const config = getR2Config();

  if (!config.endpoint || !config.bucket) {
    return { passed: false, manifestsChecked: 0, manifestsValid: 0, lifecycleDrift: false, errors: ['R2 not configured'] };
  }

  const s3 = getS3Client(config);

  // ── 1. Lifecycle policy ──
  let lifecycleDrift = false;
  try {
    lifecycleDrift = !(await checkLifecyclePolicy(s3, config));
  } catch (err: any) {
    lifecycleDrift = true;
    errors.push(`Lifecycle check failed: ${err.message}`);
  }

  // ── 2. Sample manifests (3 random in last 7 days) ──
  const manifestRes = await pool.query(
    `SELECT r2_key FROM backup_metadata
     WHERE status = 'completed' AND created_at >= now() - interval '7 days'
     ORDER BY random() LIMIT $1`,
    [MIN_SAMPLE_COUNT],
  );

  let manifestsChecked = 0;
  let manifestsValid = 0;

  for (const row of manifestRes.rows) {
    manifestsChecked++;
    const tempDir = path.join(process.cwd(), '.tmp', 'r2-verify');
    let encryptedPath = '';
    let decryptedPath = '';
    try {
      await fs.mkdir(tempDir, { recursive: true });
      const manifestKey = row.r2_key.replace('.enc.parts', '.manifest.json');
      const manifest = await downloadManifest(s3, config, manifestKey);
      encryptedPath = path.join(tempDir, `${manifest.backupId}.enc`);
      decryptedPath = path.join(tempDir, `${manifest.backupId}.dump`);

      await downloadAndDecrypt(s3, config, manifest, encryptedPath, decryptedPath);

      // Checksum: manifest stores sha256 of the PLAINTEXT dump → hash the DECRYPTED file.
      const actualHash = await sha256File(decryptedPath);
      if (actualHash !== manifest.checksumSha256) {
        errors.push(`Manifest ${manifestKey}: checksum mismatch (plaintext hash != manifest.checksumSha256)`);
        continue;
      }

      // Schema: pg_restore --list must enumerate the archive TOC.
      const schemaOk = await pgRestoreList(decryptedPath);
      if (!schemaOk) {
        errors.push(`Manifest ${manifestKey}: pg_restore --list failed or empty`);
        continue;
      }

      manifestsValid++;
    } catch (err: any) {
      errors.push(`Manifest ${row.r2_key}: ${err.message}`);
    } finally {
      if (encryptedPath) await fs.unlink(encryptedPath).catch(() => {});
      if (decryptedPath) await fs.unlink(decryptedPath).catch(() => {});
    }
  }

  return {
    passed: manifestsValid === manifestsChecked && !lifecycleDrift,
    manifestsChecked,
    manifestsValid,
    lifecycleDrift,
    errors,
  };
}
