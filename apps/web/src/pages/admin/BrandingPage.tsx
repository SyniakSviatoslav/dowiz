import { safeStorage } from '../../lib/safeStorage.js';
import React, { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { Button, Input, ColorInput, FormField, useI18n, useToast } from '@deliveryos/ui';
import type { ThemeConfig } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

export function BrandingPage() {
  const { t } = useI18n();
  const { showToast } = useToast();
  const previewRef = useRef<HTMLIFrameElement>(null);
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
    }).catch(() => {});
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
      const res = await fetch(`/api/owner/locations/${locationId}/theme/logo`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${safeStorage.get('dos_access_token') || ''}` },
        body: form,
      });
      if (res.ok) {
        const data = await res.json();
        if (data.logo_url) { setLogoUrl(data.logo_url); setLogoDataUrl(''); }
      }
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

  // URL depends only on slug, so colour edits update via postMessage instead of
  // reloading the whole storefront (which reset scroll + flashed the slug name).
  const iframeUrl = useMemo(() => {
    if (!slug) return '';
    return `/branding-preview/${slug}?embed=true&draft=true`;
  }, [slug]);

  return (
    <div className="p-4 max-w-6xl mx-auto flex flex-col lg:flex-row gap-8">
      <div className="flex-1 space-y-6">
        <h2 className="text-2xl font-bold border-b border-[var(--brand-border)] pb-4" style={{ fontFamily: 'var(--brand-font-heading)' }}>
          {t('admin.branding', 'Branding Settings')}
        </h2>
        <form onSubmit={handleSave} className="space-y-6">
          {/* Auto-brand: derive a coherent storefront theme from the venue's
              existing website and/or logo — a starting point the owner can tweak. */}
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl p-5 space-y-3">
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
                <i className="ti ti-wand mr-1.5" />{t('admin.generate', 'Generate')}
              </Button>
            </div>
          </div>
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl p-5 space-y-4">
            <h3 className="font-semibold text-lg">{t('admin.colors', 'Colors')}</h3>
            <ColorInput label={t('admin.primary_color', 'Primary Color')} value={config.primary} onChange={(c: string) => setConfig({ ...config, primary: c })} />
            <ColorInput label={t('admin.bg_color', 'Background Color')} value={config.bg} onChange={(c: string) => setConfig({ ...config, bg: c })} />
            <ColorInput label={t('admin.text_color', 'Text Color')} value={config.text} onChange={(c: string) => setConfig({ ...config, text: c })} />
          </div>
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl p-5 space-y-4">
            <h3 className="font-semibold text-lg">{t('admin.logo', 'Logo')}</h3>
            <p className="text-xs text-[var(--brand-text-muted)]">{t('admin.logo_hint', 'Recommended: 512x512px PNG with transparent background. Max 2MB.')}</p>
            <div className="flex items-start gap-4">
              <div className="flex-1 space-y-3">
                <FormField label={t('admin.upload_logo', logoUploading ? 'Uploading…' : 'Upload Logo File')}>
                  <input type="file" accept="image/png,image/jpeg,image/svg+xml" onChange={handleLogoUpload} disabled={logoUploading}
                    className="w-full text-sm file:mr-3 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-[var(--brand-primary)] file:text-white hover:file:opacity-90 disabled:opacity-50" />
                </FormField>
                <FormField label={t('admin.or_logo_url', '— or Logo URL —')}>
                  <Input value={logoUrl} onChange={e => setLogoUrl(e.target.value)} placeholder="https://cdn.example.com/logo.png" />
                </FormField>
              </div>
              {logoPreview && (
                <div className="shrink-0 w-20 h-20 rounded-xl border border-[var(--brand-border)] overflow-hidden bg-white/5 flex items-center justify-center">
                  <img src={logoPreview} alt="Logo preview" className="max-w-full max-h-full object-contain" />
                </div>
              )}
            </div>
          </div>
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl p-5 space-y-4">
            <h3 className="font-semibold text-lg">{t('admin.google_info', 'Google Maps Info')}</h3>
            <p className="text-xs text-[var(--brand-text-muted)]">{t('admin.google_info_hint', 'Displayed on client menu page. Update periodically from your Google Maps listing.')}</p>
            <div className="grid grid-cols-2 gap-4">
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
          </div>
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl p-5 space-y-4">
            <h3 className="font-semibold text-lg">{t('admin.social_links', 'Social links')}</h3>
            <p className="text-xs text-[var(--brand-text-muted)]">{t('admin.social_links_hint', 'Shown as icons in your storefront footer. Leave blank to hide.')}</p>
            <FormField label={t('admin.instagram', 'Instagram')}>
              <Input value={socialInstagram} onChange={e => setSocialInstagram(e.target.value)} placeholder="https://instagram.com/yourplace" />
            </FormField>
            <FormField label={t('admin.facebook', 'Facebook')}>
              <Input value={socialFacebook} onChange={e => setSocialFacebook(e.target.value)} placeholder="https://facebook.com/yourplace" />
            </FormField>
          </div>
          <div className="flex items-center gap-4">
            <Button type="submit" isLoading={loading} size="lg">{t('common.save', 'Save Changes')}</Button>
          </div>
        </form>
      </div>

      {/* Live Preview — real client page in iframe */}
      <div className="flex-1 border-l border-[var(--brand-border)] pl-0 lg:pl-8">
        <h3 className="font-semibold text-lg mb-4 text-[var(--brand-text-muted)]">{t('admin.live_preview', 'Live Preview (Client View)')}</h3>
        {iframeUrl ? (
          <div className="border border-[var(--brand-border)] rounded-[40px] overflow-hidden w-full max-w-sm mx-auto shadow-2xl relative h-[700px]">
            <div className="absolute top-0 inset-x-0 h-6 bg-black flex justify-center items-center z-20">
              <div className="w-20 h-4 bg-[var(--brand-surface)] border border-[var(--brand-border)] border-t-0 rounded-b-xl" />
            </div>
            <iframe
              ref={previewRef}
              src={iframeUrl}
              className="w-full h-full border-0"
              style={{ marginTop: '24px' }}
              title={t('admin.live_preview', 'Live Preview (Client View)')}
              loading="lazy"
              sandbox="allow-scripts allow-same-origin"
              onLoad={() => {
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
          <div className="border border-[var(--brand-border)] rounded-[40px] overflow-hidden w-full max-w-sm mx-auto shadow-2xl relative h-[700px] flex items-center justify-center"
            style={{ backgroundColor: config.bg }}>
            <div className="absolute top-0 inset-x-0 h-6 bg-black flex justify-center items-center z-20">
              <div className="w-20 h-4 bg-[var(--brand-surface)] border border-[var(--brand-border)] border-t-0 rounded-b-xl" />
            </div>
            <p className="text-sm text-center px-8" style={{ color: config.text || 'var(--brand-text-muted)' }}>
              {t('admin.branding_preview_hint', 'Complete onboarding to generate your client page URL. The preview will show the real client page with draft branding applied.')}
            </p>
          </div>
        )}
        {slug && (
          <>
            <p className="text-xs text-center mt-3" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.client_url_ssr', 'Website URL (SEO)')}
            </p>
            <div className="flex items-center justify-center gap-2 mt-1">
              <a
                href={`${window.location.origin}/s/${slug}`}
                target="_blank"
                rel="noopener noreferrer"
                className="font-mono text-xs text-[var(--brand-primary)] hover:underline"
              >
                {window.location.origin}/s/{slug}
              </a>
              <button
                type="button"
                onClick={() => { navigator.clipboard.writeText(`${window.location.origin}/s/${slug}`); }}
                className="text-[10px] text-[var(--brand-text-muted)] hover:text-[var(--brand-primary)] underline"
                title={t('common.copy', 'Copy')}
              >
                {t('common.copy', 'Copy')}
              </button>
            </div>

            <p className="text-xs text-center mt-2" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.client_url_spa', 'Website URL (App)')}
            </p>
            <div className="flex items-center justify-center gap-2 mt-1">
              <a
                href={`${window.location.origin}/branding-preview/${slug}`}
                target="_blank"
                rel="noopener noreferrer"
                className="font-mono text-xs text-[var(--brand-primary)] hover:underline"
              >
                {window.location.origin}/branding-preview/{slug}
              </a>
              <button
                type="button"
                onClick={() => { navigator.clipboard.writeText(`${window.location.origin}/branding-preview/${slug}`); }}
                className="text-[10px] text-[var(--brand-text-muted)] hover:text-[var(--brand-primary)] underline"
                title={t('common.copy', 'Copy')}
              >
                {t('common.copy', 'Copy')}
              </button>
            </div>

            <p className="text-[10px] text-center mt-1 opacity-60" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.branding_preview_note', 'Preview loads client page with draft colors')}
            </p>
          </>
        )}
      </div>
    </div>
  );
}
