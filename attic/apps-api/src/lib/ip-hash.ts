import { createHash } from 'node:crypto';

export interface IpHashConfig {
  dailySalt: string;
}

export function hashIp(ip: string, config: IpHashConfig): string {
  return createHash('sha256').update(`${ip}:${config.dailySalt}`).digest('hex');
}

export function getDailySalt(): string {
  const date = new Date().toISOString().slice(0, 10);
  const key = `IP_HASH_SALT_${date.replace(/-/g, '_')}`;
  return process.env[key] || process.env.IP_HASH_SALT || 'fallback-dev-salt';
}
