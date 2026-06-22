import { safeStorage } from '../lib/safeStorage.js';
import React, { useCallback, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Input, FormField, useI18n } from '@deliveryos/ui';
import { PHONE_E164_PATTERN } from '@deliveryos/shared-types';
import { apiClient, ApiError } from '../lib/index.js';
import { SwanHero } from '../components/SwanHero.js';
import { AccessRequestGate } from '../components/AccessRequestForm.js';

// Menu-first onboarding. The front door is "upload your menu" — we parse it with
// the zero-dependency heuristic parser, pre-fill the storefront identity
// (name·phone·slug) and show the items we found, then the owner claims it.
//   mode="anonymous" (public /start): claim = authenticate with Telegram, then
//     POST /owner/onboarding/start with the stashed import id → location seeded.
//   mode="authed" (/admin/onboarding, already signed in): no Telegram step —
//     the same upload+review, then create+seed directly.
// A "start without a menu" path preserves the manual create flow.

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
    card: { background: 'var(--brand-surface)', border: '1px solid var(--brand-border)', borderRadius: 'var(--brand-radius)', padding: '20px' },
    heading: { fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' },
    muted: { color: 'var(--brand-text-muted)', fontSize: '13px' },
  };

  const claimLabel = mode === 'anonymous'
    ? (tgWaiting ? t('admin.tg_waiting', 'Waiting for Telegram…') : t('start.claim_telegram', 'Claim with Telegram'))
    : t('admin.create_continue', 'Create & continue');

  return (
    <div className="min-h-screen bg-[var(--brand-bg)] text-[var(--brand-text)]">
      <div className="max-w-lg mx-auto p-4 md:p-8">
        <input
          ref={fileRef}
          type="file"
          accept="application/pdf,image/*"
          className="hidden"
          data-testid="menu-file-input"
          onChange={(e) => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) void onFile(f); }}
        />

        {error && (
          <div role="alert" aria-live="polite" className="mb-4 p-3 text-sm rounded-lg" style={{ background: 'var(--status-cancelled-light)', border: '1px solid var(--status-cancelled-border)', color: 'var(--color-danger)' }}>
            {error}
          </div>
        )}

        {/* ── CHOOSE: upload your menu ── */}
        {phase === 'choose' && (
          <div className="space-y-5">
            {/* Soft access gate (ADR-soft-access-gate). Renders only when the build flag
                is on (default off) — public "register interest" capture on the landing. */}
            {mode === 'anonymous' && <AccessRequestGate />}
            {mode === 'anonymous'
              ? <SwanHero />
              : (
                <div>
                  <h2 className="text-xl font-bold" style={S.heading}>{t('start.title', 'Start with your menu')}</h2>
                  <p className="mt-1" style={S.muted}>{t('start.subtitle', 'Upload a PDF or photo of your menu. We’ll read it, set up your storefront, and bring your items to life — review everything before anything goes public.')}</p>
                </div>
              )}
            <div style={S.card} className="space-y-5">
            <button
              type="button"
              onClick={onPickFile}
              data-testid="upload-menu-cta"
              className="w-full rounded-lg py-10 px-4 text-center transition-all active:scale-[0.99]"
              style={{ border: '2px dashed var(--brand-border)', background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}
            >
              <i className="ti ti-file-upload text-3xl" />
              <div className="mt-2 font-medium">{t('start.upload_cta', 'Upload your menu')}</div>
              <div style={S.muted}>{t('start.upload_hint', 'PDF or photo · up to 10MB')}</div>
            </button>
            <div className="flex items-center justify-between pt-1">
              <button type="button" className="text-sm underline" style={S.muted}
                onClick={() => (mode === 'anonymous' ? navigate('/login') : setPhase('blank'))}>
                {t('start.no_menu', 'Start without a menu')}
              </button>
              {mode === 'anonymous' && (
                <button type="button" className="text-sm underline" style={S.muted} onClick={() => navigate('/login')}>
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
            className="space-y-4"
            onSubmit={(e) => { e.preventDefault(); void handleClaim(); }}
          >
            <div>
              <h2 className="text-xl font-bold" style={S.heading}>
                {importId ? t('start.review_title', 'Here’s your storefront') : t('admin.create_storefront', 'Create your storefront')}
              </h2>
              <p className="mt-1" style={S.muted}>
                {importId
                  ? t('start.review_desc', 'We pre-filled these from your menu. Edit anything, then claim it — nothing is public until you publish.')
                  : t('admin.create_storefront_desc', "Three details to start. You'll add your menu and go live on the next screen — nothing is public until you publish.")}
              </p>
            </div>

            {importId && preview && (
              <div className="rounded-lg p-3" style={{ background: 'var(--brand-surface-raised)', border: '1px solid var(--brand-border)' }} data-testid="menu-preview">
                <div className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>
                  {t('start.found_items', '{count} items across {cats} categories')
                    .replace('{count}', String(preview.productCount))
                    .replace('{cats}', String(preview.categories))}
                </div>
                {preview.productNames.length > 0 && (
                  <div className="mt-1" style={S.muted}>{preview.productNames.join(' · ')}{preview.productCount > preview.productNames.length ? ' …' : ''}</div>
                )}
              </div>
            )}

            <FormField label={t('admin.restaurant_name', 'Restaurant name')}>
              <Input value={name} onChange={e => handleNameChange((e.target as HTMLInputElement).value)} placeholder="e.g. Pizza Roma" />
            </FormField>

            <FormField label={t('admin.phone_fallback', 'Phone (fallback for customers)')}>
              <Input value={phone} onChange={e => setPhone((e.target as HTMLInputElement).value)} placeholder="+355 69 XXX XXXX" pattern={PHONE_E164_PATTERN} title="+355 followed by 7-14 digits" />
            </FormField>

            <FormField label={t('admin.your_link', 'Your link')}>
              <div className="flex items-center gap-2">
                <Input value={slug} onChange={e => handleSlugChange((e.target as HTMLInputElement).value)} placeholder="pizza-roma" />
                <span className="text-sm whitespace-nowrap" style={S.muted}>.dowiz.org</span>
              </div>
              {slugError && <p className="text-xs mt-1" style={{ color: 'var(--color-danger)' }}>{slugError}</p>}
            </FormField>

            <Button type="submit" disabled={!canSubmit || tgWaiting} isLoading={phase === 'submitting'} className="w-full" size="lg" data-testid="claim-cta">
              {claimLabel} {mode === 'anonymous' && !tgWaiting ? '' : '→'}
            </Button>

            {mode === 'anonymous' && tgWaiting && tgLink && (
              <p className="text-xs text-center" style={S.muted}>
                {t('admin.tg_open_hint', "Didn't open?")}{' '}
                <a href={tgLink} target="_blank" rel="noreferrer" style={{ color: 'var(--brand-primary)' }}>{t('admin.tg_open_link', 'Open Telegram')}</a>
              </p>
            )}

            {importId && (
              <button type="button" className="text-sm underline w-full text-center" style={S.muted} onClick={() => { setImportId(null); setPreview(null); setPhase('choose'); }}>
                {t('start.different_file', 'Upload a different file')}
              </button>
            )}
          </form>
        )}
      </div>
    </div>
  );
}

// Parsing state — "reading your menu" as a moment, not a dead spinner. An
// on-brand document with a sweeping scan-line + cycling reassurance copy.
// Reduced-motion → a still document and a single stable line. Default theme.
function ParsingState() {
  const { t } = useI18n();
  return (
    <div className="dz-parse" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)', borderRadius: 'var(--brand-radius)' }} data-testid="menu-parsing">
      <style>{PARSE_CSS}</style>
      <div className="dz-parse-stage" aria-hidden="true">
        <div className="dz-parse-doc">
          {['70%', '90%', '52%', '82%', '40%'].map((w, i) => (
            <span key={i} className="dz-parse-row" style={{ width: w }} />
          ))}
          <div className="dz-parse-scan" />
        </div>
      </div>
      <div className="dz-parse-copy" aria-hidden="true">
        <span className="dz-parse-line">{t('start.reading', 'Reading your menu…')}</span>
        <span className="dz-parse-line l2">{t('start.reading_dishes', 'Finding your dishes…')}</span>
        <span className="dz-parse-line l3">{t('start.reading_store', 'Setting up your storefront…')}</span>
      </div>
      <span className="sr-only" role="status">{t('start.reading', 'Reading your menu…')}</span>
    </div>
  );
}

const PARSE_CSS = `
.dz-parse{padding:34px 20px;display:flex;flex-direction:column;align-items:center;gap:20px}
.dz-parse-stage{position:relative}
.dz-parse-doc{position:relative;width:124px;height:148px;border-radius:12px;overflow:hidden;
  background:linear-gradient(180deg, color-mix(in srgb, var(--brand-primary) 8%, var(--brand-surface-raised)), var(--brand-surface-raised));
  border:1px solid var(--brand-border);display:flex;flex-direction:column;gap:9px;padding:16px 14px;
  box-shadow:0 12px 32px color-mix(in srgb, var(--brand-primary) 13%, transparent)}
.dz-parse-row{height:7px;border-radius:4px;background:color-mix(in srgb, var(--brand-text) 16%, transparent)}
.dz-parse-scan{position:absolute;left:0;right:0;height:34px;top:-34px;
  background:linear-gradient(180deg, transparent, color-mix(in srgb, var(--brand-primary) 40%, transparent), transparent);
  box-shadow:0 0 16px color-mix(in srgb, var(--brand-primary) 55%, transparent)}
.dz-parse-copy{position:relative;height:22px;min-width:230px;text-align:center}
.dz-parse-line{position:absolute;left:0;right:0;font-size:14px;font-weight:600;color:var(--brand-text);opacity:0}
@media (prefers-reduced-motion: no-preference){
  .dz-parse-scan{animation:dzScan 2.1s ease-in-out infinite}
  .dz-parse-row{animation:dzRowPulse 2.1s ease-in-out infinite}
  .dz-parse-line{animation:dzCycle 6s ease-in-out infinite}
  .dz-parse-line.l2{animation-delay:2s}
  .dz-parse-line.l3{animation-delay:4s}
}
@media (prefers-reduced-motion: reduce){
  .dz-parse-copy{height:auto}
  .dz-parse-line{opacity:1;position:static}
  .dz-parse-line.l2,.dz-parse-line.l3{display:none}
}
@keyframes dzScan{0%{top:-34px}55%{top:148px}100%{top:148px}}
@keyframes dzRowPulse{0%,100%{opacity:.5}50%{opacity:.95}}
@keyframes dzCycle{0%{opacity:0;transform:translateY(6px)}8%{opacity:1;transform:none}28%{opacity:1;transform:none}36%{opacity:0;transform:translateY(-6px)}100%{opacity:0}}
`;

// Public front door (unauthenticated): anonymous parse → claim with Telegram.
export function StartPage() {
  return <MenuFirstOnboarding mode="anonymous" />;
}
