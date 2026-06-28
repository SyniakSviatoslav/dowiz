import React, { useEffect, useRef, useState } from 'react';
import { useI18n } from '@deliveryos/ui';
import { apiClient } from '../lib/index.js';
import { safeStorage } from '../lib/safeStorage.js';

// flow-simplification §6 — owner claim page. The token-safe transport (G-F2 / R2-3 / R3-1):
//  • the claim token arrives in the URL FRAGMENT (#token=...), never the query string (a query leaks via
//    Referer/access-logs/history). On mount we read it, then immediately history.replaceState to SCRUB it
//    from the visible URL, and hold it only in memory (a ref) for the lifetime of the page.
//  • accept POSTs the token in the BODY over TLS (apiClient adds the owner Bearer); the server refuses a
//    token-only (NULL-contact) invite (G-F2g) and binds ownership to the authed identity.
//  • PROTECTED FRICTION (council CC2/CC3): claim binds OWNERSHIP + login ONLY — it does NOT publish. Go-live
//    stays a separate, deliberate act in /admin (published_at stays NULL through claim).
const readToken = (): string => {
  if (typeof window === 'undefined') return '';
  const hash = window.location.hash || '';
  const fromHash = /[#&]token=([^&]+)/.exec(hash)?.[1];
  // Backward-compat: an older ?token= link still works, but is scrubbed just the same.
  const fromQuery = new URLSearchParams(window.location.search).get('token');
  // Post-sign-in return (the signin CTA stashes it so it survives the auth redirect; cleared once read).
  const stashed = sessionStorage.getItem('dos_claim_token') || '';
  if (stashed) sessionStorage.removeItem('dos_claim_token');
  return decodeURIComponent(fromHash || fromQuery || stashed);
};

type Phase = 'idle' | 'working' | 'claimed' | 'declined' | 'error';

export function ClaimPage() {
  const { t } = useI18n();
  const tokenRef = useRef<string>('');
  const [hasToken, setHasToken] = useState(false);
  const [authed, setAuthed] = useState(false);
  const [phase, setPhase] = useState<Phase>('idle');
  const [errorMsg, setErrorMsg] = useState('');

  useEffect(() => {
    // PROTECTED-FRICTION:claim — claim binds ownership+login only; go-live is a separate gated act (do not collapse).
    tokenRef.current = readToken();
    setHasToken(!!tokenRef.current);
    // Scrub the token from the visible URL (Referer/history/log hygiene) — keep the path, drop hash+query.
    try { window.history.replaceState(null, '', '/claim'); } catch { /* noop */ }
    setAuthed(!!safeStorage.get('dos_access_token') || !!sessionStorage.getItem('dos_access_token'));
  }, []);

  const humaneError = (code?: string): string => {
    switch (code) {
      case 'INVALID_OR_EXPIRED_TOKEN': return t('claim.err_expired', 'This claim link has expired or was already used. Ask us for a fresh one.');
      case 'ALREADY_CLAIMED': return t('claim.err_already', 'This restaurant has already been claimed.');
      case 'CONTACT_MISMATCH': return t('claim.err_contact', 'This claim link was sent to a different contact. Sign in with the invited account.');
      case 'CONTACT_REQUIRED': return t('claim.err_contact_required', 'This link can’t be claimed online. Please contact us to complete the claim.');
      default: return t('claim.err_generic', 'Something went wrong completing the claim. Please try again.');
    }
  };

  const doAccept = async () => {
    if (!tokenRef.current) return;
    setPhase('working'); setErrorMsg('');
    try {
      await apiClient('/claim/accept', { method: 'POST', body: { token: tokenRef.current } });
      tokenRef.current = '';
      setPhase('claimed');
    } catch (err: any) {
      setErrorMsg(humaneError(err?.data?.error || err?.data?.code));
      setPhase('error');
    }
  };

  const doDecline = async () => {
    if (!tokenRef.current) return;
    setPhase('working'); setErrorMsg('');
    try {
      // Token-only erase (no account needed) — the restaurant can remove the preview in one action.
      await apiClient('/claim/decline', { method: 'POST', body: { token: tokenRef.current } });
      tokenRef.current = '';
      setPhase('declined');
    } catch (err: any) {
      setErrorMsg(humaneError(err?.data?.error || err?.data?.code));
      setPhase('error');
    }
  };

  const Card: React.FC<{ children: React.ReactNode }> = ({ children }) => (
    <div className="min-h-screen flex items-center justify-center p-4" style={{ background: 'var(--brand-bg)' }}>
      <div className="w-full max-w-md rounded-[var(--brand-radius)] border p-6 space-y-4"
        style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }}>
        {children}
      </div>
    </div>
  );

  if (!hasToken) {
    return (
      <Card>
        <h1 className="text-step-xl font-bold" style={{ color: 'var(--brand-text)' }}>{t('claim.title', 'Claim your restaurant')}</h1>
        <p className="text-step-sm" style={{ color: 'var(--brand-text-muted)' }}>
          {t('claim.no_token', 'This page needs a valid claim link. Open the link from the message we sent you.')}
        </p>
      </Card>
    );
  }

  if (phase === 'claimed') {
    return (
      <Card>
        <div className="text-3xl" aria-hidden="true">🎉</div>
        <h1 className="text-step-xl font-bold" style={{ color: 'var(--brand-text)' }}>{t('claim.claimed_title', 'It’s yours')}</h1>
        <p className="text-step-sm" style={{ color: 'var(--brand-text-muted)' }}>
          {t('claim.claimed_body', 'You now own this restaurant. Review your menu, prices and details — then publish to go live. Nothing is public until you publish.')}
        </p>
        <a href="/admin" data-testid="claim-go-admin" className="inline-flex items-center justify-center w-full h-12 rounded-full font-bold text-step-sm"
          style={{ background: 'var(--brand-primary)', color: 'var(--brand-bg)' }}>
          {t('claim.review_publish', 'Review & publish')}
        </a>
      </Card>
    );
  }

  if (phase === 'declined') {
    return (
      <Card>
        <h1 className="text-step-xl font-bold" style={{ color: 'var(--brand-text)' }}>{t('claim.declined_title', 'Removed')}</h1>
        <p className="text-step-sm" style={{ color: 'var(--brand-text-muted)' }}>
          {t('claim.declined_body', 'We’ve deleted the preview we built. Nothing of yours remains. Sorry for the intrusion.')}
        </p>
      </Card>
    );
  }

  return (
    <Card>
      <h1 className="text-step-xl font-bold" style={{ color: 'var(--brand-text)' }} data-testid="claim-heading">{t('claim.title', 'Claim your restaurant')}</h1>
      <p className="text-step-sm" style={{ color: 'var(--brand-text-muted)' }}>
        {t('claim.intro', 'We built a working preview of your restaurant’s ordering page. Claim it to take ownership — then review and publish when you’re ready. Claiming does not make it public.')}
      </p>
      {phase === 'error' && <p role="alert" className="text-step-sm" style={{ color: 'var(--color-danger)' }}>{errorMsg}</p>}

      {authed ? (
        <button data-testid="claim-accept" onClick={doAccept} disabled={phase === 'working'}
          className="w-full h-12 rounded-full font-bold text-step-sm disabled:opacity-60"
          style={{ background: 'var(--brand-primary)', color: 'var(--brand-bg)' }}>
          {phase === 'working' ? t('claim.claiming', 'Claiming…') : t('claim.accept', 'Claim this restaurant')}
        </button>
      ) : (
        <button data-testid="claim-signin" onClick={() => { sessionStorage.setItem('dos_claim_token', tokenRef.current); window.location.href = '/login'; }}
          className="w-full h-12 rounded-full font-bold text-step-sm"
          style={{ background: 'var(--brand-primary)', color: 'var(--brand-bg)' }}>
          {t('claim.signin_to_claim', 'Sign in to claim')}
        </button>
      )}

      <button data-testid="claim-decline" onClick={doDecline} disabled={phase === 'working'}
        className="w-full text-step-xs py-2 disabled:opacity-60" style={{ color: 'var(--brand-text-muted)' }}>
        {t('claim.decline', 'This isn’t my restaurant — remove it')}
      </button>
    </Card>
  );
}

export default ClaimPage;
