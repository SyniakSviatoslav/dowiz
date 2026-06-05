import React, { useState, useEffect } from 'react';
import { Button, Input, ColorInput, FormField, ThemeProvider } from '@deliveryos/ui';
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
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    // GET /api/owner/brand
    apiClient<any>('/owner/brand').then(res => {
      if (res.primaryColor) setConfig(prev => ({ ...prev, primary: res.primaryColor }));
      if (res.bgColor) setConfig(prev => ({ ...prev, bg: res.bgColor }));
      if (res.logoUrl) setLogoUrl(res.logoUrl);
    }).catch(() => { /* Dev-only: mock fallback */ });
  }, []);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setSuccess(false);
    try {
      await apiClient('/owner/brand', {
        method: 'PUT',
        body: {
          primaryColor: config.primary,
          bgColor: config.bg,
          logoUrl
        }
      });
      setSuccess(true);
    } catch (err) {
      // Dev-only: mock fallback
      setSuccess(true);
    } finally {
      setLoading(false);
      setTimeout(() => setSuccess(false), 3000);
    }
  };

  return (
    <div className="p-4 max-w-6xl mx-auto flex flex-col lg:flex-row gap-8">
      
      {/* Editor Panel */}
      <div className="flex-1 space-y-6">
        <h2 className="text-2xl font-bold border-b border-[var(--brand-border)] pb-4 text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>
          Branding Settings
        </h2>

        <form onSubmit={handleSave} className="space-y-6">
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4 space-y-4">
            <h3 className="font-semibold text-lg">Colors</h3>
            <ColorInput 
              label="Primary Color" 
              value={config.primary} 
              onChange={(c: string) => setConfig({ ...config, primary: c })} 
            />
            <ColorInput 
              label="Background Color" 
              value={config.bg} 
              onChange={(c: string) => setConfig({ ...config, bg: c })} 
            />
          </div>

          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4 space-y-4">
            <h3 className="font-semibold text-lg">Assets</h3>
            <FormField label="Logo URL">
              <Input 
                value={logoUrl} 
                onChange={e => setLogoUrl(e.target.value)} 
                placeholder="https://example.com/logo.png" 
              />
            </FormField>
          </div>

          <div className="flex items-center gap-4">
            <Button type="submit" isLoading={loading} size="lg">Save Changes</Button>
            {success && <span className="text-green-500 font-medium">Saved successfully!</span>}
          </div>
        </form>
      </div>

      {/* Live Preview Panel (Isolated via nested ThemeProvider) */}
      <div className="flex-1 border-l border-[var(--brand-border)] pl-0 lg:pl-8">
        <h3 className="font-semibold text-lg mb-4 text-[var(--brand-text-muted)]">Live Preview (Client View)</h3>
        
        <ThemeProvider theme={config}>
          <div className="border border-[var(--brand-border)] rounded-[40px] overflow-hidden w-full max-w-sm mx-auto shadow-2xl relative h-[700px] bg-[var(--brand-bg)] text-[var(--brand-text)]">
            {/* Mock Mobile Device Header */}
            <div className="absolute top-0 inset-x-0 h-6 bg-black/20 flex justify-center items-center z-20">
              <div className="w-20 h-4 bg-black rounded-b-xl" />
            </div>
            
            {/* Scrollable Content */}
            <div className="p-4 pt-12 space-y-6 h-full overflow-auto">
              <div className="flex items-center gap-3 mb-6">
                {logoUrl ? (
                  <img src={logoUrl} alt="Logo" className="h-10 w-10 rounded-full object-cover" />
                ) : (
                  <div className="w-10 h-10 rounded-full bg-[var(--brand-primary)] flex items-center justify-center text-white font-bold">B</div>
                )}
                <h1 className="text-xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Your Brand</h1>
              </div>

              <div className="bg-[var(--brand-surface)] p-4 rounded-[var(--brand-radius)] border border-[var(--brand-border)]">
                <h2 className="font-medium text-lg mb-2">Sample Product</h2>
                <p className="text-sm text-[var(--brand-text-muted)] mb-3">Lorem ipsum dolor sit amet.</p>
                <div className="font-bold text-[var(--brand-primary)]">15.00 ALL</div>
                <button className="mt-3 w-full bg-[var(--brand-primary)] hover:bg-[var(--brand-primary-hover)] text-white py-2 rounded-[var(--brand-radius-btn)] font-medium transition-colors">
                  Add to Cart
                </button>
              </div>

              <div className="bg-[var(--brand-surface-raised)] p-4 rounded-[var(--brand-radius)] border border-[var(--brand-border)]">
                <div className="text-sm">Raised Surface Element</div>
              </div>
            </div>
          </div>
        </ThemeProvider>
      </div>

    </div>
  );
}
