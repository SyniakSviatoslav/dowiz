import { safeStorage } from '../../lib/safeStorage.js';
import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, useReducedMotion } from 'framer-motion';
import { Button, Input, FormField, LanguageSwitcher, useI18n, ease, duration } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { CourierLoginResponse } from '@deliveryos/shared-types';

export function LoginPage() {
  const { t } = useI18n();
  const reduceMotion = useReducedMotion();
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
        safeStorage.set('dos_access_token', data.jwt);
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
      <motion.div
        className="w-full max-w-sm"
        initial={reduceMotion ? { opacity: 0 } : { opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: duration.slow, ease: ease.out }}
      >
        <div className="absolute top-4 right-4">
          <LanguageSwitcher variant="full" />
        </div>
        <div className="mb-6 text-center">
          {/* Brand wordmark — a proper noun, not translated (mirrors the "Courier" wordmark in CourierRoutes). */}
          <span className="text-2xl font-bold tracking-tight" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-primary)' }}>
            DeliveryOS
          </span>
        </div>

        <div className="card-base p-8 space-y-6 block shadow-[var(--elev-1)]">
          <div className="text-center space-y-1.5">
            <h1 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>
              {t('courier.login', 'Courier Login')}
            </h1>
            <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>
              {t('courier.login_subtitle', 'Enter your email and password to continue')}
            </p>
          </div>

          {error && (
            <motion.div
              role="alert"
              aria-live="polite"
              className="p-3 text-sm text-center"
              style={{ background: 'var(--status-cancelled-light)', border: '1px solid var(--status-cancelled-border)', color: 'var(--color-danger)', borderRadius: 'var(--brand-radius-sm)' }}
              initial={{ opacity: 0 }}
              animate={reduceMotion ? { opacity: 1 } : { opacity: 1, x: [0, -6, 6, -4, 4, 0] }}
              transition={reduceMotion ? { duration: duration.fast } : { x: { duration: duration.slow, ease: ease.inOut }, opacity: { duration: duration.fast } }}
            >
              {error}
            </motion.div>
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
                placeholder={t('login.password_placeholder', 'Enter your password')}
                required
                error={!!error}
              />
            </FormField>

            <Button
              type="submit"
              className="w-full disabled:!bg-[var(--brand-surface-raised)] disabled:!text-[var(--brand-text-muted)] disabled:!opacity-100"
              size="lg"
              isLoading={loading}
              disabled={!email.trim() || !password}
            >
              {t('auth.login', 'Log In')}
            </Button>
          </form>
        </div>
      </motion.div>
    </div>
  );
}
