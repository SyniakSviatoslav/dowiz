import fs from 'fs/promises';
import path from 'path';
import type { StorageProvider } from '../ports.js';

export class LocalFsStorageProvider implements StorageProvider {
  private baseDir: string;

  constructor(baseDir: string = 'tmp/imports') {
    this.baseDir = path.resolve(process.cwd(), baseDir);
  }

  async put(key: string, data: Buffer, ttlSeconds?: number): Promise<void> {
    const filePath = path.join(this.baseDir, key);
    await fs.mkdir(path.dirname(filePath), { recursive: true });
    await fs.writeFile(filePath, data);
    // Note: ttlSeconds is ignored in local fs as cleanup is done via cron/pg-boss elsewhere
  }

  async get(key: string): Promise<Buffer | null> {
    const filePath = path.join(this.baseDir, key);
    try {
      return await fs.readFile(filePath);
    } catch (e: any) {
      if (e.code === 'ENOENT') return null;
      throw e;
    }
  }

  async delete(key: string): Promise<void> {
    const filePath = path.join(this.baseDir, key);
    try {
      await fs.unlink(filePath);
    } catch (e: any) {
      if (e.code !== 'ENOENT') throw e;
    }
  }
}
