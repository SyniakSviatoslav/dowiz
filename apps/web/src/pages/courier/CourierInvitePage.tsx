import React, { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { PHONE_E164_PATTERN } from '@deliveryos/shared-types';
import { Button, Input, FormField, EmptyState, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';
import { CourierInviteRedeemResponse } from '@deliveryos/shared-types';

const CourierInviteDetailResponse = z.custom<{
  locationName: string;
  role: string;
  isValid: boolean;
  isExpired: boolean;
  isUsed: boolean;
  isRevoked: boolean;
}>();

export function CourierInvitePage() {
  const { inviteId } = useParams<{ inviteId: string }>();
  const navigate = useNavigate();
  const { t } = useI18n();

  const [loadingInvite, setLoadingInvite] = useState(true);
  const [inviteError, setInviteError] = useState('');
  const [inviteData, setInviteData] = useState<{
    locationName: string;
    role: string;
    isValid: boolean;
    isExpired: boolean;
    isUsed: boolean;
    isRevoked: boolean;
  } | null>(null);

  // Form fields
  const [fullName, setFullName] = useState('');
  const [email, setEmail] = useState('');
  const [phone, setPhone] = useState('');
  const [password, setPassword] = useState('');
  const [code, setCode] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState('');

  useEffect(() => {
    if (!inviteId) {
      setInviteError(t('courier.invite.invalid_link', 'Invalid invite link'));
      setLoadingInvite(false);
      return;
    }

    setLoadingInvite(true);
    setInviteError('');

    apiClient<typeof CourierInviteDetailResponse>(`/courier/auth/invites/${inviteId}`, { schema: CourierInviteDetailResponse })
      .then((data) => {
        setInviteData(data);
        if (!data.isValid) {
          if (data.isExpired) {
            setInviteError(t('courier.invite.expired', 'This invite has expired'));
          } else if (data.isUsed) {
            setInviteError(t('courier.invite.used', 'This invite has already been used'));
          } else if (data.isRevoked) {
            setInviteError(t('courier.invite.revoked', 'This invite has been revoked'));
          } else {
            setInviteError(t('courier.invite.invalid', 'This invite is no longer valid'));
          }
        }
      })
      .catch((err) => {
        setInviteError(t('courier.invite.not_found', 'Invite not found or invalid'));
      })
      .finally(() => {
        setLoadingInvite(false);
      });
  }, [inviteId]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inviteId) return;

    setSubmitError('');
    setSubmitting(true);

    try {
      const data = await apiClient<typeof CourierInviteRedeemResponse>(`/courier/auth/invites/${inviteId}/redeem`, {
        method: 'POST',
        schema: CourierInviteRedeemResponse,
        body: {
          email,
          code: code.trim(),
          password,
          full_name: fullName,
          phone: phone.trim() || undefined,
        },
      });

      if (data?.jwt) {
        localStorage.setItem('dos_access_token', data.jwt);
        navigate('/courier');
      } else {
        setSubmitError(t('common.invalid_response', 'Invalid response from server'));
      }
    } catch (err: any) {
      if (err.status === 410) {
        setSubmitError(t('courier.invite.expired_or_invalid', 'Invite expired or invalid'));
      } else if (err.status === 401) {
        setSubmitError(t('courier.invite.wrong_code', 'Invalid invite code'));
      } else {
        const msg = err.message || t('courier.invite.registration_failed', 'Registration failed');
        setSubmitError(msg);
      }
    } finally {
      setSubmitting(false);
    }
  };

  if (loadingInvite) {
    return (
      <div className="min-h-screen bg-[var(--brand-bg)] flex items-center justify-center p-6 text-[var(--brand-text)]">
        <div className="text-center space-y-4">
          <div className="animate-spin h-8 w-8 border-2 border-brand-primary border-t-transparent rounded-full mx-auto" />
          <p className="text-sm text-[var(--brand-text-muted)]">{t('courier.invite.loading', 'Loading invite details...')}</p>
        </div>
      </div>
    );
  }

  if (inviteError || !inviteData?.isValid) {
    return (
      <div className="min-h-screen bg-[var(--brand-bg)] flex flex-col justify-center p-6 text-[var(--brand-text)]">
        <div className="max-w-md w-full mx-auto">
          <EmptyState
            title={t('courier.invite.invalid_title', 'Invalid Invite')}
            description={inviteError || t('courier.invite.cannot_use', 'The invite cannot be used')}
          />
          <div className="text-center mt-6">
            <Button onClick={() => navigate('/courier/login')} variant="outline" className="w-full">
              {t('courier.invite.return_to_login', 'Return to Login')}
            </Button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[var(--brand-bg)] flex flex-col justify-center p-6 text-[var(--brand-text)]">
      <div className="max-w-md w-full mx-auto space-y-8">
        <div className="text-center">
          <h1 className="text-3xl font-bold tracking-tight" style={{ fontFamily: 'var(--brand-font-heading)' }}>
            {t('courier.invite.join_as', 'Join as')} {inviteData.role === 'dispatcher' ? t('courier.invite.dispatcher', 'Dispatcher') : t('courier.title', 'Courier')}
          </h1>
          <p className="mt-2 text-sm text-[var(--brand-text-muted)]">
            {t('courier.invite.invite_from', 'Invite from')} <span className="font-semibold text-[var(--brand-text)]">{inviteData.locationName}</span>
          </p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4 bg-[var(--brand-surface)] p-6 rounded-2xl border border-[var(--brand-border)] shadow-sm">
          {submitError && (
            <div role="alert" aria-live="polite" className="bg-[var(--status-cancelled-light)] border border-[var(--status-cancelled-border)] text-[var(--color-danger)] p-3 rounded-xl text-sm">
              {submitError}
            </div>
          )}

          <FormField label={t('courier.invite.full_name', 'Full Name')}>
            <Input
              value={fullName}
              onChange={(e) => setFullName(e.target.value)}
              placeholder={t('courier.invite.full_name_placeholder', 'e.g. Alban Hoxha')}
              required
              disabled={submitting}
            />
          </FormField>

          <FormField label="Email">
            <Input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder={t('courier.invite.email_placeholder', 'example@email.com')}
              required
              disabled={submitting}
            />
          </FormField>

          <FormField label={t('courier.invite.phone_label', 'Phone Number (Optional)')}>
            <Input
              type="tel"
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
              placeholder="+355 69 123 4567"
              pattern={PHONE_E164_PATTERN}
              title="+355 followed by 7-14 digits"
              disabled={submitting}
            />
          </FormField>

          <FormField label={t('courier.invite.password_label', 'Password (Min. 12 characters)')}>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t('courier.invite.password_placeholder', 'Choose a strong password')}
              required
              minLength={12}
              disabled={submitting}
            />
          </FormField>

          <FormField label={t('courier.invite.code_label', 'Invite Security Code (16 chars)')}>
            <Input
              value={code}
              onChange={(e) => setCode(e.target.value)}
              placeholder={t('courier.invite.code_placeholder', 'Enter the 16-digit security code')}
              required
              maxLength={16}
              disabled={submitting}
              className="font-mono text-center tracking-widest text-lg"
            />
          </FormField>

          <Button
            type="submit"
            className="w-full mt-2"
            size="lg"
            isLoading={submitting}
            disabled={!fullName.trim() || !email.trim() || password.length < 12 || code.trim().length !== 16}
          >
            {t('courier.invite.accept_register', 'Accept & Register')}
          </Button>
        </form>

        <div className="text-center">
          <button
            type="button"
            onClick={() => navigate('/courier/login')}
            className="text-sm text-[var(--brand-primary)] hover:underline"
          >
            {t('courier.invite.already_have_account', 'Already have an account?')}
          </button>
        </div>
      </div>
    </div>
  );
}
