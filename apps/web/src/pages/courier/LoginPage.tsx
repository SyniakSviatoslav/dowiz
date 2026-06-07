import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Input, FormField, EmptyState } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

const isDev = typeof window !== 'undefined' && sessionStorage.getItem('dos_dev') === '1';

export function LoginPage() {
  const [phone, setPhone] = useState('');
  const [password, setPassword] = useState('');
  const [inviteCode, setInviteCode] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [mode, setMode] = useState<'login' | 'invite'>('login');
  const navigate = useNavigate();

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const data = await apiClient<any>('/courier/auth/login', {
        method: 'POST',
        body: { phone, password, location_id: undefined }
      });
      if (data?.token) {
        localStorage.setItem('dos_access_token', data.token);
        navigate('/courier');
      } else {
        setError('Invalid response from server');
      }
    } catch (err: any) {
      if (err.status === 404 && isDev) {
        // Dev-only mock fallback — never active in production
        localStorage.setItem('dos_access_token', 'dev_mock_token');
        navigate('/courier');
        return;
      }
      setError(err.status === 401 ? 'Invalid phone or password' : 'Login failed. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const handleActivateInvite = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const data = await apiClient<any>('/courier/auth/activate', {
        method: 'POST',
        body: { inviteCode }
      });
      if (data?.token) {
        localStorage.setItem('dos_access_token', data.token);
        navigate('/courier');
      } else if (data?.courierId) {
        navigate('/courier');
      }
    } catch (err: any) {
      if (err.status === 404 && isDev) {
        // Dev-only mock fallback
        localStorage.setItem('dos_access_token', 'dev_mock_token');
        navigate('/courier');
        return;
      }
      setError(err.status === 404 ? 'Invalid invite code' : 'Activation failed. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  if (mode === 'invite') {
    return (
      <div className="min-h-screen bg-[var(--brand-bg)] flex flex-col justify-center p-6">
        <div className="max-w-md w-full mx-auto space-y-8">
          <div className="text-center">
            <h1 className="text-3xl font-bold text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>
              Activate Account
            </h1>
            <p className="mt-2 text-sm text-[var(--brand-text-muted)]">
              Enter your invite code to get started
            </p>
          </div>

          <form onSubmit={handleActivateInvite} className="space-y-6">
            {error && (
              <div className="bg-[var(--status-cancelled-light)] border border-[var(--status-cancelled-border)] text-[var(--color-danger)] p-3 rounded-[var(--brand-radius-sm)] text-sm">
                {error}
              </div>
            )}

            <FormField label="Invite Code">
              <Input
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                placeholder="e.g. INVITE-123"
                required
                error={!!error}
              />
            </FormField>

            <Button
              type="submit"
              className="w-full"
              size="lg"
              isLoading={loading}
              disabled={!inviteCode.trim()}
            >
              Activate
            </Button>
          </form>

          <div className="text-center">
            <button
              type="button"
              onClick={() => { setMode('login'); setError(''); }}
              className="text-sm text-[var(--brand-primary)] hover:underline"
            >
              Already have an account? Log in
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[var(--brand-bg)] flex flex-col justify-center p-6">
      <div className="max-w-md w-full mx-auto space-y-8">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>
            Courier Login
          </h1>
          <p className="mt-2 text-sm text-[var(--brand-text-muted)]">
            Enter your phone and password to continue
          </p>
        </div>

        <form onSubmit={handleLogin} className="space-y-6">
          {error && (
            <div className="bg-[var(--status-cancelled-light)] border border-[var(--status-cancelled-border)] text-[var(--color-danger)] p-3 rounded-[var(--brand-radius-sm)] text-sm">
              {error}
            </div>
          )}

          <FormField label="Phone Number">
            <Input
              type="tel"
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
              placeholder="+355 69 123 4567"
              required
              error={!!error}
            />
          </FormField>

          <FormField label="Password">
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
            disabled={!phone.trim() || !password}
          >
            Log In
          </Button>
        </form>

        <div className="text-center">
          <button
            type="button"
            onClick={() => { setMode('invite'); setError(''); }}
            className="text-sm text-[var(--brand-primary)] hover:underline"
          >
            Activate with invite code
          </button>
        </div>
      </div>
    </div>
  );
}
