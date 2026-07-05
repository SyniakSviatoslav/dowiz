import React, { useRef } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { useI18n, ease, Select } from '@deliveryos/ui';
import { MESSENGER_KINDS, messengerLabel, messengerInputType, messengerIsPhone } from '../../../lib/messenger.js';
import { normalizeAlbanianPhone } from './phone.js';
import type { DeliveryType } from './types.js';

interface ContactInfoSectionProps {
  deliveryType: DeliveryType;
  customerName: string;
  setCustomerName: React.Dispatch<React.SetStateAction<string>>;
  phone: string;
  setPhone: React.Dispatch<React.SetStateAction<string>>;
  phoneError: string;
  setPhoneError: React.Dispatch<React.SetStateAction<string>>;
  commError: string;
  setCommError: React.Dispatch<React.SetStateAction<string>>;
  messengerKind: string;
  setMessengerKind: React.Dispatch<React.SetStateAction<string>>;
  messengerHandle: string;
  setMessengerHandle: React.Dispatch<React.SetStateAction<string>>;
  sameReceiver: boolean;
  setSameReceiver: React.Dispatch<React.SetStateAction<boolean>>;
  receiverName: string;
  setReceiverName: React.Dispatch<React.SetStateAction<string>>;
  receiverKind: string;
  setReceiverKind: React.Dispatch<React.SetStateAction<string>>;
  receiverHandle: string;
  setReceiverHandle: React.Dispatch<React.SetStateAction<string>>;
  entryPhotoKey: string;
  entryPhotoPreview: string;
  photoUploading: boolean;
  uploadEntryPhoto: (file: File) => Promise<void>;
}

// Contact card: name + REQUIRED communication channel (ADR-0016) + receiver + entrance photo.
export function ContactInfoSection({
  deliveryType,
  customerName, setCustomerName,
  phone, setPhone,
  phoneError, setPhoneError,
  commError, setCommError,
  messengerKind, setMessengerKind,
  messengerHandle, setMessengerHandle,
  sameReceiver, setSameReceiver,
  receiverName, setReceiverName,
  receiverKind, setReceiverKind,
  receiverHandle, setReceiverHandle,
  entryPhotoKey, entryPhotoPreview, photoUploading, uploadEntryPhoto,
}: ContactInfoSectionProps) {
  const { t } = useI18n();
  const prefersReducedMotion = useReducedMotion();
  const entryFileRef = useRef<HTMLInputElement>(null);

  return (
    <motion.div
      initial={prefersReducedMotion ? false : { opacity: 0, y: 12 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-40px' }}
      transition={{ duration: 0.25, ease: ease.out }}
      className="rounded-[var(--brand-radius)] p-4 border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }}>
      <h2 className="text-step-xl font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.contact_info', 'Contact Info')}</h2>
      <div className="space-y-3">
        <div>
          <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.name', 'Name')}</label>
          <div className="relative">
            <i className="ti ti-user absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
            <input required value={customerName} onChange={e => setCustomerName(e.target.value)} placeholder={t('checkout.name_placeholder', 'Your name')} autoComplete="name" className="w-full h-[48px] pl-10 pr-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          </div>
        </div>
        {/* Communication (ADR-0016): REQUIRED channel + per-kind input. Phone-yielding kinds
            (phone/whatsapp/viber/signal) drive `phone` (throttle/OTP/dedup); Telegram=username,
            SimpleX=text-only. The phone field is folded into the Phone kind's input. */}
        <div>
          <label className="text-step-sm font-bold mb-1 block" style={{ color: 'var(--brand-text)' }}>
            {t('checkout.communication', 'Communication')} <span aria-hidden="true" style={{ color: 'var(--color-danger)' }}>*</span>
          </label>
          <p className="text-step-2xs mb-2" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.communication_why', 'The courier will message you about your order.')}</p>
          <Select
            value={messengerKind}
            onChange={e => { setMessengerKind(e.target.value); setMessengerHandle(''); setPhone(''); setPhoneError(''); setCommError(''); }}
            data-testid="checkout-communication"
            aria-label={t('checkout.communication', 'Communication')}>
            <option value="" disabled>{t('checkout.communication_choose', 'Choose a channel…')}</option>
            {MESSENGER_KINDS.map(k => (
              <option key={k} value={k}>{messengerLabel(k)}</option>
            ))}
          </Select>
          {messengerKind && (
            <div className="relative mt-2">
              {messengerIsPhone(messengerKind) && <i className="ti ti-phone absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />}
              <input
                value={messengerIsPhone(messengerKind) ? phone : messengerHandle}
                onChange={e => { const v = e.target.value; if (messengerIsPhone(messengerKind)) { setPhone(v); setPhoneError(''); } else { setMessengerHandle(v); } setCommError(''); }}
                onBlur={messengerIsPhone(messengerKind) ? () => setPhone(p => normalizeAlbanianPhone(p)) : undefined}
                type={messengerIsPhone(messengerKind) ? 'tel' : 'text'} inputMode={messengerIsPhone(messengerKind) ? 'tel' : undefined}
                autoComplete={messengerIsPhone(messengerKind) ? 'tel' : 'off'}
                aria-label={t('checkout.communication_handle', 'Your contact')} data-testid="checkout-comm-handle"
                placeholder={messengerInputType(messengerKind) === 'phone' ? '+355 6X XXX XXXX' : messengerInputType(messengerKind) === 'username' ? '@username' : t('checkout.simplex_placeholder', 'Paste your SimpleX invite link')}
                className="w-full h-[48px] pr-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]"
                style={{ paddingLeft: messengerIsPhone(messengerKind) ? '2.5rem' : '0.75rem', background: 'var(--brand-surface-raised)', borderColor: (phoneError || commError) ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
            </div>
          )}
          {(phoneError || commError) && <p role="alert" className="text-step-xs mt-1" style={{ color: 'var(--color-danger)' }}>{phoneError || commError}</p>}
        </div>
        {/* "Deliver to someone else" — same-receiver checked by default; else a receiver contact + notice. */}
        <div>
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <input type="checkbox" checked={sameReceiver} onChange={e => { setSameReceiver(e.target.checked); setCommError(''); }} data-testid="checkout-same-receiver" className="w-4 h-4" style={{ accentColor: 'var(--brand-primary)' }} />
            <span className="text-step-sm" style={{ color: 'var(--brand-text)' }}>{t('checkout.same_receiver', 'I am the receiver')}</span>
          </label>
          {!sameReceiver && (
            <div className="mt-3 space-y-2 rounded-[var(--brand-radius-sm)] border p-3" style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface-raised)' }} data-testid="receiver-fields">
              <input value={receiverName} onChange={e => { setReceiverName(e.target.value); setCommError(''); }} placeholder={t('checkout.receiver_name', 'Receiver’s name')} data-testid="receiver-name"
                className="w-full h-[48px] px-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
              <Select
                value={receiverKind}
                onChange={e => { setReceiverKind(e.target.value); setReceiverHandle(''); setCommError(''); }}
                data-testid="receiver-communication"
                aria-label={t('checkout.communication', 'Communication')}>
                <option value="" disabled>{t('checkout.communication_choose', 'Choose a channel…')}</option>
                {MESSENGER_KINDS.map(k => (
                  <option key={k} value={k}>{messengerLabel(k)}</option>
                ))}
              </Select>
              {receiverKind && (
                <input value={receiverHandle} onChange={e => { setReceiverHandle(e.target.value); setCommError(''); }}
                  type={messengerIsPhone(receiverKind) ? 'tel' : 'text'} data-testid="receiver-handle"
                  placeholder={messengerInputType(receiverKind) === 'phone' ? '+355 6X XXX XXXX' : messengerInputType(receiverKind) === 'username' ? '@username' : t('checkout.simplex_placeholder', 'Paste your SimpleX invite link')}
                  className="w-full h-[48px] px-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
              )}
              <p className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.receiver_privacy', 'We share this contact with the courier only to deliver this order, then delete it.')}</p>
            </div>
          )}
        </div>
        {/* UX-3: optional entrance photo (delivery only) — camera or gallery */}
        {deliveryType !== 'pickup' && (
          <div>
            <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.entry_photo', 'Entrance photo (optional)')}</label>
            <div className="flex items-center gap-3">
              <button type="button" onClick={() => entryFileRef.current?.click()} disabled={photoUploading}
                className="inline-flex items-center gap-2 min-h-[44px] px-4 py-2 border rounded-[var(--brand-radius-sm)] cursor-pointer text-sm transition-[background-color,box-shadow,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 disabled:opacity-60"
                style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
                <i className="ti ti-camera" aria-hidden="true" />
                {photoUploading ? t('checkout.uploading', 'Uploading…') : (entryPhotoKey ? t('checkout.change_photo', 'Change photo') : t('checkout.add_photo', 'Add photo'))}
              </button>
              <input ref={entryFileRef} type="file" accept="image/*" className="hidden" data-testid="entry-photo-input" disabled={photoUploading}
                onChange={e => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) void uploadEntryPhoto(f); }} />
              {entryPhotoPreview && (
                <img src={entryPhotoPreview} alt={t('checkout.entry_photo', 'Entrance photo')} data-testid="entry-photo-preview" className="h-12 w-12 object-cover rounded-[var(--brand-radius-sm)] border" style={{ borderColor: 'var(--brand-border)' }} />
              )}
            </div>
            <p className="text-step-xs mt-1" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.entry_photo_hint', 'Helps the courier find your entrance.')}</p>
          </div>
        )}
      </div>
    </motion.div>
  );
}
