import { safeStorage } from '../lib/safeStorage.js';
import React, { useCallback, useRef, useState, Suspense } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Input, FormField, useI18n, ArtNouveauFrame, ArtNouveauDivider, NomadicCredit, NomadicScene } from '@deliveryos/ui';
import { PHONE_E164_PATTERN } from '@deliveryos/shared-types';
import { apiClient, ApiError } from '../lib/index.js';
import { AccessRequestGate } from '../components/AccessRequestForm.js';
import { DeliverySwan } from '../components/DeliverySwan.js';

// Path A (Nomadic-Tribe redesign): the anonymous hero is a real-time 3D
// paper/Moebius canvas. React.lazy keeps three.js in its own chunk, off the
// main bundle. SwanHero is the Suspense fallback (during load) AND the runtime
// fallback PaperScene swaps to for reduced-motion / no-WebGL / SSR / any error.
const PaperScene = React.lazy(() => import('../components/PaperScene.js'));

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

  // Drag-and-drop onto the dropzone (the brief calls it a dropzone, so honour it).
  // dragover must preventDefault for a drop to fire; we mirror the click path.
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
    // Inked-outline "drawn on paper" card: a soft paper surface fill with a
    // 1.75px ink contour (not the soft default border), keeping --brand-radius.
    card: {
      background: 'var(--paper-surface, var(--brand-surface))',
      borderRadius: 'var(--brand-radius)',
      border: '1.75px solid var(--ink, var(--brand-text))',
      boxShadow: 'var(--elev-1)',
      padding: 'var(--space-5, 1.25rem)',
    },
    // Display face (Yeseva One, Art-Nouveau) for the big hero/section titles,
    // with generous tracking/leading for an elegant display feel.
    heading: {
      fontFamily: 'var(--font-display, var(--brand-font-heading))',
      color: 'var(--brand-text)',
      letterSpacing: '-0.01em',
      lineHeight: 1.12,
    },
    // Body/helper copy uses --brand-text (not -muted) so it clears 4.5:1.
    helper: { color: 'var(--brand-text)' },
    muted: { color: 'var(--brand-text-muted)' },
  } as const;

  const claimLabel = mode === 'anonymous'
    ? (tgWaiting ? t('admin.tg_waiting', 'Waiting for Telegram…') : t('start.claim_telegram', 'Claim with Telegram'))
    : t('admin.create_continue', 'Create & continue');

  return (
    // dowiz-owned page → always wears the Paper/Nomadic (Moebius) identity, independent of the
    // global paper-skin flag. data-skin="paper" remaps --brand-* to the cream/ink palette + grain.
    <div data-skin="paper" className="min-h-screen bg-[var(--brand-bg)] text-[var(--brand-text)]">
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
          <div className="space-y-6">
            {/* Soft access gate (ADR-soft-access-gate). Renders only when the build flag
                is on (default off) — public "register interest" capture on the landing. */}
            {mode === 'anonymous' && <AccessRequestGate />}
            {mode === 'anonymous'
              ? (
                // Comic-panel hero: the live paper scene framed in an inked Art-Nouveau
                // border with corner flourishes. The frame is decorative (aria-hidden in
                // the component); PaperScene wiring is untouched.
                <div>
                  {/* Hero stage: the live paper panel with the delivery swan gliding
                      ON TOP (pointer-events:none so it never blocks the canvas). */}
                  <div className="dz-stage dz-stage-1" style={{ position: 'relative' }}>
                    <ArtNouveauFrame
                      style={{ background: 'var(--paper-surface, var(--brand-surface))', borderRadius: 'var(--brand-radius)', overflow: 'hidden', boxShadow: 'var(--elev-2)', padding: 6 }}
                    >
                      {/* Hero art: live 3D paper scene; falls back to the Moebius SVG scene
                          (on-brand, art-only) for reduced-motion / no-WebGL / lazy-load. The
                          value-prop headline lives BELOW in the page so it shows either way. */}
                      <Suspense fallback={<NomadicScene variant="journey" animated />}>
                        <PaperScene fallback={<NomadicScene variant="journey" animated />} />
                      </Suspense>
                    </ArtNouveauFrame>
                    {/* Signature delivery swan — flies the parcel across the sky. */}
                    <DeliverySwan />
                  </div>
                  <h1 className="dz-headline mt-5 text-3xl leading-[1.1]" style={{ ...S.heading, letterSpacing: '-0.01em' }}>
                    {t('start.hero_title', 'Your menu, online tonight.').split(' ').map((w, i) => (
                      <React.Fragment key={i}>
                        <span className="dz-word" style={{ '--dz-w': i } as React.CSSProperties}>{w}</span>{' '}
                      </React.Fragment>
                    ))}
                  </h1>
                  <p className="dz-stage dz-stage-3 mt-2 text-sm leading-relaxed" style={S.helper}>
                    {t('start.hero_sub', 'Snap a photo of your menu — we read it, build your storefront, and you’re taking orders. No code, no wait.')}
                  </p>
                </div>
              )
              : (
                <div>
                  <h2 className="text-3xl font-bold" style={S.heading}>{t('start.title', 'Start with your menu')}</h2>
                  <p className="mt-2 text-sm leading-relaxed" style={S.helper}>{t('start.subtitle', 'Upload a PDF or photo of your menu. We’ll read it, set up your storefront, and bring your items to life — review everything before anything goes public.')}</p>
                </div>
              )}

            {/* Ornamental divider carrying the Art-Nouveau line into the chrome. */}
            <ArtNouveauDivider className={mode === 'anonymous' ? 'dz-stage dz-stage-4' : 'dz-fade-in'} style={{ maxWidth: 220, margin: '0.25rem auto' }} />

            {/* Three inked feature glyphs in the limited palette (upload / AI / online). */}
            <FeatureGlyphs staged={mode === 'anonymous'} />

            <div style={S.card} className={`${mode === 'anonymous' ? 'dz-stage dz-stage-6' : 'dz-fade-in'} space-y-5`}>
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
                border: dragActive
                  ? '2px solid var(--brand-primary)'
                  : '2px dashed var(--ink-line, var(--brand-border))',
                borderRadius: 'var(--brand-radius)',
                background: dragActive ? 'var(--paper-surface, var(--brand-surface))' : 'var(--paper-raised, var(--brand-surface-raised))',
                color: 'var(--brand-text)',
              }}
            >
              <i className={`ti ${dragActive ? 'ti-file-download' : 'ti-file-upload'} text-3xl motion-safe:transition-transform duration-150 ease-[var(--ease-soft)] group-hover:-translate-y-0.5`} style={{ color: 'var(--teal-deep, var(--brand-primary))' }} aria-hidden="true" />
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
              <h2 className="text-3xl font-bold" style={S.heading}>
                {importId ? t('start.review_title', 'Here’s your storefront') : t('admin.create_storefront', 'Create your storefront')}
              </h2>
              <ArtNouveauDivider className="mt-3" style={{ maxWidth: 160, marginLeft: 0 }} />
              <p className="mt-3 text-sm leading-relaxed" style={S.helper}>
                {importId
                  ? t('start.review_desc', 'We pre-filled these from your menu. Edit anything, then claim it — nothing is public until you publish.')
                  : t('admin.create_storefront_desc', "Three details to start. You'll add your menu and go live on the next screen — nothing is public until you publish.")}
              </p>
            </div>

            {importId && preview && (
              <div className="dz-fade-in flex items-start gap-3 p-3" style={{ background: 'var(--paper-raised, var(--brand-surface-raised))', borderRadius: 'var(--brand-radius)', border: '1.5px solid var(--ink-line, var(--brand-border))' }} data-testid="menu-preview">
                <i className="ti ti-checks shrink-0 mt-0.5 text-lg" style={{ color: 'var(--teal-deep, var(--brand-primary))' }} aria-hidden="true" />
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
              <Input value={name} onChange={e => handleNameChange((e.target as HTMLInputElement).value)} placeholder="e.g. Pizza Roma" />
            </FormField>

            <FormField label={t('admin.phone_fallback', 'Phone (fallback for customers)')}>
              <Input value={phone} onChange={e => setPhone((e.target as HTMLInputElement).value)} placeholder="+355 69 XXX XXXX" pattern={PHONE_E164_PATTERN} title="+355 followed by 7-14 digits" />
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

        {/* Homage attribution (Nomadic Tribe / makemepulse) — paper-margin footer. */}
        <footer className="mt-10 pt-2">
          <ArtNouveauDivider className="dz-fade-in" style={{ maxWidth: 180, margin: '0 auto 0.75rem' }} />
          <NomadicCredit />
        </footer>
      </div>
    </div>
  );
}

// Three feature glyphs in the limited paper palette (sand / teal / gold on ink),
// rendered as small inked icons — the "drawn on paper" how-it-works trio. Purely
// decorative iconography (aria-hidden); the visible labels carry the meaning.
function FeatureGlyphs({ staged = false }: { staged?: boolean }) {
  const { t } = useI18n();
  const items: { icon: string; tint: string; label: string }[] = [
    { icon: 'ti ti-file-upload', tint: 'var(--sand, #987654)', label: t('start.feat_upload', 'Upload your menu') },
    { icon: 'ti ti-sparkles', tint: 'var(--teal-deep, #3EA094)', label: t('start.feat_ai', 'We read it for you') },
    { icon: 'ti ti-bolt', tint: 'var(--gold, #ECD06F)', label: t('start.feat_online', 'Go online fast') },
  ];
  return (
    <ul className={`${staged ? 'dz-stage dz-stage-5' : 'dz-fade-in'} grid grid-cols-3 gap-3 text-center`} aria-label={t('start.how_it_works', 'How it works')}>
      {items.map((it, i) => (
        <li key={it.label} className="dz-glyph flex flex-col items-center gap-2 min-w-0" style={staged ? ({ '--dz-i': i } as React.CSSProperties) : undefined}>
          <span
            className="inline-flex items-center justify-center"
            aria-hidden="true"
            style={{
              width: 44, height: 44, borderRadius: 'var(--brand-radius)',
              border: '1.75px solid var(--ink, #241F1A)',
              background: 'var(--paper-raised, var(--brand-surface-raised))',
              color: 'var(--ink, #241F1A)',
            }}
          >
            <i className={`${it.icon} text-xl`} style={{ color: it.tint }} />
          </span>
          <span className="text-xs leading-tight" style={{ color: 'var(--brand-text)' }}>{it.label}</span>
        </li>
      ))}
    </ul>
  );
}

// Parsing state — "reading your menu" as a moment, not a dead spinner. An
// on-brand document with a sweeping scan-line + cycling reassurance copy.
// Reduced-motion → a still document and a single stable line. Default theme.
function ParsingState() {
  const { t } = useI18n();
  return (
    <div className="dz-parse dz-fade-in" style={{ background: 'var(--brand-surface)', borderRadius: 'var(--brand-radius)', boxShadow: 'var(--elev-2)' }} data-testid="menu-parsing">
      <style>{ONBOARD_CSS}</style>
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

const ONBOARD_CSS = `
@media (prefers-reduced-motion: no-preference){
  .dz-fade-in{animation:dzFadeIn var(--motion-base,240ms) var(--ease-out,cubic-bezier(.16,1,.3,1)) both}

  /* First-paint entrance choreography (anonymous hero). Beats arrive in sequence
     ~1.5–2.2s total: panel → headline → (swan flies in, owned by DeliverySwan)
     → divider → glyphs (per-item stagger) → upload card. Each element's own
     reveal stays snappy (≤320ms ease-out); the cascade comes from delays, not
     slow transitions. Runs once on mount (these nodes only exist in 'choose'). */
  .dz-stage{opacity:0;transform:translateY(10px) scale(.985);
    animation:dzStageRise 320ms var(--ease-out,cubic-bezier(.16,1,.3,1)) both}
  .dz-stage-1{animation-delay:0ms}
  .dz-stage-2{animation-delay:140ms}
  .dz-stage-3{animation-delay:260ms}
  .dz-stage-4{animation-delay:1340ms}
  .dz-stage-5{animation-delay:1460ms}
  .dz-stage-6{animation-delay:1640ms}
  /* glyphs cascade within their own beat (stage-5) */
  .dz-stage-5 .dz-glyph{opacity:0;transform:translateY(8px);
    animation:dzStageRise 300ms var(--ease-out,cubic-bezier(.16,1,.3,1)) both;
    animation-delay:calc(1460ms + var(--dz-i,0) * 70ms)}
  /* headline reveals word-by-word with an ink-bleed: each word arrives soft and
     blurred, then sharpens as if the ink were settling into the paper. */
  .dz-word{opacity:0;
    animation:dzInkBleed 520ms var(--ease-out,cubic-bezier(.16,1,.3,1)) both;
    animation-delay:calc(160ms + var(--dz-w,0) * 80ms)}
}
/* default (reduced-motion-safe): words visible, inline so they wrap naturally */
.dz-word{display:inline-block}
@keyframes dzFadeIn{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
@keyframes dzStageRise{to{opacity:1;transform:none}}
@keyframes dzInkBleed{
  0%{opacity:0;filter:blur(7px);transform:translateY(0.42em) scale(.98)}
  55%{opacity:1}
  100%{opacity:1;filter:blur(0);transform:none}
}
.dz-dropzone:hover{border-color:var(--brand-primary);box-shadow:var(--elev-2)}
/* drag-over: the dropzone lifts (border colour/style cue is inline-driven so it
   beats the inline base border). Reduced-motion keeps only the colour cue. */
.dz-dropzone-active{box-shadow:var(--elev-2)}
@media (prefers-reduced-motion: no-preference){
  .dz-dropzone-active{transform:translateY(-2px) scale(1.012)}
  .dz-dropzone-active .ti{transform:translateY(2px)}
}
`;

const PARSE_CSS = `
.dz-parse{padding:34px 20px;display:flex;flex-direction:column;align-items:center;gap:20px}
.dz-parse-stage{position:relative}
.dz-parse-doc{position:relative;width:124px;height:148px;border-radius:var(--brand-radius);overflow:hidden;
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
