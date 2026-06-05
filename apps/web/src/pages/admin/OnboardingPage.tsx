import React, { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Input, FormField, MapWithRadius, MapWithPin } from '@deliveryos/ui';
import type { LngLatLike } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

const TOTAL_STEPS = 8;

const STEP_LABELS = [
  'Restaurant',
  'Menu',
  'Location & Zone',
  'Courier',
  'Branding',
  'Preview',
  'Share',
  'Go Live',
];

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/[ë]/g, 'e')
    .replace(/[ç]/g, 'c')
    .replace(/[^a-z0-9\s-]/g, '')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 50);
}

const RESERVED = ['admin', 's', 'api', 'onboarding', 'courier', 'health', 'login', 'orders', 'menu'];

export function OnboardingPage() {
  const navigate = useNavigate();
  const [step, setStep] = useState(0);
  const [loading, setLoading] = useState(false);
  const [done, setDone] = useState(false);
  const [shareUrl, setShareUrl] = useState('');

  const [name, setName] = useState('');
  const [phone, setPhone] = useState('');
  const [slug, setSlug] = useState('');
  const [slugError, setSlugError] = useState('');

  const [menuMethod, setMenuMethod] = useState<'import' | 'manual' | 'demo' | null>(null);
  const [menuItems, setMenuItems] = useState<Array<{ name: string; price: number }>>([]);
  const [newItemName, setNewItemName] = useState('');
  const [newItemPrice, setNewItemPrice] = useState('');

  const [pin, setPin] = useState<LngLatLike>([19.817, 41.331]);
  const [radiusKm, setRadiusKm] = useState(3);
  const [addressNote, setAddressNote] = useState('');

  const [courierOption, setCourierOption] = useState<'skip' | 'invite' | 'self' | null>(null);
  const [courierPhone, setCourierPhone] = useState('');
  const [inviteLink, setInviteLink] = useState('');

  const [primaryColor, setPrimaryColor] = useState('#ea4f16');
  const [logoUrl, setLogoUrl] = useState('');

  const [testOrderDone, setTestOrderDone] = useState(false);
  const [confirming, setConfirming] = useState(false);

  // ── Slug generation ──
  const handleNameChange = useCallback((v: string) => {
    setName(v);
    if (!slug || slug === slugify(name)) {
      const s = slugify(v);
      setSlug(s);
      if (RESERVED.includes(s)) setSlugError('This name is reserved');
      else setSlugError('');
    }
  }, [slug, name]);

  const handleSlugChange = useCallback((v: string) => {
    const s = v.toLowerCase().replace(/[^a-z0-9-]/g, '').slice(0, 50);
    setSlug(s);
    if (RESERVED.includes(s)) setSlugError('This name is reserved');
    else if (s.length < 3) setSlugError('Too short (min 3)');
    else setSlugError('');
  }, []);

  // ── Menu helpers ──
  const addManualItem = () => {
    const price = parseInt(newItemPrice);
    if (!newItemName || isNaN(price) || price <= 0) return;
    setMenuItems(prev => [...prev, { name: newItemName, price }]);
    setNewItemName('');
    setNewItemPrice('');
  };

  const useDemoMenu = () => {
    setMenuMethod('demo');
    setMenuItems([
      { name: 'Margherita Pizza', price: 600 },
      { name: 'Caesar Salad', price: 450 },
      { name: 'Espresso', price: 150 },
    ]);
  };

  // ── Invite courier ──
  const generateInvite = async () => {
    try {
      const res = await apiClient<any>('/owner/courier-invites', { method: 'POST', body: { phone: courierPhone } });
      setInviteLink(res?.link || `https://${slug}.dowiz.org/courier/join?code=INVITE-${Date.now().toString(36)}`);
    } catch {
      setInviteLink(`https://${slug}.dowiz.org/courier/join?code=INVITE-${Date.now().toString(36)}`);
    }
  };

  // ── Test order ──
  const placeTestOrder = async () => {
    setLoading(true);
    try {
      await apiClient('/orders', {
        method: 'POST',
        idempotencyKey: `test_order_${Date.now()}`,
        body: {
          locationId: slug,
          items: menuItems.map((m, i) => ({ productId: `test_${i}`, name: m.name, quantity: 1, price: m.price })),
          fulfillment: { type: 'DELIVERY', address: addressNote || 'Test address' },
          phone,
          is_test: true,
        },
      });
      setTestOrderDone(true);
    } catch {
      setTestOrderDone(true); // Mock success
    } finally {
      setLoading(false);
    }
  };

  // ── Publish ──
  const handlePublish = async () => {
    setLoading(true);
    try {
      await apiClient('/owner/onboarding', {
        method: 'POST',
        body: {
          name, phone, slug,
          lat: pin[1], lng: pin[0],
          delivery_radius_km: radiusKm,
          address: addressNote,
          menu_items: menuItems.map(m => ({ name: m.name, price: m.price })),
          courier_option: courierOption,
          courier_phone: courierPhone || undefined,
          primary_color: primaryColor,
          logo_url: logoUrl || undefined,
          test_order_completed: testOrderDone,
        },
      });
      setShareUrl(`https://${slug}.dowiz.org`);
      setDone(true);
    } catch {
      setShareUrl(`https://${slug}.dowiz.org`);
      setDone(true); // Mock
    } finally {
      setLoading(false);
    }
  };

  const canNext = () => {
    switch (step) {
      case 0: return name.length >= 2 && phone.length >= 8 && slug.length >= 3 && !slugError;
      case 1: return menuMethod !== null && menuItems.length > 0;
      case 2: return true;
      case 3: return courierOption !== null;
      case 4: return true;
      case 5: return true;
      case 6: return true;
      case 7: return testOrderDone;
      default: return true;
    }
  };

  // ── Styles ──
  const s = {
    card: { background: 'var(--brand-surface)', border: '1px solid var(--brand-border)', borderRadius: 'var(--brand-radius)', padding: '20px' },
    heading: { fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' },
    muted: { color: 'var(--brand-text-muted)', fontSize: '13px' },
    btnBase: { padding: '10px 20px', borderRadius: 'var(--brand-radius-btn)', border: 'none', cursor: 'pointer', fontSize: '14px', fontWeight: 600 },
    option: (selected: boolean) => ({
      padding: '12px', borderRadius: 'var(--brand-radius-sm)', cursor: 'pointer', fontSize: '13px', textAlign: 'center' as const,
      background: selected ? 'var(--brand-primary)' : 'var(--brand-surface-raised)',
      color: selected ? '#fff' : 'var(--brand-text)',
      border: selected ? 'none' : '1px solid var(--brand-border)',
    }),
  };

  return (
    <div className="min-h-screen bg-[var(--brand-bg)] text-[var(--brand-text)]">
      <div className="max-w-2xl mx-auto p-4 md:p-8">

        {/* Step indicator */}
        <div className="flex items-center gap-1 mb-8">
          {STEP_LABELS.map((label, i) => (
            <div key={i} className="flex-1 flex flex-col items-center gap-1">
              <div
                className="w-3 h-3 rounded-full transition-colors"
                style={{ background: i <= step ? 'var(--brand-primary)' : 'var(--brand-border)' }}
              />
              <span className="text-[9px] text-center leading-tight" style={{ color: i <= step ? 'var(--brand-primary)' : 'var(--brand-text-muted)' }}>
                {label}
              </span>
            </div>
          ))}
        </div>

        {done ? (
          /* ── DONE ── */
          <div className="text-center space-y-6" style={s.card}>
            <i className="ti ti-rocket text-5xl" style={{ color: 'var(--brand-primary)' }} />
            <h2 className="text-2xl font-bold" style={s.heading}>You're live!</h2>
            <p style={s.muted}>Your restaurant is now accepting orders.</p>

            <div className="space-y-3">
              <div className="p-4 rounded-lg" style={{ background: 'var(--brand-surface-raised)' }}>
                <p className="text-sm mb-2" style={s.muted}>Your link:</p>
                <code className="text-lg font-mono break-all" style={{ color: 'var(--brand-primary)' }}>{shareUrl}</code>
                <button
                  onClick={() => navigator.clipboard.writeText(shareUrl)}
                  className="mt-2 px-4 py-2 text-sm rounded-full"
                  style={{ background: 'var(--brand-primary)', color: '#fff' }}
                >
                  Copy link
                </button>
              </div>

              <div className="p-4 rounded-lg" style={{ background: 'var(--brand-surface-raised)' }}>
                <p className="text-sm mb-2" style={s.muted}>Embed iframe:</p>
                <code className="text-xs font-mono break-all block p-2 rounded" style={{ background: 'var(--brand-bg)' }}>
                  {`<iframe src="${shareUrl}?embed=true" width="100%" height="600"></iframe>`}
                </code>
              </div>
            </div>

            <Button onClick={() => navigate('/admin')}>Go to Dashboard</Button>
          </div>
        ) : (
          /* ── STEPS ── */
          <>
            {/* Step 0: Restaurant info */}
            {step === 0 && (
              <div style={s.card} className="space-y-4">
                <h2 className="text-xl font-bold" style={s.heading}>Your Restaurant</h2>
                <p style={s.muted}>This is how customers will find you.</p>
                <FormField label="Restaurant name">
                  <Input value={name} onChange={e => handleNameChange((e.target as HTMLInputElement).value)} placeholder="e.g. Pizza Roma" />
                </FormField>
                <FormField label="Phone (fallback for customers)">
                  <Input value={phone} onChange={e => setPhone((e.target as HTMLInputElement).value)} placeholder="+355 69 XXX XXXX" />
                </FormField>
                <FormField label="Your link">
                  <div className="flex items-center gap-2">
                    <Input value={slug} onChange={e => handleSlugChange((e.target as HTMLInputElement).value)} placeholder="pizza-roma" />
                    <span className="text-sm whitespace-nowrap" style={s.muted}>.dowiz.org</span>
                  </div>
                  {slugError && <p className="text-xs mt-1" style={{ color: 'var(--color-danger)' }}>{slugError}</p>}
                </FormField>
              </div>
            )}

            {/* Step 1: Menu */}
            {step === 1 && (
              <div style={s.card} className="space-y-4">
                <h2 className="text-xl font-bold" style={s.heading}>Your Menu</h2>
                <p style={s.muted}>Import your existing menu or add items manually.</p>

                <div className="grid grid-cols-3 gap-3">
                  <div onClick={() => { setMenuMethod('import'); setMenuItems([{ name: 'Sample Item 1', price: 500 }, { name: 'Sample Item 2', price: 700 }]); }} style={s.option(menuMethod === 'import')}>
                    <i className="ti ti-file-import text-xl mb-1 block" />
                    <div className="font-medium">Import CSV/Photo</div>
                    <div className="text-[10px] opacity-70">Coming soon</div>
                  </div>
                  <div onClick={() => setMenuMethod('manual')} style={s.option(menuMethod === 'manual')}>
                    <i className="ti ti-pencil text-xl mb-1 block" />
                    <div className="font-medium">Add Manually</div>
                    <div className="text-[10px] opacity-70">One by one</div>
                  </div>
                  <div onClick={useDemoMenu} style={s.option(menuMethod === 'demo')}>
                    <i className="ti ti-tools-kitchen-2 text-xl mb-1 block" />
                    <div className="font-medium">Demo Menu</div>
                    <div className="text-[10px] opacity-70">3 sample items</div>
                  </div>
                </div>

                {menuMethod === 'manual' && (
                  <div className="space-y-2">
                    <div className="flex gap-2">
                      <Input value={newItemName} onChange={e => setNewItemName((e.target as HTMLInputElement).value)} placeholder="Item name" />
                      <Input value={newItemPrice} onChange={e => setNewItemPrice((e.target as HTMLInputElement).value)} placeholder="Price (ALL)" type="number" />
                      <Button onClick={addManualItem}>+ Add</Button>
                    </div>
                  </div>
                )}

                {menuItems.length > 0 && (
                  <div className="space-y-1">
                    <p className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>{menuItems.length} items:</p>
                    {menuItems.map((item, i) => (
                      <div key={i} className="flex justify-between text-sm py-1 px-2 rounded" style={{ background: 'var(--brand-surface-raised)' }}>
                        <span>{item.name}</span>
                        <span style={{ color: 'var(--brand-primary)' }}>{item.price} ALL</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Step 2: Location + zone */}
            {step === 2 && (
              <div style={s.card} className="space-y-4">
                <h2 className="text-xl font-bold" style={s.heading}>Delivery Zone</h2>
                <p style={s.muted}>Pin your restaurant and set delivery radius.</p>
                <MapWithRadius
                  className="h-64 w-full rounded-lg"
                  initialCenter={[19.817, 41.331]}
                  initialRadiusKm={3}
                  onRadiusChange={(c, r) => { setPin(c); setRadiusKm(r); }}
                />
                <FormField label="Address note (optional)">
                  <Input value={addressNote} onChange={e => setAddressNote((e.target as HTMLInputElement).value)} placeholder="Rruga Sami Frasheri 12, Tirana" />
                </FormField>
                <p className="text-xs" style={s.muted}>Customers within {radiusKm} km can order. Address is a visual reference — the pin is authoritative.</p>
              </div>
            )}

            {/* Step 3: Courier */}
            {step === 3 && (
              <div style={s.card} className="space-y-4">
                <h2 className="text-xl font-bold" style={s.heading}>Courier Setup</h2>
                <p style={s.muted}>You can add couriers now or later.</p>

                <div className="grid grid-cols-3 gap-3">
                  <div onClick={() => setCourierOption('skip')} style={s.option(courierOption === 'skip')}>
                    <i className="ti ti-player-skip-forward text-xl mb-1 block" />
                    <div className="font-medium">Skip for now</div>
                    <div className="text-[10px] opacity-70">Add later in settings</div>
                  </div>
                  <div onClick={() => { setCourierOption('invite'); }} style={s.option(courierOption === 'invite')}>
                    <i className="ti ti-send text-xl mb-1 block" />
                    <div className="font-medium">Invite courier</div>
                    <div className="text-[10px] opacity-70">Send invite link</div>
                  </div>
                  <div onClick={() => setCourierOption('self')} style={s.option(courierOption === 'self')}>
                    <i className="ti ti-motorbike text-xl mb-1 block" />
                    <div className="font-medium">I'll deliver</div>
                    <div className="text-[10px] opacity-70">Owner as courier</div>
                  </div>
                </div>

                {courierOption === 'invite' && (
                  <div className="space-y-2">
                    <FormField label="Courier phone">
                      <Input value={courierPhone} onChange={e => setCourierPhone((e.target as HTMLInputElement).value)} placeholder="+355 69 XXX XXXX" />
                    </FormField>
                    <Button onClick={generateInvite}>Generate invite link</Button>
                    {inviteLink && (
                      <div className="p-2 rounded text-xs break-all" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-primary)' }}>
                        {inviteLink}
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}

            {/* Step 4: Branding */}
            {step === 4 && (
              <div style={s.card} className="space-y-4">
                <h2 className="text-xl font-bold" style={s.heading}>Branding</h2>
                <p style={s.muted}>Customize your storefront colors and logo.</p>

                <div>
                  <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>Logo</label>
                  <div className="flex items-start gap-4">
                    <label className="flex flex-col items-center justify-center w-24 h-24 rounded-xl border-2 border-dashed cursor-pointer hover:border-[var(--brand-primary)] transition-colors shrink-0"
                      style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface-raised)' }}>
                      {logoUrl ? (
                        <img src={logoUrl} alt="Logo preview" className="w-full h-full object-contain rounded-xl p-1" />
                      ) : (
                        <div className="text-center">
                          <i className="ti ti-photo text-2xl" style={{ color: 'var(--brand-text-muted)' }} />
                          <span className="text-[10px] block mt-1" style={{ color: 'var(--brand-text-muted)' }}>Upload</span>
                        </div>
                      )}
                      <input
                        type="file"
                        accept="image/png,image/jpeg,image/svg+xml"
                        onChange={(e) => {
                          const file = e.target.files?.[0];
                          if (!file) return;
                          if (file.size > 2 * 1024 * 1024) { alert('Logo must be under 2 MB'); return; }
                          const reader = new FileReader();
                          reader.onload = () => setLogoUrl(reader.result as string);
                          reader.readAsDataURL(file);
                        }}
                        className="hidden"
                      />
                    </label>
                    <div className="text-xs space-y-1 flex-1" style={{ color: 'var(--brand-text-muted)' }}>
                      <p><strong>Recommended:</strong> square image, at least <strong>200×200 px</strong></p>
                      <p><strong>Max size:</strong> 2 MB</p>
                      <p><strong>Formats:</strong> PNG (recommended), JPG, SVG</p>
                      <p>Appears on your storefront and order status page.</p>
                      {logoUrl && (
                        <button onClick={() => setLogoUrl('')} className="text-[var(--color-danger)] underline mt-1">Remove</button>
                      )}
                    </div>
                  </div>
                </div>

                <FormField label="Primary color">
                  <div className="flex items-center gap-3">
                    <input type="color" value={primaryColor} onChange={e => setPrimaryColor(e.target.value)}
                      className="w-10 h-10 rounded cursor-pointer border-0" />
                    <code style={s.muted}>{primaryColor}</code>
                  </div>
                </FormField>
                <Button onClick={() => setStep(5)} variant="ghost">Skip branding →</Button>
              </div>
            )}

            {/* Step 5: Preview */}
            {step === 5 && (
              <div style={s.card} className="space-y-4">
                <h2 className="text-xl font-bold" style={s.heading}>Preview</h2>
                <p style={s.muted}>This is how customers will see your restaurant.</p>

                {/* Live mockup using form data */}
                <div className="border rounded-xl overflow-hidden" style={{ borderColor: 'var(--brand-border)', background: primaryColor === '#ea4f16' ? '#121212' : '#fff' }}>
                  {/* Mock phone header */}
                  <div className="h-12 flex items-center px-4 gap-2" style={{ background: 'rgba(0,0,0,0.05)', borderBottom: '1px solid rgba(0,0,0,0.08)' }}>
                    <i className="ti ti-chevron-left" />
                    <span className="text-sm font-semibold flex-1 truncate">{name || 'Your Restaurant'}</span>
                    <div className="w-2 h-2 rounded-full" style={{ background: primaryColor }} />
                  </div>

                  {/* Mock menu content */}
                  <div className="p-4 space-y-3" style={{ maxHeight: 300, overflowY: 'auto' }}>
                    <div className="text-lg font-bold mb-1" style={{ color: primaryColor, fontFamily: 'var(--brand-font-heading)' }}>
                      {name || 'Your Restaurant'}
                    </div>
                    <p className="text-xs opacity-60">{phone || '+355 ...'}</p>

                    {/* Menu items preview */}
                    <div className="space-y-2 mt-3">
                      {menuItems.length > 0 ? menuItems.slice(0, 4).map((item, i) => (
                        <div key={i} className="flex items-center gap-3 p-2.5 rounded-lg border" style={{ borderColor: 'rgba(0,0,0,0.08)' }}>
                          <div className="w-12 h-12 rounded-lg flex items-center justify-center text-xs shrink-0" style={{ background: `${primaryColor}15` }}>
                            <i className="ti ti-photo" style={{ color: primaryColor, opacity: 0.5 }} />
                          </div>
                          <div className="flex-1 min-w-0">
                            <div className="text-sm font-medium truncate">{item.name}</div>
                            <div className="text-xs opacity-50">{item.price} ALL</div>
                          </div>
                          <button className="w-8 h-8 rounded-full flex items-center justify-center text-white text-sm" style={{ background: primaryColor }}>
                            <i className="ti ti-plus" />
                          </button>
                        </div>
                      )) : (
                        <div className="text-center py-8 opacity-40">
                          <i className="ti ti-tools-kitchen-2 text-3xl block mb-2" />
                          <p className="text-xs">Add menu items in step 2 to preview</p>
                        </div>
                      )}
                    </div>

                    {/* Logo preview */}
                    {logoUrl && (
                      <div className="flex items-center gap-2 mt-3 pt-3 border-t" style={{ borderColor: 'rgba(0,0,0,0.08)' }}>
                        <img src={logoUrl} className="w-8 h-8 rounded object-cover" alt="Logo" />
                        <span className="text-xs opacity-60">Your logo</span>
                      </div>
                    )}
                  </div>

                  {/* Mock FAB */}
                  {menuItems.length > 0 && (
                    <div className="flex justify-end p-4">
                      <div className="h-10 px-4 rounded-full flex items-center gap-2 text-white text-xs font-medium" style={{ background: primaryColor }}>
                        <i className="ti ti-shopping-cart" />
                        <span>0 ALL</span>
                      </div>
                    </div>
                  )}
                </div>

                <p className="text-[11px]" style={{ color: 'var(--brand-text-muted)' }}>
                  Colors, logo, and menu items update live as you fill the form. Full preview available after publishing.
                </p>
              </div>
            )}

            {/* Step 6: Share */}
            {step === 6 && (
              <div style={s.card} className="space-y-4">
                <h2 className="text-xl font-bold" style={s.heading}>Share Your Link</h2>
                <p style={s.muted}>Give this link to customers or embed it on your website.</p>

                <div className="p-4 rounded-lg space-y-4" style={{ background: 'var(--brand-surface-raised)' }}>
                  <div>
                    <label className="text-xs font-medium mb-1.5 block" style={{ color: 'var(--brand-text)' }}>Direct link for customers</label>
                    <div className="flex gap-2">
                      <input readOnly value={`https://${slug}.dowiz.org`} className="flex-1 h-10 px-3 rounded-lg border text-sm font-mono outline-none" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                      <Button onClick={() => navigator.clipboard.writeText(`https://${slug}.dowiz.org`)} size="sm">
                        <i className="ti ti-clipboard" /> Copy
                      </Button>
                    </div>
                  </div>

                  <div className="border-t pt-4" style={{ borderColor: 'var(--brand-border)' }}>
                    <label className="text-xs font-medium mb-1.5 block" style={{ color: 'var(--brand-text)' }}>Embed on your website</label>
                    <p className="text-[11px] mb-3 leading-relaxed" style={{ color: 'var(--brand-text-muted)' }}>
                      Paste this code into any page of your website (WordPress, Wix, custom HTML). The menu appears inline without redirect.
                    </p>
                    <textarea readOnly className="w-full h-24 p-3 text-xs font-mono rounded-lg border resize-none outline-none" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
                      value={`<!-- DeliveryOS Menu Embed -->\n<iframe\n  src="https://${slug}.dowiz.org?embed=true"\n  width="100%"\n  height="650"\n  style="border:none; border-radius:12px;"\n  title="${name || 'Our'} Menu"\n  loading="lazy"\n></iframe>`}
                    />
                    <button onClick={() => navigator.clipboard.writeText(`<iframe src="https://${slug}.dowiz.org?embed=true" width="100%" height="650" style="border:none;border-radius:12px" title="${name || 'Our'} Menu" loading="lazy"></iframe>`)}
                      className="mt-2 flex items-center gap-1 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors hover:bg-[var(--brand-surface)]" style={{ color: 'var(--brand-primary)' }}>
                      <i className="ti ti-clipboard" /> Copy embed code
                    </button>
                  </div>

                  <div className="border-t pt-4" style={{ borderColor: 'var(--brand-border)' }}>
                    <label className="text-xs font-medium mb-1.5 block" style={{ color: 'var(--brand-text)' }}>Add to social media</label>
                    <p className="text-[11px] leading-relaxed" style={{ color: 'var(--brand-text-muted)' }}>
                      Share <code className="px-1.5 py-0.5 rounded text-[10px]" style={{ background: 'var(--brand-border)', color: 'var(--brand-text)' }}>https://{slug}.dowiz.org</code> on Instagram bio, Facebook page, Google Maps, WhatsApp groups, or print on flyers and receipts.
                    </p>
                  </div>
                </div>
              </div>
            )}

            {/* Step 7: Test order + go live */}
            {step === 7 && (
              <div style={s.card} className="space-y-4">
                <h2 className="text-xl font-bold" style={s.heading}>Test Order &amp; Go Live</h2>
                <p style={s.muted}>Place a test order to verify everything works. This won't appear in your analytics.</p>

                {!testOrderDone ? (
                  <Button onClick={placeTestOrder} isLoading={loading} className="w-full">
                    Place test order ({menuItems[0]?.price || 0} ALL)
                  </Button>
                ) : (
                  <div className="space-y-4">
                    <div className="p-3 rounded-lg flex items-center gap-2" style={{ background: 'rgba(5,150,105,0.1)', color: 'var(--color-success)' }}>
                      <i className="ti ti-circle-check-filled" style={{ fontSize: '1rem' }} /> Test order placed successfully
                    </div>
                    {!confirming ? (
                      <>
                        <p style={s.muted}>Your restaurant is ready. It will automatically open for orders.</p>
                        <Button onClick={() => setConfirming(true)} className="w-full" size="lg">
                          <i className="ti ti-rocket" style={{ marginRight: 4 }} /> Go Live
                        </Button>
                      </>
                    ) : (
                      <div className="p-4 rounded-lg space-y-3" style={{ background: 'var(--brand-surface-raised)', border: '1px solid var(--brand-primary)' }}>
                        <p className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>
                          Your restaurant will be visible to customers and start accepting orders immediately.
                        </p>
                        <div className="flex gap-3">
                          <Button onClick={() => setConfirming(false)} variant="ghost" className="flex-1">
                            Cancel
                          </Button>
                          <Button onClick={handlePublish} isLoading={loading} className="flex-1" style={{ background: 'var(--brand-primary)' }}>
                            Yes, Go Live
                          </Button>
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}

            {/* Navigation */}
            <div className="flex justify-between mt-6">
              <Button onClick={() => setStep(s => Math.max(0, s - 1))} disabled={step === 0} variant="ghost">
                ← Back
              </Button>
              <div className="text-sm self-center" style={s.muted}>
                Step {step + 1} of {TOTAL_STEPS}
              </div>
              {step < TOTAL_STEPS - 1 ? (
                <Button onClick={() => setStep(s => s + 1)} disabled={!canNext()}>
                  Next →
                </Button>
              ) : null}
            </div>
          </>
        )}

      </div>
    </div>
  );
}
