import { BASE_URL } from '../config.js';
import { authHeaders } from './auth.js';

export interface QueueState {
  name: string;
  state: string;
  count: number;
}

export interface AuditEntry {
  event: string;
  status: string;
  channel: string;
  locationId: string;
  targetId: string | null;
  errorMessage: string | null;
  createdAt: string;
}

export async function observeHealth(): Promise<any> {
  const res = await fetch(`${BASE_URL}/health`);
  if (!res.ok) return { status: 'error', error: `HTTP ${res.status}` };
  return res.json();
}

export async function observeQueues(): Promise<QueueState[]> {
  const { status, body } = await fetch(`${BASE_URL}/api/admin/queues`, {
    headers: await authHeaders(),
  }).then(r => r.json().catch(() => null)).catch(() => ({ status: 0, body: null }));
  return body?.queues || [];
}

export async function observeAudit(locationId?: string, limit: number = 20): Promise<AuditEntry[]> {
  // We can query notification_outbox_audit via the health endpoint if available
  // For now, use a public endpoint or direct DB query
  const { status, body } = await fetch(`${BASE_URL}/api/owner/audit?limit=${limit}${locationId ? `&locationId=${locationId}` : ''}`, {
    headers: await authHeaders(),
  }).then(r => r.json().catch(() => ({ status: r.status, body: null }))).catch(() => ({ status: 0, body: null }));
  return body?.audit || [];
}

export async function observeLocationInfo(slug: string): Promise<any> {
  const res = await fetch(`${BASE_URL}/public/locations/${slug}/info`);
  if (!res.ok) return null;
  return res.json();
}

export interface TelemetryCheck {
  label: string;
  ok: boolean;
  detail?: string;
}

export async function probeEndpoints(endpoints: { path: string; method: string; expectStatus: number; auth?: boolean }[]): Promise<TelemetryCheck[]> {
  const results: TelemetryCheck[] = [];
  for (const ep of endpoints) {
    try {
      const opts: RequestInit = { method: ep.method };
      if (ep.auth) opts.headers = await authHeaders();
      const res = await fetch(`${BASE_URL}${ep.path}`, opts);
      results.push({
        label: `${ep.method} ${ep.path}`,
        ok: res.status === ep.expectStatus,
        detail: `Expected ${ep.expectStatus}, got ${res.status}`,
      });
    } catch (err: any) {
      results.push({
        label: `${ep.method} ${ep.path}`,
        ok: false,
        detail: `Error: ${err.message}`,
      });
    }
  }
  return results;
}
