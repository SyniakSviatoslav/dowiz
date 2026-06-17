import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Button, Input, EmptyState, MapWithRadius, Toggle, useI18n, LanguageSwitcher, useToast } from '@deliveryos/ui';
import type { LngLatLike, Locale } from '@deliveryos/ui';
import { PHONE_E164_REGEX, PHONE_E164_PATTERN } from '@deliveryos/shared-types';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

const OwnerSettingsResponse = z.object({
  id: z.string().optional(),
  name: z.string().optional(),
  locationName: z.string().optional(),
  slug: z.string().optional(),
  phone: z.string().optional(),
  address: z.string().optional(),
}).passthrough();

const NotificationTargetsResponse = z.object({
  targets: z.array(z.any()),
}).passthrough();

const TelegramConnectResponse = z.object({
  deepLink: z.string().optional(),
}).passthrough();
import QRCode from 'qrcode';

interface DaySchedule {
  isOpen: boolean;
  open: string;
  close: string;
}
type WeeklySchedule = Record<string, DaySchedule>;

const DEFAULT_SCHEDULE: WeeklySchedule = {
  monday: { isOpen: true, open: '09:00', close: '22:00' },
  tuesday: { isOpen: true, open: '09:00', close: '22:00' },
  wednesday: { isOpen: true, open: '09:00', close: '22:00' },
  thursday: { isOpen: true, open: '09:00', close: '22:00' },
  friday: { isOpen: true, open: '09:00', close: '23:00' },
  saturday: { isOpen: true, open: '10:00', close: '23:00' },
  sunday: { isOpen: true, open: '10:00', close: '22:00' },
};

interface LocationSettings {
  locationName: string;
  phone: string;
  address: string;
  deliveryFee: number;
  minOrder: number;
  radiusKm: number;
  lat: number;
  lng: number;
  hoursJson: WeeklySchedule;
}

const MOCK_SETTINGS: LocationSettings = {
  locationName: 'Downtown Tirana',
  phone: '+35542345678',
  address: 'Rruga Ismail Qemali 45, Tirana',
  deliveryFee: 120,
  minOrder: 500,
  radiusKm: 8,
  lat: 41.331,
  lng: 19.817,
  hoursJson: DEFAULT_SCHEDULE,
};

export function SettingsPage() {
  const { t, locale, setLocale } = useI18n();
  const { showToast } = useToast();
  const [settings, setSettings] = useState<LocationSettings>({
    locationName: '',
    phone: '',
    address: '',
    deliveryFee: 0,
    minOrder: 0,
    radiusKm: 0,
    lat: 41.331,
    lng: 19.817,
    hoursJson: DEFAULT_SCHEDULE,
  });
  const [deliveryPaused, setDeliveryPaused] = useState(false);
  const [togglingDelivery, setTogglingDelivery] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  // Telegram state
  const [locationId, setLocationId] = useState<string | null>(null);
  const [tgTargets, setTgTargets] = useState<any[]>([]);
  const [tgDeepLink, setTgDeepLink] = useState<string | null>(null);
  const [tgLoading, setTgLoading] = useState(false);
  const [tgTesting, setTgTesting] = useState(false);
  const [tgMessage, setTgMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const [tgQrDataUrl, setTgQrDataUrl] = useState<string | null>(null);
  const [expandedTarget, setExpandedTarget] = useState<string | null>(null);
  const [prefsSaving, setPrefsSaving] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (!tgDeepLink) { setTgQrDataUrl(null); return; }
    QRCode.toDataURL(tgDeepLink, {
      width: 200,
      margin: 1,
      color: { dark: getComputedStyle(document.documentElement).getPropertyValue('--brand-text').trim() || '#000000', light: getComputedStyle(document.documentElement).getPropertyValue('--brand-bg').trim() || '#ffffff' },
    }).then(setTgQrDataUrl).catch(() => {});
  }, [tgDeepLink]);

  const fetchSettings = async () => {
    try {
      setLoading(true);
      const data = await apiClient<typeof OwnerSettingsResponse>('/owner/settings', { schema: OwnerSettingsResponse });
      if (data && (data.name || (data as any).locationName)) {
        setSettings({
          ...(data as any),
          hoursJson: (data as any).hoursJson || DEFAULT_SCHEDULE,
          lat: (data as any).lat || 41.331,
          lng: (data as any).lng || 19.817,
        });
        if (data.id) setLocationId(data.id);
        setDeliveryPaused((data as any).deliveryPaused ?? false);
      } else {
        setSettings(MOCK_SETTINGS);
      }
    } catch (err: any) {
      if (err.status === 404) {
        setSettings(MOCK_SETTINGS);
      } else {
        setError('Failed to load settings');
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchSettings();
  }, []);

  const fetchTgTargets = useCallback(async () => {
    if (!locationId) return;
    try {
      const res = await apiClient<typeof NotificationTargetsResponse>(`/owner/locations/${locationId}/notifications/targets`, { schema: NotificationTargetsResponse });
      setTgTargets(res?.targets || []);
    } catch (err) { console.warn('[SettingsPage] fetch tg targets failed:', err); }
  }, [locationId]);

  useEffect(() => {
    if (!locationId) return;
    fetchTgTargets();
  }, [locationId, fetchTgTargets]);

  const handleTgConnect = async () => {
    if (!locationId) return;
    setTgLoading(true);
    setTgMessage(null);
    try {
      const res = await apiClient<typeof TelegramConnectResponse>(`/owner/locations/${locationId}/notifications/telegram/connect-init`, { method: 'POST', schema: TelegramConnectResponse });
      setTgDeepLink(res.deepLink ?? null);
    } catch (err: any) {
      setTgMessage({ type: 'error', text: err.message || 'Failed to initiate connection' });
    } finally {
      setTgLoading(false);
    }
  };

  const handleTgTest = async () => {
    if (!locationId) return;
    setTgTesting(true);
    setTgMessage(null);
    try {
      await apiClient(`/owner/locations/${locationId}/notifications/test`, { method: 'POST' });
      setTgMessage({ type: 'success', text: 'Test notification sent! Check your Telegram.' });
    } catch (err: any) {
      setTgMessage({ type: 'error', text: err.message || 'Failed to send test' });
    } finally {
      setTgTesting(false);
    }
  };

  const handleTgToggle = async (targetId: string, currentStatus: string) => {
    if (!locationId) return;
    const newStatus = currentStatus === 'active' ? 'disabled' : 'active';
    try {
      await apiClient(`/owner/locations/${locationId}/notifications/targets/${targetId}`, {
        method: 'PUT',
        body: { status: newStatus }
      });
      await fetchTgTargets();
    } catch (err) { console.warn('[SettingsPage] toggle tg target failed:', err); }
  };

  const handleSavePrefs = useCallback(async (targetId: string, patch: Record<string, any>) => {
    if (!locationId) return;
    setPrefsSaving(prev => ({ ...prev, [targetId]: true }));
    try {
      await apiClient(`/owner/locations/${locationId}/notifications/targets/${targetId}`, {
        method: 'PUT',
        body: { prefs: patch }
      });
      await fetchTgTargets();
    } catch (err) {
      console.warn('[SettingsPage] save prefs failed:', err);
    } finally {
      setPrefsSaving(prev => ({ ...prev, [targetId]: false }));
    }
  }, [locationId, fetchTgTargets]);

  const handleChange = (field: keyof LocationSettings, value: any) => {
    setSettings((prev) => ({
      ...prev,
      [field]:
        field === 'deliveryFee' || field === 'minOrder' || field === 'radiusKm'
          ? Number(value) || 0
          : value,
    }));
  };

  const handleScheduleChange = (day: string, field: keyof DaySchedule, value: any) => {
    setSettings((prev) => {
      const currentDay = prev.hoursJson[day as keyof WeeklySchedule] || DEFAULT_SCHEDULE[day];
      const newSchedule = {
        ...prev.hoursJson,
        [day]: {
          ...currentDay,
          [field]: value
        }
      } as WeeklySchedule;
      return {
        ...prev,
        hoursJson: newSchedule
      };
    });
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (settings.phone && !PHONE_E164_REGEX.test(settings.phone)) {
      setError('Phone must be in international format (+355...)');
      return;
    }
    setSaving(true);
    try {
      await apiClient('/owner/settings', {
        method: 'PUT',
        body: { ...settings, deliveryPaused },
      });
      showToast(t('common.saved', 'Settings saved'), 'success');
    } catch (err: any) {
      if (err.status === 404) {
        showToast(t('common.saved', 'Settings saved'), 'success');
      } else {
        showToast(t('common.error', 'Failed to save settings'), 'error');
        setError('Failed to save settings');
      }
    } finally {
      setSaving(false);
    }
  };

  const fieldStyle: React.CSSProperties = {
    backgroundColor: 'var(--brand-surface)',
    borderColor: 'var(--brand-border)',
    borderRadius: 'var(--brand-radius)',
    color: 'var(--brand-text)',
  };

  const labelStyle: React.CSSProperties = {
    color: 'var(--brand-text-muted)',
  };

  return (
    <div className="p-4 space-y-6 max-w-2xl mx-auto">
      <div className="border-b border-[var(--brand-border)] pb-4">
        <h2
          className="text-2xl font-bold"
          style={{ fontFamily: 'var(--brand-font-heading)' }}
        >
          {t('admin.settings')}
        </h2>
      </div>

      {loading ? (
        <div className="animate-pulse space-y-4">
          {[1, 2, 3, 4, 5, 6].map((i) => (
            <div key={i}>
              <div className="h-4 bg-[var(--brand-surface)] rounded w-24 mb-2" />
              <div className="h-10 bg-[var(--brand-surface)] rounded-[var(--brand-radius)] w-full" />
            </div>
          ))}
          <div className="h-10 bg-[var(--brand-surface)] rounded-[var(--brand-radius)] w-32" />
        </div>
      ) : error && settings.locationName === '' ? (
        <EmptyState title={t('common.error')} description={error} />
      ) : (
        <form onSubmit={handleSubmit} className="space-y-5">
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <div>
              <label htmlFor="settings-locationName" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('admin.location_name', 'Location Name')}
              </label>
              <Input
                id="settings-locationName"
                value={settings.locationName}
                onChange={(e) => handleChange('locationName', e.target.value)}
                placeholder={t('admin.placeholder_location_name', 'e.g. Downtown Tirana')}
                required
              />
            </div>

            <div>
              <label htmlFor="settings-phone" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('common.phone', 'Phone')}
              </label>
              <Input
                id="settings-phone"
                value={settings.phone}
                onChange={(e) => handleChange('phone', e.target.value)}
                placeholder={t('admin.placeholder_phone', '+355...')}
                pattern={PHONE_E164_PATTERN}
                title={t('admin.phone_format_hint', '+355 followed by 7-14 digits')}
                required
              />
            </div>

            <div>
              <label htmlFor="settings-address" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('common.address', 'Address')}
              </label>
              <Input
                id="settings-address"
                value={settings.address}
                onChange={(e) => handleChange('address', e.target.value)}
                placeholder={t('admin.placeholder_address', 'Street, City')}
                required
              />
            </div>
          </div>

          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
              {t('admin.delivery_config', 'Delivery Config')}
            </h3>

            <div>
              <label htmlFor="settings-deliveryFee" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('cart.delivery_fee', 'Delivery Fee')} (ALL)
              </label>
              <Input
                id="settings-deliveryFee"
                type="number"
                value={String(settings.deliveryFee)}
                onChange={(e) => handleChange('deliveryFee', e.target.value)}
                min="0"
                required
              />
            </div>

            <div>
              <label htmlFor="settings-minOrder" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('admin.min_order', 'Minimum Order')} (ALL)
              </label>
              <Input
                id="settings-minOrder"
                type="number"
                value={String(settings.minOrder)}
                onChange={(e) => handleChange('minOrder', e.target.value)}
                min="0"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-3" style={labelStyle}>
                {t('admin.delivery_zone', 'Delivery Zone (Radius & Location)')}
              </label>
              <MapWithRadius
                className="h-64 w-full rounded-lg mb-2"
                initialCenter={[settings.lng, settings.lat]}
                initialRadiusKm={settings.radiusKm}
                onRadiusChange={(c, r) => {
                  handleChange('lng', c[0]);
                  handleChange('lat', c[1]);
                  handleChange('radiusKm', r);
                }}
              />
              <p className="text-xs" style={labelStyle}>{t('admin.map_hint', 'Drag the pin to update location, adjust radius for delivery zone.')}</p>
            </div>
          </div>

          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
              {t('admin.working_hours', 'Working Hours')}
            </h3>
            <div className="grid grid-cols-1 sm:grid-cols-[120px_1fr] gap-2">
              {['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'].map(day => {
                const dayData = (settings.hoursJson[day as keyof WeeklySchedule] || DEFAULT_SCHEDULE[day]) as DaySchedule;
                return (
                  <div key={day} className="contents">
                    <div className="flex items-center h-10 px-2 rounded-lg border border-[var(--brand-border)] bg-[var(--brand-bg)] font-medium text-sm capitalize">
                      {t(`admin.days.${day}`, day)}
                    </div>
                    <div className="flex items-center gap-2 p-2 rounded-lg border border-[var(--brand-border)] bg-[var(--brand-bg)]">
                      <Toggle checked={dayData.isOpen} onChange={(v) => handleScheduleChange(day, 'isOpen', v)} aria-label={`${t(`admin.days.${day}`, day)} ${dayData.isOpen ? t('admin.open', 'Open') : t('admin.closed', 'Closed')}`} />
                      <span className="text-xs w-10 text-[var(--brand-text-muted)] shrink-0">{dayData.isOpen ? t('admin.open', 'Open') : t('admin.closed', 'Closed')}</span>
                      {dayData.isOpen && (
                        <div className="flex items-center gap-2 ml-auto">
                          <Input type="time" value={dayData.open} onChange={(e) => handleScheduleChange(day, 'open', e.target.value)} aria-label={`${t(`admin.days.${day}`, day)} ${t('admin.open', 'open')}`} className="w-20 sm:w-28 text-sm" />
                          <span className="text-xs text-[var(--brand-text-muted)] shrink-0">{t('admin.to', 'to')}</span>
                          <Input type="time" value={dayData.close} onChange={(e) => handleScheduleChange(day, 'close', e.target.value)} aria-label={`${t(`admin.days.${day}`, day)} ${t('admin.close', 'close')}`} className="w-20 sm:w-28 text-sm" />
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* ── Language Preference ── */}
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <h3 className="font-semibold text-sm" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.language', 'Language')}
            </h3>
            <p className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.language_desc', 'Language for admin panel and Telegram notifications.')}
            </p>
            <LanguageSwitcher variant="full" />
          </div>

          {/* ── Telegram Notifications ── */}
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <div className="flex items-center gap-2">
              <i className="ti ti-brand-telegram text-lg" style={{ color: 'var(--color-info)' }} />
              <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
                {t('admin.telegram_notifications', 'Telegram Notifications')}
              </h3>
            </div>

            <p className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.telegram_desc', 'Receive order alerts and manage your restaurant directly from Telegram.')}
            </p>

            {/* Connected targets */}
            {tgTargets.length > 0 && (
              <div className="space-y-2">
                {tgTargets.map((tgt: any) => {
                  const prefs = tgt.prefs || {};
                  const isExpanded = expandedTarget === tgt.id;
                  const opsOn = prefs.category_operations !== false;
                  const analyticsOn = prefs.category_analytics === true;
                  const quietStart: string = prefs.quiet_start || '';
                  const quietEnd: string = prefs.quiet_end || '';
                  return (
                    <div key={tgt.id} className="rounded-xl border overflow-hidden" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
                      {/* Header row */}
                      <div className="flex items-center gap-3 p-3">
                        <div className={`w-2 h-2 rounded-full shrink-0 ${tgt.status === 'active' ? 'bg-[var(--color-success)]' : 'bg-[var(--brand-text-muted)]'}`} />
                        <div className="flex-1 min-w-0">
                          <div className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>
                            {tgt.channel === 'telegram' ? 'Telegram' : tgt.channel}
                          </div>
                          <div className="text-[10px] font-mono" style={{ color: 'var(--brand-text-muted)' }}>
                            {tgt.address?.slice(0, 14)}…
                          </div>
                        </div>
                        <button
                          onClick={() => setExpandedTarget(isExpanded ? null : tgt.id)}
                          className="flex items-center gap-1 text-[11px] font-medium px-2 py-1 rounded-md transition-colors"
                          style={{ color: 'var(--brand-text-muted)', background: 'var(--brand-surface-raised)' }}
                          aria-expanded={isExpanded}
                        >
                          <i className={`ti ti-settings text-xs`} />
                          {t('admin.prefs', 'Prefs')}
                          <i className={`ti ti-chevron-${isExpanded ? 'up' : 'down'} text-[9px]`} />
                        </button>
                        <button
                          onClick={() => handleTgToggle(tgt.id, tgt.status)}
                          aria-label={tgt.status === 'active' ? t('admin.disable', 'Disable') : t('admin.enable', 'Enable')}
                          title={tgt.status === 'active' ? t('admin.disable', 'Disable') : t('admin.enable', 'Enable')}
                          className={`w-8 h-8 shrink-0 flex items-center justify-center rounded-lg transition-colors ${tgt.status === 'active' ? 'bg-[var(--color-success-light)]' : 'bg-[var(--brand-surface-raised)]'}`}
                        >
                          <i className={`ti ti-${tgt.status === 'active' ? 'check' : 'power'} text-xs`} style={{ color: tgt.status === 'active' ? 'var(--color-success)' : 'var(--brand-text-muted)' }} />
                        </button>
                      </div>

                      {/* Expanded: category prefs + quiet hours */}
                      {isExpanded && (
                        <div className="border-t px-4 py-4 space-y-4" style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface)' }}>

                          {/* Category toggles */}
                          <div>
                            <p className="text-[11px] font-semibold uppercase tracking-wide mb-2" style={{ color: 'var(--brand-text-muted)' }}>
                              {t('admin.notif_categories', 'Notification types')}
                            </p>
                            <div className="space-y-2">
                              {/* 🔴 Orders — locked */}
                              <div className="flex items-start gap-3 p-2.5 rounded-lg" style={{ background: 'color-mix(in srgb, var(--color-danger) 6%, var(--brand-surface))' }}>
                                <div className="w-8 h-5 rounded-full flex items-center justify-center shrink-0 mt-0.5" style={{ background: 'var(--color-danger)', opacity: 0.9 }}>
                                  <i className="ti ti-lock text-white" style={{ fontSize: '9px' }} />
                                </div>
                                <div className="flex-1">
                                  <div className="text-[12px] font-semibold" style={{ color: 'var(--brand-text)' }}>
                                    {t('admin.notif_orders', 'Orders')}
                                  </div>
                                  <div className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>
                                    {t('admin.notif_orders_desc', 'New orders, confirmations, escalations — always on')}
                                  </div>
                                </div>
                              </div>

                              {/* 🟠 Operations */}
                              <div className="flex items-start gap-3 p-2.5 rounded-lg" style={{ background: 'var(--brand-bg)' }}>
                                <div className="shrink-0 mt-0.5">
                                  <button
                                    role="switch"
                                    aria-checked={opsOn}
                                    onClick={() => handleSavePrefs(tgt.id, { category_operations: !opsOn })}
                                    disabled={!!prefsSaving[tgt.id]}
                                    className={`relative w-8 h-5 rounded-full transition-colors duration-200 ${opsOn ? 'bg-[var(--color-warning)]' : 'bg-[var(--brand-border)]'}`}
                                    style={{ opacity: prefsSaving[tgt.id] ? 0.5 : 1 }}
                                  >
                                    <div className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 ${opsOn ? 'left-[14px]' : 'left-0.5'}`} />
                                  </button>
                                </div>
                                <div className="flex-1">
                                  <div className="text-[12px] font-semibold" style={{ color: 'var(--brand-text)' }}>
                                    {t('admin.notif_operations', 'Operations')}
                                  </div>
                                  <div className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>
                                    {t('admin.notif_operations_desc', 'Cash discrepancies, timeout cancellations, system alerts')}
                                  </div>
                                </div>
                              </div>

                              {/* 🟡 Analytics */}
                              <div className="flex items-start gap-3 p-2.5 rounded-lg" style={{ background: 'var(--brand-bg)' }}>
                                <div className="shrink-0 mt-0.5">
                                  <button
                                    role="switch"
                                    aria-checked={analyticsOn}
                                    onClick={() => handleSavePrefs(tgt.id, { category_analytics: !analyticsOn })}
                                    disabled={!!prefsSaving[tgt.id]}
                                    className={`relative w-8 h-5 rounded-full transition-colors duration-200 ${analyticsOn ? 'bg-[var(--brand-primary)]' : 'bg-[var(--brand-border)]'}`}
                                    style={{ opacity: prefsSaving[tgt.id] ? 0.5 : 1 }}
                                  >
                                    <div className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 ${analyticsOn ? 'left-[14px]' : 'left-0.5'}`} />
                                  </button>
                                </div>
                                <div className="flex-1">
                                  <div className="text-[12px] font-semibold" style={{ color: 'var(--brand-text)' }}>
                                    {t('admin.notif_analytics', 'Insights & Ratings')}
                                  </div>
                                  <div className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>
                                    {t('admin.notif_analytics_desc', 'Low ratings, daily summaries — off by default')}
                                  </div>
                                </div>
                              </div>
                            </div>
                          </div>

                          {/* Quiet hours */}
                          <div>
                            <p className="text-[11px] font-semibold uppercase tracking-wide mb-1" style={{ color: 'var(--brand-text-muted)' }}>
                              {t('admin.quiet_hours', 'Quiet hours')}
                            </p>
                            <p className="text-[10px] mb-2" style={{ color: 'var(--brand-text-muted)' }}>
                              {t('admin.quiet_hours_desc', 'Operations and insights are held. Orders always break through.')}
                            </p>
                            <div className="flex items-center gap-2">
                              <input
                                type="time"
                                value={quietStart}
                                onChange={e => handleSavePrefs(tgt.id, { quiet_start: e.target.value || null })}
                                className="h-8 px-2 text-[12px] rounded-lg border outline-none"
                                style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)', minWidth: 90 }}
                                placeholder="23:00"
                              />
                              <span className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.to', 'to')}</span>
                              <input
                                type="time"
                                value={quietEnd}
                                onChange={e => handleSavePrefs(tgt.id, { quiet_end: e.target.value || null })}
                                className="h-8 px-2 text-[12px] rounded-lg border outline-none"
                                style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)', minWidth: 90 }}
                                placeholder="08:00"
                              />
                              {(quietStart || quietEnd) && (
                                <button
                                  onClick={() => handleSavePrefs(tgt.id, { quiet_start: null, quiet_end: null })}
                                  className="text-[10px] px-2 py-1 rounded-md transition-colors"
                                  style={{ color: 'var(--color-danger)', background: 'var(--color-danger-light)' }}
                                >
                                  {t('common.clear', 'Clear')}
                                </button>
                              )}
                            </div>
                            {!quietStart && !quietEnd && (
                              <p className="text-[10px] mt-1" style={{ color: 'var(--brand-text-muted)' }}>
                                {t('admin.quiet_hours_default', 'Default: 22:00 – 08:00 UTC')}
                              </p>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}

            {/* Deep link flow + QR */}
            {tgDeepLink && (
              <div className="p-3 rounded-lg border" style={{ background: 'var(--brand-primary-light)', borderColor: 'var(--brand-primary)' }}>
                <div className="flex items-start gap-4">
                  {tgQrDataUrl && (
                    <div className="shrink-0">
                      <img src={tgQrDataUrl} alt="QR Code" className="w-24 h-24 rounded-lg" />
                    </div>
                  )}
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-semibold mb-1.5" style={{ color: 'var(--brand-primary)' }}>
                      {t('admin.tg_step1', '1. Scan QR or open this link in Telegram:')}
                    </div>
                    <a href={tgDeepLink} target="_blank" rel="noopener noreferrer"
                      className="text-sm font-mono underline break-all block" style={{ color: 'var(--brand-primary)' }}>
                      {tgDeepLink}
                    </a>
                    <div className="text-[10px] mt-2" style={{ color: 'var(--brand-text-muted)' }}>
                      {t('admin.tg_step2', '2. Click Start in the bot. Your Telegram will be connected automatically.')}
                    </div>
                  </div>
                </div>
                <div className="flex gap-2 mt-2">
                  <Button onClick={() => { setTgDeepLink(null); handleTgConnect(); }} variant="ghost" size="sm">
                    <i className="ti ti-refresh" /> {t('common.refresh', 'Refresh')}
                  </Button>
                </div>
              </div>
            )}

            {/* Messages */}
            {tgMessage && (
              <div role="alert" aria-live="polite" className="p-3 rounded-lg text-xs" style={{
                background: tgMessage.type === 'success' ? 'var(--color-success-light)' : 'var(--color-danger-light)',
                color: tgMessage.type === 'success' ? 'var(--color-success)' : 'var(--color-danger)'
              }}>
                {tgMessage.text}
              </div>
            )}

            {/* Actions */}
            <div className="flex gap-2">
              <Button onClick={handleTgConnect} isLoading={tgLoading} variant="ghost" size="sm">
                <i className="ti ti-brand-telegram" /> {tgTargets.length > 0 ? t('admin.tg_add_another', 'Add Another') : t('admin.tg_connect', 'Connect Telegram')}
              </Button>
              {tgTargets.length > 0 && (
                <Button onClick={handleTgTest} isLoading={tgTesting} variant="ghost" size="sm">
                  <i className="ti ti-send" /> {t('admin.tg_test', 'Send Test')}
                </Button>
              )}
            </div>
          </div>

          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5">
            <div className="flex items-center justify-between gap-4">
              <div>
                <h3 className="font-semibold text-sm">{t('admin.delivery_status', 'Delivery Status')}</h3>
                <p className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>
                  {deliveryPaused
                    ? t('admin.delivery_paused_hint', 'Delivery is paused. Customers see a "Closed" message.')
                    : t('admin.delivery_open_hint', 'Delivery is open based on your hours schedule.')}
                </p>
              </div>
              <div style={{ opacity: togglingDelivery ? 0.5 : 1, pointerEvents: togglingDelivery ? 'none' : 'auto' }}>
                <Toggle
                  checked={!deliveryPaused}
                  onChange={async (v) => {
                    const newPaused = !v;
                    setTogglingDelivery(true);
                    try {
                      await apiClient('/owner/settings', { method: 'PUT', body: { deliveryPaused: newPaused } });
                      setDeliveryPaused(newPaused);
                      showToast(newPaused ? t('admin.delivery_paused', 'Delivery paused') : t('admin.delivery_resumed', 'Delivery resumed'), 'success');
                    } catch { showToast(t('common.error', 'Failed to update delivery status'), 'error'); }
                    finally { setTogglingDelivery(false); }
                  }}
                  label={deliveryPaused ? t('admin.delivery_closed_label', 'Closed') : t('admin.delivery_open_label', 'Open')}
                />
              </div>
            </div>
          </div>

          <div className="flex items-center gap-4">
            <Button type="submit" isLoading={saving} size="lg">
              {t('common.save', 'Save Changes')}
            </Button>
            {error && settings.locationName !== '' && (
              <span role="alert" className="text-[var(--color-danger)] text-sm">{error}</span>
            )}
          </div>
        </form>
      )}
    </div>
  );
}
