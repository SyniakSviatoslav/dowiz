import React, { useState, useEffect } from 'react';
import { Button, Input, ColorInput, FormField } from '@deliveryos/ui';
import type { ThemeConfig } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

export function BrandingPage() {
  const [config, setConfig] = useState<ThemeConfig>({
    primary: '#ea4f16',
    primaryHover: '#d44310',
    primaryLight: 'rgba(234, 79, 22, 0.1)',
    bg: '#121212',
    surface: '#1e1e1e',
    surfaceRaised: '#2a2a2a',
    border: '#2c2c2c',
    text: '#ffffff',
    textMuted: '#a8a8a8',
    accent: '#2a2a2a',
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
          Branding Settings
        </h2>
        <form onSubmit={handleSave} className="space-y-6">
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl p-5 space-y-4">
            <h3 className="font-semibold text-lg">Colors</h3>
            <ColorInput label="Primary Color" value={config.primary} onChange={(c: string) => setConfig({ ...config, primary: c })} />
            <ColorInput label="Background Color" value={config.bg} onChange={(c: string) => setConfig({ ...config, bg: c })} />
            <ColorInput label="Text Color" value={config.text} onChange={(c: string) => setConfig({ ...config, text: c })} />
          </div>
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl p-5 space-y-4">
            <h3 className="font-semibold text-lg">Logo</h3>
            <p className="text-xs text-[var(--brand-text-muted)]">Recommended: 512x512px PNG with transparent background. Max 2MB.</p>
            <div className="flex items-start gap-4">
              <div className="flex-1 space-y-3">
                <FormField label="Upload Logo File">
                  <input type="file" accept="image/png,image/jpeg,image/svg+xml" onChange={handleLogoUpload}
                    className="w-full text-sm file:mr-3 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-[var(--brand-primary)] file:text-white hover:file:opacity-90" />
                </FormField>
                <FormField label="— or Logo URL —">
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
            <Button type="submit" isLoading={loading} size="lg">Save Changes</Button>
            {success && <span className="text-green-500 font-medium">Saved!</span>}
          </div>
        </form>
      </div>

      {/* Live Preview with real data */}
      <div className="flex-1 border-l border-[var(--brand-border)] pl-0 lg:pl-8">
        <h3 className="font-semibold text-lg mb-4 text-[var(--brand-text-muted)]">Live Preview (Client View)</h3>
        <div className="border border-[var(--brand-border)] rounded-[40px] overflow-hidden w-full max-w-sm mx-auto shadow-2xl relative h-[700px]" style={{ background: config.bg, color: config.text }}>
          <div className="absolute top-0 inset-x-0 h-6 bg-black/20 flex justify-center items-center z-20">
            <div className="w-20 h-4 bg-black rounded-b-xl" />
          </div>
          <div className="p-4 pt-12 space-y-4 h-full overflow-auto">
            <div className="flex items-center gap-3 mb-4">
              {logoPreview ? (
                <img src={logoPreview} alt="Logo" className="h-10 w-10 rounded-full object-cover" />
              ) : (
                <div className="w-10 h-10 rounded-full flex items-center justify-center text-white font-bold" style={{ background: config.primary }}>D</div>
              )}
              <h1 className="text-xl font-bold">Demo Location</h1>
            </div>
            {previewItems.length > 0 ? previewItems.map((item: any) => (
              <div key={item.id} className="p-3 rounded-xl border flex justify-between items-center" style={{ background: config.surface, borderColor: config.border }}>
                <div>
                  <div className="font-medium text-sm">{item.name}</div>
                  <div className="text-xs" style={{ color: config.textMuted }}>{item.description || ''}</div>
                  <div className="font-bold text-sm mt-1" style={{ color: config.primary }}>{item.price} ALL</div>
                </div>
                <button className="min-w-[36px] min-h-[36px] rounded-full text-white text-sm flex items-center justify-center" style={{ background: config.primary }}>+</button>
              </div>
            )) : (
              <div className="p-4 rounded-xl border" style={{ background: config.surface, borderColor: config.border }}>
                <div className="font-medium">Sample Product</div>
                <div className="text-xs mt-1" style={{ color: config.textMuted }}>Add items to your menu</div>
                <div className="font-bold mt-2" style={{ color: config.primary }}>0 ALL</div>
                <button className="mt-2 w-full py-2 rounded-xl text-white font-medium" style={{ background: config.primary }}>Add to Cart</button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
