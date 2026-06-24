import { safeStorage } from '../../lib/safeStorage.js';
import React, { useState } from 'react';
import { Button, Input, useI18n, LanguageSwitcher, NomadicScene, NomadicCredit, isPaperSkinEnabled, paperSkinAttr } from '@deliveryos/ui';
import { apiClient } from '../../lib/apiClient.js';
import { z } from 'zod';
import { useNavigate } from 'react-router-dom';

// Google OAuth sign-in is hidden by default (temporarily disabled). Re-enable by building with
// VITE_GOOGLE_OAUTH_ENABLED=true. The backend /api/auth/google route stays live, so this is a
// one-flag flip with no server change. Email/password + Telegram login are unaffected.
const GOOGLE_OAUTH_ENABLED = import.meta.env.VITE_GOOGLE_OAUTH_ENABLED === 'true';

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
  const [tgWaiting, setTgWaiting] = useState(false);
  const [tgLink, setTgLink] = useState('');

  const sessionExpired = typeof window !== 'undefined' && sessionStorage.getItem('dos_auth_expired') === '1';
  React.useEffect(() => {
    if (sessionExpired) sessionStorage.removeItem('dos_auth_expired');
  }, []);

  const isDev = typeof window !== 'undefined' && (new URLSearchParams(window.location.search).get('dev') === 'true');

  const handleTelegramLogin = async () => {
    setError('');
    setTgWaiting(true);
    try {
      const res = await apiClient<any>('/auth/telegram/start', { method: 'POST' });
      setTgLink(res.deepLink);
      window.open(res.deepLink, '_blank');
      const token = res.token;
      const deadline = Date.now() + 5 * 60 * 1000;
      const poll = async () => {
        if (Date.now() > deadline) { setTgWaiting(false); setError(t('admin.tg_timeout', 'Telegram login timed out. Please try again.')); return; }
        try {
          const p = await apiClient<any>(`/auth/telegram/poll?token=${token}`);
          if (p.status === 'authenticated' && p.access_token) {
            sessionStorage.setItem('dos_access_token', p.access_token);
            safeStorage.set('dos_access_token', p.access_token);
            if (p.refresh_token) safeStorage.set('dos_refresh_token', p.refresh_token);
            navigate('/admin');
            return;
          }
        } catch (e: any) {
          if (e?.status === 410) { setTgWaiting(false); setError(t('admin.tg_expired', 'Telegram login link expired. Please try again.')); return; }
        }
        setTimeout(poll, 2000);
      };
      setTimeout(poll, 2000);
    } catch {
      setTgWaiting(false);
      setError(t('admin.tg_failed', 'Could not start Telegram login.'));
    }
  };

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
      safeStorage.set('dos_access_token', res.access_token);
      if ((res as any).refresh_token) safeStorage.set('dos_refresh_token', (res as any).refresh_token);
      navigate('/admin');
    } catch (err: any) {
      setError(err.status === 401
        ? t('admin.error_invalid_credentials', 'Invalid email or password.')
        : t('admin.error_login_failed', 'Login failed.'));
    } finally {
      setLoading(false);
    }
  };

  const handleDevLogin = async () => {
    try {
      const res = await apiClient<typeof DevMockAuthResponse>('/dev/mock-auth', { method: 'POST', schema: DevMockAuthResponse });
      sessionStorage.setItem('dos_access_token', res.access_token);
      safeStorage.set('dos_access_token', res.access_token);
      navigate('/admin');
    } catch (err: any) {
      setError(t('admin.error_login_failed', 'Login failed.'));
    }
  };

  return (
    <div {...paperSkinAttr()} className="min-h-screen flex items-center justify-center p-4 text-[var(--brand-text)]" style={{ background: 'var(--brand-bg)' }}>
      <div className="w-full max-w-sm">
        {/* Brand accent bar (or the Nomadic oasis hero under the paper skin) */}
        {isPaperSkinEnabled() ? (
          <div className="mb-5 overflow-hidden rounded-[24px]" style={{ background: 'color-mix(in srgb, var(--teal) 16%, var(--paper-surface))', border: '1.5px solid var(--ink-line)' }}>
            <NomadicScene variant="oasis" animated />
            <div className="px-5 pb-5 -mt-2 text-center">
              <p className="uppercase tracking-[0.25em] text-[11px] font-semibold" style={{ color: 'var(--teal-deep)' }}>{t('admin.login_kicker', 'Welcome back')}</p>
              <h1 className="text-4xl leading-[1.05] mt-1" style={{ fontFamily: 'var(--font-display)', fontWeight: 600, letterSpacing: '-0.02em' }}>DeliveryOS</h1>
            </div>
          </div>
        ) : (
          <div className="h-1 w-16 mx-auto mb-6 rounded-full" style={{ background: 'linear-gradient(90deg, var(--brand-primary), var(--brand-primary-hover))' }} />
        )}
        
        <div className="card-base p-8 space-y-6">
          <div className="text-center space-y-1.5">
            {!isPaperSkinEnabled() && <h1 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>DeliveryOS</h1>}
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
            {GOOGLE_OAUTH_ENABLED && (
              <a
                href="/api/auth/google"
                className="flex items-center justify-center gap-3 w-full px-4 py-2.5 rounded-lg transition-[background-color,transform,opacity] duration-200 active:scale-[0.98]"
                style={{ border: '1px solid var(--brand-border)', background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}
              >
                <i className="ti ti-brand-google" />
                <span className="text-sm font-medium">{t('admin.sign_in_google', 'Sign in with Google')}</span>
              </a>
            )}

            <button
              type="button"
              onClick={handleTelegramLogin}
              disabled={tgWaiting}
              className="flex items-center justify-center gap-3 w-full px-4 py-2.5 rounded-lg transition-[background-color,transform,opacity] duration-200 active:scale-[0.98] disabled:opacity-60"
              style={{ background: 'var(--brand-telegram, #229ED9)', color: '#fff' }}
            >
              <i className="ti ti-brand-telegram" />
              <span className="text-sm font-medium">
                {tgWaiting ? t('admin.tg_waiting', 'Waiting for Telegram…') : t('admin.sign_in_telegram', 'Continue with Telegram')}
              </span>
            </button>
            {tgWaiting && tgLink && (
              <p className="text-xs text-center" style={{ color: 'var(--brand-text-muted)' }}>
                {t('admin.tg_open_hint', "Didn't open?")}{' '}
                <a href={tgLink} target="_blank" rel="noreferrer" style={{ color: 'var(--brand-primary)' }}>{t('admin.tg_open_link', 'Open Telegram')}</a>
              </p>
            )}

            {isDev && (
              <Button variant="secondary" className="w-full" onClick={handleDevLogin}>
                {t('admin.dev_login', '[DEV] One-Click Login')}
              </Button>
            )}
          </div>
        </div>
        {isPaperSkinEnabled() && <NomadicCredit className="mt-6" />}
      </div>
    </div>
  );
}
