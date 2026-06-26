import React, { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { Button, Input, ColorInput, FormField, SkeletonBase, useI18n, useToast, ease, duration, contrastRatio, parseColor } from '@deliveryos/ui';
import type { ThemeConfig } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

export function BrandingPage() {
  const { t } = useI18n();
  const { showToast } = useToast();
  const reduce = useReducedMotion();
  const previewRef = useRef<HTMLIFrameElement>(null);
  const [initLoading, setInitLoading] = useState(true);
  const [previewLoaded, setPreviewLoaded] = useState(false);
  const [config, setConfig] = useState<ThemeConfig>({
    primary: 'var(--brand-primary)',
    primaryHover: 'var(--brand-primary-hover)',
    primaryLight: 'var(--brand-primary-light)',
    bg: 'var(--brand-bg)',
    surface: 'var(--brand-surface)',
    surfaceRaised: 'var(--brand-surface-raised)',
    border: 'var(--brand-border)',
    text: 'var(--brand-text)',
    textMuted: 'var(--brand-text-muted)',
    accent: 'var(--brand-accent)',
  });
  const [logoUrl, setLogoUrl] = useState('');
  const [logoDataUrl, setLogoDataUrl] = useState('');
  const [logoUploading, setLogoUploading] = useState(false);
  const [loading, setLoading] = useState(false);
  const [slug, setSlug] = useState('');
  const [locationId, setLocationId] = useState('');
  const [googleRating, setGoogleRating] = useState('');
  const [googleReviewCount, setGoogleReviewCount] = useState('');
  const [googleMapsUrl, setGoogleMapsUrl] = useState('');
  const [googlePlaceId, setGooglePlaceId] = useState('');
  const [socialInstagram, setSocialInstagram] = useState('');
  const [socialFacebook, setSocialFacebook] = useState('');
  const [website, setWebsite] = useState('');
  const [generating, setGenerating] = useState(false);

  // A 'var(--…)' colour means "unset" — strip it so we never store/preview the
  // literal token string in place of a real hex value.
  const concrete = (v: string) => (v && !v.startsWith('var(') ? v : undefined);

  useEffect(() => {
    // Untyped read: the strict ThemeResponse contract (shared-types) doesn't yet
    // include the UX-1 storefront-link fields, so parse leniently here.
    apiClient<any>('/owner/brand').then((res: any) => {
      if (res.primaryColor) setConfig(prev => ({ ...prev, primary: res.primaryColor }));
      if (res.bgColor) setConfig(prev => ({ ...prev, bg: res.bgColor }));
      if (res.textColor) setConfig(prev => ({ ...prev, text: res.textColor }));
      if (res.logoUrl) setLogoUrl(res.logoUrl);
      if (res.locationId) setLocationId(res.locationId);
      if (res.googleRating != null) setGoogleRating(String(res.googleRating));
      if (res.googleReviewCount != null) setGoogleReviewCount(String(res.googleReviewCount));
      if (res.googleMapsUrl) setGoogleMapsUrl(res.googleMapsUrl);
      if (res.googlePlaceId) setGooglePlaceId(res.googlePlaceId);
      if (res.socialInstagram) setSocialInstagram(res.socialInstagram);
      if (res.socialFacebook) setSocialFacebook(res.socialFacebook);
    }).catch(() => {}).finally(() => setInitLoading(false));
    apiClient<any>('/owner/settings').then((res: any) => {
      if (res.slug) {
        setSlug(res.slug);
      } else if (res.locationName || res.name) {
        const generated = (res.locationName || res.name).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 50);
        setSlug(generated);
      }
    }).catch(() => {});
  }, []);

  const handleLogoUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (file.size > 2 * 1024 * 1024) { alert(t('admin.error_file_too_large', 'File too large. Max 2MB.')); return; }
    // Show preview immediately via data URL
    const reader = new FileReader();
    reader.onload = () => setLogoDataUrl(reader.result as string);
    reader.readAsDataURL(file);
    // Upload via multipart to persist the logo
    if (!locationId) return;
    setLogoUploading(true);
    try {
      const form = new FormData();
      form.append('logo', file);
      // Via apiClient: transparent 401→refresh→retry + error mapping (was a raw
      // fetch with a hand-rolled Bearer header that silently failed on token expiry).
      const data = await apiClient<any>(`/owner/locations/${locationId}/theme/logo`, { method: 'POST', body: form });
      if (data?.logo_url) { setLogoUrl(data.logo_url); setLogoDataUrl(''); }
    } catch {
      // Preview still shows; logo not persisted until next save
    } finally {
      setLogoUploading(false);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    try {
      await apiClient('/owner/brand', {
        method: 'PUT',
        body: {
          // Persist only concrete colours — a 'var(--…)' value means "unset" and
          // must not be written as a literal (it would poison the stored theme).
          primaryColor: concrete(config.primary) || null,
          bgColor: concrete(config.bg) || null,
          textColor: concrete(config.text) || null,
          logoUrl: logoUrl || null,
          googleRating: googleRating ? parseFloat(googleRating) : null,
          googleReviewCount: googleReviewCount ? parseInt(googleReviewCount, 10) : null,
          googleMapsUrl: googleMapsUrl || null,
          googlePlaceId: googlePlaceId || null,
          socialInstagram: socialInstagram || null,
          socialFacebook: socialFacebook || null,
        }
      });
      showToast(t('common.saved', 'Branding saved'), 'success');
    } catch (e) {
      console.error('[BrandingPage] Failed to save branding:', e);
      showToast(t('common.error', 'Failed to save branding'), 'error');
    } finally { setLoading(false); }
  };

  // Auto-generate brand seed colours from an existing website and/or the logo.
  // Returns suggestions; they flow into the live preview for review before Save.
  const handleGenerate = async () => {
    if (!website.trim() && !logoDataUrl && !logoUrl) {
      showToast(t('admin.brand_need_source', 'Add a website URL or upload a logo first'), 'error');
      return;
    }
    setGenerating(true);
    try {
      const res = await apiClient<any>('/owner/brand/generate', {
        method: 'POST',
        body: { website: website.trim() || undefined, logoDataUrl: logoDataUrl || undefined },
      });
      let applied = false;
      setConfig(prev => {
        const next = { ...prev };
        if (res.primaryColor) { next.primary = res.primaryColor; applied = true; }
        if (res.bgColor) { next.bg = res.bgColor; applied = true; }
        if (res.textColor) { next.text = res.textColor; applied = true; }
        return next;
      });
      if (res.logoUrl) setLogoUrl(res.logoUrl);
      showToast(
        applied ? t('admin.brand_generated', 'Brand colours detected — review and Save') : t('admin.brand_no_signal', 'No brand colours found'),
        applied ? 'success' : 'error',
      );
    } catch {
      showToast(t('admin.brand_generate_failed', 'Could not detect colours from that website/logo'), 'error');
    } finally { setGenerating(false); }
  };

  const logoPreview = logoDataUrl || logoUrl;
  // Track a broken/404 logo URL so we render a placeholder instead of a broken <img>.
  // Reset whenever the source changes so a fixed URL gets a fresh chance to load.
  const [logoBroken, setLogoBroken] = useState(false);
  useEffect(() => { setLogoBroken(false); }, [logoPreview]);

  // Push the concrete (non-token) colour payload to the live preview.
  const postTheme = useCallback((win: Window | null | undefined) => {
    if (!win) return;
    const primary = concrete(config.primary), bg = concrete(config.bg), text = concrete(config.text);
    if (!primary && !bg && !text) return; // nothing set yet → leave stored theme
    win.postMessage({ type: 'branding_preview_theme', primary, bg, text }, '*');
  }, [config.primary, config.bg, config.text]);

  // Listen for iframe ready ping, respond with current logo + theme.
  useEffect(() => {
    const handler = (e: MessageEvent) => {
      if (e.data?.type === 'branding_preview_ready' && previewRef.current?.contentWindow) {
        const url = logoDataUrl || logoUrl;
        if (url) {
          previewRef.current.contentWindow.postMessage({ type: 'branding_preview_logo', logoUrl: url }, '*');
        }
        postTheme(previewRef.current.contentWindow);
      }
    };
    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [logoDataUrl, logoUrl, postTheme]);

  // Push logo + theme to the iframe whenever they change — no reload, no flicker.
  useEffect(() => {
    if (logoPreview && previewRef.current?.contentWindow) {
      previewRef.current.contentWindow.postMessage({ type: 'branding_preview_logo', logoUrl: logoPreview }, '*');
    }
  }, [logoPreview]);
  useEffect(() => {
    postTheme(previewRef.current?.contentWindow);
  }, [postTheme]);

  // AA contrast guardrail: derivePalette auto-corrects body TEXT on the live storefront,
  // but the brand PRIMARY (the price colour) is never nudged — a pale primary on a pale
  // background ships an illegible price. Warn the owner BEFORE they save either failure.
  const contrastWarnings = useMemo(() => {
    const bg = parseColor(concrete(config.bg)), text = parseColor(concrete(config.text)), primary = parseColor(concrete(config.primary));
    const out: string[] = [];
    if (bg && text && contrastRatio(text, bg) < 4.5) {
      out.push(t('admin.contrast_text_warn', 'Text is hard to read on this background (below AA 4.5:1) — it will be auto-adjusted on your storefront.'));
    }
    if (bg && primary && contrastRatio(primary, bg) < 3) {
      out.push(t('admin.contrast_primary_warn', 'Your primary colour is low-contrast on this background — prices and buttons may be hard to see. Pick a bolder primary or a different background.'));
    }
    return out;
  }, [config.bg, config.text, config.primary, t]);

  // URL depends only on slug, so colour edits update via postMessage instead of
  // reloading the whole storefront (which reset scroll + flashed the slug name).
  const iframeUrl = useMemo(() => {
    if (!slug) return '';
    return `/branding-preview/${slug}?embed=true&draft=true`;
  }, [slug]);

  // Re-arm the preview skeleton whenever the iframe source changes.
  useEffect(() => { setPreviewLoaded(false); }, [iframeUrl]);

  // Shareable storefront URL with a copy button that confirms inline.
  const [copied, setCopied] = useState<string | null>(null);
  const UrlRow = ({ label, url, k }: { label: string; url: string; k: string }) => (
    <>
      <p className="text-xs text-center mt-3" style={{ color: 'var(--brand-text)' }}>{label}</p>
      <div className="flex items-center justify-center gap-2 mt-1 min-w-0">
        <a
          href={url}
          target="_blank"
          rel="noopener noreferrer"
          className="font-mono text-xs text-[var(--brand-primary)] hover:underline truncate min-w-0 rounded-[var(--brand-radius-sm)] transition-colors duration-150 ease-[var(--ease-soft)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-bg)]"
        >
          {url}
        </a>
        <button
          type="button"
          onClick={() => { navigator.clipboard.writeText(url); setCopied(k); setTimeout(() => setCopied(c => (c === k ? null : c)), 1500); }}
          className="shrink-0 inline-flex items-center gap-1 text-step-2xs text-[var(--brand-text-muted)] hover:text-[var(--brand-primary)] underline rounded-[var(--brand-radius-sm)] px-1 py-0.5 transition-colors duration-150 ease-[var(--ease-soft)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-bg)] active:scale-[0.97]"
          title={t('common.copy', 'Copy')}
        >
          <i className={`ti ${copied === k ? 'ti-check' : 'ti-copy'}`} aria-hidden="true" />
          {copied === k ? t('common.copied', 'Copied') : t('common.copy', 'Copy')}
        </button>
      </div>
    </>
  );

  // Soft-UI card shell — one radius/elevation/hover system across every section.
  // Entrance is a gentle staggered ease-out fade-rise (instant under reduced-motion).
  const cardCls =
    'bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4 ' +
    'transition-[box-shadow,transform] duration-150 ease-[var(--ease-soft)] ' +
    'shadow-[var(--elev-1)] hover:shadow-[var(--elev-2)] hover:-translate-y-0.5 motion-reduce:transform-none';
  const sectionMotion = (i: number) =>
    reduce
      ? {}
      : {
          initial: { opacity: 0, y: 8 },
          animate: { opacity: 1, y: 0 },
          transition: { duration: duration.base, delay: i * 0.05, ease: ease.out },
        };

  if (initLoading) {
    return (
      <div className="p-4 max-w-6xl mx-auto flex flex-col lg:flex-row gap-8">
        <div className="flex-1 space-y-6">
          <SkeletonBase className="h-8 w-48" />
          {[1, 2, 3, 4].map(i => <SkeletonBase key={i} className="h-40 w-full rounded-[var(--brand-radius)]" />)}
        </div>
        <div className="flex-1">
          <SkeletonBase className="h-6 w-40 mb-4" />
          <SkeletonBase className="h-[700px] w-full max-w-sm mx-auto rounded-[40px]" />
        </div>
      </div>
    );
  }

  return (
    <div className="p-4 max-w-6xl mx-auto flex flex-col lg:flex-row gap-8">
      <div className="flex-1 min-w-0 space-y-6">
        <h2 className="text-2xl font-bold border-b border-[var(--brand-border)] pb-4" style={{ fontFamily: 'var(--brand-font-heading)' }}>
          {t('admin.branding', 'Branding Settings')}
        </h2>
        <form onSubmit={handleSave} className="space-y-6">
          {/* Auto-brand: derive a coherent storefront theme from the venue's
              existing website and/or logo — a starting point the owner can tweak. */}
          <motion.div {...sectionMotion(0)} className={cardCls + ' !space-y-3'}>
            <div>
              <h3 className="font-semibold text-lg">{t('admin.auto_brand', 'Auto-generate from your brand')}</h3>
              <p className="text-xs text-[var(--brand-text-muted)] mt-1">{t('admin.auto_brand_hint', 'Paste your existing website and/or upload your logo — we’ll detect your colours and build a matching storefront. You can fine-tune below.')}</p>
            </div>
            <div className="flex flex-col sm:flex-row gap-2">
              <Input
                type="url"
                inputMode="url"
                placeholder={t('admin.website_placeholder', 'https://your-restaurant.com')}
                value={website}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setWebsite(e.target.value)}
                className="flex-1"
              />
              <Button type="button" onClick={handleGenerate} isLoading={generating} disabled={generating}>
                <i className="ti ti-wand mr-1.5" aria-hidden="true" />{t('admin.generate', 'Generate')}
              </Button>
            </div>
          </motion.div>
          <motion.div {...sectionMotion(1)} className={cardCls}>
            <h3 className="font-semibold text-lg">{t('admin.colors', 'Colors')}</h3>
            <ColorInput label={t('admin.primary_color', 'Primary Color')} value={config.primary} onChange={(c: string) => setConfig({ ...config, primary: c })} />
            <ColorInput label={t('admin.bg_color', 'Background Color')} value={config.bg} onChange={(c: string) => setConfig({ ...config, bg: c })} />
            <ColorInput label={t('admin.text_color', 'Text Color')} value={config.text} onChange={(c: string) => setConfig({ ...config, text: c })} />
            {contrastWarnings.length > 0 && (
              <div role="status" aria-live="polite" data-testid="branding-contrast-warning" className="rounded-[var(--brand-radius)] px-3 py-2 text-xs space-y-1" style={{ background: 'var(--status-pending-light, rgba(217,119,6,0.12))', border: '1px solid var(--color-warning)', color: 'var(--brand-text)' }}>
                {contrastWarnings.map((w, i) => (
                  <p key={i} className="flex items-start gap-1.5">
                    <i className="ti ti-alert-triangle shrink-0 mt-0.5" aria-hidden="true" style={{ color: 'var(--color-warning)' }} />
                    <span className="min-w-0">{w}</span>
                  </p>
                ))}
              </div>
            )}
          </motion.div>
          <motion.div {...sectionMotion(2)} className={cardCls}>
            <h3 className="font-semibold text-lg">{t('admin.logo', 'Logo')}</h3>
            <p className="text-xs text-[var(--brand-text-muted)]">{t('admin.logo_hint', 'Recommended: 512x512px PNG with transparent background. Max 2MB.')}</p>
            <div className="flex items-start gap-4">
              <div className="flex-1 min-w-0 space-y-3">
                <FormField label={t(logoUploading ? 'admin.uploading' : 'admin.upload_logo', logoUploading ? 'Uploading…' : 'Upload Logo File')}>
                  <input type="file" accept="image/png,image/jpeg,image/svg+xml" onChange={handleLogoUpload} disabled={logoUploading}
                    aria-label={t('admin.upload_logo', 'Upload Logo File')}
                    className="w-full text-sm rounded-[var(--brand-radius-sm)] transition-shadow duration-150 ease-[var(--ease-soft)]
                      file:mr-3 file:py-2 file:px-4 file:rounded-[var(--brand-radius-sm)] file:border-0 file:text-sm file:font-medium file:bg-[var(--brand-primary)] file:text-white file:transition-opacity file:duration-150 hover:file:opacity-90 disabled:opacity-50
                      focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]" />
                </FormField>
                <FormField label={t('admin.or_logo_url', '— or Logo URL —')}>
                  <Input value={logoUrl} onChange={e => setLogoUrl(e.target.value)} placeholder="https://cdn.example.com/logo.png" />
                </FormField>
              </div>
              <div className="shrink-0 w-20 h-20 rounded-[var(--brand-radius)] border border-[var(--brand-border)] overflow-hidden bg-[var(--brand-surface-raised)] flex items-center justify-center relative">
                {logoUploading && (
                  <div className="absolute inset-0 flex items-center justify-center bg-[var(--brand-surface)]/70 z-10" aria-live="polite">
                    <i className="ti ti-loader-2 animate-spin text-xl" style={{ color: 'var(--brand-primary)' }} aria-hidden="true" />
                    <span className="sr-only">{t('admin.uploading', 'Uploading…')}</span>
                  </div>
                )}
                {logoPreview && !logoBroken ? (
                  // onError → placeholder so a missing/invalid stored URL never shows a
                  // broken-image glyph or 404s the preview box.
                  <img
                    src={logoPreview}
                    alt={t('admin.logo_preview_alt', 'Logo preview')}
                    data-testid="logo-preview"
                    className="max-w-full max-h-full object-contain"
                    onError={() => setLogoBroken(true)}
                  />
                ) : (
                  <i className="ti ti-photo text-2xl" style={{ color: 'var(--brand-text-muted)' }} aria-hidden="true" />
                )}
              </div>
            </div>
          </motion.div>
          <motion.div {...sectionMotion(3)} className={cardCls}>
            <h3 className="font-semibold text-lg">{t('admin.google_info', 'Google Maps Info')}</h3>
            <p className="text-xs text-[var(--brand-text-muted)]">{t('admin.google_info_hint', 'Displayed on client menu page. Update periodically from your Google Maps listing.')}</p>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <FormField label={t('admin.google_rating', 'Rating (0–5)')}>
                <Input type="number" min="0" max="5" step="0.1" value={googleRating} onChange={e => setGoogleRating(e.target.value)} placeholder="4.8" />
              </FormField>
              <FormField label={t('admin.google_review_count', 'Review Count')}>
                <Input type="number" min="0" step="1" value={googleReviewCount} onChange={e => setGoogleReviewCount(e.target.value)} placeholder="124" />
              </FormField>
            </div>
            <FormField label={t('admin.google_maps_url', 'Google Maps URL')}>
              <Input value={googleMapsUrl} onChange={e => setGoogleMapsUrl(e.target.value)} placeholder="https://maps.app.goo.gl/..." />
            </FormField>
            <FormField label={t('admin.google_place_id', 'Google Place ID')}>
              <Input value={googlePlaceId} onChange={e => setGooglePlaceId(e.target.value)} placeholder="ChIJ..." />
              <p className="text-xs text-[var(--brand-text-muted)] mt-1">{t('admin.google_place_id_hint', 'Lets customers leave a Google review after delivery. Find it on your Google Business listing.')}</p>
            </FormField>
          </motion.div>
          <motion.div {...sectionMotion(4)} className={cardCls}>
            <h3 className="font-semibold text-lg">{t('admin.social_links', 'Social links')}</h3>
            <p className="text-xs text-[var(--brand-text-muted)]">{t('admin.social_links_hint', 'Shown as icons in your storefront footer. Leave blank to hide.')}</p>
            <FormField label={t('admin.instagram', 'Instagram')}>
              <Input value={socialInstagram} onChange={e => setSocialInstagram(e.target.value)} placeholder="https://instagram.com/yourplace" />
            </FormField>
            <FormField label={t('admin.facebook', 'Facebook')}>
              <Input value={socialFacebook} onChange={e => setSocialFacebook(e.target.value)} placeholder="https://facebook.com/yourplace" />
            </FormField>
          </motion.div>
          <div className="flex items-center gap-4">
            <Button type="submit" isLoading={loading} disabled={loading} size="lg">{t('common.save', 'Save Changes')}</Button>
          </div>
        </form>
      </div>

      {/* Live Preview — real client page in iframe */}
      <div className="flex-1 min-w-0 border-l border-[var(--brand-border)] pl-0 lg:pl-8">
        <h3 className="font-semibold text-lg mb-4 text-[var(--brand-text)]">{t('admin.live_preview', 'Live Preview (Client View)')}</h3>
        {iframeUrl ? (
          <div className="border border-[var(--brand-border)] rounded-[40px] overflow-hidden w-full max-w-sm mx-auto shadow-[var(--elev-4)] relative h-[700px]">
            <div className="absolute top-0 inset-x-0 h-6 bg-black flex justify-center items-center z-20">
              <div className="w-20 h-4 bg-[var(--brand-surface)] border border-[var(--brand-border)] border-t-0 rounded-b-xl" />
            </div>
            {/* Preview-loading skeleton — covers the iframe until first paint. */}
            {!previewLoaded && (
              <div className="absolute inset-0 z-10 flex flex-col gap-3 p-5 pt-10 bg-[var(--brand-surface)]" aria-hidden="true">
                <SkeletonBase className="h-32 w-full rounded-[var(--brand-radius)]" />
                <SkeletonBase className="h-5 w-2/3 rounded-md" />
                <SkeletonBase className="h-5 w-1/2 rounded-md" />
                <SkeletonBase className="h-20 w-full rounded-[var(--brand-radius)]" />
                <SkeletonBase className="h-20 w-full rounded-[var(--brand-radius)]" />
              </div>
            )}
            <iframe
              ref={previewRef}
              src={iframeUrl}
              className="w-full border-0"
              style={{ marginTop: 'var(--space-6)', height: 'calc(100% - var(--space-6))' }}
              title={t('admin.live_preview', 'Live Preview (Client View)')}
              loading="lazy"
              sandbox="allow-scripts allow-same-origin"
              onLoad={() => {
                setPreviewLoaded(true);
                if (logoPreview && previewRef.current?.contentWindow) {
                  previewRef.current.contentWindow.postMessage(
                    { type: 'branding_preview_logo', logoUrl: logoPreview },
                    '*'
                  );
                }
              }}
            />
          </div>
        ) : (
          <div className="border border-[var(--brand-border)] rounded-[40px] overflow-hidden w-full max-w-sm mx-auto shadow-[var(--elev-4)] relative h-[700px] flex items-center justify-center"
            style={{ backgroundColor: config.bg }}>
            <div className="absolute top-0 inset-x-0 h-6 bg-black flex justify-center items-center z-20">
              <div className="w-20 h-4 bg-[var(--brand-surface)] border border-[var(--brand-border)] border-t-0 rounded-b-xl" />
            </div>
            <div className="text-center px-8 space-y-3">
              <i className="ti ti-device-mobile text-3xl" style={{ color: config.text || 'var(--brand-text-muted)' }} aria-hidden="true" />
              <p className="text-sm" style={{ color: config.text || 'var(--brand-text-muted)' }}>
                {t('admin.branding_preview_hint', 'Complete onboarding to generate your client page URL. The preview will show the real client page with draft branding applied.')}
              </p>
            </div>
          </div>
        )}
        {slug && (
          <>
            <UrlRow label={t('admin.client_url_ssr', 'Website URL (SEO)')} url={`${window.location.origin}/s/${slug}`} k="ssr" />
            <UrlRow label={t('admin.client_url_spa', 'Website URL (App)')} url={`${window.location.origin}/branding-preview/${slug}`} k="spa" />

            <p className="text-step-2xs text-center mt-2 opacity-60" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.branding_preview_note', 'Preview loads client page with draft colors')}
            </p>
          </>
        )}
      </div>
    </div>
  );
}
