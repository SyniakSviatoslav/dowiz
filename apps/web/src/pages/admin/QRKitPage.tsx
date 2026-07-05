import React, { useEffect, useState } from 'react';
import QRCode from 'qrcode';
import { Button, EmptyState, SkeletonBase, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

// QR/ATTRIBUTION build lane — dark behind VITE_CHANNEL_KIT_ENABLED (default false; gated
// in AdminRoutes.tsx). Owner-facing "QR kit": a printable QR sticker (?ch=qr) and a
// copyable NFC-tag URL (?ch=nfc) for the venue's public storefront. The `qrcode` package
// is already a dependency (see SettingsPage.tsx's Telegram-connect QR) — reused here, no
// new dependency. `channel` travels write-only with the eventual order (apps/web/src/lib/
// channel.ts + apps/api/src/lib/channel.ts) — this page only builds the tagged URLs.

const QR_SIZE = 240;

export function QRKitPage() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [slug, setSlug] = useState('');
  const [locationName, setLocationName] = useState('');
  const [qrDataUrl, setQrDataUrl] = useState('');
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let alive = true;
    apiClient<any>('/owner/settings')
      .then((res: any) => {
        if (!alive) return;
        setSlug(res?.slug || '');
        setLocationName(res?.locationName || res?.name || '');
      })
      .catch(() => { if (alive) setLoadError(true); })
      .finally(() => { if (alive) setLoading(false); });
    return () => { alive = false; };
  }, []);

  const origin = typeof window !== 'undefined' ? window.location.origin : '';
  const qrUrl = slug ? `${origin}/s/${encodeURIComponent(slug)}?ch=qr` : '';
  const nfcUrl = slug ? `${origin}/s/${encodeURIComponent(slug)}?ch=nfc` : '';

  useEffect(() => {
    if (!qrUrl) { setQrDataUrl(''); return; }
    let alive = true;
    QRCode.toDataURL(qrUrl, {
      width: QR_SIZE,
      margin: 1,
      color: {
        dark: getComputedStyle(document.documentElement).getPropertyValue('--brand-text').trim() || '#000000',
        light: getComputedStyle(document.documentElement).getPropertyValue('--brand-bg').trim() || '#ffffff',
      },
    }).then((url) => { if (alive) setQrDataUrl(url); }).catch(() => { if (alive) setQrDataUrl(''); });
    return () => { alive = false; };
  }, [qrUrl]);

  const copyNfcUrl = () => {
    navigator.clipboard.writeText(nfcUrl).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }).catch(() => {});
  };

  if (loading) {
    return (
      <div className="p-4 max-w-2xl mx-auto space-y-4">
        <SkeletonBase className="h-8 w-48" />
        <SkeletonBase className="h-72 w-full rounded-[var(--brand-radius)]" />
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="p-4 max-w-2xl mx-auto">
        <EmptyState
          title={t('admin.qr_kit_load_error_title', 'Could not load your QR kit')}
          description={t('admin.qr_kit_load_error_desc', 'Check your connection and try reloading the page.')}
          icon={<i className="ti ti-alert-circle" aria-hidden="true" />}
        />
      </div>
    );
  }

  if (!slug) {
    return (
      <div className="p-4 max-w-2xl mx-auto">
        <EmptyState
          title={t('admin.qr_kit_empty_title', 'Publish your storefront first')}
          description={t('admin.qr_kit_empty_desc', 'Your QR kit needs a live storefront link — finish onboarding to get one.')}
          icon={<i className="ti ti-qrcode" aria-hidden="true" />}
        />
      </div>
    );
  }

  return (
    <div className="p-4 max-w-2xl mx-auto space-y-6">
      <style>{`
        @media print {
          body * { visibility: hidden; }
          #qr-kit-printable, #qr-kit-printable * { visibility: visible; }
          #qr-kit-printable { position: fixed; inset: 0; display: flex; align-items: center; justify-content: center; }
        }
      `}</style>

      <div>
        <h1 className="text-xl font-bold" style={{ color: 'var(--brand-text)' }}>{t('admin.qr_kit', 'QR kit')}</h1>
        <p className="text-sm mt-1" style={{ color: 'var(--brand-text-muted)' }}>
          {t('admin.qr_kit_subtitle', 'Print the QR code for tables/packaging, or copy the NFC-tag link — both point customers straight to your menu and let you see which one drove the order.')}
        </p>
      </div>

      <div
        id="qr-kit-printable"
        className="card-section bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-6 flex flex-col items-center gap-3 shadow-[var(--elev-1)]"
      >
        {locationName && (
          <div className="text-base font-bold" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{locationName}</div>
        )}
        {qrDataUrl ? (
          <img src={qrDataUrl} alt={t('admin.qr_kit_qr_alt', 'QR code linking to your storefront')} width={QR_SIZE} height={QR_SIZE} />
        ) : (
          <SkeletonBase className="w-[240px] h-[240px] rounded-[var(--brand-radius)]" />
        )}
        <p className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>{t('admin.qr_kit_qr_caption', 'Scan to order')}</p>
        <p className="font-mono text-xs break-all text-center" style={{ color: 'var(--brand-text-muted)' }}>{qrUrl}</p>
      </div>

      <div className="flex justify-center print:hidden">
        <Button variant="secondary" onClick={() => window.print()}>
          <i className="ti ti-printer" aria-hidden="true" />
          <span className="ml-1.5">{t('admin.qr_kit_print', 'Print')}</span>
        </Button>
      </div>

      <div className="card-section bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-2 shadow-[var(--elev-1)] print:hidden">
        <p className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>{t('admin.qr_kit_nfc_caption', 'NFC tag link')}</p>
        <p className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.qr_kit_nfc_hint', 'Write this URL to an NFC sticker so a tap opens your menu.')}</p>
        <div className="flex items-center gap-2 min-w-0">
          <span className="font-mono text-xs truncate min-w-0" style={{ color: 'var(--brand-primary)' }}>{nfcUrl}</span>
          <button
            type="button"
            onClick={copyNfcUrl}
            className="shrink-0 inline-flex items-center gap-1 text-step-2xs text-[var(--brand-text-muted)] hover:text-[var(--brand-primary)] underline rounded-[var(--brand-radius-sm)] px-1 py-0.5 transition-colors duration-150 active:scale-[0.97]"
          >
            <i className={`ti ${copied ? 'ti-check' : 'ti-copy'}`} aria-hidden="true" />
            {copied ? t('common.copied', 'Copied') : t('common.copy', 'Copy')}
          </button>
        </div>
      </div>
    </div>
  );
}
