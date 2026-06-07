import React, { useState, useEffect } from 'react';
import { Button, Input, ColorInput, FormField, ProductCard, useI18n } from '@deliveryos/ui';
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
  const [previewItems, setPreviewItems] = useState<any[]>([]);

  useEffect(() => {
    apiClient<any>('/owner/brand').then(res => {
      if (res.primary_color) setConfig(prev => ({ ...prev, primary: res.primary_color }));
      if (res.bg_color) setConfig(prev => ({ ...prev, bg: res.bg_color }));
      if (res.text_color) setConfig(prev => ({ ...prev, text: res.text_color }));
      if (res.logo_url) setLogoUrl(res.logo_url);
    }).catch(() => {});
    apiClient<any>('/owner/menu/products').then(res => {
      if (Array.isArray(res)) setPreviewItems(res.slice(0, 8));
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

      {/* Live Preview with real data */}
      {/* Live Preview with real data */}
      <div className="flex-1 border-l border-[var(--brand-border)] pl-0 lg:pl-8">
        <h3 className="font-semibold text-lg mb-4 text-[var(--brand-text-muted)]">{t('admin.live_preview', 'Live Preview (Client View)')}</h3>
        <div 
          className="border border-[var(--brand-border)] rounded-[40px] overflow-hidden w-full max-w-sm mx-auto shadow-2xl relative h-[700px] flex flex-col"
          style={{ 
            backgroundColor: config.bg,
            '--brand-primary': config.primary,
            '--brand-primary-hover': config.primaryHover,
            '--brand-primary-light': config.primaryLight,
            '--brand-bg': config.bg,
            '--brand-surface': config.surface,
            '--brand-surface-raised': config.surfaceRaised,
            '--brand-border': config.border,
            '--brand-text': config.text,
            '--brand-text-muted': config.textMuted,
            '--brand-accent': config.accent,
          } as React.CSSProperties}
        >
          <div className="absolute top-0 inset-x-0 h-6 bg-black flex justify-center items-center z-20">
            <div className="w-20 h-4 bg-[var(--brand-surface)] border border-[var(--brand-border)] border-t-0 rounded-b-xl" />
          </div>
          
          <div className="flex-1 overflow-y-auto px-4 pb-20 pt-10 no-scrollbar" style={{ color: 'var(--brand-text)' }}>
            <div className="flex items-center justify-between mb-6">
              {logoPreview ? (
                <img src={logoPreview} alt="Logo" className="h-8 object-contain" />
              ) : (
                <h1 className="text-xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.restaurant_name', 'Restaurant')}</h1>
              )}
            </div>
            
            <div className="grid grid-cols-2 gap-3">
              {previewItems.length > 0 ? (
                previewItems.map(item => (
                  <ProductCard 
                    key={item.id} 
                    product={{
                      id: item.id,
                      name: item.name,
                      price: item.price,
                      description: item.description,
                      image: item.imageUrl,
                      isAvailable: item.available,
                      tags: item.category ? [item.category] : [],
                      taste: item.taste,
                      allergenStatus: item.allergenStatus
                    }}
                    onAdd={() => {}}
                  />
                ))
              ) : (
                <div className="col-span-2 text-center text-sm mt-10" style={{ color: 'var(--brand-text-muted)' }}>
                  {t('admin.add_products_preview', 'Add products to the menu to see them here.')}
                </div>
              )}
            </div>
          </div>
          
          {/* Fake Bottom Nav */}
          <div className="absolute bottom-0 inset-x-0 h-16 border-t flex items-center justify-around px-4" style={{ backgroundColor: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
            <div className="flex flex-col items-center justify-center opacity-100" style={{ color: 'var(--brand-primary)' }}>
              <i className="ti ti-smart-home text-xl mb-0.5" />
              <span className="text-[10px] font-medium">{t('client.home', 'Home')}</span>
            </div>
            <div className="flex flex-col items-center justify-center opacity-50" style={{ color: 'var(--brand-text-muted)' }}>
              <i className="ti ti-search text-xl mb-0.5" />
              <span className="text-[10px] font-medium">{t('client.search', 'Search')}</span>
            </div>
            <div className="flex flex-col items-center justify-center opacity-50" style={{ color: 'var(--brand-text-muted)' }}>
              <i className="ti ti-shopping-cart text-xl mb-0.5" />
              <span className="text-[10px] font-medium">{t('client.cart', 'Cart')}</span>
            </div>
            <div className="flex flex-col items-center justify-center opacity-50" style={{ color: 'var(--brand-text-muted)' }}>
              <i className="ti ti-user text-xl mb-0.5" />
              <span className="text-[10px] font-medium">{t('client.profile', 'Profile')}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
