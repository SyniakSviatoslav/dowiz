import { test, describe, before, after } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { Readable, Transform } from 'node:stream';
import { createEncryptionStream, createDecryptionStream } from '../src/workers/backup/encrypt.js';
import { loadEnv } from '@deliveryos/config';
import { createOperationalPool, createSessionPool } from '@deliveryos/db';
import { BackupCronWorker } from '../src/workers/backup/index.js';
import * as dumpModule from '../src/workers/backup/dump.js';
import * as uploadModule from '../src/workers/backup/upload.js';
import fs from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import path from 'node:path';

// Mock test data storage
const mockS3Store = new Map<string, Buffer | string>();

// Ensure we have an encryption key for tests
const testEncryptionKey = crypto.randomBytes(32).toString('base64');
process.env.BACKUP_ENCRYPTION_KEY = testEncryptionKey;
process.env.BACKUP_ENABLED = 'true';

test('Stage 20: R2 Database Backup', async (t) => {
  const env = loadEnv();

  let pool: any;
  let backupPool: any;

  await t.test('setup db state', async () => {
    pool = createOperationalPool();
    backupPool = createSessionPool();
  });

  await t.test('encryption wrapper correctness', async () => {
    const rawData = 'hello world, this is a secret database dump containing PII like +35512345678 and test@gmail.com';
    const { stream: cipherStream, meta, getAuthTag } = createEncryptionStream(testEncryptionKey);

    const buffers: Buffer[] = [];
    cipherStream.on('data', (c) => buffers.push(c));

    cipherStream.write(Buffer.from(rawData));
    cipherStream.end();

    await new Promise(resolve => cipherStream.on('end', resolve));
    const encryptedPayload = Buffer.concat(buffers);
    meta.authTag = getAuthTag();

    // Verify it is encrypted (not containing plaintext)
    assert.strictEqual(encryptedPayload.toString().includes('+35512345678'), false);
    assert.strictEqual(encryptedPayload.toString().includes('hello world'), false);

    // Decrypt
    const decipherStream = createDecryptionStream(testEncryptionKey, meta.iv, meta.authTag);
    const outBuffers: Buffer[] = [];
    decipherStream.on('data', c => outBuffers.push(c));

    decipherStream.write(encryptedPayload);
    decipherStream.end();

    await new Promise(resolve => decipherStream.on('end', resolve));
    const decryptedPayload = Buffer.concat(outBuffers);

    assert.strictEqual(decryptedPayload.toString(), rawData);
  });

  await t.test('BackupWorker completes end-to-end (mocked dump & S3)', async () => {
    const mockDumpFile = path.join(process.cwd(), '.tmp', 'mock-dump.sql');
    await fs.mkdir(path.dirname(mockDumpFile), { recursive: true });
    await fs.writeFile(mockDumpFile, "MOCK PG_DUMP CONTENT WITH fake PII: +355691234567");

    const mockCreateLogicalDump = async (dbUrl: string, id: string) => {
      const stream = createReadStream(mockDumpFile);
      return {
        stream,
        tempFile: mockDumpFile,
        cleanup: async () => {} // Don't delete so we can reuse
      };
    };

    const mockUploadStream = async (config: any, key: string, stream: NodeJS.ReadableStream) => {
      const bufs: Buffer[] = [];
      stream.on('data', d => bufs.push(Buffer.from(d)));
      await new Promise(resolve => stream.on('end', resolve));
      mockS3Store.set(key, Buffer.concat(bufs));
    };

    const mockUploadJson = async (config: any, key: string, data: any) => {
      mockS3Store.set(key, JSON.stringify(data));
    };

    const mockBoss = { work: async () => {}, schedule: async () => {} } as any;
    const mockMessageBus = { publish: async () => {} } as any;

    const worker = new BackupCronWorker(
      pool, 
      backupPool, 
      mockBoss, 
      mockMessageBus,
      {
        createLogicalDump: mockCreateLogicalDump as any,
        uploadStream: mockUploadStream as any,
        uploadJson: mockUploadJson as any
      }
    );
    await worker.handleBackup('daily');

    // Assertions
    const client = await pool.connect();
    try {
      const metaRes = await client.query(`SELECT * FROM backup_metadata WHERE type = 'daily' ORDER BY created_at DESC LIMIT 1`);
      assert.strictEqual(metaRes.rowCount, 1);
      const metadata = metaRes.rows[0];
      assert.strictEqual(metadata.status, 'completed');
      assert.ok(metadata.r2_key.includes('/daily/'));
      
      const auditRes = await client.query(`SELECT * FROM backup_audit_log WHERE backup_id = $1 ORDER BY created_at ASC`, [metadata.id]);
      assert.strictEqual(auditRes.rowCount, 2); // started, completed
      assert.strictEqual(auditRes.rows[0].action, 'started');
      assert.strictEqual(auditRes.rows[1].action, 'completed');

      // Check mock S3
      assert.ok(mockS3Store.has(metadata.r2_key));
      const manifestKey = metadata.r2_key.replace('.enc.parts', '.manifest.json');
      assert.ok(mockS3Store.has(manifestKey));

      const manifestText = mockS3Store.get(manifestKey) as string;
      const manifest = JSON.parse(manifestText);
      assert.strictEqual(manifest.piiRedacted, false);
      assert.strictEqual(manifest.piiEncrypted, true);
      assert.ok(manifest.rowCounts.orders !== undefined);
      assert.ok(manifest.checksumSha256);
    } finally {
      client.release();
    }
  });

  await t.test('cleanup', async () => {
    const client = await pool.connect();
    try {
      await client.query(`DELETE FROM backup_audit_log`);
      await client.query(`DELETE FROM backup_metadata`);
    } finally {
      client.release();
    }
    await pool.end();
    await backupPool.end();
  });
});
