import { BASE_URL } from '../config.js';

export interface AuthSession {
  token: string;
  refreshToken: string;
  userId: string;
  role: string;
}

let session: AuthSession | null = null;

export function getSession(): AuthSession | null {
  return session;
}

export async function loginLocal(email: string, password: string): Promise<AuthSession> {
  const res = await fetch(`${BASE_URL}/api/auth/local/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Login failed (${res.status}): ${body}`);
  }
  const data = await res.json();
  // Decode JWT payload to get userId + role
  const parts = data.access_token.split('.');
  const padding = 4 - (parts[1].length % 4);
  const padded = padding < 4 ? parts[1] + '='.repeat(padding) : parts[1];
  const payload = JSON.parse(atob(padded));
  session = {
    token: data.access_token,
    refreshToken: data.refresh_token,
    userId: payload.userId || payload.sub,
    role: payload.role,
  };
  return session;
}

export async function loginMockOwner(): Promise<AuthSession> {
  const res = await fetch(`${BASE_URL}/api/dev/mock-auth`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
  });
  if (!res.ok) throw new Error(`Mock auth failed (${res.status})`);
  const data = await res.json();
  const parts = data.access_token.split('.');
  const padding = 4 - (parts[1].length % 4);
  const padded = padding < 4 ? parts[1] + '='.repeat(padding) : parts[1];
  const payload = JSON.parse(atob(padded));
  session = {
    token: data.access_token,
    refreshToken: data.refresh_token || '',
    userId: payload.userId || payload.sub,
    role: payload.role,
  };
  return session;
}

export async function authHeaders(): Promise<Record<string, string>> {
  if (!session) throw new Error('Not authenticated');
  return {
    Authorization: `Bearer ${session.token}`,
    'Content-Type': 'application/json',
  };
}

export async function apiGet(path: string): Promise<any> {
  const res = await fetch(`${BASE_URL}${path}`, { headers: await authHeaders() });
  const body = res.status === 204 ? null : await res.json();
  return { status: res.status, body };
}

export async function apiPost(path: string, data?: any): Promise<{ status: number; body: any }> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'POST',
    headers: await authHeaders(),
    body: data ? JSON.stringify(data) : undefined,
  });
  const body = res.status === 204 ? null : await res.json().catch(() => null);
  return { status: res.status, body };
}

export async function apiPatch(path: string, data: any): Promise<{ status: number; body: any }> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'PATCH',
    headers: await authHeaders(),
    body: JSON.stringify(data),
  });
  const body = res.status === 204 ? null : await res.json().catch(() => null);
  return { status: res.status, body };
}
