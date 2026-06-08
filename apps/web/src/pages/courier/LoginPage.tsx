import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Input, FormField, LanguageSwitcher } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

export function LoginPage() {
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
      const data = await apiClient<any>('/courier/auth/login', {
        method: 'POST',
        body: { email, password }
      });
      if (data?.jwt) {
        localStorage.setItem('dos_access_token', data.jwt);
        navigate('/courier');
      } else {
        setError('Invalid response from server');
      }
    } catch (err: any) {
      setError(err.status === 401 ? 'Invalid email or password' : 'Login failed. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-[var(--brand-bg)] flex flex-col justify-center p-6">
      <div className="absolute top-4 right-4">
        <LanguageSwitcher variant="full" />
      </div>
      <div className="max-w-md w-full mx-auto space-y-8">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>
            Courier Login
          </h1>
          <p className="mt-2 text-sm text-[var(--brand-text-muted)]">
            Enter your email and password to continue
          </p>
        </div>

        <form onSubmit={handleLogin} className="space-y-6">
          {error && (
            <div className="bg-[var(--status-cancelled-light)] border border-[var(--status-cancelled-border)] text-[var(--color-danger)] p-3 rounded-[var(--brand-radius-sm)] text-sm">
              {error}
            </div>
          )}

          <FormField label="Email">
            <Input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="courier@example.com"
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
            disabled={!email.trim() || !password}
          >
            Log In
          </Button>
        </form>
      </div>
    </div>
  );
}
