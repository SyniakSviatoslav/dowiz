import React, { useState } from 'react';
import { Button, Input, useI18n } from '@deliveryos/ui';
import { Link } from 'react-router-dom';
import { decideOutcome, type FormState, type ErrKind } from './accessRequestOutcome.js';

export { decideOutcome };

// AccessRequestForm — public "register interest" capture (ADR-soft-access-gate).
// Render gate (secondary to the backend route gate): only mounts when the build flag is
// on. Backend POST is the load-bearing STOP-1 gate (route 404s while ACCESS_GATE_PUBLIC_
// ENABLED is off); this hides the UI so nothing is exposed pre-launch.
export const ACCESS_GATE_ENABLED =
  (import.meta as any).env?.VITE_ACCESS_GATE_PUBLIC_ENABLED === 'true';

export function AccessRequestForm() {
  const { t } = useI18n();
  const [email, setEmail] = useState('');
  const [consent, setConsent] = useState(false);
  const [honeypot, setHoneypot] = useState(''); // company_url_hp — bots fill it, humans don't
  const [state, setState] = useState<FormState>('idle');
  const [errKind, setErrKind] = useState<ErrKind>('generic');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!consent) return; // button is disabled too; belt + suspenders
    setState('submitting');

    // Build the exact body we send, then locally verify consent is in it (R3-3a).
    const payload = {
      email,
      consent, // boolean
      website: honeypot,
      locale: typeof document !== 'undefined' ? document.documentElement.lang || undefined : undefined,
    };
    const sentConsent = payload.consent === true;

    try {
      const res = await fetch('/api/access-requests', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      const outcome = decideOutcome(sentConsent, { ok: res.ok, status: res.status });
      if (outcome.state === 'success') {
        setState('success');
      } else {
        setErrKind(outcome.err ?? 'generic');
        setState('error');
      }
    } catch {
      setErrKind('generic');
      setState('error');
    }
  };

  if (state === 'success') {
    return (
      <div
        data-testid="access-request-success"
        role="status"
        aria-live="polite"
        className="rounded-lg p-4 text-sm text-center"
        style={{ background: 'var(--status-confirmed-light, rgba(34,197,94,0.10))', border: '1px solid var(--brand-border)', color: 'var(--brand-text)' }}
      >
        {t('accessRequest.success', "Thanks — we've got your email and we'll be in touch.")}
      </div>
    );
  }

  return (
    <form data-testid="access-request-form" onSubmit={handleSubmit} className="rounded-lg p-5 space-y-4" style={{ background: 'var(--brand-surface-raised)', border: '1px solid var(--brand-border)' }}>
      <div className="space-y-1">
        <h3 className="text-lg font-bold" style={{ color: 'var(--brand-text)' }}>{t('accessRequest.heading', 'Be the first to know')}</h3>
        <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{t('accessRequest.sub', "Leave your email and we'll reach out when we're ready for you.")}</p>
      </div>

      <div className="space-y-1.5">
        <label htmlFor="ar-email" className="text-sm font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('accessRequest.emailLabel', 'Email')}</label>
        <Input
          id="ar-email"
          data-testid="access-request-email"
          type="email"
          autoComplete="email"
          required
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder={t('accessRequest.emailPlaceholder', 'you@example.com')}
        />
      </div>

      {/* Honeypot (B11, secondary): off-screen, not display:none, tabbable-out, autofill-resistant name. */}
      <div aria-hidden="true" style={{ position: 'absolute', left: '-9999px', width: 1, height: 1, overflow: 'hidden' }}>
        <label htmlFor="company_url_hp">Company URL</label>
        <input
          id="company_url_hp"
          name="company_url_hp"
          type="text"
          tabIndex={-1}
          autoComplete="off"
          value={honeypot}
          onChange={(e) => setHoneypot(e.target.value)}
        />
      </div>

      <label className="flex items-start gap-2 text-sm" style={{ color: 'var(--brand-text-muted)' }}>
        <input
          data-testid="access-request-consent"
          type="checkbox"
          className="mt-0.5"
          style={{ minWidth: 18, minHeight: 18 }}
          checked={consent}
          onChange={(e) => setConsent(e.target.checked)}
        />
        <span>
          {t('accessRequest.consentLabel', 'I agree to be contacted by email, and I agree to the')}{' '}
          <Link to="/privacy" data-testid="access-request-privacy-link" style={{ color: 'var(--brand-primary)', textDecoration: 'underline' }}>
            {t('accessRequest.privacyLink', 'Privacy Notice')}
          </Link>.
        </span>
      </label>

      {state === 'error' && (
        <div data-testid="access-request-error" role="alert" aria-live="polite" className="text-sm" style={{ color: 'var(--color-danger)' }}>
          {errKind === 'rate'
            ? t('accessRequest.err429', 'One moment — too many tries. Please wait a minute.')
            : t('accessRequest.errGeneric', 'Something went wrong. Please try again.')}
        </div>
      )}

      <Button
        type="submit"
        variant="primary"
        className="w-full"
        data-testid="access-request-submit"
        disabled={!consent || state === 'submitting'}
        style={{ minHeight: 44 }}
      >
        {state === 'submitting' ? t('accessRequest.submitting', 'Sending…') : t('accessRequest.cta', 'Keep me posted')}
      </Button>
    </form>
  );
}

/** Flag-gated wrapper: renders nothing when the feature is off (default). */
export function AccessRequestGate() {
  if (!ACCESS_GATE_ENABLED) return null;
  return <AccessRequestForm />;
}
