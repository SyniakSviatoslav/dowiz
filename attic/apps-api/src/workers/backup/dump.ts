// @ts-nocheck
import { spawn } from 'node:child_process';
import { Readable } from 'node:stream';
import path from 'node:path';
import fs from 'node:fs/promises';
import { createReadStream, createWriteStream } from 'node:fs';
import { registerChildProcess } from '../../shutdown.js';

export interface DumpResult {
  stream: Readable;
  cleanup: () => Promise<void>;
  tempFile: string;
}

export async function createLogicalDump(databaseUrl: string, backupId: string): Promise<DumpResult> {
  const tempDir = path.join(process.cwd(), '.tmp', 'backups');
  await fs.mkdir(tempDir, { recursive: true });
  
  const tempFile = path.join(tempDir, `backup-${backupId}.dump`);
  const logFile = path.join(tempDir, `backup-${backupId}.log`);

  return new Promise((resolve, reject) => {
    const args = [
      databaseUrl,
      '--format=custom',
      '--compress=9',
      '--no-owner',
      '--no-acl',
      '--quote-all-identifiers',
      `--file=${tempFile}`,
      '--verbose'
    ];

    const child = spawn('pg_dump', args, {
      env: { ...process.env, PGCONNECT_TIMEOUT: '10' },
      stdio: ['ignore', 'inherit', 'pipe']
    });

    // Register for SIGTERM forwarding
    registerChildProcess(child);

    const logStream = createWriteStream(logFile);
    child.stderr.pipe(logStream);

    child.on('close', (code) => {
      if (code === 0) {
        // Resolve with a readable stream from the temp file
        const stream = createReadStream(tempFile);
        
        const cleanup = async () => {
          stream.destroy();
          await fs.unlink(tempFile).catch(() => {});
          await fs.unlink(logFile).catch(() => {});
        };

        resolve({ stream, cleanup, tempFile });
      } else {
        reject(new Error(`pg_dump failed with exit code ${code}. Check logs at ${logFile}`));
      }
    });

    child.on('error', (err) => {
      reject(new Error(`Failed to start pg_dump: ${err.message}`));
    });
  });
}
