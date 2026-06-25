import { safeStorage } from '../lib/safeStorage.js';
import React, { useCallback, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Input, FormField, useI18n } from '@deliveryos/ui';
import { PHONE_E164_PATTERN } from '@deliveryos/shared-types';
import { apiClient, ApiError } from '../lib/index.js';
import { AccessRequestGate } from '../components/AccessRequestForm.js';

// Menu-first onboarding. The front door is "upload your menu" — we parse it with
// the zero-dependency heuristic parser, pre-fill the storefront identity
// (name·phone·slug) and show the items we found, then the owner claims it.
//   mode="anonymous" (public /start): claim = authenticate with Telegram, then
//     POST /owner/onboarding/start with the stashed import id → location seeded.
//   mode="authed" (/admin/onboarding, already signed in): no Telegram step —
//     the same upload+review, then create+seed directly.
// A "start without a menu" path preserves the manual create flow.
//
// Design: deliberately simple — a heading, the upload card, and the form. No
// animated hero / WebGL scene (the paper/Nomadic landing was rolled back).

const RESERVED = ['admin', 's', 'api', 'onboarding', 'courier', 'health', 'login', 'orders', 'menu', 'start'];

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/[ë]/g, 'e').replace(/[ç]/g, 'c')
    .replace(/[^a-z0-9\s-]/g, '')
    .replace(/\s+/g, '-').replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 50);
}

type Phase = 'choose' | 'parsing' | 'review' | 'blank' | 'submitting';

interface Preview {
  categories: number;
  productNames: string[];
  productCount: number;
}

export function MenuFirstOnboarding({ mode }: { mode: 'anonymous' | 'authed' }) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const fileRef = useRef<HTMLInputElement>(null);

  const [phase, setPhase] = useState<Phase>('choose');
  const [importId, setImportId] = useState<string | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [name, setName] = useState('');
  const [phone, setPhone] = useState('');
  const [slug, setSlug] = useState('');
  const [slugError, setSlugError] = useState('');
  const [tgWaiting, setTgWaiting] = useState(false);
  const [tgLink, setTgLink] = useState('');
  const [error, setError] = useState('');
  const [dragActive, setDragActive] = useState(false);

  const handleNameChange = useCallback((v: string) => {
    setName(v);
    if (!slug || slug === slugify(name)) {
      const s = slugify(v);
      setSlug(s);
      setSlugError(RESERVED.includes(s) ? t('admin.reserved_name', 'This name is reserved') : '');
    }
  }, [slug, name, t]);

  const handleSlugChange = useCallback((v: string) => {
    const s = v.toLowerCase().replace(/[^a-z0-9-]/g, '').slice(0, 50);
    setSlug(s);
    if (RESERVED.includes(s)) setSlugError(t('admin.reserved_name', 'This name is reserved'));
    else if (s.length < 3) setSlugError(t('admin.too_short', 'Too short (min 3)'));
    else setSlugError('');
  }, [t]);

  const onPickFile = () => fileRef.current?.click();

  // Drag-and-drop onto the dropzone. dragover must preventDefault for a drop to
  // fire; we mirror the click path.
  const onDragOver = (e: React.DragEvent) => { e.preventDefault(); if (!dragActive) setDragActive(true); };
  const onDragLeave = (e: React.DragEvent) => { e.preventDefault(); setDragActive(false); };
  const onDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragActive(false);
    const f = e.dataTransfer?.files?.[0];
    if (f) void onFile(f);
  };

  const onFile = async (file: File) => {
    if (!file) return;
    setError('');
    setPhase('parsing');
    try {
      const form = new FormData();
      form.append('file', file);
      const res = await apiClient<any>('/owner/menu/import/anonymous', { method: 'POST', body: form, timeout: 120000 });
      setImportId(res.anonymous_import_id);
      const r = res.restaurant || {};
      const nm = (r.name || '').trim();
      if (nm) handleNameChange(nm);
      if ((r.phone || '').trim()) setPhone(String(r.phone).trim());
      const prods = res.draft_preview?.products || [];
      const cats = res.draft_preview?.categories || [];
      setPreview({ categories: cats.length, productCount: prods.length, productNames: prods.slice(0, 6).map((p: any) => p.name) });
      setPhase('review');
    } catch (err) {
      setPhase('choose');
      const msg = err instanceof ApiError && err.data?.code === 'UNSUPPORTED_TYPE'
        ? t('start.unsupported', 'Please upload a PDF or photo of your menu.')
        : t('start.parse_failed', "We couldn't read that file. Try a clearer PDF or photo.");
      setError(msg);
    }
  };

  const canSubmit = name.trim().length >= 2 && phone.trim().length >= 8 && slug.length >= 3 && !slugError;

  const submitOnboarding = async (): Promise<boolean> => {
    setPhase('submitting');
    try {
      await apiClient<any>('/owner/onboarding/start', {
        method: 'POST',
        body: { name: name.trim(), phone: phone.trim(), slug, ...(importId ? { anonymous_import_id: importId } : {}) },
      });
      navigate('/admin/activation', { replace: true });
      return true;
    } catch (err) {
      setPhase(importId ? 'review' : 'blank');
      if (err instanceof ApiError && (err.status === 409 || err.data?.code === 'SLUG_TAKEN')) {
        setSlugError(t('admin.slug_taken', 'That link is already taken — try another.'));
      } else {
        setError(t('admin.create_failed', 'Could not create your storefront. Please try again.'));
      }
      return false;
    }
  };

  // Anonymous claim: authenticate with Telegram, then create+seed.
  const handleClaim = async () => {
    if (!canSubmit) return;
    if (mode === 'authed') { void submitOnboarding(); return; }
    setError('');
    setTgWaiting(true);
    try {
      const res = await apiClient<any>('/auth/telegram/start', { method: 'POST' });
      setTgLink(res.deepLink);
      window.open(res.deepLink, '_blank');
      const token = res.token;
      const deadline = Date.now() + 5 * 60 * 1000;
      const poll = async () => {
        if (Date.now() > deadline) { setTgWaiting(false); setError(t('admin.tg_timeout', 'Telegram login timed out. Please try again.')); return; }
        try {
          const p = await apiClient<any>(`/auth/telegram/poll?token=${token}`);
          if (p.status === 'authenticated' && p.access_token) {
            sessionStorage.setItem('dos_access_token', p.access_token);
            safeStorage.set('dos_access_token', p.access_token);
            if (p.refresh_token) safeStorage.set('dos_refresh_token', p.refresh_token);
            setTgWaiting(false);
            void submitOnboarding();
            return;
          }
        } catch (e: any) {
          if (e?.status === 410) { setTgWaiting(false); setError(t('admin.tg_expired', 'Telegram login link expired. Please try again.')); return; }
        }
        setTimeout(poll, 2000);
      };
      setTimeout(poll, 2000);
    } catch {
      setTgWaiting(false);
      setError(t('admin.tg_failed', 'Could not start Telegram login.'));
    }
  };

  const S = {
    card: {
      background: 'var(--brand-surface)',
      borderRadius: 'var(--brand-radius)',
      boxShadow: 'var(--elev-2)',
      padding: 'var(--space-5, 1.25rem)',
    },
    heading: { fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' },
    // Body/helper copy uses --brand-text (not -muted) so it clears 4.5:1.
    helper: { color: 'var(--brand-text)' },
    muted: { color: 'var(--brand-text-muted)' },
  } as const;

  const claimLabel = mode === 'anonymous'
    ? (tgWaiting ? t('admin.tg_waiting', 'Waiting for Telegram…') : t('start.claim_telegram', 'Claim with Telegram'))
    : t('admin.create_continue', 'Create & continue');

  return (
    <div className="min-h-screen bg-[var(--brand-bg)] text-[var(--brand-text)]">
      <style>{ONBOARD_CSS}</style>
      <div className="w-full max-w-lg mx-auto px-4 py-8 md:py-12 min-w-0">
        <input
          ref={fileRef}
          type="file"
          accept="application/pdf,image/*"
          className="hidden"
          data-testid="menu-file-input"
          onChange={(e) => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) void onFile(f); }}
        />

        {error && (
          <div
            role="alert"
            aria-live="polite"
            className="dz-fade-in mb-5 flex items-start gap-2 p-3 text-sm"
            style={{ background: 'var(--status-cancelled-light)', border: '1px solid var(--status-cancelled-border)', borderRadius: 'var(--brand-radius)', color: 'var(--color-danger)' }}
          >
            <i className="ti ti-alert-circle shrink-0 mt-0.5" aria-hidden="true" />
            <span className="min-w-0">{error}</span>
          </div>
        )}

        {/* ── CHOOSE: upload your menu ── */}
        {phase === 'choose' && (
          <div className="space-y-5">
            {/* Soft access gate (ADR-soft-access-gate). Renders only when the build flag
                is on (default off) — public "register interest" capture on the landing. */}
            {mode === 'anonymous' && <AccessRequestGate />}

            <div>
              <h1 className="text-2xl font-bold tracking-tight" style={S.heading}>{t('start.title', 'Start with your menu')}</h1>
              <p className="mt-2 text-sm leading-relaxed" style={S.helper}>
                {t('start.subtitle', 'Upload a PDF or photo of your menu. We’ll read it, set up your storefront, and bring your items to life — review everything before anything goes public.')}
              </p>
            </div>

            <div style={S.card} className="dz-fade-in space-y-5">
              <button
                type="button"
                onClick={onPickFile}
                onDragOver={onDragOver}
                onDragEnter={onDragOver}
                onDragLeave={onDragLeave}
                onDrop={onDrop}
                data-testid="upload-menu-cta"
                data-drag-active={dragActive ? '' : undefined}
                className={`dz-dropzone group w-full py-10 px-4 text-center min-w-0
                  motion-safe:transition-[transform,box-shadow,border-color] duration-150 ease-[var(--ease-soft)]
                  hover:-translate-y-0.5 active:scale-[0.99]
                  focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2${dragActive ? ' dz-dropzone-active' : ''}`}
                style={{
                  border: dragActive ? '2px solid var(--brand-primary)' : '2px dashed var(--brand-border)',
                  borderRadius: 'var(--brand-radius)',
                  background: 'var(--brand-surface-raised)',
                  color: 'var(--brand-text)',
                }}
              >
                <i className={`ti ${dragActive ? 'ti-file-download' : 'ti-file-upload'} text-3xl motion-safe:transition-transform duration-150 ease-[var(--ease-soft)] group-hover:-translate-y-0.5`} style={{ color: 'var(--brand-primary)' }} aria-hidden="true" />
                <div className="mt-2 font-semibold">{dragActive ? t('start.drop_cta', 'Drop to upload') : t('start.upload_cta', 'Upload your menu')}</div>
                <div className="mt-0.5 text-sm" style={S.muted}>{t('start.upload_hint', 'PDF or photo · up to 10MB')}</div>
              </button>
              <div className="flex flex-wrap items-center justify-between gap-3 pt-1">
                <button type="button" className="text-sm underline underline-offset-2 rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 motion-safe:transition-colors duration-150" style={S.muted}
                  onClick={() => (mode === 'anonymous' ? navigate('/login') : setPhase('blank'))}>
                  {t('start.no_menu', 'Start without a menu')}
                </button>
                {mode === 'anonymous' && (
                  <button type="button" className="text-sm underline underline-offset-2 rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 motion-safe:transition-colors duration-150" style={S.muted} onClick={() => navigate('/login')}>
                    {t('start.have_account', 'I already have an account')}
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

        {/* ── PARSING ── */}
        {phase === 'parsing' && <ParsingState />}

        {/* ── REVIEW (after parse) or BLANK (manual) ── */}
        {(phase === 'review' || phase === 'blank' || phase === 'submitting') && (
          <form
            style={S.card}
            className="dz-fade-in space-y-5"
            onSubmit={(e) => { e.preventDefault(); void handleClaim(); }}
          >
            <div>
              <h2 className="text-2xl font-bold tracking-tight" style={S.heading}>
                {importId ? t('start.review_title', 'Here’s your storefront') : t('admin.create_storefront', 'Create your storefront')}
              </h2>
              <p className="mt-2 text-sm leading-relaxed" style={S.helper}>
                {importId
                  ? t('start.review_desc', 'We pre-filled these from your menu. Edit anything, then claim it — nothing is public until you publish.')
                  : t('admin.create_storefront_desc', "Three details to start. You'll add your menu and go live on the next screen — nothing is public until you publish.")}
              </p>
            </div>

            {importId && preview && (
              <div className="dz-fade-in flex items-start gap-3 p-3" style={{ background: 'var(--brand-surface-raised)', borderRadius: 'var(--brand-radius)', border: '1px solid var(--brand-border)' }} data-testid="menu-preview">
                <i className="ti ti-checks shrink-0 mt-0.5 text-lg" style={{ color: 'var(--brand-primary)' }} aria-hidden="true" />
                <div className="min-w-0">
                  <div className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>
                    {t('start.found_items', '{count} items across {cats} categories')
                      .replace('{count}', String(preview.productCount))
                      .replace('{cats}', String(preview.categories))}
                  </div>
                  {preview.productNames.length > 0 && (
                    <div className="mt-1 text-sm truncate" style={S.muted}>{preview.productNames.join(' · ')}{preview.productCount > preview.productNames.length ? ' …' : ''}</div>
                  )}
                </div>
              </div>
            )}

            <FormField label={t('admin.restaurant_name', 'Restaurant name')}>
              <Input value={name} onChange={e => handleNameChange((e.target as HTMLInputElement).value)} placeholder={t('admin.restaurant_name_ph', 'e.g. Pizza Roma')} />
            </FormField>

            <FormField label={t('admin.phone_fallback', 'Phone (fallback for customers)')}>
              <Input value={phone} onChange={e => setPhone((e.target as HTMLInputElement).value)} placeholder="+355 69 XXX XXXX" pattern={PHONE_E164_PATTERN} title={t('admin.phone_pattern_hint', '+355 followed by 7–14 digits')} />
            </FormField>

            <FormField label={t('admin.your_link', 'Your link')}>
              <div className="flex items-center gap-2 min-w-0">
                <div className="min-w-0 flex-1"><Input value={slug} onChange={e => handleSlugChange((e.target as HTMLInputElement).value)} placeholder="pizza-roma" /></div>
                <span className="text-sm whitespace-nowrap" style={S.muted}>.dowiz.org</span>
              </div>
              {slugError && (
                <p className="dz-fade-in text-xs mt-1.5 flex items-center gap-1" role="alert" style={{ color: 'var(--color-danger)' }}>
                  <i className="ti ti-alert-circle" aria-hidden="true" />{slugError}
                </p>
              )}
            </FormField>

            <Button type="submit" disabled={!canSubmit || tgWaiting} isLoading={phase === 'submitting'} className="w-full" size="lg" data-testid="claim-cta">
              {claimLabel} {mode === 'anonymous' && !tgWaiting ? '' : '→'}
            </Button>

            {mode === 'anonymous' && tgWaiting && tgLink && (
              <p className="dz-fade-in text-xs text-center" style={S.muted}>
                {t('admin.tg_open_hint', "Didn't open?")}{' '}
                <a href={tgLink} target="_blank" rel="noreferrer" className="underline underline-offset-2 rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2" style={{ color: 'var(--brand-primary)' }}>{t('admin.tg_open_link', 'Open Telegram')}</a>
              </p>
            )}

            {importId && (
              <button type="button" className="text-sm underline underline-offset-2 w-full text-center rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 motion-safe:transition-colors duration-150" style={S.muted} onClick={() => { setImportId(null); setPreview(null); setError(''); setPhase('choose'); }}>
                {t('start.different_file', 'Upload a different file')}
              </button>
            )}
          </form>
        )}
      </div>
    </div>
  );
}

// Parsing state — a simple, calm "reading your menu" moment: a spinner and a
// short reassurance line. Reduced-motion → the spinner ring is static.
function ParsingState() {
  const { t } = useI18n();
  return (
    <div className="dz-fade-in flex flex-col items-center gap-4 py-16 text-center" data-testid="menu-parsing">
      <span className="dz-spinner" aria-hidden="true" />
      <div>
        <div className="font-semibold" style={{ color: 'var(--brand-text)' }}>{t('start.reading', 'Reading your menu…')}</div>
        <div className="mt-1 text-sm" style={{ color: 'var(--brand-text-muted)' }}>{t('start.reading_store', 'Setting up your storefront…')}</div>
      </div>
      <span className="sr-only" role="status">{t('start.reading', 'Reading your menu…')}</span>
    </div>
  );
}

const ONBOARD_CSS = `
@media (prefers-reduced-motion: no-preference){
  .dz-fade-in{animation:dzFadeIn var(--motion-base,240ms) var(--ease-out,cubic-bezier(.16,1,.3,1)) both}
  .dz-spinner{animation:dzSpin .8s linear infinite}
}
@keyframes dzFadeIn{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
.dz-dropzone:hover{border-color:var(--brand-primary);box-shadow:var(--elev-2)}
.dz-dropzone-active{box-shadow:var(--elev-2)}
.dz-spinner{display:inline-block;width:36px;height:36px;border-radius:9999px;
  border:3px solid var(--brand-border);border-top-color:var(--brand-primary)}
@keyframes dzSpin{to{transform:rotate(360deg)}}
`;

// Public front door (unauthenticated): anonymous parse → claim with Telegram.
export function StartPage() {
  return <MenuFirstOnboarding mode="anonymous" />;
}
