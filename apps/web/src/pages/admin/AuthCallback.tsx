import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../../lib/apiClient.js';

// OAuth return page: the backend redirects here as /auth/callback#code=<opaque>.
// Exchange the one-time code for the owner session, store it, and enter /admin.
export function AuthCallback() {
  const navigate = useNavigate();
  const [error, setError] = useState('');

  useEffect(() => {
    (async () => {
      try {
        const raw = window.location.hash.replace(/^#/, '');
        const code = new URLSearchParams(raw).get('code');
        if (!code) { setError('Missing login code.'); return; }
        const res = await apiClient<any>('/auth/exchange', { method: 'POST', body: { code } });
        if (res?.access_token) {
          sessionStorage.setItem('dos_access_token', res.access_token);
          localStorage.setItem('dos_access_token', res.access_token);
          if (res.refresh_token) localStorage.setItem('dos_refresh_token', res.refresh_token);
          navigate('/admin', { replace: true });
        } else {
          setError('Login failed.');
        }
      } catch {
        setError('Login failed.');
      }
    })();
  }, [navigate]);

  return (
    <div className="min-h-screen flex items-center justify-center text-sm" style={{ background: 'var(--brand-surface)', color: 'var(--brand-text)' }}>
      {error || 'Signing you in…'}
    </div>
  );
}
