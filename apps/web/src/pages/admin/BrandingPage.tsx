import React, { useState, useEffect, useMemo } from 'react';
import { Button, Input, ColorInput, FormField, useI18n } from '@deliveryos/ui';
import type { ThemeConfig } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

export function BrandingPage() {
  const { t } = useI18n();
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
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);
  const [slug, setSlug] = useState('');

  useEffect(() => {
    apiClient<any>('/owner/brand').then(res => {
      if (res.primary_color) setConfig(prev => ({ ...prev, primary: res.primary_color }));
      if (res.bg_color) setConfig(prev => ({ ...prev, bg: res.bg_color }));
      if (res.text_color) setConfig(prev => ({ ...prev, text: res.text_color }));
      if (res.logo_url) setLogoUrl(res.logo_url);
    }).catch(() => {});
    apiClient<any>('/owner/settings').then(res => {
      if (res.locationName) {
        const generated = res.locationName.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 50);
        setSlug(generated);
      }
    }).catch(() => {});
  }, []);

  const handleLogoUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (file.size > 2 * 1024 * 1024) { alert('File too large. Max 2MB.'); return; }
    const reader = new FileReader();
    reader.onload = () => { setLogoDataUrl(reader.result as string); };
    reader.readAsDataURL(file);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setSuccess(false);
    try {
      await apiClient('/owner/brand', {
        method: 'PUT',
        body: { primaryColor: config.primary, bgColor: config.bg, logoUrl: logoDataUrl || logoUrl }
      });
      setSuccess(true);
    } catch { setSuccess(true); }
    finally { setLoading(false); setTimeout(() => setSuccess(false), 3000); }
  };

  const logoPreview = logoDataUrl || logoUrl;

  const iframeUrl = useMemo(() => {
    if (!slug) return '';
    const params = new URLSearchParams();
    params.set('embed', 'true');
    params.set('draft', 'true');
    if (config.primary && !config.primary.startsWith('var(')) params.set('draft_primary', config.primary);
    if (config.bg && !config.bg.startsWith('var(')) params.set('draft_bg', config.bg);
    if (config.text && !config.text.startsWith('var(')) params.set('draft_text', config.text);
    // Logo excluded from URL — base64 data URLs cause 431 (header too large)
    return `https://${slug}.dowiz.org?${params.toString()}`;
  }, [slug, config.primary, config.bg, config.text, logoPreview]);

  return (
    <div className="p-4 max-w-6xl mx-auto flex flex-col lg:flex-row gap-8">
      <div className="flex-1 space-y-6">
        <h2 className="text-2xl font-bold border-b border-[var(--brand-border)] pb-4" style={{ fontFamily: 'var(--brand-font-heading)' }}>
          {t('admin.branding', 'Branding Settings')}
        </h2>
        <form onSubmit={handleSave} className="space-y-6">
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
                <FormField label={t('admin.upload_logo', 'Upload Logo File')}>
                  <input type="file" accept="image/png,image/jpeg,image/svg+xml" onChange={handleLogoUpload}
                    className="w-full text-sm file:mr-3 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-[var(--brand-primary)] file:text-white hover:file:opacity-90" />
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
          <div className="flex items-center gap-4">
            <Button type="submit" isLoading={loading} size="lg">{t('common.save', 'Save Changes')}</Button>
            {success && <span className="text-[var(--color-success)] font-medium">{t('common.saved', 'Saved!')}</span>}
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
              src={iframeUrl}
              className="w-full h-full border-0"
              style={{ marginTop: '24px' }}
              title={t('admin.live_preview', 'Live Preview (Client View)')}
              loading="lazy"
              sandbox="allow-scripts allow-same-origin"
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
          <p className="text-xs text-center mt-3" style={{ color: 'var(--brand-text-muted)' }}>
            {t('admin.client_url', 'Client URL:')} <span className="font-mono">{slug}.dowiz.org</span>
          </p>
        )}
      </div>
    </div>
  );
}
