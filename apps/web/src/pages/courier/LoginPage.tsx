import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Input, FormField, LanguageSwitcher, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { CourierLoginResponse } from '@deliveryos/shared-types';

export function LoginPage() {
  const { t } = useI18n();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const navigate = useNavigate();

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const data = await apiClient<typeof CourierLoginResponse>('/courier/auth/login', {
        method: 'POST',
        body: { email, password },
        schema: CourierLoginResponse,
      });
      if (data?.jwt) {
        localStorage.setItem('dos_access_token', data.jwt);
        navigate('/courier');
      } else {
        setError(t('common.invalid_response', 'Invalid response from server'));
      }
    } catch (err: any) {
      setError(err.status === 401 ? t('auth.invalid_credentials', 'Invalid email or password') : t('auth.login_failed', 'Login failed. Please try again.'));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4" style={{ background: 'var(--brand-bg)' }}>
      <div className="w-full max-w-sm">
        <div className="absolute top-4 right-4">
          <LanguageSwitcher variant="full" />
        </div>
        <div className="h-1 w-16 mx-auto mb-6 rounded-full" style={{ background: 'linear-gradient(90deg, var(--brand-primary), var(--brand-primary-hover))' }} />

        <div className="card-base p-8 space-y-6">
          <div className="text-center space-y-1.5">
            <h1 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>
              {t('courier.login', 'Courier Login')}
            </h1>
            <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>
              {t('courier.login_subtitle', 'Enter your email and password to continue')}
            </p>
          </div>

          {error && (
            <div role="alert" aria-live="polite" className="p-3 text-sm text-center rounded-lg" style={{ background: 'var(--status-cancelled-light)', border: '1px solid var(--status-cancelled-border)', color: 'var(--color-danger)' }}>
              {error}
            </div>
          )}

          <form onSubmit={handleLogin} className="space-y-4">
            <FormField label={t('login.email', 'Email')}>
              <Input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="courier@example.com"
                required
                error={!!error}
              />
            </FormField>

            <FormField label={t('login.password', 'Password')}>
              <Input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Enter your password"
                required
                error={!!error}
              />
            </FormField>

            <Button
              type="submit"
              className="w-full"
              size="lg"
              isLoading={loading}
              disabled={!email.trim() || !password}
            >
              {t('auth.login', 'Log In')}
            </Button>
          </form>
        </div>
      </div>
    </div>
  );
}
