import { useState, useEffect } from 'react';
import type { BrandPreset } from '../theme/index.js';

export interface AuthState {
  token: string | null;
  refreshToken: string | null;
  user: AuthUser | null;
  role: UserRole | null;
  tenantSlug: string | null;
  isAuthenticated: boolean;
}

export interface AuthUser {
  id: string;
  phone: string;
  name?: string;
  role: UserRole;
  tenantSlug: string;
  preset?: BrandPreset;
}

export type UserRole = 'client' | 'owner' | 'courier' | 'admin';

interface JwtPayload {
  sub: string;
  role: UserRole;
  tenant_slug: string;
  exp: number;
  iat: number;
  preset?: BrandPreset;
}

type Listener = (state: AuthState) => void;

function decodeJwt(token: string): JwtPayload | null {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const p = parts[1];
    if (!p) return null;
    const payload = JSON.parse(atob(p));
    return payload as JwtPayload;
  } catch {
    return null;
  }
}

function isExpired(payload: JwtPayload): boolean {
    const now = Math.floor(Date.now() / 1000);
    return payload.exp < now;
}

class AuthService {
  private listeners: Set<Listener> = new Set();
  private state: AuthState = {
    token: null,
    refreshToken: null,
    user: null,
    role: null,
    tenantSlug: null,
    isAuthenticated: false,
  };

  constructor() {
    this.restore();
  }

  private setState(partial: Partial<AuthState>): void {
    this.state = { ...this.state, ...partial };
    this.listeners.forEach((fn) => fn(this.state));
  }

  subscribe(fn: Listener): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  getState(): AuthState {
    return { ...this.state };
  }

  async login(phone: string, otp: string): Promise<AuthState> {
    const res = await fetch('/api/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ phone, otp }),
    });
    if (!res.ok) throw new Error('Login failed');
    const data = await res.json();
    this.handleTokens(data.token, data.refreshToken);
    return this.getState();
  }

  async logout(): Promise<void> {
    const token = this.state.token;
    this.clearTokens();
    try {
      await fetch('/api/auth/logout', {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}` },
      });
    } catch {
      // Ignore logout errors
    }
  }

  async refresh(): Promise<boolean> {
    const refreshToken = this.state.refreshToken;
    if (!refreshToken) return false;
    try {
      const res = await fetch('/api/auth/refresh', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refreshToken }),
      });
      if (!res.ok) return false;
      const data = await res.json();
      this.handleTokens(data.token, data.refreshToken || refreshToken);
      return true;
    } catch {
      return false;
    }
  }

  async restore(): Promise<void> {
    const token = sessionStorage.getItem('dos_token');
    const refreshToken = localStorage.getItem('dos_refresh_token');
    if (!token) return;
    const payload = decodeJwt(token);
    if (!payload || isExpired(payload)) {
      if (refreshToken) {
        const ok = await this.refresh();
        if (!ok) this.clearTokens();
      } else {
        this.clearTokens();
      }
      return;
    }
    this.handleTokens(token, refreshToken);
  }

  private handleTokens(token: string, refreshToken: string | null): void {
    const payload = decodeJwt(token);
    if (!payload) return;
    sessionStorage.setItem('dos_token', token);
    if (refreshToken) localStorage.setItem('dos_refresh_token', refreshToken);
    this.setState({
      token,
      refreshToken,
      user: {
        id: payload.sub,
        phone: payload.sub,
        role: payload.role,
        tenantSlug: payload.tenant_slug,
        preset: payload.preset,
      },
      role: payload.role,
      tenantSlug: payload.tenant_slug,
      isAuthenticated: true,
    });
  }

  private clearTokens(): void {
    sessionStorage.removeItem('dos_token');
    localStorage.removeItem('dos_refresh_token');
    this.setState({
      token: null,
      refreshToken: null,
      user: null,
      role: null,
      tenantSlug: null,
      isAuthenticated: false,
    });
  }

  getToken(): string | null {
    return this.state.token;
  }
}

export const authService = new AuthService();

export function useAuth(): AuthState & { login: AuthService['login']; logout: AuthService['logout'] } {
  const [state, setState] = useState<AuthState>(() => authService.getState());

  useEffect(() => {
    const unsub = authService.subscribe(setState);
    return unsub;
  }, []);

  return { ...state, login: authService.login.bind(authService), logout: authService.logout.bind(authService) };
}
