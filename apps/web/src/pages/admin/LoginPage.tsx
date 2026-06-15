import React, { useState } from 'react';
import { Button, Input, useI18n, LanguageSwitcher } from '@deliveryos/ui';
import { apiClient } from '../../lib/apiClient.js';
import { z } from 'zod';
import { useNavigate } from 'react-router-dom';

const AuthLoginResponse = z.object({
  access_token: z.string(),
}).passthrough();

const DevMockAuthResponse = z.object({
  access_token: z.string(),
}).passthrough();

export function LoginPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const sessionExpired = typeof window !== 'undefined' && sessionStorage.getItem('dos_auth_expired') === '1';
  React.useEffect(() => {
    if (sessionExpired) sessionStorage.removeItem('dos_auth_expired');
  }, []);

  const isDev = typeof window !== 'undefined' && (new URLSearchParams(window.location.search).get('dev') === 'true');

  const handleLocalLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const res = await apiClient<typeof AuthLoginResponse>('/auth/local/login', {
        method: 'POST',
        body: { email, password },
        schema: AuthLoginResponse,
      });
      sessionStorage.setItem('dos_access_token', res.access_token);
      localStorage.setItem('dos_access_token', res.access_token);
      navigate('/admin');
    } catch (err: any) {
      setError(err.status === 401 ? 'Invalid email or password.' : 'Login failed.');
    } finally {
      setLoading(false);
    }
  };

  const handleDevLogin = async () => {
    try {
      const res = await apiClient<typeof DevMockAuthResponse>('/dev/mock-auth', { method: 'POST', schema: DevMockAuthResponse });
      sessionStorage.setItem('dos_access_token', res.access_token);
      localStorage.setItem('dos_access_token', res.access_token);
      navigate('/admin');
    } catch (err: any) {
      setError('Dev login failed.');
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4" style={{ background: 'var(--brand-bg)' }}>
      <div className="w-full max-w-sm">
        {/* Brand accent bar */}
        <div className="h-1 w-16 mx-auto mb-6 rounded-full" style={{ background: 'linear-gradient(90deg, var(--brand-primary), var(--brand-primary-hover))' }} />
        
        <div className="card-base p-8 space-y-6">
          <div className="text-center space-y-1.5">
            <h1 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>DeliveryOS</h1>
            <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.sign_in_owner', 'Sign in to your owner account')}</p>
          </div>

          {sessionExpired && (
            <div role="alert" className="p-3 text-sm text-center rounded-lg" style={{ background: 'rgba(234,179,8,0.10)', border: '1px solid rgba(234,179,8,0.30)', color: 'var(--brand-text)' }}>
              <i className="ti ti-clock-exclamation mr-1.5" />
              {t('admin.session_expired', 'Your session has expired. Please sign in again.')}
            </div>
          )}

          {error && (
            <div role="alert" aria-live="polite" className="p-3 text-sm text-center rounded-lg" style={{ background: 'var(--status-cancelled-light)', border: '1px solid var(--status-cancelled-border)', color: 'var(--color-danger)' }}>
              {error}
            </div>
          )}

          <form onSubmit={handleLocalLogin} className="space-y-4">
            <div className="space-y-1.5">
              <label htmlFor="login-email" className="text-sm font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.email', 'Email')}</label>
              <Input id="login-email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} required placeholder="owner@restaurant.com" />
            </div>
            <div className="space-y-1.5">
              <label htmlFor="login-password" className="text-sm font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.password', 'Password')}</label>
              <Input id="login-password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} required placeholder="••••••••" />
            </div>
            
            <Button type="submit" variant="primary" className="w-full" disabled={loading}>
              {loading ? t('common.loading', 'Loading...') : t('admin.sign_in', 'Sign In')}
            </Button>
          </form>

          <div className="flex justify-center">
            <LanguageSwitcher variant="full" />
          </div>

          <div className="flex items-center gap-4">
            <div className="flex-1 h-px" style={{ background: 'var(--brand-border)' }} />
            <span className="text-xs uppercase tracking-wider" style={{ color: 'var(--brand-text-muted)' }}>{t('common.or', 'OR')}</span>
            <div className="flex-1 h-px" style={{ background: 'var(--brand-border)' }} />
          </div>

          <div className="space-y-3">
            <a 
              href="/api/auth/google" 
              className="flex items-center justify-center gap-3 w-full px-4 py-2.5 rounded-lg transition-all duration-200 active:scale-[0.98]"
              style={{ border: '1px solid var(--brand-border)', background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}
            >
              <i className="ti ti-brand-google" />
              <span className="text-sm font-medium">{t('admin.sign_in_google', 'Sign in with Google')}</span>
            </a>

            {isDev && (
              <Button variant="secondary" className="w-full" onClick={handleDevLogin}>
                {t('admin.dev_login', '[DEV] One-Click Login')}
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
