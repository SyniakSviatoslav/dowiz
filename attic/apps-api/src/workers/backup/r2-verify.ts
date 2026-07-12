// @ts-nocheck
import { Pool } from 'pg';
import { S3Client, GetObjectCommand, HeadBucketCommand, GetBucketLifecycleConfigurationCommand } from '@aws-sdk/client-s3';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import fs from 'node:fs/promises';
import path from 'node:path';
import { getS3Client } from './upload.js';
import type { R2Config } from './upload.js';
import { createDecryptionStream } from './encrypt.js';
import { spawn } from 'node:child_process';

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

async function verifyManifestChecksum(
  s3: S3Client,
  config: R2Config,
  manifest: R2Manifest,
): Promise<boolean> {
  const command = new GetObjectCommand({ Bucket: config.bucket, Key: manifest.r2Key });
  const response = await s3.send(command);
  const chunks: Buffer[] = [];
  for await (const chunk of response.Body as any) {
    chunks.push(Buffer.from(chunk));
  }
  const actualHash = createHash('sha256').update(Buffer.concat(chunks)).digest('hex');
  return actualHash === manifest.checksumSha256;
}

async function verifySchemaViaList(s3: S3Client, config: R2Config, manifest: R2Manifest): Promise<boolean> {
  const tempDir = path.join(process.cwd(), '.tmp', 'r2-verify');
  await fs.mkdir(tempDir, { recursive: true });
  const encryptedPath = path.join(tempDir, `${manifest.backupId}.enc`);
  const decryptedPath = path.join(tempDir, `${manifest.backupId}.dump`);

  try {
    const getCmd = new GetObjectCommand({ Bucket: config.bucket, Key: manifest.r2Key });
    const response = await s3.send(getCmd);
    const writeStream = (await import('node:fs')).createWriteStream(encryptedPath);
    await new Promise<void>((resolve, reject) => {
      (response.Body as any).pipe(writeStream);
      writeStream.on('finish', resolve);
      writeStream.on('error', reject);
    });

    const decipher = createDecryptionStream(
      process.env.BACKUP_ENCRYPTION_KEY || '',
      manifest.encryption.iv,
      manifest.encryption.authTag,
    );
    const readStream = createReadStream(encryptedPath);
    const decWriteStream = (await import('node:fs')).createWriteStream(decryptedPath);
    await new Promise<void>((resolve, reject) => {
      readStream.pipe(decipher).pipe(decWriteStream);
      decWriteStream.on('finish', resolve);
      decWriteStream.on('error', reject);
      decipher.on('error', reject);
    });

    return new Promise((resolve, reject) => {
      const child = spawn('pg_restore', ['--list', decryptedPath], { stdio: ['ignore', 'pipe', 'pipe'] });
      let output = '';
      child.stdout.on('data', (d: Buffer) => { output += d.toString(); });
      child.on('close', (code) => {
        resolve(code === 0 && output.length > 0);
      });
      child.on('error', reject);
    });
  } finally {
    await fs.unlink(encryptedPath).catch(() => {});
    await fs.unlink(decryptedPath).catch(() => {});
  }
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
    try {
      const manifestKey = row.r2_key.replace('.enc.parts', '.manifest.json');
      const manifest = await downloadManifest(s3, config, manifestKey);

      const checksumOk = await verifyManifestChecksum(s3, config, manifest);
      if (!checksumOk) {
        errors.push(`Manifest ${manifestKey}: checksum mismatch`);
        continue;
      }

      const schemaOk = await verifySchemaViaList(s3, config, manifest);
      if (!schemaOk) {
        errors.push(`Manifest ${manifestKey}: pg_restore --list failed or empty`);
        continue;
      }

      manifestsValid++;
    } catch (err: any) {
      errors.push(`Manifest ${row.r2_key}: ${err.message}`);
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
