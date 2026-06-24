import React, { useEffect, useState, useCallback } from 'react';
import { apiClient } from '../../lib/index.js';
import { useI18n, useToast } from '@deliveryos/ui';

// Menu-first onboarding — split-screen activation tool (O2).
// Left: the gate checklist + Publish. Right: the live draft storefront preview.
// Tool-as-onboarding: see your menu as a real storefront (aha), publish when the
// trinity is green. Mobile: Edit ↔ Preview tabs.

interface GateStatus {
  published: boolean;
  publishedAt: string | null;
  slug: string;
  menuVersion: number;
  gate: { menuConfirmed: boolean; notificationsConnected: boolean; fulfillmentReady: boolean };
  pickupEnabled: boolean;
  canPublish: boolean;
  missing: Array<{ key: string; message: string }>;
}

export function ActivationPage() {
  const { t } = useI18n();
  const { showToast } = useToast();
  const [locationId, setLocationId] = useState('');
  const [slug, setSlug] = useState('');
  const [status, setStatus] = useState<GateStatus | null>(null);
  const [publishing, setPublishing] = useState(false);
  const [tab, setTab] = useState<'edit' | 'preview'>('edit');
  const [settingsError, setSettingsError] = useState(false);
  // Inline product edit (driven by taps inside the preview iframe, O2.2).
  const [editing, setEditing] = useState<{ id: string; name: string; price: number } | null>(null);
  const [editName, setEditName] = useState('');
  const [editPrice, setEditPrice] = useState('');
  const [savingProduct, setSavingProduct] = useState(false);
  const [iframeKey, setIframeKey] = useState(0);

  useEffect(() => {
    apiClient<any>('/owner/settings')
      .then((res: any) => { if (res.id) setLocationId(res.id); if (res.slug) setSlug(res.slug); setSettingsError(false); })
      .catch(() => setSettingsError(true)); // surface, don't swallow → preview shows a retry, not a stuck "Loading…"
  }, []);

  const refresh = useCallback(async () => {
    if (!locationId) return;
    try { setStatus(await apiClient<any>(`/owner/activation/${locationId}/status`)); } catch { /* keep last */ }
  }, [locationId]);

  // Guard the status fetch until locationId resolves so the first render doesn't hit
  // /owner/activation//status (empty id → 404 in console).
  useEffect(() => { if (locationId) refresh(); }, [locationId, refresh]);
  // Poll while a draft so connecting notifications / committing the menu lights the
  // checklist without a manual refresh.
  useEffect(() => {
    if (!locationId || status?.published) return;
    const iv = setInterval(refresh, 8000);
    return () => clearInterval(iv);
  }, [locationId, status?.published, refresh]);

  // Tap-to-edit from the preview iframe (MenuPage posts in activation mode).
  useEffect(() => {
    const onMsg = (e: MessageEvent) => {
      if (e.data?.type === 'dos_activation_edit_product' && e.data.product) {
        const p = e.data.product;
        setEditing({ id: p.id, name: p.name, price: p.price });
        setEditName(p.name ?? '');
        setEditPrice(String(p.price ?? ''));
        setTab('edit');
      }
    };
    window.addEventListener('message', onMsg);
    return () => window.removeEventListener('message', onMsg);
  }, []);

  const saveProduct = async () => {
    if (!editing) return;
    const price = parseInt(editPrice, 10);
    if (!editName.trim() || !Number.isFinite(price) || price < 0) {
      showToast(t('activation.invalid_product', 'Enter a name and a valid price.'), 'error');
      return;
    }
    setSavingProduct(true);
    try {
      await apiClient<any>(`/owner/menu/products/${editing.id}`, { method: 'PATCH', body: { name: editName.trim(), price } });
      setEditing(null);
      setIframeKey((k) => k + 1); // reload preview with the edit
      await refresh();
      showToast(t('activation.saved', 'Saved'), 'success');
    } catch {
      showToast(t('activation.save_failed', 'Could not save.'), 'error');
    } finally {
      setSavingProduct(false);
    }
  };

  const [togglingPickup, setTogglingPickup] = useState(false);
  const togglePickup = async () => {
    if (!locationId) return;
    setTogglingPickup(true);
    try {
      // The route returns the refreshed gate, so the checklist + Publish button
      // update in one round-trip (enabling pickup can flip fulfillment → green).
      const next = await apiClient<any>(`/owner/activation/${locationId}/pickup`, {
        method: 'POST', body: { enabled: !status?.pickupEnabled },
      });
      setStatus(next);
    } catch {
      showToast(t('activation.pickup_failed', 'Could not update pickup.'), 'error');
    } finally {
      setTogglingPickup(false);
    }
  };

  const publish = async () => {
    if (!status?.canPublish || !locationId) return;
    setPublishing(true);
    try {
      await apiClient<any>(`/owner/activation/${locationId}/publish`, { method: 'POST' });
      showToast(t('activation.published_toast', 'Published — your storefront is live!'), 'success');
      await refresh();
    } catch {
      showToast(t('activation.publish_failed', 'Could not publish. Check the checklist.'), 'error');
    } finally {
      setPublishing(false);
    }
  };

  const previewUrl = slug ? `/s/${slug}?embed=true&activation=1` : '';

  const Check = ({ done }: { done: boolean }) => (
    <span
      className="inline-flex items-center justify-center w-6 h-6 rounded-full shrink-0"
      style={{ background: done ? 'var(--color-success)' : 'var(--brand-surface)', color: done ? '#fff' : 'var(--brand-text-muted)', border: done ? 'none' : '2px solid var(--brand-text-muted)' }}
      aria-hidden
    >
      <i className={done ? 'ti ti-check' : 'ti ti-minus'} />
    </span>
  );

  const gateItems = status ? [
    { key: 'menu', done: status.gate.menuConfirmed, href: '/admin/menu', title: t('activation.gate_menu', 'Confirm your menu'), hint: t('activation.gate_menu_hint', 'Upload & review prices/allergens — or tap items in the preview to edit.') },
    { key: 'notifications', done: status.gate.notificationsConnected, href: '/admin/settings', title: t('activation.gate_notifs', 'Connect notifications'), hint: t('activation.gate_notifs_hint', 'So you see new orders instantly (Telegram).') },
    { key: 'fulfillment', done: status.gate.fulfillmentReady, href: '/admin/couriers', title: t('activation.gate_fulfillment', 'Set up fulfillment'), hint: t('activation.gate_fulfillment_hint', 'Enable pickup or add a courier, plus a contact phone.') },
  ] : [];

  const Checklist = (
    <div className="flex flex-col h-full">
      <div className="px-5 pt-5 pb-3">
        <h1 className="text-xl font-bold" style={{ color: 'var(--brand-text)' }}>
          {status?.published ? t('activation.title_live', 'Your storefront is live') : t('activation.title', 'Get your storefront live')}
        </h1>
        <p className="text-sm mt-1" style={{ color: 'var(--brand-text-muted)' }}>
          {/* Viewport-neutral copy: on mobile the preview is a separate "Preview" tab,
              not on the right — so avoid "on the right" / "në të djathtë". */}
          {status?.published
            ? t('activation.subtitle_live_v2', 'Customers can order now. Keep polishing in the preview.')
            : t('activation.subtitle_v2', 'Three steps left. Watch it come together in the preview.')}
        </p>
      </div>

      <div className="flex-1 overflow-auto px-5 space-y-3">
        {gateItems.map((it) => {
          const body = (
            <>
              <Check done={it.done} />
              <div className="min-w-0 flex-1">
                <div className="font-semibold text-sm" style={{ color: 'var(--brand-text)' }}>{it.title}</div>
                <div className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>{it.hint}</div>
              </div>
              {!it.done && <i className="ti ti-chevron-right self-center" style={{ color: 'var(--brand-text-muted)' }} />}
            </>
          );
          return it.done ? (
            <div key={it.key} className="flex gap-3 p-3 rounded-xl" style={{ background: 'var(--brand-bg)' }}>{body}</div>
          ) : (
            <a key={it.key} href={it.href} className="flex gap-3 p-3 rounded-xl transition hover:opacity-90" style={{ background: 'var(--brand-bg)' }}>{body}</a>
          );
        })}

        {/* Zero-friction fulfillment: pickup-only publish (no courier needed). */}
        {status && (
          <button
            type="button"
            onClick={togglePickup}
            disabled={togglingPickup}
            className="flex items-center gap-3 w-full p-3 rounded-xl transition hover:opacity-90 disabled:opacity-60"
            style={{ background: 'var(--brand-bg)' }}
          >
            <span
              className="inline-flex items-center justify-center w-6 h-6 rounded-full shrink-0"
              style={{ background: status.pickupEnabled ? 'var(--color-success)' : 'var(--brand-surface)', color: status.pickupEnabled ? '#fff' : 'var(--brand-text-muted)', border: status.pickupEnabled ? 'none' : '2px solid var(--brand-text-muted)' }}
            >
              <i className={status.pickupEnabled ? 'ti ti-check' : 'ti ti-shopping-bag'} />
            </span>
            <div className="min-w-0 flex-1 text-left">
              <div className="font-semibold text-sm" style={{ color: 'var(--brand-text)' }}>{t('activation.pickup_toggle', 'Offer pickup')}</div>
              <div className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>{t('activation.pickup_hint', 'Go live without a courier — customers collect at the venue.')}</div>
            </div>
            <span className="text-xs font-bold uppercase tracking-wide" style={{ color: status.pickupEnabled ? 'var(--color-success)' : 'var(--brand-text-muted)' }}>
              {togglingPickup ? '…' : status.pickupEnabled ? t('common.on', 'On') : t('common.off', 'Off')}
            </span>
          </button>
        )}

        {!status && <div className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{t('common.loading', 'Loading…')}</div>}

        {/* Optional, visually separate from the must-do trinity (§4). */}
        <div className="pt-2 text-xs uppercase tracking-wide" style={{ color: 'var(--brand-text-muted)' }}>
          {t('activation.optional', 'Recommended (optional)')}
        </div>
        <div className="flex gap-3 p-3 rounded-xl opacity-80" style={{ background: 'var(--brand-bg)' }}>
          <span className="inline-flex items-center justify-center w-6 h-6 rounded-full shrink-0" style={{ border: '2px dashed var(--brand-text-muted)', color: 'var(--brand-text-muted)' }}><i className="ti ti-flask" /></span>
          <div className="min-w-0">
            <div className="font-semibold text-sm" style={{ color: 'var(--brand-text)' }}>{t('activation.test_order', 'Place a test order')}</div>
            <div className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>{t('activation.test_order_hint', 'Try the flow end-to-end before going live.')}</div>
          </div>
        </div>
      </div>

      <div className="p-5 border-t" style={{ borderColor: 'var(--brand-bg)' }}>
        {!status?.published ? (
          <>
            <button
              onClick={publish}
              disabled={!status?.canPublish || publishing}
              className="w-full py-3 rounded-xl font-bold transition disabled:opacity-50 disabled:cursor-not-allowed"
              style={{ background: 'var(--brand-primary-strong)', color: '#fff' }}
            >
              {publishing ? t('activation.publishing', 'Publishing…') : t('activation.publish', 'Publish storefront')}
            </button>
            {status && !status.canPublish && status.missing.length > 0 && (
              <p className="text-xs mt-2 text-center" style={{ color: 'var(--color-warning)' }}>
                {t('activation.still_needed', 'Still needed:')} {status.missing.map((m) => m.message).join(' · ')}
              </p>
            )}
          </>
        ) : (
          <a href={`/s/${slug}`} target="_blank" rel="noreferrer" className="block w-full py-3 rounded-xl font-bold text-center" style={{ background: 'var(--color-success)', color: '#fff' }}>
            {t('activation.view_live', 'View live storefront')} ↗
          </a>
        )}
      </div>
    </div>
  );

  const Preview = (
    <div className="h-full w-full relative" style={{ background: 'var(--brand-bg)' }}>
      {settingsError ? (
        // Don't leave the pane stuck on "Loading…" when settings failed to load — offer a retry.
        <div className="flex flex-col items-center justify-center h-full text-center px-6 gap-3" style={{ color: 'var(--brand-text-muted)' }}>
          <i className="ti ti-plug-connected-x text-3xl" style={{ color: 'var(--color-warning)' }} aria-hidden="true" />
          <p className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>{t('activation.preview_load_failed', "Couldn't load your storefront preview.")}</p>
          <button onClick={() => window.location.reload()} className="px-4 py-2 rounded-xl text-sm font-semibold" style={{ background: 'var(--brand-primary-strong)', color: '#fff' }}>
            <i className="ti ti-refresh mr-1.5" />{t('common.retry', 'Retry')}
          </button>
        </div>
      ) : !status ? (
        <div className="flex items-center justify-center h-full text-sm" style={{ color: 'var(--brand-text-muted)' }}>{t('common.loading', 'Loading…')}</div>
      ) : status.published && previewUrl ? (
        <iframe key={iframeKey} src={previewUrl} title="storefront preview" className="w-full h-full border-0" />
      ) : (
        // Unpublished: the public storefront read excludes drafts, so an iframe would render a
        // "not found" — show an honest publish-to-preview state instead. (Live owner draft-preview
        // is a tracked follow-up requiring an owner-gated read.)
        <div className="flex flex-col items-center justify-center h-full text-center px-6 gap-3" style={{ color: 'var(--brand-text-muted)' }}>
          <i className="ti ti-eye-check text-3xl" style={{ color: 'var(--brand-primary)' }} aria-hidden="true" />
          <p className="text-base font-semibold" style={{ color: 'var(--brand-text)' }}>{t('activation.preview_unpublished_title', 'Preview goes live when you publish')}</p>
          <p className="text-sm max-w-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('activation.preview_unpublished_hint', 'Finish the checklist and publish — your live storefront appears here. Use the checklist to confirm your menu meanwhile.')}</p>
        </div>
      )}
    </div>
  );

  return (
    <div className="h-full" style={{ background: 'var(--brand-surface)' }}>
      {/* Mobile tabs */}
      <div className="md:hidden flex border-b" style={{ borderColor: 'var(--brand-bg)' }}>
        {(['edit', 'preview'] as const).map((tk) => (
          <button key={tk} onClick={() => setTab(tk)} className="flex-1 py-3 text-sm font-semibold" style={{ color: tab === tk ? 'var(--brand-primary)' : 'var(--brand-text-muted)', borderBottom: tab === tk ? '2px solid var(--brand-primary)' : '2px solid transparent' }}>
            {tk === 'edit' ? t('activation.tab_edit', 'Checklist') : t('activation.tab_preview', 'Preview')}
          </button>
        ))}
      </div>

      {/* Desktop split / mobile single */}
      <div className="md:grid md:grid-cols-2 h-[calc(100%-49px)] md:h-full">
        <div className={`${tab === 'edit' ? 'block' : 'hidden'} md:block h-full overflow-hidden`}>{Checklist}</div>
        <div className={`${tab === 'preview' ? 'block' : 'hidden'} md:block h-full border-l`} style={{ borderColor: 'var(--brand-bg)' }}>{Preview}</div>
      </div>

      {editing && (
        <div className="fixed inset-0 z-50 flex items-end md:items-center justify-center" style={{ background: 'var(--brand-overlay, rgba(0,0,0,0.4))' }}>
          <div className="w-full md:max-w-sm rounded-t-2xl md:rounded-2xl p-5" style={{ background: 'var(--brand-surface)' }}>
            <h3 className="font-bold mb-3" style={{ color: 'var(--brand-text)' }}>{t('activation.edit_item', 'Edit menu item')}</h3>
            <label className="block text-xs mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('activation.item_name', 'Name')}</label>
            <input value={editName} onChange={(e) => setEditName(e.target.value)} className="w-full mb-3 px-3 py-2 rounded-lg outline-none" style={{ background: 'var(--brand-bg)', color: 'var(--brand-text)' }} />
            <label className="block text-xs mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('activation.item_price', 'Price (minor units, e.g. 850 = 8.50)')}</label>
            <input value={editPrice} onChange={(e) => setEditPrice(e.target.value.replace(/[^0-9]/g, ''))} inputMode="numeric" className="w-full mb-4 px-3 py-2 rounded-lg outline-none" style={{ background: 'var(--brand-bg)', color: 'var(--brand-text)' }} />
            <div className="flex gap-2">
              <button onClick={() => setEditing(null)} className="flex-1 py-2 rounded-lg font-semibold" style={{ background: 'var(--brand-bg)', color: 'var(--brand-text)' }}>{t('common.cancel', 'Cancel')}</button>
              <button onClick={saveProduct} disabled={savingProduct} className="flex-1 py-2 rounded-lg font-semibold disabled:opacity-50" style={{ background: 'var(--brand-primary-strong)', color: '#fff' }}>{savingProduct ? t('common.saving', 'Saving…') : t('common.save', 'Save')}</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
