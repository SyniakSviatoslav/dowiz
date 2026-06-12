import React, { useState } from 'react';
import { Button, Input, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/apiClient.js';
import { useNavigate } from 'react-router-dom';

export function LoginPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const isDev = typeof window !== 'undefined' && (new URLSearchParams(window.location.search).get('dev') === 'true');

  const handleLocalLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const res = await apiClient<any>('/auth/local/login', {
        method: 'POST',
        body: { email, password }
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
      const res = await apiClient<any>('/dev/mock-auth', { method: 'POST' });
      sessionStorage.setItem('dos_access_token', res.access_token);
      localStorage.setItem('dos_access_token', res.access_token);
      navigate('/admin');
    } catch (err: any) {
      setError('Dev login failed.');
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-[var(--brand-bg)] p-4">
      <div className="w-full max-w-md p-8 bg-[var(--brand-surface)] rounded-xl shadow-elevation-2">
        <div className="text-center mb-8">
          <h1 className="text-2xl font-bold mb-2" style={{ fontFamily: 'var(--brand-font-heading)' }}>DeliveryOS</h1>
          <p className="text-[var(--brand-text-muted)]">{t('admin.sign_in_owner', 'Sign in to your owner account')}</p>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-[var(--status-cancelled-light)] border border-[var(--status-cancelled-border)] text-[var(--color-danger)] rounded-md text-sm text-center">
            {error}
          </div>
        )}

        <form onSubmit={handleLocalLogin} className="space-y-4">
          <div>
            <label htmlFor="login-email" className="block text-sm font-medium mb-1 text-[var(--brand-text-muted)]">{t('admin.email', 'Email')}</label>
            <Input 
              id="login-email"
              type="email" 
              value={email} 
              onChange={(e) => setEmail(e.target.value)} 
              required 
              placeholder="owner@restaurant.com"
            />
          </div>
          <div>
            <label htmlFor="login-password" className="block text-sm font-medium mb-1 text-[var(--brand-text-muted)]">{t('admin.password', 'Password')}</label>
            <Input 
              id="login-password"
              type="password" 
              value={password} 
              onChange={(e) => setPassword(e.target.value)} 
              required 
              placeholder="••••••••"
            />
          </div>
          
          <Button type="submit" variant="primary" className="w-full" disabled={loading}>
            {loading ? t('common.loading') : t('admin.sign_in', 'Sign In')}
          </Button>
        </form>

        <div className="mt-6 flex items-center justify-center gap-4">
          <div className="h-px bg-[var(--brand-border)] flex-1" />
          <span className="text-xs text-[var(--brand-text-muted)] uppercase tracking-wider">{t('common.or', 'OR')}</span>
          <div className="h-px bg-[var(--brand-border)] flex-1" />
        </div>

        <div className="mt-6 space-y-3">
          <a 
            href="/api/auth/google" 
            className="flex items-center justify-center gap-3 w-full px-4 py-2 border border-[var(--brand-border)] rounded-[var(--brand-radius)] hover:bg-[var(--brand-surface-raised)] transition-colors"
          >
            <i className="ti ti-brand-google" />
            {t('admin.sign_in_google', 'Sign in with Google')}
          </a>

          {isDev && (
            <Button variant="secondary" className="w-full" onClick={handleDevLogin}>
              {t('admin.dev_login', '[DEV] One-Click Login')}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
