// @ts-nocheck
import { Client } from 'pg';

const DB_NAME_REGEX = /^[a-z0-9_]+$/;
const SANDBOX_PREFIX = 'dowiz_restore_sandbox_';

function validateDbName(name: string): void {
  if (!DB_NAME_REGEX.test(name)) {
    throw new Error(`Invalid DB name: ${name} — must match ^[a-z0-9_]+$`);
  }
}

function getAdminUrl(): string {
  const url = process.env.DATABASE_URL_ADMIN;
  if (!url) throw new Error('DATABASE_URL_ADMIN is required for sandbox management');
  return url;
}

function extractDbName(sandboxUrl: string): string {
  const dbName = sandboxUrl.split('/').pop();
  if (!dbName) throw new Error('Cannot parse DB name from sandbox URL');
  return dbName;
}

function buildSandboxUrl(adminUrl: string, dbName: string): string {
  return adminUrl.replace(/\/[^/]+$/, `/${dbName}`);
}

export async function createSandboxDatabase(): Promise<string> {
  const adminUrl = getAdminUrl();
  const timestamp = Date.now();
  const dbName = `${SANDBOX_PREFIX}${timestamp}`;

  validateDbName(dbName);

  const client = new Client({ connectionString: adminUrl });
  try {
    await client.connect();
    await client.query(`CREATE DATABASE "${dbName}"`);
    const sandboxUrl = buildSandboxUrl(adminUrl, dbName);
    return sandboxUrl;
  } finally {
    await client.end();
  }
}

export async function dropSandboxDatabase(sandboxUrl: string): Promise<void> {
  const adminUrl = getAdminUrl();
  const dbName = extractDbName(sandboxUrl);

  validateDbName(dbName);

  const client = new Client({ connectionString: adminUrl });
  try {
    await client.connect();
    await client.query(
      `SELECT pg_terminate_backend(pid)
       FROM pg_stat_activity
       WHERE datname = $1 AND pid <> pg_backend_pid()`,
      [dbName],
    );
    await client.query(`DROP DATABASE IF EXISTS "${dbName}"`);
  } finally {
    await client.end();
  }
}

export async function listSandboxDatabases(): Promise<string[]> {
  const adminUrl = getAdminUrl();
  const client = new Client({ connectionString: adminUrl });
  try {
    await client.connect();
    const res = await client.query(
      `SELECT datname FROM pg_database WHERE datname LIKE '${SANDBOX_PREFIX}%'`,
    );
    return res.rows.map((r: any) => r.datname);
  } finally {
    await client.end();
  }
}

export function isSandboxDb(dbName: string): boolean {
  return dbName.startsWith(SANDBOX_PREFIX);
}
