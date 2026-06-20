import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Button, Modal, useI18n } from '../../index.js';

interface OTPModalProps {
  open: boolean;
  onClose: () => void;
  /** The phone number being verified (already collected at checkout). */
  phone: string;
  /** Send (or re-send) a code to `phone`. Resolves when the code is on its way. */
  onResend: () => Promise<void>;
  /** Verify the 6-digit code. Reject with an Error whose message is shown inline. */
  onVerify: (code: string) => Promise<void>;
  /** When true, a code has already been sent (skip the implicit auto-send). */
  alreadySent?: boolean;
}

const CODE_LENGTH = 6;

/**
 * Brand-register phone verification step. Mobile-first: a single 6-digit input
 * with `inputmode="numeric"` + `autocomplete="one-time-code"` so iOS/Android
 * surface the SMS code, 44px targets, focus-visible ring, resend with cooldown.
 */
export function OTPModal({ open, onClose, phone, onResend, onVerify, alreadySent }: OTPModalProps) {
  const { t } = useI18n();
  const [code, setCode] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [cooldown, setCooldown] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Reset + focus whenever the modal opens.
  useEffect(() => {
    if (open) {
      setCode('');
      setError('');
      setCooldown(alreadySent ? 30 : 0);
      // Focus the field once the portal is mounted.
      const id = setTimeout(() => inputRef.current?.focus(), 80);
      return () => clearTimeout(id);
    }
    return undefined;
  }, [open, alreadySent]);

  // Resend cooldown ticker.
  useEffect(() => {
    if (cooldown <= 0) return undefined;
    const id = setInterval(() => setCooldown((c) => (c <= 1 ? 0 : c - 1)), 1000);
    return () => clearInterval(id);
  }, [cooldown]);

  const handleVerify = useCallback(async () => {
    if (code.length !== CODE_LENGTH || loading) return;
    setLoading(true);
    setError('');
    try {
      await onVerify(code);
      // Parent closes on success.
    } catch (err: any) {
      setError(err?.message || t('otp.invalid', 'That code didn’t match. Try again.'));
      setCode('');
      inputRef.current?.focus();
    } finally {
      setLoading(false);
    }
  }, [code, loading, onVerify, t]);

  // Auto-submit once 6 digits are entered. Intentionally keyed on `code` only.
  useEffect(() => {
    if (code.length === CODE_LENGTH && !loading) {
      void handleVerify();
    }
  }, [code, loading, handleVerify]);

  const handleResend = async () => {
    if (cooldown > 0 || loading) return;
    setError('');
    setCode('');
    try {
      await onResend();
      setCooldown(30);
      inputRef.current?.focus();
    } catch (err: any) {
      setError(err?.message || t('otp.send_failed', 'Couldn’t send the code. Try again.'));
    }
  };

  return (
    <Modal isOpen={open} onClose={onClose} title={t('otp.title', 'Confirm your number')}>
      <div className="space-y-4">
        <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>
          {t('otp.sent_to', 'We sent a 6-digit code to')}{' '}
          <span className="font-semibold" style={{ color: 'var(--brand-text)' }}>{phone}</span>.
        </p>

        <label htmlFor="otp-code" className="text-[13px] font-bold block" style={{ color: 'var(--brand-text)' }}>
          {t('otp.code_label', 'Verification code')}
        </label>
        <input
          id="otp-code"
          ref={inputRef}
          type="text"
          inputMode="numeric"
          autoComplete="one-time-code"
          pattern="\d{6}"
          maxLength={CODE_LENGTH}
          value={code}
          onChange={(e) => { setError(''); setCode(e.target.value.replace(/\D/g, '').slice(0, CODE_LENGTH)); }}
          onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); void handleVerify(); } }}
          placeholder="000000"
          aria-label={t('otp.code_label', 'Verification code')}
          aria-invalid={!!error}
          data-testid="otp-code-input"
          className="w-full h-[56px] text-center text-2xl font-bold tracking-[0.5em] outline-none border rounded-[10px] transition-colors focus-visible:ring-2"
          style={{
            background: 'var(--brand-surface-raised)',
            borderColor: error ? 'var(--color-danger)' : 'var(--brand-border)',
            color: 'var(--brand-text)',
          }}
        />

        {error && (
          <p role="alert" className="text-sm" style={{ color: 'var(--color-danger)' }}>{error}</p>
        )}

        <Button
          className="w-full"
          style={{ minHeight: 'var(--tap-critical, 44px)' }}
          onClick={handleVerify}
          isLoading={loading}
          disabled={code.length !== CODE_LENGTH}
          data-testid="otp-verify-button"
        >
          {t('otp.verify_cta', 'Verify & place order')}
        </Button>

        <div className="text-center">
          <button
            type="button"
            onClick={handleResend}
            disabled={cooldown > 0 || loading}
            className="text-sm underline disabled:opacity-50 disabled:no-underline"
            style={{ color: 'var(--brand-primary)', minHeight: 'var(--tap-min, 44px)' }}
          >
            {cooldown > 0
              ? t('otp.resend_in', 'Resend code in {{n}}s', { n: cooldown })
              : t('otp.resend', 'Resend code')}
          </button>
        </div>
      </div>
    </Modal>
  );
}
